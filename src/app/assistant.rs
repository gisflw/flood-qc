mod bridge;
mod commands;
mod protocol;
mod settings;

pub use self::bridge::assistant_mcp_stdio;
pub use self::commands::{FieldChange, ProposalReview};
pub use self::settings::{AgentSetting, AgentSettings, SettingChoice, SettingKind};

use super::Session;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fmt,
    net::{Shutdown, TcpStream},
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
    thread,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const QUEUE: usize = 128;
const MAX_MESSAGE: usize = 1024 * 1024;
const MAX_PROMPT: usize = 64 * 1024;

pub const SETUP_NOTICE: &str = "The selected agent and its provider may receive your prompts and MGB Ops tool results. The installed agent's configuration governs its file, shell, and network permissions.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentPreset {
    Codex,
    Gemini,
    Claude,
}

impl AgentPreset {
    pub const ALL: [Self; 3] = [Self::Codex, Self::Gemini, Self::Claude];

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Gemini => "Gemini CLI",
            Self::Claude => "Claude",
        }
    }

    fn executable(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Codex => ("codex-acp", &[]),
            Self::Gemini => ("gemini", &["--acp"]),
            Self::Claude => ("claude-agent-acp", &[]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistantStage {
    Discovery,
    Process,
    Protocol,
    SignIn,
    ToolConnection,
    Configuration,
    Prompt,
    Cancellation,
}

impl fmt::Display for AssistantStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Discovery => "executable discovery",
            Self::Process => "subprocess",
            Self::Protocol => "ACP negotiation",
            Self::SignIn => "sign-in",
            Self::ToolConnection => "tool connection",
            Self::Configuration => "agent settings",
            Self::Prompt => "prompt",
            Self::Cancellation => "cancellation",
        })
    }
}

#[derive(Debug)]
pub enum AssistantEvent {
    Handshake,
    Ready,
    ToolConnected,
    Text(String),
    Thought(String),
    ToolStatus(String),
    Settings(AgentSettings),
    SettingChanged {
        settings: AgentSettings,
        error: Option<String>,
    },
    Permission {
        request: String,
        operation: Value,
        options: Vec<PermissionOption>,
    },
    PermissionCleared,
    ReviewStaged,
    Finished {
        cancelled: bool,
    },
    Failed {
        stage: AssistantStage,
        message: String,
        closed: bool,
    },
}

#[derive(Debug)]
pub struct PermissionOption {
    pub id: String,
    pub label: String,
}

enum Input {
    Prompt(String),
    Setting { kind: SettingKind, value: String },
    Permission { request: String, option: String },
    Stop,
}

type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
struct Events {
    sender: SyncSender<AssistantEvent>,
    wake: Option<Wake>,
}

impl Events {
    fn try_send(&self, event: AssistantEvent) -> Result<(), ()> {
        self.sender.try_send(event).map_err(|_| ())?;
        if let Some(wake) = &self.wake {
            wake();
        }
        Ok(())
    }
}

#[cfg(test)]
impl From<SyncSender<AssistantEvent>> for Events {
    fn from(sender: SyncSender<AssistantEvent>) -> Self {
        Self { sender, wake: None }
    }
}

