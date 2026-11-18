mod charts;
mod data;
mod export;
mod layout;
mod map;
mod runners;
use self::layout::{Form, Page};

use self::data::{Job, Loaded, Project, Run, View, ViewRequest, Worker};
use eframe::egui::{self, Color32};
use mgb_ops::analysis::HourlyWindow;
use mgb_ops::app::Location;
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
    loading: [bool; 5],
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
    settings: bool,
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
            loading: [false; 5],
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
            settings: false,
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
        let slot = match &job {
            Job::Open(..) => 0,
            Job::Run(..) => 1,
            Job::View(..) => 2,
            Job::Local(..) => 3,
            Job::Export(..) | Job::Png(..) => 4,
        };
        if slot == 4 && self.loading[4] {
            self.status = text(
                self.preferences.portuguese,
                "An export is already running; wait for completion",
                "Uma exportação está em andamento; aguarde a conclusão",
            );
            return;
        }
        if slot == 0 {
            for dependent in 1..4 {
                self.worker.invalidate(dependent);
                self.loading[dependent] = false;
            }
        }
        if slot == 1 {
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
        let request = (|| {
            let window = self.resolve_window()?;
            let chart_window = HourlyWindow::Explicit {
                start: data::timestamp(&self.display.chart_start)?,
                end: data::timestamp(&self.display.chart_end)?,
            };
            mgb_ops::analysis::resolve_hourly_window(&self.display.zone, &chart_window)?;
            Ok::<_, String>(ViewRequest {
                project,
                run: self.run.clone(),
                location: self.display.selection.clone(),
                overlays: self.display.overlays.clone(),
                observations: self.display.observations,
                map_scenario: self.display.map_scenario.clone(),
                grid: self.display.grid.clone(),
                zone: self.display.zone.clone(),
                window,
                chart_window,
                upstream: self.display.upstream,
            })
        })();
        match request {
            Ok(request) => self.submit(Job::View(request)),
            Err(error) => self.status = error,
        }
    }
    fn poll(&mut self, context: &egui::Context) {
        for reply in self.worker.poll() {
            self.loading[reply.slot] = false;
            if reply.slot == 0 {
                self.switching_project = false;
            }
            match reply.result {
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
                        self.charts.clear();
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
                            self.display.chart_start = (now - chrono::Duration::hours(168))
                                .format("%Y-%m-%dT%H:%M:%S")
                                .to_string();
                            self.display.chart_end = self.display.reference.clone();
                        }
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
                            self.charts.clear();
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
                    } else if !self.display.reference.is_empty() {
                        self.request_view();
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
                    self.display.run = Some(run.root.clone());
                    self.display.zone = run.time_zone.clone();
                    self.display.reference = run.reference.clone();
                    if self.display.chart_start.is_empty() || changed {
                        self.display.chart_start = run.start.clone();
                    }
                    if self.display.chart_end.is_empty() || changed {
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
                    self.run = Some(run);
                    if self.display.grid.is_none() {
                        self.run_rainfall();
                    }
                    self.request_view();
                }
                Ok(Loaded::View(view)) => {
                    let mut view = *view;
                    if let Some(series) = &view.series {
                        let references=self.project.as_ref().map(|p|p.references.iter().filter(|r|matches!(&self.display.selection,Some(Location::Station(id)) if *id==r.station_id)).cloned().collect::<Vec<_>>()).unwrap_or_default();
                        if series.series.is_empty() && !series.findings.is_empty() {
                            view.findings.push(text(
                                self.preferences.portuguese,
                                "Previous charts retained; requested series are unavailable",
                                "Gráficos anteriores mantidos; séries solicitadas indisponíveis",
                            ));
                        } else {
                            if let Some(query) = &view.query {
                                self.charts.source_labels = query
                                    .selections
                                    .iter()
                                    .map(|s| match s {
                                        mgb_ops::app::SeriesSelection::Observation {
                                            series_id,
                                            ..
                                        } => series_id.clone(),
                                        mgb_ops::app::SeriesSelection::Model {
                                            scenario_id,
                                            ..
                                        } => scenario_id.clone(),
                                    })
                                    .collect();
                            }
                            if let Err(error) = self.charts.load(
                                series.clone(),
                                &references,
                                self.preferences.portuguese,
                            ) {
                                view.findings.push(error);
                            }
                        }
                    }
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
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.horizontal_wrapped(|ui| {
            ui.heading("MGB Ops");
            ui.menu_button(text(pt, "▣ Project", "▣ Projeto"), |ui| {
                if ui
                    .button(text(pt, "↗ Open TOML…", "↗ Abrir TOML…"))
                    .clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("TOML", &["toml"])
                        .pick_file()
                {
                    self.open_path(path);
                    ui.close();
                }
                for recent in self.preferences.recent.clone() {
                    if ui.button(recent.config.display().to_string()).clicked() {
                        self.depart(Departure::Project(recent.config, recent.workspace));
                        ui.close();
                    }
                }
                if ui
                    .button(text(pt, "× Close project", "× Fechar projeto"))
                    .clicked()
                {
                    self.depart(Departure::Close);
                    ui.close();
                }
            });
            ui.menu_button(text(pt, "▤ Draft", "▤ Rascunho"), |ui| {
                if !self.runners.initialized || self.runners.busy() || self.switching_project {
                    ui.disable();
                }
                if ui
                    .button(text(pt, "+ New run", "+ Nova execução"))
                    .clicked()
                {
                    self.depart(Departure::New);
                    ui.close();
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
                        Ok(reference) => self.open_candidate = Some((path, reference, zone)),
                        Err(e) => self.runners.status = e,
                    };
                    ui.close();
                }
                if ui.button(text(pt, "▣ Save", "▣ Salvar")).clicked() {
                    match self.runners.save_path() {
                        Ok(path) => self.runners.save(path, ui.ctx()),
                        Err(e) => self.runners.status = e,
                    };
                    ui.close();
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
                                    path.file_name().unwrap_or_default().to_string_lossy(),
                                )
                                .add_filter("Draft JSON", &["json"])
                                .save_file()
                            {
                                self.runners.save(path, ui.ctx());
                            }
                        }
                        Err(e) => self.runners.status = e,
                    };
                    ui.close();
                }
            });
            ui.menu_button(text(pt, "◫ View", "◫ Exibir"), |ui| {
                for page in [Page::Map, Page::Charts, Page::Data] {
                    if ui.button(page.label(pt)).clicked() {
                        self.display.layout.dock.open(page);
                        ui.close();
                    }
                }
                ui.checkbox(
                    &mut self.display.layout.left,
                    text(pt, "Navigation", "Navegação"),
                );
                ui.checkbox(
                    &mut self.display.layout.right,
                    text(pt, "Forms", "Formulários"),
                );
                if ui
                    .button(text(pt, "Reset layout", "Restaurar layout"))
                    .clicked()
                {
                    self.display.layout = layout::Layout::default();
                    for name in ["navigation", "forms"] {
                        let id = egui::Id::new((name, &self.display.config));
                        ui.ctx()
                            .data_mut(|d| d.remove::<egui::containers::panel::PanelState>(id));
                    }
                    ui.close();
                }
            });
            ui.menu_button(text(pt, "▶ Processing", "▶ Processamento"), |ui| {
                for form in [Form::Prepare, Form::Execute, Form::RunAll, Form::Jobs] {
                    if ui.button(form.label(pt)).clicked() {
                        self.display.layout.form(form);
                        ui.close();
                    }
                }
            });
            if ui
                .add_enabled(
                    !self.switching_project,
                    egui::Button::new(text(pt, "↻ Refresh (F5)", "↻ Atualizar (F5)")),
                )
                .clicked()
                && let Some(p) = &self.project
            {
                self.submit(Job::Open(p.config.clone(), p.workspace.clone()));
            }
            if ui
                .button(text(pt, "⚙ Settings", "⚙ Configurações"))
                .clicked()
            {
                self.settings = true;
            }
        });
    }
    fn settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings {
            return;
        }
        let pt = self.preferences.portuguese;
        egui::Modal::new(egui::Id::new("interface-settings")).show(ctx, |ui| {
            ui.heading(text(pt, "Settings", "Configurações"));
            let old = self.preferences.portuguese;
            egui::ComboBox::from_label(text(pt, "Language", "Idioma"))
                .selected_text(if pt { "Português (Brasil)" } else { "English" })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.preferences.portuguese, false, "English");
                    ui.selectable_value(
                        &mut self.preferences.portuguese,
                        true,
                        "Português (Brasil)",
                    );
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
                if let Err(error) =
                    self.charts
                        .load(report, &references, self.preferences.portuguese)
                {
                    self.status = error;
                }
                for (plot, (a, b, c, d)) in self.charts.plots.iter_mut().zip(limits) {
                    plot.set_graph_x_limits(a, b);
                    plot.set_graph_y_limits(c, d, rsplot::YAxis::Left);
                }
            }
            if ui.button(Form::Workspace.label(pt)).clicked() {
                self.display.layout.form(Form::Workspace);
                self.settings = false;
            }
            if ui.button(text(pt, "Close", "Fechar")).clicked() {
                self.settings = false;
            }
        });
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.horizontal_wrapped(|ui| {
            for (i, label) in [
                text(pt, "Layers", "Camadas"),
                text(pt, "Files", "Arquivos"),
                text(pt, "Configs", "Configurações"),
            ]
            .into_iter()
            .enumerate()
            {
                ui.selectable_value(&mut self.display.layout.left_tab, i, label);
            }
            if icon_button(
                ui,
                "◀",
                &text(pt, "Collapse navigation", "Recolher navegação"),
            )
            .clicked()
            {
                self.display.layout.left = false;
            }
        });
        let Some(project) = self.project.clone() else {
            return;
        };
        egui::ScrollArea::vertical().show(ui,|ui|{ match self.display.layout.left_tab {
            2=>{if ui.button(Form::Workspace.label(pt)).clicked(){self.display.layout.form(Form::Workspace);}},
            1=>{            ui.collapsing(
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
            if ui
                .add_enabled(
                    self.view.as_ref().is_some_and(|v| v.query.is_some()) && !self.loading[4],
                    egui::Button::new(text(
                        pt,
                        "Export selected CSV…",
                        "Exportar CSV selecionado…",
                    )),
                )
                .clicked()
                && let Some(query) = self.view.as_ref().and_then(|v| v.query.clone())
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("CSV", &["csv"])
                    .set_file_name("series.csv")
                    .save_file()
            {
                self.submit(Job::Export(query, path));
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
                    charts::location_label(&report.location,pt),
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
                };
                self.submit(Job::Png(Box::new(export), path));
            }
},
            _=>{            ui.heading(text(pt, "Monitoring", "Monitoramento"));
            ui.label(project.workspace.display().to_string());
            for path in &project.retained_sources {ui.colored_label(Color32::YELLOW,format!("{}: {}",text(pt,"Retained source","Fonte anterior mantida"),path.display()));}
            ui.separator();
            let old = self.display.run.clone();
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
                            &mut self.display.run,
                            Some(root.clone()),
                            root.file_name().unwrap_or_default().to_string_lossy(),
                        );
                    }
                });
            if old != self.display.run
                && let Some(root) = self.display.run.clone()
            {
                self.display.chart_start.clear();
                self.display.chart_end.clear();
                self.submit(Job::Run(root));
            }
            let mut changed = false;
            if let Some(run) = self.run.clone() {
                ui.small(format!(
                    "{} · {} · {}",
                    run.root.file_name().unwrap_or_default().to_string_lossy(),
                    status_label(&run.manifest.state,pt),
                    run.manifest.created_at
                ));
                ui.small(format!(
                    "{}: {}",
                    text(pt, "Reference", "Referência"),
                    run.reference
                ));
                let old = self.display.map_scenario.clone();
                egui::ComboBox::from_label(text(pt, "Map scenario", "Cenário do mapa"))
                    .width(125.)
                    .truncate()
                    .selected_text(&self.display.map_scenario)
                    .show_ui(ui, |ui| {
                        for id in &run.scenarios {
                            ui.selectable_value(&mut self.display.map_scenario, id.clone(), id);
                        }
                    });
                if old != self.display.map_scenario {
                    self.run_rainfall();
                    changed = true;
                }
                ui.label(text(pt, "Chart overlays", "Cenários nos gráficos"));
                for id in &run.scenarios {
                    let mut selected = self.display.overlays.contains(id);
                    if ui.checkbox(&mut selected, id).changed() {
                        self.display.overlays.retain(|s| s != id);
                        if selected {
                            self.display.overlays.push(id.clone());
                        }
                        changed = true;
                    }
                }
                ui.collapsing(
                    text(pt, "Status and provenance", "Estado e proveniência"),
                    |ui| {
                        ui.label(run.root.display().to_string());
                        for finding in &run.findings {
                            ui.colored_label(Color32::LIGHT_RED, finding);
                        }
                        if let Some(error) = &run.manifest.error {
                            ui.colored_label(Color32::LIGHT_RED, error);
                        }
                        for scenario in &run.manifest.scenarios {
                            ui.label(format!("{}: {}", scenario.id, status_label(&scenario.state,pt)));
                            if let Some(error) = &scenario.error {
                                ui.colored_label(Color32::LIGHT_RED, &error.message);
                            }
                        }
                        ui.label(serde_json::to_string_pretty(&run.manifest.recipe).unwrap_or_default());
                    },
                );
            }
            changed |= ui
                .checkbox(
                    &mut self.display.observations,
                    text(pt, "Observed series", "Séries observadas"),
                )
                .changed();
            ui.separator();
            ui.label(text(pt, "Search station or MINI", "Buscar estação ou MINI"));
            let search_response = ui.text_edit_singleline(&mut self.search);
            if search_response.lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                && let Ok(mini) = self.search.parse::<i64>()
                && mini > 0
            {
                self.display.selection = Some(Location::Mini(mini));
                changed = true;
            }
            let search = self.search.to_lowercase();
            egui::ScrollArea::vertical().id_salt("locations").max_height(170.).show(ui,|ui| {
                        for entry in project.stations.iter().filter(|s|format!("{} {}",s.station.name,s.station.station_id).to_lowercase().contains(&search)) {
                            let station=&entry.station;
                            let selected=matches!(&self.display.selection,Some(Location::Station(id)) if *id==station.station_id);
                            let label=format!("{} · {}{}",station.name,station.station_id,if station.latitude.is_none() || station.longitude.is_none() {
        " ⊘"}
         else {
        ""}
        );
                            if ui.selectable_label(selected,label).clicked() {
        self.display.selection=Some(Location::Station(station.station_id.clone()));
        changed=true;
        }
                        }
                        for mini in project.topology.iter().filter(|m|m.mini_id.to_string().contains(&search)) {
                            let selected=matches!(&self.display.selection,Some(Location::Mini(id)) if *id==mini.mini_id);
                            if ui.selectable_label(selected,format!("MINI {}",mini.mini_id)).clicked() {
        self.display.selection=Some(Location::Mini(mini.mini_id));
        changed=true;
        }
                        }
                    }
        );
            if changed {self.request_view();}
            ui.collapsing(
                text(pt, "Layers and basemap", "Camadas e mapa base"),
                |ui| {
                    egui::ComboBox::from_label(text(pt, "Basemap", "Mapa base"))
                        .selected_text(
                            [if pt {"Nenhum"} else {"None"}, "OpenStreetMap", "OpenTopoMap"][self.display.basemap.min(2)],
                        )
                        .show_ui(ui, |ui| {
                            for (i, label) in [if pt {"Nenhum"} else {"None"}, "OpenStreetMap", "OpenTopoMap"].iter().enumerate() {
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
                    let mut reorder = None;
                    for (i, layer) in self.display.layers.iter_mut().enumerate() {
                        let label = match layer.kind.as_str() {
                            "catchments" => text(pt, "Catchments", "Bacias"),
                            "rainfall" => text(pt, "Rainfall", "Chuva"),
                            "upstream" => text(pt, "Upstream", "Montante"),
                            "rivers" => text(pt, "Rivers", "Rios"),
                            _ => text(pt, "Stations", "Estações"),
                        };
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut layer.visible, label);
                            if i > 0 && icon_button(ui,"↑",&text(pt,"Move layer up","Mover camada acima")).clicked() {
                                reorder = Some(i);
                            }
                        });
                        ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text(text(
                            pt,
                            "Opacity",
                            "Opacidade",
                        )));
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
                            if i > 0 && icon_button(ui,"↑",&text(pt,"Move layer up","Mover camada acima")).clicked() {
                                reorder = Some(i);
                            }
                            if icon_button(ui,"×",&text(pt,"Remove local layer","Remover camada local")).clicked() {
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
                },
            );
}
        }});
    }
    fn map_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        let Some(project) = self.project.clone() else {
            return;
        };
        ui.collapsing(text(pt, "Map display", "Exibição do mapa"), |ui| {
            ui.heading(text(pt, "Rainfall window", "Período de chuva"));
            ui.label(text(
                pt,
                "Reference · local time",
                "Referência · hora local",
            ));
            ui.text_edit_singleline(&mut self.display.reference);
            ui.text_edit_singleline(&mut self.display.zone);
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
                ui.text_edit_singleline(&mut self.display.start);
                ui.text_edit_singleline(&mut self.display.end);
            }
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
                .button(text(pt, "Apply map window", "Aplicar período do mapa"))
                .clicked()
            {
                self.request_view();
            }
        });
    }
    fn chart_controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        ui.collapsing(text(pt, "Chart display", "Exibição dos gráficos"), |ui| {
            let mut changed = false;
            ui.label(text(pt, "Chart start / end", "Início / fim dos gráficos"));
            ui.text_edit_singleline(&mut self.display.chart_start);
            ui.text_edit_singleline(&mut self.display.chart_end);
            changed |= ui
                .button(text(
                    pt,
                    "Apply window and sources",
                    "Aplicar período e fontes",
                ))
                .clicked();
            if changed {
                self.request_view();
            }
        });
    }
    fn location_summary(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;
        if let Some(location) = &self.display.selection {
            ui.label(match location {
                Location::Station(id) => format!("{}: {id}", text(pt, "Station", "Estação")),
                Location::Mini(id) => format!("MINI: {id}"),
            });
        }
        let changed = ui
            .checkbox(
                &mut self.display.upstream,
                text(pt, "Include upstream", "Incluir montante"),
            )
            .changed();
        if let Some(view) = &self.view
            && let Some(query) = &view.query
        {
            ui.small(format!(
                "{}: {}",
                text(pt, "Displayed summary", "Resumo exibido"),
                charts::location_label(&query.location, pt)
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
        if changed {
            self.request_view();
        }
    }
}

impl Monitoring {
    fn map_page(&mut self, ui: &mut egui::Ui) {
        self.map_controls(ui);
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
        if let Some(tiles) = &self.tiles {
            use walkers::Tiles;
            let attribution = tiles.attribution();
            ui.hyperlink_to(attribution.text, attribution.url);
        }
        ui.horizontal(|ui| {
            if icon_button(
                ui,
                "+",
                &text(self.preferences.portuguese, "Zoom in", "Aproximar"),
            )
            .clicked()
            {
                let _ = self.display.camera.zoom_in();
            }
            if icon_button(
                ui,
                "−",
                &text(self.preferences.portuguese, "Zoom out", "Afastar"),
            )
            .clicked()
            {
                let _ = self.display.camera.zoom_out();
            }
        });
        let mut clicked = None;
        if let Some(project) = &self.project {
            let upstream = self
                .view
                .as_ref()
                .map(|v| v.upstream.as_slice())
                .unwrap_or(&[]);
            Map::new(
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
                clicked: &mut clicked,
                cache: &mut self.cache,
                labels: self.display.labels,
                local: &self.local,
                local_display: &self.display.local,
            })
            .show(ui, |_, _, _, _| {});
        } else {
            ui.centered_and_justified(|ui| {
                ui.heading(text(
                    self.preferences.portuguese,
                    "Open a regional project to monitor available data",
                    "Abra um projeto regional para visualizar os dados",
                ));
            });
        }
        if let Some(location) = clicked {
            self.display.selection = Some(location);
            self.request_view();
        }
    }
    fn data_page(&mut self, ui: &mut egui::Ui) {
        self.location_summary(ui);
        egui::ScrollArea::both().show(ui, |ui| {
            if let Some(report) = self.charts.report.as_ref() {
                ui.label(charts::location_label(
                    &report.location,
                    self.preferences.portuguese,
                ));
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
                self.project = None;
                self.run = None;
                self.view = None;
                self.rainfall = None;
                self.local.clear();
                self.cache.clear();
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
        if self.loading.iter().any(|loading| *loading) || self.worker.is_busy() {
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
                    self.display.chart_start.clear();
                    self.display.chart_end.clear();
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
        if self.loading.iter().any(|b| *b) || self.worker.is_busy() || self.runners.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)) {
            self.depart(Departure::Quit);
        }
        self.runners.shortcuts(&ctx, self.switching_project);
        for (key, page) in [
            (egui::Key::Num1, Page::Map),
            (egui::Key::Num2, Page::Charts),
            (egui::Key::Num3, Page::Data),
        ] {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, key)) {
                self.display.layout.dock.open(page);
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
            ui.horizontal_wrapped(|ui| {
                ui.label(&self.status);
                if self.runners.busy() {
                    ui.spinner();
                }
                if ui
                    .button(Form::Jobs.label(self.preferences.portuguese))
                    .clicked()
                {
                    self.display.layout.form(Form::Jobs);
                }
            });
        });
        if self.display.layout.left {
            let response = egui::Panel::left(egui::Id::new(("navigation", &self.display.config)))
                .resizable(true)
                .default_size(self.display.layout.left_width)
                .min_size(180.)
                .show(ui, |ui| self.controls(ui));
            self.display.layout.left_width = response.response.rect.width();
        } else {
            egui::Panel::left("navigation-closed")
                .exact_size(28.)
                .show(ui, |ui| {
                    if icon_button(
                        ui,
                        "▶",
                        &text(self.preferences.portuguese, "Navigation", "Navegação"),
                    )
                    .clicked()
                    {
                        self.display.layout.left = true;
                    }
                });
        }
        if self.display.layout.right {
            let response = egui::Panel::right(egui::Id::new(("forms", &self.display.config)))
                .resizable(true)
                .default_size(self.display.layout.right_width)
                .min_size(240.)
                .show(ui, |ui| {
                    let pt = self.preferences.portuguese;
                    ui.horizontal_wrapped(|ui| {
                        for form in self.display.layout.forms.clone() {
                            if ui
                                .selectable_label(
                                    self.display.layout.active == form,
                                    form.label(pt),
                                )
                                .clicked()
                            {
                                self.display.layout.active = form;
                            }
                            if form != Form::Jobs
                                && icon_button(
                                    ui,
                                    "×",
                                    &format!(
                                        "{} {}",
                                        text(pt, "Hide form", "Ocultar formulário"),
                                        form.label(pt)
                                    ),
                                )
                                .clicked()
                            {
                                self.display.layout.forms.retain(|f| *f != form);
                                if self.display.layout.active == form {
                                    self.display.layout.active = Form::Jobs;
                                }
                            }
                        }
                        if icon_button(ui, "▶", &text(pt, "Collapse forms", "Recolher formulários"))
                            .clicked()
                        {
                            self.display.layout.right = false;
                        }
                    });
                    ui.separator();
                    ui.add_enabled_ui(!self.switching_project, |ui| {
                        self.runners.show(ui, self.display.layout.active, pt)
                    });
                });
            self.display.layout.right_width = response.response.rect.width();
        } else {
            egui::Panel::right("forms-closed")
                .exact_size(28.)
                .show(ui, |ui| {
                    if icon_button(
                        ui,
                        "◀",
                        &text(self.preferences.portuguese, "Forms", "Formulários"),
                    )
                    .clicked()
                    {
                        self.display.layout.right = true;
                    }
                });
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let mut dock = std::mem::take(&mut self.display.layout.dock);
            dock.show(
                ui,
                self.preferences.portuguese,
                &mut |ui, page| match page {
                    Page::Map => self.map_page(ui),
                    Page::Charts => {
                        self.chart_controls(ui);
                        self.charts.show(ui, self.preferences.portuguese);
                    }
                    Page::Data => self.data_page(ui),
                    #[cfg(test)]
                    Page::Fixture => {
                        ui.label("Fixture");
                    }
                },
            );
            self.display.layout.dock = dock;
        });
        self.settings_dialog(&ctx);
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
        let legacy: Display = serde_json::from_str("{}").unwrap();
        assert_eq!(legacy.selected_operation_zone("UTC"), "UTC");
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
