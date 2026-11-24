use super::{text, theme};
use eframe::egui;
use mgb_ops::app::{
    AgentPreset, AgentSetting, AgentSettings, AssistantEvent, Conversation, PermissionOption,
    SETUP_NOTICE, Session, SettingKind,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

const TRANSCRIPT_LIMIT: usize = 256 * 1024;
const MESSAGE_LIMIT: usize = 80 * 1024;
const TURN_LIMIT: usize = 128;

pub type Preferences = BTreeMap<String, RememberedSettings>;

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RememberedSettings {
    model: Option<String>,
    effort: Option<RememberedEffort>,
}

#[derive(Serialize, Deserialize)]
struct RememberedEffort {
    model: Option<String>,
    value: String,
}

#[derive(Default)]
pub struct Pane {
    agent: Option<AgentPreset>,
    conversation: Option<Conversation>,
    draft_prompt: String,
    turns: VecDeque<Turn>,
    next_turn: u64,
    status: String,
    active: bool,
    ready: bool,
    tools_connected: bool,
    permission: Option<Permission>,
    settings: AgentSettings,
    pending_setting: Option<PendingSetting>,
}

struct PendingSetting {
    kind: SettingKind,
    value: String,
    restoring: bool,
}

struct Permission {
    request: String,
    operation: Value,
    options: Vec<PermissionOption>,
}

struct Turn {
    id: u64,
    prompt: Message,
    answer: Message,
    activity: Message,
    thinking_open: bool,
}

impl Turn {
    fn len(&self) -> usize {
        self.prompt.text.len() + self.answer.text.len() + self.activity.text.len()
    }
}

#[derive(Default)]
struct Message {
    text: String,
    layout: Option<TextLayout>,
}

struct TextLayout {
    width: f32,
    scale: f32,
    color: egui::Color32,
    galley: Arc<egui::Galley>,
}

impl Message {
    fn push(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        if self.text.len() > MESSAGE_LIMIT {
            let start = self
                .text
                .ceil_char_boundary(self.text.len() - MESSAGE_LIMIT);
            self.text.drain(..start);
        }
        self.layout = None;
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        let width = ui.available_width();
        let scale = ui.pixels_per_point();
        let color = ui.visuals().text_color();
        if self.layout.as_ref().is_none_or(|layout| {
            layout.width != width || layout.scale != scale || layout.color != color
        }) {
            let font = egui::TextStyle::Body.resolve(ui.style());
            self.layout = Some(TextLayout {
                width,
                scale,
                color,
                galley: ui.painter().layout(self.text.clone(), font, color, width),
            });
        }
        ui.add(egui::Label::new(self.layout.as_ref().unwrap().galley.clone()).selectable(true));
    }
}

fn agent_id(agent: AgentPreset) -> &'static str {
    match agent {
        AgentPreset::Codex => "codex",
        AgentPreset::Gemini => "gemini",
        AgentPreset::Claude => "claude",
    }
}

impl Pane {
    pub fn reset(&mut self) {
        *self = Self {
            agent: self.agent,
            next_turn: self.next_turn,
            ..Self::default()
        };
    }

