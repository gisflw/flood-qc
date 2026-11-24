use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    #[serde(skip_serializing)]
    Workspace,
    #[serde(alias = "Execute", alias = "UpdateRainfall")]
    NewRun,
    #[serde(alias = "Prepare")]
    RunAll,
    Jobs,
    Intake,
    Review,
    Forcing,
    RunBundle,
    Instructions,
    Geometry,
    Soil,
    Analysis,
    Bulletin,
    Automation,
    Assistant,
}
impl Form {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Workspace => super::text(
                pt,
                "Workspace Settings",
                "Configurações da área de trabalho",
            ),
            Self::NewRun => super::text(pt, "New run", "Nova execução"),
            Self::RunAll => super::text(pt, "Run experiment", "Executar experimento"),
            Self::Jobs => super::text(pt, "Jobs", "Tarefas"),
            Self::Intake => super::text(pt, "Import data", "Importar dados"),
            Self::Review => super::text(pt, "Observation review", "Revisão de observações"),
            Self::Forcing => super::text(pt, "Forcing and state", "Entradas e estado"),
            Self::RunBundle => super::text(pt, "Share runs", "Compartilhar execuções"),
            Self::Instructions => super::text(pt, "Review instructions", "Instruções de revisão"),
            Self::Geometry => super::text(pt, "Geometry editor", "Editor de geometria"),
            Self::Soil => super::text(pt, "Soil editor", "Editor de solo"),
            Self::Analysis => super::text(pt, "Run comparison", "Comparar execuções"),
            Self::Bulletin => super::text(pt, "Bulletins", "Boletins"),
            Self::Assistant => super::text(pt, "Assistant", "Assistente"),
            Self::Automation => super::text(pt, "Automation", "Automação"),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SideTab {
    Map,
    Charts,
    Data,
    #[serde(skip_serializing)]
    Rainfall,
    Files,
    #[serde(skip_serializing)]
    Interface,
    Form(Form),
}

impl SideTab {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Map => super::text(pt, "Map", "Mapa"),
            Self::Charts => super::text(pt, "Charts", "Gráficos"),
            Self::Data => super::text(pt, "Data", "Dados"),
            Self::Rainfall => super::text(pt, "Rainfall", "Chuva"),
            Self::Files => super::text(pt, "Files", "Arquivos"),
            Self::Interface => super::text(pt, "Interface Settings", "Configurações da interface"),
            Self::Form(form) => form.label(pt),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "SavedLayout")]
pub struct Layout {
    dock: Dock,
    focused: Option<u64>,
    pub width: f32,
    #[serde(skip)]
    next_group: u64,
}

