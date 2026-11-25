use super::{text, theme};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use mgb_ops::app::{
    AgentPreset, AgentSetting, AgentSettings, AssistantEvent, Conversation, PermissionOption,
    SETUP_NOTICE, Session, SettingKind,
};
use mgb_ops::store::{ChatHistory, ChatHistoryStore, SavedActivity, SavedChat, SavedTurn};
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::{Duration, Instant},
};

const EDITOR_ROWS: usize = 2;
const EDITOR_MARGIN: egui::Margin = egui::Margin::symmetric(8, 5);

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
    editor_content_height: f32,
    turns: VecDeque<Turn>,
    next_turn: u64,
    status: String,
    active: bool,
    ready: bool,
    tools_connected: bool,
    permission: Option<Permission>,
    settings: AgentSettings,
    pending_setting: Option<PendingSetting>,
    data_dir: Option<PathBuf>,
    workspace: Option<PathBuf>,
    store: Option<ChatHistoryStore>,
    writer: Option<HistoryWriter>,
    history: ChatHistory,
    selected: Option<usize>,
    history_open: bool,
    renaming: Option<String>,
    rename_focus: bool,
    transcript_away: bool,
    transcript_end: f32,
    resume_failed: bool,
    carry_history: bool,
    history_deadline: Option<Instant>,
    visible_turns: usize,
}

#[derive(Default)]
struct HistoryWriteState {
    pending: Option<ChatHistory>,
    stopping: bool,
    error: Option<String>,
}
struct HistoryWriter {
    state: Arc<(Mutex<HistoryWriteState>, Condvar)>,
    worker: Option<thread::JoinHandle<()>>,
}
impl HistoryWriter {
    fn start(store: ChatHistoryStore, ctx: egui::Context) -> Self {
        let state = Arc::new((Mutex::new(HistoryWriteState::default()), Condvar::new()));
        let shared = state.clone();
        let worker = thread::spawn(move || {
            loop {
                let (lock, wake) = &*shared;
                let mut state = lock.lock().unwrap();
                while state.pending.is_none() && !state.stopping {
                    state = wake.wait(state).unwrap();
                }
                let pending = state.pending.take();
                if pending.is_none() && state.stopping {
                    break;
                }
                drop(state);
                if let Some(history) = pending
                    && let Err(error) = store.save(&history)
                {
                    lock.lock().unwrap().error = Some(error);
                    ctx.request_repaint();
                }
            }
        });
        Self {
            state,
            worker: Some(worker),
        }
    }
    fn schedule(&self, history: ChatHistory) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().pending = Some(history);
        wake.notify_one();
    }
    fn take_error(&self) -> Option<String> {
        self.state.0.lock().unwrap().error.take()
    }
}
impl Drop for HistoryWriter {
    fn drop(&mut self) {
        let (lock, wake) = &*self.state;
        lock.lock().unwrap().stopping = true;
        wake.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
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
    answer: MarkdownReply,
    activity: Vec<Activity>,
    thinking_open: bool,
    interrupted: bool,
    layout: Option<TurnLayout>,
}

impl Turn {
    fn invalidate_layout(&mut self) {
        if let Some(layout) = &mut self.layout {
            layout.dirty = true;
        }
    }
}

enum Activity {
    Thought(MarkdownReply),
    Tool {
        id: Option<String>,
        title: Option<String>,
        status: Option<String>,
    },
}

#[derive(Clone, Copy, PartialEq)]
struct TurnGeometry {
    width: f32,
    scale: f32,
    dark: bool,
    pt: bool,
    busy: bool,
}

struct TurnLayout {
    geometry: TurnGeometry,
    height: f32,
    dirty: bool,
}

#[derive(Default)]
struct MarkdownReply {
    text: String,
    cache: CommonMarkCache,
    dirty: bool,
    fitted: Option<Rc<FittedMarkdown>>,
    table_cache: Rc<RefCell<CommonMarkCache>>,
}

fn append_text(message: &mut String, chunk: &str) {
    message.push_str(chunk);
}

#[derive(Default)]
struct MarkdownTable {
    headings: Vec<String>,
    rows: Vec<Vec<String>>,
}

struct FittedMarkdown {
    source: String,
    marker: String,
    definitions: String,
    tables: Vec<MarkdownTable>,
}

impl MarkdownReply {
    fn push(&mut self, chunk: &str) {
        if !chunk.is_empty() {
            append_text(&mut self.text, chunk);
            self.dirty = true;
        }
    }

