use super::super::Session;
use super::{AssistantEvent, Control, Event, Shared, commands, invalid_data, wire};
use serde_json::{Value, json};
use std::{
    io::{self, Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpStream as BlockingStream},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{Semaphore, mpsc, watch},
    task::{Id, JoinSet},
};

pub const ASSISTANT_MCP_ARG: &str = "--assistant-mcp-bridge";
const TOKEN_ENV: &str = "MGB_OPS_ASSISTANT_TOKEN";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const CALL_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_CONNECTIONS: usize = 4;
const MAX_ONGOING_CALLS: usize = 4;
const MAX_RESULT_BYTES: usize = 96 * 1024;
const TOKEN_BYTES: usize = 32;

pub(super) struct Bridge {
    listener: TcpListener,
    token: [u8; TOKEN_BYTES],
}

#[derive(Clone)]
pub(super) struct Gateway {
    pub(super) shared: Arc<Shared>,
    pub(super) session: Session,
    pub(super) events: mpsc::Sender<Event>,
    pub(super) control: watch::Receiver<Control>,
    pub(super) connected: Arc<AtomicBool>,
}

impl Bridge {
    pub(super) async fn bind() -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let mut token = [0; TOKEN_BYTES];
        getrandom::fill(&mut token)
            .map_err(|_| io::Error::other("Could not create the assistant credential"))?;
        Ok(Self { listener, token })
    }

    pub(super) fn declaration(&self, helper: &Path) -> io::Result<Value> {
        let port = self.listener.local_addr()?.port().to_string();
        let token = self
            .token
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let command = helper
            .to_str()
            .ok_or_else(|| invalid_data("MCP helper path is not UTF-8"))?;
        Ok(json!({
            "name": "mgb_ops", "command": command,
            "args": [ASSISTANT_MCP_ARG, port],
            "env": [{"name": TOKEN_ENV, "value": token}]
        }))
    }

    pub(super) async fn serve(self, gateway: Gateway) {
        let connections = Arc::new(Semaphore::new(MAX_CONNECTIONS));
        let calls = Arc::new(Semaphore::new(MAX_ONGOING_CALLS));
        while let Ok((socket, _)) = self.listener.accept().await {
            if gateway.shared.is_closed() {
                return;
            }
            let Ok(permit) = connections.clone().try_acquire_owned() else {
                continue;
            };
            let gateway = gateway.clone();
            let calls = calls.clone();
            let token = self.token;
            tokio::spawn(async move {
                let _permit = permit;
                let _ = connection(socket, &token, gateway, calls).await;
            });
        }
    }
}

struct Pending {
    task: Id,
    id: Value,
    turn: u64,
    cancelled: Arc<AtomicBool>,
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct Reply {
    turn: u64,
    id: Value,
    tool: String,
    command: Option<String>,
    result: commands::CommandResult,
}

struct Connection {
    initialized: bool,
    pending: Vec<Pending>,
    jobs: JoinSet<Reply>,
}

impl Connection {
    fn new() -> Self {
        Self {
            initialized: false,
            pending: Vec::new(),
            jobs: JoinSet::new(),
        }
    }

    fn cancel(&mut self, request_id: &Value) {
        self.pending.retain(|pending| &pending.id != request_id);
    }

    fn expire_turn(&mut self, cancelled: u64) {
        self.pending.retain(|pending| pending.turn > cancelled);
    }