#[derive(Clone, Serialize, Deserialize)]
enum Dock {
    Empty,
    Tabs(Group),
    Split {
        axis: Axis,
        first: Box<Dock>,
        second: Box<Dock>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
struct Group {
    id: u64,
    tabs: Vec<SideTab>,
    active: SideTab,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub(super) enum Destination {
    Insert {
        group: u64,
        index: usize,
    },
    Split {
        group: u64,
        axis: Axis,
        before: bool,
    },
}

#[derive(Clone, Copy)]
pub(super) enum Action {
    Activate(SideTab),
    Close(SideTab),
    Drop(SideTab, Destination),
}

#[derive(Deserialize)]
#[serde(default)]
struct SavedLayout {
    #[serde(deserialize_with = "read_dock")]
    dock: Option<Dock>,
    focused: Option<u64>,
    tabs: Vec<SideTab>,
    active: SideTab,
    width: f32,
}

fn read_dock<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<Dock>, D::Error> {
    Dock::deserialize(deserializer).map(Some)
}

impl Default for SavedLayout {
    fn default() -> Self {
        Self {
            dock: None,
            focused: None,
            tabs: vec![SideTab::Map, SideTab::Charts, SideTab::Data, SideTab::Files],
            active: SideTab::Map,
            width: 390.,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        SavedLayout::default().into()
    }
}

impl From<SavedLayout> for Layout {
    fn from(saved: SavedLayout) -> Self {
        let dock = saved.dock.unwrap_or_else(|| {
            Dock::Tabs(Group {
                id: 0,
                tabs: saved.tabs,
                active: if saved.active == SideTab::Rainfall {
                    SideTab::Map
                } else {
                    saved.active
                },
            })
        });
        let mut layout = Self {
            dock,
            focused: saved.focused,
            width: if saved.width.is_finite() && saved.width > 0. {
                saved.width
            } else {
                390.
            },
            next_group: 0,
        };
        let mut seen = Vec::new();
        layout.dock.visit_mut(&mut |group| {
            group.tabs.retain(|tab| {
                if matches!(
                    tab,
                    SideTab::Rainfall | SideTab::Interface | SideTab::Form(Form::Workspace)
                ) || seen.contains(tab)
                {
                    return false;
                }
                seen.push(*tab);
                true
            });
            if !group.tabs.contains(&group.active)
                && let Some(&first) = group.tabs.first()
            {
                group.active = first;
            }
            layout.next_group = layout.next_group.max(group.id.saturating_add(1));
        });
        layout.dock.prune();
        layout.restore_focus();
        layout
    }
}

impl Layout {
    pub fn form(&mut self, form: Form) {
        self.open(SideTab::Form(form));
    }

    pub fn open(&mut self, tab: SideTab) {
        if let Some(id) = self.dock.with_tab(tab) {
            self.focused = Some(id);
            self.dock.group_mut(id).unwrap().active = tab;
        } else if let Some(group) = self.focused.and_then(|id| self.dock.group_mut(id)) {
            group.tabs.push(tab);
            group.active = tab;
        } else {
            let group = self.new_group(tab);
            self.focused = Some(group.id);
            self.dock = Dock::Tabs(group);
        }
    }

    pub fn apply(&mut self, action: Action) {
        match action {
            Action::Activate(tab) => self.open(tab),
            Action::Close(tab) => self.close(tab),
            Action::Drop(tab, destination) => {
                if let Some((dock, focused)) = self.dropped(tab, destination) {
                    self.dock = dock;
                    self.focused = Some(focused);
                    self.next_group = self.next_group.max(focused.saturating_add(1));
                }
            }
        }
    }

    fn close(&mut self, tab: SideTab) {
        if let Some(id) = self.dock.with_tab(tab) {
            self.dock.group_mut(id).unwrap().remove(tab);
            self.dock.prune();
            self.restore_focus();
        }
    }

    fn restore_focus(&mut self) {
        if !self.focused.is_some_and(|id| self.dock.group(id).is_some()) {
            self.focused = self.dock.first();
        }
    }

    fn new_group(&mut self, tab: SideTab) -> Group {
        let group = Group::new(self.next_group, tab);
        self.next_group += 1;
        group
    }

    fn dropped(&self, tab: SideTab, destination: Destination) -> Option<(Dock, u64)> {
        let source = self.dock.with_tab(tab)?;
        let mut dock = self.dock.clone();
        let group = dock.group_mut(source)?;
        let old_index = group.remove(tab)?;
        match destination {
            Destination::Insert { group, mut index } => {
                let target = dock.group_mut(group)?;
                if source == group && old_index < index {
                    index -= 1;
                }
                target.tabs.insert(index.min(target.tabs.len()), tab);
                target.active = tab;
                dock.prune();
                Some((dock, group))
            }
            Destination::Split {
                group,
                axis,
                before,
            } => {
                dock.prune();
                let id = self.next_group;
                dock.split(group, Group::new(id, tab), axis, before, None)
                    .then_some((dock, id))
            }
        }
    }

    #[cfg(test)]
    pub fn is_open(&self, tab: SideTab) -> bool {
        self.dock.with_tab(tab).is_some()
    }

    #[cfg(test)]
    pub fn focused_tab(&self) -> Option<SideTab> {
        self.focused
            .and_then(|id| self.dock.group(id))
            .map(|group| group.active)
    }

    pub fn has_columns(&self) -> bool {
        self.dock.has_columns()
    }

    pub fn focus_group_at(&mut self, rect: egui::Rect, pointer: egui::Pos2) {
        let mut cells = Vec::with_capacity(4);
        self.dock.cells(rect, &mut cells);
        if let Some((_, group)) = cells.into_iter().find(|(rect, _)| rect.contains(pointer)) {
            self.focused = Some(group.id);
        }
    }

    pub fn show(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        pt: bool,
        mut page: impl FnMut(&mut egui::Ui, SideTab),
    ) -> Option<Action> {
        let mut cells = Vec::with_capacity(4);
        self.dock.cells(rect, &mut cells);
        if cells.is_empty() {
            let mut empty =
                ui.new_child(egui::UiBuilder::new().id_salt("empty-dock").max_rect(rect));
            empty.set_clip_rect(rect.intersect(ui.clip_rect()));
            empty.centered_and_justified(|ui| {
                ui.weak(super::text(
                    pt,
                    "Open a tab from the Pages group on the View ribbon.",
                    "Abra uma aba no grupo Páginas da guia Exibir.",
                ));
            });
            return None;
        }
        let mut action = None;
        let mut strips = Vec::new();
        for (rect, group) in &cells {
            let strip = Self::show_group(ui, *rect, group, pt, &mut page);
            action = strip.action.or(action);
            strips.push(strip);
        }
        if let Some(tab) = egui::DragAndDrop::payload::<SideTab>(ui.ctx())
            && let Some(pointer) = ui.input(|input| input.pointer.interact_pos())
            && let Some(((rect, group), strip)) = cells
                .iter()
                .zip(&strips)
                .find(|((rect, _), _)| rect.contains(pointer))
        {
            let (destination, feedback) = drop_target(*rect, group, strip, pointer);
            if self.dropped(*tab, destination).is_some() {
                let visuals = ui.visuals();
                ui.painter().rect(
                    feedback.shrink(1.),
                    3.,
                    visuals.selection.bg_fill.gamma_multiply(0.55),
                    egui::Stroke::new(2., visuals.hyperlink_color),
                    egui::StrokeKind::Inside,
                );
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary)) {
                    egui::DragAndDrop::clear_payload(ui.ctx());
                    return Some(Action::Drop(*tab, destination));
                }
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::NotAllowed);
            }
        }
        action
    }

    fn show_group(
        parent: &mut egui::Ui,
        rect: egui::Rect,
        group: &Group,
        pt: bool,
        page: &mut impl FnMut(&mut egui::Ui, SideTab),
    ) -> Strip {
        let mut ui = parent.new_child(
            egui::UiBuilder::new()
                .id_salt(("dock-group", group.id))
                .max_rect(rect),
        );
        ui.set_clip_rect(rect.intersect(parent.clip_rect()));
        ui.painter().rect_filled(rect, 3., ui.visuals().panel_fill);
        let mut action = None;
        let mut tabs = Vec::with_capacity(group.tabs.len());
        let id = ui.id().with("active-tab");
        let ensure_visible = ui.data_mut(|data| {
            let old = data.get_temp::<SideTab>(id);
            data.insert_temp(id, group.active);
            old != Some(group.active)
        });
        ui.spacing_mut().scroll = egui::style::ScrollStyle {
            bar_width: 4.,
            bar_inner_margin: 2.,
            ..egui::style::ScrollStyle::solid()
        };
        let scroll = egui::ScrollArea::horizontal()
            .id_salt("tabs")
            .auto_shrink([false, true])
            .show(&mut ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.;
                    for (index, &tab) in group.tabs.iter().enumerate() {
                        let selected = tab == group.active;
                        let item = ui
                            .push_id(tab.label(false), |ui| {
                                let visuals = ui.visuals();
                                egui::Frame::new()
                                    .corner_radius(4.)
                                    .stroke(if selected {
                                        egui::Stroke::new(1., visuals.hyperlink_color)
                                    } else {
                                        visuals.widgets.inactive.bg_stroke
                                    })
                                    .fill(if selected {
                                        visuals.selection.bg_fill
                                    } else {
                                        visuals.widgets.inactive.bg_fill
                                    })
                                    .show(ui, |ui| {
                                        ui.spacing_mut().item_spacing.x = 0.;
                                        ui.horizontal(|ui| {
                                            let label = tab.label(pt);
                                            let title = ui.add(
                                                egui::Button::new(&label)
                                                    .sense(egui::Sense::click_and_drag())
                                                    .selected(selected)
                                                    .frame_when_inactive(false),
                                            );
                                            title.dnd_set_drag_payload(tab);
                                            if title.clicked() {
                                                action = Some(Action::Activate(tab));
                                            }
                                            if title.gained_focus() || selected && ensure_visible {
                                                title.scroll_to_me(None);
                                            }
                                            let close = ui.add(
                                                egui::Button::new("×").frame_when_inactive(false),
                                            );
                                            let name = format!(
                                                "{} {label}",
                                                super::text(pt, "Close", "Fechar")
                                            );
                                            super::button_label(&close, &name);
                                            if close.on_hover_text(name).clicked() {
                                                action = Some(Action::Close(tab));
                                            }
                                        });
                                    })
                                    .response
                            })
                            .inner;
                        let visible = item.rect.intersect(ui.clip_rect());
                        if visible.is_positive() {
                            tabs.push((index, visible));
                        }
                    }
                    if egui::DragAndDrop::has_payload_of_type::<SideTab>(ui.ctx())
                        && let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
                        && ui.clip_rect().contains(pointer)
                    {
                        let bounds = ui.clip_rect();
                        let delta = if pointer.x < bounds.left() + 18. {
                            14.
                        } else if pointer.x > bounds.right() - 18. {
                            -14.
                        } else {
                            0.
                        };
                        if delta != 0. {
                            ui.scroll_with_delta(egui::vec2(delta, 0.));
                            ui.request_repaint();
                        }
                    }
                })
                .response
                .rect
            });
        ui.separator();
        let page_rect = ui.available_rect_before_wrap().intersect(rect);
        let id = parent.id().with(("dock-page", group.active.label(false)));
        let mut page_ui = ui.new_child(egui::UiBuilder::new().id(id).max_rect(page_rect));
        page_ui.set_clip_rect(page_rect.intersect(ui.clip_rect()));
        page(&mut page_ui, group.active);
        Strip {
            rect: egui::Rect::from_min_max(
                egui::pos2(scroll.inner_rect.left(), scroll.inner.top()),
                egui::pos2(scroll.inner_rect.right(), scroll.inner.bottom()),
            )
            .intersect(rect),
            tabs,
            action,
        }
    }
}