    fn prepare(&mut self) {
        self.cache.link_hooks_clear();
        let mut table_cache = self.table_cache.borrow_mut();
        table_cache.link_hooks_clear();
        self.fitted = None;
        self.dirty = false;
        if !self.text.contains(['[', '<', '|']) {
            return;
        }

        let options = Options::ENABLE_TABLES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_FOOTNOTES
            | Options::ENABLE_DEFINITION_LIST;
        let parser = Parser::new_ext(&self.text, options);
        let mut fitted = FittedMarkdown {
            source: String::new(),
            marker: "mgb-ops-table-".into(),
            definitions: parser
                .reference_definitions()
                .iter()
                .map(|(_, link)| &self.text[link.span.clone()])
                .collect::<Vec<_>>()
                .join("\n"),
            tables: Vec::new(),
        };
        while self.text.contains(&fitted.marker) {
            fitted.marker.push('-');
        }
        let mut last_end = 0;
        let mut table: Option<MarkdownTable> = None;
        for (event, span) in parser.into_offset_iter() {
            match event {
                Event::Start(Tag::Link { dest_url, .. })
                    if !dest_url.starts_with("https://") && !dest_url.starts_with("http://") =>
                {
                    self.cache.add_link_hook(dest_url.to_string());
                    table_cache.add_link_hook(dest_url.into_string());
                }
                Event::Start(Tag::Table(_)) => {
                    fitted.source.push_str(&self.text[last_end..span.start]);
                    fitted.source.push_str(&format!(
                        "<!--{}{}-->\n\n",
                        fitted.marker,
                        fitted.tables.len()
                    ));
                    last_end = span.end;
                    table = Some(MarkdownTable::default());
                }
                Event::Start(Tag::TableRow) => {
                    if let Some(table) = &mut table {
                        table.rows.push(Vec::new());
                    }
                }
                Event::Start(Tag::TableCell) => {
                    if let Some(table) = &mut table {
                        table
                            .rows
                            .last_mut()
                            .unwrap_or(&mut table.headings)
                            .push(self.text[span].trim().into());
                    }
                }
                Event::End(TagEnd::Table) => {
                    if let Some(table) = table.take() {
                        fitted.tables.push(table);
                    }
                }
                _ => {}
            }
        }
        if !fitted.tables.is_empty() {
            fitted.source.push_str(&self.text[last_end..]);
            self.fitted = Some(Rc::new(fitted));
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        if self.dirty {
            self.prepare();
        }
        let viewer = CommonMarkViewer::new().explicit_image_uri_scheme(true);
        if let Some(fitted) = self.fitted.clone() {
            let tables = fitted.clone();
            let cache = self.table_cache.clone();
            let html = move |ui: &mut egui::Ui, html: &str| {
                let table = html
                    .trim()
                    .strip_prefix("<!--")
                    .and_then(|html| html.strip_prefix(&tables.marker))
                    .and_then(|html| html.strip_suffix("-->"))
                    .and_then(|index| index.parse::<usize>().ok())
                    .and_then(|index| tables.tables.get(index));
                if let Some(table) = table {
                    let mut cache = cache.borrow_mut();
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.;
                        theme::fitted_table(
                            ui,
                            &table.headings,
                            &table.rows,
                            |ui, cell, heading| {
                                let source = if cell.contains('[') && !tables.definitions.is_empty()
                                {
                                    std::borrow::Cow::Owned(format!(
                                        "{cell}\n\n{}",
                                        tables.definitions
                                    ))
                                } else {
                                    std::borrow::Cow::Borrowed(cell)
                                };
                                if heading {
                                    ui.visuals_mut().override_text_color =
                                        Some(ui.visuals().strong_text_color());
                                }
                                CommonMarkViewer::new()
                                    .explicit_image_uri_scheme(true)
                                    .show(ui, &mut cache, &source);
                            },
                        );
                    });
                } else {
                    ui.label(html);
                }
            };
            viewer
                .render_html_fn(Some(&html))
                .show(ui, &mut self.cache, &fitted.source);
        } else {
            viewer.show(ui, &mut self.cache, &self.text);
        }
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
    pub fn set_data_dir(&mut self, path: Option<PathBuf>) {
        self.data_dir = path;
    }

    fn ensure_workspace(&mut self, session: &Session, ctx: &egui::Context) {
        if self.workspace.as_deref() == Some(session.workspace()) {
            return;
        }
        self.conversation = None;
        self.workspace = Some(session.workspace().to_path_buf());
        self.turns.clear();
        self.draft_prompt.clear();
        self.selected = None;
        self.writer = None;
        self.store = None;
        self.history = ChatHistory::default();
        if let Some(root) = &self.data_dir {
            match ChatHistoryStore::open(root, session.workspace()).and_then(|store| {
                let history = store.load()?;
                Ok((store, history))
            }) {
                Ok((store, history)) => {
                    self.writer = Some(HistoryWriter::start(store.clone(), ctx.clone()));
                    self.store = Some(store);
                    self.history = history;
                    if let Some(index) =
                        self.history.selected.as_ref().and_then(|id| {
                            self.history.chats.iter().position(|chat| &chat.id == id)
                        })
                    {
                        self.choose_chat(index, ctx, session);
                    }
                }
                Err(error) => self.status = format!("Chat history: {error}"),
            }
        } else {
            self.status =
                "Chat history cannot be saved: application data directory is unavailable.".into();
        }
    }

    fn save_history(&mut self) {
        self.history_deadline = None;
        if let Some(chat) = self.selected.and_then(|i| self.history.chats.get_mut(i)) {
            if chat.title == "New chat"
                && chat.turns.is_empty()
                && let Some(first) = self.turns.front()
            {
                chat.title = first
                    .prompt
                    .text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(60)
                    .collect();
            }
            chat.draft = self.draft_prompt.clone();
            chat.carry_history = self.carry_history;
            chat.turns = self
                .turns
                .iter()
                .map(|turn| SavedTurn {
                    prompt: turn.prompt.text.clone(),
                    answer: turn.answer.text.clone(),
                    activity: turn
                        .activity
                        .iter()
                        .map(|activity| match activity {
                            Activity::Thought(reply) => SavedActivity::Thought(reply.text.clone()),
                            Activity::Tool { id, title, status } => SavedActivity::Tool {
                                id: id.clone(),
                                title: title.clone(),
                                status: status.clone(),
                            },
                        })
                        .collect(),
                    interrupted: turn.interrupted
                        || (self.active
                            && self.turns.back().is_some_and(|last| last.id == turn.id)),
                })
                .collect();
        }
        self.history.selected = self
            .selected
            .and_then(|i| self.history.chats.get(i))
            .map(|chat| chat.id.clone());
        if let Some(writer) = &self.writer {
            writer.schedule(self.history.clone());
        }
    }

    fn defer_history(&mut self, ctx: &egui::Context) {
        const DELAY: Duration = Duration::from_millis(300);
        self.history_deadline = Some(Instant::now() + DELAY);
        ctx.request_repaint_after(DELAY);
    }

