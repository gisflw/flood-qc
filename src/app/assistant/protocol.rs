use super::{
    AssistantEvent, AssistantStage, Events, Input, PermissionOption, QUEUE, SettingKind, Tools,
    bounded_line,
    bridge::{Bridge, rpc_error, rpc_result, write_message},
    settings::Configuration,
};
use serde_json::Value;
use std::{
    ffi::{OsStr, OsString},
    io::BufReader,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(3);
const SETTINGS_TIMEOUT: Duration = if cfg!(test) {
    Duration::from_secs(1)
} else {
    STARTUP_TIMEOUT
};

pub(super) struct Launch {
    pub(super) executable: OsString,
    pub(super) args: Vec<OsString>,
}

fn discover(executable: &OsStr) -> Option<PathBuf> {
    if Path::new(executable).components().count() > 1 {
        return Path::new(executable)
            .canonicalize()
            .ok()
            .filter(|path| path.is_file());
    }
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(executable);
        if candidate.is_file() {
            // Preserve the executable name when it is a symlink or shim.
            return std::path::absolute(candidate).ok();
        }
        #[cfg(windows)]
        for extension in ["exe", "cmd", "bat"] {
            let candidate = candidate.with_extension(extension);
            if candidate.is_file() {
                return std::path::absolute(candidate).ok();
            }
        }
        None
    })
}

struct Process(Child);

