mod assistant;
mod charts;
mod data;
mod export;
mod inundation;
mod layout;
mod map;
mod runners;
mod theme;
mod tools;
mod workflows;
use self::layout::{Form, SideTab};

use self::data::{
    CHART_SLOT, ChartRequest, Job, Loaded, Project, RUN_SLOT, RainSide, SLOTS, SelectedRun,
    UPSTREAM_SLOT, View, ViewRequest, Worker,
};
use eframe::egui::{self, Color32};
use mgb_ops::analysis::HourlyWindow;
use mgb_ops::app::{
    Location, RainfallPeriod, RunSelection, RunSummary, SeriesQuery, SeriesSelection,
};
use mgb_ops::store::RainfallCacheInfo;
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use walkers::{HttpTiles, MapMemory, lon_lat};

const APP_ID: &str = "MGB Ops · Monitoring";
const STORAGE: &str = "mgb-ops.monitoring.v1";
fn text(pt: bool, en: &str, br: &str) -> String {
    if pt { br } else { en }.into()
}

fn icon_button(ui: &mut egui::Ui, glyph: &str, name: &str) -> egui::Response {
    let response = ui.small_button(glyph);
    response
        .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), name));
    response.on_hover_text(name)
}

fn series_table(ui: &mut egui::Ui, report: &mgb_ops::app::SeriesReport, pt: bool) {
    use egui_extras::{Column, TableBuilder};

    let headers = if pt {
        ["Hora", "Série", "Variável", "Valor", "Unidade"]
    } else {
        ["Time", "Series", "Variable", "Value", "Unit"]
    };
    let mut rows = 0;
    let ends: Vec<_> = report
        .series
        .iter()
        .map(|series| {
            rows += series.points.len();
            rows
        })
        .collect();
    let width = ui.available_width() - ui.spacing().scroll.allocated_width();
    let compact = width < 460.;
    let line_height = ui.text_style_height(&egui::TextStyle::Body);
    let spacing = ui.spacing().item_spacing;
    let mut table = TableBuilder::new(ui)
        .id_salt("series-table")
        .striped(true)
        .cell_layout(egui::Layout::top_down(egui::Align::Min));
    if compact {
        table = table
            .column(Column::exact((width * 0.62).max(0.)).clip(true))
            .column(Column::remainder().clip(true));
    } else {
        let available = width - 4. * spacing.x;
        for fraction in [0.30, 0.2, 0.16, 0.21] {
            table = table.column(Column::exact(available * fraction).clip(true));
        }
        table = table.column(Column::remainder().clip(true));
    }
    let headings = if compact {
        vec![headers[0], headers[3]]
    } else {
        headers.to_vec()
    };
    let row_height = if compact {
        2. * line_height + spacing.y + 4.
    } else {
        line_height + 3.
    };
    table
        .header(line_height + 4., |mut header| {
            for label in headings {
                header.col(|ui| theme::table_text(ui, egui::RichText::new(label).strong()));
            }
        })
        .body(|body| {
            body.rows(row_height, rows, |mut row| {
                let index = row.index();
                let series_index = ends.partition_point(|end| *end <= index);
                let start = series_index.checked_sub(1).map_or(0, |index| ends[index]);
                let series = &report.series[series_index];
                let point = &series.points[index - start];
                let time = point.timestamp.to_string();
                let scenario = series.scenario_id.as_deref().unwrap_or("OBS");
                let value = point
                    .value
                    .map_or_else(|| "NA".into(), |value| value.to_string());
                if compact {
                    row.col(|ui| {
                        theme::table_text(ui, egui::RichText::new(time));
                        theme::table_text(ui, egui::RichText::new(scenario).weak());
                    });
                    row.col(|ui| {
                        theme::table_text(
                            ui,
                            egui::RichText::new(format!("{value} {}", series.unit)),
                        );
                        theme::table_text(ui, egui::RichText::new(&series.variable).weak());
                    });
                } else {
                    for value in [&time, scenario, &series.variable, &value, &series.unit] {
                        row.col(|ui| theme::table_text(ui, egui::RichText::new(value)));
                    }
                }
            });
        });
}

fn section(ui: &mut egui::Ui, title: String, content: impl FnOnce(&mut egui::Ui)) {
    ui.separator();
    ui.strong(title);
    content(ui);
}

fn rainfall_maximum(input: &str) -> Option<f64> {
    input
        .trim()
        .replace(',', ".")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 1.)
}

fn footer_box(ui: &mut egui::Ui, contents: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(4, 2))
        .show(ui, contents);
}

fn ribbon_group(ui: &mut egui::Ui, title: String, actions: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.horizontal_top(actions);
        ui.small(title);
    });
    ui.separator();
}

fn ribbon_action(ui: &egui::Ui, emoji: &str, label: &str) -> egui::Button<'static> {
    let emoji_font = egui::FontId::proportional(20.);
    let label_font = egui::TextStyle::Button.resolve(ui.style());
    let [emoji_width, label_width] = ui.fonts_mut(|fonts| {
        [(emoji, &emoji_font), (label, &label_font)].map(|(text, font)| {
            fonts
                .layout_no_wrap(text.to_owned(), font.clone(), Color32::PLACEHOLDER)
                .size()
                .x
        })
    });
    let width = emoji_width.max(label_width);
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &format!("{emoji}\n"),
        0.5 * (width - emoji_width),
        egui::TextFormat {
            font_id: emoji_font,
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    job.append(
        label,
        0.5 * (width - label_width),
        egui::TextFormat {
            font_id: label_font,
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    egui::Button::new(job).wrap_mode(egui::TextWrapMode::Extend)
}

fn button_label(response: &egui::Response, label: &str) {
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, response.enabled(), label)
    });
    if response.gained_focus() {
        response.scroll_to_me(None);
    }
}

fn ribbon_button(ui: &mut egui::Ui, emoji: &str, label: String) -> egui::Response {
    let response = ui.add(ribbon_action(ui, emoji, &label));
    button_label(&response, &label);
    response
}

fn layer_row(
    ui: &mut egui::Ui,
    pt: bool,
    label: &str,
    color: &mut Color32,
    visible: &mut bool,
) -> bool {
    let id = ui.id().with("layer-settings-open");
    let mut open = ui.data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    ui.horizontal_wrapped(|ui| {
        let action = if open {
            text(pt, "Hide settings", "Ocultar configurações")
        } else {
            text(pt, "Show settings", "Mostrar configurações")
        };
        if icon_button(
            ui,
            if open { "-" } else { "+" },
            &format!("{action} · {label}"),
        )
        .clicked()
        {
            open = !open;
            ui.data_mut(|data| data.insert_temp(id, open));
        }
        theme::color_palette(ui, color, label, pt);
        ui.checkbox(visible, label);
    });
    open
}

fn layer_opacity(ui: &mut egui::Ui, pt: bool, opacity: &mut f32) {
    let label = ui.label(text(pt, "Opacity", "Opacidade"));
    ui.add(egui::Slider::new(opacity, 0.0..=1.0))
        .labelled_by(label.id);
}

fn layer_width(ui: &mut egui::Ui, pt: bool, width: &mut Option<f32>, default: f32) {
    let mut value = width.unwrap_or(default);
    if ui
        .add(egui::Slider::new(&mut value, 0.0..=8.0).text(text(
            pt,
            "Thickness (pt)",
            "Espessura (pt)",
        )))
        .changed()
    {
        *width = Some(value);
    }
}