struct Strip {
    rect: egui::Rect,
    tabs: Vec<(usize, egui::Rect)>,
    action: Option<Action>,
}

fn drop_target(
    rect: egui::Rect,
    group: &Group,
    strip: &Strip,
    pointer: egui::Pos2,
) -> (Destination, egui::Rect) {
    let id = group.id;
    if strip.rect.contains(pointer) {
        let target = strip
            .tabs
            .iter()
            .find(|(_, tab)| pointer.x <= tab.center().x);
        let (index, x) = target.map_or_else(
            || {
                strip
                    .tabs
                    .last()
                    .map_or((0, strip.rect.left()), |(index, tab)| {
                        (index + 1, tab.right())
                    })
            },
            |&(index, tab)| (index, tab.left()),
        );
        let mark = egui::Rect::from_center_size(
            egui::pos2(
                x.clamp(strip.rect.left(), strip.rect.right()),
                strip.rect.center().y,
            ),
            egui::vec2(5., strip.rect.height()),
        );
        return (Destination::Insert { group: id, index }, mark);
    }
    let body = egui::Rect::from_min_max(
        egui::pos2(rect.left(), strip.rect.bottom()),
        rect.right_bottom(),
    );
    let x = (pointer.x - body.left()) / body.width().max(1.);
    let y = (pointer.y - body.top()) / body.height().max(1.);
    let (distance, axis, before) = [
        (x, Axis::Horizontal, true),
        (1. - x, Axis::Horizontal, false),
        (y, Axis::Vertical, true),
        (1. - y, Axis::Vertical, false),
    ]
    .into_iter()
    .min_by(|a, b| a.0.total_cmp(&b.0))
    .unwrap();
    if distance < 0.23 {
        let (first, second) = split_rect(body, axis, 0.);
        (
            Destination::Split {
                group: id,
                axis,
                before,
            },
            if before { first } else { second },
        )
    } else {
        (
            Destination::Insert {
                group: id,
                index: group.tabs.len(),
            },
            body,
        )
    }
}

