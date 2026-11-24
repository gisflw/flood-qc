use super::{AssistantEvent, Tools, bounded_line, commands};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufReader, Write},
    net::{Ipv4Addr, TcpListener, TcpStream},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::SyncSender,
    },
    thread,
    time::Duration,
};

const BOOTSTRAP: &str = "MGB_OPS_ASSISTANT_BOOTSTRAP";
const IO_TIMEOUT: Duration = Duration::from_secs(3);

pub(super) struct Bridge {
    endpoint: String,
    credential: String,
    tools: Arc<Tools>,
}

impl Bridge {
    pub(super) fn start(
        tools: Arc<Tools>,
        events: SyncSender<AssistantEvent>,
    ) -> Result<Self, String> {
        let listener =
            TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|error| error.to_string())?;
        listener
            .set_nonblocking(true)
            .map_err(|error| error.to_string())?;
        let endpoint = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .to_string();
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).map_err(|_| "Cannot generate a local tool credential")?;
        let credential = secret
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let listener_tools = tools.clone();
        let credential_copy = credential.clone();
        let connected = Arc::new(AtomicBool::new(false));
        thread::spawn(move || {
            while !listener_tools.closed() {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let Some((id, turn)) = listener_tools.register(&stream) else {
                            continue;
                        };

                        let request = Request {
                            tools: listener_tools.clone(),
                            credential: credential_copy.clone(),
                            events: events.clone(),
                            connected: connected.clone(),
                            id,
                            turn,
                        };
                        thread::spawn(move || {
                            let _ = request.serve(stream);
                        });
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(15));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            endpoint,
            credential,
            tools,
        })
    }

    #[cfg(test)]
    pub(super) fn disconnected(
        tools: Arc<Tools>,
        _: SyncSender<AssistantEvent>,
    ) -> Result<Self, String> {
        // Fake ACP agents exercise startup and presentation without opening a socket.
        Ok(Self {
            endpoint: "127.0.0.1:1".into(),
            credential: "test-credential".into(),
            tools,
        })
    }

    #[cfg(test)]
    pub(super) fn fixture_tool(
        &self,
        credential: &str,
        events: SyncSender<AssistantEvent>,
        params: &Value,
    ) -> Option<Result<Value, commands::Finding>> {
        let turn = self.tools.state().turn?;
        let request = Request {
            tools: self.tools.clone(),
            credential: self.credential.clone(),
            events,
            connected: Arc::new(AtomicBool::new(false)),
            id: u64::MAX,
            turn: Some(turn),
        };
        request
            .authenticated(credential)
            .then(|| request.call(turn, params))
    }

    pub(super) fn environment(&self, directory: &Path) -> Result<Value, String> {
        let path = directory.join("mgb-ops-mcp.json");
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(&path).map_err(|error| error.to_string())?;
        // Some adapters forward the MCP server configuration as command-line arguments.
        // It carries a file path so the token itself stays in the private bootstrap file.
        let bootstrap = Bootstrap {
            endpoint: self.endpoint.clone(),
            credential: self.credential.clone(),
        };
        serde_json::to_writer(file, &bootstrap).map_err(|error| error.to_string())?;
        Ok(serde_json::json!([{"name":BOOTSTRAP,"value":path}]))
    }

    #[cfg(test)]
    pub(super) fn connection(&self) -> (&str, &str) {
        (&self.endpoint, &self.credential)
    }

    pub(super) fn redact(&self, content: &str) -> String {
        content.replace(&self.credential, "[redacted]")
    }

    pub(super) fn redact_value(&self, value: &mut Value) {
        match value {
            Value::String(text) => *text = self.redact(text),
            Value::Array(items) => items.iter_mut().for_each(|item| self.redact_value(item)),
            Value::Object(items) => items.values_mut().for_each(|item| self.redact_value(item)),
            _ => {}
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.tools.close();
    }
}

struct Request {
    tools: Arc<Tools>,
    credential: String,
    events: SyncSender<AssistantEvent>,
    connected: Arc<AtomicBool>,
    id: u64,
    turn: Option<u64>,
}