fn status_label(state: &str, portuguese: bool) -> &str {
    if !portuguese {
        return state;
    }
    match state {
        "completed" => "Concluído",
        "failed" => "Falhou",
        "pending" => "Pendente",
        "running" => "Em execução",
        "cancelled" => "Cancelado",
        "prepared" => "Preparado",
        other => other,
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Display {
    config: PathBuf,
    workspace: PathBuf,
    camera: MapMemory,
    layers: Vec<map::LayerControl>,
    basemap: usize,
    rain_maximum: f64,
    labels: bool,
    local: Vec<map::LocalDisplay>,
    selection: Option<Location>,
    comparison: Option<ComparisonFields>,
    rainfall_scenario: String,
    inundation_scenario: String,
    overlays: Vec<String>,
    observations: bool,
    hours: i32,
    explicit: bool,
    start: String,
    end: String,
    map_mode: map::Mode,
    swipe: f32,
    right_rainfall: Option<RainfallSettings>,
    chart_start: String,
    chart_end: String,
    #[serde(rename = "right_pane")]
    layout: layout::Layout,
}
impl Default for Display {
    fn default() -> Self {
        let mut camera = MapMemory::default();
        let _ = camera.set_zoom(7.);
        camera.center_at(lon_lat(-52., -30.));
        Self {
            config: PathBuf::new(),
            workspace: PathBuf::new(),
            camera,
            layers: map::default_layers(),
            basemap: 1,
            rain_maximum: map::RAIN_MAX_MM,
            labels: true,
            local: vec![],
            selection: None,
            comparison: None,
            rainfall_scenario: String::new(),
            inundation_scenario: String::new(),
            overlays: vec![],
            observations: true,
            hours: -24,
            explicit: false,
            start: String::new(),
            end: String::new(),
            map_mode: map::Mode::Single,
            swipe: 0.5,
            right_rainfall: None,
            chart_start: String::new(),
            chart_end: String::new(),
            layout: layout::Layout::default(),
        }
    }
}
impl Display {
    fn restore_layers(&mut self) {
        if !self
            .layers
            .iter()
            .any(|layer| layer.kind == inundation::KIND)
            && let Some((position, layer)) = map::default_layers()
                .into_iter()
                .enumerate()
                .find(|(_, layer)| layer.kind == inundation::KIND)
        {
            self.layers.insert(position.min(self.layers.len()), layer);
        }
    }

    fn rainfall_settings(&self, side: RainSide) -> RainfallSettings {
        if side == RainSide::Right
            && let Some(settings) = &self.right_rainfall
        {
            return settings.clone();
        }
        RainfallSettings {
            scenario: self.rainfall_scenario.clone(),
            window: RainfallWindow {
                hours: self.hours,
                explicit: self.explicit,
                start: self.start.clone(),
                end: self.end.clone(),
            },
        }
    }

    fn set_rainfall_settings(&mut self, side: RainSide, settings: RainfallSettings) {
        match side {
            RainSide::Left => {
                self.rainfall_scenario = settings.scenario;
                self.hours = settings.window.hours;
                self.explicit = settings.window.explicit;
                self.start = settings.window.start;
                self.end = settings.window.end;
            }
            RainSide::Right => self.right_rainfall = Some(settings),
        }
    }

    fn set_map_mode(&mut self, mode: map::Mode) {
        self.map_mode = mode;
        if mode.comparing() && self.right_rainfall.is_none() {
            self.right_rainfall = Some(self.rainfall_settings(RainSide::Left));
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct RainfallSettings {
    scenario: String,
    window: RainfallWindow,
}

#[derive(Clone, Serialize, Deserialize)]
struct RainfallWindow {
    hours: i32,
    explicit: bool,
    start: String,
    end: String,
}

impl RainfallWindow {
    fn period(&self) -> Result<RainfallPeriod, String> {
        if self.explicit {
            Ok(RainfallPeriod::Explicit {
                start: data::timestamp(&self.start)?,
                end: data::timestamp(&self.end)?,
            })
        } else {
            Ok(RainfallPeriod::SignedHours(self.hours))
        }
    }
}

#[derive(Default)]
struct RainfallView {
    image: Option<map::RainImage>,
    view: Option<View>,
    finding: Option<String>,
    draft: Option<RainfallWindow>,
}

impl RainfallView {
    fn replace(&mut self, context: &egui::Context, mut view: View, maximum: f64, hue: Color32) {
        if let Some(rain) = view.rainfall.take() {
            if self.image.as_ref().is_none_or(|image| {
                image.rain.generation != rain.generation
                    || image.rain.scenario != rain.scenario
                    || image.rain.window != rain.window
            }) {
                self.image = Some(map::RainImage::new_with_color(context, rain, maximum, hue));
            }
            self.finding = None;
            self.view = Some(view);
        } else {
            self.finding = view
                .rainfall_finding
                .clone()
                .or_else(|| self.finding.take())
                .or_else(|| Some("Rainfall is unavailable; previous map retained.".into()));
            if self.image.is_none()
                || self
                    .view
                    .as_ref()
                    .is_none_or(|old| old.location != view.location)
            {
                self.view = Some(view);
            }
        }
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    portuguese: bool,
    appearance: usize,
    recent: Vec<Display>,
    assistant: assistant::Preferences,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct ComparisonFields {
    mini: String,
    station: String,
}

impl ComparisonFields {
    fn select(&mut self, location: &Location, stations: &[mgb_ops::store::StationLocation]) {
        *self = Self::default();
        match location {
            Location::Mini(id) => {
                self.mini = id.to_string();
                let mut linked = stations.iter().filter(|station| {
                    station
                        .offset
                        .as_ref()
                        .is_some_and(|offset| offset.mini_id == *id)
                });
                if let Some(station) = linked.next()
                    && linked.next().is_none()
                {
                    self.station.clone_from(&station.station.station_id);
                }
            }
            Location::Station(id) => {
                self.station.clone_from(id);
                if let Some(mini) = data::location_mini(location, stations) {
                    self.mini = mini.to_string();
                }
            }
        }
    }
}

enum Departure {
    Project(PathBuf, PathBuf),
    Close,
    Quit,
}

struct Monitoring {
    preferences: Preferences,
    display: Display,
    project: Option<Arc<Project>>,
    run: Option<Arc<RunSummary>>,
    run_selection_ready: bool,
    active_run: Result<Option<PathBuf>, String>,
    comparison_runs: Vec<PathBuf>,
    run_check: Instant,
    rainfall: [RainfallView; 2],
    rainfall_info: Result<Option<RainfallCacheInfo>, String>,
    inundation: inundation::Inundation,
    worker: Worker,
    chart_worker: Worker,
    loading: [bool; SLOTS],
    chart_fields: ComparisonFields,
    chart_message: String,
    chart_retained: bool,
    status: String,
    search: String,
    charts: charts::Charts,
    tiles: Option<HttpTiles>,
    tile_source: usize,
    cache: [map::StaticCache; 2],
    local: Vec<map::LocalLayer>,
    runners: runners::Runners,
    assistant: assistant::Pane,
    workflows: workflows::Forms,
    calibration: tools::CalibrationTools,
    pending: Option<Departure>,
    saving_departure: bool,
    leaving_job: bool,
    allow_quit: bool,
    switching_project: bool,
    map_menu: Option<Location>,
    upstream_basin: Option<(i64, map::BasinPaths)>,
    export_scale: u32,
}
impl Monitoring {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let mut preferences: Preferences = cc
            .storage
            .and_then(|s| eframe::get_value(s, STORAGE))
            .unwrap_or_default();
        preferences.appearance = if preferences.appearance == 2 { 2 } else { 1 };
        for display in &mut preferences.recent {
            if !display.rain_maximum.is_finite() || display.rain_maximum <= 1. {
                display.rain_maximum = map::RAIN_MAX_MM;
            }
            if display.basemap > 3 {
                display.basemap = 1;
            }
            display.restore_layers();
            display.set_map_mode(display.map_mode);
            display.swipe = if display.swipe.is_finite() {
                display.swipe.clamp(0., 1.)
            } else {
                0.5
            };
        }
        let display = preferences.recent.first().cloned().unwrap_or_default();
        let mut app = Self {
            preferences,
            display,
            project: None,
            run: None,
            run_selection_ready: false,
            active_run: Ok(None),
            comparison_runs: Vec::new(),
            run_check: Instant::now(),
            rainfall: Default::default(),
            rainfall_info: Ok(None),
            inundation: inundation::Inundation::new(cc.egui_ctx.clone()),
            worker: Worker::new(cc.egui_ctx.clone()),
            chart_worker: Worker::new(cc.egui_ctx.clone()),
            loading: [false; SLOTS],
            chart_fields: ComparisonFields::default(),
            chart_message: String::new(),
            chart_retained: false,
            status: String::new(),
            search: String::new(),
            charts: charts::Charts::new(
                cc.wgpu_render_state
                    .as_ref()
                    .expect("desktop requires wgpu"),
                0,
            ),
            tiles: None,
            tile_source: 0,
            cache: Default::default(),
            local: vec![],
            runners: runners::Runners::default(),
            assistant: assistant::Pane::default(),
            workflows: workflows::Forms::default(),
            calibration: tools::CalibrationTools::default(),
            pending: None,
            saving_departure: false,
            leaving_job: false,
            allow_quit: false,
            switching_project: false,
            map_menu: None,
            upstream_basin: None,
            export_scale: 1,
        };
        app.assistant.set_data_dir(eframe::storage_dir(APP_ID));
        let arguments: Vec<_> = std::env::args_os().skip(1).collect();
        if arguments.len() == 1 {
            let config = PathBuf::from(&arguments[0]);
            let config = std::path::absolute(&config).unwrap_or(config);
            app.open_path(config);
        } else if !app.display.config.as_os_str().is_empty() {
            app.submit(Job::Open(
                app.display.config.clone(),
                app.display.workspace.clone(),
            ));
        }
        app
    }
    fn submit(&mut self, job: Job) {
        let slot = job.slot();
        if slot == CHART_SLOT {
            self.loading[slot] = true;
            self.chart_worker.submit(job);
            return;
        }
        if slot == 4 && self.loading[4] {
            self.status = text(
                self.preferences.portuguese,
                "An export is already running; wait for completion",
                "Uma exportação está em andamento; aguarde a conclusão",
            );
            return;
        }
        if slot == 0 {
            self.invalidate_chart();
            for dependent in 1..4 {
                self.worker.invalidate(dependent);
                self.loading[dependent] = false;
            }
            self.worker.invalidate(RainSide::Right.slot());
            self.loading[RainSide::Right.slot()] = false;
        }
        self.loading[slot] = true;
        self.worker.submit(job);
        let pt = self.preferences.portuguese;
        match slot {
            UPSTREAM_SLOT => {}
            RUN_SLOT => self.status = text(pt, "Loading run…", "Carregando execução…"),
            _ => {
                self.status = text(
                    pt,
                    "Loading; previous view retained",
                    "Carregando; visualização anterior mantida",
                );
            }
        }
    }
    fn remember(&mut self) {
        if self.display.config.as_os_str().is_empty() {
            return;
        }
        self.preferences
            .recent
            .retain(|p| p.config != self.display.config);
        self.preferences.recent.insert(0, self.display.clone());
        self.preferences.recent.truncate(10);
    }
    fn choose_project(&mut self) {
        if let Some(workspace) = rfd::FileDialog::new().pick_folder() {
            self.depart(Departure::Project(
                mgb_ops::app::workspace_config_path(&workspace),
                workspace,
            ));
        }
    }

    fn open_path(&mut self, config: PathBuf) {
        if config.is_dir() {
            let path = mgb_ops::app::workspace_config_path(&config);
            self.depart(Departure::Project(path, config));
            return;
        }
        let workspace = self
            .preferences
            .recent
            .iter()
            .find(|p| p.config == config)
            .map(|p| p.workspace.clone())
            .or_else(|| data::infer_workspace(&config))
            .or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Workspace / Área de trabalho")
                    .pick_folder()
            });
        if let Some(workspace) = workspace {
            self.depart(Departure::Project(config, workspace));
        }
    }
    fn check_active_run(&mut self, refresh: bool) {
        let Some(project) = &self.project else {
            return;
        };
        self.run_check = Instant::now() + Duration::from_secs(2);
        let active = mgb_ops::app::discover_trials(&project.workspace)
            .map(|roots| roots.into_iter().next())
            .map_err(|error| error.to_string());
        let changed = self.active_run != active;
        let following_active = project.session.run_selection() == RunSelection::Active;
        self.active_run = active;
        if changed && following_active {
            let target = self.active_run.as_ref().ok().cloned().flatten();
            if target.is_none() {
                self.status = self.active_run.as_ref().err().cloned().unwrap_or_else(|| {
                    text(
                        self.preferences.portuguese,
                        "No active run.",
                        "Nenhuma execução ativa.",
                    )
                });
            }
            self.load_run();
        } else if refresh {
            self.load_run();
        }
        if changed && self.check_rainfall_cache() {
            self.request_view();
        }
    }

    fn load_run(&mut self) {
        self.run_selection_ready = false;
        self.invalidate_chart();
        self.worker.invalidate(RUN_SLOT);
        self.loading[RUN_SLOT] = false;
        self.charts.clear();
        self.chart_message.clear();
        self.chart_retained = false;
        self.display.overlays.clear();
        if let Some(project) = &self.project {
            let expected = match project.session.run_selection() {
                RunSelection::Active => self.active_run.as_ref().ok().cloned().flatten(),
                RunSelection::Saved(root) => Some(root),
            };
            if expected.is_some() {
                self.submit(Job::Run(project.session.run_snapshot(), expected));
                return;
            }
        }
        if self.run.is_none() {
            self.request_chart();
        }
    }

    fn selected_run(&self, root: &std::path::Path) -> Option<SelectedRun> {
        let project = self.project.as_ref()?;
        Some(SelectedRun {
            snapshot: project.session.run_snapshot(),
            root: root.into(),
        })
    }

    fn check_rainfall_cache(&mut self) -> bool {
        let info = self.project.as_ref().map_or(Ok(None), |project| {
            mgb_ops::app::rainfall_cache_info(&project.workspace)
        });
        if self.rainfall_info == info {
            return false;
        }
        self.rainfall_info = info;
        if let Ok(Some(info)) = &self.rainfall_info
            && let Some(first) = info.scenarios.first()
        {
            for side in RainSide::BOTH {
                if side == RainSide::Right && self.display.right_rainfall.is_none() {
                    continue;
                }
                let mut settings = self.display.rainfall_settings(side);
                if !info.scenarios.contains(&settings.scenario) {
                    settings.scenario.clone_from(first);
                    self.display.set_rainfall_settings(side, settings);
                }
            }
        }
        for side in RainSide::BOTH {
            self.worker.invalidate(side.slot());
            self.loading[side.slot()] = false;
            let state = &mut self.rainfall[side.index()];
            state.finding = None;
        }
        true
    }

    fn request_view(&mut self) {
        self.request_rainfall(RainSide::Left);
        if self.display.map_mode.comparing() {
            self.request_rainfall(RainSide::Right);
        }
    }

    fn request_rainfall(&mut self, side: RainSide) {
        let Some(project) = self.project.clone() else {
            return;
        };
        let settings = self.display.rainfall_settings(side);
        let state = &mut self.rainfall[side.index()];
        state.finding = None;
        let period =
            if let Ok(Some(info)) = &self.rainfall_info {
                match settings
                    .window
                    .period()
                    .and_then(|period| Ok((period.resolve(info)?, period)))
                {
                    Ok((_, period)) => Some(period),
                    Err(error) => {
                        state.finding = Some(error);
                        None
                    }
                }
            } else {
                state.finding = Some(self.rainfall_info.as_ref().err().cloned().unwrap_or_else(
                    || {
                        text(
                            self.preferences.portuguese,
                            "Rainfall is unavailable; previous map retained.",
                            "Chuva indisponível; mapa anterior mantido.",
                        )
                    },
                ));
                None
            };
        self.loading[side.slot()] = true;
        self.worker.submit(Job::View(
            side,
            ViewRequest {
                project,
                location: self.display.selection.clone(),
                scenario: settings.scenario,
                period,
            },
        ));
    }

    fn set_map_mode(&mut self, mode: map::Mode) {
        let comparing = self.display.map_mode.comparing();
        self.display.set_map_mode(mode);
        if mode.comparing() && !comparing {
            self.request_rainfall(RainSide::Right);
        } else if !mode.comparing() && comparing {
            self.worker.invalidate(RainSide::Right.slot());
            self.loading[RainSide::Right.slot()] = false;
            let state = &mut self.rainfall[RainSide::Right.index()];
            state.image = None;
            state.view = None;
        }
    }

    fn invalidate_chart(&mut self) {
        self.chart_worker.invalidate(CHART_SLOT);
        self.loading[CHART_SLOT] = false;
    }

    fn select_location(&mut self, location: Location) {
        self.display.selection = Some(location);
        self.request_view();
    }

    fn view_time_series(&mut self, location: &Location) {
        if let Some(project) = &self.project {
            self.chart_fields.select(location, &project.stations);
            self.display.comparison = Some(self.chart_fields.clone());
            self.display.layout.open(SideTab::Charts);
            self.request_chart();
        }
    }

    fn upstream_outlet(&self, location: &Location) -> Option<i64> {
        let project = self.project.as_ref()?;
        let mini = data::location_mini(location, &project.stations)?;
        project
            .topology
            .iter()
            .any(|entry| entry.mini_id == mini)
            .then_some(mini)
    }

    fn clear_upstream(&mut self) {
        self.worker.invalidate(UPSTREAM_SLOT);
        self.loading[UPSTREAM_SLOT] = false;
        self.upstream_basin = None;
    }

    fn load_upstream(&mut self, mini: i64) {
        self.worker.invalidate(UPSTREAM_SLOT);
        if let Some(project) = self.project.clone() {
            if let Some(layer) = self
                .display
                .layers
                .iter_mut()
                .find(|l| l.kind == "upstream")
            {
                layer.visible = true;
            }
            self.submit(Job::Upstream(project, mini));
        }
    }

    fn request_chart(&mut self) {
        self.invalidate_chart();
        self.chart_message.clear();
        self.chart_retained = self.charts.report.is_some();
        let Some(project) = &self.project else {
            return;
        };
        let Some(fields) = &self.display.comparison else {
            return;
        };
        let pt = self.preferences.portuguese;
        let station = (!fields.station.trim().is_empty()).then(|| fields.station.trim().to_owned());
        let mini = fields.mini.trim();
        let location = if !mini.is_empty() {
            match mini.parse::<i64>() {
                Ok(id) if id > 0 => Location::Mini(id),
                _ => {
                    self.chart_message = text(
                        pt,
                        "Enter a positive MINI ID.",
                        "Digite um ID MINI positivo.",
                    );
                    return;
                }
            }
        } else if let Some(id) = &station {
            Location::Station(id.clone())
        } else {
            self.charts.clear();
            self.chart_retained = false;
            return;
        };
        let zone = self
            .run
            .as_ref()
            .filter(|run| self.run_selection_ready && !run.time_zone.is_empty())
            .map_or(&project.time_zone, |run| &run.time_zone)
            .clone();
        let window = data::timestamp(&self.display.chart_start).and_then(|start| {
            Ok(HourlyWindow::Explicit {
                start,
                end: data::timestamp(&self.display.chart_end)?,
            })
        });
        let window = match window {
            Ok(window) => window,
            Err(error) => {
                self.chart_message = error;
                return;
            }
        };
        let observations = station.as_deref().filter(|_| self.display.observations);
        let run = self
            .run
            .as_deref()
            .filter(|_| self.run_selection_ready && matches!(location, Location::Mini(_)));
        let selections = data::chart_selections(project, run, &self.display.overlays, observations);
        let mut notices = Vec::new();
        if let Some(id) = observations
            && !selections
                .iter()
                .any(|selection| matches!(selection, SeriesSelection::Observation { .. }))
        {
            notices.push(format!(
                "{} {id}",
                text(
                    pt,
                    "No observation series for station",
                    "Sem séries observadas para a estação"
                )
            ));
        }
        self.submit(Job::Chart(ChartRequest {
            run: run.and_then(|run| self.selected_run(&run.root)),
            query: SeriesQuery {
                location,
                time_zone: zone,
                window,
                selections,
            },
            comparison_station: station.filter(|_| !mini.is_empty()),
            notices,
        }));
    }

    fn poll(&mut self, context: &egui::Context) {
        let replies = self
            .worker
            .poll()
            .into_iter()
            .chain(self.chart_worker.poll());
        for reply in replies {
            let rain_side = RainSide::from_slot(reply.slot);
            if (matches!(reply.slot, RUN_SLOT | CHART_SLOT) || rain_side.is_some())
                && !self.loading[0]
            {
                self.check_active_run(false);
            }
            let worker = if reply.slot == CHART_SLOT {
                &self.chart_worker
            } else {
                &self.worker
            };
            if worker.generations[reply.slot] != reply.generation {
                continue;
            }
            self.loading[reply.slot] = false;
            if reply.slot == 0 {
                self.switching_project = false;
            }
            if rain_side.is_some() && self.check_rainfall_cache() {
                self.request_view();
                continue;
            }
            match reply.result {
                Err(error) if reply.slot == CHART_SLOT => self.chart_message = error,
                Err(error) if reply.slot == UPSTREAM_SLOT => self.status = error,
                Err(error) if reply.slot == RUN_SLOT => {
                    self.status = error;
                    if self.run.is_none() {
                        self.request_chart();
                    }
                }
                Err(error) if rain_side.is_some() => {
                    self.rainfall[rain_side.unwrap().index()].finding = Some(error);
                }
                Err(error) => {
                    if reply.slot == 0 {
                        self.request_view();
                    }
                    self.status = format!(
                        "{error} · {}",
                        text(
                            self.preferences.portuguese,
                            "Previous view retained",
                            "Visualização anterior mantida"
                        )
                    );
                }
                Ok(Loaded::Project(project)) => {
                    let same = self.project.as_ref().is_some_and(|p| {
                        p.config == project.config && p.workspace == project.workspace
                    });
                    let geometry_changed = self
                        .project
                        .as_ref()
                        .is_none_or(|old| !Arc::ptr_eq(&old.fills, &project.fills));
                    let reload_basin = if same && geometry_changed {
                        self.upstream_basin.as_ref().map(|(mini, _)| *mini)
                    } else {
                        None
                    };
                    if same {
                        self.inundation.refresh();
                        self.runners.start(
                            runners::Action::Initialize {
                                config: project.config.clone(),
                            },
                            context,
                        );
                    }
                    if !same {
                        self.inundation.select(None);
                        self.remember();
                        self.display = self
                            .preferences
                            .recent
                            .iter()
                            .find(|p| p.config == project.config)
                            .cloned()
                            .unwrap_or_default();
                        self.display.config = project.config.clone();
                        self.display.workspace = project.workspace.clone();
                        self.invalidate_chart();
                        self.charts.clear();
                        self.chart_message.clear();
                        self.run = None;
                        self.active_run = Ok(None);
                        self.comparison_runs.clear();
                        self.rainfall = Default::default();
                        self.rainfall_info = Ok(None);
                        for side in RainSide::BOTH {
                            self.worker.invalidate(side.slot());
                            self.loading[side.slot()] = false;
                        }
                        self.local.clear();
                        self.worker.invalidate(1);
                        self.worker.invalidate(3);
                        self.loading[1..4].fill(false);
                    }
                    if geometry_changed {
                        self.cache.iter_mut().for_each(map::StaticCache::clear);
                        self.worker.invalidate(UPSTREAM_SLOT);
                        self.loading[UPSTREAM_SLOT] = false;
                        if !same {
                            self.upstream_basin = None;
                        }
                        self.map_menu = None;
                    }
                    if !same {
                        self.assistant.reset();
                        self.runners = runners::Runners::new(project.session.clone());
                        self.workflows = workflows::Forms::default();
                        self.calibration = tools::CalibrationTools::default();
                        self.runners.start(
                            runners::Action::Initialize {
                                config: project.config.clone(),
                            },
                            context,
                        );
                    }
                    self.project = Some(project.clone());
                    if let Some(mini) = reload_basin {
                        self.load_upstream(mini);
                    }
                    self.check_rainfall_cache();
                    if !self.display.local.is_empty() {
                        self.submit(Job::Local(
                            self.display.local.iter().map(|l| l.path.clone()).collect(),
                        ));
                    }
                    if self.display.chart_start.is_empty() || self.display.chart_end.is_empty() {
                        use chrono::Timelike;
                        let now = chrono::Utc::now()
                            .with_timezone(
                                &project
                                    .time_zone
                                    .parse::<chrono_tz::Tz>()
                                    .unwrap_or(chrono_tz::UTC),
                            )
                            .naive_local()
                            .with_minute(0)
                            .unwrap()
                            .with_second(0)
                            .unwrap()
                            .with_nanosecond(0)
                            .unwrap();
                        if self.display.chart_start.is_empty() {
                            self.display.chart_start = (now - chrono::Duration::hours(168))
                                .format("%Y-%m-%dT%H:%M:%S")
                                .to_string();
                        }
                        if self.display.chart_end.is_empty() {
                            self.display.chart_end = now.format("%Y-%m-%dT%H:%M:%S").to_string();
                        }
                    }
                    if self.display.comparison.is_none() {
                        let mut fields = ComparisonFields::default();
                        if let Some(location) = &self.display.selection {
                            fields.select(location, &project.stations);
                        }
                        self.display.comparison = Some(fields);
                    }
                    if !same {
                        self.chart_fields = self.display.comparison.clone().unwrap_or_default();
                    }
                    if let Some(location) = &self.display.selection {
                        let valid = match location {
                            Location::Station(id) => {
                                project.stations.iter().any(|s| s.station.station_id == *id)
                            }
                            Location::Mini(id) => project.topology.iter().any(|m| m.mini_id == *id),
                        };
                        if !valid {
                            self.display.selection = None;
                            self.status = text(
                                self.preferences.portuguese,
                                "Remembered selection is unavailable",
                                "Seleção lembrada indisponível",
                            );
                        }
                    }
                    self.status = format!(
                        "{} · {} {} · {} MINIs · {} {}",
                        project.workspace.display(),
                        project.stations.len(),
                        text(self.preferences.portuguese, "stations", "estações"),
                        project.topology.len(),
                        project.findings.len(),
                        text(self.preferences.portuguese, "findings", "avisos")
                    );
                    self.check_active_run(true);
                    self.request_view();
                }
                Ok(Loaded::Run(run)) => {
                    if self.active_run.as_ref().ok().and_then(Option::as_ref) != Some(&run.root)
                        && !self.comparison_runs.contains(&run.root)
                    {
                        self.comparison_runs.push(run.root.clone());
                    }
                    if !run.start.is_empty() {
                        self.display.chart_start.clone_from(&run.start);
                        self.display.chart_end.clone_from(&run.end);
                    }
                    self.display.overlays.clone_from(&run.scenarios);
                    self.status = format!(
                        "{} · {}",
                        text(
                            self.preferences.portuguese,
                            "Run loaded",
                            "Execução carregada"
                        ),
                        run.root.display(),
                    );
                    self.run = Some(run);
                    self.run_selection_ready = true;
                    self.request_chart();
                }
                Ok(Loaded::Chart {
                    report,
                    station,
                    labels,
                    notices,
                }) => {
                    self.apply_chart(report, station, labels, notices);
                }
                Ok(Loaded::View(view)) => {
                    let view = *view;
                    let side = rain_side.expect("rainfall replies use a rainfall slot");
                    if view.rainfall.as_ref().is_some_and(|rain| {
                        self.rainfall_info
                            .as_ref()
                            .ok()
                            .and_then(Option::as_ref)
                            .is_none_or(|info| info.generation != rain.generation)
                    }) {
                        self.request_rainfall(side);
                        continue;
                    }
                    let hue = self.rainfall_color();
                    self.rainfall[side.index()].replace(
                        context,
                        view,
                        self.display.rain_maximum,
                        hue,
                    );
                }
                Ok(Loaded::Upstream { mini, paths }) => {
                    self.upstream_basin = Some((mini, paths));
                }
                Ok(Loaded::Local(layers, findings)) => {
                    self.local.retain(|old| {
                        self.display
                            .local
                            .iter()
                            .any(|style| style.path == old.path)
                            && !layers.iter().any(|new| new.path == old.path)
                    });
                    self.local.extend(layers.into_iter().filter(|layer| {
                        self.display
                            .local
                            .iter()
                            .any(|style| style.path == layer.path)
                    }));
                    self.status = if findings.is_empty() {
                        text(
                            self.preferences.portuguese,
                            "Display layers loaded",
                            "Camadas de visualização carregadas",
                        )
                    } else {
                        format!(
                            "{} · {}",
                            findings.join("; "),
                            text(
                                self.preferences.portuguese,
                                "Existing display layers retained where available",
                                "Camadas anteriores mantidas quando disponíveis"
                            )
                        )
                    };
                }
                Ok(Loaded::Export(path)) => {
                    self.status = format!(
                        "{}: {}",
                        text(self.preferences.portuguese, "Exported", "Exportado"),
                        path.display()
                    )
                }
            }
        }
    }
    fn apply_chart(
        &mut self,
        report: mgb_ops::app::SeriesReport,
        station: Option<String>,
        labels: Vec<String>,
        mut notices: Vec<String>,
    ) {
        let pt = self.preferences.portuguese;
        notices.extend(
            report
                .findings
                .iter()
                .map(|finding| finding.message.clone()),
        );
        if report.series.is_empty() && !report.findings.is_empty() {
            self.chart_message = notices.join(" · ");
            return;
        }
        self.chart_retained = false;
        if !report
            .series
            .iter()
            .any(|series| series.points.iter().any(|point| point.value.is_some()))
        {
            notices.insert(
                0,
                text(
                    pt,
                    "No data in the chart window.",
                    "Sem dados no período dos gráficos.",
                ),
            );
        }
        if report.series.is_empty() {
            self.charts.clear();
            self.chart_message = notices.join(" · ");
            return;
        }
        let station_id = station.as_deref().or(match &report.location {
            Location::Station(id) => Some(id.as_str()),
            Location::Mini(_) => None,
        });
        let references = self
            .project
            .as_ref()
            .map(|project| {
                project
                    .references
                    .iter()
                    .filter(|reference| Some(reference.station_id.as_str()) == station_id)
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.charts.source_labels = labels;
        self.charts.scenario_order = self
            .run
            .as_ref()
            .map(|run| run.scenarios.clone())
            .unwrap_or_default();
        match self.charts.load(report, &references, pt) {
            Ok(()) => self.charts.comparison_station = station,
            Err(error) => {
                self.charts.clear();
                notices.push(error);
            }
        }
        self.chart_message = notices.join(" · ");
    }

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        egui::ScrollArea::horizontal()
            .id_salt("ribbon-actions")
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ribbon_group(ui, text(pt, "Workspace", "Área de trabalho"), |ui| {
                        let label = text(pt, "Open workspace…", "Abrir área de trabalho…");
                        let button = ribbon_action(ui, "📂", &label);
                        let open = ui.add(if self.project.is_none() {
                            button.fill(ui.visuals().selection.bg_fill)
                        } else {
                            button
                        });
                        button_label(&open, &label);
                        if open.clicked() {
                            self.choose_project();
                        }
                        if !self.preferences.recent.is_empty() {
                            let label = text(pt, "Recent", "Recentes");
                            let (response, _) = egui::containers::menu::MenuButton::from_button(
                                ribbon_action(ui, "🕒", &label),
                            )
                            .ui(ui, |ui| {
                                for recent in self.preferences.recent.clone() {
                                    if ui.button(recent.config.display().to_string()).clicked() {
                                        self.depart(Departure::Project(
                                            recent.config,
                                            recent.workspace,
                                        ));
                                        ui.close();
                                    }
                                }
                            });
                            button_label(&response, &label);
                        }
                        ui.add_enabled_ui(self.project.is_some(), |ui| {
                            if ribbon_button(ui, "⚙", Form::Workspace.label(pt)).clicked() {
                                self.display.layout.form(Form::Workspace);
                            }
                            if ribbon_button(ui, "🚪", text(pt, "Close project", "Fechar projeto"))
                                .clicked()
                            {
                                self.depart(Departure::Close);
                            }
                        });
                    });
                    ribbon_group(ui, text(pt, "Monitoring", "Monitoramento"), |ui| {
                        if ribbon_button(ui, "📥", Form::Intake.label(pt)).clicked() {
                            self.display.layout.form(Form::Intake);
                        }
                        ui.add_enabled_ui(
                            !self.switching_project && self.project.is_some(),
                            |ui| {
                                if ribbon_button(
                                    ui,
                                    "🔄",
                                    text(pt, "Refresh (F5)", "Atualizar (F5)"),
                                )
                                .clicked()
                                    && let Some(project) = &self.project
                                {
                                    self.submit(Job::Open(
                                        project.config.clone(),
                                        project.workspace.clone(),
                                    ));
                                }
                            },
                        );
                    });
                    ribbon_group(ui, text(pt, "Pages", "Páginas"), |ui| {
                        for (tab, emoji) in [
                            (SideTab::Map, "🗺"),
                            (SideTab::Charts, "📈"),
                            (SideTab::Data, "📊"),
                            (SideTab::Files, "📁"),
                        ] {
                            if ribbon_button(ui, emoji, tab.label(pt)).clicked() {
                                self.display.layout.open(tab);
                            }
                        }
                        if ribbon_button(ui, "💬", Form::Assistant.label(pt)).clicked() {
                            self.display.layout.form(Form::Assistant);
                        }
                    });
                    ribbon_group(
                        ui,
                        text(pt, "Review and runs", "Revisão e execuções"),
                        |ui| {
                            for (form, emoji) in [
                                (Form::Review, "🔍"),
                                (Form::Analysis, "📉"),
                                (Form::Bulletin, "📰"),
                                (Form::RunBundle, "📦"),
                                (Form::Jobs, "📋"),
                            ] {
                                if ribbon_button(ui, emoji, form.label(pt)).clicked() {
                                    self.display.layout.form(form);
                                }
                            }
                        },
                    );
                    ribbon_group(ui, text(pt, "Layout", "Layout"), |ui| {
                        self.interface_settings(ui);
                        if ribbon_button(ui, "🔁", text(pt, "Reset layout", "Restaurar layout"))
                            .clicked()
                        {
                            self.display.layout = layout::Layout::default();
                            let id = egui::Id::new(("right-pane", &self.display.config));
                            ui.ctx()
                                .data_mut(|d| d.remove::<egui::containers::panel::PanelState>(id));
                        }
                    });
                    ribbon_group(ui, text(pt, "Operation", "Operação"), |ui| {
                        let label = Form::NewRun.label(pt);
                        let response = ui.add(
                            ribbon_action(ui, "▶", &label).fill(ui.visuals().selection.bg_fill),
                        );
                        button_label(&response, &label);
                        if response.clicked() {
                            self.display.layout.form(Form::NewRun);
                        }
                        if ribbon_button(ui, "⬇", Form::Acquisition.label(pt)).clicked() {
                            self.display.layout.form(Form::Acquisition);
                        }
                        if ribbon_button(ui, "📥", Form::Forcing.label(pt)).clicked() {
                            self.display.layout.form(Form::Forcing);
                        }
                        if ribbon_button(ui, "🕒", Form::Automation.label(pt)).clicked() {
                            self.display.layout.form(Form::Automation);
                        }
                    });
                    ribbon_group(ui, text(pt, "Calibration", "Calibração"), |ui| {
                        for (form, emoji) in [(Form::Geometry, "📐"), (Form::Soil, "🌱")] {
                            if ribbon_button(ui, emoji, form.label(pt)).clicked() {
                                self.display.layout.form(form);
                            }
                        }
                    });
                });
            });
    }
    fn interface_settings(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        let old = self.preferences.portuguese;
        let previous_appearance = self.preferences.appearance;
        let language = text(pt, "Language", "Idioma");
        let (response, _) = egui::containers::menu::MenuButton::from_button(ribbon_action(
            ui, "🌐", &language,
        ))
        .ui(ui, |ui| {
            for (portuguese, name) in [(false, "English"), (true, "Português (Brasil)")] {
                if ui
                    .selectable_value(&mut self.preferences.portuguese, portuguese, name)
                    .clicked()
                {
                    ui.close();
                }
            }
        });
        button_label(&response, &language);
        let normal = ui.visuals().widgets.inactive;
        egui::Frame::new()
            .fill(normal.bg_fill)
            .stroke(normal.bg_stroke)
            .corner_radius(normal.corner_radius)
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.;
                    for (value, icon, label) in [
                        (1, "☀", text(pt, "Light", "Claro")),
                        (2, "🌙", text(pt, "Dark", "Escuro")),
                    ] {
                        if value == 2 {
                            ui.add_space(1.);
                        }
                        let selected = (self.preferences.appearance == 2) == (value == 2);
                        let radius = normal.corner_radius.nw.saturating_sub(1);
                        let corners = if value == 1 {
                            egui::CornerRadius {
                                nw: radius,
                                sw: radius,
                                ..egui::CornerRadius::ZERO
                            }
                        } else {
                            egui::CornerRadius {
                                ne: radius,
                                se: radius,
                                ..egui::CornerRadius::ZERO
                            }
                        };
                        let mut button = ribbon_action(ui, icon, &label)
                            .min_size(egui::vec2(48., 0.))
                            .corner_radius(corners)
                            .stroke(egui::Stroke::NONE);
                        if selected {
                            button = button.fill(ui.visuals().selection.bg_fill);
                        }
                        let response = ui.add(button);
                        if value == 1 {
                            ui.painter().vline(
                                response.rect.right() + 0.5,
                                response.rect.y_range(),
                                normal.bg_stroke,
                            );
                        }
                        if response.clicked() {
                            self.preferences.appearance = value;
                        }
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::Button,
                                response.enabled(),
                                selected,
                                &label,
                            )
                        });
                    }
                });
            });
        if previous_appearance != self.preferences.appearance || old != self.preferences.portuguese
        {
            ui.ctx().request_repaint();
        }
        if old != self.preferences.portuguese
            && let Err(error) = self.charts.relabel(self.preferences.portuguese)
        {
            self.status = error;
        }
    }

    fn right_pane(&mut self, ui: &mut egui::Ui) {
        let size = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        if let Some(pointer) = ui.input(|input| {
            input
                .pointer
                .button_pressed(egui::PointerButton::Primary)
                .then(|| input.pointer.interact_pos())
                .flatten()
        }) {
            self.display.layout.focus_group_at(rect, pointer);
        }
        let snapshot = self.display.layout.clone();
        let pt = self.preferences.portuguese;
        if let Some(action) = snapshot.show(ui, rect, pt, |ui, tab| self.right_tab(ui, tab, pt)) {
            self.display.layout.apply(action);
            ui.request_repaint();
        }
    }

    fn right_tab(&mut self, ui: &mut egui::Ui, tab: SideTab, pt: bool) {
        match tab {
            SideTab::Map => self.map_controls(ui),
            SideTab::Charts => self.chart_page(ui, pt),
            SideTab::Data => self.data_page(ui),
            SideTab::Files => {
                egui::ScrollArea::vertical().show(ui, |ui| self.files(ui));
            }
            SideTab::Rainfall | SideTab::Interface => {}
            SideTab::Form(form) => {
                ui.add_enabled_ui(!self.switching_project, |ui| match form {
                    Form::Workspace => {
                        egui::ScrollArea::vertical()
                            .id_salt("workspace-settings")
                            .auto_shrink([false, false])
                            .show(ui, |ui| self.runners.workspace_settings(ui, pt));
                    }
                    Form::Assistant => {
                        self.assistant.show(
                            ui,
                            pt,
                            self.project.as_ref().map(|project| &project.session),
                        );
                    }
                    Form::Intake
                    | Form::Acquisition
                    | Form::Review
                    | Form::RunBundle
                    | Form::Forcing
                    | Form::Geometry
                    | Form::Soil
                    | Form::Analysis
                    | Form::Bulletin
                    | Form::Automation => self.workflow_form(ui, form, pt),
                    _ => self.runners.show(
                        ui,
                        form,
                        pt,
                        self.project
                            .as_ref()
                            .map(|project| project.workspace.as_path()),
                    ),
                });
            }
        }
    }

    fn workflow_form(&mut self, ui: &mut egui::Ui, form: Form, pt: bool) {
        let busy = self.runners.busy();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if form == Form::Automation
                    && let Some(active) = self
                        .runners
                        .active
                        .as_ref()
                        .filter(|active| active.automation.is_some())
                {
                    if ui
                        .button(text(pt, "Stop scheduling", "Parar agendamento"))
                        .clicked()
                    {
                        active.cancel();
                    }
                    self.runners.jobs(ui, pt);
                }
                if form == Form::Acquisition {
                    ui.add_enabled_ui(!busy, |ui| {
                        if let Some(project) = &self.project {
                            if let Some(task) =
                                self.workflows.acquisition(ui, pt, &project.workspace)
                            {
                                self.runners
                                    .start(runners::Action::Workflow(task), ui.ctx());
                            }
                        } else {
                            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
                        }
                    });
                    self.runners.jobs(ui, pt);
                    return;
                }
                ui.add_enabled_ui(!busy, |ui| {
                    let Some(project) = &self.project else {
                        ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
                        return;
                    };
                    let action = match form {
                        Form::Forcing => {
                            if let Some(setup) = project.session.run_setup() {
                                let mut parameters = setup.operation.parameters().clone();
                                self.workflows.forcing(ui, pt, &mut parameters);
                                if !busy {
                                    let _ = project
                                        .session
                                        .update_run_setup(&setup.revision, parameters);
                                }
                            }
                            None
                        }
                        Form::Geometry | Form::Soil => {
                            let setup = project.session.run_setup();
                            let mut parameters = setup
                                .as_ref()
                                .map(|setup| setup.operation.parameters().clone())
                                .unwrap_or_default();
                            let geometry = parameters
                                .geometry
                                .as_ref()
                                .map(|path| project.workspace.join(path))
                                .unwrap_or_else(|| workflows::baseline_geometry(project));
                            if form == Form::Geometry {
                                if let Some(path) =
                                    self.calibration.geometry(ui, pt, Some(&geometry))
                                {
                                    parameters.geometry = Some(path);
                                    parameters
                                        .calibration
                                        .get_or_insert_default()
                                        .use_edited_geometry = true;
                                }
                            } else {
                                let soil = parameters
                                    .calibration
                                    .as_ref()
                                    .and_then(|selection| selection.soil.as_ref())
                                    .map(|path| project.workspace.join(path));
                                if let Some(path) =
                                    self.calibration
                                        .soil(ui, pt, Some(&geometry), soil.as_deref())
                                {
                                    parameters.calibration.get_or_insert_default().soil =
                                        Some(path);
                                }
                            }
                            if let Some(setup) = setup
                                && !busy
                            {
                                let _ = project
                                    .session
                                    .update_run_setup(&setup.revision, parameters);
                            }
                            None
                        }
                        _ => match self
                            .workflows
                            .show(ui, form, pt, project, self.run.as_deref())
                        {
                            Ok(task) => task.map(runners::Action::Workflow),
                            Err(error) => {
                                self.runners.status = error;
                                None
                            }
                        },
                    };
                    if !busy && let Some(action) = action {
                        self.runners.start(action, ui.ctx());
                    }
                });
                ui.label(self.runners.localized_status(pt));
            });
    }

    fn files(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        let Some(project) = self.project.clone() else {
            return;
        };
        ui.heading(text(pt, "Workspace", "Área de trabalho"));
        ui.label(project.workspace.display().to_string());
        for path in &project.retained_sources {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                format!(
                    "{}: {}",
                    text(pt, "Retained source", "Fonte anterior mantida"),
                    path.display()
                ),
            );
        }
        let active_label = text(pt, "Active run", "Execução ativa");
        if let Err(error) = &self.active_run {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        let current = project.session.run_selection();
        let mut selected = current.clone();
        let mut open_comparison = false;
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_label(text(pt, "Run source", "Fonte da execução"))
                .width(180.)
                .truncate()
                .selected_text(match &selected {
                    RunSelection::Active => active_label.clone(),
                    RunSelection::Saved(path) => path.display().to_string(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected, RunSelection::Active, active_label);
                    for root in &self.comparison_runs {
                        ui.selectable_value(
                            &mut selected,
                            RunSelection::Saved(root.clone()),
                            root.display().to_string(),
                        );
                    }
                });
            if ui
                .button(text(
                    pt,
                    "Open comparison run…",
                    "Abrir execução para comparação…",
                ))
                .clicked()
                && let Some(root) = rfd::FileDialog::new().pick_folder()
            {
                selected = RunSelection::Saved(root);
                open_comparison = true;
            }
        });
        if !self.loading[0] && (selected != current || open_comparison) {
            match project.session.select_run(selected) {
                Ok(()) => self.load_run(),
                Err(error) => self.status = error,
            }
        }
        if project.session.run_selection() == RunSelection::Active
            && matches!(self.active_run, Ok(None))
        {
            ui.label(text(pt, "No active run.", "Nenhuma execução ativa."));
        }

        if let Some(run) = &self.run {
            section(
                ui,
                text(
                    pt,
                    "Run status and provenance",
                    "Estado e proveniência da execução",
                ),
                |ui| {
                    ui.small(status_label(&run.manifest.state, pt));
                    ui.small(format!(
                        "{}: {}",
                        text(pt, "Reference", "Referência"),
                        run.reference
                    ));
                    ui.label(run.root.display().to_string());
                    for finding in &run.findings {
                        ui.colored_label(ui.visuals().warn_fg_color, finding);
                    }
                    if let Some(error) = &run.manifest.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    for scenario in &run.manifest.scenarios {
                        ui.label(format!(
                            "{}: {}",
                            scenario.id,
                            status_label(&scenario.state, pt)
                        ));
                        if let Some(error) = &scenario.error {
                            ui.colored_label(ui.visuals().error_fg_color, &error.message);
                        }
                    }
                    ui.label(
                        serde_json::to_string_pretty(&run.manifest.recipe).unwrap_or_default(),
                    );
                },
            );
        }
        section(
            ui,
            text(
                pt,
                "Available assets and findings",
                "Dados disponíveis e avisos",
            ),
            |ui| {
                for asset in &project.assets {
                    ui.label(asset);
                }
                for finding in &project.findings {
                    ui.colored_label(ui.visuals().warn_fg_color, finding);
                }
            },
        );
    }

    fn location_search(&mut self, ui: &mut egui::Ui, project: &Project) {
        let pt = self.preferences.portuguese;
        let search = self.search.to_lowercase();
        let mut clicked = None;
        ui.horizontal_wrapped(|ui| {
            let name = text(pt, "Search station or MINI", "Buscar estação ou MINI");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.search)
                    .hint_text(&name)
                    .desired_width(ui.available_width().min(165.)),
            );
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, ui.is_enabled(), &name)
            });
            if response.lost_focus()
                && ui.input(|input| input.key_pressed(egui::Key::Enter))
                && let Ok(id) = self.search.trim().parse::<i64>()
                && id > 0
            {
                clicked = Some(Location::Mini(id));
            }
            ui.menu_button(text(pt, "Locations", "Locais"), |ui| {
                let stations = project.stations.iter().filter_map(|entry| {
                    let station = &entry.station;
                    let label = format!("{} · {}", station.name, station.station_id);
                    label
                        .to_lowercase()
                        .contains(&search)
                        .then(|| (Location::Station(station.station_id.clone()), label))
                });
                let minis = project.topology.iter().filter_map(|mini| {
                    let id = mini.mini_id;
                    id.to_string()
                        .contains(&search)
                        .then(|| (Location::Mini(id), format!("MINI {id}")))
                });
                let rows: Vec<_> = stations.chain(minis).collect();
                egui::ScrollArea::vertical().max_height(240.).show_rows(
                    ui,
                    ui.spacing().interact_size.y,
                    rows.len(),
                    |ui, visible| {
                        for (location, label) in &rows[visible] {
                            if ui.button(label).clicked() {
                                clicked = Some(location.clone());
                                ui.close();
                            }
                        }
                    },
                );
            });
        });
        if let Some(location) = clicked {
            self.select_location(location);
        }
    }

    fn map_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.map_cards(ui, pt);
            ui.heading(text(pt, "Layers", "Camadas"));
            self.layer_controls(ui);
            self.inundation_controls(ui, pt);
            self.rainfall_controls(ui);
        });
    }

    fn inundation_visible(&self) -> bool {
        self.display
            .layers
            .iter()
            .any(|layer| layer.kind == inundation::KIND && layer.visible)
    }

    fn sync_inundation(&mut self) {
        if self.switching_project
            || self.loading[0]
            || self.loading[RUN_SLOT]
            || !self.run_selection_ready
        {
            return;
        }
        let run = self.run.as_deref();
        let selection = run.and_then(|run| self.selected_run(&run.root));
        let scenario = &mut self.display.inundation_scenario;
        let source = run.and_then(|run| {
            if !run.scenarios.contains(scenario) {
                scenario.clone_from(run.scenarios.first()?);
            }
            Some(inundation::Source {
                selection,
                root: run.root.clone(),
                scenario: scenario.clone(),
            })
        });
        self.inundation.select(source);
    }

    fn inundation_controls(&mut self, ui: &mut egui::Ui, pt: bool) {
        if !self.inundation_visible() {
            return;
        }
        self.sync_inundation();
        section(ui, text(pt, "Inundation", "Inundação"), |ui| {
            let Some(run) = self.run.as_ref().filter(|run| {
                !self.switching_project && !self.loading[0] && !run.scenarios.is_empty()
            }) else {
                ui.label(text(
                    pt,
                    "Run results will appear here when available.",
                    "Os resultados disponíveis aparecerão aqui.",
                ));
                return;
            };
            let scenario = &mut self.display.inundation_scenario;
            egui::ComboBox::from_label(text(pt, "Inundation scenario", "Cenário de inundação"))
                .selected_text(scenario.as_str())
                .truncate()
                .show_ui(ui, |ui| {
                    for id in &run.scenarios {
                        ui.selectable_value(scenario, id.clone(), id);
                    }
                });
            self.sync_inundation();
            self.inundation.controls(ui, pt);
        });
    }

    fn run_summary(&self, pt: bool) -> String {
        let cache = self.rainfall_info.as_ref().ok().and_then(Option::as_ref);
        let run = self.run.as_deref();
        let reference = run
            .filter(|run| !run.reference.is_empty())
            .map(|run| run.reference.replace('T', " "))
            .or_else(|| cache.map(|info| info.reference.format("%Y-%m-%d %H:%M").to_string()))
            .unwrap_or_else(|| "—".into());
        let zones = [
            run.filter(|run| !run.time_zone.is_empty())
                .map(|run| (text(pt, "Run", "Execução"), run.time_zone.as_str())),
            cache.map(|info| (text(pt, "Rainfall", "Chuva"), info.time_zone.as_str())),
            self.charts
                .report
                .as_ref()
                .map(|report| (text(pt, "Charts", "Gráficos"), report.time_zone.as_str())),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let zone = zones.first().map_or_else(
            || {
                self.project
                    .as_ref()
                    .map_or("—", |project| &project.time_zone)
                    .to_owned()
            },
            |(_, first)| {
                if zones.iter().all(|(_, zone)| zone == first) {
                    (*first).to_owned()
                } else {
                    zones
                        .iter()
                        .map(|(name, zone)| format!("{name} {zone}"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            },
        );
        let mut result = format!(
            "{}: {reference} | {}: {zone}",
            text(pt, "Reference time", "Hora de referência"),
            text(pt, "Timezone", "Fuso horário")
        );
        if run.is_some() && !self.run_selection_ready {
            result = format!(
                "{} | {result}",
                text(pt, "Previous run retained", "Execução anterior mantida")
            );
        }
        if let Some(run) = run {
            let (obs, fc) = run
                .day_horizons
                .map_or(("—".into(), "—".into()), |(obs, fc)| {
                    (format!("{obs} d"), format!("{fc} d"))
                });
            let scenarios = if run.scenarios.is_empty() {
                "—".into()
            } else {
                run.scenarios.join(", ")
            };
            result.push_str(&format!(
                " | {}: {obs} | {}: {fc} | {}: {scenarios}",
                text(pt, "Observed", "Observado"),
                text(pt, "Forecast", "Previsto"),
                text(pt, "Scenarios", "Cenários")
            ));
        }
        result
    }

    fn feature_label(&self, location: &Location) -> String {
        if let Location::Station(id) = location
            && let Some(station) = self.project.as_ref().and_then(|project| {
                project
                    .stations
                    .iter()
                    .find(|entry| entry.station.station_id == *id)
            })
        {
            return format!("{} · {id}", station.station.name);
        }
        charts::location_label(location, self.preferences.portuguese)
    }

    fn basemap_label(index: usize, pt: bool) -> String {
        match index {
            1 => text(pt, "Street (OpenStreetMap)", "Ruas (OpenStreetMap)"),
            2 => text(pt, "Topographic (OpenTopoMap)", "Topográfico (OpenTopoMap)"),
            3 => text(
                pt,
                "Satellite (NASA Blue Marble)",
                "Satélite (NASA Blue Marble)",
            ),
            _ => text(pt, "None", "Nenhum"),
        }
    }

    fn layer_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        egui::ComboBox::from_label(text(pt, "Basemap", "Mapa base"))
            .selected_text(Self::basemap_label(self.display.basemap, pt))
            .show_ui(ui, |ui| {
                for index in 0..=3 {
                    ui.selectable_value(
                        &mut self.display.basemap,
                        index,
                        Self::basemap_label(index, pt),
                    );
                }
            });
        ui.checkbox(
            &mut self.display.labels,
            text(
                pt,
                "Station / rainfall labels",
                "Rótulos de estações / chuva",
            ),
        );
        if let Some(stations) = self
            .display
            .layers
            .iter()
            .find(|layer| layer.kind == "stations")
        {
            ui.horizontal_wrapped(|ui| {
                theme::color_key(
                    ui,
                    stations.display_color(),
                    &text(pt, "Known model link", "Vínculo conhecido"),
                );
                theme::color_key(
                    ui,
                    stations.display_secondary_color(),
                    &text(pt, "Unknown link", "Vínculo desconhecido"),
                );
            });
        }
        let mut clear = false;
        let mut reorder = None;
        let mut rainfall_color_changed = false;
        for (i, layer) in self.display.layers.iter_mut().enumerate() {
            let label = match layer.kind.as_str() {
                "catchments" => text(pt, "Catchments", "Bacias"),
                "rainfall" => text(pt, "Rainfall", "Chuva"),
                inundation::KIND => text(pt, "Modeled inundation", "Inundação modelada"),
                "upstream" => text(pt, "Upstream basin", "Bacia a montante"),
                "rivers" => text(pt, "Rivers", "Rios"),
                _ => text(pt, "Stations", "Estações"),
            };
            ui.push_id(layer.kind.clone(), |ui| {
                let previous_color = layer.display_color();
                let mut color = previous_color;
                let expanded = layer_row(ui, pt, &label, &mut color, &mut layer.visible);
                if color != previous_color {
                    layer.color = Some(color.to_srgba_unmultiplied());
                    rainfall_color_changed |= layer.kind == "rainfall";
                }
                if expanded {
                    ui.indent("layer-settings", |ui| {
                        layer_opacity(ui, pt, &mut layer.opacity);
                        if layer.kind == "stations" {
                            ui.horizontal_wrapped(|ui| {
                                let name =
                                    text(pt, "Unknown link colour", "Cor do vínculo desconhecido");
                                let previous = layer.display_secondary_color();
                                let mut color = previous;
                                theme::color_palette(ui, &mut color, &name, pt);
                                ui.label(name);
                                if color != previous {
                                    layer.secondary_color = Some(color.to_srgba_unmultiplied());
                                }
                            });
                        }
                        if !matches!(layer.kind.as_str(), "rainfall" | inundation::KIND) {
                            let default = if layer.kind == "catchments" { 0. } else { 1. };
                            layer_width(ui, pt, &mut layer.width, default);
                        }
                        if layer.kind == "upstream" {
                            ui.horizontal_wrapped(|ui| {
                                ui.selectable_value(
                                    &mut layer.filled,
                                    false,
                                    text(pt, "Outline", "Contorno"),
                                );
                                ui.selectable_value(
                                    &mut layer.filled,
                                    true,
                                    text(pt, "Filled", "Preenchida"),
                                );
                            });
                        }
                        if ui
                            .add_enabled(
                                i > 0,
                                egui::Button::new(text(pt, "Move up", "Mover acima")),
                            )
                            .clicked()
                        {
                            reorder = Some(i);
                        }
                    });
                }
                let loading = self.loading[UPSTREAM_SLOT];
                if layer.kind == "upstream" && (loading || self.upstream_basin.is_some()) {
                    ui.horizontal_wrapped(|ui| {
                        if let Some((mini, _)) = &self.upstream_basin {
                            ui.strong(format!("MINI {mini}"));
                        }
                        if loading {
                            ui.spinner();
                        }
                        if ui
                            .small_button(text(pt, "Clear basin", "Limpar bacia"))
                            .clicked()
                        {
                            clear = true;
                        }
                    });
                }
            });
        }
        if clear {
            self.clear_upstream();
            ui.ctx().request_discard("Remove cleared basin controls");
        }
        if let Some(i) = reorder {
            self.display.layers.swap(i, i - 1);
        }
        if rainfall_color_changed {
            self.recolor_rainfall();
        }
        ui.separator();
        if ui
            .button(text(pt, "Add local GIS…", "Adicionar GIS local…"))
            .clicked()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("GIS", &["gpkg", "geojson", "shp"])
                .pick_file()
        {
            self.display.local.push(map::LocalDisplay {
                path,
                visible: true,
                opacity: 1.,
                color: [255, 120, 120, 255],
                width: None,
            });
            self.submit(Job::Local(
                self.display.local.iter().map(|l| l.path.clone()).collect(),
            ));
        }
        let mut remove = None;
        let mut reorder = None;
        for (i, style) in self.display.local.iter_mut().enumerate() {
            let name = style
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            ui.push_id((&style.path, i), |ui| {
                let mut color = Color32::from_rgba_unmultiplied(
                    style.color[0],
                    style.color[1],
                    style.color[2],
                    style.color[3],
                );
                let expanded = layer_row(ui, pt, &name, &mut color, &mut style.visible);
                style.color = color.to_srgba_unmultiplied();
                if expanded {
                    ui.indent("local-layer-settings", |ui| {
                        layer_opacity(ui, pt, &mut style.opacity);
                        layer_width(ui, pt, &mut style.width, 1.);
                        if ui
                            .add_enabled(
                                i > 0,
                                egui::Button::new(text(pt, "Move up", "Mover acima")),
                            )
                            .clicked()
                        {
                            reorder = Some(i);
                        }
                        if ui.button(text(pt, "Remove", "Remover")).clicked() {
                            remove = Some(i);
                        }
                        for layer in self.local.iter().filter(|l| l.path == style.path) {
                            ui.small(&layer.name);
                        }
                    });
                }
            });
        }
        if let Some(i) = reorder {
            self.display.local.swap(i, i - 1);
        }
        if let Some(i) = remove {
            let path = self.display.local.remove(i).path;
            self.local.retain(|l| l.path != path);
        }
    }

    fn chart_exports(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        if ui
            .add_enabled(
                self.charts.report.is_some() && !self.loading[4],
                egui::Button::new(text(
                    pt,
                    "Export selected CSV…",
                    "Exportar CSV selecionado…",
                )),
            )
            .clicked()
            && let Some(report) = self.charts.report.clone()
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .set_file_name("series.csv")
                .save_file()
        {
            self.submit(Job::Export(
                report,
                self.charts.comparison_station.clone(),
                path,
            ));
        }
        ui.horizontal(|ui| {
            ui.label("PNG");
            for scale in 1..=3 {
                ui.selectable_value(&mut self.export_scale, scale, format!("{scale}×"));
            }
        });
        if ui
            .add_enabled(
                self.charts.report.is_some() && !self.loading[4],
                egui::Button::new(text(pt, "Export complete PNG…", "Exportar PNG completo…")),
            )
            .clicked()
            && let Some(report) = self.charts.report.clone()
            && let Some(path) = rfd::FileDialog::new()
                .set_file_name("monitoring.png")
                .save_file()
        {
            let references = self.charts.references.clone();
            let title = format!(
                "MGB Ops · {} · {}",
                self.charts.location_caption(&report, pt),
                report
                    .series
                    .iter()
                    .find_map(|s| s.run_id.as_deref())
                    .unwrap_or("OBS")
            );
            let export = export::Export {
                state: self.charts.state.clone(),
                report,
                source_labels: self.charts.source_labels.clone(),
                scenario_order: self.charts.scenario_order.clone(),
                references,
                portuguese: pt,
                scale: self.export_scale,
                title,
                limits: std::array::from_fn(|i| self.charts.plots[i].plot().limits),
                right_level_limits: self.charts.plots[2].plot().y2,
            };
            self.submit(Job::Png(Box::new(export), path));
        }
    }

    fn rainfall_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        if self.project.is_none() {
            return;
        }
        ui.separator();
        ui.heading(text(pt, "Rainfall", "Chuva"));
        let info = self.rainfall_info.clone();
        match &info {
            Ok(Some(info)) => {
                ui.small(format!(
                    "{}: {}",
                    text(pt, "Reference", "Referência"),
                    info.reference.format("%Y-%m-%d %H:%M")
                ));
            }
            Ok(None) => {
                ui.label(text(
                    pt,
                    "No active rainfall cache. Use Operation → New run.",
                    "Sem cache ativo de chuva. Use Operação → Nova execução.",
                ));
            }
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        }
        let comparing = self.display.map_mode.comparing();
        for side in RainSide::BOTH {
            if side == RainSide::Right && !comparing {
                continue;
            }
            ui.push_id(side.index(), |ui| {
                if comparing {
                    ui.separator();
                    ui.strong(map::side_label(side, pt));
                }
                if let Ok(Some(info)) = &info {
                    self.rainfall_period_controls(ui, side, info);
                }
            });
        }
    }

    fn rainfall_period_controls(
        &mut self,
        ui: &mut egui::Ui,
        side: RainSide,
        info: &RainfallCacheInfo,
    ) {
        let pt = self.preferences.portuguese;
        let mut applied = self.display.rainfall_settings(side);
        let scenario_label = ui.label(text(pt, "Scenario", "Cenário"));
        let before = applied.scenario.clone();
        egui::ComboBox::from_id_salt("rainfall-scenario")
            .selected_text(&applied.scenario)
            .show_ui(ui, |ui| {
                for id in &info.scenarios {
                    ui.selectable_value(&mut applied.scenario, id.clone(), id);
                }
            })
            .response
            .labelled_by(scenario_label.id);
        let draft = self.rainfall[side.index()]
            .draft
            .get_or_insert_with(|| applied.window.clone());
        let explicit = draft.explicit;
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut draft.explicit,
                false,
                text(pt, "Signed hours", "Horas com sinal"),
            );
            ui.selectable_value(
                &mut draft.explicit,
                true,
                text(pt, "Start / end", "Início / fim"),
            );
        });
        if draft.explicit
            && !explicit
            && draft.start.is_empty()
            && draft.end.is_empty()
            && let Ok(window) = RainfallPeriod::SignedHours(draft.hours).resolve(info)
        {
            draft.start = window.start.format("%Y-%m-%dT%H:%M:%S").to_string();
            draft.end = window.end.format("%Y-%m-%dT%H:%M:%S").to_string();
        }
        if draft.explicit {
            for (name, value) in [
                (text(pt, "From", "Início"), &mut draft.start),
                (text(pt, "To", "Fim"), &mut draft.end),
            ] {
                let label = ui.label(name);
                ui.add(egui::TextEdit::singleline(value).desired_width(145.))
                    .labelled_by(label.id);
            }
        } else {
            let label = ui.small(text(
                pt,
                "− Observed · + Forecast",
                "− Observado · + Previsto",
            ));
            ui.add(
                egui::DragValue::new(&mut draft.hours)
                    .range(-8760..=8760)
                    .suffix(" h"),
            )
            .labelled_by(label.id);
        }
        let window = draft.period().and_then(|period| period.resolve(info));
        match &window {
            Err(error) => {
                ui.small(error);
            }
            Ok(window) if window.start < info.coverage.start || window.end > info.coverage.end => {
                ui.small(text(
                    pt,
                    "Window extends beyond cached coverage",
                    "Período fora da cobertura do cache",
                ));
            }
            Ok(_) => (),
        }
        let apply = ui
            .add_enabled(
                window.is_ok(),
                egui::Button::new(text(pt, "Apply", "Aplicar")),
            )
            .clicked();
        if apply {
            applied.window = draft.clone();
        }
        if apply || before != applied.scenario {
            self.display.set_rainfall_settings(side, applied);
            self.request_rainfall(side);
        }
    }

    fn rainfall_total(&self, ui: &mut egui::Ui, side: RainSide, pt: bool) {
        if self.display.map_mode.comparing() {
            ui.strong(map::side_label(side, pt));
        }
        let state = &self.rainfall[side.index()];
        if let Some(rain) = &state.image {
            ui.small(&rain.rain.scenario);
            ui.small(format!(
                "{} – {}",
                rain.rain.window.start.format("%Y-%m-%d %H:%M"),
                rain.rain.window.end.format("%Y-%m-%d %H:%M"),
            ));
        }
        let matching_view = state
            .view
            .as_ref()
            .filter(|view| view.location == self.display.selection);
        let summary = matching_view.and_then(|view| view.summary.as_ref());
        let finding = state.finding.as_ref().or_else(|| {
            let view = matching_view?;
            view.rainfall_finding
                .as_ref()
                .or(view.summary_finding.as_ref())
        });
        if self.loading[side.slot()] {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.small(text(pt, "Loading rainfall…", "Carregando chuva…"));
            });
        }
        if let Some(finding) = finding {
            ui.colored_label(ui.visuals().error_fg_color, finding);
        }
        if state.image.is_some() && (self.loading[side.slot()] || finding.is_some()) {
            ui.weak(text(
                pt,
                "Previous rainfall display retained",
                "Visualização anterior de chuva mantida",
            ));
        }
        if self.display.selection.is_none() {
            ui.weak(text(
                pt,
                "Select a station or MINI",
                "Selecione uma estação ou MINI",
            ));
        } else if let Some(summary) = summary {
            let rain = summary
                .mean_mm
                .map_or_else(|| "NA".into(), |mm| format!("{mm:.1} mm"));
            ui.label(rain).on_hover_text(text(
                pt,
                "Area-weighted cumulative rainfall",
                "Chuva acumulada ponderada pela área",
            ));
        } else if !self.loading[side.slot()] {
            ui.weak(text(pt, "Rainfall unavailable", "Chuva indisponível"));
        }
    }

    fn chart_page(&mut self, ui: &mut egui::Ui, pt: bool) {
        if self.project.is_none() {
            theme::inset(ui.style()).show(ui, |ui| {
                ui.strong(text(pt, "Time series", "Séries temporais"));
                ui.label(text(
                    pt,
                    "Open a regional project to compare rainfall, flow, and level.",
                    "Abra um projeto regional para comparar chuva, vazão e nível.",
                ));
            });
            return;
        }

        ui.horizontal_wrapped(|ui| {
            let settings = ui.button(text(pt, "Settings", "Configurações"));
            egui::Popup::from_toggle_button_response(&settings)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_max_width(365.);
                    egui::ScrollArea::vertical()
                        .max_height(300.)
                        .show(ui, |ui| self.chart_settings(ui, pt));
                });
            let export = ui.add_enabled(
                self.charts.report.is_some(),
                egui::Button::new(text(pt, "Export", "Exportar")),
            );
            egui::Popup::from_toggle_button_response(&export)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_max_width(330.);
                    self.chart_exports(ui);
                });
        });
        self.chart_status(ui, pt);
        if self.charts.report.is_some() {
            let available_height = ui.available_height();
            egui::ScrollArea::vertical()
                .id_salt("chart-plots")
                .show(ui, |ui| {
                    theme::exclude_text_actions(ui.ctx(), ui.available_rect_before_wrap());
                    self.charts.show(ui, pt, available_height)
                });
        }
    }

    fn chart_settings(&mut self, ui: &mut egui::Ui, pt: bool) {
        let mut apply = false;
        ui.horizontal_wrapped(|ui| {
            for (name, value) in [
                (text(pt, "MINI ID", "ID MINI"), &mut self.chart_fields.mini),
                (
                    text(pt, "Station ID", "ID da estação"),
                    &mut self.chart_fields.station,
                ),
            ] {
                ui.horizontal(|ui| {
                    let label = ui.label(name);
                    let input = ui
                        .add(egui::TextEdit::singleline(value).desired_width(80.))
                        .labelled_by(label.id);
                    apply |= input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
            }
            apply |= ui
                .add(theme::primary(ui, text(pt, "Apply", "Aplicar")))
                .clicked();
        });
        ui.horizontal_wrapped(|ui| {
            for (name, value) in [
                (text(pt, "From", "Início"), &mut self.display.chart_start),
                (text(pt, "To", "Fim"), &mut self.display.chart_end),
            ] {
                ui.horizontal(|ui| {
                    let label = ui.label(name);
                    let input = ui
                        .add(egui::TextEdit::singleline(value).desired_width(145.))
                        .labelled_by(label.id);
                    apply |= input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                });
            }
        });
        ui.horizontal_wrapped(|ui| {
            apply |= ui
                .checkbox(
                    &mut self.display.observations,
                    text(pt, "Observed series", "Séries observadas"),
                )
                .changed();
            if let Some(run) = &self.run {
                ui.separator();
                ui.label(text(pt, "Model scenarios", "Cenários do modelo"));
                for id in &run.scenarios {
                    let mut selected = self.display.overlays.contains(id);
                    if ui.checkbox(&mut selected, id).changed() {
                        self.display.overlays.retain(|scenario| scenario != id);
                        if selected {
                            self.display.overlays.push(id.clone());
                        }
                        apply = true;
                    }
                }
            }
        });
        if apply {
            self.display.comparison = Some(self.chart_fields.clone());
            self.request_chart();
        }
    }

    fn chart_status(&mut self, ui: &mut egui::Ui, pt: bool) {
        if self.loading[CHART_SLOT] {
            theme::inset(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(text(pt, "Loading comparison…", "Carregando comparação…"));
                });
            });
        } else if !self.chart_message.is_empty() {
            theme::inset(ui.style()).show(ui, |ui| {
                ui.add(egui::Label::new(&self.chart_message).truncate())
                    .on_hover_text(&self.chart_message);
            });
        } else if self.charts.report.is_none() {
            theme::inset(ui.style()).show(ui, |ui| {
                ui.strong(text(pt, "Choose a location", "Escolha uma localização"));
                ui.label(text(
                    pt,
                    "Choose a location in Settings or right-click a station or MINI on the map and choose View time series.",
                    "Escolha uma localização em Configurações ou clique com o botão direito em uma estação ou MINI no mapa e escolha Ver séries temporais.",
                ));
                if let Some(location) = self.display.selection.clone()
                    && ui.button(text(pt, "Chart selected feature", "Mostrar séries da feição selecionada")).clicked()
                {
                    self.view_time_series(&location);
                }
            });
        }
        if self.chart_retained && self.charts.report.is_some() {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                text(
                    pt,
                    "Showing previous comparison",
                    "Exibindo comparação anterior",
                ),
            );
        }
    }
}

