use eframe::egui::{self, Rect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page {
    Map,
    Charts,
    Data,
    #[cfg(test)]
    Fixture,
}
impl Page {
    pub fn label(self, pt: bool) -> &'static str {
        match (self, pt) {
            #[cfg(test)]
            (Self::Fixture, _) => "Fixture",
            (Self::Map, false) => "Map",
            (Self::Map, true) => "Mapa",
            (Self::Charts, false) => "Charts",
            (Self::Charts, true) => "Gráficos",
            (Self::Data, false) => "Data",
            (Self::Data, true) => "Dados",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Dock {
    Tabs {
        pages: Vec<Page>,
        active: usize,
    },
    Split {
        vertical: bool,
        ratio: f32,
        first: Box<Dock>,
        second: Box<Dock>,
    },
}
type Drop = (Page, Vec<bool>, Option<(bool, bool)>);

impl Default for Dock {
    fn default() -> Self {
        Self::Split {
            vertical: true,
            ratio: 0.5,
            first: Box::new(Self::tab(Page::Map)),
            second: Box::new(Self::tab(Page::Charts)),
        }
    }
}
impl Dock {
    fn tab(page: Page) -> Self {
        Self::Tabs {
            pages: vec![page],
            active: 0,
        }
    }
    pub fn focus(&mut self, page: Page) -> bool {
        match self {
            Self::Tabs { pages, active } => {
                if let Some(i) = pages.iter().position(|p| *p == page) {
                    *active = i;
                    true
                } else {
                    false
                }
            }
            Self::Split { first, second, .. } => first.focus(page) || second.focus(page),
        }
    }
    pub fn open(&mut self, page: Page) {
        if !self.focus(page) {
            match self {
                Self::Tabs { pages, active } => {
                    pages.push(page);
                    *active = pages.len() - 1;
                }
                Self::Split { first, .. } => first.open(page),
            }
        }
    }
    pub fn remove(&mut self, page: Page) {
        match self {
            Self::Tabs { pages, active } => {
                pages.retain(|p| *p != page);
                *active = (*active).min(pages.len().saturating_sub(1));
            }
            Self::Split { first, second, .. } => {
                first.remove(page);
                second.remove(page);
                if first.empty() {
                    *self = *second.clone();
                } else if second.empty() {
                    *self = *first.clone();
                }
            }
        }
    }
    fn empty(&self) -> bool {
        matches!(self,Self::Tabs{pages,..} if pages.is_empty())
    }
    fn at_mut(&mut self, path: &[bool]) -> &mut Self {
        if let Some((head, tail)) = path.split_first() {
            match self {
                Self::Split { first, second, .. } => {
                    if *head {
                        second.at_mut(tail)
                    } else {
                        first.at_mut(tail)
                    }
                }
                _ => self,
            }
        } else {
            self
        }
    }
    pub fn move_page(&mut self, page: Page, path: &[bool], side: Option<(bool, bool)>) -> bool {
        let mut axes = [false; 2];
        let mut node = &*self;
        for part in path {
            if let Self::Split {
                vertical,
                first,
                second,
                ..
            } = node
            {
                axes[usize::from(*vertical)] = true;
                node = if *part { second } else { first };
            } else {
                return false;
            }
        }
        if side.is_some_and(|(vertical, _)| axes[usize::from(vertical)]) {
            return false;
        }
        // Remove after inserting: source removal may collapse ancestors of the target.
        if let Some((vertical, before)) = side {
            let shared_ratio = self.axis_ratio(vertical).unwrap_or(0.5);
            let target = self.at_mut(path);
            if matches!(target,Self::Tabs{pages,..} if pages.len()==1 && pages[0]==page) {
                return false;
            }
            let old = target.clone();
            *target = Self::Split {
                vertical,
                ratio: shared_ratio,
                first: Box::new(if before { Self::tab(page) } else { old.clone() }),
                second: Box::new(if before { old } else { Self::tab(page) }),
            };
            let keep: Vec<bool> = path.iter().copied().chain([!before]).collect();
            self.remove_except(page, &keep, &[]);
        } else {
            self.remove_except(page, path, &[]);
            self.at_mut(path).open(page);
        }
        self.prune();
        true
    }
    fn remove_except(&mut self, page: Page, keep: &[bool], here: &[bool]) {
        match self {
            Self::Tabs { pages, active } => {
                if here != keep {
                    pages.retain(|p| *p != page);
                    *active = (*active).min(pages.len().saturating_sub(1));
                }
            }
            Self::Split { first, second, .. } => {
                first.remove_except(page, keep, &[here, &[false]].concat());
                second.remove_except(page, keep, &[here, &[true]].concat());
            }
        }
    }
    fn prune(&mut self) {
        if let Self::Split { first, second, .. } = self {
            first.prune();
            second.prune();
            if first.empty() {
                *self = *second.clone()
            } else if second.empty() {
                *self = *first.clone()
            }
        }
    }
    fn axis_ratio(&self, axis: bool) -> Option<f32> {
        match self {
            Self::Tabs { .. } => None,
            Self::Split {
                vertical,
                ratio,
                first,
                second,
            } => {
                if *vertical == axis {
                    Some(*ratio)
                } else {
                    first.axis_ratio(axis).or_else(|| second.axis_ratio(axis))
                }
            }
        }
    }
    fn set_ratio(&mut self, axis: bool, value: f32) {
        if let Self::Split {
            vertical,
            ratio,
            first,
            second,
        } = self
        {
            if *vertical == axis {
                *ratio = value;
            }
            first.set_ratio(axis, value);
            second.set_ratio(axis, value);
        }
    }
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        pt: bool,
        render: &mut impl FnMut(&mut egui::Ui, Page),
    ) {
        let mut drop = None;
        let mut close = None;
        let mut resized = None;
        self.draw(
            ui,
            ui.available_rect_before_wrap(),
            pt,
            &[],
            [false; 2],
            render,
            &mut drop,
            &mut close,
            &mut resized,
        );
        if let Some((vertical, ratio)) = resized {
            self.set_ratio(vertical, ratio);
        }
        if let Some(page) = close {
            self.remove(page)
        }
        if let Some((page, path, side)) = drop {
            self.move_page(page, &path, side);
        }
    }
    #[allow(clippy::too_many_arguments)] // Recursive drawing carries the bounded split path and one deferred edit.
    fn draw(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        pt: bool,
        path: &[bool],
        axes: [bool; 2],
        render: &mut impl FnMut(&mut egui::Ui, Page),
        drop: &mut Option<Drop>,
        close: &mut Option<Page>,
        resized: &mut Option<(bool, f32)>,
    ) {
        match self {
            Self::Split {
                vertical,
                ratio,
                first,
                second,
            } => {
                let length = if *vertical {
                    rect.width()
                } else {
                    rect.height()
                };
                let start = if *vertical { rect.left() } else { rect.top() };
                let cut = start + length * ratio.clamp(0.15, 0.85);
                let (a, b, divider) = if *vertical {
                    (
                        Rect::from_min_max(rect.min, egui::pos2(cut - 3., rect.bottom())),
                        Rect::from_min_max(egui::pos2(cut + 3., rect.top()), rect.max),
                        Rect::from_min_max(
                            egui::pos2(cut - 3., rect.top()),
                            egui::pos2(cut + 3., rect.bottom()),
                        ),
                    )
                } else {
                    (
                        Rect::from_min_max(rect.min, egui::pos2(rect.right(), cut - 3.)),
                        Rect::from_min_max(egui::pos2(rect.left(), cut + 3.), rect.max),
                        Rect::from_min_max(
                            egui::pos2(rect.left(), cut - 3.),
                            egui::pos2(rect.right(), cut + 3.),
                        ),
                    )
                };
                let response = ui.interact(
                    divider,
                    ui.id().with(("divider", path)),
                    egui::Sense::drag(),
                );
                if response.dragged()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    *ratio = ((if *vertical { pos.x } else { pos.y }) - start) / length;
                    *ratio = ratio.clamp(0.15, 0.85);
                    *resized = Some((*vertical, *ratio));
                }
                ui.painter().rect_filled(
                    divider,
                    0.,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                );
                let mut axes = axes;
                axes[usize::from(*vertical)] = true;
                first.draw(
                    ui,
                    a,
                    pt,
                    &[path, &[false]].concat(),
                    axes,
                    render,
                    drop,
                    close,
                    resized,
                );
                second.draw(
                    ui,
                    b,
                    pt,
                    &[path, &[true]].concat(),
                    axes,
                    render,
                    drop,
                    close,
                    resized,
                );
            }
            Self::Tabs { pages, active } => {
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("dock", path))
                        .max_rect(rect)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_clip_rect(rect);
                child.horizontal(|ui| {
                    for (i, page) in pages.iter().enumerate() {
                        let response = ui.add(
                            egui::Button::new(page.label(pt))
                                .selected(i == *active)
                                .sense(egui::Sense::click_and_drag()),
                        );
                        if response.clicked() {
                            *active = i;
                        }
                        response.dnd_set_drag_payload(*page);
                        if super::icon_button(
                            ui,
                            "×",
                            &format!(
                                "{} {}",
                                super::text(pt, "Close page", "Fechar página"),
                                page.label(pt)
                            ),
                        )
                        .clicked()
                        {
                            *close = Some(*page);
                        }
                    }
                });
                child.separator();
                if let Some(page) = pages.get(*active) {
                    render(&mut child, *page);
                }
                if let Some(page) = egui::DragAndDrop::payload::<Page>(ui.ctx())
                    && let Some(pos) = ui.ctx().pointer_hover_pos()
                    && rect.contains(pos)
                {
                    let x = (pos.x - rect.left()) / rect.width();
                    let y = (pos.y - rect.top()) / rect.height();
                    let side = if x < 0.22 && !axes[1] {
                        Some((true, true))
                    } else if x > 0.78 && !axes[1] {
                        Some((true, false))
                    } else if y < 0.22 && !axes[0] {
                        Some((false, true))
                    } else if y > 0.78 && !axes[0] {
                        Some((false, false))
                    } else {
                        None
                    };
                    let mut preview = rect;
                    if let Some((vertical, before)) = side {
                        if vertical {
                            if before {
                                preview.max.x = rect.center().x
                            } else {
                                preview.min.x = rect.center().x
                            }
                        } else if before {
                            preview.max.y = rect.center().y
                        } else {
                            preview.min.y = rect.center().y
                        }
                    }
                    ui.painter().rect_filled(
                        preview,
                        3.,
                        ui.visuals().selection.bg_fill.linear_multiply(0.45),
                    );
                    if ui.input(|i| i.pointer.any_released()) {
                        *drop = Some((*page, path.to_vec(), side));
                        egui::DragAndDrop::clear_payload(ui.ctx());
                    }
                }
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Form {
    Workspace,
    Prepare,
    Execute,
    RunAll,
    Jobs,
}
impl Form {
    pub fn label(self, pt: bool) -> String {
        match self {
            Self::Workspace => super::text(pt, "Workspace Settings", "Configurações"),
            Self::Prepare => super::text(pt, "Prepare inputs", "Preparar entradas"),
            Self::Execute => super::text(pt, "Execute MGB", "Executar MGB"),
            Self::RunAll => super::text(pt, "Run All", "Executar tudo"),
            Self::Jobs => super::text(pt, "Jobs", "Tarefas"),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Layout {
    pub dock: Dock,
    pub left: bool,
    pub right: bool,
    pub left_tab: usize,
    pub forms: Vec<Form>,
    pub active: Form,
    pub left_width: f32,
    pub right_width: f32,
}
impl Default for Layout {
    fn default() -> Self {
        Self {
            dock: Dock::default(),
            left: true,
            right: false,
            left_tab: 0,
            forms: vec![Form::Jobs],
            active: Form::Jobs,
            left_width: 270.,
            right_width: 390.,
        }
    }
}
impl Layout {
    pub fn form(&mut self, form: Form) {
        if !self.forms.contains(&form) {
            self.forms.push(form)
        }
        self.active = form;
        self.right = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn docking_preserves_single_instances_and_bounded_splits() {
        let mut d = Dock::default();
        d.open(Page::Data);
        assert!(d.move_page(Page::Data, &[true], Some((false, false))));
        {
            let unchanged = !(d.move_page(Page::Map, &[false], Some((true, true))));
            assert!(unchanged);
        };
        d.open(Page::Map);
        let json = serde_json::to_string(&d).unwrap();
        let mut restored: Dock = serde_json::from_str(&json).unwrap();
        restored.remove(Page::Map);
        {
            let unchanged = !(restored.focus(Page::Map));
            assert!(unchanged);
        };
        assert!(restored.focus(Page::Data));
    }
    #[test]
    fn four_area_fixture_has_two_rows_and_columns() {
        let mut grid = Dock::Split {
            vertical: true,
            ratio: 0.6,
            first: Box::new(Dock::Split {
                vertical: false,
                ratio: 0.4,
                first: Box::new(Dock::tab(Page::Map)),
                second: Box::new(Dock::tab(Page::Charts)),
            }),
            second: Box::new(Dock::Split {
                vertical: false,
                ratio: 0.7,
                first: Box::new(Dock::tab(Page::Data)),
                second: Box::new(Dock::tab(Page::Fixture)),
            }),
        };
        for page in [Page::Map, Page::Charts, Page::Data, Page::Fixture] {
            assert!(grid.focus(page));
        }
        {
            let unchanged = !(grid.move_page(Page::Data, &[false, false], Some((true, true))));
            assert!(unchanged);
        };
        {
            let unchanged = !(grid.move_page(Page::Data, &[true, true], Some((false, false))));
            assert!(unchanged);
        };
        grid.set_ratio(false, 0.3);
        if let Dock::Split { first, second, .. } = &grid {
            assert_eq!(first.axis_ratio(false), Some(0.3));
            assert_eq!(second.axis_ratio(false), Some(0.3));
        }
        assert!(serde_json::from_str::<Dock>(&serde_json::to_string(&grid).unwrap()).is_ok());
    }
}
