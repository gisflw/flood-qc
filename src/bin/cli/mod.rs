use clap::{Args, ValueEnum};
use mgb_ops::app::{
    AgentPreset, Assistant, AssistantEvent, AssistantPermission, AssistantPermissionKind,
    AssistantStopReason, Draft, DraftEdit, InspectionRequest, ProposalOutcome, ProposalReview,
    Session,
};
use std::{
    collections::VecDeque,
    io::{self, BufRead, IsTerminal, Write},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{Receiver, RecvTimeoutError, sync_channel},
    },
    time::Duration,
};

#[derive(Args)]
pub struct Arguments {
    #[arg(long)]
    workspace: PathBuf,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, value_enum)]
    agent: Agent,
    #[arg(long)]
    run: Option<PathBuf>,
    #[arg(long)]
    draft: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
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

pub fn assistant(
    arguments: Arguments,
    out: &mut impl Write,
) -> Result<bool, Box<dyn std::error::Error>> {
    let session = Session::new(InspectionRequest {
        invocation_dir: std::env::current_dir()?,
        workspace: arguments.workspace,
        config: arguments.config,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    if let Some(path) = arguments.run {
        session.select_run(session.run().select_saved(&path)?)?;
    }
    if let Some(path) = arguments.draft {
        session.set_draft(session.workspace().open_draft(&path)?)?;
    }
    let terminal_input = io::stdin().is_terminal();
    let interrupted = Arc::new(AtomicBool::new(false));
    let signal = interrupted.clone();
    ctrlc::set_handler(move || signal.store(true, Ordering::Relaxed))?;
    let agent: AgentPreset = arguments.agent.into();
    writeln!(
        out,
        "The selected agent and its provider may receive prompts and tool results."
    )?;
    writeln!(
        out,
        "Native file, shell, and network permissions depend on the installed agent's configuration."
    )?;
    writeln!(
        out,
        "Enter a prompt or use /stop, /reset, /apply, /reject, /save, /quit. Permission commands appear when needed."
    )?;
    writeln!(out, "Starting {}.", agent.name())?;
    out.flush()?;
    let assistant = Assistant::start(session.clone(), agent)?;
    let mut terminal = Terminal {
        session,
        agent,
        assistant,
        permissions: VecDeque::new(),
        review: None,
        confirming_quit: false,
        streamed_text: false,
    };
    let result = terminal.run(input_lines(), &interrupted, terminal_input, out);
    terminal.close();
    Ok(result?)
}

fn input_lines() -> Receiver<io::Result<String>> {
    let (send, input) = sync_channel(1);
    std::thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            if send.send(line).is_err() {
                break;
            }
        }
    });
    input
}

struct Terminal {
    session: Session,
    agent: AgentPreset,
    assistant: Assistant,
    permissions: VecDeque<AssistantPermission>,
    review: Option<ProposalReview>,
    confirming_quit: bool,
    streamed_text: bool,
}

