use super::{layout::Form, text};
use chrono::{TimeZone, Timelike};
use eframe::egui;
use mgb_ops::app::{
    AcquisitionReport, ForecastAdjustment, ForecastPreview, OperationalRun, OperationalRunReport,
    RunScenario, RunSetupRevision, Session, WorkspaceConfig, WorkspaceJob,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
};

pub fn reference_now(zone: &str) -> Result<String, String> {
    reference_at(zone, chrono::Utc::now())
}
fn reference_at(zone: &str, now: chrono::DateTime<chrono::Utc>) -> Result<String, String> {
    let zone: chrono_tz::Tz = zone.parse().map_err(|_| "Select an IANA timezone")?;
    let local = now
        .with_timezone(&zone)
        .naive_local()
        .with_minute(0)
        .and_then(|v| v.with_second(0))
        .and_then(|v| v.with_nanosecond(0))
        .ok_or("Cannot truncate local reference to the hour")?;
    zone.from_local_datetime(&local).single().ok_or("Current local hour is ambiguous or absent in this timezone; choose an explicit unique reference")?;
    Ok(local.format("%Y-%m-%dT%H:%M:%S").to_string())
}
#[derive(Clone)]
pub enum Action {
    Initialize { config: PathBuf },
    SaveConfig,
    Run,
    Prepare(PathBuf),
    ForecastChoices,
    PreviewForecast(RunScenario),
    Workflow(super::workflows::Task),
}

pub enum Outcome {
    Configuration(WorkspaceConfig, Option<Box<OperationalRun>>),
    Completed,
    Run(Box<OperationalRunReport>),
    Preparation(PathBuf),
    Choices {
        window: [String; 4],
        assets: Vec<String>,
        defaults: Vec<RunScenario>,
    },
    Preview(RunSetupRevision, ForecastPreview),
    Workflow(super::workflows::Output),
}
pub struct Active {
    _guard: WorkspaceJob,
    control: mgb_ops::app::OperationControl,
    pub automation: Option<mgb_ops::app::AutomationControl>,
    receiver: mpsc::Receiver<Result<Outcome, String>>,
    thread: Option<JoinHandle<()>>,
    logs: Arc<Mutex<String>>,
}
impl Active {
    pub fn cancel(&self) {
        self.control.cancel();
        if let Some(automation) = &self.automation {
            automation.stop();
        }
    }
    fn progress(&self) -> Option<mgb_ops::app::OperationProgress> {
        self.automation
            .as_ref()
            .and_then(|control| control.progress())
            .or_else(|| self.control.progress())
    }
}
impl Drop for Active {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
#[derive(Default)]
pub struct Runners {
    session: Option<Session>,
    config: Option<WorkspaceConfig>,
    baseline: Option<WorkspaceConfig>,
    config_path: PathBuf,
    pub active: Option<Active>,
    pub status: String,
    logs: String,
    pub cleanup_failed: bool,
    pub saved: bool,
    progress: Option<mgb_ops::app::OperationProgress>,
    acquisition: Option<Box<AcquisitionReport>>,
    window: Option<[String; 4]>,
    assets: Vec<String>,
    defaults: Vec<RunScenario>,
    scenario_choice: String,
    preview: Option<(RunSetupRevision, ForecastPreview)>,
    preview_step: usize,
}

impl Runners {
    pub fn new(session: Session) -> Self {
        Self {
            session: Some(session),
            ..Self::default()
        }
    }

    pub fn localized_status(&self, pt: bool) -> String {
        match self.status.as_str() {
            "Working" => text(pt, "Working", "Em andamento"),
            "Completed" => text(pt, "Completed", "Concluído"),
            "Acquisition cancelled" => text(pt, "Acquisition cancelled", "Aquisição cancelada"),
            "Acquisition completed with findings" => text(
                pt,
                "Acquisition completed with findings",
                "Aquisição concluída com ocorrências",
            ),
            message => message.into(),
        }
    }

    pub fn busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn dirty(&self) -> bool {
        self.config != self.baseline
    }
    pub fn discard(&mut self) {
        self.config = self.baseline.clone();
    }