impl Drop for Request {
    fn drop(&mut self) {
        self.tools.state().sockets.remove(&self.id);
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authenticated {
    credential: String,
    message: Value,
}

impl Request {
    fn authenticated(&self, credential: &str) -> bool {
        self.credential == credential && !self.tools.closed()
    }

    fn serve(&self, mut stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))?;
        let Some(line) = bounded_line(&mut BufReader::new(&stream))? else {
            return Ok(());
        };
        let Authenticated {
            credential,
            message,
        } = serde_json::from_slice(&line)?;
        if !self.authenticated(&credential) {
            return Ok(());
        }
        let Some(id) = message.get("id") else {
            return Ok(());
        };
        let method = message["method"].as_str().unwrap_or_default();
        let params = &message["params"];
        let response = match method {
            "initialize" => {
                if !self.connected.swap(true, Ordering::AcqRel) {
                    let _ = self.events.try_send(AssistantEvent::ToolConnected);
                }
                let supported = ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"];
                let version = params["protocolVersion"]
                    .as_str()
                    .filter(|version| supported.contains(version))
                    .unwrap_or(supported[0]);
                rpc_result(
                    id,
                    serde_json::json!({
                        "protocolVersion":version, "capabilities":{"tools":{}},
                        "serverInfo":{"name":"mgb-ops","version":super::VERSION}
                    }),
                )
            }
            "ping" => rpc_result(id, serde_json::json!({})),
            "tools/list" => rpc_result(
                id,
                serde_json::json!({"tools":[
                    {"name":"search_commands","description":"Find MGB Ops commands, IDs and typed argument schemas",
                        "inputSchema":{"type":"object","properties":{"query":{"type":"string"}},"required":["query"],"additionalProperties":false}},
                    {"name":"run_command","description":"Invoke an MGB Ops command against the selected workspace session",
                        "inputSchema":{"type":"object","properties":{"id":{"type":"string"},"arguments":{"type":"object"}},
                            "required":["id","arguments"],"additionalProperties":false}}
                ]}),
            ),
            "tools/call" => {
                let Some(turn) = self.turn else {
                    write_message(&mut stream, &rpc_error(id, -32000, "No prompt is active"))?;
                    return Ok(());
                };
                if self.tools.state().turn != Some(turn) {
                    return Ok(());
                }
                let result = self.call(turn, params);
                // Finishing a turn shuts down its sockets, including writes in progress.
                if self.tools.state().turn != Some(turn) {
                    return Ok(());
                }
                let response = tool_response(id, result);
                write_message(&mut stream, &response)?;
                return Ok(());
            }
            _ => rpc_error(id, -32601, "Unknown MCP method"),
        };
        if !self.tools.closed() {
            write_message(&mut stream, &response)?;
        }
        Ok(())
    }

    fn call(&self, turn: u64, params: &Value) -> Result<Value, commands::Finding> {
        let invalid = |message| commands::Finding::new("invalid_arguments", message);
        match params["name"].as_str() {
            Some("search_commands") => {
                let query = params["arguments"]["query"]
                    .as_str()
                    .ok_or_else(|| invalid("Expected a search query string"))?;
                Ok(commands::search(query))
            }
            Some("run_command") => {
                let args = &params["arguments"];
                let id = args["id"]
                    .as_str()
                    .ok_or_else(|| invalid("Expected a command ID"))?;
                if !args["arguments"].is_object() {
                    return Err(invalid("Expected a command arguments object"));
                }
                if id == "draft.propose_edit" && self.tools.state().proposal.is_some() {
                    return Err(commands::Finding::new(
                        "review_pending",
                        "Ask the operator to apply or reject the pending review first",
                    ));
                }
                match commands::run(&self.tools.session, id, args["arguments"].clone())? {
                    commands::Output::Data(result) => Ok(result),
                    commands::Output::Proposal(proposal) => {
                        let mut state = self.tools.state();
                        if state.turn != Some(turn) || state.closed {
                            return Err(commands::Finding::new(
                                "cancelled",
                                "The prompt was cancelled",
                            ));
                        }
                        if state.proposal.is_some() {
                            return Err(commands::Finding::new(
                                "review_pending",
                                "Ask the operator to apply or reject the pending review first",
                            ));
                        }
                        if self.tools.session.draft_version().as_ref() != Some(&proposal.version) {
                            return Err(commands::Finding::new(
                                "stale_draft",
                                "Read the current draft and submit a new proposal",
                            ));
                        }
                        self.events
                            .try_send(AssistantEvent::ReviewStaged)
                            .map_err(|_| {
                                commands::Finding::new(
                                    "host_unavailable",
                                    "The operator interface is unavailable",
                                )
                            })?;
                        state.proposal = Some((turn, proposal));
                        Ok(
                            serde_json::json!({"status":"staged","message":"The operator can apply or reject the review in MGB Ops"}),
                        )
                    }
                }
            }
            _ => Err(invalid("Use search_commands or run_command")),
        }
    }
}