fn split_rect(rect: egui::Rect, axis: Axis, gap: f32) -> (egui::Rect, egui::Rect) {
    let mut first = rect;
    let mut second = rect;
    match axis {
        Axis::Horizontal => {
            first.max.x = rect.center().x - gap * 0.5;
            second.min.x = rect.center().x + gap * 0.5;
        }
        Axis::Vertical => {
            first.max.y = rect.center().y - gap * 0.5;
            second.min.y = rect.center().y + gap * 0.5;
        }
    }
    (first, second)
}

impl Group {
    fn new(id: u64, tab: SideTab) -> Self {
        Self {
            id,
            tabs: vec![tab],
            active: tab,
        }
    }

    fn remove(&mut self, tab: SideTab) -> Option<usize> {
        let index = self.tabs.iter().position(|&item| item == tab)?;
        self.tabs.remove(index);
        if self.active == tab
            && let Some(&next) = self.tabs.get(index.min(self.tabs.len().saturating_sub(1)))
        {
            self.active = next;
        }
        Some(index)
    }
}

impl Dock {
    fn group(&self, id: u64) -> Option<&Group> {
        match self {
            Self::Tabs(group) if group.id == id => Some(group),
            Self::Split { first, second, .. } => first.group(id).or_else(|| second.group(id)),
            _ => None,
        }
    }