    pub fn start(&mut self, action: Action, ctx: &egui::Context) {
        if self.busy() {
            return;
        }
        let Some(session) = self.session.clone() else {
            self.status = "Open a workspace first".into();
            return;
        };
        let guard = match session.start_workspace_job() {
            Ok(guard) => guard,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        if let Action::Initialize { config } = &action {
            self.config_path = config.clone();
        }
        let config_path = self.config_path.clone();
        let config = self.config.clone();
        let setup = session.run_setup();
        let control = mgb_ops::app::OperationControl::default();
        let operation_control = control.clone();
        let automation = match &action {
            Action::Workflow(task) if task.needs_automation_control() => {
                Some(mgb_ops::app::AutomationControl::default())
            }
            _ => None,
        };
        let automation_control = automation.clone();
        let (sender, receiver) = mpsc::channel();
        let context = ctx.clone();
        let logs = Arc::new(Mutex::new(String::new()));
        let worker_logs = logs.clone();
        self.status = "Working".into();
        self.saved = false;
        self.cleanup_failed = false;
        self.progress = None;
        self.logs.clear();
        if matches!(
            action,
            Action::Run
                | Action::Workflow(
                    super::workflows::Task::Acquire(..) | super::workflows::Task::ImportAnaCsv(..)
                )
        ) {
            self.acquisition = None;
        }
        let worker = thread::spawn(move || {
            let workspace = session.workspace();
            let result = (|| -> Result<Outcome, String> {
                match action {
                    Action::Initialize { config } => {
                        let config = mgb_ops::app::read_workspace_config(&config)?;
                        let run =
                            OperationalRun::open_workspace(workspace.into(), chrono::Utc::now())
                                .ok()
                                .map(Box::new);
                        Ok(Outcome::Configuration(config, run))
                    }
                    Action::SaveConfig => {
                        let config = config.ok_or("Open a workspace first")?;
                        mgb_ops::app::write_workspace_config(&config_path, &config)?;
                        let run =
                            OperationalRun::open_workspace(workspace.into(), chrono::Utc::now())
                                .ok()
                                .map(|mut run| {
                                    if let Some(setup) = setup {
                                        run.set_parameters(setup.operation.parameters().clone());
                                    }
                                    Box::new(run)
                                });
                        Ok(Outcome::Configuration(config, run))
                    }
                    Action::Workflow(task) => task
                        .run(workspace, &operation_control, automation_control.as_ref())
                        .map(Outcome::Workflow),
                    Action::ForecastChoices => {
                        let operation = setup.ok_or("Open the run form first")?.operation;
                        Ok(Outcome::Choices {
                            window: operation.coverage()?,
                            assets: operation.available_forecasts()?,
                            defaults: operation.default_scenarios()?,
                        })
                    }
                    Action::PreviewForecast(scenario) => {
                        let setup = setup.ok_or("Open the run form first")?;
                        let preview = setup.operation.preview_forecast(&scenario)?;
                        Ok(Outcome::Preview(setup.revision, preview))
                    }
                    Action::Prepare(output) => {
                        let operation = setup.ok_or("Open the run form first")?.operation;
                        operation
                            .prepare_saved(workspace.into(), output, &operation_control)
                            .map(|report| Outcome::Preparation(report.prepared))
                            .map_err(|error| error.to_string())
                    }
                    Action::Run => {
                        let operation = setup.ok_or("Open the run form first")?.operation;
                        let finished = Arc::new(AtomicBool::new(false));
                        let done = finished.clone();
                        let control = operation_control.clone();
                        let observer_context = context.clone();
                        let observer = thread::spawn(move || {
                            while !done.load(Ordering::Acquire) {
                                if let Some(root) =
                                    control.progress().and_then(|progress| progress.attempt)
                                {
                                    let mut contents = String::new();
                                    read_logs(&root, 0, &mut contents);
                                    *worker_logs
                                        .lock()
                                        .unwrap_or_else(|error| error.into_inner()) = contents;
                                }
                                observer_context.request_repaint();
                                thread::park_timeout(std::time::Duration::from_millis(250));
                            }
                        });
                        let report = operation.run_all_report(&operation_control);
                        finished.store(true, Ordering::Release);
                        observer.thread().unpark();
                        let _ = observer.join();
                        Ok(Outcome::Run(Box::new(report)))
                    }
                }
            })();
            let _ = sender.send(result);
            context.request_repaint();
        });
        self.active = Some(Active {
            _guard: guard,
            control,
            automation,
            receiver,
            thread: Some(worker),
            logs,
        });
    }

    pub fn poll(&mut self) -> Option<Outcome> {
        let active = self.active.as_ref()?;
        if let Some(progress) = active.progress() {
            self.progress = Some(progress);
        }
        self.logs = active
            .logs
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let result = match active.receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err("Worker stopped unexpectedly".into()),
        };
        let mut active = self.active.take().expect("active worker");
        if let Some(thread) = active.thread.take()
            && thread.join().is_err()
        {
            self.cleanup_failed = true;
            self.status = "Worker panicked".into();
            return None;
        }
        drop(active);
        self.status = "Completed".into();
        match result {
            Err(error) => {
                self.cleanup_failed = error.to_lowercase().contains("cleanup");
                self.status = error;
                None
            }
            Ok(Outcome::Configuration(config, operation)) => {
                self.baseline = Some(config.clone());
                self.config = Some(config);
                if let Some(operation) = operation
                    && let Some(session) = &self.session
                {
                    session.open_run_setup(*operation);
                }
                self.window = None;
                self.saved = true;
                None
            }
            Ok(Outcome::Choices {
                window,
                assets,
                defaults,
            }) => {
                self.window = Some(window);
                self.assets = assets;
                self.defaults = defaults;
                None
            }
            Ok(Outcome::Preview(revision, preview)) => {
                self.preview = Some((revision, preview));
                self.preview_step = 0;
                None
            }
            Ok(Outcome::Preparation(path)) => {
                self.status = format!("Prepared {}", path.display());
                None
            }
            Ok(Outcome::Workflow(super::workflows::Output::Acquisition(report))) => {
                self.window = None;
                self.status = if report.cancelled {
                    "Acquisition cancelled"
                } else if report.is_complete() {
                    "Completed"
                } else {
                    "Acquisition completed with findings"
                }
                .into();
                self.acquisition = Some(report);
                None
            }
            Ok(Outcome::Run(report)) => {
                self.window = None;
                self.acquisition = report.acquisition.map(Box::new);
                if let Some(error) = report.error {
                    self.status = error.to_string();
                    self.cleanup_failed = self.status.to_lowercase().contains("cleanup");
                    None
                } else {
                    Some(Outcome::Completed)
                }
            }
            Ok(outcome) => Some(outcome),
        }
    }