impl Monitoring {
    fn map_page(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let pt = self.preferences.portuguese;
        if self.project.is_some()
            && (self.tile_source != self.display.basemap
                || (self.tiles.is_none() && self.display.basemap != 0))
        {
            self.tile_source = self.display.basemap;
            self.tiles = match self.tile_source {
                1 => Some(HttpTiles::new(
                    walkers::sources::OpenStreetMap,
                    ui.ctx().clone(),
                )),
                2 => Some(HttpTiles::new(
                    walkers::sources::OpenTopoMap(walkers::sources::OpenTopoServer::A),
                    ui.ctx().clone(),
                )),
                3 => Some(HttpTiles::new(map::BlueMarble, ui.ctx().clone())),
                _ => None,
            };
        }
        let focus_map =
            ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Num1));
        let mut mode = self.display.map_mode;
        ui.horizontal_wrapped(|ui| {
            ui.strong(text(pt, "Map view", "Visualização do mapa"));
            for (value, name) in [
                (map::Mode::Single, text(pt, "Single", "Único")),
                (map::Mode::Double, text(pt, "Double", "Duplo")),
                (map::Mode::Swipe, text(pt, "Swipe", "Deslizar")),
            ] {
                let response = ui.selectable_value(&mut mode, value, name);
                if focus_map && mode == value {
                    response.request_focus();
                }
            }
        });
        if mode != self.display.map_mode {
            self.set_map_mode(mode);
        }
        self.sync_inundation();
        let mut map_hover = None;
        let map_rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        let frame = egui::Frame::group(ui.style())
            .inner_margin(1)
            .fill(ui.visuals().window_fill());
        theme::exclude_text_actions(ui.ctx(), ui.available_rect_before_wrap());
        if ui.available_height() > 16. && ui.available_width() > 16. {
            frame.show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                if let Some(project) = &self.project {
                    let upstream = self.upstream_basin.as_ref().map(|(_, paths)| paths);
                    let images =
                        RainSide::BOTH.map(|side| self.rainfall[side.index()].image.as_ref());
                    let mut render = |ui: &mut egui::Ui,
                                      side: RainSide,
                                      camera: &mut MapMemory,
                                      interactive,
                                      swipe| {
                        let mut hit = None;
                        let response = map::Overlay {
                            project,
                            layers: &self.display.layers,
                            rainfall: images,
                            inundation: &mut self.inundation,
                            side: (mode == map::Mode::Double).then_some(side),
                            selection: &self.display.selection,
                            upstream,
                            hit: &mut hit,
                            cache: &mut self.cache[side.index()],
                            labels: self.display.labels,
                            local: &self.local,
                            local_display: &self.display.local,
                        }
                        .show(
                            ui,
                            self.tiles.as_mut().map(|t| t as &mut dyn walkers::Tiles),
                            camera,
                            interactive,
                            swipe,
                            pt,
                        );
                        if response.inner.is_some() {
                            map_hover = response.inner;
                        }
                        (response.response, hit)
                    };
                    if mode == map::Mode::Double {
                        let responses = map::double(
                            ui,
                            &mut self.display.camera,
                            |ui, side, camera, interactive| {
                                render(ui, side, camera, interactive, None)
                            },
                        );
                        for (response, hit) in responses {
                            self.map_interaction(response, hit);
                        }
                    } else {
                        let swipe = (mode == map::Mode::Swipe).then_some(&mut self.display.swipe);
                        let (response, hit) =
                            render(ui, RainSide::Left, &mut self.display.camera, true, swipe);
                        self.map_interaction(response, hit);
                    }
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() * 0.3).min(140.));
                        if self.loading[0] {
                            ui.spinner();
                            ui.heading(text(pt, "Opening project…", "Abrindo projeto…"));
                        } else {
                            ui.heading(text(
                                pt,
                                "Open a regional project",
                                "Abra um projeto regional",
                            ));
                            if ui
                                .add(theme::primary(
                                    ui,
                                    text(pt, "Open workspace…", "Abrir área de trabalho…"),
                                ))
                                .clicked()
                            {
                                self.choose_project();
                            }
                        }
                    });
                }
            });
        }
        if let Some(project) = &self.project
            && map_rect.width() >= 48.
            && map_rect.height() >= 96.
        {
            egui::Area::new(egui::Id::new("map-navigation"))
                .order(egui::Order::Foreground)
                .fixed_pos(map_rect.left_top() + egui::vec2(10., 10.))
                .constrain_to(map_rect)
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 3.;
                        if icon_button(ui, "+", &text(pt, "Zoom in", "Aproximar")).clicked() {
                            let _ = self.display.camera.zoom_in();
                        }
                        if icon_button(ui, "−", &text(pt, "Zoom out", "Afastar")).clicked() {
                            let _ = self.display.camera.zoom_out();
                        }
                        if icon_button(ui, "⛶", &text(pt, "Full extent", "Extensão completa"))
                            .clicked()
                        {
                            let mut size = map_rect.size();
                            if mode == map::Mode::Double {
                                size.x /= 2.;
                            }
                            map::fit_extent(project, size, &mut self.display.camera);
                        }
                    });
                });
        }
        self.inundation
            .finish_view(ui.ctx(), self.inundation_visible());
        map_hover
    }

    fn map_interaction(&mut self, response: egui::Response, hit: Option<Location>) {
        if response.secondary_clicked() {
            self.map_menu = hit;
        } else if let Some(location) = hit {
            self.select_location(location);
            response.ctx.request_repaint();
        }
        map::feature_menu(&response, self.map_menu.clone(), |ui, location| {
            ui.label(self.feature_label(location));
            self.feature_actions(ui, location);
        });
    }

    fn map_cards(&mut self, ui: &mut egui::Ui, pt: bool) {
        if self.project.is_none() {
            return;
        }
        theme::inset(ui.style()).show(ui, |ui| {
            let id = ui.id().with(&self.display.config);
            let mut input = ui
                .data(|data| data.get_temp::<String>(id))
                .unwrap_or_else(|| self.display.rain_maximum.to_string());
            ui.horizontal_wrapped(|ui| {
                let label = ui.label(text(pt, "Maximum (mm)", "Máximo (mm)"));
                let edit = ui
                    .add(egui::TextEdit::singleline(&mut input).desired_width(72.))
                    .labelled_by(label.id);
                let parsed = rainfall_maximum(&input);
                if (edit.lost_focus()
                    || edit.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                    && let Some(value) = parsed
                    && value != self.display.rain_maximum
                {
                    self.display.rain_maximum = value;
                    self.recolor_rainfall();
                }
                ui.data_mut(|data| data.insert_temp(id, input));
                if parsed.is_none() {
                    ui.colored_label(
                        ui.visuals().error_fg_color,
                        text(
                            pt,
                            "Enter a number greater than 1",
                            "Informe um número maior que 1",
                        ),
                    );
                }
            });
            map::rain_legend_with(ui, pt, self.display.rain_maximum, self.rainfall_color());
        });
        if let Some(location) = &self.display.selection {
            theme::inset(ui.style()).show(ui, |ui| self.selected_feature(ui, location));
        }
    }

    fn selected_feature(&self, ui: &mut egui::Ui, location: &Location) {
        let Some(project) = &self.project else { return };
        let pt = self.preferences.portuguese;
        ui.strong(self.feature_label(location));
        if let Location::Station(id) = location
            && let Some(entry) = project
                .stations
                .iter()
                .find(|entry| entry.station.station_id == *id)
        {
            let station = &entry.station;
            ui.small(format!(
                "{} · {}",
                station.provider_code, station.station_code
            ));
            if let (Some(lat), Some(lon)) = (station.latitude, station.longitude) {
                ui.small(format!("{lat:.5}, {lon:.5}"));
            }
            if let Some(offset) = &entry.offset {
                ui.small(format!(
                    "MINI {} · {} {:+.2} m",
                    offset.mini_id,
                    text(pt, "Level offset", "Ajuste de nível"),
                    offset.offset_m
                ));
            }
        } else if let Location::Mini(id) = location
            && let Some(mini) = project.topology.iter().find(|mini| mini.mini_id == *id)
            && let Some(downstream) = mini.downstream
        {
            ui.small(format!(
                "{} MINI {downstream}",
                text(pt, "Downstream", "Jusante")
            ));
        }
        let mut views = self
            .rainfall
            .iter()
            .filter_map(|state| state.view.as_ref())
            .filter(|view| view.location.as_ref() == Some(location));
        let area = views
            .clone()
            .find_map(|view| view.summary.as_ref().map(|summary| summary.total_area_km2))
            .or_else(|| views.find_map(|view| view.area_km2));
        let value = area.map_or_else(|| "NA".into(), |area| format!("{area:.1} km²"));
        let response = ui.small(format!(
            "{} · {value}",
            text(pt, "Selected basin area", "Área da bacia selecionada")
        ));
        if area.is_none()
            && let Some(finding) = self
                .rainfall
                .iter()
                .filter_map(|state| state.view.as_ref())
                .filter(|view| view.location.as_ref() == Some(location))
                .find_map(|view| view.findings.first())
        {
            response.on_hover_text(finding);
        }
        ui.separator();
        ui.strong(text(
            pt,
            "Upstream basin rainfall over period",
            "Chuva na bacia a montante no período",
        ));
        self.rainfall_total(ui, RainSide::Left, pt);
        if self.display.map_mode.comparing() {
            ui.separator();
            self.rainfall_total(ui, RainSide::Right, pt);
        }
    }

    fn rainfall_color(&self) -> Color32 {
        self.display
            .layers
            .iter()
            .find(|layer| layer.kind == "rainfall")
            .map_or_else(
                || map::layer_swatch("rainfall"),
                map::LayerControl::display_color,
            )
    }

    fn recolor_rainfall(&mut self) {
        let hue = self.rainfall_color();
        for state in &mut self.rainfall {
            if let Some(image) = &mut state.image {
                image.recolor_with_color(self.display.rain_maximum, hue);
            }
        }
    }

    fn feature_actions(&mut self, ui: &mut egui::Ui, location: &Location) {
        let pt = self.preferences.portuguese;
        if ui
            .button(text(pt, "View time series", "Ver séries temporais"))
            .clicked()
        {
            self.view_time_series(location);
            ui.close_kind(egui::UiKind::Menu);
        }
        if let Some(mini) = self.upstream_outlet(location)
            && ui
                .button(text(pt, "Load upstream basin", "Carregar bacia a montante"))
                .clicked()
        {
            self.load_upstream(mini);
            ui.close_kind(egui::UiKind::Menu);
        }
    }

    fn data_page(&mut self, ui: &mut egui::Ui) {
        if let Some(project) = self.project.clone() {
            self.location_search(ui, &project);
            if let Some(location) = self.display.selection.clone() {
                ui.horizontal_wrapped(|ui| self.feature_actions(ui, &location));
            }
        }
        if let Some(report) = self.charts.report.as_ref() {
            let pt = self.preferences.portuguese;
            ui.label(self.charts.location_caption(report, pt));
            series_table(ui, report, pt);
        }
    }
    fn depart(&mut self, departure: Departure) {
        if self.pending.is_none() {
            self.pending = Some(departure);
            self.saving_departure = false;
            self.leaving_job = false;
        }
    }
    fn finish_departure(&mut self, ctx: &egui::Context) {
        let Some(departure) = self.pending.take() else {
            return;
        };
        self.saving_departure = false;
        self.leaving_job = false;
        match departure {
            Departure::Project(config, workspace) => {
                self.switching_project = true;
                self.submit(Job::Open(config, workspace));
            }
            Departure::Quit => {
                self.assistant.reset();
                self.allow_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Departure::Close => {
                self.assistant.reset();
                self.remember();
                for slot in 0..4 {
                    self.worker.invalidate(slot);
                }
                self.charts.clear();
                self.worker = Worker::new(ctx.clone());
                self.chart_worker = Worker::new(ctx.clone());
                self.chart_message.clear();
                self.chart_fields = ComparisonFields::default();
                self.project = None;
                self.run = None;
                self.run_selection_ready = false;
                self.active_run = Ok(None);
                self.comparison_runs.clear();
                self.inundation.select(None);
                self.rainfall = Default::default();
                self.rainfall_info = Ok(None);
                self.local.clear();
                self.cache.iter_mut().for_each(map::StaticCache::clear);
                self.clear_upstream();
                self.map_menu = None;
                self.tiles = None;
                self.loading.fill(false);
                self.runners = runners::Runners::default();
                self.workflows = workflows::Forms::default();
                self.calibration = tools::CalibrationTools::default();
            }
        }
    }
    fn departure_dialog(&mut self, ctx: &egui::Context) {
        let pt = self.preferences.portuguese;
        if self.pending.is_none() {
            return;
        }
        if self.loading.iter().any(|loading| *loading)
            || self.worker.is_busy()
            || self.chart_worker.is_busy()
        {
            egui::Modal::new(egui::Id::new("finishing-monitoring")).show(ctx, |ui| {
                ui.heading(text(
                    pt,
                    "Finishing monitoring work",
                    "Concluindo monitoramento",
                ));
                ui.spinner();
                ui.label(text(
                    pt,
                    "The current project stays available until its reads and exports finish.",
                    "O projeto permanece disponível até suas leituras e exportações terminarem.",
                ));
                if ui.button(text(pt, "Stay", "Permanecer")).clicked() {
                    self.pending = None;
                }
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
            return;
        }
        if self.runners.busy() {
            if self.saving_departure {
                egui::Modal::new(egui::Id::new("Saving work")).show(ctx, |ui| {
                    ui.spinner();
                    ui.label(text(pt, "Saving work…", "Salvando alterações…"));
                });
                return;
            }
            egui::Modal::new(egui::Id::new("Active job")).show(ctx, |ui| {
                ui.heading(text(pt, "Active job", "Tarefa ativa"));
                ui.label(text(
                    pt,
                    "Keep this project open until workers and cleanup finish.",
                    "Mantenha o projeto aberto até os trabalhadores e a limpeza terminarem.",
                ));
                if self.leaving_job {
                    ui.spinner();
                    ui.label(self.runners.localized_status(pt));
                } else {
                    ui.horizontal(|ui| {
                        if ui
                            .button(text(pt, "Cancel job and leave", "Cancelar tarefa e sair"))
                            .clicked()
                        {
                            if let Some(active) = &self.runners.active {
                                active.cancel();
                            }
                            self.leaving_job = true;
                            self.display.layout.form(Form::Jobs);
                        }
                        if ui.button(text(pt, "Stay", "Permanecer")).clicked() {
                            self.pending = None;
                        }
                    });
                }
            });
            return;
        }
        if self.runners.cleanup_failed {
            self.pending = None;
            return;
        }
        if self.runners.unsaved_inputs() {
            egui::Modal::new(egui::Id::new("leave-unsaved-inputs")).show(ctx, |ui| {
                ui.heading(text(pt, "Keep run inputs?", "Manter entradas da execução?"));
                ui.label(text(
                    pt,
                    "Save the inputs to regenerate this run later.",
                    "Salve as entradas para regenerar esta execução mais tarde.",
                ));
                ui.label(self.runners.localized_status(pt));
                ui.horizontal(|ui| {
                    if ui.button(text(pt, "Save", "Salvar")).clicked() {
                        self.saving_departure = true;
                        self.runners.save_inputs_dialog(ctx);
                    }
                    if ui.button(text(pt, "Discard", "Descartar")).clicked() {
                        self.saving_departure = true;
                        self.runners.discard_inputs(ctx);
                    }
                    if ui.button(text(pt, "Cancel", "Cancelar")).clicked() {
                        self.pending = None;
                        self.saving_departure = false;
                    }
                });
            });
            return;
        }
        if !self.runners.dirty() {
            self.finish_departure(ctx);
            return;
        }
        self.saving_departure = false;
        egui::Modal::new(egui::Id::new("Unsaved work")).show(ctx, |ui| {
            ui.heading(text(pt, "Unsaved work", "Alterações não salvas"));
            ui.label(text(
                pt,
                "Save workspace settings before continuing?",
                "Salvar configurações antes de continuar?",
            ));
            ui.label(self.runners.localized_status(pt));
            ui.horizontal(|ui| {
                if ui.button(text(pt, "Save", "Salvar")).clicked() {
                    self.saving_departure = true;
                    self.runners.start(runners::Action::SaveConfig, ctx);
                }
                if ui.button(text(pt, "Discard", "Descartar")).clicked() {
                    self.runners.discard();
                    self.finish_departure(ctx);
                }
                if ui.button(text(pt, "Cancel", "Cancelar")).clicked() {
                    self.pending = None;
                    self.saving_departure = false;
                }
            });
        });
    }
}
impl eframe::App for Monitoring {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        theme::begin_text_actions(&ctx);
        if self.project.is_some() && !self.loading[0] {
            if Instant::now() >= self.run_check
                || ctx.input(|input| input.events.contains(&egui::Event::WindowFocused(true)))
            {
                self.check_active_run(false);
            }
            ctx.request_repaint_after(self.run_check.saturating_duration_since(Instant::now()));
        }
        self.poll(&ctx);
        self.assistant.poll(
            &ctx,
            &mut self.preferences.assistant,
            self.preferences.portuguese,
        );
        if let Some(outcome) = self.runners.poll() {
            match outcome {
                runners::Outcome::Completed => {
                    self.check_active_run(false);
                    if self.check_rainfall_cache() {
                        self.request_view();
                    }
                }
                runners::Outcome::Workflow(output) => {
                    self.workflows.receive(output, &ctx);
                }
                _ => {}
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.depart(Departure::Quit);
        }
        ctx.set_theme(match self.preferences.appearance {
            1 => egui::ThemePreference::Light,
            2 => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::Light,
        });
        if self.loading.iter().any(|b| *b)
            || self.worker.is_busy()
            || self.chart_worker.is_busy()
            || self.runners.busy()
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)) {
            self.depart(Departure::Quit);
        }
        self.runners.shortcuts(&ctx, self.switching_project);
        self.runners
            .input_retention_dialog(&ctx, self.preferences.portuguese);
        for (key, tab) in [
            (egui::Key::Num2, SideTab::Charts),
            (egui::Key::Num3, SideTab::Data),
        ] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                self.display.layout.open(tab);
            }
        }
        if !self.switching_project
            && ui.input(|i| i.key_pressed(egui::Key::F5))
            && let Some(p) = &self.project
        {
            self.submit(Job::Open(p.config.clone(), p.workspace.clone()));
        }
        let toolbar_frame = theme::toolbar(ui.style());
        egui::Panel::top("ribbon")
            .frame(toolbar_frame)
            .show(ui, |ui| self.toolbar(ui));
        let footer = egui::Panel::bottom("status")
            .frame(toolbar_frame)
            .show(ui, |ui| {
                let height = ui.spacing().interact_size.y.max(16.) + 6.;
                ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), height),
                    egui::Sense::hover(),
                )
                .0
            })
            .inner;
        let width = ui.available_width();
        let single_min = (width * 0.30).clamp(190., 290.);
        let (min, max) = if self.display.layout.has_columns() {
            let max = (width * 0.60).max(single_min);
            ((single_min * 2.).min(max), max)
        } else {
            (single_min, (width * 0.41).max(single_min))
        };
        let response = egui::Panel::right(egui::Id::new(("right-pane", &self.display.config)))
            .frame(egui::Frame::side_top_panel(ui.style()).fill(ui.visuals().faint_bg_color))
            .resizable(true)
            .default_size(self.display.layout.width)
            .min_size(min)
            .max_size(max)
            .show(ui, |ui| self.right_pane(ui));
        self.display.layout.width = response.response.rect.width();
        let map_hover = egui::CentralPanel::default()
            .show(ui, |ui| self.map_page(ui))
            .inner;
        let mut footer_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(footer)
                .id_salt("status-contents"),
        );
        footer_ui.set_clip_rect(footer);
        let ui = &mut footer_ui;
        use walkers::Tiles;
        let pt = self.preferences.portuguese;
        let run = self.run_summary(pt);
        let attribution = self
            .tiles
            .as_ref()
            .filter(|_| self.project.is_some() && self.tile_source == self.display.basemap)
            .map(Tiles::attribution);
        ui.horizontal(|ui| {
            if let Some(credit) = attribution {
                footer_box(ui, |ui| {
                    ui.hyperlink_to(egui::RichText::new(credit.text).small(), credit.url);
                });
            }
            let available = (ui.available_width() - 12.).max(1.);
            let hover = map_hover.as_deref().unwrap_or("—");
            for (label, fraction) in [(&run[..], 0.43), (hover, 0.42), (&self.status[..], 0.15)] {
                footer_box(ui, |ui| {
                    ui.add_sized(
                        egui::vec2((available * fraction - 12.).max(1.), 16.),
                        egui::Label::new(egui::RichText::new(label).small()).truncate(),
                    )
                    .on_hover_text(label);
                });
            }
        });

        self.departure_dialog(&ctx);
        theme::text_actions(&ctx, self.preferences.portuguese);
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.remember();
        eframe::set_value(storage, STORAGE, &self.preferences);
    }
}

