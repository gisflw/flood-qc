use super::super::Session;
use super::{
    AgentPreset, AssistantEvent, AssistantFailure, AssistantOptions, AssistantPermission,
    AssistantPermissionKind, AssistantPermissionOption, AssistantStage, AssistantStopReason,
    Control, Event, HostCommand, Shared, mcp, wire,
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, BufReader},
    process::{ChildStdin, ChildStdout, Command},
    sync::{mpsc, watch},
    time::{Instant, sleep_until},
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(35);
const TURN_TIMEOUT: Duration = Duration::from_secs(300);
const STOP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_PERMISSIONS: usize = 8;
const MAX_OPTIONS: usize = 16;
const MAX_TOOL_DETAILS: usize = 16;

pub(super) struct Channels {
    pub(super) control: watch::Receiver<Control>,
    pub(super) commands: mpsc::Receiver<HostCommand>,
    pub(super) events: mpsc::Sender<Event>,
    pub(super) shared: Arc<Shared>,
}

pub(super) async fn run(
    agent: AgentPreset,
    options: AssistantOptions,
    session: Session,
    channels: Channels,
) -> Result<(), AssistantFailure> {
    let failure = |stage, message| AssistantFailure::new(agent, stage, message);
    let agent_program = find_executable(agent, options.executable.as_deref()).ok_or_else(|| {
        AssistantFailure::new(
            agent,
            AssistantStage::Executable,
            format!("Could not find executable {}", agent.executable()),
        )
    })?;
    let working = WorkingDirectory::new(options.working_directory).map_err(|_| {
        failure(
            AssistantStage::Subprocess,
            "Could not open the separate agent working directory",
        )
    })?;
    let bridge = mcp::Bridge::bind().await.map_err(|_| {
        failure(
            AssistantStage::ToolConnection,
            "Could not open the loopback MCP endpoint",
        )
    })?;
    let helper = options
        .helper_executable
        .or_else(|| std::env::current_exe().ok())
        .and_then(executable)
        .ok_or_else(|| {
            failure(
                AssistantStage::ToolConnection,
                "Could not locate the MCP stdio helper",
            )
        })?;
    let declaration = bridge.declaration(&helper).map_err(|_| {
        failure(
            AssistantStage::ToolConnection,
            "Could not declare the MCP stdio helper",
        )
    })?;
    let mut command = Command::new(agent_program);
    if agent == AgentPreset::Gemini {
        command.arg("--acp");
    }
    command
        .current_dir(&working.path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(target_os = "linux")]
    command.process_group(0);
    let mut child = command.spawn().map_err(|_| {
        failure(
            AssistantStage::Subprocess,
            "Could not start the agent executable",
        )
    })?;
    let process_id = child.id();
    let auth_hint = Arc::new(AtomicBool::new(false));
    let mut stderr = tokio::spawn(scan_stderr(child.stderr.take().unwrap(), auth_hint.clone()));
    let connected = Arc::new(AtomicBool::new(false));
    let bridge_task = tokio::spawn(bridge.serve(mcp::Gateway {
        shared: channels.shared.clone(),
        session,
        events: channels.events.clone(),
        control: channels.control.clone(),
        connected: connected.clone(),
    }));
    let mut client = Client {
        agent,
        channels,
        auth_hint: auth_hint.clone(),
        connected,
        reader: wire::Reader::default(),
        output: BufReader::new(child.stdout.take().unwrap()),
        input: child.stdin.take().unwrap(),
        session: None,
        turn: None,
        next_permission: 1,
        reported_mcp: false,
    };
    let result = client.converse(&working.path, declaration).await;
    client.channels.shared.close();
    bridge_task.abort();
    drop(client);
    kill_group(process_id);
    let _ = child.start_kill();
    let reaped = child.wait().await;
    if tokio::time::timeout(Duration::from_millis(100), &mut stderr)
        .await
        .is_err()
    {
        stderr.abort();
    }
    if result
        .as_ref()
        .is_err_and(|error| error.stage == AssistantStage::Subprocess)
        && auth_hint.load(Ordering::Acquire)
    {
        return Err(failure(
            AssistantStage::SignIn,
            "Sign in using the installed agent before retrying",
        ));
    }
    if result.is_ok() && reaped.is_err() {
        return Err(failure(
            AssistantStage::Subprocess,
            "Could not reap the agent process",
        ));
    }
    result
}

