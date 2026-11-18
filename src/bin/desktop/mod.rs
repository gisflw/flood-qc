mod charts;
mod data;
mod export;
mod layout;
mod map;
mod runners;
use self::layout::{Form, SideTab};

use self::data::{
    CHART_SLOT, ChartRequest, Job, Loaded, Project, Run, UPSTREAM_SLOT, View, ViewRequest, Worker,
};
use eframe::egui::{self, Color32};
use mgb_ops::analysis::HourlyWindow;
use mgb_ops::app::{Location, SeriesQuery, SeriesSelection};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use walkers::{HttpTiles, Map, MapMemory, lon_lat};

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

fn section(ui: &mut egui::Ui, title: String, content: impl FnOnce(&mut egui::Ui)) {
    ui.separator();
    ui.strong(title);
    content(ui);
}

fn ribbon_group(ui: &mut egui::Ui, title: String, actions: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.horizontal(actions);
        ui.small(title);
    });
    ui.separator();
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Ribbon {
    Project,
    Draft,
    View,
    Processing,
    Settings,
}

impl Ribbon {
    fn label(self, pt: bool) -> String {
        match self {
            Self::Project => text(pt, "Project", "Projeto"),
            Self::Draft => text(pt, "Draft", "Rascunho"),
            Self::View => text(pt, "View", "Exibir"),
            Self::Processing => text(pt, "Processing", "Processamento"),
            Self::Settings => text(pt, "Settings", "Configurações"),
        }
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
    labels: bool,
    local: Vec<map::LocalDisplay>,
    selection: Option<Location>,
    comparison: Option<ComparisonFields>,
    run: Option<PathBuf>,
    map_scenario: String,
    overlays: Vec<String>,
    observations: bool,
    grid: Option<PathBuf>,
    reference: String,
    zone: String,
    hours: i32,
    explicit: bool,
    start: String,
    end: String,
    chart_start: String,
    chart_end: String,
    upstream: bool,
    #[serde(rename = "right_pane")]
    layout: layout::Layout,
    draft: Option<PathBuf>,
    operation_zone: Option<String>,
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
            labels: true,
            local: vec![],
            selection: None,
            comparison: None,
            run: None,
            map_scenario: String::new(),
            overlays: vec![],
            observations: true,
            grid: None,
            reference: String::new(),
            zone: "America/Sao_Paulo".into(),
            hours: -24,
            explicit: false,
            start: String::new(),
            end: String::new(),
            chart_start: String::new(),
            chart_end: String::new(),
            upstream: true,
            layout: layout::Layout::default(),
            draft: None,
            operation_zone: None,
        }
    }
}
impl Display {
    fn selected_operation_zone<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.operation_zone.as_deref().unwrap_or(fallback)
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    portuguese: bool,
    appearance: usize,
    recent: Vec<Display>,
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
    New,
    Draft(PathBuf, String, String),
}