    fn request(
        &mut self,
        request: Value,
        gateway: &Gateway,
        calls: &Arc<Semaphore>,
    ) -> Option<Value> {
        let id = request.get("id").cloned();
        let parameters = &request["params"];
        match request["method"].as_str() {
            Some("initialize") if !self.initialized => {
                let id = id?;
                let version = match parameters["protocolVersion"].as_str() {
                    Some(version @ ("2024-11-05" | "2025-03-26" | "2025-06-18" | "2025-11-25")) => {
                        version
                    }
                    Some(_) => "2025-11-25",
                    None => return Some(wire::error(id, -32602, "Missing MCP protocol version")),
                };
                self.initialized = true;
                Some(wire::response(
                    id,
                    json!({
                        "protocolVersion": version, "capabilities": {"tools": {}},
                        "serverInfo": {"name": "mgb_ops", "version": env!("CARGO_PKG_VERSION")}
                    }),
                ))
            }
            Some("notifications/initialized") if self.initialized => {
                gateway.mark_connected();
                None
            }
            Some("notifications/cancelled") => {
                self.cancel(&parameters["requestId"]);
                None
            }
            Some("ping") => id.map(|id| wire::response(id, json!({}))),
            Some("tools/list") if self.initialized => {
                gateway.mark_connected();
                id.map(|id| wire::response(id, json!({"tools": commands::tools()})))
            }
            Some("tools/call") if self.initialized => self.call(id?, parameters, gateway, calls),
            _ => id.map(|id| wire::error(id, -32601, "Unsupported MCP method")),
        }
    }

    fn call(
        &mut self,
        id: Value,
        params: &Value,
        gateway: &Gateway,
        calls: &Arc<Semaphore>,
    ) -> Option<Value> {
        let Some(tool) = params["name"].as_str() else {
            return Some(wire::error(id, -32602, "Missing tool name"));
        };
        let Some(turn) = gateway.shared.turn() else {
            return Some(wire::error(id, -32000, "No active assistant turn"));
        };
        if self.pending.iter().any(|pending| pending.id == id) {
            return Some(wire::error(id, -32600, "Duplicate request ID"));
        }
        if self.jobs.len() >= MAX_ONGOING_CALLS {
            return Some(wire::error(id, -32000, "Too many pending tool calls"));
        }
        let Ok(permit) = calls.clone().try_acquire_owned() else {
            return Some(wire::error(id, -32000, "Assistant tools are busy"));
        };
        let args = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let command = args["id"]
            .as_str()
            .filter(|_| tool == "run_command")
            .map(str::to_owned);
        let cancelled = Arc::new(AtomicBool::new(false));
        let request_cancelled = cancelled.clone();
        let context = commands::CommandContext::new(
            gateway.session.clone(),
            gateway.shared.clone(),
            turn,
            cancelled.clone(),
        );
        let request_id = id.clone();
        let tool = tool.to_owned();
        let task = self
            .jobs
            .spawn(async move {
                let name = tool.clone();
                let work = tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    commands::dispatch(&context, &name, args)
                });
                let result = match tokio::time::timeout(CALL_TIMEOUT, work).await {
                    Ok(Ok(result)) => result,
                    Ok(Err(_)) => Err(commands::CommandFinding::new(
                        "tool_failure",
                        "The assistant tool worker failed",
                    )),
                    Err(_) => {
                        request_cancelled.store(true, Ordering::Release);
                        Err(commands::CommandFinding::new(
                            "tool_timeout",
                            "The assistant tool exceeded its time limit",
                        ))
                    }
                };
                Reply {
                    turn,
                    id: request_id,
                    tool,
                    command,
                    result,
                }
            })
            .id();
        self.pending.push(Pending {
            task,
            id,
            turn,
            cancelled,
        });
        None
    }

    fn completed(&mut self, task: Id, reply: Reply, gateway: &Gateway) -> Option<Value> {
        let index = self
            .pending
            .iter()
            .position(|pending| pending.task == task)?;
        self.pending.swap_remove(index);
        if gateway.shared.turn() != Some(reply.turn) {
            return None;
        }
        let (body, failed) = match reply.result {
            Ok(value) => (value, false),
            Err(error) => (json!({"finding": error}), true),
        };
        let (text, failed) = match serde_json::to_string(&body) {
            Ok(text) if text.len() <= MAX_RESULT_BYTES => (text, failed),
            _ => (
                String::from(
                    r#"{"finding":{"code":"response_limit","message":"The result exceeds the assistant response size limit"}}"#,
                ),
                true,
            ),
        };
        let _ = gateway.events.try_send(Event {
            turn: Some(reply.turn),
            value: AssistantEvent::ToolCallCompleted {
                tool: reply.tool,
                command: reply.command,
                ok: !failed,
            },
        });
        Some(wire::response(
            reply.id,
            json!({"content": [{"type": "text", "text": text}], "isError": failed}),
        ))
    }
}