    pub fn shortcuts(&mut self, context: &egui::Context, switching_project: bool) {
        if !switching_project
            && !self.busy()
            && self.dirty()
            && context.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
        {
            self.start(Action::SaveConfig, context);
        }
    }

    pub fn workspace_settings(&mut self, ui: &mut egui::Ui, pt: bool) {
        ui.heading(Form::Workspace.label(pt));
        let status = self.localized_status(pt);
        let busy = self.busy();
        let Some(config) = &mut self.config else {
            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
            return;
        };
        let original = config.clone();
        ui.add_enabled_ui(!busy, |ui| {
            ui.spacing_mut().text_edit_width = ui.available_width();
            string(
                ui,
                &text(pt, "Geometry file", "Arquivo de geometria"),
                &mut config.spatial.get_or_insert_default().gpkg_path,
            );
            string(
                ui,
                &text(pt, "Engine executable", "Executável MGB"),
                &mut config.engine.get_or_insert_default().executable,
            );
            string(
                ui,
                &text(pt, "Baseline Input directory", "Diretório Input base"),
                &mut config.engine.get_or_insert_default().input_dir,
            );
            restore_empty(&original.spatial, &mut config.spatial);
            restore_empty(&original.engine, &mut config.engine);
        });
        if !status.is_empty() {
            ui.label(status);
        }
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(!busy, |ui| {
                if ui.button(text(pt, "Save", "Salvar")).clicked() {
                    self.start(Action::SaveConfig, ui.ctx());
                }
                if ui.button(text(pt, "Revert", "Reverter")).clicked() {
                    self.config = self.baseline.clone();
                }
            });
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, form: Form, pt: bool, workspace: Option<&Path>) {
        if form == Form::Jobs {
            self.jobs(ui, pt);
            return;
        }
        ui.heading(Form::NewRun.label(pt));
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_enabled_ui(!self.busy(), |ui| self.show_operation(ui, pt, workspace));
        });
        ui.label(self.localized_status(pt));
    }

    fn show_operation(&mut self, ui: &mut egui::Ui, pt: bool, workspace: Option<&Path>) {
        let Some(session) = self.session.clone() else {
            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
            return;
        };
        let Some(setup) = session.run_setup() else {
            ui.label(text(
                pt,
                "Set the workspace run timezone in config.toml",
                "Defina o fuso horário da execução em config.toml",
            ));
            return;
        };
        let operation = &setup.operation;
        let mut parameters = operation.parameters().clone();
        let mut reference = operation.reference().to_owned();
        ui.label(text(pt, "Reference hour", "Hora de referência"));
        if ui.text_edit_singleline(&mut reference).changed() {
            parameters.reference_time = Some(reference);
        }
        let defaults = self
            .config
            .as_ref()
            .and_then(|config| config.run.clone())
            .unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            let mut days = parameters.lookback_days.unwrap_or(defaults.lookback_days);
            if ui
                .add(egui::DragValue::new(&mut days).prefix(text(
                    pt,
                    "Observed days ",
                    "Dias observados ",
                )))
                .changed()
            {
                parameters.lookback_days = Some(days);
            }
            let mut days = parameters.forecast_days.unwrap_or(defaults.forecast_days);
            if ui
                .add(
                    egui::DragValue::new(&mut days)
                        .range(1..=u32::MAX)
                        .prefix(text(pt, "Forecast days ", "Dias previstos ")),
                )
                .changed()
            {
                parameters.forecast_days = Some(days);
            }
        });
        let mut current = operation.clone();
        current.set_parameters(parameters.clone());
        let coverage = current.coverage();
        match &coverage {
            Ok([history_start, history_end, forecast_start, forecast_end]) => {
                ui.label(format!(
                    "{}  {history_start} → {history_end}",
                    text(pt, "Observed", "Observado")
                ));
                ui.label(format!(
                    "{}  {forecast_start} → {forecast_end}",
                    text(pt, "Forecast", "Previsão")
                ));
            }
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        }
        let mut selections = parameters
            .scenarios
            .clone()
            .unwrap_or_else(|| self.defaults.clone());
        let mut choices_changed = false;
        let mut preview_action = None;
        let mut movement = None;
        for (index, scenario) in selections.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let label = match scenario {
                        RunScenario::Zero => text(pt, "Zero future rain", "Sem chuva futura"),
                        RunScenario::Raw { forecast } => format!(
                            "{} · {}",
                            text(pt, "Raw", "Bruta"),
                            display_forecast(forecast, operation.time_zone())
                        ),
                        RunScenario::Corrected { forecast, .. } => format!(
                            "{} · {}",
                            text(pt, "Corrected", "Corrigida"),
                            display_forecast(forecast, operation.time_zone())
                        ),
                    };
                    ui.label(label);
                    if ui.small_button("↑").clicked() && index > 0 {
                        movement = Some((index, Some(index - 1)));
                    }
                    if ui.small_button("↓").clicked() {
                        movement = Some((index, Some(index + 1)));
                    }
                    if ui.small_button("×").clicked() {
                        movement = Some((index, None));
                    }
                });
                if let RunScenario::Corrected { corrections, .. } = scenario {
                    choices_changed |= correction_editor(ui, pt, corrections);
                    if ui
                        .button(text(pt, "Preview correction", "Prévia da correção"))
                        .clicked()
                    {
                        preview_action = Some(Action::PreviewForecast(scenario.clone()));
                    }
                }
            });
        }
        if let Some((index, destination)) = movement {
            if let Some(destination) = destination {
                if destination < selections.len() {
                    selections.swap(index, destination);
                    choices_changed = true;
                }
            } else {
                selections.remove(index);
                choices_changed = true;
            }
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    !selections.contains(&RunScenario::Zero),
                    egui::Button::new(text(pt, "Add zero", "Adicionar sem chuva")),
                )
                .clicked()
            {
                selections.push(RunScenario::Zero);
                choices_changed = true;
            }
            if ui
                .button(text(pt, "Restore defaults", "Restaurar padrões"))
                .clicked()
            {
                parameters.scenarios = None;
                choices_changed = false;
            }
        });
        egui::ComboBox::from_id_salt("forecast-cycle")
            .selected_text(if self.scenario_choice.is_empty() {
                text(pt, "Forecast cycle", "Ciclo da previsão")
            } else {
                display_forecast(&self.scenario_choice, operation.time_zone())
            })
            .show_ui(ui, |ui| {
                for choice in &self.assets {
                    ui.selectable_value(
                        &mut self.scenario_choice,
                        choice.clone(),
                        display_forecast(choice, operation.time_zone()),
                    );
                }
            });
        ui.horizontal_wrapped(|ui| {
            let selected = !self.scenario_choice.is_empty();
            let raw = RunScenario::Raw { forecast: self.scenario_choice.clone() };
            if ui.add_enabled(selected && !selections.contains(&raw), egui::Button::new(text(pt, "Add raw", "Adicionar bruta"))).clicked() {
                selections.push(raw);
                choices_changed = true;
            }
            let corrected = selections.iter().any(|scenario| matches!(scenario, RunScenario::Corrected { forecast, .. } if forecast == &self.scenario_choice));
            if ui.add_enabled(selected && !corrected, egui::Button::new(text(pt, "Add corrected", "Adicionar corrigida"))).clicked() {
                selections.push(RunScenario::Corrected { forecast: self.scenario_choice.clone(), corrections: vec![ForecastAdjustment::default()] });
                choices_changed = true;
            }
        });
        if choices_changed {
            parameters.scenarios = Some(selections);
        }
        if !self.busy()
            && let Err(error) = session.update_run_setup(&setup.revision, parameters)
        {
            self.status = error;
        }
        if let Some((revision, preview)) = &self.preview
            && session
                .run_setup()
                .is_some_and(|setup| &setup.revision == revision)
        {
            forecast_preview(ui, pt, preview, &mut self.preview_step);
        }
        let valid = coverage.is_ok();
        if !self.busy() && valid && coverage.ok().as_ref() != self.window.as_ref() {
            // Record attempts by window so a read error does not start a worker every frame.
            self.window = current.coverage().ok();
            self.start(Action::ForecastChoices, ui.ctx());
            return;
        }
        if self.busy() {
            return;
        }
        if let Some(action) = preview_action {
            self.start(action, ui.ctx());
            return;
        }
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(text(pt, "Run All", "Executar tudo")),
                )
                .clicked()
            {
                self.start(Action::Run, ui.ctx());
            }
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(text(pt, "Save preparation", "Salvar preparação")),
                )
                .clicked()
                && let Some(workspace) = workspace
                && let Some(path) = rfd::FileDialog::new()
                    .set_directory(workspace.join("runs"))
                    .save_file()
            {
                self.start(Action::Prepare(path), ui.ctx());
            }
        });
    }

    pub fn jobs(&mut self, ui: &mut egui::Ui, pt: bool) {
        ui.heading(text(pt, "Jobs", "Tarefas"));
        if self.busy() {
            ui.spinner();
        }
        if let Some(progress) = self
            .active
            .as_ref()
            .and_then(Active::progress)
            .or_else(|| self.progress.clone())
        {
            ui.label(match progress.stage {
                mgb_ops::app::OperationStage::Acquisition => text(pt, "Acquisition", "Aquisição"),
                mgb_ops::app::OperationStage::Preparation => text(pt, "Preparation", "Preparação"),
                mgb_ops::app::OperationStage::History => text(pt, "History", "Histórico"),
                mgb_ops::app::OperationStage::Scenarios => text(pt, "Scenarios", "Cenários"),
                mgb_ops::app::OperationStage::Finalization => {
                    text(pt, "Finalization", "Finalização")
                }
            });
            if let Some(detail) = progress.detail {
                ui.label(detail);
            }
            for scenario in progress.scenarios {
                ui.label(format!(
                    "{} · {}",
                    scenario.id,
                    super::status_label(&scenario.state, pt)
                ));
            }
            if let Some(fraction) = progress.fraction {
                ui.add(egui::ProgressBar::new(fraction as f32));
            } else if self.busy() {
                ui.label(text(pt, "Progress unavailable", "Progresso indeterminado"));
            }
        }
        if let Some(active) = &self.active
            && ui.button(text(pt, "Cancel", "Cancelar")).clicked()
        {
            active.cancel();
        }
        ui.label(self.localized_status(pt));
        if let Some(report) = &self.acquisition {
            show_acquisition(ui, pt, report);
        }
        super::section(ui, text(pt, "Logs", "Registros"), |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(egui::Label::new(egui::RichText::new(&self.logs).monospace()).wrap());
            });
        });
    }
}