impl Drop for Process {
    fn drop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(pid) = i32::try_from(self.0.id())
            .ok()
            .and_then(rustix::process::Pid::from_raw)
        {
            let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

struct Failure {
    stage: AssistantStage,
    message: String,
}
type Result<T> = std::result::Result<T, Failure>;

impl Failure {
    fn new(stage: AssistantStage, message: impl Into<String>) -> Self {
        Self {
            stage,
            message: message.into(),
        }
    }
}

pub(super) fn supervise(
    launch: Launch,
    helper: PathBuf,
    tools: Arc<Tools>,
    input: Receiver<Input>,
    events: Events,
    connect: impl FnOnce(Arc<Tools>, Events) -> std::result::Result<Bridge, String>,
) {
    let result = run(&launch, &helper, tools.clone(), input, &events, connect);
    tools.close();
    if let Err(failure) = result {
        let _ = events.try_send(AssistantEvent::Failed {
            stage: failure.stage,
            message: failure.message,
            closed: true,
        });
    }
}

fn run(
    launch: &Launch,
    helper: &Path,
    tools: Arc<Tools>,
    input: Receiver<Input>,
    events: &Events,
    connect: impl FnOnce(Arc<Tools>, Events) -> std::result::Result<Bridge, String>,
) -> Result<()> {
    let executable = discover(&launch.executable).ok_or_else(|| {
        Failure::new(
            AssistantStage::Discovery,
            format!(
                "Install {} and put it on PATH",
                launch.executable.to_string_lossy()
            ),
        )
    })?;
    let mut directory = tempfile::Builder::new();
    directory.prefix("mgb-ops-agent-");
    #[cfg(test)]
    let cwd = directory.tempdir_in("runs");
    #[cfg(not(test))]
    let cwd = directory.tempdir();
    let cwd = cwd.map_err(|error| Failure::new(AssistantStage::Process, error.to_string()))?;
    let cwd_path = cwd
        .path()
        .canonicalize()
        .map_err(|error| Failure::new(AssistantStage::Process, error.to_string()))?;
    let bridge = connect(tools.clone(), events.clone())
        .map_err(|message| Failure::new(AssistantStage::ToolConnection, message))?;
    let bridge_environment = bridge
        .environment(&cwd_path)
        .map_err(|message| Failure::new(AssistantStage::ToolConnection, message))?;
    let helper = helper
        .to_str()
        .filter(|_| helper.is_file() && helper.is_absolute())
        .ok_or_else(|| {
            Failure::new(
                AssistantStage::ToolConnection,
                "The MCP helper needs an absolute executable path",
            )
        })?;
    let mut command = Command::new(executable);
    command
        .args(&launch.args)
        .current_dir(&cwd_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = Process(
        command
            .spawn()
            .map_err(|error| Failure::new(AssistantStage::Process, error.to_string()))?,
    );
    let stdout = child.0.stdout.take().unwrap();
    let mut stdin = child.0.stdin.take().unwrap();

    let (received, wire) = sync_channel(QUEUE);
    thread::spawn(move || {
        let mut stdout = BufReader::new(stdout);
        loop {
            let message = match bounded_line(&mut stdout) {
                Ok(Some(line)) => serde_json::from_slice::<Value>(&line).map_err(|_| {
                    Failure::new(AssistantStage::Protocol, "Agent emitted malformed ACP JSON")
                }),
                Ok(None) => break,
                Err(error) => Err(Failure::new(AssistantStage::Protocol, error.to_string())),
            };
            let terminal = message.is_err();
            if received.send(message).is_err() || terminal {
                break;
            }
        }
    });
    let (output, writes) = sync_channel(16);
    thread::spawn(move || {
        while let Ok(message) = writes.recv() {
            if write_message(&mut stdin, &message).is_err() {
                break;
            }
        }
    });

    let mut client = Client {
        events,
        output,
        bridge: &bridge,
        bridge_environment,
        tools: &tools,
        session: None,
        configuration: Configuration::default(),
        setting_change: None,
        next_id: 3,
        turn: None,
        startup: (1, Instant::now() + STARTUP_TIMEOUT),
    };
    client.write(serde_json::json!({
        "jsonrpc":"2.0","id":1,"method":"initialize","params":{
            "protocolVersion":1,
            "clientCapabilities":{"fs":{"readTextFile":false,"writeTextFile":false},"terminal":false},
            "clientInfo":{"name":"mgb-ops","title":"MGB Ops","version":super::VERSION}
        }
    }))?;

    while !tools.closed() {
        for _ in 0..16 {
            match input.try_recv() {
                Ok(command) => client.input(command)?,
                Err(TryRecvError::Disconnected) => return Ok(()),
                Err(TryRecvError::Empty) => break,
            }
        }
        match wire.recv_timeout(Duration::from_millis(20)) {
            Ok(message) => client.receive(message?, &cwd_path, helper)?,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                return Err(Failure::new(
                    AssistantStage::Process,
                    "Agent closed its ACP output",
                ));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }
        if client.session.is_none() && Instant::now() >= client.startup.1 {
            return Err(Failure::new(
                AssistantStage::Protocol,
                "Agent timed out during ACP startup",
            ));
        }
        if client
            .setting_change
            .as_ref()
            .is_some_and(|change| Instant::now() >= change.deadline)
        {
            return Err(Failure::new(
                AssistantStage::Configuration,
                "The agent did not confirm the settings change. Reconnect to continue.",
            ));
        }
        if client
            .turn
            .as_ref()
            .and_then(|turn| turn.stop)
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(Failure::new(
                AssistantStage::Cancellation,
                "Agent did not acknowledge cancellation",
            ));
        }
    }
    Ok(())
}

struct Client<'a> {
    events: &'a Events,
    output: SyncSender<Value>,
    bridge: &'a Bridge,
    bridge_environment: Value,
    tools: &'a Tools,
    session: Option<Value>,
    configuration: Configuration,
    setting_change: Option<SettingChange>,
    next_id: u64,
    turn: Option<Turn>,
    startup: (u64, Instant),
}

struct SettingChange {
    id: u64,
    kind: SettingKind,
    value: String,
    legacy: bool,
    deadline: Instant,
}

struct Turn {
    id: u64,
    permission: Option<Permission>,
    stop: Option<Instant>,
}

struct Permission {
    id: Value,
    request_key: String,
    options: Vec<String>,
}

impl Client<'_> {
    fn write(&self, message: Value) -> Result<()> {
        self.output.try_send(message).map_err(|_| {
            Failure::new(
                AssistantStage::Process,
                "Agent stopped reading protocol messages",
            )
        })
    }

    fn emit(&self, event: AssistantEvent) -> Result<()> {
        self.events.try_send(event).map_err(|_| {
            Failure::new(
                AssistantStage::Process,
                "Assistant event queue is unavailable",
            )
        })
    }

