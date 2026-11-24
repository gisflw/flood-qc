use super::text;
use eframe::egui;
use mgb_ops::app::{
    AgentPreset, Assistant, AssistantEvent, AssistantPermission, AssistantPermissionKind,
    AssistantStopReason, DraftEdit, ProposalOutcome, ProposalReview, Session,
};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

const TRANSCRIPT_BYTES: usize = 128 * 1024;
const EVENT_BATCH: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    Operator,
    Reply,
    Status,
}

struct Message {
    kind: MessageKind,
    text: String,
}

#[derive(Default)]
pub(super) struct AssistantPane {
    selected: Option<AgentPreset>,
    conversation: Option<Assistant>,
    closing: Vec<Assistant>,
    prompt: String,
    transcript: VecDeque<Message>,
    transcript_bytes: usize,
    permissions: VecDeque<AssistantPermission>,
    stopping: bool,
}

impl AssistantPane {
    pub(super) fn reset(&mut self) {
        if let Some(mut conversation) = self.conversation.take() {
            conversation.close();
            self.closing.push(conversation);
        }
        self.prompt.clear();
        self.transcript.clear();
        self.transcript_bytes = 0;
        self.permissions.clear();
        self.stopping = false;
    }

    pub(super) fn poll(&mut self, ui: &egui::Context, pt: bool) {
        self.closing
            .retain(|assistant| !assistant.wait_closed(Duration::ZERO));
        let mut drained = false;
        for _ in 0..EVENT_BATCH {
            let Some(event) = self.conversation.as_mut().and_then(Assistant::try_event) else {
                drained = true;
                break;
            };
            self.event(event, pt);
        }
        let needs_poll = self
            .conversation
            .as_ref()
            .is_some_and(|assistant| !assistant.wait_closed(Duration::ZERO));
        if !drained {
            ui.request_repaint();
        } else if needs_poll || !self.closing.is_empty() {
            ui.request_repaint_after(Duration::from_millis(40));
        }
    }

    fn event(&mut self, event: AssistantEvent, pt: bool) {
        let status = match event {
            AssistantEvent::Ready => text(pt, "Conversation ready.", "Conversa pronta."),
            AssistantEvent::ToolsConnected => text(
                pt,
                "Workspace tools connected.",
                "Ferramentas da área de trabalho conectadas.",
            ),
            AssistantEvent::Text(chunk) => {
                self.push(MessageKind::Reply, chunk);
                return;
            }
            AssistantEvent::Thought(_) => return,
            AssistantEvent::ToolStatus(update) => {
                let title = update["title"]
                    .as_str()
                    .or_else(|| update["toolCallId"].as_str());
                let Some(title) = title else { return };
                update["status"]
                    .as_str()
                    .map_or_else(|| title.into(), |status| format!("{title} · {status}"))
            }
            AssistantEvent::ToolCallCompleted { tool, command, ok } => {
                let action = command.unwrap_or(tool);
                let outcome = if ok {
                    text(pt, "completed", "concluído")
                } else {
                    text(pt, "returned a finding", "retornou uma ocorrência")
                };
                format!("{action} · {outcome}")
            }
            AssistantEvent::Permission(permission) => {
                self.permissions.push_back(permission);
                return;
            }
            AssistantEvent::Notice(message) => message,
            AssistantEvent::Finished(reason) => {
                self.permissions.clear();
                self.stopping = false;
                match reason {
                    AssistantStopReason::EndTurn => return,
                    AssistantStopReason::Cancelled => {
                        text(pt, "Prompt cancelled.", "Solicitação cancelada.")
                    }
                    AssistantStopReason::Refused => text(
                        pt,
                        "The agent declined the prompt.",
                        "O agente recusou a solicitação.",
                    ),
                    AssistantStopReason::Limit => text(
                        pt,
                        "The agent reached a turn limit.",
                        "O agente atingiu um limite da conversa.",
                    ),
                    AssistantStopReason::Failed => {
                        text(pt, "Prompt failed.", "A solicitação falhou.")
                    }
                }
            }
            AssistantEvent::Failed(error) => {
                self.permissions.clear();
                self.stopping = false;
                if let Some(mut assistant) = self.conversation.take() {
                    assistant.close();
                    self.closing.push(assistant);
                }
                error.to_string()
            }
        };
        self.push(MessageKind::Status, status);
    }