fn tool_response(id: &Value, result: Result<Value, commands::Finding>) -> Value {
    let (data, error) = match result {
        Ok(value) => (value, false),
        Err(finding) => (serde_json::json!({"findings":[finding]}), true),
    };
    rpc_result(
        id,
        serde_json::json!({
            "content":[{"type":"text","text":data.to_string()}], "isError":error
        }),
    )
}

pub(super) fn rpc_result(id: &Value, result: Value) -> Value {
    serde_json::json!({"jsonrpc":"2.0","id":id,"result":result})
}

pub(super) fn rpc_error(id: &Value, code: i32, message: &str) -> Value {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

pub(super) fn write_message(writer: &mut impl Write, message: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(message)?;
    if bytes.len() + 1 > super::MAX_MESSAGE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Message exceeds size limit",
        ));
    }
    writer.write_all(&bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Bootstrap {
    endpoint: String,
    credential: String,
}

/// Run the stdio side of the conversation-local MCP bridge.
pub fn assistant_mcp_stdio() -> Result<(), String> {
    let path = std::env::var_os(BOOTSTRAP).ok_or("Missing local MCP bootstrap file")?;
    let file = File::open(path).map_err(|_| "Cannot open local MCP bootstrap file")?;
    let Bootstrap {
        endpoint,
        credential,
    } = serde_json::from_reader(file).map_err(|_| "Invalid local MCP bootstrap file")?;
    let endpoint: std::net::SocketAddr = endpoint
        .parse::<std::net::SocketAddr>()
        .ok()
        .filter(std::net::SocketAddr::is_ipv4)
        .filter(|address| address.ip() == Ipv4Addr::LOCALHOST)
        .ok_or("Invalid local MCP endpoint")?;
    let input = io::stdin();
    let mut input = input.lock();
    let output = io::stdout();
    let mut output = output.lock();
    while let Some(line) = bounded_line(&mut input).map_err(|error| error.to_string())? {
        let message: Value = match serde_json::from_slice(&line) {
            Ok(message) => message,
            Err(_) => {
                write_message(
                    &mut output,
                    &rpc_error(&Value::Null, -32700, "Invalid MCP JSON"),
                )
                .map_err(|error| error.to_string())?;
                continue;
            }
        };
        let Some(id) = message.get("id") else {
            continue;
        };
        let response = (|| -> io::Result<Option<Vec<u8>>> {
            let mut connection = TcpStream::connect_timeout(&endpoint, IO_TIMEOUT)?;
            connection.set_read_timeout(Some(Duration::from_secs(30)))?;
            connection.set_write_timeout(Some(IO_TIMEOUT))?;
            write_message(
                &mut connection,
                &serde_json::json!({"credential":credential,"message":message}),
            )?;
            bounded_line(&mut BufReader::new(connection))
        })();
        match response {
            Ok(Some(response)) => output.write_all(&response).and_then(|()| output.flush()),
            _ => write_message(
                &mut output,
                &rpc_error(
                    id,
                    -32000,
                    "MGB Ops tool request ended or the connection is unavailable",
                ),
            ),
        }
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}