struct Monitoring {
    preferences: Preferences,
    display: Display,
    project: Option<Arc<Project>>,
    run: Option<Arc<Run>>,
    view: Option<View>,
    rainfall: Option<map::RainImage>,
    worker: Worker,
    chart_worker: Worker,
    loading: [bool; CHART_SLOT + 1],
    chart_fields: ComparisonFields,
    chart_message: String,
    chart_retained: bool,
    status: String,
    search: String,
    charts: charts::Charts,
    tiles: Option<HttpTiles>,
    tile_source: usize,
    cache: map::StaticCache,
    local: Vec<map::LocalLayer>,
    runners: runners::Runners,
    pending: Option<Departure>,
    saving_departure: bool,
    leaving_job: bool,
    allow_quit: bool,
    switching_project: bool,
    open_candidate: Option<(PathBuf, String, String)>,
    ribbon: Ribbon,
    map_menu: Option<Location>,
    upstream_basin: Option<(i64, Vec<i64>)>,
    chart_exports: bool,
    export_scale: u32,
}
impl Monitoring {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let preferences: Preferences = cc
            .storage
            .and_then(|s| eframe::get_value(s, STORAGE))
            .unwrap_or_default();
        let display = preferences.recent.first().cloned().unwrap_or_default();
        let mut app = Self {
            preferences,
            display,
            project: None,
            run: None,
            view: None,
            rainfall: None,
            worker: Worker::new(cc.egui_ctx.clone()),
            chart_worker: Worker::new(cc.egui_ctx.clone()),
            loading: [false; CHART_SLOT + 1],
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
            cache: map::StaticCache::default(),
            local: vec![],
            runners: runners::Runners::default(),
            pending: None,
            saving_departure: false,
            leaving_job: false,
            allow_quit: false,
            switching_project: false,
            open_candidate: None,
            ribbon: Ribbon::Project,
            map_menu: None,
            upstream_basin: None,
            chart_exports: false,
            export_scale: 1,
        };
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
        }
        if slot == 1 {
            self.invalidate_chart();
            self.worker.invalidate(2);
            self.loading[2] = false;
        }
        self.loading[slot] = true;
        self.worker.submit(job);
        self.status = text(
            self.preferences.portuguese,
            "Loading; previous view retained",
            "Carregando; visualização anterior mantida",
        );
    }
    fn remember(&mut self) {
        if self.runners.initialized
            && let Some(zone) = self
                .runners
                .draft
                .as_ref()
                .and_then(|d| d.document.operation.time_zone.clone())
                .filter(|z| z.parse::<chrono_tz::Tz>().is_ok())
        {
            self.display.operation_zone = Some(zone);
        }
        if self.display.config.as_os_str().is_empty() {
            return;
        }
        self.preferences
            .recent
            .retain(|p| p.config != self.display.config);
        self.preferences.recent.insert(0, self.display.clone());
        self.preferences.recent.truncate(10);
    }
    fn open_path(&mut self, config: PathBuf) {
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
    fn resolve_window(&mut self) -> Result<HourlyWindow, String> {
        if self.display.explicit {
            let window = HourlyWindow::Explicit {
                start: data::timestamp(&self.display.start)?,
                end: data::timestamp(&self.display.end)?,
            };
            mgb_ops::analysis::resolve_hourly_window(&self.display.zone, &window)?;
            Ok(window)
        } else {
            let (_, interval) = mgb_ops::analysis::signed_rainfall_window(
                &self.display.zone,
                data::timestamp(&self.display.reference)?,
                self.display.hours,
            )?;
            self.display.start = interval.start.format("%Y-%m-%dT%H:%M:%S").to_string();
            self.display.end = interval.end.format("%Y-%m-%dT%H:%M:%S").to_string();
            Ok(HourlyWindow::Explicit {
                start: interval.start,
                end: interval.end,
            })
        }
    }
    fn run_rainfall(&mut self) {
        if let Some(run) = &self.run {
            self.display.grid = Some(if !self.display.explicit && self.display.hours < 0 {
                run.root.join("checkpoints/observed-grid.nc")
            } else {
                run.root
                    .join("checkpoints/forecast")
                    .join(&self.display.map_scenario)
                    .join("working.nc")
            });
        }
    }
    fn request_view(&mut self) {
        self.worker.invalidate(2);
        self.loading[2] = false;
        if self.display.grid.is_none() {
            self.rainfall = None;
        }
        let Some(project) = self.project.clone() else {
            return;
        };
        match self.resolve_window() {
            Ok(window) => self.submit(Job::View(ViewRequest {
                project,
                run: self.run.clone(),
                location: self.display.selection.clone(),
                map_scenario: self.display.map_scenario.clone(),
                grid: self.display.grid.clone(),
                zone: self.display.zone.clone(),
                window,
                upstream: self.display.upstream,
            })),
            Err(error) => self.status = error,
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
        self.clear_upstream();
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
            .filter(|_| matches!(location, Location::Mini(_)));
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
            match reply.result {
                Err(error) if reply.slot == CHART_SLOT => self.chart_message = error,
                Err(error) if reply.slot == UPSTREAM_SLOT => self.status = error,
                Err(error) => {
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
                    if !same {
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
                        self.view = None;
                        self.rainfall = None;
                        self.local.clear();
                        self.worker.invalidate(1);
                        self.worker.invalidate(2);
                        self.worker.invalidate(3);
                        self.loading[1..4].fill(false);
                    }
                    if self
                        .project
                        .as_ref()
                        .is_none_or(|old| !Arc::ptr_eq(&old.fills, &project.fills))
                    {
                        self.cache.clear();
                        self.clear_upstream();
                        self.map_menu = None;
                    }
                    if !same {
                        self.runners = runners::Runners::default();
                        self.runners.start(
                            runners::Action::Initialize {
                                workspace: project.workspace.clone(),
                                config: project.config.clone(),
                                zone: self
                                    .display
                                    .selected_operation_zone(&project.time_zone)
                                    .into(),
                                remembered: self.display.draft.clone(),
                            },
                            context,
                        );
                    }
                    self.project = Some(project.clone());
                    if !self.display.local.is_empty() {
                        self.submit(Job::Local(
                            self.display.local.iter().map(|l| l.path.clone()).collect(),
                        ));
                    }
                    if self.display.reference.is_empty() {
                        use chrono::Timelike;
                        self.display.zone = project.time_zone.clone();
                        if let Ok(zone) = project.time_zone.parse::<chrono_tz::Tz>() {
                            let now = chrono::Utc::now()
                                .with_timezone(&zone)
                                .naive_local()
                                .with_minute(0)
                                .unwrap()
                                .with_second(0)
                                .unwrap()
                                .with_nanosecond(0)
                                .unwrap();
                            self.display.reference = now.format("%Y-%m-%dT%H:%M:%S").to_string();
                        }
                    }
                    if self.display.run.is_none()
                        && (self.display.chart_start.is_empty()
                            || self.display.chart_end.is_empty())
                    {
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
                    if self.display.zone.is_empty() {
                        self.display.zone = project.time_zone.clone();
                    }
                    if let Some(location) = &self.display.selection {
                        let valid = match location {
                            Location::Station(id) => {
                                project.stations.iter().any(|s| s.station.station_id == *id)
                            }
                            Location::Mini(id) => project.topology.iter().any(|m| m.mini_id == *id),
                        };
                        if !valid {
                            self.view = None;
                            self.display.selection = None;
                            self.status = text(
                                self.preferences.portuguese,
                                "Remembered selection is unavailable",
                                "Seleção lembrada indisponível",
                            );
                        }
                    }
                    self.status = format!(
                        "{} · {} {} · {} MINIs · {} {} · {} {}",
                        project.workspace.display(),
                        project.stations.len(),
                        text(self.preferences.portuguese, "stations", "estações"),
                        project.topology.len(),
                        project.runs.len(),
                        text(self.preferences.portuguese, "runs", "execuções"),
                        project.findings.len(),
                        text(self.preferences.portuguese, "findings", "avisos")
                    );
                    if let Some(root) = self.display.run.clone() {
                        self.submit(Job::Run(root));
                    } else {
                        if !self.display.reference.is_empty() {
                            self.request_view();
                        }
                        self.request_chart();
                    }
                }
                Ok(Loaded::Run(run)) => {
                    let changed = self.run.as_ref().is_some_and(|old| old.root != run.root);
                    if changed {
                        self.display.overlays.clear();
                        self.display.map_scenario.clear();
                        self.display.grid = None;
                        self.rainfall = None;
                    }
                    self.display.zone = run.time_zone.clone();
                    self.display.reference = run.reference.clone();
                    let new_window = self.display.run.as_ref() != Some(&run.root);
                    if self.display.chart_start.is_empty() || new_window {
                        self.display.chart_start = run.start.clone();
                    }
                    if self.display.chart_end.is_empty() || new_window {
                        self.display.chart_end = run.end.clone();
                    }
                    if self.display.map_scenario.is_empty() {
                        self.display.map_scenario = run.scenarios[0].clone();
                    }
                    if self.display.overlays.is_empty() {
                        self.display.overlays = run.scenarios.clone();
                    }
                    if !run.scenarios.contains(&self.display.map_scenario) {
                        self.status = text(
                            self.preferences.portuguese,
                            "Remembered scenario is unavailable; select an available scenario",
                            "Cenário lembrado indisponível; selecione um cenário disponível",
                        );
                    }
                    self.display.run = Some(run.root.clone());
                    self.run = Some(run);
                    if self.display.grid.is_none() {
                        self.run_rainfall();
                    }
                    self.request_view();
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
                    let mut view = *view;
                    if view.rainfall.is_none()
                        && self.display.grid.is_some()
                        && let Some(retained) = &self.rainfall
                    {
                        view.findings.push(format!(
                            "{}: {}",
                            text(
                                self.preferences.portuguese,
                                "Retained rainfall",
                                "Chuva anterior mantida"
                            ),
                            retained.rain.source.display()
                        ));
                    }
                    if let Some(rain) = view.rainfall.take()
                        && self
                            .rainfall
                            .as_ref()
                            .is_none_or(|image| !Arc::ptr_eq(&image.rain, &rain))
                    {
                        self.rainfall = Some(map::RainImage::new(context, rain));
                    }
                    self.status = if view.findings.is_empty() {
                        text(self.preferences.portuguese, "Loaded", "Carregado")
                    } else {
                        view.findings.join("; ")
                    };
                    self.view = Some(view);
                }
                Ok(Loaded::Upstream { mini, ids }) => {
                    self.upstream_basin = Some((mini, ids));
                    self.status = text(
                        self.preferences.portuguese,
                        "Upstream basin loaded",
                        "Bacia a montante carregada",
                    );
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
        ui.horizontal_wrapped(|ui| {
            ui.strong("MGB Ops");
            ui.separator();
            for tab in [
                Ribbon::Project,
                Ribbon::Draft,
                Ribbon::View,
                Ribbon::Processing,
                Ribbon::Settings,
            ] {
                ui.selectable_value(&mut self.ribbon, tab, tab.label(pt));
            }
        });
        ui.separator();
        egui::ScrollArea::horizontal()
            .id_salt("ribbon-actions")
            .show(ui, |ui| {
                ui.horizontal(|ui| match self.ribbon {
                    Ribbon::Project => {
                        ribbon_group(ui, text(pt, "Workspace", "Área de trabalho"), |ui| {
                            if ui
                                .button(text(pt, "↗ Open TOML…", "↗ Abrir TOML…"))
                                .clicked()
                                && let Some(path) = rfd::FileDialog::new()
                                    .add_filter("TOML", &["toml"])
                                    .pick_file()
                            {
                                self.open_path(path);
                            }
                            if !self.preferences.recent.is_empty() {
                                ui.menu_button(text(pt, "Recent", "Recentes"), |ui| {
                                    for recent in self.preferences.recent.clone() {
                                        if ui.button(recent.config.display().to_string()).clicked()
                                        {
                                            self.depart(Departure::Project(
                                                recent.config,
                                                recent.workspace,
                                            ));
                                            ui.close();
                                        }
                                    }
                                });
                            }
                            if ui
                                .add_enabled(
                                    self.project.is_some(),
                                    egui::Button::new(text(pt, "Close project", "Fechar projeto")),
                                )
                                .clicked()
                            {
                                self.depart(Departure::Close);
                            }
                        });
                        ribbon_group(ui, text(pt, "Monitoring", "Monitoramento"), |ui| {
                            if ui
                                .add_enabled(
                                    !self.switching_project && self.project.is_some(),
                                    egui::Button::new(text(
                                        pt,
                                        "↻ Refresh (F5)",
                                        "↻ Atualizar (F5)",
                                    )),
                                )
                                .clicked()
                                && let Some(project) = &self.project
                            {
                                self.submit(Job::Open(
                                    project.config.clone(),
                                    project.workspace.clone(),
                                ));
                            }
                        });
                    }
                    Ribbon::Draft => {
                        let enabled = self.runners.initialized
                            && !self.runners.busy()
                            && !self.switching_project;
                        ribbon_group(ui, text(pt, "Draft files", "Arquivos de rascunho"), |ui| {
                            ui.add_enabled_ui(enabled, |ui| {
                                if ui
                                    .button(text(pt, "+ New run", "+ Nova execução"))
                                    .clicked()
                                {
                                    self.depart(Departure::New);
                                }
                                if ui.button(text(pt, "↗ Open…", "↗ Abrir…")).clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .add_filter("Draft JSON", &["json"])
                                        .pick_file()
                                {
                                    let zone = self
                                        .runners
                                        .draft
                                        .as_ref()
                                        .and_then(|d| d.document.operation.time_zone.clone())
                                        .unwrap_or_else(|| self.display.zone.clone());
                                    match runners::reference_now(&zone) {
                                        Ok(reference) => {
                                            self.open_candidate = Some((path, reference, zone))
                                        }
                                        Err(e) => self.runners.status = e,
                                    };
                                }
                            });
                        });
                        ribbon_group(ui, text(pt, "Save draft", "Salvar rascunho"), |ui| {
                            ui.add_enabled_ui(enabled, |ui| {
                                if ui.button(text(pt, "▣ Save", "▣ Salvar")).clicked() {
                                    match self.runners.save_path() {
                                        Ok(path) => self.runners.save(path, ui.ctx()),
                                        Err(e) => self.runners.status = e,
                                    };
                                }
                                if ui
                                    .button(text(pt, "▣ Save As…", "▣ Salvar como…"))
                                    .clicked()
                                {
                                    match self.runners.save_path() {
                                        Ok(path) => {
                                            if let Some(path) = rfd::FileDialog::new()
                                                .set_directory(path.parent().unwrap_or(&path))
                                                .set_file_name(
                                                    path.file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy(),
                                                )
                                                .add_filter("Draft JSON", &["json"])
                                                .save_file()
                                            {
                                                self.runners.save(path, ui.ctx());
                                            }
                                        }
                                        Err(e) => self.runners.status = e,
                                    };
                                }
                            });
                        });
                    }
                    Ribbon::View => {
                        ribbon_group(ui, text(pt, "Pages", "Páginas"), |ui| {
                            for tab in [
                                SideTab::Charts,
                                SideTab::Data,
                                SideTab::Rainfall,
                                SideTab::Files,
                            ] {
                                if ui.button(tab.label(pt)).clicked() {
                                    self.display.layout.open(tab);
                                }
                            }
                        });
                        ribbon_group(ui, text(pt, "Layout", "Layout"), |ui| {
                            if ui
                                .button(text(pt, "Reset layout", "Restaurar layout"))
                                .clicked()
                            {
                                self.display.layout = layout::Layout::default();
                                let id = egui::Id::new(("right-pane", &self.display.config));
                                ui.ctx().data_mut(|d| {
                                    d.remove::<egui::containers::panel::PanelState>(id)
                                });
                            }
                        });
                    }
                    Ribbon::Processing => {
                        ribbon_group(ui, text(pt, "Operation", "Operação"), |ui| {
                            for form in [Form::Prepare, Form::Execute, Form::RunAll] {
                                if ui.button(form.label(pt)).clicked() {
                                    self.display.layout.form(form);
                                }
                            }
                        });
                        ribbon_group(ui, text(pt, "Activity", "Atividade"), |ui| {
                            if ui.button(Form::Jobs.label(pt)).clicked() {
                                self.display.layout.form(Form::Jobs);
                            }
                        });
                    }
                    Ribbon::Settings => {
                        ribbon_group(ui, text(pt, "Configuration", "Configuração"), |ui| {
                            for tab in [SideTab::Interface, SideTab::Form(Form::Workspace)] {
                                if ui.button(tab.label(pt)).clicked() {
                                    self.display.layout.open(tab);
                                }
                            }
                        });
                    }
                });
            });
    }
    fn interface_settings(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.heading(SideTab::Interface.label(pt));
        let old = self.preferences.portuguese;
        egui::ComboBox::from_label(text(pt, "Language", "Idioma"))
            .selected_text(if pt { "Português (Brasil)" } else { "English" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.preferences.portuguese, false, "English");
                ui.selectable_value(&mut self.preferences.portuguese, true, "Português (Brasil)");
            });
        egui::ComboBox::from_label(text(pt, "Appearance", "Aparência"))
            .selected_text(
                [
                    text(pt, "System", "Sistema"),
                    text(pt, "Light", "Clara"),
                    text(pt, "Dark", "Escura"),
                ][self.preferences.appearance.min(2)]
                .clone(),
            )
            .show_ui(ui, |ui| {
                for (i, label) in [
                    text(pt, "System", "Sistema"),
                    text(pt, "Light", "Clara"),
                    text(pt, "Dark", "Escura"),
                ]
                .into_iter()
                .enumerate()
                {
                    ui.selectable_value(&mut self.preferences.appearance, i, label);
                }
            });
        if old != self.preferences.portuguese
            && let Some(report) = self.charts.report.clone()
        {
            let limits: Vec<_> = self.charts.plots.iter().map(|p| p.plot().limits).collect();
            let references = self.charts.references.clone();
            let right = self.charts.plots[2].plot().y2;
            if let Err(error) = self
                .charts
                .load(report, &references, self.preferences.portuguese)
            {
                self.status = error;
            }
            for (plot, (a, b, c, d)) in self.charts.plots.iter_mut().zip(limits) {
                plot.set_graph_x_limits(a, b);
                plot.set_graph_y_limits(c, d, rsplot::YAxis::Left);
            }
            if let Some((min, max)) = right {
                self.charts.plots[2].set_graph_y_limits(min, max, rsplot::YAxis::Right);
            }
        }
    }

    fn right_pane(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.horizontal_wrapped(|ui| {
            for tab in self.display.layout.tabs.clone() {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.display.layout.active, tab, tab.label(pt));
                    if matches!(tab, SideTab::Interface | SideTab::Form(_))
                        && icon_button(
                            ui,
                            "×",
                            &format!("{} {}", text(pt, "Close", "Fechar"), tab.label(pt)),
                        )
                        .clicked()
                    {
                        self.display.layout.close(tab);
                    }
                });
            }
        });
        ui.separator();
        let tab = self.display.layout.active;
        ui.push_id(("right-pane-content", tab.label(false)), |ui| match tab {
            SideTab::Charts => {
                self.chart_controls(ui);
                if self.charts.report.is_some() {
                    let height = ((ui.available_height() - 65.) / 3.).max(110.);
                    egui::ScrollArea::vertical()
                        .id_salt("chart-plots")
                        .show(ui, |ui| self.charts.show(ui, pt, height));
                }
            }
            SideTab::Data => self.data_page(ui),
            SideTab::Rainfall => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.project.is_some()
                        && ui
                            .button(text(pt, "Apply map window", "Aplicar período do mapa"))
                            .clicked()
                    {
                        self.request_view();
                    }
                    self.rainfall_controls(ui);
                });
            }
            SideTab::Files => {
                egui::ScrollArea::vertical().show(ui, |ui| self.files(ui));
            }
            SideTab::Interface => {
                egui::ScrollArea::vertical().show(ui, |ui| self.interface_settings(ui));
            }
            SideTab::Form(form) => {
                ui.add_enabled_ui(!self.switching_project, |ui| {
                    self.runners.show(ui, form, pt)
                });
            }
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
                Color32::YELLOW,
                format!(
                    "{}: {}",
                    text(pt, "Retained source", "Fonte anterior mantida"),
                    path.display()
                ),
            );
        }
        let mut selected = self.display.run.clone();
        egui::ComboBox::from_label(text(pt, "Retained run", "Execução retida"))
            .width(125.)
            .truncate()
            .selected_text(
                self.display
                    .run
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "—".into()),
            )
            .show_ui(ui, |ui| {
                for root in &project.runs {
                    ui.selectable_value(
                        &mut selected,
                        Some(root.clone()),
                        root.file_name().unwrap_or_default().to_string_lossy(),
                    );
                }
            });
        if selected != self.display.run
            && let Some(root) = selected
        {
            self.submit(Job::Run(root));
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
                    ui.small(format!(
                        "{} · {}",
                        status_label(&run.manifest.state, pt),
                        run.manifest.created_at
                    ));
                    ui.small(format!(
                        "{}: {}",
                        text(pt, "Reference", "Referência"),
                        run.reference
                    ));
                    ui.label(run.root.display().to_string());
                    for finding in &run.findings {
                        ui.colored_label(Color32::LIGHT_RED, finding);
                    }
                    if let Some(error) = &run.manifest.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                    for scenario in &run.manifest.scenarios {
                        ui.label(format!(
                            "{}: {}",
                            scenario.id,
                            status_label(&scenario.state, pt)
                        ));
                        if let Some(error) = &scenario.error {
                            ui.colored_label(Color32::LIGHT_RED, &error.message);
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
                    ui.colored_label(Color32::LIGHT_RED, finding);
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

    fn left_panel(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.heading(text(pt, "Run", "Execução"));
        ui.strong("reference_time");
        ui.label(self.run.as_ref().map_or_else(
            || "—".into(),
            |run| {
                let document = &run.manifest.recipe["document"];
                let reference = document["reference_time"]
                    .as_str()
                    .filter(|value| !value.is_empty())
                    .unwrap_or(&run.reference);
                let zone = document["time_zone"].as_str().unwrap_or(&run.time_zone);
                if reference.is_empty() {
                    "—".into()
                } else {
                    format!("{reference} · {zone}")
                }
            },
        ));
        let horizons = self.run.as_ref().and_then(|run| run.day_horizons);
        for (label, value) in [
            (
                "observed_horizon_days",
                horizons.map(|(observed, _)| observed),
            ),
            (
                "forecast_horizon_days",
                horizons.map(|(_, forecast)| forecast),
            ),
        ] {
            ui.horizontal_wrapped(|ui| {
                ui.strong(label);
                ui.label(value.map_or_else(|| "—".into(), |days| days.to_string()));
            });
        }
        ui.strong(text(pt, "Available scenarios", "Cenários disponíveis"));
        egui::ScrollArea::vertical()
            .id_salt("run-scenarios")
            .max_height(96.)
            .show(ui, |ui| {
                ui.label(
                    self.run
                        .as_ref()
                        .map_or_else(|| "—".into(), |run| run.scenarios.join(", ")),
                );
            });
        ui.separator();
        ui.heading(text(pt, "Layers", "Camadas"));
        let selection = self.project.as_ref().and(self.display.selection.as_ref());
        ui.label(selection.map_or_else(
            || text(pt, "No feature selected", "Nenhuma feição selecionada"),
            |location| self.feature_label(location),
        ));
        egui::ScrollArea::vertical().show(ui, |ui| self.layer_controls(ui));
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

    fn layer_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        egui::ComboBox::from_label(text(pt, "Basemap", "Mapa base"))
            .selected_text(
                [
                    if pt { "Nenhum" } else { "None" },
                    "OpenStreetMap",
                    "OpenTopoMap",
                ][self.display.basemap.min(2)],
            )
            .show_ui(ui, |ui| {
                for (i, label) in [
                    if pt { "Nenhum" } else { "None" },
                    "OpenStreetMap",
                    "OpenTopoMap",
                ]
                .iter()
                .enumerate()
                {
                    ui.selectable_value(&mut self.display.basemap, i, *label);
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
        ui.small(text(
            pt,
            "Green: known model link · Gray: unknown link",
            "Verde: vínculo conhecido · Cinza: vínculo desconhecido",
        ));
        let mut clear = false;
        let mut reorder = None;
        for (i, layer) in self.display.layers.iter_mut().enumerate() {
            let label = match layer.kind.as_str() {
                "catchments" => text(pt, "Catchments", "Bacias"),
                "rainfall" => text(pt, "Rainfall", "Chuva"),
                "upstream" => text(pt, "Upstream basin", "Bacia a montante"),
                "rivers" => text(pt, "Rivers", "Rios"),
                _ => text(pt, "Stations", "Estações"),
            };
            ui.horizontal(|ui| {
                ui.checkbox(&mut layer.visible, label);
                if i > 0
                    && icon_button(ui, "↑", &text(pt, "Move layer up", "Mover camada acima"))
                        .clicked()
                {
                    reorder = Some(i);
                }
            });
            if layer.kind == "upstream" {
                ui.horizontal_wrapped(|ui| {
                    if let Some((mini, _)) = &self.upstream_basin {
                        ui.label(format!("MINI {mini}"));
                    }
                    let loading = self.loading[UPSTREAM_SLOT];
                    if loading {
                        ui.spinner();
                    }
                    if (loading || self.upstream_basin.is_some())
                        && ui
                            .small_button(text(pt, "Clear basin", "Limpar bacia"))
                            .clicked()
                    {
                        clear = true;
                    }
                });
            }
            ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text(text(
                pt,
                "Opacity",
                "Opacidade",
            )));
        }
        if clear {
            self.clear_upstream();
            ui.ctx().request_discard("Remove cleared basin controls");
        }
        if let Some(i) = reorder {
            self.display.layers.swap(i, i - 1);
        }
        ui.horizontal_wrapped(|ui| {
            for (value, label) in [
                (0., "0–1"),
                (1., "1–10"),
                (10., "10–25"),
                (25., "25–50"),
                (50., "50–100"),
                (100., "100+"),
            ] {
                ui.colored_label(map::rain_color(Some(value)), format!("■ {label}"));
            }
            ui.label("mm");
        });
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
            });
            self.submit(Job::Local(
                self.display.local.iter().map(|l| l.path.clone()).collect(),
            ));
        }
        let mut remove = None;
        let mut reorder = None;
        for (i, style) in self.display.local.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(
                    &mut style.visible,
                    style.path.file_name().unwrap_or_default().to_string_lossy(),
                );
                if i > 0
                    && icon_button(ui, "↑", &text(pt, "Move layer up", "Mover camada acima"))
                        .clicked()
                {
                    reorder = Some(i);
                }
                if icon_button(
                    ui,
                    "×",
                    &text(pt, "Remove local layer", "Remover camada local"),
                )
                .clicked()
                {
                    remove = Some(i);
                }
            });
            ui.add(egui::Slider::new(&mut style.opacity, 0.0..=1.0).text(text(
                pt,
                "Opacity",
                "Opacidade",
            )));
            ui.color_edit_button_srgba_unmultiplied(&mut style.color);
            for layer in self.local.iter().filter(|l| l.path == style.path) {
                ui.small(&layer.name);
            }
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
        let Some(project) = self.project.clone() else {
            return;
        };
        if let Some(run) = self.run.clone() {
            let old = self.display.map_scenario.clone();
            egui::ComboBox::from_label(text(pt, "Map scenario", "Cenário do mapa"))
                .selected_text(&self.display.map_scenario)
                .show_ui(ui, |ui| {
                    for id in &run.scenarios {
                        ui.selectable_value(&mut self.display.map_scenario, id.clone(), id);
                    }
                });
            if old != self.display.map_scenario {
                self.run_rainfall();
                self.request_view();
            }
        }
        ui.horizontal_wrapped(|ui| {
            let reference = ui.label(text(
                pt,
                "Reference · local time",
                "Referência · hora local",
            ));
            ui.add(egui::TextEdit::singleline(&mut self.display.reference).desired_width(145.))
                .labelled_by(reference.id);
            let zone = ui.label(text(pt, "Timezone", "Fuso horário"));
            ui.add(egui::TextEdit::singleline(&mut self.display.zone).desired_width(120.))
                .labelled_by(zone.id);
        });
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.display.explicit,
                false,
                text(pt, "Signed hours", "Horas com sinal"),
            );
            ui.selectable_value(
                &mut self.display.explicit,
                true,
                text(pt, "Start / end", "Início / fim"),
            );
        });
        ui.horizontal_wrapped(|ui| {
            if !self.display.explicit {
                if ui
                    .add(
                        egui::DragValue::new(&mut self.display.hours)
                            .range(-8760..=8760)
                            .suffix(" h"),
                    )
                    .changed()
                {
                    self.run_rainfall();
                }
                ui.small(text(
                    pt,
                    "− Observed · + Forecast",
                    "− Observado · + Previsto",
                ));
            } else {
                for (name, value) in [
                    (text(pt, "From", "Início"), &mut self.display.start),
                    (text(pt, "To", "Fim"), &mut self.display.end),
                ] {
                    let label = ui.label(name);
                    ui.add(egui::TextEdit::singleline(value).desired_width(145.))
                        .labelled_by(label.id);
                }
            }
        });
        if let Err(error) = self.resolve_window() {
            ui.small(error);
        } else {
            ui.small(format!("{} → {}", self.display.start, self.display.end));
        }
        ui.label(text(pt, "Rainfall source", "Fonte de chuva"));
        egui::ComboBox::from_id_salt("rainfall-source")
            .width(ui.available_width())
            .truncate()
            .selected_text(
                self.display
                    .grid
                    .as_ref()
                    .map(|p| {
                        p.strip_prefix(&project.workspace)
                            .unwrap_or(p)
                            .display()
                            .to_string()
                    })
                    .unwrap_or_else(|| "—".into()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.display.grid, None, "—");
                for path in &project.grids {
                    ui.selectable_value(
                        &mut self.display.grid,
                        Some(path.clone()),
                        path.strip_prefix(&project.workspace)
                            .unwrap_or(path)
                            .display()
                            .to_string(),
                    );
                }
            });
        if let Some(rain) = &self.rainfall {
            ui.small(format!(
                "{} · {} → {}",
                rain.rain.source.display(),
                rain.rain.window.start,
                rain.rain.window.end
            ));
        }
        if ui
            .checkbox(
                &mut self.display.upstream,
                text(
                    pt,
                    "Include upstream in summary",
                    "Incluir montante no resumo",
                ),
            )
            .changed()
        {
            self.request_view();
        }
    }
    fn chart_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        let mut apply = false;
        ui.horizontal_wrapped(|ui| {
            for (name, value) in [
                (text(pt, "MINI ID", "ID MINI"), &mut self.chart_fields.mini),
                (
                    text(pt, "Station ID", "ID da estação"),
                    &mut self.chart_fields.station,
                ),
            ] {
                let label = ui.label(name);
                let input = ui
                    .add(egui::TextEdit::singleline(value).desired_width(90.))
                    .labelled_by(label.id);
                apply |= input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            }
            apply |= ui
                .add_enabled(
                    self.project.is_some(),
                    egui::Button::new(text(pt, "Apply", "Aplicar")),
                )
                .clicked();
        });
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut self.chart_exports,
                false,
                text(pt, "Window and sources", "Período e fontes"),
            );
            ui.selectable_value(
                &mut self.chart_exports,
                true,
                text(pt, "Export", "Exportar"),
            );
        });
        egui::ScrollArea::vertical()
            .id_salt("chart-controls")
            .max_height((ui.available_height() * 0.30).clamp(45., 110.))
            .show(ui, |ui| {
                if self.chart_exports {
                    self.chart_exports(ui);
                } else {
                    ui.horizontal_wrapped(|ui| {
                        for (name, value) in [
                            (text(pt, "From", "Início"), &mut self.display.chart_start),
                            (text(pt, "To", "Fim"), &mut self.display.chart_end),
                        ] {
                            let label = ui.label(name);
                            let input = ui
                                .add(egui::TextEdit::singleline(value).desired_width(145.))
                                .labelled_by(label.id);
                            apply |=
                                input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
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
                }
            });
        if apply && self.project.is_some() {
            self.display.comparison = Some(self.chart_fields.clone());
            self.request_chart();
        }
        if self.loading[CHART_SLOT] {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.small(text(pt, "Loading comparison…", "Carregando comparação…"));
            });
        } else if !self.chart_message.is_empty() {
            ui.add(egui::Label::new(&self.chart_message).truncate())
                .on_hover_text(&self.chart_message);
        } else if self.charts.report.is_none() {
            ui.small(text(
                pt,
                "Enter IDs or choose View time series from the map menu.",
                "Informe os IDs ou escolha Ver séries temporais no menu do mapa.",
            ));
        }
        if self.chart_retained && self.charts.report.is_some() {
            ui.small(text(
                pt,
                "Showing previous comparison",
                "Exibindo comparação anterior",
            ));
        }
        ui.separator();
    }

    fn location_summary(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        if let Some(location) = &self.display.selection {
            ui.label(match location {
                Location::Station(id) => format!("{}: {id}", text(pt, "Station", "Estação")),
                Location::Mini(id) => format!("MINI: {id}"),
            });
        }
        if let Some(view) = &self.view
            && let Some(location) = &view.location
        {
            ui.small(format!(
                "{}: {}",
                text(pt, "Displayed summary", "Resumo exibido"),
                charts::location_label(location, pt)
            ));
            if let Some(run) = &view.run {
                ui.small(format!(
                    "{} · {}",
                    run.root.file_name().unwrap_or_default().to_string_lossy(),
                    view.map_scenario
                ));
            }
        }
        if let Some(summary) = self.view.as_ref().and_then(|v| v.summary.as_ref()) {
            let mean = summary
                .mean_mm
                .map(|v| format!("{v:.1}"))
                .unwrap_or_else(|| "NA".into());
            ui.label(format!("{:.1} km² · {mean} mm", summary.total_area_km2));
            ui.small(format!(
                "{:.1}/{:.1} km²·h",
                summary.covered_area_hours, summary.expected_area_hours
            ));
            if let Some(view) = &self.view {
                ui.small(format!(
                    "{} → {} · {}",
                    summary.window.start, summary.window.end, view.zone
                ));
            }
        } else if let Some(area) = self.view.as_ref().and_then(|view| view.area_km2) {
            ui.label(format!("{area:.1} km²"));
        }
        ui.separator();
    }
}