    fn group_mut(&mut self, id: u64) -> Option<&mut Group> {
        match self {
            Self::Tabs(group) if group.id == id => Some(group),
            Self::Split { first, second, .. } => {
                first.group_mut(id).or_else(|| second.group_mut(id))
            }
            _ => None,
        }
    }

    fn with_tab(&self, tab: SideTab) -> Option<u64> {
        match self {
            Self::Tabs(group) if group.tabs.contains(&tab) => Some(group.id),
            Self::Split { first, second, .. } => {
                first.with_tab(tab).or_else(|| second.with_tab(tab))
            }
            _ => None,
        }
    }

    fn first(&self) -> Option<u64> {
        match self {
            Self::Tabs(group) => Some(group.id),
            Self::Split { first, second, .. } => first.first().or_else(|| second.first()),
            Self::Empty => None,
        }
    }

    fn visit_mut(&mut self, visit: &mut impl FnMut(&mut Group)) {
        match self {
            Self::Tabs(group) => visit(group),
            Self::Split { first, second, .. } => {
                first.visit_mut(visit);
                second.visit_mut(visit);
            }
            Self::Empty => {}
        }
    }

    fn prune(&mut self) {
        match self {
            Self::Tabs(group) if group.tabs.is_empty() => *self = Self::Empty,
            Self::Split { first, second, .. } => {
                first.prune();
                second.prune();
                if matches!(**first, Self::Empty) {
                    *self = std::mem::replace(second.as_mut(), Self::Empty);
                } else if matches!(**second, Self::Empty) {
                    *self = std::mem::replace(first.as_mut(), Self::Empty);
                }
            }
            _ => {}
        }
    }