fn show_acquisition(ui: &mut egui::Ui, pt: bool, report: &AcquisitionReport) {
    if let Some(ana) = &report.ana {
        if let Some(error) = &ana.error {
            ui.label(error);
        }
        for station in &ana.stations {
            let status = if station.error.is_some() {
                text(pt, "Failed", "Falhou")
            } else if station.normalized_rows == 0 {
                text(pt, "No new data", "Sem dados novos")
            } else {
                format!(
                    "{} · {}",
                    station.changed_samples,
                    text(pt, "samples changed", "amostras alteradas")
                )
            };
            ui.collapsing(format!("ANA {} · {status}", station.station_code), |ui| {
                if let Some(error) = &station.error {
                    ui.label(error);
                }
                for path in station.csv.iter().chain(&station.raw_xml) {
                    ui.label(path.display().to_string());
                }
            });
        }
    }
    for forecast in &report.forecasts {
        let provider = match forecast.provider {
            mgb_ops::store::ForecastProvider::Ecmwf => "ECMWF",
            mgb_ops::store::ForecastProvider::Noaa => "NOAA",
        };
        ui.collapsing(
            format!(
                "{provider} · {}",
                if forecast.error.is_some() {
                    text(pt, "Failed", "Falhou")
                } else if forecast.reused {
                    text(pt, "Reused", "Reutilizado")
                } else {
                    text(pt, "Registered", "Registrado")
                }
            ),
            |ui| {
                if let Some(error) = &forecast.error {
                    ui.label(error);
                }
                if let Some(cycle) = forecast.valid_from {
                    ui.label(cycle.format("%Y-%m-%d %H:%M").to_string());
                }
                if let Some(id) = &forecast.asset_id {
                    ui.label(id);
                }
                for path in [&forecast.retained_source, &forecast.native_artifact]
                    .into_iter()
                    .flatten()
                {
                    ui.label(path.display().to_string());
                }
            },
        );
    }
}
fn read_logs(root: &Path, depth: usize, out: &mut String) {
    if depth > 3 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                read_logs(&path, depth + 1, out)
            } else if path.extension().is_some_and(|e| e == "log")
                && let Ok(bytes) = mgb_ops::app::tail_log(&path, 16384)
            {
                out.push_str(&format!(
                    "{}\n{}\n",
                    path.display(),
                    String::from_utf8_lossy(&bytes)
                ));
            }
        }
    }
}
fn string(ui: &mut egui::Ui, label: &str, value: &mut Option<String>) {
    let label = ui.label(label);
    let mut input = value.clone().unwrap_or_default();
    if ui
        .text_edit_singleline(&mut input)
        .labelled_by(label.id)
        .changed()
    {
        *value = (!input.is_empty()).then_some(input);
    }
}
fn restore_empty<T: Default + PartialEq>(original: &Option<T>, current: &mut Option<T>) {
    if original.is_none() && current.as_ref().is_some_and(|v| *v == T::default()) {
        *current = None;
    }
}