impl Gateway {
    fn mark_connected(&self) {
        if !self.connected.swap(true, Ordering::AcqRel) {
            let _ = self.events.try_send(Event {
                turn: None,
                value: AssistantEvent::ToolsConnected,
            });
        }
    }
}

async fn connection(
    mut socket: TcpStream,
    token: &[u8; TOKEN_BYTES],
    mut gateway: Gateway,
    calls: Arc<Semaphore>,
) -> io::Result<()> {
    let mut presented = [0; TOKEN_BYTES];
    tokio::time::timeout(CONNECT_TIMEOUT, socket.read_exact(&mut presented)).await??;
    let difference = presented
        .iter()
        .zip(token)
        .fold(0, |acc, (a, b)| acc | (a ^ b));
    if difference != 0 || gateway.shared.is_closed() {
        return Ok(());
    }
    socket.write_all(&[1]).await?;
    let (read, mut write) = socket.split();
    let mut input = BufReader::new(read);
    let mut reader = wire::Reader::default();
    let mut state = Connection::new();
    loop {
        let response = tokio::select! {
            biased;
            control = gateway.control.changed() => {
                if control.is_err() || gateway.control.borrow().closed {
                    return Ok(());
                }
                state.expire_turn(gateway.control.borrow().cancelled);
                None
            },
            message = reader.next(&mut input) => {
                let Some(message) = message? else { return Ok(()) };
                state.request(message, &gateway, &calls)
            },
            Some(result) = state.jobs.join_next_with_id(), if !state.jobs.is_empty() => {
                match result {
                    Ok((task, reply)) => state.completed(task, reply, &gateway),
                    Err(error) => {
                        state.pending.retain(|pending| pending.task != error.id());
                        None
                    }
                }
            }
        };
        if let Some(response) = response {
            if gateway.shared.is_closed() {
                return Ok(());
            }
            wire::send(&mut write, &response).await?;
        }
    }
}

/// Run the stdio relay when invoked with the host-owned helper argument.
/// Returns `None` for ordinary application entry points.
pub fn assist_mcp_entry() -> Option<io::Result<()>> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(ASSISTANT_MCP_ARG)) {
        return None;
    }
    let port = args
        .next()
        .and_then(|value| value.to_str()?.parse::<u16>().ok())
        .filter(|port| *port != 0);
    Some(match (port, args.next()) {
        (Some(port), None) => assist_mcp_stdio(port),
        _ => Err(invalid_data("Expected a single MCP loopback port")),
    })
}

pub fn assist_mcp_stdio(port: u16) -> io::Result<()> {
    let token = std::env::var(TOKEN_ENV)
        .ok()
        .filter(|token| token.len() == TOKEN_BYTES * 2 && token.is_ascii())
        .and_then(|token| {
            (0..TOKEN_BYTES)
                .map(|index| u8::from_str_radix(&token[2 * index..2 * index + 2], 16).ok())
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Missing or malformed MCP credential",
            )
        })?;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut socket = BlockingStream::connect_timeout(&address, CONNECT_TIMEOUT)?;
    socket.set_read_timeout(Some(CONNECT_TIMEOUT))?;
    socket.set_write_timeout(Some(CONNECT_TIMEOUT))?;
    socket.write_all(&token)?;
    let mut accepted = [0];
    socket.read_exact(&mut accepted)?;
    if accepted != [1] {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "MCP credential rejected",
        ));
    }
    socket.set_read_timeout(None)?;
    socket.set_write_timeout(None)?;
    let mut input = socket.try_clone()?;
    std::thread::spawn(move || {
        let _ = relay(&mut io::stdin().lock(), &mut input);
        let _ = input.shutdown(Shutdown::Write);
    });
    relay(&mut socket, &mut io::stdout().lock())
}

fn relay(source: &mut impl Read, destination: &mut impl Write) -> io::Result<()> {
    let mut bytes = [0; 8192];
    loop {
        let count = match source.read(&mut bytes) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            read => read?,
        };
        if count == 0 {
            return Ok(());
        }
        destination.write_all(&bytes[..count])?;
        destination.flush()?;
    }
}

#[cfg(test)]
mod tests;