impl Monitoring {
    fn map_page(&mut self, ui: &mut egui::Ui) {
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
                _ => None,
            };
        }
        let focus_map =
            ui.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Num1));
        ui.horizontal_wrapped(|ui| {
            for (glyph, name, zoom_in) in [
                ("+", text(pt, "Zoom in", "Aproximar"), true),
                ("−", text(pt, "Zoom out", "Afastar"), false),
            ] {
                let response = icon_button(ui, glyph, &name);
                if zoom_in && focus_map {
                    response.request_focus();
                }
                if response.clicked() {
                    if zoom_in {
                        let _ = self.display.camera.zoom_in();
                    } else {
                        let _ = self.display.camera.zoom_out();
                    }
                }
            }
            if let Some(tiles) = &self.tiles {
                use walkers::Tiles;
                let attribution = tiles.attribution();
                ui.hyperlink_to(attribution.text, attribution.url);
            }
        });
        let frame = egui::Frame::group(ui.style())
            .outer_margin(6)
            .inner_margin(1)
            .fill(ui.visuals().extreme_bg_color);
        if ui.available_height() > 16. && ui.available_width() > 16. {
            frame.show(ui, |ui| {
                ui.set_clip_rect(ui.clip_rect().intersect(ui.max_rect()));
                if let Some(project) = &self.project {
                    let upstream = self.upstream_basin.as_ref().map_or(&[][..], |(_, ids)| ids);
                    let mut hit = None;
                    let response = Map::new(
                        self.tiles.as_mut().map(|t| t as &mut dyn walkers::Tiles),
                        &mut self.display.camera,
                        lon_lat(-52., -30.),
                    )
                    .with_plugin(map::Overlay {
                        project,
                        layers: &self.display.layers,
                        rainfall: self.rainfall.as_ref(),
                        selection: &self.display.selection,
                        upstream,
                        hit: &mut hit,
                        cache: &mut self.cache,
                        labels: self.display.labels,
                        local: &self.local,
                        local_display: &self.display.local,
                    })
                    .show(ui, |_, _, _, _| {})
                    .response;
                    if response.secondary_clicked() {
                        self.map_menu = hit;
                    } else if let Some(location) = hit {
                        self.select_location(location);
                        ui.ctx().request_repaint();
                    }
                    map::feature_menu(&response, self.map_menu.clone(), |ui, location| {
                        ui.label(self.feature_label(location));
                        self.feature_actions(ui, location);
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.heading(text(
                            self.preferences.portuguese,
                            "Open a regional project to monitor available data",
                            "Abra um projeto regional para visualizar os dados",
                        ));
                    });
                }
            });
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
        self.location_summary(ui);
        egui::ScrollArea::both().show(ui, |ui| {
            if let Some(report) = self.charts.report.as_ref() {
                ui.label(
                    self.charts
                        .location_caption(report, self.preferences.portuguese),
                );
                egui::Grid::new("series-table")
                    .striped(true)
                    .show(ui, |ui| {
                        for label in if self.preferences.portuguese {
                            ["Hora", "Série", "Variável", "Valor", "Unidade"]
                        } else {
                            ["Time", "Series", "Variable", "Value", "Unit"]
                        } {
                            ui.strong(label);
                        }
                        ui.end_row();
                        for series in &report.series {
                            for point in &series.points {
                                ui.label(point.timestamp.to_string());
                                ui.label(series.scenario_id.as_deref().unwrap_or("OBS"));
                                ui.label(&series.variable);
                                ui.label(
                                    point
                                        .value
                                        .map(|v| v.to_string())
                                        .unwrap_or_else(|| "NA".into()),
                                );
                                ui.label(&series.unit);
                                ui.end_row();
                            }
                        }
                    });
            }
        });
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
            Departure::New => self.runners.start(runners::Action::New, ctx),
            Departure::Draft(path, reference, zone) => self.runners.start(
                runners::Action::Open {
                    path,
                    reference,
                    zone,
                },
                ctx,
            ),
            Departure::Quit => {
                self.allow_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Departure::Close => {
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
                self.view = None;
                self.rainfall = None;
                self.local.clear();
                self.cache.clear();
                self.clear_upstream();
                self.map_menu = None;
                self.tiles = None;
                self.loading.fill(false);
                self.runners = runners::Runners::default();
            }
        }
    }
    fn departure_dialog(&mut self, ctx: &egui::Context) {
        let pt = self.preferences.portuguese;
        if let Some((path, mut reference, mut zone)) = self.open_candidate.clone() {
            let mut open = false;
            let mut cancel = false;
            egui::Modal::new(egui::Id::new("Open reference")).show(ctx, |ui| {
                ui.heading(text(
                    pt,
                    "Open draft for reference",
                    "Abrir rascunho para referência",
                ));
                ui.label(path.display().to_string());
                ui.label(text(
                    pt,
                    "Candidate reference · local time",
                    "Referência candidata · hora local",
                ));
                ui.text_edit_singleline(&mut reference);
                ui.label(text(pt, "IANA timezone", "Fuso IANA"));
                ui.text_edit_singleline(&mut zone);
                ui.horizontal(|ui| {
                    open = ui
                        .button(text(pt, "Open exact match", "Abrir correspondência exata"))
                        .clicked();
                    cancel = ui.button(text(pt, "Cancel", "Cancelar")).clicked();
                });
            });
            self.open_candidate = Some((path.clone(), reference.clone(), zone.clone()));
            if open {
                self.open_candidate = None;
                self.depart(Departure::Draft(path, reference, zone));
            } else if cancel {
                self.open_candidate = None;
            }
            return;
        }
        if let Some(path) = self.runners.overwrite.clone() {
            egui::Modal::new(egui::Id::new("Destination exists")).show(ctx, |ui| {
                ui.heading(text(pt, "Destination exists", "Destino existente"));
                ui.label(path.display().to_string());
                ui.label(text(
                    pt,
                    "Overwrite this draft?",
                    "Sobrescrever este rascunho?",
                ));
                ui.horizontal(|ui| {
                    if ui.button(text(pt, "Overwrite", "Sobrescrever")).clicked() {
                        self.runners.overwrite = None;
                        self.runners
                            .start(runners::Action::Save(path.clone(), true), ctx);
                    }
                    if ui.button(text(pt, "Cancel", "Cancelar")).clicked() {
                        self.runners.overwrite = None;
                        self.saving_departure = false;
                    }
                });
            });
            return;
        }
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
                                active.control.cancel();
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
        if !self.runners.dirty() {
            self.finish_departure(ctx);
            return;
        }
        if self.saving_departure {
            if !self.runners.saved {
                self.saving_departure = false;
            } else if self.runners.config_dirty() {
                self.runners.start(runners::Action::SaveConfig, ctx);
                return;
            } else if self
                .runners
                .draft
                .as_ref()
                .is_some_and(mgb_ops::app::Draft::is_dirty)
            {
                match self.runners.save_path() {
                    Ok(path) => self.runners.save(path, ctx),
                    Err(e) => {
                        self.runners.status = e;
                        self.saving_departure = false;
                    }
                }
                return;
            }
        }
        egui::Modal::new(egui::Id::new("Unsaved work")).show(ctx, |ui| {
            ui.heading(text(pt, "Unsaved work", "Alterações não salvas"));
            ui.label(text(
                pt,
                "Save workspace settings and operation draft before continuing?",
                "Salvar configurações e rascunho operacional antes de continuar?",
            ));
            ui.label(self.runners.localized_status(pt));
            ui.horizontal(|ui| {
                if ui.button(text(pt, "Save", "Salvar")).clicked() {
                    self.saving_departure = true;
                    if self.runners.config_dirty() {
                        self.runners.start(runners::Action::SaveConfig, ctx);
                    } else {
                        match self.runners.save_path() {
                            Ok(path) => self.runners.save(path, ctx),
                            Err(e) => {
                                self.runners.status = e;
                                self.saving_departure = false;
                            }
                        }
                    }
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
        self.poll(&ctx);
        if let Some(outcome) = self.runners.poll() {
            match outcome {
                runners::Outcome::Completed(root) => {
                    self.submit(Job::Run(root));
                }
                runners::Outcome::Prepared(root) => {
                    self.runners.prepared = root.clone();
                    self.display.grid = Some(root.join("checkpoints/observed-grid.nc"));
                    self.request_view();
                }
                _ => {}
            }
        }
        if self.runners.initialized {
            self.display.draft = self.runners.last_saved.clone();
            if let Some(zone) = self
                .runners
                .draft
                .as_ref()
                .and_then(|d| d.document.operation.time_zone.clone())
                .filter(|z| z.parse::<chrono_tz::Tz>().is_ok())
            {
                self.display.operation_zone = Some(zone);
            }
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.depart(Departure::Quit);
        }
        ctx.set_theme(match self.preferences.appearance {
            1 => egui::ThemePreference::Light,
            2 => egui::ThemePreference::Dark,
            _ => egui::ThemePreference::System,
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
        egui::Panel::top("ribbon").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(Form::Jobs.label(self.preferences.portuguese))
                    .clicked()
                {
                    self.display.layout.form(Form::Jobs);
                }
                if self.runners.busy() {
                    ui.spinner();
                }
                ui.add(egui::Label::new(&self.status).truncate())
                    .on_hover_text(&self.status);
            });
        });
        let width = ui.available_width();
        egui::Panel::left("run-and-layers")
            .exact_size((width * 0.24).clamp(156., 215.))
            .show(ui, |ui| self.left_panel(ui));
        let min = (width * 0.30).clamp(190., 290.);
        let response = egui::Panel::right(egui::Id::new(("right-pane", &self.display.config)))
            .resizable(true)
            .default_size(self.display.layout.width)
            .min_size(min)
            .max_size((width * 0.41).max(min))
            .show(ui, |ui| self.right_pane(ui));
        self.display.layout.width = response.response.rect.width();
        egui::CentralPanel::default().show(ui, |ui| self.map_page(ui));
        self.departure_dialog(&ctx);
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
    fn preferences_preserve_storage_identity_and_independent_operation_zone() {
        assert_eq!(APP_ID, "MGB Ops · Monitoring");
        let mut display = Display {
            operation_zone: Some("Asia/Tokyo".into()),
            draft: Some(PathBuf::from("operation-stale-0300.json")),
            ..Default::default()
        };
        let serialized = serde_json::to_string(&display).unwrap();
        display = serde_json::from_str(&serialized).unwrap();
        assert_eq!(display.selected_operation_zone("UTC"), "Asia/Tokyo");
        display.draft = Some("different-stale-file.json".into());
        assert_eq!(display.selected_operation_zone("UTC"), "Asia/Tokyo");
        let legacy: Display = serde_json::from_str(
            r#"{"config":"region/config/workspace.toml","run":"region/data/runs/run-1",
                 "layout":{"dock":{"Tabs":{"pages":["Map","Data"],"active":1}},
                           "left":false,"tabs":["Files",{"Form":"Prepare"}]},
                 "operation_zone":"America/Sao_Paulo","basemap":2}"#,
        )
        .unwrap();
        assert_eq!(legacy.selected_operation_zone("UTC"), "America/Sao_Paulo");
        assert_eq!(legacy.config, PathBuf::from("region/config/workspace.toml"));
        assert_eq!(legacy.run, Some(PathBuf::from("region/data/runs/run-1")));
        assert_eq!(legacy.basemap, 2);
        assert_eq!(legacy.layout.active, SideTab::Charts);
        assert!(legacy.layout.tabs.contains(&SideTab::Data));
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