    fn connect(&mut self, ctx: &egui::Context, session: &Session) {
        let Some(agent) = self.agent else { return };
        match std::env::current_exe() {
            Ok(executable) => {
                let ctx = ctx.clone();
                self.conversation = Some(Conversation::start_with_wake(
                    session.clone(),
                    agent,
                    executable,
                    move || ctx.request_repaint(),
                ));
                self.status.clear();
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn submit(&mut self) {
        let Some(conversation) = &self.conversation else {
            return;
        };
        match conversation.prompt(self.draft_prompt.clone()) {
            Ok(()) => {
                self.status.clear();
                self.active = true;
                self.turns.push_back(Turn {
                    id: self.next_turn,
                    prompt: Message {
                        text: std::mem::take(&mut self.draft_prompt),
                        layout: None,
                    },
                    answer: Message::default(),
                    activity: Message::default(),
                    thinking_open: false,
                });
                self.next_turn += 1;
                self.trim_transcript();
            }
            Err(error) => self.status = error,
        }
    }

    fn trim_transcript(&mut self) {
        let mut total: usize = self.turns.iter().map(Turn::len).sum();
        while self.turns.len() > 1 && (self.turns.len() > TURN_LIMIT || total > TRANSCRIPT_LIMIT) {
            total -= self.turns.pop_front().unwrap().len();
        }
    }

    fn select(&mut self, kind: SettingKind, value: String, restoring: bool) -> bool {
        let Some(conversation) = &self.conversation else {
            return false;
        };
        match conversation.set_option(kind, value.clone()) {
            Ok(()) => {
                self.pending_setting = Some(PendingSetting {
                    kind,
                    value,
                    restoring,
                });
                true
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    fn model(&self) -> Option<&str> {
        self.settings
            .model
            .as_ref()
            .map(|setting| setting.selected.as_str())
    }

    fn restore_setting(&mut self, kind: SettingKind, value: &str, pt: bool) -> bool {
        if let Some(setting) = self.settings.get(kind) {
            if setting.selected == value {
                return false;
            }
            if setting.choices.iter().any(|choice| choice.id == value) {
                return self.select(kind, value.into(), true);
            }
        }
        self.status = match kind {
            SettingKind::Model => text(
                pt,
                "Saved model is unavailable.",
                "O modelo salvo está indisponível.",
            ),
            SettingKind::Effort => text(
                pt,
                "Saved reasoning effort is unavailable.",
                "O esforço de raciocínio salvo está indisponível.",
            ),
        };
        false
    }

    fn restore_effort(&mut self, preferences: &Preferences, pt: bool) {
        if let Some(effort) = self
            .agent
            .and_then(|agent| preferences.get(agent_id(agent)))
            .and_then(|prefs| prefs.effort.as_ref())
        {
            if effort.model.as_deref() == self.model() {
                self.restore_setting(SettingKind::Effort, &effort.value, pt);
            } else if self.status.is_empty() {
                self.status = text(
                    pt,
                    "Saved reasoning effort belongs to a different model.",
                    "O esforço de raciocínio salvo pertence a outro modelo.",
                );
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context, preferences: &mut Preferences, pt: bool) {
        const EVENTS_PER_FRAME: usize = 128;
        let mut transcript_changed = false;
        let mut drained = 0;
        for _ in 0..EVENTS_PER_FRAME {
            let Some(event) = self.conversation.as_mut().and_then(Conversation::try_event) else {
                break;
            };
            drained += 1;
            match event {
                AssistantEvent::Handshake => {}
                AssistantEvent::Ready => {
                    self.ready = true;
                    let saved_model = self
                        .agent
                        .and_then(|agent| preferences.get(agent_id(agent)))
                        .and_then(|prefs| prefs.model.as_deref());
                    if !saved_model
                        .is_some_and(|value| self.restore_setting(SettingKind::Model, value, pt))
                    {
                        self.restore_effort(preferences, pt);
                    }
                }
                AssistantEvent::Settings(settings) => self.settings = settings,
                AssistantEvent::SettingChanged { settings, error } => {
                    self.settings = settings;
                    if let Some(pending) = self.pending_setting.take() {
                        if let Some(error) = error {
                            self.status = error;
                        } else if !pending.restoring
                            && let Some(agent) = self.agent
                        {
                            self.status.clear();
                            let saved = preferences.entry(agent_id(agent).into()).or_default();
                            match pending.kind {
                                SettingKind::Model => {
                                    saved.model = Some(pending.value);
                                    if saved.effort.as_ref().is_some_and(|effort| {
                                        effort.model.as_deref() != self.model()
                                    }) {
                                        saved.effort = None;
                                    }
                                }
                                SettingKind::Effort => {
                                    saved.effort = Some(RememberedEffort {
                                        model: self.model().map(str::to_owned),
                                        value: pending.value,
                                    });
                                }
                            }
                        }
                        if pending.restoring && pending.kind == SettingKind::Model {
                            self.restore_effort(preferences, pt);
                        }
                    }
                }
                AssistantEvent::ToolConnected => self.tools_connected = true,
                AssistantEvent::Text(chunk) => {
                    if let Some(turn) = self.turns.back_mut() {
                        turn.answer.push(&chunk);
                        transcript_changed = true;
                    }
                }
                AssistantEvent::Thought(chunk) => {
                    if let Some(turn) = self.turns.back_mut() {
                        turn.activity.push(&chunk);
                        transcript_changed = true;
                    }
                }
                AssistantEvent::ToolStatus(status) => {
                    if let Some(turn) = self.turns.back_mut() {
                        let status: String = status.chars().take(500).collect();
                        if !turn.activity.text.is_empty() {
                            turn.activity.push("\n\n");
                        }
                        turn.activity.push(&status);
                        turn.activity.push("\n\n");
                        transcript_changed = true;
                    }
                }
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
                        self.status = text(pt, "Prompt stopped", "Solicitação interrompida");
                    }
                }
                AssistantEvent::Failed {
                    stage,
                    message,
                    closed,
                } => {
                    self.status = format!("{stage} · {message}");
                    if closed {
                        self.active = false;
                        self.ready = false;
                        self.permission = None;
                        self.pending_setting = None;
                        self.settings = AgentSettings::default();
                        self.tools_connected = false;
                        self.conversation = None;
                    }
                }
            }
        }
        if transcript_changed {
            self.trim_transcript();
        }
        if drained == EVENTS_PER_FRAME {
            ctx.request_repaint();
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, pt: bool, session: Option<&Session>) -> bool {
        let Some(session) = session else {
            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
            return false;
        };
        ui.scope(|ui| {
            for style in [
                egui::TextStyle::Body,
                egui::TextStyle::Button,
                egui::TextStyle::Small,
            ] {
                let mut font = style.resolve(ui.style());
                font.family = theme::chat_font();
                ui.style_mut().text_styles.insert(style, font);
            }
            self.header(ui, pt, session);
            let applied = egui::Panel::bottom(ui.id().with("composer"))
                .resizable(false)
                .frame(theme::toolbar(ui.style()))
                .show(ui, |ui| self.composer(ui, pt))
                .inner;
            egui::ScrollArea::vertical()
                .id_salt("assistant-transcript")
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| self.transcript(ui, pt));
            applied
        })
        .inner
    }

    fn header(&mut self, ui: &mut egui::Ui, pt: bool, session: &Session) {
        let old_agent = self.agent;
        ui.horizontal_wrapped(|ui| {
            let label = ui.label(text(pt, "Agent", "Agente"));
            egui::ComboBox::from_id_salt("agent")
                .selected_text(self.agent.map_or_else(
                    || text(pt, "Select an agent", "Selecione um agente"),
                    |agent| agent.label().into(),
                ))
                .show_ui(ui, |ui| {
                    for agent in AgentPreset::ALL {
                        ui.selectable_value(&mut self.agent, Some(agent), agent.label());
                    }
                })
                .response
                .labelled_by(label.id);
            if old_agent != self.agent {
                self.reset();
                self.connect(ui.ctx(), session);
            }
            if self.conversation.is_some() {
                if ui
                    .button(text(pt, "Reset conversation", "Reiniciar conversa"))
                    .clicked()
                {
                    self.reset();
                    self.connect(ui.ctx(), session);
                }
            } else if self.agent.is_some() && ui.button(text(pt, "Connect", "Conectar")).clicked() {
                self.connect(ui.ctx(), session);
            }
        });

        if self.ready {
            ui.columns(2, |columns| {
                for (column, (kind, label)) in columns.iter_mut().zip([
                    (SettingKind::Model, text(pt, "Model", "Modelo")),
                    (
                        SettingKind::Effort,
                        text(pt, "Reasoning effort", "Esforço de raciocínio"),
                    ),
                ]) {
                    if let Some(value) = setting_picker(
                        column,
                        &label,
                        self.settings.get(kind),
                        !self.active && self.pending_setting.is_none(),
                        pt,
                    ) {
                        self.select(kind, value, false);
                    }
                }
            });
        }
        if self.conversation.is_some() && (!self.ready || self.pending_setting.is_some()) {
            ui.horizontal(|ui| {
                ui.spinner();
                if self.ready {
                    ui.small(text(pt, "Updating settings…", "Atualizando configurações…"));
                } else {
                    ui.small(text(pt, "Connecting…", "Conectando…"));
                }
            });
        }
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.separator();
    }

    fn transcript(&mut self, ui: &mut egui::Ui, pt: bool) {
        if self.turns.is_empty() {
            ui.small(text(pt, SETUP_NOTICE, "O agente e seu provedor podem receber as suas solicitações e os resultados das ferramentas do MGB Ops. A configuração do agente instalado controla suas permissões de arquivos, terminal e rede."));
            if self.tools_connected {
                ui.small(text(
                    pt,
                    "MGB Ops tools connected",
                    "Ferramentas do MGB Ops conectadas",
                ));
            }
        }
        let latest = self.turns.back().map(|turn| turn.id);
        for turn in &mut self.turns {
            let busy = self.active && latest == Some(turn.id);
            ui.push_id(turn.id, |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().selection.bg_fill)
                    .inner_margin(8)
                    .corner_radius(6.)
                    .show(ui, |ui| {
                        ui.small(text(pt, "You", "Você"));
                        turn.prompt.show(ui);
                    });
                ui.add_space(4.);
                ui.strong(self.agent.map_or("Assistant", AgentPreset::label));
                if busy || !turn.activity.text.is_empty() {
                    ui.horizontal(|ui| {
                        let icon = if turn.thinking_open { "−" } else { "+" };
                        let label = text(pt, "Thinking", "Raciocínio");
                        let title = format!("{icon} {label}");
                        if ui
                            .add(egui::Button::new(title).small().frame_when_inactive(false))
                            .clicked()
                        {
                            turn.thinking_open = !turn.thinking_open;
                        }
                        if busy && self.permission.is_none() {
                            ui.spinner();
                        }
                    });
                    if turn.thinking_open {
                        ui.indent("activity", |ui| turn.activity.show(ui));
                    }
                }
                if !turn.answer.text.is_empty() {
                    turn.answer.show(ui);
                }
                ui.add_space(8.);
            });
        }
    }

    fn composer(&mut self, ui: &mut egui::Ui, pt: bool) -> bool {
        let has_review = self
            .conversation
            .as_ref()
            .is_some_and(|conversation| conversation.review().is_some());
        let applied = if self.permission.is_some() || has_review {
            egui::ScrollArea::vertical()
                .id_salt("assistant-actions")
                .max_height((ui.available_height() * 0.24).min(170.))
                .show(ui, |ui| self.actions(ui, pt))
                .inner
        } else {
            false
        };
        let label = ui.label(text(pt, "Message", "Mensagem"));
        let editor_height = (ui.available_height() * 0.22).clamp(42., 120.);
        let edit = egui::ScrollArea::vertical()
            .id_salt("message-editor")
            .auto_shrink([false, true])
            .max_height(editor_height)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.draft_prompt)
                        .desired_width(f32::INFINITY)
                        .desired_rows(2)
                        .return_key(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        )),
                )
                .labelled_by(label.id)
            })
            .inner;
        let submit = edit.has_focus() && ui.input(|input| input.events.iter().any(plain_enter));
        let can_send = self.ready
            && !self.active
            && self.pending_setting.is_none()
            && !self.draft_prompt.trim().is_empty();
        ui.horizontal_wrapped(|ui| {
            let send = ui.add_enabled(can_send, theme::primary(ui, text(pt, "Send", "Enviar")));
            if send.clicked() || submit && can_send {
                self.submit();
                edit.request_focus();
                ui.request_repaint();
            }
            if self.active
                && ui.button(text(pt, "Stop", "Parar")).clicked()
                && let Some(conversation) = &self.conversation
            {
                self.status = match conversation.stop() {
                    Ok(()) => text(pt, "Stopping prompt", "Interrompendo solicitação"),
                    Err(error) => error,
                };
            }
        });
        applied
    }

    fn actions(&mut self, ui: &mut egui::Ui, pt: bool) -> bool {
        let mut applied = false;
        if let (Some(permission), Some(conversation)) = (&self.permission, &self.conversation) {
            ui.separator();
            ui.strong(text(pt, "Permission requested", "Permissão solicitada"));
            ui.monospace(format!("{:#}", permission.operation));
            ui.horizontal_wrapped(|ui| {
                for option in &permission.options {
                    if ui.button(&option.label).clicked() {
                        match conversation.answer(permission.request.clone(), option.id.clone()) {
                            Ok(()) => {
                                self.status = text(
                                    pt,
                                    "Permission answer sent",
                                    "Resposta de permissão enviada",
                                )
                            }
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
                    format!(
                        " · {} {} ({index})",
                        text(pt, "Scenario", "Cenário"),
                        change.scenario_id.as_deref().unwrap_or("")
                    )
                });
                ui.label(format!(
                    "{field}{scenario}   {} → {}",
                    change.current, change.proposed
                ));
            }
            for finding in review.readiness {
                ui.small(finding);
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button(text(pt, "Apply", "Aplicar")).clicked() {
                    self.status = match conversation.apply() {
                        Ok(()) => {
                            applied = true;
                            text(
                                pt,
                                "Applied to the working draft. Save it from Draft.",
                                "Aplicado ao rascunho. Salve-o na aba Rascunho.",
                            )
                        }
                        Err(error) => error,
                    };
                }
                if ui.button(text(pt, "Reject", "Rejeitar")).clicked() && conversation.reject() {
                    self.status = text(pt, "Draft proposal rejected", "Proposta rejeitada");
                }
            });
        }
        applied
    }
}

