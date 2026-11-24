mod acp;
mod commands;
mod mcp;
mod proposal;
mod wire;

pub use self::mcp::{ASSISTANT_MCP_ARG, assist_mcp_entry, assist_mcp_stdio};
pub use self::proposal::{DraftEdit, ProposalOutcome, ProposalReview};
use super::Session;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt, io,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, watch};

const EVENT_CAPACITY: usize = 128;
const MAX_PROMPT_BYTES: usize = 65_536;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum AgentPreset {
    #[default]
    Codex,
    Gemini,
    Claude,
}

impl AgentPreset {
    pub fn name(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Gemini => "Gemini CLI",
            Self::Claude => "Claude",
        }
    }

    pub fn executable(self) -> &'static str {
        match self {
            Self::Codex => "codex-acp",
            Self::Gemini => "gemini",
            Self::Claude => "claude-agent-acp",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssistantStage {
    Executable,
    Protocol,
    SignIn,
    ToolConnection,
    Subprocess,
    Timeout,
    Usage,
}

impl fmt::Display for AssistantStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Executable => "executable discovery",
            Self::Protocol => "ACP protocol",
            Self::SignIn => "agent sign-in",
            Self::ToolConnection => "MCP connection",
            Self::Subprocess => "agent subprocess",
            Self::Timeout => "timeout",
            Self::Usage => "conversation",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssistantFailure {
    pub agent: AgentPreset,
    pub stage: AssistantStage,
    pub message: String,
}

impl AssistantFailure {
    fn new(agent: AgentPreset, stage: AssistantStage, message: impl Into<String>) -> Self {
        Self {
            agent,
            stage,
            message: message.into(),
        }
    }
}

impl fmt::Display for AssistantFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} ({}): {}",
            self.agent.name(),
            self.stage,
            self.message
        )
    }
}
impl std::error::Error for AssistantFailure {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssistantPermissionKind {
    AllowOnce,
    RejectOnce,
}

#[derive(Clone, Debug)]
pub struct AssistantPermissionOption {
    pub id: String,
    pub name: String,
    pub kind: AssistantPermissionKind,
}

#[derive(Clone, Debug)]
pub struct AssistantPermission {
    pub request: u64,
    pub tool_call: Value,
    pub options: Vec<AssistantPermissionOption>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssistantStopReason {
    EndTurn,
    Cancelled,
    Refused,
    Limit,
    Failed,
}

#[derive(Debug)]
pub enum AssistantEvent {
    Ready,
    ToolsConnected,
    Text(String),
    Thought(String),
    ToolStatus(Value),
    ToolCallCompleted {
        tool: String,
        command: Option<String>,
        ok: bool,
    },
    Permission(AssistantPermission),
    Notice(String),
    Finished(AssistantStopReason),
    Failed(AssistantFailure),
}

/// Host-controlled launch inputs. Defaults use the preset executable on PATH,
/// the host executable as the MCP helper, and a fresh local working directory.
#[derive(Clone, Debug, Default)]
pub struct AssistantOptions {
    pub executable: Option<PathBuf>,
    pub helper_executable: Option<PathBuf>,
    pub working_directory: Option<PathBuf>,
}

#[derive(Clone, Copy, Default)]
struct Control {
    closed: bool,
    cancelled: u64,
}

enum HostCommand {
    Prompt {
        turn: u64,
        text: String,
    },
    Permission {
        turn: u64,
        request: u64,
        choice: String,
    },
}

struct Event {
    turn: Option<u64>,
    value: AssistantEvent,
}

struct Shared {
    state: Mutex<AccessState>,
    outcome: Mutex<WorkerOutcome>,
    wake: Condvar,
}

#[derive(Default)]
struct AccessState {
    closed: bool,
    turn: Option<u64>,
    cancelled: u64,
    review: proposal::ProposalReviewState,
}

#[derive(Default)]
struct WorkerOutcome {
    done: bool,
    failure: Option<AssistantFailure>,
}

impl Shared {
    fn new() -> Self {
        Self {
            state: Mutex::default(),
            outcome: Mutex::default(),
            wake: Condvar::new(),
        }
    }

    fn turn(&self) -> Option<u64> {
        self.state.lock().unwrap().turn
    }

    fn begin(&self, turn: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.closed || state.cancelled >= turn {
            return false;
        }
        state.turn = Some(turn);
        true
    }

    fn finish(&self, turn: u64, keep_review: bool) {
        let mut state = self.state.lock().unwrap();
        if state.turn == Some(turn) {
            state.turn = None;
        }
        if !keep_review {
            state.cancelled = state.cancelled.max(turn);
            state.review.cancel_turn(turn);
        }
    }

    fn close(&self) {
        let mut state = self.state.lock().unwrap();
        state.closed = true;
        state.turn = None;
        state.review.clear();
    }

    fn is_closed(&self) -> bool {
        self.state.lock().unwrap().closed
    }
}

/// A single agent conversation bound to a clone of the originating workspace session.
pub struct Assistant {
    agent: AgentPreset,
    session: Session,
    events: mpsc::Receiver<Event>,
    commands: mpsc::Sender<HostCommand>,
    control: watch::Sender<Control>,
    shared: Arc<Shared>,
    next_turn: u64,
    active: Option<u64>,
    permission_options: Vec<(u64, Vec<String>)>,
}

impl Assistant {
    pub fn start(session: Session, agent: AgentPreset) -> Result<Self, AssistantFailure> {
        Self::start_with_options(session, agent, AssistantOptions::default())
    }