fn find_executable(agent: AgentPreset, supplied: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = supplied {
        return executable(path.to_path_buf());
    }
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths).find_map(|directory| {
        let path = directory.join(agent.executable());
        #[cfg(windows)]
        return ["exe", "cmd", "bat"]
            .into_iter()
            .find_map(|extension| executable(path.with_extension(extension)));
        #[cfg(not(windows))]
        executable(path)
    })
}

fn executable(path: PathBuf) -> Option<PathBuf> {
    let metadata = path.metadata().ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    if !metadata.is_file() {
        return None;
    }
    std::path::absolute(path).ok()
}

#[cfg(target_os = "linux")]
fn kill_group(process: Option<u32>) {
    if let Some(pid) = process
        .and_then(|id| i32::try_from(id).ok())
        .and_then(rustix::process::Pid::from_raw)
    {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
}

#[cfg(not(target_os = "linux"))]
fn kill_group(_: Option<u32>) {}

struct WorkingDirectory {
    path: PathBuf,
    owned: bool,
}

impl WorkingDirectory {
    fn new(supplied: Option<PathBuf>) -> io::Result<Self> {
        if let Some(path) = supplied {
            return Ok(Self {
                path: path.canonicalize()?,
                owned: false,
            });
        }
        let mut random = [0; 16];
        getrandom::fill(&mut random)
            .map_err(|_| io::Error::other("Could not choose an agent directory"))?;
        let path = std::env::temp_dir().join(format!(
            "mgb-ops-agent-{:032x}",
            u128::from_le_bytes(random)
        ));
        let mut builder = std::fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        Ok(Self { path, owned: true })
    }
}

impl Drop for WorkingDirectory {
    fn drop(&mut self) {
        if self.owned {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

struct Client {
    agent: AgentPreset,
    channels: Channels,
    auth_hint: Arc<AtomicBool>,
    connected: Arc<AtomicBool>,
    reader: wire::Reader,
    output: BufReader<ChildStdout>,
    input: ChildStdin,
    session: Option<String>,
    turn: Option<Turn>,
    next_permission: u64,
    reported_mcp: bool,
}

struct Turn {
    id: u64,
    deadline: Instant,
    stopping: bool,
    permissions: Vec<PendingPermission>,
    tool_details: VecDeque<Value>,
}

impl Turn {
    fn remember_tool(&mut self, update: &Value) {
        let Some(id) = update["toolCallId"].as_str() else {
            return;
        };
        let mut details = self
            .tool_details
            .iter()
            .position(|tool| tool["toolCallId"] == id)
            .and_then(|index| self.tool_details.remove(index))
            .unwrap_or_else(|| json!({"toolCallId": id}));
        extend_tool_details(&mut details, update);
        if self.tool_details.len() == MAX_TOOL_DETAILS {
            self.tool_details.pop_front();
        }
        self.tool_details.push_back(details);
    }

    fn permission_details(&self, update: &Value) -> Value {
        let Some(id) = update["toolCallId"].as_str() else {
            return update.clone();
        };
        let mut details = self
            .tool_details
            .iter()
            .find(|tool| tool["toolCallId"] == id)
            .cloned()
            .unwrap_or_else(|| json!({"toolCallId": id}));
        if let Some(fields) = update.as_object() {
            for (field, value) in fields.iter().filter(|(_, value)| !value.is_null()) {
                details[field] = value.clone();
            }
        }
        details
    }
}

fn extend_tool_details(details: &mut Value, update: &Value) {
    for field in [
        "title",
        "kind",
        "locations",
        "content",
        "rawInput",
        "status",
    ] {
        if let Some(value) = update.get(field).filter(|value| !value.is_null()) {
            details[field] = value.clone();
        }
    }
}

struct PendingPermission {
    request: u64,
    rpc_id: Value,
    options: Vec<AssistantPermissionOption>,
}

impl Client {
    async fn converse(
        &mut self,
        working: &Path,
        declaration: Value,
    ) -> Result<(), AssistantFailure> {
        if self.channels.control.borrow().closed {
            return Ok(());
        }
        self.send(&wire::request(
            Some(1),
            "initialize",
            json!({
                "protocolVersion": 1, "clientCapabilities": {},
                "clientInfo": {"name": "mgb-ops", "version": env!("CARGO_PKG_VERSION")}
            }),
        ))
        .await?;
        let Some(initialized) = self.setup_response(1, "initialize").await? else {
            return Ok(());
        };
        if initialized["protocolVersion"].as_u64() != Some(1) {
            return Err(self.failure(
                AssistantStage::Protocol,
                "The agent did not negotiate ACP v1",
            ));
        }
        let working = working.to_str().ok_or_else(|| {
            self.failure(
                AssistantStage::Protocol,
                "Agent working directory is not UTF-8",
            )
        })?;
        self.send(&wire::request(
            Some(2),
            "session/new",
            json!({"cwd": working, "mcpServers": [declaration]}),
        ))
        .await?;
        let Some(session) = self.setup_response(2, "session/new").await? else {
            return Ok(());
        };
        self.session = Some(
            session["sessionId"]
                .as_str()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| {
                    self.failure(
                        AssistantStage::Protocol,
                        "ACP session/new returned no session ID",
                    )
                })?
                .into(),
        );
        self.emit(None, AssistantEvent::Ready)?;

        loop {
            if self.channels.control.borrow().closed {
                return Ok(());
            }
            let deadline = self
                .turn
                .as_ref()
                .map_or_else(|| Instant::now() + TURN_TIMEOUT, |turn| turn.deadline);
            tokio::select! {
                biased;
                changed = self.channels.control.changed() => if changed.is_err() || self.control().await? {
                    return Ok(());
                },
                _ = sleep_until(deadline), if self.turn.is_some() => {
                    let message = if self.turn.as_ref().is_some_and(|turn| turn.stopping) {
                        "The agent did not acknowledge ACP cancellation"
                    } else {
                        "The agent did not finish the turn in time"
                    };
                    return Err(self.failure(AssistantStage::Timeout, message));
                },
                command = self.channels.commands.recv() => match command {
                    Some(command) => self.host_command(command).await?,
                    None => return Ok(()),
                },
                message = self.reader.next(&mut self.output) => {
                    let message = self.incoming(message)?;
                    self.receive(message).await?;
                },
            }
        }
    }

    async fn setup_response(
        &mut self,
        id: u64,
        method: &'static str,
    ) -> Result<Option<Value>, AssistantFailure> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        loop {
            tokio::select! {
                biased;
                changed = self.channels.control.changed() => {
                    if changed.is_err() || self.channels.control.borrow().closed {
                        return Ok(None);
                    }
                },
                _ = sleep_until(deadline) => {
                    return Err(AssistantFailure::new(self.agent, AssistantStage::Timeout, format!("ACP {method} timed out")));
                },
                message = self.reader.next(&mut self.output) => {
                    let message = self.incoming(message)?;
                    if message["method"].is_string() {
                        self.receive(message).await?;
                    } else if message["id"].as_u64() == Some(id) {
                        if let Some(error) = message.get("error") {
                            return Err(self.rpc_failure(error, method));
                        }
                        let result = message.get("result").filter(|result| result.is_object()).cloned()
                            .ok_or_else(|| self.failure(AssistantStage::Protocol, "Malformed ACP response"))?;
                        return Ok(Some(result));
                    } else {
                        return Err(self.failure(AssistantStage::Protocol, "Unexpected ACP response ID"));
                    }
                }
            }
        }
    }

    async fn control(&mut self) -> Result<bool, AssistantFailure> {
        let control = *self.channels.control.borrow();
        if control.closed {
            return Ok(true);
        }
        if self
            .turn
            .as_ref()
            .is_some_and(|turn| turn.id <= control.cancelled && !turn.stopping)
        {
            self.cancel().await?;
        }
        Ok(false)
    }

    async fn host_command(&mut self, command: HostCommand) -> Result<(), AssistantFailure> {
        match command {
            HostCommand::Prompt { turn, text } => {
                if !self.channels.shared.begin(turn) {
                    return self.emit(
                        Some(turn),
                        AssistantEvent::Finished(AssistantStopReason::Cancelled),
                    );
                }
                self.turn = Some(Turn {
                    id: turn,
                    deadline: Instant::now() + TURN_TIMEOUT,
                    stopping: false,
                    permissions: Vec::new(),
                    tool_details: VecDeque::new(),
                });
                let session_id = self.session.as_deref().unwrap();
                self.send(&wire::request(
                    Some(turn + 2),
                    "session/prompt",
                    json!({
                        "sessionId": session_id, "prompt": [{"type": "text", "text": text}]
                    }),
                ))
                .await
            }
            HostCommand::Permission {
                turn,
                request,
                choice,
            } => {
                let Some(current) = self.turn.as_mut().filter(|current| {
                    !current.stopping
                        && current.id == turn
                        && self.channels.shared.turn() == Some(turn)
                }) else {
                    return Ok(());
                };
                let Some(index) = current.permissions.iter().position(|pending| {
                    pending.request == request
                        && pending.options.iter().any(|option| option.id == choice)
                }) else {
                    return Ok(());
                };
                let pending = current.permissions.remove(index);
                self.send(&wire::response(
                    pending.rpc_id,
                    json!({"outcome": {"outcome": "selected", "optionId": choice}}),
                ))
                .await
            }
        }
    }

    async fn receive(&mut self, message: Value) -> Result<(), AssistantFailure> {
        match message["method"].as_str() {
            Some("session/update") => self.update(&message["params"]),
            Some("session/request_permission") => self.permission(&message).await,
            Some(_method) => {
                if let Some(id) = message.get("id") {
                    self.send(&wire::error(
                        id.clone(),
                        -32601,
                        "Unsupported ACP client method",
                    ))
                    .await?;
                }
                Ok(())
            }
            None => self.turned(message).await,
        }
    }

    fn update(&mut self, params: &Value) -> Result<(), AssistantFailure> {
        let Some(turn) = self.current_turn(params) else {
            return Ok(());
        };
        let update = &params["update"];
        let text = || {
            update
                .pointer("/content/text")
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        let event = match update["sessionUpdate"].as_str() {
            Some("agent_message_chunk") => text().map(AssistantEvent::Text),
            Some("agent_thought_chunk") => text().map(AssistantEvent::Thought),
            Some("tool_call" | "tool_call_update") => {
                self.turn.as_mut().unwrap().remember_tool(update);
                Some(AssistantEvent::ToolStatus(update.clone()))
            }
            _ => None,
        };
        event.map_or(Ok(()), |event| self.emit(Some(turn), event))
    }

    fn current_turn(&self, params: &Value) -> Option<u64> {
        self.turn
            .as_ref()
            .filter(|turn| {
                !turn.stopping
                    && self.session.as_deref() == params["sessionId"].as_str()
                    && self.channels.shared.turn() == Some(turn.id)
            })
            .map(|turn| turn.id)
    }

    async fn permission(&mut self, message: &Value) -> Result<(), AssistantFailure> {
        let id = message
            .get("id")
            .filter(|id| id.is_string() || id.is_number())
            .ok_or_else(|| {
                self.failure(
                    AssistantStage::Protocol,
                    "Permission request has an invalid ACP ID",
                )
            })?
            .clone();
        let parameters = &message["params"];
        let Some(turn_id) = self.current_turn(parameters) else {
            return self.send(&permission_cancelled(id)).await;
        };
        let options = parameters["options"]
            .as_array()
            .filter(|options| options.len() <= MAX_OPTIONS)
            .map_or_else(Vec::new, |options| {
                options.iter().filter_map(permission_option).collect()
            });
        let details = self
            .turn
            .as_ref()
            .unwrap()
            .permission_details(&parameters["toolCall"]);
        if !options
            .iter()
            .any(|option| option.kind == AssistantPermissionKind::RejectOnce)
        {
            self.emit(Some(turn_id), AssistantEvent::ToolStatus(details))?;
            self.emit(
                Some(turn_id),
                AssistantEvent::Notice(
                    "The agent offered no one-time deny choice. The turn was stopped.".into(),
                ),
            )?;
            self.send(&permission_cancelled(id)).await?;
            return self.cancel().await;
        }
        let turn = self.turn.as_mut().unwrap();
        if turn.permissions.len() >= MAX_PERMISSIONS
            || turn.permissions.iter().any(|pending| pending.rpc_id == id)
        {
            return Err(self.failure(
                AssistantStage::Protocol,
                "Too many or duplicate ACP permission requests",
            ));
        }
        let request = self.next_permission;
        self.next_permission += 1;
        turn.permissions.push(PendingPermission {
            request,
            rpc_id: id,
            options: options.clone(),
        });
        self.emit(
            Some(turn_id),
            AssistantEvent::Permission(AssistantPermission {
                request,
                tool_call: details,
                options,
            }),
        )
    }

    async fn cancel(&mut self) -> Result<(), AssistantFailure> {
        let turn = self.turn.as_mut().unwrap();
        turn.stopping = true;
        turn.deadline = Instant::now() + STOP_TIMEOUT;
        let permissions = std::mem::take(&mut turn.permissions);
        self.channels.shared.finish(turn.id, false);
        let session = self.session.as_deref().unwrap();
        self.send(&wire::request(
            None,
            "session/cancel",
            json!({"sessionId": session}),
        ))
        .await?;
        for pending in permissions {
            self.send(&permission_cancelled(pending.rpc_id)).await?;
        }
        Ok(())
    }

    async fn turned(&mut self, message: Value) -> Result<(), AssistantFailure> {
        let Some(turn) = self
            .turn
            .take()
            .filter(|turn| message["id"].as_u64() == Some(turn.id + 2))
        else {
            return Err(self.failure(AssistantStage::Protocol, "Unexpected ACP response ID"));
        };
        let reason = if let Some(error) = message.get("error") {
            if turn.stopping {
                return Err(self.failure(
                    AssistantStage::Protocol,
                    "The agent did not return the ACP cancelled stop reason",
                ));
            }
            let failure = self.rpc_failure(error, "session/prompt");
            if failure.stage == AssistantStage::SignIn {
                return Err(failure);
            }
            self.emit(Some(turn.id), AssistantEvent::Notice(failure.to_string()))?;
            AssistantStopReason::Failed
        } else {
            match message
                .pointer("/result/stopReason")
                .and_then(Value::as_str)
            {
                Some("cancelled") => AssistantStopReason::Cancelled,
                _ if turn.stopping => {
                    return Err(self.failure(
                        AssistantStage::Protocol,
                        "The agent did not return the ACP cancelled stop reason",
                    ));
                }
                Some("end_turn") => AssistantStopReason::EndTurn,
                Some("refusal") => AssistantStopReason::Refused,
                Some("max_tokens" | "max_turn_requests") => AssistantStopReason::Limit,
                Some(_) => AssistantStopReason::Failed,
                None => {
                    return Err(self.failure(
                        AssistantStage::Protocol,
                        "ACP prompt response omitted stopReason",
                    ));
                }
            }
        };
        let keep_review = matches!(
            reason,
            AssistantStopReason::EndTurn | AssistantStopReason::Limit
        );
        self.channels.shared.finish(turn.id, keep_review);
        for pending in turn.permissions {
            self.send(&permission_cancelled(pending.rpc_id)).await?;
        }
        if !self.connected.load(Ordering::Acquire)
            && !self.reported_mcp
            && reason == AssistantStopReason::EndTurn
        {
            self.reported_mcp = true;
            self.emit(
                Some(turn.id),
                AssistantEvent::Notice(
                    self.failure(
                        AssistantStage::ToolConnection,
                        "The turn ended before the agent established the MCP connection",
                    )
                    .to_string(),
                ),
            )?;
        }
        self.emit(Some(turn.id), AssistantEvent::Finished(reason))
    }

    fn incoming(&self, message: io::Result<Option<Value>>) -> Result<Value, AssistantFailure> {
        match message {
            Ok(Some(message)) => Ok(message),
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                Err(self.failure(AssistantStage::Protocol, error.to_string()))
            }
            _ if self.auth_hint.load(Ordering::Acquire) => Err(self.failure(
                AssistantStage::SignIn,
                "Sign in using the installed agent before retrying",
            )),
            _ => Err(self.failure(
                AssistantStage::Subprocess,
                "The agent closed its ACP output",
            )),
        }
    }

    fn rpc_failure(&self, error: &Value, method: &str) -> AssistantFailure {
        let details = error.to_string().to_ascii_lowercase();
        if error["code"].as_i64() == Some(-32000) || auth_required(&details) {
            return self.failure(
                AssistantStage::SignIn,
                "Sign in using the installed agent before retrying",
            );
        }
        if details.contains("mcp") {
            return AssistantFailure::new(
                self.agent,
                AssistantStage::ToolConnection,
                format!("The agent reported an MCP error during ACP {method}"),
            );
        }
        AssistantFailure::new(
            self.agent,
            AssistantStage::Protocol,
            format!("The agent rejected ACP {method}"),
        )
    }

    async fn send(&mut self, message: &Value) -> Result<(), AssistantFailure> {
        wire::send(&mut self.input, message).await.map_err(|error| {
            if error.kind() == io::ErrorKind::TimedOut {
                self.failure(
                    AssistantStage::Timeout,
                    "The agent stopped reading its ACP input",
                )
            } else {
                self.failure(
                    AssistantStage::Subprocess,
                    "Could not write to the agent ACP input",
                )
            }
        })
    }

    fn emit(&self, turn: Option<u64>, value: AssistantEvent) -> Result<(), AssistantFailure> {
        self.channels
            .events
            .try_send(Event { turn, value })
            .map_err(|_| {
                self.failure(
                    AssistantStage::Protocol,
                    "The assistant event queue reached its limit",
                )
            })
    }

    fn failure(&self, stage: AssistantStage, message: impl Into<String>) -> AssistantFailure {
        AssistantFailure::new(self.agent, stage, message)
    }
}

fn permission_option(value: &Value) -> Option<AssistantPermissionOption> {
    let kind = match value["kind"].as_str()? {
        "allow_once" => AssistantPermissionKind::AllowOnce,
        "reject_once" => AssistantPermissionKind::RejectOnce,
        _ => return None,
    };
    Some(AssistantPermissionOption {
        id: value["optionId"].as_str()?.into(),
        name: value["name"].as_str()?.into(),
        kind,
    })
}

fn permission_cancelled(id: Value) -> Value {
    wire::response(id, json!({"outcome": {"outcome": "cancelled"}}))
}

async fn scan_stderr(mut error: impl AsyncRead + Unpin, auth_hint: Arc<AtomicBool>) {
    let mut buf = [0; 4096];
    let mut tail = String::new();
    while let Ok(size @ 1..) = error.read(&mut buf).await {
        tail.push_str(&String::from_utf8_lossy(&buf[..size]));
        if auth_required(&tail.to_ascii_lowercase()) {
            auth_hint.store(true, Ordering::Release);
            tail.clear();
        }
        if tail.len() > 512 {
            let keep_from = tail.ceil_char_boundary(tail.len() - 512);
            tail.drain(..keep_from);
        }
    }
}

fn auth_required(details: &str) -> bool {
    [
        "authentication required",
        "not authenticated",
        "not signed in",
        "not logged in",
        "login required",
        "please sign in",
        "run /login",
        "api key",
        "unauthorized",
        "credential required",
    ]
    .iter()
    .any(|hint| details.contains(hint))
        || (details.contains("api_key")
            && ["must specify", "not set", "missing", "required"]
                .iter()
                .any(|hint| details.contains(hint)))
}
