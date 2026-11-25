use clap::{Args, ValueEnum};
use mgb_ops::app::{
    AgentPreset, AssistantEvent, Conversation, InspectionRequest, PermissionOption, RunSelection,
    SETUP_NOTICE, Session,
};
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{RecvTimeoutError, sync_channel},
    },
    thread,
    time::Duration,
};

#[derive(Args)]
pub(crate) struct Options {
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long, value_enum)]
    agent: Agent,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    saved_run: Option<PathBuf>,
}

#[derive(Clone, ValueEnum)]
enum Agent {
    Codex,
    Gemini,
    Claude,
}

impl From<Agent> for AgentPreset {
    fn from(agent: Agent) -> Self {
        match agent {
            Agent::Codex => Self::Codex,
            Agent::Gemini => Self::Gemini,
            Agent::Claude => Self::Claude,
        }
    }
}

struct Terminal {
    agent: AgentPreset,
    session: Session,
    executable: PathBuf,
    conversation: Conversation,
    ready: bool,
    active: bool,
    permission: Option<(String, Vec<PermissionOption>)>,
}

impl Terminal {
    fn reset(&mut self) {
        self.conversation.close();
        self.conversation =
            Conversation::start(self.session.clone(), self.agent, self.executable.clone());
        self.ready = false;
        self.active = false;
        self.permission = None;
    }

    fn events(&mut self, out: &mut impl Write) -> io::Result<bool> {
        while let Some(event) = self.conversation.try_event() {
            match event {
                AssistantEvent::Handshake => {
                    writeln!(out, "{} negotiated ACP v1.", self.agent.label())?
                }
                AssistantEvent::Ready => {
                    self.ready = true;
                    writeln!(out, "Ready for a prompt.")?;
                }
                AssistantEvent::ToolConnected => writeln!(out, "MGB Ops tools connected.")?,
                AssistantEvent::Text(text) => write!(out, "{text}")?,
                AssistantEvent::SessionStarted(_)
                | AssistantEvent::ResumeUnavailable(_)
                | AssistantEvent::Thought(_)
                | AssistantEvent::Settings(_)
                | AssistantEvent::SettingChanged { .. } => {}
                AssistantEvent::ToolStatus { id, title, status } => {
                    let parts = [title.as_deref(), status.as_deref(), id.as_deref()]
                        .into_iter()
                        .flatten()
                        .collect::<Vec<_>>();
                    writeln!(out, "\n{}", parts.join(" · "))?;
                }
                AssistantEvent::Permission {
                    request,
                    operation,
                    options,
                } => {
                    writeln!(out, "\nPermission requested\n{operation:#}")?;
                    for (index, option) in options.iter().enumerate() {
                        writeln!(out, "{}. {}", index + 1, option.label)?;
                    }
                    writeln!(out, "Enter an option number or :stop.")?;
                    self.permission = Some((request, options));
                }
                AssistantEvent::PermissionCleared => self.permission = None,
                AssistantEvent::ReviewStaged => {
                    if let Some(review) = self.conversation.review() {
                        writeln!(out, "\nRun proposal")?;
                        for field in review.changes {
                            let scenario = field.scenario_index.map_or(String::new(), |index| {
                                format!(
                                    " · scenario {index} ({})",
                                    field.scenario_id.as_deref().unwrap_or("unnamed")
                                )
                            });
                            writeln!(
                                out,
                                "  {}{scenario}  {} → {}",
                                field.field, field.current, field.proposed
                            )?;
                        }
                        for finding in review.readiness {
                            writeln!(out, "  {finding}")?;
                        }
                        writeln!(out, "Use :apply or :reject.")?;
                    }
                }
                AssistantEvent::Finished { cancelled } => {
                    self.active = false;
                    self.permission = None;
                    writeln!(out, "{}", if cancelled { "\nPrompt stopped." } else { "" })?;
                }
                AssistantEvent::Failed {
                    stage,
                    message,
                    closed,
                } => {
                    writeln!(out, "\n{} · {stage} · {message}", self.agent.label())?;
                    if closed {
                        self.active = false;
                        self.ready = false;
                        self.permission = None;
                        return Ok(false);
                    }
                }
            }
        }
        out.flush()?;
        Ok(true)
    }

    fn line(&mut self, line: &str, out: &mut impl Write) -> io::Result<bool> {
        let input = line.trim();
        let result = match input {
            ":quit" => return Ok(false),
            ":reset" => {
                self.reset();
                writeln!(out, "Conversation reset.")?;
                Ok(())
            }
            ":stop" => self.conversation.stop(),
            ":apply" => match self.conversation.apply() {
                Ok(()) => {
                    writeln!(
                        out,
                        "Applied to this session. Use the displayed parameters when running the CLI."
                    )?;
                    Ok(())
                }
                Err(error) => Err(error),
            },
            ":reject" => {
                let message = if self.conversation.reject() {
                    "Rejected run proposal."
                } else {
                    "No pending run proposal."
                };
                writeln!(out, "{message}")?;
                Ok(())
            }
            "" => Ok(()),
            _ if self.permission.is_some() => {
                let (request, options) = self.permission.as_ref().unwrap();
                match input
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| index.checked_sub(1))
                    .and_then(|index| options.get(index))
                {
                    Some(option) => self.conversation.answer(request.clone(), option.id.clone()),
                    None => Err("Enter an offered option number or :stop".into()),
                }
            }
            _ if !self.ready => Err("Wait for the agent or use :reset".into()),
            _ if self.active => Err("A prompt is active. Use :stop to cancel it.".into()),
            _ => self
                .conversation
                .prompt(line.to_owned())
                .map(|()| self.active = true),
        };
        if let Err(message) = result {
            writeln!(out, "{message}")?;
        }
        out.flush()?;
        Ok(true)
    }
}

pub(crate) fn run(options: Options) -> Result<bool, Box<dyn std::error::Error>> {
    let session = Session::new(InspectionRequest {
        workspace: options.workspace,
        invocation_dir: std::env::current_dir()?,
        config: options.config,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    if let Some(path) = options.saved_run {
        session.select_run(RunSelection::Saved(path))?;
    }
    if let Ok(operation) =
        mgb_ops::app::OperationalRun::open_workspace(session.workspace().into(), chrono::Utc::now())
    {
        session.open_run_setup(operation);
    }
    let executable = std::env::current_exe()?;
    let agent = options.agent.into();
    let mut out = io::stdout().lock();
    writeln!(out, "{SETUP_NOTICE}")?;
    writeln!(
        out,
        "Enter a prompt or use :stop, :reset, :apply, :reject, :quit."
    )?;
    out.flush()?;

    let interrupted = Arc::new(AtomicBool::new(false));
    let signal = interrupted.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Release))?;
    let (sender, lines) = sync_channel(1);
    thread::spawn(move || {
        for line in io::stdin().lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let conversation = Conversation::start(session.clone(), agent, executable.clone());
    let mut terminal = Terminal {
        agent,
        session,
        executable,
        conversation,
        ready: false,
        active: false,
        permission: None,
    };
    let mut failed = false;
    loop {
        if interrupted.swap(false, Ordering::AcqRel) && terminal.active {
            terminal.conversation.stop()?;
        }
        if !terminal.events(&mut out)? {
            failed = true;
        }
        match lines.recv_timeout(Duration::from_millis(20)) {
            Ok(line) => {
                let line = line?;
                if !terminal.line(&line, &mut out)? {
                    return Ok(!failed);
                }
                if line.trim() == ":reset" {
                    failed = false;
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Ok(!failed);
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}
