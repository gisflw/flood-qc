use super::text;
use eframe::egui;
use mgb_ops::app::{
    AgentPreset, AssistantEvent, Conversation, PermissionOption, SETUP_NOTICE, Session,
};
use serde_json::Value;
use std::time::Duration;

const TRANSCRIPT_LIMIT: usize = 256 * 1024;

#[derive(Default)]
pub struct Pane {
    agent: Option<AgentPreset>,
    conversation: Option<Conversation>,
    draft_prompt: String,
    initial_prompt: Option<String>,
    transcript: String,
    status: String,
    connection: String,
    active: bool,
    permission: Option<Permission>,
}

struct Permission {
    request: String,
    operation: Value,
    options: Vec<PermissionOption>,
}

impl Pane {
    pub fn reset(&mut self) {
        self.conversation = None;
        self.initial_prompt = None;
        self.draft_prompt.clear();
        self.status.clear();
        self.connection.clear();
        self.transcript.clear();
        self.active = false;
        self.permission = None;
    }

    fn append(&mut self, text: &str) {
        self.transcript.push_str(text);
        if self.transcript.len() > TRANSCRIPT_LIMIT {
            let first = self
                .transcript
                .ceil_char_boundary(self.transcript.len() - TRANSCRIPT_LIMIT);
            self.transcript.drain(..first);
        }
    }