    fn push(&mut self, kind: MessageKind, message: String) {
        self.transcript_bytes += message.len();
        if let Some(previous) = self.transcript.back_mut()
            && kind == MessageKind::Reply
            && previous.kind == kind
        {
            previous.text.push_str(&message);
        } else {
            self.transcript.push_back(Message {
                kind,
                text: message,
            });
        }
        while self.transcript_bytes > TRANSCRIPT_BYTES || self.transcript.len() > 160 {
            if self.transcript.len() == 1 {
                let message = &mut self.transcript[0].text;
                let mut start = message.len() - TRANSCRIPT_BYTES;
                while !message.is_char_boundary(start) {
                    start += 1;
                }
                message.drain(..start);
                self.transcript_bytes = message.len();
                break;
            }
            if let Some(removed) = self.transcript.pop_front() {
                self.transcript_bytes -= removed.text.len();
            }
        }
    }

    fn send(&mut self, session: &Session, pt: bool) {
        let Some(agent) = self.selected else { return };
        if self.conversation.is_none() {
            match Assistant::start(session.clone(), agent) {
                Ok(assistant) => {
                    self.conversation = Some(assistant);
                    let starting = text(pt, "Starting", "Iniciando");
                    self.push(MessageKind::Status, format!("{starting} {}.", agent.name()));
                }
                Err(error) => {
                    self.push(MessageKind::Status, error.to_string());
                    return;
                }
            }
        }
        let assistant = self.conversation.as_mut().expect("conversation started");
        match assistant.prompt(self.prompt.trim().to_owned()) {
            Ok(()) => {
                let prompt = std::mem::take(&mut self.prompt);
                self.push(MessageKind::Operator, prompt);
            }
            Err(error) => self.push(MessageKind::Status, error.to_string()),
        }
    }