pub fn run() -> eframe::Result {
    eframe::run_native(
        "MGB Ops",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default()
                .with_app_id(APP_ID)
                .with_inner_size([1500., 950.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Monitoring::new(cc)))),
    )
}

#[cfg(test)]
mod interface_tests {
    use super::*;

    #[test]
    fn rainfall_failure_retains_image_identity_and_new_selection_gets_its_own_area() {
        let context = egui::Context::default();
        let start = data::timestamp("2026-01-01T00:00:00").unwrap();
        let make_rain = |generation: &str, value| {
            Arc::new(data::Rain {
                generation: generation.into(),
                scenario: "test".into(),
                window: mgb_ops::store::TimeInterval {
                    start,
                    end: start + chrono::Duration::hours(1),
                },
                grid: mgb_ops::store::RegularGridSpec {
                    requested_bbox: [-20., 10., 20., 50.],
                    effective_bbox: [-20., 10., 20., 50.],
                    resolution_degrees: 20.,
                    latitudes: vec![20., 40.],
                    longitudes: vec![-10., 10.],
                },
                values: vec![value; 4],
            })
        };
        let view = |rainfall, mini, finding| View {
            location: Some(Location::Mini(mini)),
            rainfall,
            summary: None,
            rainfall_finding: finding,
            summary_finding: None,
            area_km2: Some(mini as f64),
            findings: vec![],
        };
        let mut state = RainfallView::default();
        let initial = make_rain("first", Some(12.));
        state.replace(
            &context,
            view(Some(initial.clone()), 1, None),
            100.,
            Color32::BLUE,
        );
        state.replace(
            &context,
            view(None, 2, Some("read failed".into())),
            100.,
            Color32::BLUE,
        );
        assert!(Arc::ptr_eq(&state.image.as_ref().unwrap().rain, &initial));
        assert_eq!(
            state.view.as_ref().unwrap().location,
            Some(Location::Mini(2))
        );
        assert_eq!(state.view.as_ref().unwrap().area_km2, Some(2.));
        assert_eq!(state.finding.as_deref(), Some("read failed"));
        // An unavailable query is not an authoritative empty raster replacement.
        state.replace(&context, view(None, 2, None), 100., Color32::BLUE);
        assert!(Arc::ptr_eq(&state.image.as_ref().unwrap().rain, &initial));
        // A successfully loaded all-NA raster is an authoritative transparent result.
        let replacement = make_rain("second", None);
        state.replace(
            &context,
            view(Some(replacement.clone()), 2, None),
            100.,
            Color32::BLUE,
        );
        assert!(Arc::ptr_eq(
            &state.image.as_ref().unwrap().rain,
            &replacement
        ));
        assert!(state.finding.is_none());
    }
    #[test]
    fn data_table_builds_widgets_only_for_visible_rows() {
        use mgb_ops::app::{ChartSeries, SeriesPoint, SeriesReport};
        use mgb_ops::store::TimeInterval;

        let start = data::timestamp("2026-01-01T00:00:00").unwrap();
        let points = (0..10_000)
            .map(|index| SeriesPoint {
                timestamp: start + chrono::Duration::hours(index),
                value: Some(index as f64),
                time_segment: None,
            })
            .collect();
        let report = SeriesReport {
            location: Location::Mini(1),
            time_zone: "UTC".into(),
            window: TimeInterval {
                start,
                end: start + chrono::Duration::hours(10_000),
            },
            series: vec![ChartSeries {
                selection_index: 0,
                series_kind: "fixture".into(),
                run_id: None,
                scenario_id: None,
                forecast_reference_time: None,
                variable: "rain".into(),
                unit: "mm".into(),
                points,
            }],
            findings: Vec::new(),
        };
        let ctx = egui::Context::default();
        let frame = |events, time| {
            let mut frame = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400., 220.),
                    )),
                    events,
                    time: Some(time),
                    ..Default::default()
                },
                |ui| series_table(ui, &report, false),
            );
            frame.textures_delta.clear();
            frame
        };
        let has_first_row = |frame: &egui::FullOutput| {
            frame.shapes.iter().any(|item| {
                matches!(&item.shape, egui::Shape::Text(shape) if shape.galley.text() == "2026-01-01 00:00:00")
            })
        };
        let initial = frame(vec![egui::Event::PointerMoved(egui::pos2(120., 130.))], 0.);
        assert!(has_first_row(&initial));
        assert!(initial.shapes.len() < 150);
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0., -1500.),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        };
        frame(vec![wheel], 0.04);
        let scrolled = frame(Vec::new(), 0.08);
        let moved = !has_first_row(&scrolled);
        assert!(moved);
        assert!(scrolled.shapes.len() < 150);
    }

    #[test]
    fn rainfall_scale_accepts_finite_maxima_and_defaults_for_saved_projects() {
        assert_eq!(rainfall_maximum("27,5"), Some(27.5));
        assert_eq!(rainfall_maximum(" 50 "), Some(50.));
        for invalid in ["", "1", "0", "NaN", "inf"] {
            assert_eq!(rainfall_maximum(invalid), None);
        }
        let display: Display = serde_json::from_str("{}").unwrap();
        assert_eq!(display.rain_maximum, 100.);
    }

    #[test]
    fn saved_layer_settings_gain_inundation_without_changing_existing_order() {
        let mut display = Display::default();
        display.layers.remove(2);
        display.layers.reverse();
        let previous: Vec<_> = display
            .layers
            .iter()
            .map(|layer| layer.kind.clone())
            .collect();
        display.restore_layers();
        display.restore_layers();
        let added: Vec<_> = display
            .layers
            .iter()
            .filter(|layer| layer.kind == inundation::KIND)
            .collect();
        assert_eq!(added.len(), 1);
        let hidden = !added[0].visible;
        assert!(hidden);
        let remaining: Vec<_> = display
            .layers
            .into_iter()
            .filter(|layer| layer.kind != inundation::KIND)
            .map(|layer| layer.kind)
            .collect();
        assert_eq!(remaining, previous);
    }

    #[test]
    fn chart_targets_clear_unrelated_fields_and_use_unique_links() {
        use mgb_ops::store::{Station, StationLocation, StationOffset};
        let entry = |id: &str, mini_id: Option<i64>| StationLocation {
            station: Station {
                station_id: id.into(),
                provider_code: "provider".into(),
                station_code: id.into(),
                name: id.into(),
                latitude: None,
                longitude: None,
            },
            offset: mini_id.map(|mini_id| StationOffset {
                station_id: id.into(),
                mini_id,
                offset_m: 0.,
            }),
        };
        let stations = [
            entry("001", Some(1)),
            entry("b", Some(2)),
            entry("c", Some(2)),
            entry("d", None),
        ];
        let mut fields = ComparisonFields::default();
        fields.select(&Location::Mini(1), &stations);
        assert_eq!((&fields.mini[..], &fields.station[..]), ("1", "001"));
        fields.select(&Location::Mini(2), &stations);
        assert_eq!((&fields.mini[..], &fields.station[..]), ("2", ""));
        fields.select(&Location::Mini(3), &stations);
        assert_eq!((&fields.mini[..], &fields.station[..]), ("3", ""));
        fields.select(&Location::Station("d".into()), &stations);
        assert_eq!((&fields.mini[..], &fields.station[..]), ("", "d"));
        fields.select(&Location::Station("b".into()), &stations);
        assert_eq!((&fields.mini[..], &fields.station[..]), ("2", "b"));
    }

    #[test]
    fn preferences_preserve_storage_identity() {
        assert_eq!(APP_ID, "MGB Ops · Monitoring");
        let legacy: Display = serde_json::from_str(
            r#"{"config":"region/config.toml","run":"region/data/runs/run-1",
                 "layout":{"dock":{"Tabs":{"pages":["Map","Data"],"active":1}},
                           "left":false,"tabs":["Files",{"Form":"Prepare"}]},
                 "operation_zone":"America/Sao_Paulo","basemap":2}"#,
        )
        .unwrap();
        assert_eq!(legacy.config, PathBuf::from("region/config.toml"));
        assert!(serde_json::to_value(&legacy).unwrap().get("run").is_none());
        assert_eq!(legacy.basemap, 2);
        assert_eq!(legacy.layout.focused_tab(), Some(SideTab::Map));
        assert!(legacy.layout.is_open(SideTab::Data));
    }
    #[test]
    fn rainfall_preferences_copy_applied_left_settings_only_on_first_comparison() {
        let legacy = r#"{"rainfall_scenario":"left","hours":-18,"explicit":true,
                        "start":"2026-01-01T02:00:00","end":"2026-01-01T05:00:00",
                        "upstream":false}"#;
        let mut display: Display = serde_json::from_str(legacy).unwrap();
        assert_eq!(display.map_mode, map::Mode::Single);
        assert_eq!(display.swipe, 0.5);
        assert!(display.right_rainfall.is_none());
        display.set_map_mode(map::Mode::Double);
        let mut right = display.rainfall_settings(RainSide::Right);
        assert_eq!(right.scenario, "left");
        assert!(right.window.explicit);
        assert_eq!(right.window.start, "2026-01-01T02:00:00");
        assert_eq!(right.window.end, "2026-01-01T05:00:00");
        right.scenario = "right".into();
        right.window.explicit = false;
        right.window.hours = 12;
        display.set_rainfall_settings(RainSide::Right, right);
        display.set_map_mode(map::Mode::Single);
        display.set_map_mode(map::Mode::Swipe);
        display.swipe = 0.72;
        let saved = serde_json::to_string(&display).unwrap();
        let restored: Display = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.map_mode, map::Mode::Swipe);
        assert_eq!(restored.swipe, 0.72);
        assert_eq!(restored.rainfall_scenario, "left");
        assert_eq!(restored.hours, -18);
        let right = restored.rainfall_settings(RainSide::Right);
        assert_eq!(right.scenario, "right");
        let right_period_retained =
            matches!(right.window.period(), Ok(RainfallPeriod::SignedHours(12)));
        assert!(right_period_retained);
    }

    #[test]
    fn icon_buttons_publish_accessible_names() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            icon_button(ui, "×", "Close Map");
        });
        output.textures_delta.clear();
        let update = output.platform_output.accesskit_update.unwrap();
        assert!(
            update
                .nodes
                .iter()
                .any(|(_, node)| node.label() == Some("Close Map"))
        );
    }
}