    fn split(
        &mut self,
        id: u64,
        group: Group,
        axis: Axis,
        before: bool,
        parent: Option<Axis>,
    ) -> bool {
        match self {
            Self::Tabs(current) if current.id == id && parent != Some(axis) => {
                let existing = std::mem::replace(self, Self::Empty);
                let added = Self::Tabs(group);
                let (first, second) = if before {
                    (added, existing)
                } else {
                    (existing, added)
                };
                *self = Self::Split {
                    axis,
                    first: Box::new(first),
                    second: Box::new(second),
                };
                true
            }
            Self::Split {
                axis: current,
                first,
                second,
            } if parent.is_none() => {
                let branch = if first.group(id).is_some() {
                    first
                } else {
                    second
                };
                branch.split(id, group, axis, before, Some(*current))
            }
            _ => false,
        }
    }

    fn has_columns(&self) -> bool {
        match self {
            Self::Split {
                axis,
                first,
                second,
            } => *axis == Axis::Horizontal || first.has_columns() || second.has_columns(),
            _ => false,
        }
    }

    fn cells<'a>(&'a self, rect: egui::Rect, cells: &mut Vec<(egui::Rect, &'a Group)>) {
        match self {
            Self::Tabs(group) => cells.push((rect, group)),
            Self::Split {
                axis,
                first,
                second,
            } => {
                let (a, b) = split_rect(rect, *axis, 6.);
                first.cells(a, cells);
                second.cells(b, cells);
            }
            Self::Empty => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct TestStorage(Option<String>);
    impl eframe::Storage for TestStorage {
        fn get_string(&self, _: &str) -> Option<String> {
            self.0.clone()
        }
        fn set_string(&mut self, _: &str, value: String) {
            self.0 = Some(value);
        }
        fn remove_string(&mut self, _: &str) {
            self.0 = None;
        }
        fn flush(&mut self) {}
    }

    #[test]
    fn repair_flat_layout_and_round_trip_an_empty_dock() {
        let mut layout: Layout = serde_json::from_str(
            r#"{"tabs":["Data","Rainfall","Interface",{"Form":"Workspace"},{"Form":"Execute"},"Files","Data"],"active":{"Form":"Workspace"},"width":428}"#,
        ).unwrap();
        let group = layout.dock.group(layout.focused.unwrap()).unwrap();
        assert_eq!(
            group.tabs,
            [SideTab::Data, SideTab::Form(Form::NewRun), SideTab::Files]
        );
        assert_eq!(group.active, SideTab::Data);
        assert_eq!(layout.width, 428.);
        for tab in group.tabs.clone() {
            layout.close(tab);
        }
        let saved = serde_json::to_string(&layout).unwrap();
        let mut restored: Layout = serde_json::from_str(&saved).unwrap();
        let empty = matches!(restored.dock, Dock::Empty);
        assert!(empty);
        restored.open(SideTab::Map);
        let group = restored.dock.group(restored.focused.unwrap()).unwrap();
        assert_eq!(group.tabs, [SideTab::Map]);
    }

    #[test]
    fn tabs_move_through_columns_three_groups_and_grid() {
        let mut layout = Layout::default();
        let split = |group, axis| Destination::Split {
            group,
            axis,
            before: false,
        };
        layout.apply(Action::Drop(SideTab::Files, split(0, Axis::Horizontal)));
        let right = layout.focused.unwrap();
        assert!(layout.has_columns());
        layout.apply(Action::Drop(SideTab::Charts, split(0, Axis::Vertical)));
        let bottom_left = layout.focused.unwrap();
        layout.apply(Action::Drop(SideTab::Data, split(right, Axis::Vertical)));
        let bottom_right = layout.focused.unwrap();
        let mut cells = Vec::new();
        layout.dock.cells(
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600., 800.)),
            &mut cells,
        );
        assert_eq!(
            cells.iter().map(|(_, group)| group.id).collect::<Vec<_>>(),
            [0, bottom_left, right, bottom_right]
        );
        assert_eq!(cells[0].0.center().y, cells[2].0.center().y);
        assert_eq!(cells[1].0.center().y, cells[3].0.center().y);
        assert!(
            layout
                .dropped(SideTab::Charts, split(bottom_left, Axis::Vertical))
                .is_none()
        );
        let mut storage = TestStorage::default();
        eframe::set_value(&mut storage, "dock", &layout);
        let mut restored: Layout = eframe::get_value(&storage, "dock").unwrap();
        restored.open(SideTab::Charts);
        assert_eq!(restored.focused, Some(bottom_left));
        restored.apply(Action::Drop(
            SideTab::Data,
            Destination::Insert { group: 0, index: 0 },
        ));
        assert_eq!(
            restored.dock.group(0).unwrap().tabs,
            [SideTab::Data, SideTab::Map]
        );
        restored.apply(Action::Drop(
            SideTab::Map,
            Destination::Insert { group: 0, index: 0 },
        ));
        assert_eq!(
            restored.dock.group(0).unwrap().tabs,
            [SideTab::Map, SideTab::Data]
        );
        assert!(restored.dock.group(bottom_right).is_none());
        assert_eq!(restored.dock.with_tab(SideTab::Data), Some(0));
    }