fn display_forecast(choice: &str, zone: &str) -> String {
    choice
        .split_once('@')
        .and_then(|(product, cycle)| {
            let zone: chrono_tz::Tz = zone.parse().ok()?;
            let cycle = chrono::DateTime::parse_from_rfc3339(cycle)
                .ok()?
                .with_timezone(&zone);
            Some(format!("{product} · {}", cycle.format("%Y-%m-%d %H:%M")))
        })
        .unwrap_or_else(|| choice.to_owned())
}

fn correction_editor(
    ui: &mut egui::Ui,
    pt: bool,
    corrections: &mut Vec<ForecastAdjustment>,
) -> bool {
    let mut changed = false;
    let mut movement = None;
    for (index, correction) in corrections.iter_mut().enumerate() {
        ui.push_id(index, |ui| {
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.t0_hour)
                            .range(0..=i64::MAX)
                            .prefix(text(pt, "From hour ", "Da hora ")),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.t1_hour)
                            .range(0..=i64::MAX)
                            .prefix(text(pt, "To hour ", "Até a hora ")),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.shift_lat_pixels).prefix(text(
                            pt,
                            "Latitude pixels ",
                            "Pixels latitude ",
                        )),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.shift_lon_pixels).prefix(text(
                            pt,
                            "Longitude pixels ",
                            "Pixels longitude ",
                        )),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.rotation_deg).prefix(text(
                            pt,
                            "Rotation ° ",
                            "Rotação ° ",
                        )),
                    )
                    .changed();
                changed |= ui
                    .add(
                        egui::DragValue::new(&mut correction.multiplication_factor).prefix(text(
                            pt,
                            "Multiplier ",
                            "Multiplicador ",
                        )),
                    )
                    .changed();
                if ui.small_button("↑").clicked() && index > 0 {
                    movement = Some((index, Some(index - 1)));
                }
                if ui.small_button("↓").clicked() {
                    movement = Some((index, Some(index + 1)));
                }
                if ui.small_button("×").clicked() {
                    movement = Some((index, None));
                }
            });
        });
    }
    if let Some((index, destination)) = movement {
        if let Some(destination) = destination {
            if destination < corrections.len() {
                corrections.swap(index, destination);
                changed = true;
            }
        } else {
            corrections.remove(index);
            changed = true;
        }
    }
    if ui
        .button(text(pt, "Add correction", "Adicionar correção"))
        .clicked()
    {
        corrections.push(ForecastAdjustment::default());
        changed = true;
    }
    changed
}

fn forecast_preview(ui: &mut egui::Ui, pt: bool, report: &ForecastPreview, step: &mut usize) {
    if report.raw.is_empty() {
        return;
    }
    ui.add(egui::Slider::new(step, 0..=report.raw.len() - 1).text(text(
        pt,
        "Interval",
        "Intervalo",
    )));
    let before = &report.raw[*step];
    let after = &report.corrected[*step];
    ui.label(format!(
        "{} → {}",
        before.interval.start, before.interval.end
    ));
    let maximum = [before, after]
        .into_iter()
        .flat_map(|frame| frame.values.iter().zip(&frame.valid))
        .filter_map(|(&value, &valid)| valid.then_some(value))
        .fold(0.0_f64, f64::max);
    ui.columns(2, |columns| {
        super::workflows::grid_preview(
            &mut columns[0],
            before,
            maximum,
            &text(pt, "Original", "Original"),
        );
        super::workflows::grid_preview(
            &mut columns[1],
            after,
            maximum,
            &text(pt, "Corrected", "Corrigida"),
        );
    });
}