    fn input(&mut self, input: Input) -> Result<()> {
        match input {
            Input::Prompt(text) => {
                if let Some(session) = &self.session
                    && self.turn.is_none()
                    && self.setting_change.is_none()
                {
                    let id = self.next_id;
                    self.next_id += 1;
                    self.tools.begin(id);
                    self.turn = Some(Turn {
                        id,
                        permission: None,
                        stop: None,
                    });
                    self.write(serde_json::json!({
                        "jsonrpc":"2.0","method":"session/prompt","id":id,
                        "params":{"sessionId":session,"prompt":[{"type":"text","text":text}]}
                    }))?;
                } else {
                    self.emit(AssistantEvent::Failed {
                        stage: AssistantStage::Prompt,
                        message: "Wait for the conversation to become ready".into(),
                        closed: false,
                    })?;
                }
            }
            Input::Setting { kind, value } => self.set_option(kind, value)?,
            Input::Permission { request, option } => {
                if let Some(turn) = &mut self.turn
                    && let Some(permission) = &turn.permission
                    && permission.request_key == request
                    && permission.options.contains(&option)
                {
                    let permission = turn.permission.take().unwrap();
                    self.write(rpc_result(
                        &permission.id,
                        serde_json::json!({
                            "outcome":{"outcome":"selected","optionId":option}
                        }),
                    ))?;
                    self.emit(AssistantEvent::PermissionCleared)?;
                }
            }
            Input::Stop => self.cancel()?,
        }
        Ok(())
    }