    pub fn start_with_options(
        session: Session,
        agent: AgentPreset,
        options: AssistantOptions,
    ) -> Result<Self, AssistantFailure> {
        let shared = Arc::new(Shared::new());
        let worker_shared = shared.clone();
        let worker_session = session.clone();
        let (events_tx, events) = mpsc::channel(EVENT_CAPACITY);
        let (commands, commands_rx) = mpsc::channel(16);
        let (control, control_rx) = watch::channel(Control::default());
        std::thread::Builder::new()
            .name("mgb-ops-assistant".into())
            .spawn(move || {
                let result = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|error| {
                        AssistantFailure::new(agent, AssistantStage::Subprocess, error.to_string())
                    })
                    .and_then(|runtime| {
                        let result = runtime.block_on(acp::run(
                            agent,
                            options,
                            worker_session,
                            acp::Channels {
                                control: control_rx,
                                commands: commands_rx,
                                events: events_tx,
                                shared: worker_shared.clone(),
                            },
                        ));
                        // Cancelled read work may still be unwinding independently after revocation.
                        runtime.shutdown_background();
                        result
                    });
                worker_shared.close();
                let mut outcome = worker_shared.outcome.lock().unwrap();
                outcome.failure = result.err();
                outcome.done = true;
                worker_shared.wake.notify_all();
            })
            .map_err(|error| {
                AssistantFailure::new(agent, AssistantStage::Subprocess, error.to_string())
            })?;
        Ok(Self {
            agent,
            session,
            events,
            commands,
            control,
            shared,
            next_turn: 1,
            active: None,
            permission_options: vec![],
        })
    }

    pub fn prompt(&mut self, text: String) -> Result<(), AssistantFailure> {
        if self.active.is_some() {
            return Err(self.usage("Wait for the current assistant turn to finish"));
        }
        if text.trim().is_empty() || text.len() > MAX_PROMPT_BYTES {
            return Err(self.usage("Enter a nonempty prompt of at most 65,536 bytes"));
        }
        let turn = self.next_turn;
        self.commands
            .try_send(HostCommand::Prompt { turn, text })
            .map_err(|_| self.usage("The assistant conversation is unavailable"))?;
        self.next_turn += 1;
        self.active = Some(turn);
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.active.is_some()
    }

    pub fn answer_permission(
        &mut self,
        request: u64,
        choice: &str,
    ) -> Result<(), AssistantFailure> {
        let turn = self
            .active
            .ok_or_else(|| self.usage("No assistant turn is active"))?;
        let index = self
            .permission_options
            .iter()
            .position(|(id, options)| *id == request && options.iter().any(|id| id == choice))
            .ok_or_else(|| {
                self.usage("The permission request or one-time choice is unavailable")
            })?;
        self.commands
            .try_send(HostCommand::Permission {
                turn,
                request,
                choice: choice.into(),
            })
            .map_err(|_| self.usage("The assistant conversation is unavailable"))?;
        self.permission_options.remove(index);
        Ok(())
    }

    pub fn proposal(&self) -> Option<ProposalReview> {
        self.shared.state.lock().unwrap().review.current()
    }

    pub fn apply_proposal(&self, expected: &ProposalReview) -> ProposalOutcome {
        self.shared
            .state
            .lock()
            .unwrap()
            .review
            .apply(&self.session, expected)
    }

    pub fn reject_proposal(&self, expected: &ProposalReview) -> ProposalOutcome {
        self.shared.state.lock().unwrap().review.reject(expected)
    }

    pub fn stop(&mut self) {
        if let Some(turn) = self.active {
            self.shared.finish(turn, false);
            self.permission_options.clear();
            self.control.send_modify(|control| control.cancelled = turn);
        }
    }

    pub fn close(&mut self) {
        self.shared.close();
        self.control.send_modify(|control| control.closed = true);
        self.events.close();
        self.active = None;
        self.permission_options.clear();
    }

    /// Wait for the supervised process to be reaped after closure.
    pub fn wait_closed(&self, timeout: Duration) -> bool {
        let outcome = self.shared.outcome.lock().unwrap();
        self.shared
            .wake
            .wait_timeout_while(outcome, timeout, |state| !state.done)
            .unwrap()
            .0
            .done
    }

    pub fn try_event(&mut self) -> Option<AssistantEvent> {
        let control = *self.control.borrow();
        if control.closed {
            return None;
        }
        while let Ok(event) = self.events.try_recv() {
            if event.turn.is_some_and(|turn| turn <= control.cancelled)
                && !matches!(event.value, AssistantEvent::Finished(_))
            {
                continue;
            }
            match &event.value {
                AssistantEvent::Permission(permission) => self.permission_options.push((
                    permission.request,
                    permission
                        .options
                        .iter()
                        .map(|option| option.id.clone())
                        .collect(),
                )),
                AssistantEvent::Finished(_) => {
                    self.active = None;
                    self.permission_options.clear();
                }
                _ => {}
            }
            return Some(event.value);
        }
        let failure = self.shared.outcome.lock().unwrap().failure.take()?;
        self.active = None;
        self.permission_options.clear();
        Some(AssistantEvent::Failed(failure))
    }

    fn usage(&self, message: &'static str) -> AssistantFailure {
        AssistantFailure::new(self.agent, AssistantStage::Usage, message)
    }
}

impl Drop for Assistant {
    fn drop(&mut self) {
        self.close();
    }
}

fn invalid_data(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