    fn choose_chat(&mut self, index: usize, ctx: &egui::Context, session: &Session) {
        self.save_history();
        self.conversation = None;
        self.ready = false;
        self.active = false;
        self.permission = None;
        self.tools_connected = false;
        self.settings = AgentSettings::default();
        self.pending_setting = None;
        self.resume_failed = false;
        self.carry_history = false;
        self.visible_turns = 40;
        self.status.clear();
        let Some(chat) = self.history.chats.get(index) else {
            return;
        };
        self.agent = match chat.agent.as_str() {
            "codex" => Some(AgentPreset::Codex),
            "gemini" => Some(AgentPreset::Gemini),
            "claude" => Some(AgentPreset::Claude),
            _ => None,
        };
        self.draft_prompt = chat.draft.clone();
        self.carry_history = chat.carry_history;
        self.turns = chat
            .turns
            .iter()
            .enumerate()
            .map(|(id, saved)| {
                let mut answer = MarkdownReply::default();
                answer.push(&saved.answer);
                let activity = saved
                    .activity
                    .iter()
                    .map(|item| match item {
                        SavedActivity::Thought(text) => {
                            let mut reply = MarkdownReply::default();
                            reply.push(text);
                            Activity::Thought(reply)
                        }
                        SavedActivity::Tool { id, title, status } => Activity::Tool {
                            id: id.clone(),
                            title: title.clone(),
                            status: status.clone(),
                        },
                    })
                    .collect();
                Turn {
                    id: id as u64,
                    prompt: Message {
                        text: saved.prompt.clone(),
                        layout: None,
                    },
                    answer,
                    activity,
                    thinking_open: false,
                    interrupted: saved.interrupted,
                    layout: None,
                }
            })
            .collect();
        self.next_turn = self.turns.len() as u64;
        self.selected = Some(index);
        self.save_history();
        self.connect(ctx, session);
    }

    fn new_chat(&mut self, ctx: &egui::Context, session: &Session) {
        self.save_history();
        self.conversation = None;
        self.ready = false;
        self.active = false;
        self.permission = None;
        self.tools_connected = false;
        self.settings = AgentSettings::default();
        self.pending_setting = None;
        self.resume_failed = false;
        self.carry_history = false;
        self.visible_turns = 40;
        self.status.clear();
        self.turns.clear();
        self.draft_prompt.clear();
        self.selected = None;
        let Some(agent) = self.agent else {
            return;
        };
        let Some(store) = &self.store else {
            self.connect(ctx, session);
            self.status =
                "Chat history is unavailable; this conversation is only in memory.".into();
            return;
        };
        match store.new_cwd() {
            Ok(cwd) => {
                let id = cwd.file_name().unwrap().to_string_lossy().into_owned();
                self.history.chats.push(SavedChat {
                    id,
                    title: "New chat".into(),
                    agent: agent_id(agent).into(),
                    cwd,
                    ..SavedChat::default()
                });
                self.selected = Some(self.history.chats.len() - 1);
                self.save_history();
                self.connect(ctx, session);
            }
            Err(error) => self.status = format!("Chat history: {error}"),
        }
    }

    pub fn reset(&mut self) {
        self.save_history();
        *self = Self {
            agent: self.agent,
            data_dir: self.data_dir.clone(),
            next_turn: self.next_turn,
            ..Self::default()
        };
    }

    fn connect(&mut self, ctx: &egui::Context, session: &Session) {
        let Some(agent) = self.agent else { return };
        if let Some(chat) = self
            .selected
            .and_then(|index| self.history.chats.get(index))
            && self
                .store
                .as_ref()
                .is_none_or(|store| !store.owns_cwd(&chat.cwd))
        {
            self.status = "The conversation directory is unavailable.".into();
            self.resume_failed = true;
            return;
        }
        match std::env::current_exe() {
            Ok(executable) => {
                let ctx = ctx.clone();
                self.conversation = Some(
                    if let Some(chat) = self.selected.and_then(|i| self.history.chats.get(i)) {
                        Conversation::start_saved_with_wake(
                            session.clone(),
                            agent,
                            executable,
                            chat.cwd.clone(),
                            chat.session_id.clone(),
                            move || ctx.request_repaint(),
                        )
                    } else {
                        Conversation::start_with_wake(
                            session.clone(),
                            agent,
                            executable,
                            move || ctx.request_repaint(),
                        )
                    },
                );
                if self.store.is_some() {
                    self.status.clear();
                }
            }
            Err(error) => self.status = error.to_string(),
        }
    }