    #[test]
    fn inactive_pages_reopen_and_empty_splits_collapse() {
        let mut layout = Layout::default();
        layout.apply(Action::Drop(
            SideTab::Files,
            Destination::Split {
                group: 0,
                axis: Axis::Vertical,
                before: true,
            },
        ));
        let stacked = !layout.has_columns();
        assert!(stacked);
        layout.close(SideTab::Map);
        layout.close(SideTab::Files);
        let single_group = matches!(layout.dock, Dock::Tabs(_));
        assert!(single_group);
        layout.open(SideTab::Files);
        layout.open(SideTab::Files);
        assert_eq!(
            layout.dock.group(0).unwrap().tabs,
            [SideTab::Charts, SideTab::Data, SideTab::Files]
        );
    }

    #[test]
    fn tab_strip_drag_splits_and_outside_release_keeps_the_tab() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900., 550.));
        let dock = egui::Rect::from_min_size(egui::pos2(10., 10.), egui::vec2(720., 510.));
        let mut layout = Layout::default();
        let frame = |layout: &mut Layout, events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events,
                    ..Default::default()
                },
                |ui| {
                    if let Some(action) = layout.show(ui, dock, false, |ui, _| {
                        ui.label("Page contents");
                    }) {
                        layout.apply(action);
                    }
                },
            );
            output.textures_delta.clear();
            output
        };
        let charts_at = |output: egui::FullOutput| {
            output
                .shapes
                .iter()
                .find_map(|item| {
                    if let egui::Shape::Text(shape) = &item.shape
                        && shape.galley.text() == "Charts"
                    {
                        return Some(shape.pos + shape.galley.size() * 0.5);
                    }
                    None
                })
                .unwrap()
        };
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        };
        let drag = |layout: &mut Layout, start, end| {
            let _ = frame(layout, vec![egui::Event::PointerMoved(start)]);
            let _ = frame(layout, vec![button(start, true)]);
            let _ = frame(layout, vec![egui::Event::PointerMoved(end)]);
            let _ = frame(layout, vec![button(end, false)]);
        };
        let source = charts_at(frame(&mut layout, Vec::new()));
        drag(
            &mut layout,
            source,
            egui::pos2(dock.right() - 20., dock.center().y),
        );
        assert!(layout.has_columns());
        let group = layout.dock.with_tab(SideTab::Charts);
        assert_ne!(group, Some(0));
        let moved = charts_at(frame(&mut layout, Vec::new()));
        drag(
            &mut layout,
            moved,
            egui::pos2(dock.right() + 70., dock.center().y),
        );
        assert_eq!(layout.dock.with_tab(SideTab::Charts), group);
    }
}