    fn prompt(&mut self, prompt: String, session: &Session, agent: AgentPreset) {
        if self.conversation.is_none() {
            match std::env::current_exe() {
                Ok(executable) => {
                    self.status = format!("{} · Starting", agent.label());
                    self.initial_prompt = Some(prompt);
                    self.conversation =
                        Some(Conversation::start(session.clone(), agent, executable));
                }
                Err(error) => self.status = error.to_string(),
            }
            return;
        }
        match self.conversation.as_ref().unwrap().prompt(prompt.clone()) {
            Ok(()) => {
                self.active = true;
                self.append(&format!("\n\nYou\n{prompt}\n\n{}\n", agent.label()));
                self.status.clear();
            }
            Err(error) => {
                self.status = error;
                self.draft_prompt = prompt;
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Some(event) = self.conversation.as_mut().and_then(Conversation::try_event) {
            match event {
                AssistantEvent::Handshake => self.status = "ACP v1 connected".into(),
                AssistantEvent::Ready => {
                    self.status = "Ready".into();
                    if let Some(prompt) = self.initial_prompt.take() {
                        let agent = self.agent.unwrap();
                        match self.conversation.as_ref().unwrap().prompt(prompt.clone()) {
                            Ok(()) => {
                                self.active = true;
                                self.append(&format!("You\n{prompt}\n\n{}\n", agent.label()));
                            }
                            Err(error) => {
                                self.status = error;
                                self.draft_prompt = prompt;
                            }
                        }
                    }
                }
                AssistantEvent::ToolConnected => self.connection = "MGB Ops tools connected".into(),
                AssistantEvent::Text(chunk) => self.append(&chunk),
                AssistantEvent::ToolStatus(status) => self.status = status,
                AssistantEvent::Permission {
                    request,
                    operation,
                    options,
                } => {
                    self.permission = Some(Permission {
                        request,
                        operation,
                        options,
                    });
                }
                AssistantEvent::PermissionCleared => self.permission = None,
                AssistantEvent::ReviewStaged => {}
                AssistantEvent::Finished { cancelled } => {
                    self.active = false;
                    self.permission = None;
                    if cancelled {
                        self.status = "Prompt stopped".into();
                    }
                }
                AssistantEvent::Failed {
                    stage,
                    message,
                    closed,
                } => {
                    self.status = format!("{} · {stage} · {message}", self.agent.unwrap().label());
                    if closed {
                        self.active = false;
                        self.permission = None;
                        self.connection.clear();
                        if let Some(prompt) = self.initial_prompt.take() {
                            self.draft_prompt = prompt;
                        }
                        self.conversation = None;
                    }
                }
            }
        }
        if self.conversation.is_some() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, pt: bool, session: Option<&Session>) -> bool {
        let Some(session) = session else {
            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
            return false;
        };
        let old_agent = self.agent;
        let mut applied = false;
        egui::ScrollArea::vertical().id_salt("assistant-pane").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_label(text(pt, "Agent", "Agente"))
                    .selected_text(self.agent.map_or_else(|| text(pt, "Select an agent", "Selecione um agente"), |agent| agent.label().into()))
                    .show_ui(ui, |ui| {
                        for agent in AgentPreset::ALL {
                            ui.selectable_value(&mut self.agent, Some(agent), agent.label());
                        }
                    });
                if ui.button(text(pt, "Reset conversation", "Reiniciar conversa")).clicked() || old_agent != self.agent {
                    self.reset();
                }
                if ui.add_enabled(self.active, egui::Button::new(text(pt, "Stop", "Parar"))).clicked()
                    && let Some(conversation) = &self.conversation
                {
                    match conversation.stop() {
                        Ok(()) => self.status = text(pt, "Stopping prompt", "Interrompendo solicitação"),
                        Err(error) => self.status = error,
                    }
                }
            });
            if self.conversation.is_none() {
                ui.small(text(pt, SETUP_NOTICE, "O agente e seu provedor podem receber as suas solicitações e os resultados das ferramentas do MGB Ops. A configuração do agente instalado controla suas permissões de arquivos, terminal e rede."));
            }
            if !self.connection.is_empty() {
                ui.small(&self.connection);
            }
            if !self.status.is_empty() {
                ui.label(&self.status);
            }
            egui::ScrollArea::vertical().id_salt("assistant-transcript").max_height(250.).stick_to_bottom(true).show(ui, |ui| {
                ui.label(egui::RichText::new(&self.transcript).monospace());
            });
            let can_prompt = self.agent.is_some() && self.initial_prompt.is_none() && !self.active;
            let prompt_label = ui.label(text(pt, "Prompt", "Solicitação"));
            ui.add_enabled(can_prompt, egui::TextEdit::multiline(&mut self.draft_prompt).desired_rows(3).desired_width(f32::INFINITY)).labelled_by(prompt_label.id);
            if ui.add_enabled(can_prompt && !self.draft_prompt.trim().is_empty(), egui::Button::new(text(pt, "Send", "Enviar"))).clicked() {
                let prompt = std::mem::take(&mut self.draft_prompt);
                self.prompt(prompt, session, self.agent.unwrap());
                ui.ctx().request_repaint();
            }
            if let (Some(permission), Some(conversation)) = (&self.permission, &self.conversation) {
                ui.separator();
                ui.strong(text(pt, "Permission requested", "Permissão solicitada"));
                ui.monospace(format!("{:#}", permission.operation));
                ui.horizontal_wrapped(|ui| {
                    for option in &permission.options {
                        if ui.button(&option.label).clicked() {
                            match conversation.answer(permission.request.clone(), option.id.clone()) {
                                Ok(()) => self.status = text(pt, "Permission answer sent", "Resposta de permissão enviada"),
                                Err(error) => self.status = error,
                            }
                        }
                    }
                });
            }
            if let Some(conversation) = &self.conversation
                && let Some(review) = conversation.review()
            {
                ui.separator();
                ui.strong(text(pt, "Draft proposal", "Proposta de rascunho"));
                for change in review.changes {
                    let field = match change.field {
                        "observed_horizon_days" => text(pt, "Observed days", "Dias observados"),
                        "forecast_horizon_days" => text(pt, "Forecast days", "Dias de previsão"),
                        _ => text(pt, "Forecast asset", "Ativo de previsão"),
                    };
                    let scenario = change.scenario_index.map_or(String::new(), |index| {
                        format!(" · {} {} ({index})", text(pt, "Scenario", "Cenário"), change.scenario_id.as_deref().unwrap_or(""))
                    });
                    ui.label(format!("{field}{scenario}   {} → {}", change.current, change.proposed));
                }
                for finding in review.readiness {
                    ui.small(finding);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(text(pt, "Apply", "Aplicar")).clicked() {
                        self.status = match conversation.apply() {
                            Ok(()) => {
                                applied = true;
                                text(pt, "Applied to the working draft. Save it from Draft.", "Aplicado ao rascunho. Salve-o na aba Rascunho.")
                            }
                            Err(error) => error,
                        };
                    }
                    if ui.button(text(pt, "Reject", "Rejeitar")).clicked() && conversation.reject() {
                        self.status = text(pt, "Draft proposal rejected", "Proposta rejeitada");
                    }
                });
            }
        });
        applied
    }
}
