mod charts;
mod data;
mod export;
mod map;

use self::data::{Job, Loaded, Project, Run, View, ViewRequest, Worker};
use eframe::egui::{self, Color32};
use mgb_ops::analysis::HourlyWindow;
use mgb_ops::app::Location;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use walkers::{HttpTiles, Map, MapMemory, lon_lat};

const STORAGE: &str = "mgb-ops.monitoring.v1";
fn text(pt: bool, en: &str, br: &str) -> String {
    if pt { br } else { en }.into()
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
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    portuguese: bool,
    recent: Vec<Display>,
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
    show_table: bool,
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
            show_table: false,
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
            self.submit(Job::Open(config, workspace));
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
                        "{} · {} stations · {} minis · {} runs · {} findings",
                        project.workspace.display(),
                        project.stations.len(),
                        project.topology.len(),
                        project.runs.len(),
                        project.findings.len()
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
            if ui.button(text(pt, "Open TOML…", "Abrir TOML…")).clicked()
                && let Some(path) = rfd::FileDialog::new()
                    .add_filter("TOML", &["toml"])
                    .pick_file()
            {
                self.open_path(path);
            }
            ui.menu_button(text(pt, "Recent", "Recentes"), |ui| {
                for recent in self.preferences.recent.clone() {
                    if ui.button(recent.config.display().to_string()).clicked() {
                        self.submit(Job::Open(recent.config, recent.workspace));
                        ui.close();
                    }
                }
            });
            if ui
                .add_enabled(
                    self.project.is_some(),
                    egui::Button::new(text(pt, "Refresh (F5)", "Atualizar (F5)")),
                )
                .clicked()
                && let Some(project) = &self.project
            {
                self.submit(Job::Open(project.config.clone(), project.workspace.clone()));
            }
            if ui
                .add_enabled(
                    !self.loading.iter().any(|loading| *loading),
                    egui::Button::new(text(pt, "Close project", "Fechar projeto")),
                )
                .clicked()
            {
                self.remember();
                for slot in 0..4 {
                    self.worker.invalidate(slot);
                }
                self.charts.clear();
                self.worker = Worker::new(ui.ctx().clone());
                self.project = None;
                self.run = None;
                self.view = None;
                self.rainfall = None;
                self.local.clear();
                self.cache.clear();
                self.tiles = None;
                self.loading.fill(false);
            }
            if ui
                .checkbox(&mut self.preferences.portuguese, "Português")
                .changed()
            {
                self.request_view();
            }
            ui.checkbox(&mut self.show_table, text(pt, "Table", "Tabela"));
            if self.loading.iter().any(|b| *b) {
                ui.spinner();
            }
        });
    }
    fn controls(&mut self, ui: &mut egui::Ui) {
        let pt = self.preferences.portuguese;

        let Some(project) = self.project.clone() else {
            ui.label(text(
                pt,
                "Open a configured regional workspace",
                "Abra uma área regional configurada",
            ));

            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(text(pt, "Monitoring", "Monitoramento"));
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
            if let Some(location) = &self.display.selection {
                ui.label(match location {
                    Location::Station(id) => format!("{}: {id}", text(pt, "Station", "Estação")),
                    Location::Mini(id) => format!("MINI: {id}"),
                });
            }
            changed |= ui
                .checkbox(
                    &mut self.display.upstream,
                    text(pt, "Include upstream", "Incluir montante"),
                )
                .changed();
            if let Some(view)=&self.view && let Some(query)=&view.query {
                ui.small(format!("{}: {}",text(pt,"Displayed summary","Resumo exibido"),charts::location_label(&query.location,pt)));
                if let Some(run)=&view.run {ui.small(format!("{} · {}",run.root.file_name().unwrap_or_default().to_string_lossy(),view.map_scenario));}
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
                if let Some(view)=&self.view {ui.small(format!("{} → {} · {}",summary.window.start,summary.window.end,view.zone));}
            } else if let Some(area)=self.view.as_ref().and_then(|view|view.area_km2) {
                ui.label(format!("{area:.1} km²"));
            }
            ui.separator();
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
            if let Some(rain) = &self.rainfall {
                ui.small(format!(
                    "{} · {} → {}",
                    rain.rain.source.display(),
                    rain.rain.window.start,
                    rain.rain.window.end
                ));
            }
            ui.separator();
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
                            if i > 0 && ui.small_button("↑").clicked() {
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
                            if i > 0 && ui.small_button("↑").clicked() {
                                reorder = Some(i);
                            }
                            if ui.small_button("×").clicked() {
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
            ui.collapsing(
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
        });
    }
}

impl eframe::App for Monitoring {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll(ui.ctx());
        if self.loading.iter().any(|b| *b) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ui.input(|i| i.key_pressed(egui::Key::F5))
            && let Some(project) = &self.project
        {
            self.submit(Job::Open(project.config.clone(), project.workspace.clone()));
        }
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui));
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(&self.status);
            });
        });
        egui::Panel::left("controls")
            .resizable(true)
            .default_size(285.)
            .min_size(220.)
            .show(ui, |ui| self.controls(ui));
        egui::Panel::bottom("charts")
            .resizable(true)
            .default_size(480.)
            .min_size(440.)
            .show(ui, |ui| {
                if self.show_table {
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
                                            ui.label(
                                                series.scenario_id.as_deref().unwrap_or("OBS"),
                                            );
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
                } else {
                    self.charts.show(ui, self.preferences.portuguese);
                }
            });
        egui::CentralPanel::default().show(ui, |ui| {
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
                if ui
                    .button("+")
                    .on_hover_text(text(self.preferences.portuguese, "Zoom in", "Aproximar"))
                    .clicked()
                {
                    let _ = self.display.camera.zoom_in();
                }
                if ui
                    .button("−")
                    .on_hover_text(text(self.preferences.portuguese, "Zoom out", "Afastar"))
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
        });
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.remember();
        eframe::set_value(storage, STORAGE, &self.preferences);
    }
}

pub fn run() -> eframe::Result {
    eframe::run_native(
        "MGB Ops · Monitoring",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Wgpu,
            viewport: egui::ViewportBuilder::default().with_inner_size([1400., 950.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Monitoring::new(cc)))),
    )
}