fn plain_enter(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Key {
            key: egui::Key::Enter,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
            ..
        }
    )
}

fn setting_picker(
    ui: &mut egui::Ui,
    label: &str,
    setting: Option<&AgentSetting>,
    enabled: bool,
    pt: bool,
) -> Option<String> {
    let label = ui.label(label);
    let Some(setting) = setting else {
        ui.weak(text(pt, "Unavailable", "Indisponível"))
            .labelled_by(label.id);
        return None;
    };
    let current = setting
        .choices
        .iter()
        .find(|choice| choice.id == setting.selected)
        .map_or(setting.selected.as_str(), |choice| choice.label.as_str());
    let mut value = None;
    ui.add_enabled_ui(enabled && !setting.choices.is_empty(), |ui| {
        egui::ComboBox::from_id_salt(label.id)
            .width(ui.available_width().clamp(0., 156.))
            .truncate()
            .selected_text(current)
            .show_ui(ui, |ui| {
                for choice in &setting.choices {
                    let selected = choice.id == setting.selected;
                    if ui.selectable_label(selected, &choice.label).clicked() && !selected {
                        value = Some(choice.id.clone());
                    }
                }
            })
            .response
            .labelled_by(label.id);
    });
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composer_distinguishes_plain_enter_from_newlines_and_key_repeat() {
        let enter = |modifiers, repeat| egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat,
            modifiers,
        };
        assert_eq!(
            [
                plain_enter(&enter(egui::Modifiers::NONE, false)),
                plain_enter(&enter(egui::Modifiers::SHIFT, false)),
                plain_enter(&enter(egui::Modifiers::CTRL, false)),
                plain_enter(&enter(egui::Modifiers::NONE, true)),
            ],
            [true, false, false, false],
        );
    }
}