    fn set_option(&mut self, kind: SettingKind, value: String) -> Result<()> {
        if self.setting_change.is_some() {
            return self.emit(AssistantEvent::Failed {
                stage: AssistantStage::Configuration,
                message: "Wait for the pending settings change".into(),
                closed: false,
            });
        }
        let Some(session) = self.session.as_ref().filter(|_| self.turn.is_none()) else {
            return self.setting_result(Some("Wait for the agent to become idle".into()));
        };
        let target = match self.configuration.target(kind, &value) {
            Ok(target) => target,
            Err(error) => return self.setting_result(Some(error)),
        };
        let id = self.next_id;
        self.next_id += 1;
        let (method, params) = if let Some(config_id) = target {
            (
                "session/set_config_option",
                serde_json::json!({"sessionId":session,"configId":config_id,"value":value}),
            )
        } else {
            (
                "session/set_model",
                serde_json::json!({"sessionId":session,"modelId":value}),
            )
        };
        self.write(serde_json::json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        self.setting_change = Some(SettingChange {
            id,
            kind,
            value,
            legacy: target.is_none(),
            deadline: Instant::now() + SETTINGS_TIMEOUT,
        });
        Ok(())
    }

    fn setting_result(&self, error: Option<String>) -> Result<()> {
        self.emit(AssistantEvent::SettingChanged {
            settings: self.configuration.snapshot(),
            error,
        })
    }

    fn finish_setting(&mut self, message: &Value) -> Result<()> {
        let change = self.setting_change.take().unwrap();
        if let Some(error) = message.get("error") {
            return self.setting_result(Some(self.detail(error)));
        }
        let result = &message["result"];
        if change.legacy && result.is_object() {
            self.configuration
                .confirm_legacy_model(change.value.clone());
        } else if let Some(options) = result["configOptions"].as_array() {
            self.configuration.update(options);
        } else {
            return Err(Failure::new(
                AssistantStage::Configuration,
                "The agent returned no confirmed settings. Reconnect to continue.",
            ));
        }
        let settings = self.configuration.snapshot();
        let error = (!settings
            .get(change.kind)
            .is_some_and(|setting| setting.selected == change.value))
        .then(|| "The agent selected a different value".into());
        self.emit(AssistantEvent::SettingChanged { settings, error })
    }

    fn cancel(&mut self) -> Result<()> {
        let Some(turn) = self.turn.as_mut().filter(|turn| turn.stop.is_none()) else {
            return Ok(());
        };
        turn.stop = Some(Instant::now() + STOP_TIMEOUT);
        self.tools.finish(turn.id, true);
        let permission = turn.permission.take();
        self.write(serde_json::json!({
            "jsonrpc":"2.0","method":"session/cancel","params":{"sessionId":self.session}
        }))?;
        if let Some(permission) = permission {
            self.write(rpc_result(
                &permission.id,
                serde_json::json!({"outcome":{"outcome":"cancelled"}}),
            ))?;
            self.emit(AssistantEvent::PermissionCleared)?;
        }
        Ok(())
    }

    fn receive(&mut self, message: Value, cwd: &Path, helper: &str) -> Result<()> {
        if message["jsonrpc"] != "2.0" {
            return Err(Failure::new(
                AssistantStage::Protocol,
                "Expected an ACP JSON-RPC 2.0 message",
            ));
        }
        if let Some(method) = message["method"].as_str() {
            return self.request(method, &message);
        }
        let Some(id) = message["id"].as_u64() else {
            return Err(Failure::new(
                AssistantStage::Protocol,
                "ACP response is missing its request ID",
            ));
        };
        match id {
            1 if self.startup.0 == 1 && self.session.is_none() => {
                if let Some(error) = message.get("error") {
                    return Err(self.start_error(error));
                }
                if message["result"]["protocolVersion"] != 1 {
                    return Err(Failure::new(
                        AssistantStage::Protocol,
                        "The agent must negotiate ACP v1",
                    ));
                }
                self.emit(AssistantEvent::Handshake)?;
                self.startup = (2, Instant::now() + STARTUP_TIMEOUT);
                self.write(serde_json::json!({
                    "jsonrpc":"2.0","id":2,"method":"session/new","params":{
                        "cwd":cwd,"mcpServers":[{"name":"mgb-ops","command":helper,
                        "args":["--assistant-mcp-stdio"],"env":self.bridge_environment}]
                    }
                }))
            }
            2 if self.startup.0 == 2 && self.session.is_none() => {
                if let Some(error) = message.get("error") {
                    return Err(self.start_error(error));
                }
                let session = &message["result"]["sessionId"];
                if session.as_str().is_none_or(str::is_empty) {
                    return Err(Failure::new(
                        AssistantStage::Protocol,
                        "The agent did not provide an ACP session ID",
                    ));
                }
                self.session = Some(session.clone());
                self.configuration = Configuration::load(&message["result"]);
                self.emit(AssistantEvent::Settings(self.configuration.snapshot()))?;
                self.emit(AssistantEvent::Ready)
            }
            _ if self.session.is_none() => Err(Failure::new(
                AssistantStage::Protocol,
                "ACP startup returned an unexpected request ID",
            )),
            _ if self.turn.as_ref().is_some_and(|turn| turn.id == id) => self.finish(&message),
            _ if self
                .setting_change
                .as_ref()
                .is_some_and(|change| change.id == id) =>
            {
                self.finish_setting(&message)
            }
            _ => Ok(()),
        }
    }

    fn start_error(&self, error: &Value) -> Failure {
        let detail = self.detail(error);
        let lower = detail.to_lowercase();
        let stage = if error["code"] == -32000
            || lower.contains("auth")
            || lower.contains("sign in")
            || lower.contains("login")
        {
            AssistantStage::SignIn
        } else if lower.contains("mcp") || lower.contains("tool server") {
            AssistantStage::ToolConnection
        } else {
            AssistantStage::Protocol
        };
        Failure::new(stage, detail)
    }

    fn detail(&self, error: &Value) -> String {
        let message = error["message"]
            .as_str()
            .unwrap_or("Agent returned a protocol error");
        self.bridge.redact(message).chars().take(500).collect()
    }

    fn finish(&mut self, message: &Value) -> Result<()> {
        let turn = self.turn.take().unwrap();
        let cancelled = message["result"]["stopReason"] == "cancelled";
        let error = message.get("error");
        self.tools
            .finish(turn.id, cancelled || turn.stop.is_some() || error.is_some());
        if turn.permission.is_some() {
            self.emit(AssistantEvent::PermissionCleared)?;
        }
        if turn.stop.is_some() && !cancelled {
            return Err(Failure::new(
                AssistantStage::Cancellation,
                "Agent ended the turn without confirming cancellation",
            ));
        }
        if let Some(error) = error {
            let diagnosed = self.start_error(error);
            let stage = match diagnosed.stage {
                AssistantStage::SignIn | AssistantStage::ToolConnection => diagnosed.stage,
                _ => AssistantStage::Prompt,
            };
            self.emit(AssistantEvent::Failed {
                stage,
                message: diagnosed.message,
                closed: false,
            })?;
        } else if message["result"]["stopReason"].as_str().is_none() {
            self.tools.finish(turn.id, true);
            return Err(Failure::new(
                AssistantStage::Protocol,
                "Agent ended the prompt without a stop reason",
            ));
        }
        self.emit(AssistantEvent::Finished { cancelled })
    }

    fn request(&mut self, method: &str, message: &Value) -> Result<()> {
        let params = &message["params"];
        let current = self
            .session
            .as_ref()
            .is_some_and(|session| params["sessionId"] == *session);
        let accepting = current && self.turn.as_ref().is_some_and(|turn| turn.stop.is_none());
        if method == "session/update" {
            let update = &params["update"];
            if current
                && update["sessionUpdate"] == "config_option_update"
                && let Some(options) = update["configOptions"].as_array()
            {
                self.configuration.update(options);
                return self.emit(AssistantEvent::Settings(self.configuration.snapshot()));
            }
            if accepting {
                match update["sessionUpdate"].as_str() {
                    Some("agent_message_chunk" | "agent_thought_chunk") => {
                        if let Some(text) = update["content"]["text"].as_str() {
                            let text = self.bridge.redact(text);
                            let event = if update["sessionUpdate"] == "agent_thought_chunk" {
                                AssistantEvent::Thought(text)
                            } else {
                                AssistantEvent::Text(text)
                            };
                            self.emit(event)?;
                        }
                    }
                    Some("tool_call" | "tool_call_update") => {
                        let parts = [update["title"].as_str(), update["status"].as_str()]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>();
                        if !parts.is_empty() {
                            self.emit(AssistantEvent::ToolStatus(
                                self.bridge.redact(&parts.join(" · ")),
                            ))?;
                        }
                    }
                    _ => {}
                }
            }
            return Ok(());
        }
        let Some(id) = message.get("id") else {
            return Ok(());
        };
        if method != "session/request_permission" {
            return self.write(rpc_error(id, -32601, "Unsupported ACP client method"));
        }
        if !accepting {
            return self.write(rpc_result(
                id,
                serde_json::json!({"outcome":{"outcome":"cancelled"}}),
            ));
        }
        self.permission(id, params)
    }

    fn permission(&mut self, id: &Value, params: &Value) -> Result<()> {
        let options: Vec<_> = params["options"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|option| {
                let kind = option["kind"].as_str()?;
                matches!(kind, "allow_once" | "reject_once").then_some(())?;
                Some((
                    kind,
                    PermissionOption {
                        id: option["optionId"].as_str()?.into(),
                        label: self.bridge.redact(option["name"].as_str().unwrap_or(kind)),
                    },
                ))
            })
            .collect();
        let turn = self.turn.as_mut().unwrap();
        if turn.permission.is_some() {
            self.write(rpc_result(
                id,
                serde_json::json!({"outcome":{"outcome":"cancelled"}}),
            ))?;
            self.emit(AssistantEvent::ToolStatus(
                "The agent requested overlapping permissions. Cancelling the turn.".into(),
            ))?;
            return self.cancel();
        }
        turn.permission = Some(Permission {
            id: id.clone(),
            request_key: id.to_string(),
            options: options
                .iter()
                .map(|(_, option)| option.id.clone())
                .collect(),
        });
        if !options.iter().any(|(kind, _)| *kind == "reject_once") {
            self.emit(AssistantEvent::ToolStatus(
                "The agent offered no one-time deny choice. Cancelling the turn.".into(),
            ))?;
            return self.cancel();
        }
        let mut operation = params["toolCall"].clone();
        self.bridge.redact_value(&mut operation);
        self.emit(AssistantEvent::Permission {
            request: id.to_string(),
            operation,
            options: options.into_iter().map(|(_, option)| option).collect(),
        })
    }
}