pub struct Conversation {
    input: SyncSender<Input>,
    events: Receiver<AssistantEvent>,
    tools: Arc<Tools>,
    disconnected: bool,
    worker_done: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Conversation {
    /// Start the installed ACP adapter on a worker. The supplied executable re-enters
    /// the host with the private MCP stdio argument when the ACP agent needs a helper.
    pub fn start(session: Session, agent: AgentPreset, helper: PathBuf) -> Self {
        Self::start_agent(session, agent, helper, None)
    }

    /// Start a conversation and notify the interface when events become available.
    pub fn start_with_wake(
        session: Session,
        agent: AgentPreset,
        helper: PathBuf,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> Self {
        Self::start_agent(session, agent, helper, Some(Arc::new(wake)))
    }

    fn start_agent(
        session: Session,
        agent: AgentPreset,
        helper: PathBuf,
        wake: Option<Wake>,
    ) -> Self {
        let (executable, args) = agent.executable();
        Self::launch(
            session,
            helper,
            protocol::Launch {
                executable: executable.into(),
                args: args.iter().map(|arg| (*arg).into()).collect(),
            },
            bridge::Bridge::start,
            wake,
        )
    }

    fn launch(
        session: Session,
        helper: PathBuf,
        launch: protocol::Launch,
        connect: impl FnOnce(Arc<Tools>, Events) -> Result<bridge::Bridge, String> + Send + 'static,
        wake: Option<Wake>,
    ) -> Self {
        let (send_input, input) = sync_channel(16);
        let (send_event, events) = sync_channel(QUEUE);
        let tools = Arc::new(Tools::new(session));
        let worker = tools.clone();
        let worker_done = Arc::new(AtomicBool::new(false));
        let done = worker_done.clone();
        let worker = thread::spawn(move || {
            let events = Events {
                sender: send_event,
                wake: wake.clone(),
            };
            protocol::supervise(launch, helper, worker, input, events, connect);
            done.store(true, Ordering::Release);
            if let Some(wake) = wake {
                wake();
            }
        });
        Self {
            input: send_input,
            events,
            tools,
            disconnected: false,
            worker_done,
            worker: Some(worker),
        }
    }

    pub fn prompt(&self, text: String) -> Result<(), String> {
        if text.trim().is_empty() || text.len() > MAX_PROMPT {
            return Err(format!("Enter a prompt of 1 to {MAX_PROMPT} bytes"));
        }
        self.send(Input::Prompt(text))
    }

    pub fn set_option(&self, kind: SettingKind, value: String) -> Result<(), String> {
        self.send(Input::Setting { kind, value })
    }

    pub fn answer(&self, request: String, option: String) -> Result<(), String> {
        self.send(Input::Permission { request, option })
    }

    pub fn stop(&self) -> Result<(), String> {
        self.send(Input::Stop)?;
        self.tools.cancel_active();
        Ok(())
    }

    fn send(&self, input: Input) -> Result<(), String> {
        self.input.try_send(input).map_err(|error| {
            match error {
                std::sync::mpsc::TrySendError::Full(_) => "Assistant is busy",
                std::sync::mpsc::TrySendError::Disconnected(_) => {
                    "Assistant conversation is closed"
                }
            }
            .into()
        })
    }

    pub fn try_event(&mut self) -> Option<AssistantEvent> {
        match self.events.try_recv() {
            Ok(event) => {
                if matches!(event, AssistantEvent::Failed { closed: true, .. }) {
                    self.disconnected = true;
                }
                Some(event)
            }
            Err(error)
                if !self.disconnected
                    && (error == TryRecvError::Disconnected
                        || self.worker_done.load(Ordering::Acquire)) =>
            {
                self.disconnected = true;
                Some(AssistantEvent::Failed {
                    stage: AssistantStage::Process,
                    message: "Assistant worker stopped".into(),
                    closed: true,
                })
            }
            Err(_) => None,
        }
    }

    pub fn review(&self) -> Option<ProposalReview> {
        self.tools
            .state()
            .proposal
            .as_ref()
            .map(|(_, proposal)| proposal.review.clone())
    }

    pub fn apply(&self) -> Result<(), String> {
        let mut state = self.tools.state();
        let (_, proposal) = state.proposal.as_ref().ok_or("No pending run proposal")?;
        let result = self
            .tools
            .session
            .update_run_setup(&proposal.revision, proposal.parameters.clone());
        state.proposal = None;
        result
    }

    pub fn reject(&self) -> bool {
        self.tools.state().proposal.take().is_some()
    }

    pub fn close(&mut self) {
        self.tools.close();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for Conversation {
    fn drop(&mut self) {
        self.close();
    }
}

struct Tools {
    session: Session,
    state: Mutex<ToolState>,
}

#[derive(Default)]
struct ToolState {
    closed: bool,
    turn: Option<u64>,
    proposal: Option<(u64, Box<commands::Proposal>)>,
    sockets: BTreeMap<u64, (Option<u64>, TcpStream)>,
    next_socket: u64,
}

impl Tools {
    fn new(session: Session) -> Self {
        Self {
            session,
            state: Mutex::default(),
        }
    }

    fn state(&self) -> MutexGuard<'_, ToolState> {
        self.state.lock().unwrap_or_else(|error| error.into_inner())
    }

    fn close(&self) {
        let mut state = self.state();
        state.closed = true;
        state.turn = None;
        state.proposal = None;
        for (_, (_, socket)) in std::mem::take(&mut state.sockets) {
            let _ = socket.shutdown(Shutdown::Both);
        }
    }

    fn closed(&self) -> bool {
        self.state().closed
    }

    fn register(&self, stream: &TcpStream) -> Option<(u64, Option<u64>)> {
        let mut state = self.state();
        if state.closed || state.sockets.len() >= 4 {
            return None;
        }
        let id = state.next_socket;
        state.next_socket += 1;
        let turn = state.turn;
        state.sockets.insert(id, (turn, stream.try_clone().ok()?));
        Some((id, turn))
    }

    fn begin(&self, turn: u64) {
        let mut state = self.state();
        if !state.closed {
            state.turn = Some(turn);
        }
    }

    fn cancel_active(&self) {
        let mut state = self.state();
        if let Some(turn) = state.turn {
            state.finish(turn, true);
        }
    }

    fn finish(&self, turn: u64, discard: bool) {
        self.state().finish(turn, discard);
    }
}

impl ToolState {
    fn finish(&mut self, turn: u64, discard: bool) {
        if self.turn == Some(turn) {
            self.turn = None;
        }
        for (source, socket) in self.sockets.values() {
            if *source == Some(turn) {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if discard
            && self
                .proposal
                .as_ref()
                .is_some_and(|(source, _)| *source == turn)
        {
            self.proposal = None;
        }
    }
}

fn bounded_line(reader: &mut impl std::io::BufRead) -> std::io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Unterminated message",
                ))
            };
        }
        let end = buffer.iter().position(|byte| *byte == b'\n');
        let count = end.map_or(buffer.len(), |end| end + 1);
        if line.len() + count > MAX_MESSAGE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Message exceeds size limit",
            ));
        }
        line.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}

#[cfg(test)]
mod tests;
