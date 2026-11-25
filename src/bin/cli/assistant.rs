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
    #[arg(long)]
    draft: Option<PathBuf>,
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
    departure: bool,
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
                AssistantEvent::Thought(_)
                | AssistantEvent::Settings(_)
                | AssistantEvent::SettingChanged { .. } => {}
                AssistantEvent::ToolStatus(status) => writeln!(out, "\n{status}")?,
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
                        writeln!(out, "\nDraft proposal")?;
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

    fn dirty(&self) -> bool {
        self.session
            .draft()
            .is_some_and(|snapshot| snapshot.draft.is_dirty())
    }

    fn save(&self) -> Result<(), String> {
        let job = self
            .session
            .start_draft_job()
            .map_err(|error| error.to_string())?;
        let mut draft = job.snapshot.as_ref().ok_or("No open draft")?.draft.clone();
        let path = draft.path.clone().ok_or("No opened draft path")?;
        draft.save(&path, true)?;
        job.commit(draft, false).map_err(|error| error.to_string())
    }

    fn line(&mut self, line: &str, out: &mut impl Write) -> io::Result<bool> {
        let input = line.trim();
        if self.departure {
            self.departure = false;
            if matches!(input, "y" | "Y" | "yes" | "Yes") {
                writeln!(out, "Discarded unsaved draft edits.")?;
                return Ok(false);
            }
            writeln!(out, "Continuing the conversation.")?;
            return Ok(true);
        }
        let result = match input {
            ":quit" => {
                if !self.dirty() {
                    return Ok(false);
                }
                self.departure = true;
                writeln!(out, "Discard unsaved draft edits and quit? [y/N]")?;
                Ok(())
            }
            ":reset" => {
                self.reset();
                writeln!(out, "Conversation reset.")?;
                Ok(())
            }
            ":stop" => self.conversation.stop(),
            ":save" => match self.save() {
                Ok(()) => {
                    writeln!(out, "Saved working draft.")?;
                    Ok(())
                }
                Err(error) => Err(error),
            },
            ":apply" => match self.conversation.apply() {
                Ok(()) => {
                    writeln!(
                        out,
                        "Applied to the working draft. Use :save to write the file."
                    )?;
                    Ok(())
                }
                Err(error) => Err(error),
            },
            ":reject" => {
                let message = if self.conversation.reject() {
                    "Rejected draft proposal."
                } else {
                    "No pending draft proposal."
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
    if let Some(path) = options.draft {
        session.replace_draft(session.open_draft(&path)?)?;
    }
    let executable = std::env::current_exe()?;
    let agent = options.agent.into();
    let mut out = io::stdout().lock();
    writeln!(out, "{SETUP_NOTICE}")?;
    writeln!(
        out,
        "Enter a prompt or use :stop, :reset, :apply, :reject, :save, :quit."
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
        departure: false,
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
                if terminal.dirty() {
                    writeln!(out, "End of input. Discarded unsaved draft edits.")?;
                }
                return Ok(!failed);
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}