impl Terminal {
    fn run(
        &mut self,
        input: Receiver<io::Result<String>>,
        interrupted: &AtomicBool,
        terminal_input: bool,
        out: &mut impl Write,
    ) -> io::Result<bool> {
        self.prompt_line(terminal_input, out)?;
        loop {
            let was_running = self.assistant.is_running();
            self.poll(out)?;
            if was_running && !self.assistant.is_running() {
                self.prompt_line(terminal_input, out)?;
            }
            if interrupted.swap(false, Ordering::Relaxed) {
                self.end_reply(out)?;
                self.confirming_quit = false;
                self.stop(out)?;
                self.prompt_line(terminal_input, out)?;
            }
            out.flush()?;
            match input.recv_timeout(Duration::from_millis(25)) {
                Ok(Ok(line)) => {
                    if self.line(line.trim(), out)? {
                        return Ok(true);
                    }
                    self.prompt_line(terminal_input, out)?;
                }
                Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => continue,
                Ok(Err(error)) => {
                    self.input_ended(out)?;
                    return Err(error);
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.input_ended(out)?;
                    return Ok(true);
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }
    }

    fn close(&mut self) {
        self.assistant.close();
        let _ = self.assistant.wait_closed(Duration::from_secs(3));
    }

    fn dirty(&self) -> bool {
        self.session
            .draft()
            .is_some_and(|snapshot| snapshot.draft.is_dirty())
    }

    fn input_ended(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.end_reply(out)?;
        if self.dirty() {
            writeln!(out, "End of input. Discarding unsaved draft edits.")?;
        }
        out.flush()
    }

    fn line(&mut self, line: &str, out: &mut impl Write) -> io::Result<bool> {
        if line.is_empty() {
            return Ok(false);
        }
        self.end_reply(out)?;
        if self.confirming_quit {
            self.confirming_quit = false;
            if line.eq_ignore_ascii_case("yes") {
                writeln!(out, "Discarding unsaved draft edits.")?;
                out.flush()?;
                return Ok(true);
            }
            writeln!(out, "Quit cancelled.")?;
            return Ok(false);
        }
        let (command, argument) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        match command {
            "/quit" => {
                if self.dirty() {
                    self.confirming_quit = true;
                    writeln!(
                        out,
                        "Discard unsaved draft edits and quit? Enter yes or no."
                    )?;
                } else {
                    return Ok(true);
                }
            }
            "/stop" => self.stop(out)?,
            "/reset" => {
                self.close();
                self.permissions.clear();
                self.review = None;
                match Assistant::start(self.session.clone(), self.agent) {
                    Ok(assistant) => {
                        self.assistant = assistant;
                        writeln!(out, "Conversation reset.")?;
                    }
                    Err(error) => writeln!(out, "{error}")?,
                }
            }
            "/save" => match save(&self.session) {
                Ok(path) => writeln!(out, "Saved {}.", path.display())?,
                Err(error) => writeln!(out, "{error}")?,
            },
            "/apply" => {
                let outcome = self
                    .review
                    .as_ref()
                    .map_or(ProposalOutcome::NoPending, |review| {
                        self.assistant.apply_proposal(review)
                    });
                writeln!(out, "{outcome}")?;
            }
            "/reject" => {
                let outcome = self
                    .review
                    .as_ref()
                    .map_or(ProposalOutcome::NoPending, |review| {
                        self.assistant.reject_proposal(review)
                    });
                writeln!(out, "{outcome}")?;
            }
            "/allow" => self.answer(AssistantPermissionKind::AllowOnce, argument.trim(), out)?,
            "/deny" => self.answer(AssistantPermissionKind::RejectOnce, argument.trim(), out)?,
            _ if command.starts_with('/') => {
                writeln!(
                    out,
                    "Unknown command. Use /stop, /reset, /apply, /reject, /save, or /quit."
                )?;
            }
            _ if !self.permissions.is_empty() => {
                writeln!(out, "Answer the permission request or enter /stop.")?;
            }
            _ => {
                if let Err(error) = self.assistant.prompt(line.into()) {
                    writeln!(out, "{error}")?;
                }
            }
        }
        Ok(false)
    }

    fn stop(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.assistant.is_running() {
            self.assistant.stop();
            self.permissions.clear();
            writeln!(out, "Stopping the active prompt.")?;
        }
        Ok(())
    }

    fn answer(
        &mut self,
        kind: AssistantPermissionKind,
        id: &str,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let Some(request) = self.permissions.front() else {
            return writeln!(out, "No permission request is pending.");
        };
        if !request
            .options
            .iter()
            .any(|option| option.kind == kind && option.id == id)
        {
            writeln!(out, "Select one of the listed permission options.")?;
            return show_permission(request, out);
        }
        match self.assistant.answer_permission(request.request, id) {
            Ok(()) => {
                self.permissions.pop_front();
                if let Some(request) = self.permissions.front() {
                    show_permission(request, out)?;
                }
            }
            Err(error) => writeln!(out, "{error}")?,
        }
        Ok(())
    }

    fn prompt_line(&self, terminal_input: bool, out: &mut impl Write) -> io::Result<()> {
        if terminal_input && !self.assistant.is_running() && !self.confirming_quit {
            write!(out, "> ")?;
        }
        Ok(())
    }

    fn end_reply(&mut self, out: &mut impl Write) -> io::Result<()> {
        if self.streamed_text {
            writeln!(out)?;
            self.streamed_text = false;
        }
        Ok(())
    }

    fn poll(&mut self, out: &mut impl Write) -> io::Result<()> {
        for _ in 0..128 {
            let Some(event) = self.assistant.try_event() else {
                break;
            };
            if let AssistantEvent::Text(text) = event {
                write!(out, "{}", terminal_text(&text))?;
                self.streamed_text = true;
                continue;
            }
            if matches!(event, AssistantEvent::Thought(_)) {
                continue;
            }
            self.end_reply(out)?;
            match event {
                AssistantEvent::Ready => writeln!(out, "Conversation ready.")?,
                AssistantEvent::ToolsConnected => writeln!(out, "Workspace tools connected.")?,
                AssistantEvent::ToolStatus(update) => {
                    if let Some(title) = update["title"].as_str() {
                        writeln!(out, "{}", terminal_text(title))?;
                    }
                }
                AssistantEvent::ToolCallCompleted { tool, command, ok } => {
                    let action = terminal_text(command.as_deref().unwrap_or(&tool));
                    let outcome = if ok {
                        "completed"
                    } else {
                        "returned a finding"
                    };
                    writeln!(out, "{action} {outcome}.")?;
                }
                AssistantEvent::Notice(message) => writeln!(out, "{}", terminal_text(&message))?,
                AssistantEvent::Permission(request) => {
                    if self.permissions.is_empty() {
                        show_permission(&request, out)?;
                    }
                    self.permissions.push_back(request);
                }
                AssistantEvent::Finished(reason) => {
                    self.permissions.clear();
                    match reason {
                        AssistantStopReason::EndTurn => writeln!(out, "Prompt completed.")?,
                        AssistantStopReason::Cancelled => writeln!(out, "Prompt cancelled.")?,
                        other => writeln!(out, "Prompt ended ({other:?}).")?,
                    }
                }
                AssistantEvent::Failed(error) => {
                    self.permissions.clear();
                    writeln!(out, "{}", terminal_text(&error.to_string()))?;
                }
                _ => {}
            }
        }
        let review = self.assistant.proposal();
        if review != self.review {
            if let Some(review) = &review {
                self.end_reply(out)?;
                show_review(review, out)?;
            }
            self.review = review;
        }
        Ok(())
    }
}

fn terminal_text(text: &str) -> String {
    text.chars()
        .filter(|character| !character.is_control() || ['\n', '\t'].contains(character))
        .collect()
}

fn show_permission(request: &AssistantPermission, out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "The agent requests permission for this operation.")?;
    serde_json::to_writer_pretty(&mut *out, &request.tool_call)?;
    writeln!(out)?;
    for option in &request.options {
        let command = match option.kind {
            AssistantPermissionKind::AllowOnce => "/allow",
            AssistantPermissionKind::RejectOnce => "/deny",
        };
        let id = terminal_text(&option.id);
        let name = terminal_text(&option.name);
        writeln!(out, "  {command} {id} ({name})")?;
    }
    Ok(())
}

fn show_review(review: &ProposalReview, out: &mut impl Write) -> io::Result<()> {
    writeln!(out, "Draft proposal. Use /apply or /reject.")?;
    for edit in &review.changes {
        match edit {
            DraftEdit::ObservedHorizonDays { current, proposed } => writeln!(
                out,
                "  Observed horizon (days)   {} → {proposed}",
                display_choice(*current)
            )?,
            DraftEdit::ForecastHorizonDays { current, proposed } => writeln!(
                out,
                "  Forecast horizon (days)   {} → {proposed}",
                display_choice(*current)
            )?,
            DraftEdit::ForecastAsset {
                scenario_index,
                scenario_id,
                current,
                proposed,
            } => {
                let name = scenario_id.as_deref().unwrap_or("unnamed");
                writeln!(
                    out,
                    "  Forecast asset for scenario {name} (index {scenario_index})   {} → {proposed}",
                    display_choice(current.as_deref())
                )?;
            }
        }
    }
    for message in &review.readiness {
        writeln!(out, "  Readiness · {message}")?;
    }
    Ok(())
}

fn display_choice(value: Option<impl ToString>) -> String {
    value.map_or_else(|| "—".into(), |value| value.to_string())
}

fn save(session: &Session) -> Result<PathBuf, String> {
    let job = session
        .begin_draft_job()
        .map_err(|error| error.to_string())?;
    let mut draft: Draft = job.input().ok_or("No draft is open.")?.draft.clone();
    let path = draft
        .path
        .clone()
        .ok_or("The working draft has no opened file.")?;
    draft.save(&path, true)?;
    job.replace(session, draft)
        .map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mgb_ops::app::AssistantOptions;
    use std::fs;

    #[test]
    fn dirty_quit_requires_confirmation_whereas_end_of_input_reports_discard() {
        fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let workspace = root.path().canonicalize().unwrap();
        let session = Session::new(InspectionRequest {
            invocation_dir: workspace.clone(),
            workspace: workspace.clone(),
            config: None,
            gpkg_path: None,
            engine_executable: None,
            engine_input_dir: None,
        });
        let path = workspace.join("draft.json");
        let mut draft = Draft::new(workspace.clone(), &workspace);
        draft.save(&path, true).unwrap();
        let version = session.set_draft(draft.clone()).unwrap();
        let original = fs::read(&path).unwrap();
        draft.document.operation.observed_horizon_days = Some(2);
        session.apply_draft_update(version, draft.document).unwrap();
        let agent = AgentPreset::Codex;
        let assistant = Assistant::start_with_options(
            session.clone(),
            agent,
            AssistantOptions {
                executable: Some(workspace.join("missing")),
                ..AssistantOptions::default()
            },
        )
        .unwrap();
        let mut terminal = Terminal {
            session,
            agent,
            assistant,
            permissions: VecDeque::new(),
            review: None,
            confirming_quit: false,
            streamed_text: false,
        };
        let mut out = Vec::new();
        let waiting = !terminal.line("/quit", &mut out).unwrap();
        assert!(waiting);
        assert!(terminal.confirming_quit);
        let cancelled = !terminal.line("no", &mut out).unwrap() && !terminal.confirming_quit;
        assert!(cancelled);
        assert!(terminal.dirty());
        let waiting_again = !terminal.line("/quit", &mut out).unwrap();
        assert!(waiting_again);
        assert!(terminal.line("yes", &mut out).unwrap());
        terminal.input_ended(&mut out).unwrap();
        let output = String::from_utf8(out).unwrap();
        assert!(output.contains("Discard unsaved draft edits and quit?"));
        assert!(output.contains("Quit cancelled."));
        assert!(output.contains("End of input. Discarding unsaved draft edits."));
        assert_eq!(original, fs::read(&path).unwrap());
        terminal.close();
    }
}