    fn submit(&mut self) {
        let Some(conversation) = &self.conversation else {
            return;
        };
        let (prompt, omitted) = if self.carry_history {
            carry_prompt(&self.turns, &self.draft_prompt)
        } else {
            (self.draft_prompt.clone(), false)
        };
        match conversation.prompt(prompt) {
            Ok(()) => {
                self.status = if omitted {
                    "Some older conversation context was omitted to fit the prompt limit.".into()
                } else {
                    String::new()
                };
                self.carry_history = false;
                self.active = true;
                self.turns.push_back(Turn {
                    id: self.next_turn,
                    prompt: Message {
                        text: std::mem::take(&mut self.draft_prompt),
                        layout: None,
                    },
                    answer: MarkdownReply::default(),
                    activity: Vec::new(),
                    thinking_open: false,
                    interrupted: false,
                    layout: None,
                });
                self.next_turn += 1;
                self.save_history();
            }
            Err(error) => self.status = error,
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
        if let Some(error) = self.writer.as_ref().and_then(HistoryWriter::take_error) {
            self.status = format!("Chat history: {error}");
        }
        let mut transcript_changed = false;
        let mut checkpoint = false;
        let mut drained = 0;
        for _ in 0..EVENTS_PER_FRAME {
            let Some(event) = self.conversation.as_mut().and_then(Conversation::try_event) else {
                break;
            };
            drained += 1;
            match event {
                AssistantEvent::Handshake => {}
                AssistantEvent::SessionStarted(id) => {
                    if let Some(chat) = self.selected.and_then(|i| self.history.chats.get_mut(i)) {
                        chat.session_id = Some(id);
                    }
                    self.save_history();
                }
                AssistantEvent::ResumeUnavailable(message) => {
                    self.resume_failed = true;
                    self.status = message;
                    self.ready = false;
                    self.tools_connected = false;
                    self.permission = None;
                    self.conversation = None;
                }
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
                        turn.invalidate_layout();
                        transcript_changed = true;
                    }
                }
                AssistantEvent::Thought(chunk) => {
                    if let Some(turn) = self.turns.back_mut() {
                        if let Some(Activity::Thought(reply)) = turn.activity.last_mut() {
                            reply.push(&chunk);
                        } else {
                            let mut reply = MarkdownReply::default();
                            reply.push(&chunk);
                            turn.activity.push(Activity::Thought(reply));
                        }
                        turn.invalidate_layout();
                        transcript_changed = true;
                    }
                }
                AssistantEvent::ToolStatus { id, title, status } => {
                    if let Some(turn) = self.turns.back_mut() {
                        let title = title.or_else(|| {
                            id.as_ref().and_then(|id| {
                                turn.activity
                                    .iter()
                                    .rev()
                                    .find_map(|activity| match activity {
                                        Activity::Tool {
                                            id: Some(previous),
                                            title: Some(title),
                                            ..
                                        } if previous == id => Some(title.clone()),
                                        _ => None,
                                    })
                            })
                        });
                        turn.activity.push(Activity::Tool { id, title, status });
                        turn.invalidate_layout();
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
                    checkpoint = true;
                    self.active = false;
                    self.permission = None;
                    if let Some(turn) = self.turns.back_mut() {
                        turn.interrupted = cancelled;
                    }
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
                        checkpoint = true;
                        if self.active
                            && let Some(turn) = self.turns.back_mut()
                        {
                            turn.interrupted = true;
                        }
                        if !self.ready
                            && self
                                .selected
                                .and_then(|i| self.history.chats.get(i))
                                .is_some_and(|chat| chat.session_id.is_some())
                        {
                            self.resume_failed = true;
                        }
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
        if checkpoint {
            self.save_history();
        } else if transcript_changed {
            self.defer_history(ctx);
        } else if let Some(deadline) = self.history_deadline {
            let now = Instant::now();
            if now >= deadline {
                self.save_history();
            } else {
                ctx.request_repaint_after(deadline.duration_since(now));
            }
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
        self.ensure_workspace(session, ui.ctx());
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
            theme::fitted_columns(ui, &[1.], |ui, _| self.header(ui, pt, session));
            let applied = self.composer(ui, pt);
            let latest = self.transcript_away
                && ui
                    .button(text(pt, "Latest message", "Mensagem mais recente"))
                    .clicked();
            let mut scroll = egui::ScrollArea::vertical()
                .id_salt("assistant-transcript")
                .auto_shrink([false, false])
                .stick_to_bottom(true);
            if latest {
                scroll = scroll.vertical_scroll_offset(self.transcript_end);
            }
            let output = scroll.show(ui, |ui| self.transcript(ui, pt));
            self.transcript_end = (output.content_size.y - output.inner_rect.height()).max(0.);
            self.transcript_away = self.transcript_end - output.state.offset.y > 8.;
            applied
        })
        .inner
    }

    fn header(&mut self, ui: &mut egui::Ui, pt: bool, session: &Session) {
        ui.horizontal_wrapped(|ui| {
            if ui.button(text(pt, "History", "Histórico")).clicked() {
                self.history_open = !self.history_open;
            }
            let old_agent = self.agent;
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
            if self.agent != old_agent {
                self.new_chat(ui.ctx(), session);
            }
            if ui
                .add_enabled(
                    self.agent.is_some(),
                    egui::Button::new(text(pt, "New chat", "Nova conversa")),
                )
                .clicked()
            {
                self.new_chat(ui.ctx(), session);
            }
        });
        if let Some(chat) = self
            .selected
            .and_then(|index| self.history.chats.get(index))
        {
            ui.strong(&chat.title);
        }
        if self.history_open {
            let mut choose = None;
            let mut delete = None;
            let mut rename = None;
            let mut renamed = false;
            egui::ScrollArea::vertical()
                .id_salt("chat-history")
                .max_height(180.)
                .show(ui, |ui| {
                    for (index, chat) in self.history.chats.iter_mut().enumerate() {
                        ui.push_id(chat.id.clone(), |ui| {
                            ui.horizontal(|ui| {
                                if self.renaming.as_deref() == Some(chat.id.as_str()) {
                                    let edit = ui.add(
                                        egui::TextEdit::singleline(&mut chat.title)
                                            .desired_width((ui.available_width() - 110.).max(30.)),
                                    );
                                    renamed |= edit.changed();
                                    if self.rename_focus {
                                        edit.request_focus();
                                        self.rename_focus = false;
                                    }
                                    if edit.lost_focus() {
                                        rename = Some(None);
                                    }
                                } else if ui
                                    .add_sized(
                                        egui::vec2(
                                            (ui.available_width() - 110.).max(30.),
                                            ui.spacing().interact_size.y,
                                        ),
                                        egui::Button::selectable(
                                            self.selected == Some(index),
                                            &chat.title,
                                        )
                                        .wrap_mode(egui::TextWrapMode::Truncate),
                                    )
                                    .on_hover_text(&chat.title)
                                    .clicked()
                                {
                                    choose = Some(index);
                                }
                                ui.weak(&chat.agent);
                                ui.menu_button("⋯", |ui| {
                                    if ui.button(text(pt, "Rename", "Renomear")).clicked() {
                                        rename = Some(Some(chat.id.clone()));
                                        ui.close();
                                    }
                                    if ui.button(text(pt, "Delete", "Excluir")).clicked() {
                                        delete = Some(index);
                                        ui.close();
                                    }
                                });
                            });
                        });
                    }
                });
            if let Some(value) = rename {
                self.rename_focus = value.is_some();
                self.renaming = value;
            }
            if renamed {
                self.save_history();
            }
            if let Some(index) = choose {
                self.choose_chat(index, ui.ctx(), session);
            }
            if let Some(index) = delete {
                self.renaming = None;
                if self.selected == Some(index) {
                    self.conversation = None;
                    self.permission = None;
                    self.pending_setting = None;
                    self.resume_failed = false;
                    self.carry_history = false;
                    self.tools_connected = false;
                    self.settings = AgentSettings::default();
                    self.status.clear();
                    self.turns.clear();
                    self.draft_prompt.clear();
                    self.selected = None;
                    self.ready = false;
                    self.active = false;
                } else if let Some(selected) = &mut self.selected
                    && *selected > index
                {
                    *selected -= 1;
                }
                let chat = self.history.chats.remove(index);
                if let Some(store) = &self.store
                    && let Err(error) = store.remove_cwd(&chat.cwd)
                {
                    self.status = error;
                }
                self.save_history();
            }
        }
        if self.carry_history {
            ui.weak(text(pt, "The next message includes available prior text; older context may be omitted to fit the prompt limit.", "A próxima mensagem inclui o texto anterior disponível; parte do contexto mais antigo pode ser omitida para respeitar o limite."));
        }
        if self.resume_failed {
            ui.label(text(pt, "The saved session could not be resumed. You can continue in a new session using the visible transcript as context; tool state is not restored.", "Não foi possível retomar a sessão salva. Você pode continuar em uma nova sessão usando a conversa como contexto; o estado das ferramentas não é restaurado."));
            if ui
                .button(text(
                    pt,
                    "Continue with transcript",
                    "Continuar com a conversa",
                ))
                .clicked()
            {
                self.conversation = None;
                self.resume_failed = false;
                self.carry_history = true;
                if let Some(chat) = self.selected.and_then(|i| self.history.chats.get_mut(i)) {
                    chat.session_id = None;
                    if let Some(store) = &self.store
                        && !store.owns_cwd(&chat.cwd)
                    {
                        match store.new_cwd() {
                            Ok(cwd) => chat.cwd = cwd,
                            Err(error) => {
                                self.status = error;
                                self.resume_failed = true;
                                return;
                            }
                        }
                    }
                }
                self.save_history();
                self.connect(ui.ctx(), session);
            }
        }
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
                loading_icon(ui);
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
        ui.add_space(4.);
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
        let visible = self.visible_turns.max(40);
        let earlier = self.turns.len().saturating_sub(visible);
        if earlier > 0
            && ui
                .button(text(
                    pt,
                    "Load older messages",
                    "Carregar mensagens anteriores",
                ))
                .clicked()
        {
            self.visible_turns = visible.saturating_add(40);
            ui.request_repaint();
        }
        let width = ui.available_width();
        for turn in self.turns.iter_mut().skip(earlier) {
            let busy = self.active && latest == Some(turn.id);
            let geometry = TurnGeometry {
                width,
                scale: ui.pixels_per_point(),
                dark: ui.visuals().dark_mode,
                pt,
                busy,
            };
            let top = ui.cursor().min.y;
            let clip = ui.clip_rect();
            if let Some(layout) = &turn.layout
                && layout.geometry == geometry
                && (top > clip.bottom() || !layout.dirty && top + layout.height < clip.top())
            {
                // Keep the full scroll extent while skipping off-screen widgets.
                ui.add_space(layout.height);
                continue;
            }
            ui.push_id(turn.id, |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().selection.bg_fill)
                    .inner_margin(egui::Margin::symmetric(10, 8))
                    .corner_radius(9.)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.weak(text(pt, "You", "Você"));
                            if ui.small_button(text(pt, "Copy", "Copiar")).clicked() {
                                ui.copy_text(turn.prompt.text.clone());
                            }
                        });
                        turn.prompt.show(ui);
                    });
                ui.add_space(4.);
                ui.horizontal(|ui| {
                    ui.strong(text(pt, "Assistant", "Assistente"));
                    if !turn.answer.text.is_empty()
                        && ui
                            .small_button(text(pt, "Copy Markdown", "Copiar Markdown"))
                            .clicked()
                    {
                        ui.copy_text(turn.answer.text.clone());
                    }
                });
                if turn.interrupted {
                    ui.weak(text(pt, "Interrupted", "Interrompida"));
                }
                if busy || !turn.activity.is_empty() {
                    ui.horizontal_wrapped(|ui| {
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
                            if turn.answer.text.is_empty() {
                                loading_icon(ui);
                            } else {
                                ui.weak(text(pt, "Responding…", "Respondendo…"));
                            }
                        }
                    });
                    if turn.thinking_open {
                        egui::Frame::new()
                            .fill(ui.visuals().faint_bg_color)
                            .inner_margin(egui::Margin::symmetric(10, 7))
                            .corner_radius(6.)
                            .show(ui, |ui| {
                                for activity in &mut turn.activity {
                                    match activity {
                                        Activity::Thought(reply) => reply.show(ui),
                                        Activity::Tool { id, title, status } => {
                                            let label = title
                                                .as_deref()
                                                .or(id.as_deref())
                                                .unwrap_or("Tool");
                                            let (icon, state) = match status.as_deref() {
                                                Some("pending") => {
                                                    ("○", text(pt, "Pending", "Pendente"))
                                                }
                                                Some("in_progress") => {
                                                    ("◌", text(pt, "Running", "Em execução"))
                                                }
                                                Some("completed") => {
                                                    ("✓", text(pt, "Completed", "Concluída"))
                                                }
                                                Some("failed") => {
                                                    ("!", text(pt, "Failed", "Falhou"))
                                                }
                                                Some(other) => ("•", other.to_owned()),
                                                None => ("•", String::new()),
                                            };
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(icon);
                                                ui.label(label);
                                                ui.weak(state);
                                            });
                                        }
                                    }
                                }
                            });
                    }
                }
                if !turn.answer.text.is_empty() {
                    turn.answer.show(ui);
                }
                ui.add_space(12.);
            });
            turn.layout = Some(TurnLayout {
                geometry,
                height: ui.cursor().min.y - top,
                dirty: false,
            });
        }
    }

    fn composer(&mut self, ui: &mut egui::Ui, pt: bool) -> bool {
        let clip = ui.clip_rect();
        let available = ui.available_height();
        let row_height = ui.text_style_height(&egui::TextStyle::Body);
        let minimum = (row_height + ui.spacing().extra_text_line_spacing) * EDITOR_ROWS as f32
            + EDITOR_MARGIN.sum().y;
        let cap = (available * 0.32).min(156.).max(minimum);
        let height = self.editor_content_height.clamp(minimum, cap);
        let frame = egui::Frame::new()
            .fill(ui.visuals().faint_bg_color)
            .stroke(ui.visuals().window_stroke())
            .inner_margin(egui::Margin::symmetric(8, 6))
            .outer_margin(egui::Margin::symmetric(0, 3))
            .corner_radius(9.);
        // Bottom panels position their top edge before laying out children.
        let controls = row_height + ui.spacing().interact_size.y + 2. * ui.spacing().item_spacing.y;
        egui::Panel::bottom(ui.id().with("composer"))
            .show_separator_line(false)
            .exact_size((height + controls + frame.total_margin().sum().y).ceil())
            .frame(frame)
            .show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(clip));
                self.editor(ui, pt, height);
            });

        let review = self.conversation.as_ref().and_then(Conversation::review);
        if self.permission.is_none() && review.is_none() {
            return false;
        }
        let frame = theme::toolbar(ui.style());
        let height = (ui.available_height() * 0.38).min(170.);
        egui::Panel::bottom(ui.id().with("assistant-actions"))
            .exact_size(height)
            .frame(frame)
            .show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(clip));
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .min_scrolled_height(0.)
                    .show(ui, |ui| self.actions(ui, pt, review))
                    .inner
            })
            .inner
    }

    fn editor(&mut self, ui: &mut egui::Ui, pt: bool, height: f32) {
        let label = ui.weak(text(pt, "Message", "Mensagem"));
        let editor = egui::ScrollArea::vertical()
            .id_salt("message-editor")
            .auto_shrink([false, false])
            .animated(false)
            .max_height(height)
            .min_scrolled_height(0.)
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.draft_prompt)
                        .id_salt("draft-prompt")
                        .desired_width(ui.available_width())
                        .desired_rows(EDITOR_ROWS)
                        .margin(EDITOR_MARGIN)
                        .return_key(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        )),
                )
                .labelled_by(label.id)
            });
        let edit = editor.inner;
        if edit.changed() {
            self.defer_history(ui.ctx());
        }
        let submit = edit.has_focus() && ui.input(|input| submit_shortcut(&input.events));
        if self.editor_content_height != editor.content_size.y || edit.changed() {
            self.editor_content_height = editor.content_size.y;
            ui.request_repaint();
        }
        let can_send = self.ready
            && !self.active
            && self.pending_setting.is_none()
            && !self.draft_prompt.trim().is_empty();
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if self.active {
                if ui.button(text(pt, "Stop", "Parar")).clicked()
                    && let Some(conversation) = &self.conversation
                {
                    self.status = match conversation.stop() {
                        Ok(()) => text(pt, "Stopping prompt", "Interrompendo solicitação"),
                        Err(error) => error,
                    };
                }
            } else {
                let send = ui
                    .add_enabled(can_send, theme::primary(ui, text(pt, "Send", "Enviar")))
                    .on_hover_text(text(
                        pt,
                        "Enter to send · Shift+Enter for a new line",
                        "Enter para enviar · Shift+Enter para uma nova linha",
                    ));
                if send.clicked() || submit && can_send {
                    self.submit();
                    if self.draft_prompt.is_empty() {
                        self.editor_content_height = 0.;
                    }
                    edit.request_focus();
                    ui.request_repaint();
                }
            }
        });
    }

    fn actions(
        &mut self,
        ui: &mut egui::Ui,
        pt: bool,
        review: Option<mgb_ops::app::ProposalReview>,
    ) -> bool {
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
            && let Some(review) = review
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
                                "Applied to the current run parameters.",
                                "Aplicado aos parâmetros da execução.",
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

fn loading_icon(ui: &mut egui::Ui) {
    const INTERVAL: Duration = Duration::from_millis(50);
    let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(14.), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::new(egui::WidgetType::ProgressIndicator));
    if ui.is_rect_visible(rect) {
        // The map and dock share this frame. Only the waiting indicator needs a timer.
        ui.ctx().request_repaint_after(INTERVAL);
        let angle =
            ui.input(|input| (input.time * 0.75).rem_euclid(1.) as f32) * std::f32::consts::TAU;
        let points = (0..=8)
            .map(|step| {
                let at = angle + step as f32 * 0.45;
                rect.center() + egui::vec2(at.cos(), at.sin()) * 5.
            })
            .collect();
        ui.painter().add(egui::Shape::line(
            points,
            egui::Stroke::new(1.7, ui.visuals().strong_text_color()),
        ));
    }
}