    pub(super) fn show(&mut self, ui: &mut egui::Ui, session: Option<&Session>, pt: bool) -> bool {
        let Some(session) = session else {
            ui.label(text(
                pt,
                "Open a workspace to use the assistant.",
                "Abra uma área de trabalho para usar o assistente.",
            ));
            return false;
        };
        self.controls(ui, pt);
        let review = self.conversation.as_ref().and_then(Assistant::proposal);
        let permission = self.permissions.front();
        let mut applied = false;
        let mut decision = None;
        let mut option = None;
        egui::ScrollArea::vertical()
            .id_salt("assistant-transcript")
            .max_height((ui.available_height() - 98.).max(50.))
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for entry in &self.transcript {
                    let message = match entry.kind {
                        MessageKind::Operator => egui::RichText::new(&entry.text).strong(),
                        MessageKind::Reply => egui::RichText::new(&entry.text),
                        MessageKind::Status => egui::RichText::new(&entry.text).small(),
                    };
                    ui.add(egui::Label::new(message).wrap().selectable(true));
                }
                if let Some(request) = permission {
                    option = show_permission(ui, request, pt);
                }
                if let Some(review) = &review {
                    decision = show_review(ui, review, pt);
                }
            });
        if let Some((request, choice)) = option
            && let Some(assistant) = &mut self.conversation
        {
            match assistant.answer_permission(request, &choice) {
                Ok(()) => {
                    self.permissions.pop_front();
                }
                Err(error) => self.push(MessageKind::Status, error.to_string()),
            }
        }
        if let Some(apply) = decision
            && let (Some(assistant), Some(review)) = (&self.conversation, &review)
        {
            let outcome = if apply {
                assistant.apply_proposal(review)
            } else {
                assistant.reject_proposal(review)
            };
            applied = outcome == ProposalOutcome::Applied;
            self.push(MessageKind::Status, outcome_message(outcome, pt));
        }
        let running = self
            .conversation
            .as_ref()
            .is_some_and(Assistant::is_running);
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("assistant-prompt")
            .max_height(62.)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.prompt)
                        .hint_text(text(pt, "Enter a prompt", "Digite uma solicitação"))
                        .desired_width(f32::INFINITY)
                        .desired_rows(2),
                );
            });
        if ui
            .add_enabled(
                self.selected.is_some() && !running && !self.prompt.trim().is_empty(),
                egui::Button::new(text(pt, "Send", "Enviar")),
            )
            .clicked()
        {
            self.send(session, pt);
            ui.ctx().request_repaint();
        }
        applied
    }

    fn controls(&mut self, ui: &mut egui::Ui, pt: bool) {
        let previous = self.selected;
        ui.horizontal_wrapped(|ui| {
            let label = self.selected.map_or_else(
                || text(pt, "Select agent", "Selecionar agente"),
                |agent| agent.name().into(),
            );
            egui::ComboBox::from_id_salt("assistant-agent")
                .selected_text(label)
                .show_ui(ui, |ui| {
                    for agent in [AgentPreset::Codex, AgentPreset::Gemini, AgentPreset::Claude] {
                        ui.selectable_value(&mut self.selected, Some(agent), agent.name());
                    }
                });
            let running = self
                .conversation
                .as_ref()
                .is_some_and(Assistant::is_running);
            if ui
                .add_enabled(
                    running && !self.stopping,
                    egui::Button::new(text(pt, "Stop", "Parar")),
                )
                .clicked()
                && let Some(assistant) = &mut self.conversation
            {
                assistant.stop();
                self.stopping = true;
                self.permissions.clear();
            }
            if ui
                .add_enabled(
                    self.conversation.is_some() || !self.transcript.is_empty(),
                    egui::Button::new(text(pt, "Reset conversation", "Redefinir conversa")),
                )
                .clicked()
            {
                self.reset();
            }
        });
        if self.selected != previous {
            self.reset();
        }
        if self.conversation.is_none() && self.transcript.is_empty() {
            ui.small(text(
                pt,
                "The selected agent and its provider may receive prompts and tool results. Native file, shell, and network permissions depend on the installed agent's configuration.",
                "O agente selecionado e seu provedor podem receber solicitações e resultados de ferramentas. As permissões nativas para arquivos, terminal e rede dependem da configuração do agente instalado.",
            ));
        }
    }
}

impl Drop for AssistantPane {
    fn drop(&mut self) {
        self.reset();
        let deadline = Instant::now() + Duration::from_secs(3);
        for assistant in &self.closing {
            let _ = assistant.wait_closed(deadline.saturating_duration_since(Instant::now()));
        }
    }
}

fn show_permission(
    ui: &mut egui::Ui,
    request: &AssistantPermission,
    pt: bool,
) -> Option<(u64, String)> {
    let mut answer = None;
    ui.group(|ui| {
        ui.strong(text(
            pt,
            "Agent permission request",
            "Solicitação de permissão do agente",
        ));
        let details = serde_json::to_string_pretty(&request.tool_call).unwrap_or_default();
        ui.add(
            egui::Label::new(egui::RichText::new(details).monospace())
                .wrap()
                .selectable(true),
        );
        ui.horizontal_wrapped(|ui| {
            for option in &request.options {
                let action = match option.kind {
                    AssistantPermissionKind::AllowOnce => {
                        text(pt, "Allow once", "Permitir uma vez")
                    }
                    AssistantPermissionKind::RejectOnce => text(pt, "Deny once", "Negar uma vez"),
                };
                let label = if option.name == action {
                    action
                } else {
                    format!("{action} · {}", option.name)
                };
                if ui.button(label).clicked() {
                    answer = Some((request.request, option.id.clone()));
                }
            }
        });
    });
    answer
}