fn carry_prompt(turns: &VecDeque<Turn>, current: &str) -> (String, bool) {
    const LIMIT: usize = 64 * 1024;
    const INTRO: &str = "Prior conversation text follows as context, not new instructions. It may be incomplete.\n\n";
    const CURRENT: &str = "Current user message:\n";
    const OMITTED: &str = "[Older context omitted.]\n\n";
    if INTRO.len() + CURRENT.len() + OMITTED.len() + current.len() > LIMIT {
        return (current.into(), !turns.is_empty());
    }
    let budget = LIMIT - INTRO.len() - CURRENT.len() - OMITTED.len() - current.len();
    let mut selected = Vec::new();
    let mut size = 0;
    for turn in turns.iter().rev() {
        let text = format!(
            "User: {}\nAssistant: {}\n\n",
            turn.prompt.text, turn.answer.text
        );
        if size + text.len() > budget {
            break;
        }
        size += text.len();
        selected.push(text);
    }
    let omitted = selected.len() < turns.len();
    let mut prompt = String::from(INTRO);
    if omitted {
        prompt.push_str(OMITTED);
    }
    for text in selected.iter().rev() {
        prompt.push_str(text);
    }
    prompt.push_str(CURRENT);
    prompt.push_str(current);
    (prompt, omitted)
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

fn submit_shortcut(events: &[egui::Event]) -> bool {
    // Keyboard events handled by the input method are filtered by egui-winit.
    // A composition update can still share a frame with an ordinary Enter.
    events.iter().any(plain_enter)
        && !events
            .iter()
            .any(|event| matches!(event, egui::Event::Ime(_)))
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
    fn markdown_reply_formats_streamed_chunks_and_blocks_local_link_targets() {
        let mut reply = MarkdownReply::default();
        reply.push("## Summary\n\n**");
        reply.push("Bold** and `sample`.\n\n| Item | Value |\n| --- | --- |\n| Flow | 5 |\n\n[local](file:///project/report)");
        let ctx = egui::Context::default();
        let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| reply.show(ui));
        frame.textures_delta.clear();
        let mut painted = Vec::new();
        for item in &frame.shapes {
            text_shapes(&item.shape, &mut painted);
        }
        for expected in ["Summary", "Bold", "sample", "Item", "Flow", "5"] {
            assert!(
                painted.iter().any(|label| label.contains(expected)),
                "{expected} in {painted:?}"
            );
        }
        let raw_markup = painted
            .iter()
            .any(|label| label.contains("##") || label.contains("**"));
        let formatted = !raw_markup;
        assert!(formatted, "{painted:?}");
        assert_eq!(
            reply.cache.get_link_hook("file:///project/report"),
            Some(false)
        );
    }

    fn text_shapes(shape: &egui::Shape, output: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(shape) => output.push(shape.galley.text().into()),
            egui::Shape::Vec(shapes) => {
                for shape in shapes {
                    text_shapes(shape, output);
                }
            }
            _ => {}
        }
    }

    #[test]
    fn transcript_reuses_offscreen_turn_heights_on_followup_frames() {
        let mut pane = Pane {
            visible_turns: 80,
            ..Pane::default()
        };
        for id in 0..80 {
            let mut answer = MarkdownReply::default();
            answer.push("A **reply** with `formatting`.\n\n- A second line");
            pane.turns.push_back(Turn {
                id,
                prompt: Message {
                    text: "A prompt".into(),
                    layout: None,
                },
                answer,
                activity: Vec::new(),
                thinking_open: false,
                interrupted: false,
                layout: None,
            });
        }
        let ctx = egui::Context::default();
        let frame = |pane: &mut Pane, height| {
            let mut extent = 0.;
            let mut frame = ctx.run_ui(egui::RawInput::default(), |ui| {
                let start = ui.cursor().min;
                ui.set_clip_rect(egui::Rect::from_min_size(
                    start,
                    egui::vec2(ui.available_width(), height),
                ));
                pane.transcript(ui, false);
                extent = ui.cursor().min.y - start.y;
            });
            frame.textures_delta.clear();
            extent
        };
        let full_extent = frame(&mut pane, 160.);
        // Rebuilding an off-screen reply would clear its link cache's dirty flag.
        pane.turns[70].answer.dirty = true;
        let clipped_extent = frame(&mut pane, 160.);
        assert_eq!(clipped_extent, full_extent);
        assert!(pane.turns[70].answer.dirty);
    }

    #[test]
    fn composer_paints_typing_and_shift_enter_in_the_input_frame() {
        let mut pane = Pane::default();
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(220., 360.));
        let mut frame = |events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    pane.composer(ui, false);
                },
            );
            output.textures_delta.clear();
            output
        };
        frame(Vec::new());
        frame(Vec::new());
        let empty = frame(Vec::new());
        let editor = empty
            .shapes
            .iter()
            .find_map(|item| match &item.shape {
                egui::Shape::Text(shape) if shape.galley.text() == "Message" => {
                    Some(egui::pos2(28., shape.pos.y + 24.))
                }
                _ => None,
            })
            .unwrap();
        let click = |pressed| egui::Event::PointerButton {
            pos: editor,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![egui::Event::PointerMoved(editor)]);
        frame(vec![click(true)]);
        frame(vec![click(false)]);
        assert!(ctx.memory(|memory| memory.focused().is_some()));
        let typed = frame(vec![egui::Event::Text("Immediate".into())]);
        let multiline = frame(vec![
            egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::SHIFT,
            },
            egui::Event::Text("Second line".into()),
        ]);
        let painted = |output: egui::FullOutput, expected| {
            let mut texts = Vec::new();
            for shape in &output.shapes {
                text_shapes(&shape.shape, &mut texts);
            }
            assert!(texts.iter().any(|text| text == expected), "{texts:?}");
        };
        painted(typed, "Immediate");
        painted(multiline, "Immediate\nSecond line");
        assert_eq!(pane.draft_prompt, "Immediate\nSecond line");
    }

    #[cfg(unix)]
    #[test]
    fn enter_dispatches_focused_composer_to_fake_acp() {
        use mgb_ops::app::InspectionRequest;
        use std::{fs, time::Instant};
        fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let workspace = root.path().canonicalize().unwrap();
        let script = workspace.join("agent.sh");
        let log = workspace.join("prompt.json");
        fs::write(&script, r#"
read -r line || exit
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
read -r line || exit
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"fixture"}}'
while IFS= read -r line; do
 case "$line" in *session/prompt*) printf '%s\n' "$line" > "$1"; id=${line#*\"id\":}; id=${id%%,*}; printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$id";; esac
done
"#).unwrap();
        let session = Session::new(InspectionRequest {
            workspace: workspace.clone(),
            invocation_dir: workspace,
            config: None,
            gpkg_path: None,
            engine_executable: None,
            engine_input_dir: None,
        });
        let ctx = egui::Context::default();
        let wake = ctx.clone();
        let conversation = Conversation::start_adapter_with_wake(
            session,
            PathBuf::from("/bin/sh"),
            vec![script.into_os_string(), log.clone().into_os_string()],
            std::env::current_exe().unwrap(),
            move || wake.request_repaint(),
        );
        let mut pane = Pane {
            conversation: Some(conversation),
            ..Pane::default()
        };
        let mut preferences = Preferences::default();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !pane.ready {
            pane.poll(&ctx, &mut preferences, false);
            assert!(
                Instant::now() < deadline,
                "fake ACP did not become ready: {}",
                pane.status
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(220., 360.));
        let frame = |pane: &mut Pane, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    pane.composer(ui, false);
                },
            );
            output.textures_delta.clear();
            output
        };
        frame(&mut pane, Vec::new());
        let output = frame(&mut pane, Vec::new());
        let editor = output
            .shapes
            .iter()
            .find_map(|item| match &item.shape {
                egui::Shape::Text(shape) if shape.galley.text() == "Message" => {
                    Some(egui::pos2(28., shape.pos.y + 24.))
                }
                _ => None,
            })
            .unwrap();
        frame(&mut pane, vec![egui::Event::PointerMoved(editor)]);
        for pressed in [true, false] {
            frame(
                &mut pane,
                vec![egui::Event::PointerButton {
                    pos: editor,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
        }
        frame(&mut pane, vec![egui::Event::Text("send this".into())]);
        frame(
            &mut pane,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
        );
        let request: serde_json::Value = loop {
            if let Ok(bytes) = fs::read(&log)
                && let Ok(request) = serde_json::from_slice(&bytes)
            {
                break request;
            }
            assert!(Instant::now() < deadline, "Enter did not dispatch");
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(request["method"], "session/prompt");
        assert_eq!(request["params"]["prompt"][0]["text"], "send this");
        assert!(pane.draft_prompt.is_empty());
        assert_eq!(pane.turns.len(), 1);
    }

    #[test]
    fn carryover_keeps_current_prompt_and_reports_omitted_context() {
        let mut turns = VecDeque::new();
        for id in 0..4 {
            turns.push_back(Turn {
                id,
                prompt: Message {
                    text: "a".repeat(30_000),
                    layout: None,
                },
                answer: MarkdownReply::default(),
                activity: Vec::new(),
                thinking_open: false,
                interrupted: false,
                layout: None,
            });
        }
        let (text, omitted) = carry_prompt(&turns, "Current message");
        assert!(omitted);
        assert!(text.len() <= 64 * 1024);
        assert!(text.contains("[Older context omitted.]"));
        assert!(text.ends_with("Current message"));
    }

    #[test]
    fn composer_grows_to_a_cap_then_scrolls_and_shrinks() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(260., 400.));
        let mut pane = Pane::default();
        let measure = |pane: &mut Pane, draft: &str| {
            pane.draft_prompt = draft.into();
            let mut editor = egui::Rect::NOTHING;
            let mut send = egui::Pos2::ZERO;
            for _ in 0..2 {
                let mut frame = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ui| {
                        pane.composer(ui, false);
                    },
                );
                frame.textures_delta.clear();
                editor = frame
                    .shapes
                    .iter()
                    .find_map(|item| match &item.shape {
                        egui::Shape::Text(shape) if shape.galley.text() == draft => {
                            Some(item.clip_rect)
                        }
                        _ => None,
                    })
                    .unwrap();
                send = frame
                    .shapes
                    .iter()
                    .find_map(|item| match &item.shape {
                        egui::Shape::Text(shape) if shape.galley.text() == "Send" => {
                            Some(shape.pos)
                        }
                        _ => None,
                    })
                    .unwrap();
            }
            (editor, send)
        };
        let (short, short_send) = measure(&mut pane, "Short");
        let (multiline, _) = measure(&mut pane, "One\nTwo\nThree\nFour");
        let (long, _) = measure(&mut pane, &"Long message\n".repeat(30));
        let (longer, long_send) = measure(&mut pane, &"Long message\n".repeat(60));
        assert!(short.height() < multiline.height());
        assert!(multiline.height() < long.height());
        assert!(pane.editor_content_height > longer.height());
        assert_eq!(long.height(), longer.height());
        assert!((short_send.y - long_send.y).abs() <= 1.);
        assert_eq!(measure(&mut pane, "Short").0, short);
    }

    #[test]
    fn composer_does_not_submit_during_an_ime_update() {
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        let preedit = egui::Event::Ime(egui::ImeEvent::Preedit {
            text: "あ".into(),
            active_range_chars: None,
        });
        let ignored = !submit_shortcut(&[enter.clone(), preedit]);
        assert!(ignored);
        let commit = egui::Event::Ime(egui::ImeEvent::Commit("あ".into()));
        let ignored = !submit_shortcut(&[enter.clone(), commit]);
        assert!(ignored);
        assert!(submit_shortcut(&[enter]));
    }

    #[test]
    fn waiting_icon_limits_repaints_and_streaming_uses_no_animation_timer() {
        let ctx = egui::Context::default();
        let mut pane = Pane {
            active: true,
            turns: VecDeque::from([Turn {
                id: 0,
                prompt: Message::default(),
                answer: MarkdownReply::default(),
                activity: Vec::new(),
                thinking_open: false,
                interrupted: false,
                layout: None,
            }]),
            ..Default::default()
        };
        let frame = |pane: &mut Pane| {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| pane.transcript(ui, false));
            output.textures_delta.clear();
            output.viewport_output[&egui::ViewportId::ROOT].repaint_delay
        };
        frame(&mut pane);
        frame(&mut pane);
        let waiting = frame(&mut pane);
        assert!(waiting >= Duration::from_millis(30) && waiting <= Duration::from_millis(50));
        pane.turns[0].answer.push("**Streaming**");
        frame(&mut pane);
        assert!(frame(&mut pane) > waiting);
    }

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