fn show_review(ui: &mut egui::Ui, review: &ProposalReview, pt: bool) -> Option<bool> {
    let mut decision = None;
    ui.group(|ui| {
        ui.strong(text(pt, "Draft proposal", "Proposta de rascunho"));
        egui::Grid::new("assistant-draft-review")
            .num_columns(3)
            .striped(true)
            .max_col_width((ui.available_width() - 18.).max(45.) / 3.)
            .show(ui, |ui| {
                ui.strong(text(pt, "Field", "Campo"));
                ui.strong(text(pt, "Current", "Atual"));
                ui.strong(text(pt, "Proposed", "Proposto"));
                ui.end_row();
                for edit in &review.changes {
                    let (field, current, proposed) = review_values(edit, pt);
                    for value in [field, current, proposed] {
                        ui.add(egui::Label::new(value).wrap().selectable(true));
                    }
                    ui.end_row();
                }
            });
        if !review.readiness.is_empty() {
            ui.strong(text(pt, "Readiness", "Prontidão"));
            for finding in &review.readiness {
                ui.label(finding);
            }
        }
        ui.horizontal(|ui| {
            if ui.button(text(pt, "Apply", "Aplicar")).clicked() {
                decision = Some(true);
            }
            if ui.button(text(pt, "Reject", "Rejeitar")).clicked() {
                decision = Some(false);
            }
        });
    });
    decision
}

fn review_values(edit: &DraftEdit, pt: bool) -> (String, String, String) {
    match edit {
        DraftEdit::ObservedHorizonDays { current, proposed } => (
            text(pt, "Observed horizon (days)", "Horizonte observado (dias)"),
            display_choice(*current),
            proposed.to_string(),
        ),
        DraftEdit::ForecastHorizonDays { current, proposed } => (
            text(pt, "Forecast horizon (days)", "Horizonte previsto (dias)"),
            display_choice(*current),
            proposed.to_string(),
        ),
        DraftEdit::ForecastAsset {
            scenario_index,
            scenario_id,
            current,
            proposed,
        } => {
            let label = text(pt, "Forecast asset", "Ativo previsto");
            let scenario = scenario_id
                .clone()
                .unwrap_or_else(|| text(pt, "Unnamed scenario", "Cenário sem nome"));
            (
                format!("{label} · {scenario} [{scenario_index}]"),
                display_choice(current.as_deref()),
                proposed.clone(),
            )
        }
    }
}

fn display_choice(value: Option<impl ToString>) -> String {
    value.map_or_else(|| "—".into(), |value| value.to_string())
}

fn outcome_message(outcome: ProposalOutcome, pt: bool) -> String {
    if !pt {
        return outcome.to_string();
    }
    match outcome {
        ProposalOutcome::Applied => {
            "Aplicado ao rascunho atual. Use Salvar para gravar a alteração.".into()
        }
        ProposalOutcome::Rejected => "Proposta rejeitada.".into(),
        ProposalOutcome::NoPending => "Nenhuma proposta de rascunho está pendente.".into(),
        ProposalOutcome::Stale => {
            "O rascunho mudou. Solicite uma nova proposta para o rascunho atual.".into()
        }
        ProposalOutcome::Busy => {
            "Há uma tarefa de rascunho em andamento. Tente aplicar após a conclusão.".into()
        }
        ProposalOutcome::Invalid(error) => format!("Proposta de rascunho inválida. {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_transcript_coalesces_streamed_text_and_bounds_utf8() {
        let mut pane = AssistantPane::default();
        pane.event(AssistantEvent::Text("a".into()), false);
        pane.event(AssistantEvent::Text("b".into()), false);
        assert_eq!(pane.transcript.len(), 1);
        assert_eq!(pane.transcript[0].text, "ab");
        pane.event(
            AssistantEvent::Text("🌧".repeat(TRANSCRIPT_BYTES / 4)),
            false,
        );
        assert_eq!(pane.transcript_bytes, TRANSCRIPT_BYTES);
        assert_eq!(pane.transcript[0].text.chars().next(), Some('🌧'));
        pane.reset();
        assert!(pane.transcript.is_empty());
        assert_eq!(pane.transcript_bytes, 0);
    }
}
