use super::{layout::Form, text};
use chrono::{TimeZone, Timelike};
use eframe::egui;
use mgb_ops::app::{
    Draft, OperationalRun, RegistrationReport, RegistrationSelection, WorkspaceConfig,
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
pub fn anchored_name(draft: &Draft) -> Result<String, String> {
    let operation = &draft.document.operation;
    let zone: chrono_tz::Tz = operation
        .time_zone
        .as_deref()
        .ok_or("Timezone required")?
        .parse()
        .map_err(|_| "Select an IANA timezone")?;
    let reference = super::data::timestamp(
        operation
            .reference_time
            .as_deref()
            .ok_or("Reference required")?,
    )?;
    let local = zone
        .from_local_datetime(&reference)
        .single()
        .ok_or("Reference is ambiguous or absent in this timezone")?;
    Ok(format!(
        "operation-{}.json",
        local.format("%Y%m%dT%H%M%S%z")
    ))
}
fn matches_reference(draft: &Draft, reference: &str, zone: &str) -> bool {
    draft.document.operation.time_zone.as_deref() == Some(zone)
        && draft.document.operation.reference_time.as_deref() == Some(reference)
}
fn fresh(workspace: PathBuf, config: &WorkspaceConfig, zone: &str) -> Result<Draft, String> {
    Draft::new_run(
        workspace.clone(),
        &workspace,
        Some(config),
        &reference_now(zone)?,
        zone,
    )
}
#[derive(Clone)]
pub enum Action {
    Initialize {
        workspace: PathBuf,
        config: PathBuf,
        zone: String,
        remembered: Option<PathBuf>,
    },
    Accept,
    RunAll,
    OpenOperation {
        workspace: PathBuf,
        recipe: PathBuf,
    },
    ValidateOperation,
    AcceptOperation,
    RunOperation,
    Save(PathBuf, bool),
    SaveConfig,
    New(String),
    Open(PathBuf),
    Choices,
    Readiness,
    Workflow(super::workflows::Task),
    MaterializeReplacements,
    MaterializeCorrections(usize),
    PreviewObservations,
    PreviewForecast(usize),
    RegisterInstructions(RegistrationSelection),
}
pub enum Outcome {
    Initialized(Box<Draft>, WorkspaceConfig),
    Draft(Box<Draft>),
    OpenedDraft(Box<Draft>),
    DraftChanged,
    Registered(Box<Draft>, RegistrationReport, Vec<String>),
    RegistrationReport(RegistrationReport, Vec<String>),
    Accepted(Box<Draft>, Vec<String>),
    Saved(Box<Draft>),
    Config(WorkspaceConfig),
    Completed(PathBuf),
    OperationalCompleted(PathBuf),
    Operation(Box<OperationalRun>, Option<Vec<String>>),
    OperationalReadiness(Vec<String>),
    Choices(Choices),
    Collision(PathBuf),
    Readiness(Vec<String>),
    Workflow(super::workflows::Output),
}
#[derive(Default)]
pub struct Choices {
    replacements: Vec<String>,
    assets: Vec<String>,
    corrections: Vec<String>,
    findings: Vec<String>,
}
struct Reply {
    result: Result<Outcome, String>,
    logs: String,
}
pub struct Active {
    pub control: mgb_ops::app::OperationControl,
    pub automation: Option<mgb_ops::app::AutomationControl>,
    receiver: mpsc::Receiver<Reply>,
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
    pub draft: Option<Draft>,
    operation: Option<OperationalRun>,
    operational_readiness: Option<Vec<String>>,
    pub config: Option<WorkspaceConfig>,
    baseline: Option<WorkspaceConfig>,
    config_path: PathBuf,
    pub active: Option<Active>,
    pub status: String,
    pub logs: String,
    choices: Choices,
    pub last_saved: Option<PathBuf>,
    pub initialized: bool,
    pub cleanup_failed: bool,
    pub overwrite: Option<PathBuf>,
    pub saved: bool,
    pub readiness: Vec<String>,
    readiness_checked: bool,
    progress: Option<mgb_ops::app::OperationProgress>,
}
impl Runners {
    pub fn localized_status(&self, pt: bool) -> String {
        match self.status.as_str() {
            "Working" => text(pt, "Working", "Em andamento"),
            "Completed" => text(pt, "Completed", "Concluído"),
            "A job is already active in this workspace" => text(
                pt,
                "A job is already active in this workspace",
                "Uma tarefa já está ativa nesta área de trabalho",
            ),
            other => other.into(),
        }
    }
    pub fn busy(&self) -> bool {
        self.active.is_some()
    }
    pub fn dirty(&self) -> bool {
        self.config != self.baseline || self.draft.as_ref().is_some_and(Draft::is_dirty)
    }
    pub fn discard(&mut self) {
        self.config = self.baseline.clone();
        if let Some(draft) = &mut self.draft {
            draft.discard();
        }
    }
    pub fn config_dirty(&self) -> bool {
        self.config != self.baseline
    }
    pub fn start(&mut self, action: Action, ctx: &egui::Context) {
        if self.busy() {
            self.status = "A job is already active in this workspace".into();
            return;
        }
        let draft = self.draft.clone();
        let operation = self.operation.clone();
        let config = self.config.clone();
        let config_path = self.config_path.clone();
        if let Action::Initialize { config, .. } = &action {
            self.config_path = config.clone();
        }
        let control = mgb_ops::app::OperationControl::default();
        let operation_control = control.clone();
        let automation = match &action {
            Action::Workflow(task) if task.needs_automation_control() => {
                Some(mgb_ops::app::AutomationControl::default())
            }
            _ => None,
        };
        let automation_control = automation.clone();
        let live_logs = Arc::new(Mutex::new(String::new()));
        let worker_logs = live_logs.clone();
        let (sender, receiver) = mpsc::channel();
        let context = ctx.clone();
        if matches!(action, Action::ValidateOperation | Action::AcceptOperation) {
            self.operational_readiness = None;
        }
        self.status = "Working".into();
        self.saved = false;
        self.cleanup_failed = false;
        self.progress = None;
        self.logs.clear();
        let worker = thread::spawn(move || {
            let mut logs = String::new();
            let result = (|| -> Result<Outcome, String> {
                match action {
                    Action::Workflow(task) => {
                        let draft = draft.ok_or("Open a workspace first")?;
                        task.run(
                            &draft.workspace,
                            &operation_control,
                            automation_control.as_ref(),
                        )
                        .map(Outcome::Workflow)
                    }
                    Action::MaterializeReplacements | Action::MaterializeCorrections(_) => {
                        let mut draft = draft.ok_or("Open a draft first")?;
                        if let Action::MaterializeCorrections(index) = action {
                            draft.materialize_corrections(index)?;
                        } else {
                            draft.materialize_replacements()?;
                        }
                        Ok(Outcome::Draft(Box::new(draft)))
                    }
                    Action::PreviewObservations => {
                        let draft = draft.ok_or("Open a draft first")?;
                        draft.preview_observations().map(|value| {
                            Outcome::Workflow(super::workflows::Output::Observations(value))
                        })
                    }
                    Action::PreviewForecast(index) => {
                        let draft = draft.ok_or("Open a draft first")?;
                        draft.preview_forecast(index).map(|value| {
                            Outcome::Workflow(super::workflows::Output::Forecast(value))
                        })
                    }
                    Action::RegisterInstructions(selection) => {
                        let mut draft = draft.ok_or("Open a draft first")?;
                        let report = draft.register_selected(&selection);
                        let origin_errors = report
                            .observations
                            .mappings
                            .iter()
                            .chain(&report.forecasts.mappings)
                            .filter_map(|mapping| {
                                draft
                                    .copy_registered_for_edit(mapping)
                                    .err()
                                    .map(|error| format!("{}: {error}", mapping.catalog_id))
                            })
                            .collect();
                        Ok(Outcome::Registered(Box::new(draft), report, origin_errors))
                    }
                    Action::Initialize {
                        workspace,
                        config,
                        zone,
                        remembered,
                    } => {
                        let config = mgb_ops::app::read_workspace_config(&config)?;
                        let mut draft = fresh(workspace.clone(), &config, &zone)?;
                        if let Some(path) = remembered
                            && let Ok(saved) = Draft::open(workspace.clone(), &workspace, &path)
                            && matches_reference(
                                &saved,
                                draft
                                    .document
                                    .operation
                                    .reference_time
                                    .as_deref()
                                    .unwrap_or_default(),
                                &zone,
                            )
                        {
                            draft = saved;
                            draft.use_day_horizons()?;
                        }
                        Ok(Outcome::Initialized(Box::new(draft), config))
                    }
                    Action::New(zone) => {
                        let draft = draft.ok_or("Open a workspace first")?;
                        let config = config.as_ref().ok_or("Configuration unavailable")?;
                        let new = if let Some(operation) = operation {
                            Draft::new_run(
                                draft.workspace.clone(),
                                &draft.workspace,
                                Some(config),
                                operation.reference(),
                                operation.time_zone(),
                            )?
                        } else {
                            fresh(draft.workspace, config, &zone)?
                        };
                        Ok(Outcome::OpenedDraft(Box::new(new)))
                    }
                    Action::Open(path) => {
                        let old = draft.ok_or("Open a workspace first")?;
                        let opened = Draft::open(old.workspace.clone(), &old.workspace, &path)?;
                        Ok(Outcome::OpenedDraft(Box::new(opened)))
                    }
                    Action::Save(path, overwrite) => {
                        let mut draft = draft.ok_or("No draft")?;
                        let name = anchored_name(&draft)?;
                        if !path
                            .file_name()
                            .is_some_and(|f| f.to_string_lossy().ends_with(&name))
                        {
                            return Err(format!("Filename must end with {name}"));
                        }
                        if path.exists() && !overwrite && draft.path.as_ref() != Some(&path) {
                            return Ok(Outcome::Collision(path));
                        }
                        if let Some(parent) = path.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                        }
                        draft.save(&path, false)?;
                        Ok(Outcome::Saved(Box::new(draft)))
                    }
                    Action::SaveConfig => {
                        let config = config.ok_or("No configuration")?;
                        mgb_ops::app::write_workspace_config(&config_path, &config)?;
                        Ok(Outcome::Config(config))
                    }
                    Action::Readiness => Ok(Outcome::Readiness(
                        draft.ok_or("No draft")?.validate_preparation(),
                    )),
                    Action::Accept => {
                        let mut draft = draft.ok_or("No draft")?;
                        draft.refresh_inputs()?;
                        let readiness = draft.validate_preparation();
                        Ok(Outcome::Accepted(Box::new(draft), readiness))
                    }
                    Action::Choices => {
                        Ok(Outcome::Choices(load_choices(&draft.ok_or("No draft")?)))
                    }
                    Action::OpenOperation { workspace, recipe } => {
                        let operation =
                            OperationalRun::open(workspace, &recipe, chrono::Utc::now())?;
                        Ok(Outcome::Operation(Box::new(operation), None))
                    }
                    Action::ValidateOperation => Ok(Outcome::OperationalReadiness(
                        operation.ok_or("Select an operation recipe")?.validate(),
                    )),
                    Action::AcceptOperation => {
                        let mut operation = operation.ok_or("Select an operation recipe")?;
                        operation.accept_inputs()?;
                        Ok(Outcome::Operation(Box::new(operation), Some(Vec::new())))
                    }
                    Action::RunAll | Action::RunOperation => {
                        let finished = Arc::new(AtomicBool::new(false));
                        let finished2 = finished.clone();
                        let progress_control = operation_control.clone();
                        let context2 = context.clone();
                        let live_control = operation_control.clone();
                        let live_logs = worker_logs.clone();
                        let observer = thread::spawn(move || {
                            while !finished2.load(Ordering::Acquire) {
                                if let Some(progress) = live_control.progress()
                                    && let Some(attempt) = progress.attempt
                                {
                                    let mut logs = String::new();
                                    read_logs(&attempt, 0, &mut logs);
                                    *live_logs.lock().unwrap_or_else(|e| e.into_inner()) = logs;
                                }
                                context2.request_repaint();
                                thread::sleep(std::time::Duration::from_millis(250));
                            }
                        });
                        let result = (|| -> Result<Outcome, String> {
                            match action {
                                Action::RunAll => mgb_ops::app::run_draft(
                                    &draft.ok_or("No draft")?,
                                    &operation_control,
                                )
                                .map(|r| Outcome::Completed(r.run_root)),
                                Action::RunOperation => operation
                                    .ok_or("Select an operation recipe")?
                                    .run_all(&operation_control)
                                    .map(|r| Outcome::OperationalCompleted(r.run_root)),
                                _ => unreachable!(),
                            }
                            .map_err(|e| e.to_string())
                        })();
                        finished.store(true, Ordering::Release);
                        let _ = observer.join();
                        if let Some(progress) = progress_control.progress()
                            && let Some(attempt) = progress.attempt
                        {
                            read_logs(&attempt, 0, &mut logs);
                        }
                        result
                    }
                }
            })();
            let _ = sender.send(Reply { result, logs });
            context.request_repaint();
        });
        self.active = Some(Active {
            control,
            automation,
            receiver,
            thread: Some(worker),
            logs: live_logs,
        });
    }
    pub fn poll(&mut self) -> Option<Outcome> {
        if let Some(active) = &self.active {
            if let Some(progress) = active.progress() {
                self.progress = Some(progress);
            }
            self.logs = active
                .logs
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
        }
        let reply = match self.active.as_ref()?.receiver.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Reply {
                result: Err("Worker stopped unexpectedly; workspace retained".into()),
                logs: String::new(),
            },
        };
        let mut active = self.active.take().unwrap();
        if let Some(thread) = active.thread.take()
            && thread.join().is_err()
        {
            self.cleanup_failed = true;
            self.status = "Worker panicked; inspect workspace before leaving".into();
            return None;
        }
        self.logs = reply.logs;
        match reply.result {
            Err(error) => {
                self.cleanup_failed = error.to_lowercase().contains("cleanup");
                self.status = error;
                None
            }
            Ok(outcome) => {
                self.status = "Completed".into();
                match outcome {
                    Outcome::Initialized(draft, config) => {
                        self.last_saved = draft.path.clone();
                        self.draft = Some(*draft);
                        self.baseline = Some(config.clone());
                        self.config = Some(config);
                        self.initialized = true;
                        None
                    }
                    Outcome::Accepted(draft, readiness) => {
                        self.readiness_checked = true;
                        self.draft = Some(*draft);
                        self.readiness = readiness;
                        None
                    }
                    Outcome::Draft(draft) => {
                        self.last_saved = draft.path.clone();
                        self.readiness.clear();
                        self.draft = Some(*draft);
                        None
                    }
                    Outcome::OpenedDraft(draft) => {
                        self.last_saved = draft.path.clone();
                        self.readiness.clear();
                        self.readiness_checked = false;
                        self.choices = Choices::default();
                        self.draft = Some(*draft);
                        Some(Outcome::DraftChanged)
                    }
                    Outcome::Registered(draft, report, origin_errors) => {
                        self.draft = Some(*draft);
                        Some(Outcome::RegistrationReport(report, origin_errors))
                    }
                    Outcome::Saved(draft) => {
                        self.last_saved = draft.path.clone();
                        self.draft = Some(*draft);
                        self.saved = true;
                        None
                    }
                    Outcome::Config(config) => {
                        self.baseline = Some(config);
                        self.saved = true;
                        None
                    }
                    Outcome::Choices(choices) => {
                        self.choices = choices;
                        None
                    }
                    Outcome::Collision(path) => {
                        self.overwrite = Some(path);
                        None
                    }
                    Outcome::Readiness(findings) => {
                        self.readiness_checked = true;
                        self.readiness = findings;
                        None
                    }
                    Outcome::Operation(operation, readiness) => {
                        self.operation = Some(*operation);
                        self.operational_readiness = readiness;
                        None
                    }
                    Outcome::OperationalReadiness(findings) => {
                        self.operational_readiness = Some(findings);
                        None
                    }
                    other => Some(other),
                }
            }
        }
    }
    pub fn shortcuts(&mut self, context: &egui::Context, switching_project: bool) {
        if context.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::S))
            && !self.busy()
            && !switching_project
        {
            match self.save_path() {
                Ok(path) => self.save(path, context),
                Err(error) => self.status = error,
            }
        }
    }
    pub fn save_path(&self) -> Result<PathBuf, String> {
        let draft = self.draft.as_ref().ok_or("No draft")?;
        let name = anchored_name(draft)?;
        if let Some(path) = &draft.path
            && path
                .file_name()
                .is_some_and(|f| f.to_string_lossy().ends_with(&name))
        {
            return Ok(path.clone());
        }
        Ok(draft.workspace.join("config/drafts").join(name))
    }
    pub fn save(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.start(Action::Save(path, false), ctx);
    }
    pub fn show(&mut self, ui: &mut egui::Ui, form: Form, pt: bool, workspace: Option<&Path>) {
        let ctx = ui.ctx().clone();
        if form == Form::Jobs {
            self.jobs(ui, pt);
            return;
        }
        ui.add_enabled_ui(!self.busy(), |ui| {
            egui::ScrollArea::vertical()
                .id_salt("runner-scroll")
                .show(ui, |ui| match form {
                    Form::Workspace => {
                        if let Some(config) = &mut self.config {
                            let original=config.clone();
                            ui.heading(form.label(pt));
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
                            restore_empty(&original.spatial,&mut config.spatial);
                            restore_empty(&original.engine,&mut config.engine);
                            ui.horizontal(|ui| {
                                if ui.button(text(pt, "Save", "Salvar")).clicked() {
                                    self.start(Action::SaveConfig, &ctx)
                                }
                                if ui.button(text(pt, "Revert", "Reverter")).clicked() {
                                    self.config = self.baseline.clone();
                                }
                            });
                        }
                    }
                    Form::NewRun => self.show_operation(ui, pt, workspace),
                    Form::RunAll => {
                        ui.heading(form.label(pt));
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(text(
                                    pt,
                                    "Accept/Refresh selected inputs",
                                    "Aceitar/Atualizar entradas selecionadas",
                                ))
                                .clicked()
                            {
                                self.start(Action::Accept, &ctx);
                            }
                            if ui.button(text(pt,"Check readiness","Verificar prontidão")).clicked(){self.start(Action::Readiness,&ctx);}
                            if ui
                                .button(text(
                                    pt,
                                    "Load existing instructions/assets",
                                    "Carregar instruções/ativos existentes",
                                ))
                                .clicked()
                            {
                                self.start(Action::Choices, &ctx);
                            }
                        });
                        for finding in &self.readiness{ui.colored_label(ui.visuals().warn_fg_color,finding);}
                        if self.readiness.is_empty(){ui.small(text(pt,"Use Check readiness to validate current selected inputs","Use Verificar prontidão para validar as entradas selecionadas"));}
                        if let Some(draft) = &mut self.draft {
                            let before=draft.document.operation.clone();
                            draft_form(ui, draft, &self.choices, pt);
                            if before!=draft.document.operation {self.readiness=vec![text(pt,"Settings changed; check readiness and explicitly accept selected inputs","Configurações alteradas; verifique a prontidão e aceite as entradas explicitamente")];}
                            ui.separator();
                            if ui
                                 .button(text(pt, "▶ Run experiment", "▶ Executar experimento"))
                                .clicked()
                            {
                                self.start(Action::RunAll, &ctx);
                            }
                        }
                    }
                    _ => {}
                });
        });
        ui.separator();
        ui.label(self.localized_status(pt));
    }
    fn show_operation(&mut self, ui: &mut egui::Ui, pt: bool, workspace: Option<&Path>) {
        ui.heading(Form::NewRun.label(pt));
        let Some(workspace) = workspace else {
            ui.label(text(pt, "Open a workspace", "Abra uma área de trabalho"));
            return;
        };
        if ui
            .button(text(
                pt,
                "Select operation TOML…",
                "Selecionar TOML da operação…",
            ))
            .clicked()
            && let Some(recipe) = rfd::FileDialog::new()
                .set_directory(workspace)
                .add_filter("Operation TOML", &["toml"])
                .pick_file()
        {
            self.start(
                Action::OpenOperation {
                    workspace: workspace.to_path_buf(),
                    recipe,
                },
                ui.ctx(),
            );
            return;
        }
        let Some(operation) = &mut self.operation else {
            return;
        };
        ui.label(operation.recipe_path().display().to_string());
        ui.label(format!(
            "{} ({})",
            text(pt, "Reference hour", "Hora de referência"),
            operation.time_zone()
        ));
        let mut reference = operation.reference().to_owned();
        if ui.text_edit_singleline(&mut reference).changed() {
            operation.set_reference(reference);
            self.operational_readiness = None;
        }
        let valid_coverage = match operation.coverage() {
            Ok([observed_first, observed_last, forecast_first, forecast_last]) => {
                ui.label(text(
                    pt,
                    "Observed hourly endpoints",
                    "Fins dos intervalos horários observados",
                ));
                ui.label(format!("{observed_first} → {observed_last}"));
                ui.label(text(
                    pt,
                    "Forecast hourly endpoints",
                    "Fins dos intervalos horários previstos",
                ));
                ui.label(format!("{forecast_first} → {forecast_last}"));
                true
            }
            Err(error) => {
                ui.colored_label(ui.visuals().warn_fg_color, error);
                false
            }
        };
        ui.label(text(pt, "Scenarios", "Cenários"));
        ui.label(operation.scenarios().join(", "));
        let accepted = operation.has_accepted_inputs();
        let mut action = None;
        ui.add_enabled_ui(valid_coverage, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(text(pt, "Check readiness", "Verificar prontidão"))
                    .clicked()
                {
                    action = Some(Action::ValidateOperation);
                }
                if ui
                    .button(text(
                        pt,
                        "Accept/Refresh selected inputs",
                        "Aceitar/Atualizar entradas selecionadas",
                    ))
                    .clicked()
                {
                    action = Some(Action::AcceptOperation);
                }
            });
        });
        if let Some(findings) = &self.operational_readiness {
            if findings.is_empty() {
                ui.label(text(
                    pt,
                    "Selected inputs are ready",
                    "As entradas selecionadas estão prontas",
                ));
            }
            for finding in findings {
                ui.colored_label(ui.visuals().warn_fg_color, finding);
            }
        }
        let ready = valid_coverage
            && accepted
            && self
                .operational_readiness
                .as_ref()
                .is_some_and(Vec::is_empty);
        ui.add_enabled_ui(ready, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button(text(pt, "Run All", "Executar tudo")).clicked() {
                    action = Some(Action::RunOperation);
                }
            });
        });
        if let Some(action) = action {
            self.start(action, ui.ctx());
        }
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
                mgb_ops::app::OperationStage::Preparation => text(pt, "Preparation", "Preparação"),
                mgb_ops::app::OperationStage::History => text(pt, "History", "Histórico"),
                mgb_ops::app::OperationStage::Scenarios => text(pt, "Scenarios", "Cenários"),
                mgb_ops::app::OperationStage::Finalization => {
                    text(pt, "Finalization", "Finalização")
                }
            });
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
        super::section(ui, text(pt, "Logs", "Registros"), |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.monospace(&self.logs);
            });
        });
    }
}
fn load_choices(draft: &Draft) -> Choices {
    let mut choices = Choices::default();
    let operation = &draft.document.operation;
    if let Some(path) = operation.observed.as_ref().and_then(|o| o.path.as_ref()) {
        match mgb_ops::store::ObservationStore::open_read_only(&draft.workspace.join(path))
            .and_then(|s| s.replacement_choices())
        {
            Ok(ids) => choices.replacements = ids,
            Err(error) => choices.findings.push(format!("Observations: {error}")),
        }
    }
    if let Some(path) = operation
        .forecasts
        .as_ref()
        .and_then(|f| f.catalog.as_ref())
    {
        match mgb_ops::store::ForecastCatalog::open_read_only(&draft.workspace, path) {
            Ok(catalog) => {
                match catalog.asset_choices() {
                    Ok(assets) => choices.assets = assets.into_iter().map(|a| a.asset_id).collect(),
                    Err(error) => choices.findings.push(format!("Forecast assets: {error}")),
                }
                match catalog.correction_choices() {
                    Ok(ids) => choices.corrections = ids.into_iter().map(|(id, _)| id).collect(),
                    Err(error) => choices
                        .findings
                        .push(format!("Forecast corrections: {error}")),
                }
            }
            Err(error) => choices.findings.push(format!("Forecast catalog: {error}")),
        }
    }
    choices
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
    ui.label(label);
    let mut input = value.clone().unwrap_or_default();
    if ui.text_edit_singleline(&mut input).changed() {
        *value = (!input.is_empty()).then_some(input);
    }
}
fn path(ui: &mut egui::Ui, label: &str, value: &mut Option<PathBuf>) {
    let mut input = value.as_ref().map(|p| p.to_string_lossy().to_string());
    string(ui, label, &mut input);
    *value = input.map(PathBuf::from);
}
fn number<T>(ui: &mut egui::Ui, label: &str, value: &mut Option<T>, pt: bool)
where
    T: egui::emath::Numeric + Default,
{
    ui.horizontal(|ui| {
        ui.label(label);
        let mut enabled = value.is_some();
        let response = ui.checkbox(&mut enabled, text(pt, "Set", "Definir"));
        let name = format!("{}: {label}", text(pt, "Set value", "Definir valor"));
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), enabled, &name)
        });
        if response.on_hover_text(name).changed() {
            *value = enabled.then(T::default);
        }
        if let Some(value) = value {
            ui.add(egui::DragValue::new(value));
        }
    });
}
fn list(ui: &mut egui::Ui, label: &str, value: &mut Option<Vec<String>>, pt: bool) {
    ui.label(label);
    explicit_selection(ui, label, value, pt);
    if let Some(ids) = value {
        let mut input = ids.join(", ");
        if ui.text_edit_singleline(&mut input).changed() {
            *ids = parse_list(&input);
        }
    }
}
fn parse_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .collect()
}
fn explicit_selection(ui: &mut egui::Ui, label: &str, value: &mut Option<Vec<String>>, pt: bool) {
    let mut specified = value.is_some();
    let name = format!(
        "{}: {label}",
        text(
            pt,
            "Specify selection (empty is allowed)",
            "Definir seleção (vazia é permitida)"
        )
    );
    if ui.checkbox(&mut specified, &name).changed() {
        *value = specified.then(Vec::new);
    }
}
fn ordered(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<Vec<String>>,
    choices: &[String],
    pt: bool,
) {
    ui.label(label);
    explicit_selection(ui, label, value, pt);
    let Some(ids) = value.as_mut() else {
        return;
    };
    let mut change = None;
    for (i, id) in ids.iter().enumerate() {
        ui.horizontal(|ui| {
            ui.label(id);
            if i > 0
                && super::icon_button(
                    ui,
                    "↑",
                    &format!("{} {id}", text(pt, "Move up", "Mover acima")),
                )
                .clicked()
            {
                change = Some((i, true));
            }
            if super::icon_button(
                ui,
                "×",
                &format!("{} {id}", text(pt, "Remove selection", "Remover seleção")),
            )
            .clicked()
            {
                change = Some((i, false));
            }
        });
    }
    if let Some((i, up)) = change {
        if up {
            ids.swap(i, i - 1)
        } else {
            ids.remove(i);
        }
    }
    egui::ComboBox::from_id_salt(label)
        .selected_text(text(pt, "+ Select", "+ Selecionar"))
        .show_ui(ui, |ui| {
            for id in choices {
                if !ids.contains(id) && ui.selectable_label(false, id).clicked() {
                    ids.push(id.clone());
                }
            }
        });
}
fn restore_empty<T: Default + PartialEq>(original: &Option<T>, current: &mut Option<T>) {
    if original.is_none() && current.as_ref().is_some_and(|v| *v == T::default()) {
        *current = None;
    }
}
pub fn draft_anchor(ui: &mut egui::Ui, draft: &Draft, pt: bool) {
    let operation = &draft.document.operation;
    for (label, value) in [
        (
            text(pt, "Reference · local time", "Referência · hora local"),
            operation.reference_time.as_deref(),
        ),
        (
            text(pt, "IANA timezone", "Fuso IANA"),
            operation.time_zone.as_deref(),
        ),
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.strong(label);
            ui.label(value.unwrap_or(if pt { "Indefinido" } else { "Unset" }));
        });
    }
}

fn draft_form(ui: &mut egui::Ui, draft: &mut Draft, choices: &Choices, pt: bool) {
    use mgb_ops::app::{DraftObserved, DraftScenario};
    let original = draft.document.operation.clone();
    super::section(ui, text(pt, "Period", "Período"), |ui| {
        draft_anchor(ui, draft, pt);
        let o = &mut draft.document.operation;
        if draft.document.schema_version == 2 {
            number(
                ui,
                &text(pt, "Observed days before", "Dias observados antes"),
                &mut o.observed_horizon_days,
                pt,
            );
            number(
                ui,
                &text(pt, "Forecast days after", "Dias previstos depois"),
                &mut o.forecast_horizon_days,
                pt,
            );
        }
        match o.coverage() {
            Ok(coverage) => {
                ui.small(coverage);
            }
            Err(e) => {
                ui.small(e);
            }
        }
    });
    let o = &mut draft.document.operation;
    super::section(ui, text(pt, "Observations", "Observações"), |ui| {
        let observed = o.observed.get_or_insert_with(DraftObserved::default);
        path(ui, "SQLite", &mut observed.path);
        list(
            ui,
            &text(
                pt,
                "Providers (comma separated)",
                "Provedores (separados por vírgula)",
            ),
            &mut observed.provider_codes,
            pt,
        );
        list(
            ui,
            &text(pt, "Station IDs", "IDs de estações"),
            &mut observed.station_ids,
            pt,
        );
        list(
            ui,
            &text(pt, "Series IDs", "IDs de séries"),
            &mut observed.series_ids,
            pt,
        );
        if draft.document.instructions.replacements.is_some() {
            ui.label(text(
                pt,
                "Edit staged replacements in Review → Instructions",
                "Edite substituições em Revisão → Instruções",
            ));
        } else {
            ordered(
                ui,
                &text(
                    pt,
                    "Existing replacement instructions · order matters",
                    "Substituições existentes · ordem importa",
                ),
                &mut observed.replacement_ids,
                &choices.replacements,
                pt,
            );
        }
    });
    super::section(
        ui,
        text(pt, "Forecast scenarios", "Cenários previstos"),
        |ui| {
            path(
                ui,
                &text(pt, "Forecast catalog", "Catálogo de previsões"),
                &mut o.forecasts.get_or_insert_default().catalog,
            );
            let mut remove = None;
            for (i, scenario) in o.scenarios.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.group(|ui| {
                        string(ui, "ID", &mut scenario.id);
                        egui::ComboBox::from_label(text(pt, "Rainfall", "Chuva"))
                            .selected_text(scenario.kind.as_deref().unwrap_or("—"))
                            .show_ui(ui, |ui| {
                                for kind in ["zero", "raw", "corrected"] {
                                    ui.selectable_value(
                                        &mut scenario.kind,
                                        Some(kind.into()),
                                        kind,
                                    );
                                }
                            });
                        egui::ComboBox::from_label(text(pt, "Forecast asset", "Ativo previsto"))
                            .selected_text(scenario.asset_id.as_deref().unwrap_or("—"))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut scenario.asset_id, None, "—");
                                for id in &choices.assets {
                                    ui.selectable_value(
                                        &mut scenario.asset_id,
                                        Some(id.clone()),
                                        id,
                                    );
                                }
                            });
                        if draft
                            .document
                            .instructions
                            .corrections
                            .get(i)
                            .is_some_and(Option::is_some)
                        {
                            ui.label(text(
                                pt,
                                "Edit staged corrections in Review → Instructions",
                                "Edite correções em Revisão → Instruções",
                            ));
                        } else {
                            ordered(
                                ui,
                                &text(
                                    pt,
                                    "Existing corrections · order matters",
                                    "Correções existentes · ordem importa",
                                ),
                                &mut scenario.correction_ids,
                                &choices.corrections,
                                pt,
                            );
                        }
                        if ui
                            .button(text(pt, "Remove scenario", "Remover cenário"))
                            .clicked()
                        {
                            remove = Some(i);
                        }
                    });
                });
            }
            if let Some(i) = remove {
                o.scenarios.remove(i);
                if i < draft.document.instructions.corrections.len() {
                    draft.document.instructions.corrections.remove(i);
                }
            }
            if ui.button(text(pt, "+ Scenario", "+ Cenário")).clicked() {
                o.scenarios.push(DraftScenario::default());
            }
        },
    );
    super::section(
        ui,
        text(pt, "Output stations", "Estações de saída"),
        |ui| {
            list(
                ui,
                &text(pt, "Station IDs", "IDs de estações"),
                &mut o.levels.get_or_insert_default().station_ids,
                pt,
            );
        },
    );
    super::section(
        ui,
        text(pt, "Spatial preparation", "Preparação espacial"),
        |ui| {
            path(
                ui,
                "MINI.gtp",
                &mut o.geometry.get_or_insert_default().mini_gtp,
            );
            let grid = o.grid.get_or_insert_default();
            ui.label(text(
                pt,
                "Grid extent [west, south, east, north]",
                "Extensão [oeste, sul, leste, norte]",
            ));
            let mut enabled = grid.bbox.is_some();
            if ui
                .checkbox(&mut enabled, text(pt, "Set extent", "Definir extensão"))
                .changed()
            {
                grid.bbox = enabled.then_some([0.; 4]);
            }
            if let Some(bbox) = &mut grid.bbox {
                ui.horizontal_wrapped(|ui| {
                    for coordinate in bbox {
                        ui.add(egui::DragValue::new(coordinate).speed(0.1));
                    }
                });
            }
            number(
                ui,
                &text(pt, "Resolution (degrees)", "Resolução (graus)"),
                &mut grid.resolution_degrees,
                pt,
            );
            let interpolation = o.interpolation.get_or_insert_default();
            number(
                ui,
                &text(pt, "Nearest stations", "Estações próximas"),
                &mut interpolation.nearest_stations,
                pt,
            );
            number(
                ui,
                &text(pt, "Distance power", "Potência da distância"),
                &mut interpolation.power,
                pt,
            );
        },
    );
    super::section(ui, text(pt, "QC settings", "Configurações QC"), |ui| {
        let qc = o.qc.get_or_insert_default();
        number(
            ui,
            &text(pt, "High rainfall (mm)", "Chuva alta (mm)"),
            &mut qc.high_threshold_mm,
            pt,
        );
        number(
            ui,
            &text(pt, "Sequence lower (mm)", "Limite inferior (mm)"),
            &mut qc.sequence_lower_mm,
            pt,
        );
        number(
            ui,
            &text(pt, "Sequence upper (mm)", "Limite superior (mm)"),
            &mut qc.sequence_upper_mm,
            pt,
        );
        number(
            ui,
            &text(
                pt,
                "Minimum sequence length",
                "Comprimento mínimo da sequência",
            ),
            &mut qc.sequence_min_length,
            pt,
        );
        ui.small(text(
            pt,
            "Findings never apply corrections automatically",
            "Avisos nunca aplicam correções automaticamente",
        ));
    });
    super::section(ui, text(pt, "Model", "Modelo"), |ui| {
        let engine = o.engine.get_or_insert_default();
        path(
            ui,
            &text(pt, "Executable", "Executável"),
            &mut engine.executable,
        );
        path(
            ui,
            &text(pt, "Baseline Input directory", "Diretório Input base"),
            &mut engine.input_dir,
        );
        number(
            ui,
            &text(pt, "Threads", "Threads"),
            &mut engine.omp_threads,
            pt,
        );
    });
    restore_empty(&original.observed, &mut o.observed);
    restore_empty(&original.geometry, &mut o.geometry);
    restore_empty(&original.grid, &mut o.grid);
    restore_empty(&original.interpolation, &mut o.interpolation);
    restore_empty(&original.qc, &mut o.qc);
    restore_empty(&original.levels, &mut o.levels);
    restore_empty(&original.engine, &mut o.engine);
    restore_empty(&original.forecasts, &mut o.forecasts);
    // Readiness is computed in the worker by explicit actions, avoiding catalog I/O per frame.
    for finding in &draft.findings {
        ui.small(finding);
    }
    ui.small(if draft.is_dirty() {
        text(pt, "Unsaved draft changes", "Alterações não salvas")
    } else {
        text(pt, "Draft unchanged", "Rascunho sem alterações")
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filenames_anchor_reference_and_distinguish_zones() {
        let root = PathBuf::from("/workspace/test");
        let mut draft = Draft::new_with_workspace_config(root.clone(), &root, None);
        draft.document.operation.reference_time = Some("2026-07-20T00:00:00".into());
        draft.document.operation.time_zone = Some("America/Sao_Paulo".into());
        assert_eq!(
            anchored_name(&draft).unwrap(),
            "operation-20260720T000000-0300.json"
        );
        {
            let unchanged = !(matches_reference(
                &draft,
                "2026-07-20T00:00:00",
                "America/Argentina/Buenos_Aires",
            ));
            assert!(unchanged);
        };
        {
            let unchanged =
                !(matches_reference(&draft, "2026-07-20T01:00:00", "America/Sao_Paulo"));
            assert!(unchanged);
        };
    }
    #[test]
    fn new_run_never_restores_saved_choices_and_hidden_work_is_dirty() {
        let config = WorkspaceConfig::default();
        let mut runner = Runners::default();
        let draft = fresh("/workspace/test".into(), &config, "America/Sao_Paulo").unwrap();
        assert!(draft.document.operation.scenarios.is_empty());
        assert!(draft.document.sources == Default::default());
        runner.draft = Some(draft);
        runner.config = Some(config.clone());
        runner.baseline = Some(config);
        runner
            .draft
            .as_mut()
            .unwrap()
            .document
            .operation
            .forecast_horizon_days = Some(5);
        assert!(runner.dirty());
        runner.discard();
        assert_eq!(
            runner
                .draft
                .as_ref()
                .unwrap()
                .document
                .operation
                .forecast_horizon_days,
            None
        );
    }
}

#[cfg(test)]
mod worker_tests {
    use super::*;
    fn wait(runner: &mut Runners) -> Option<Outcome> {
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let mut outcome = None;
        while runner.busy() {
            if let Some(value) = runner.poll() {
                outcome = Some(value);
            }
            assert!(std::time::Instant::now() < limit, "worker did not finish");
            thread::sleep(std::time::Duration::from_millis(2));
        }
        outcome
    }
    fn fixture() -> (tempfile::TempDir, Runners) {
        std::fs::create_dir_all("runs").unwrap();
        let temp = tempfile::tempdir_in("runs").unwrap();
        let root = temp.path().canonicalize().unwrap();
        let config = WorkspaceConfig {
            spatial: Some(mgb_ops::app::WorkspaceSpatial {
                gpkg_path: Some("geometry.gpkg".into()),
            }),
            engine: None,
        };
        let runner = Runners {
            draft: Some(fresh(root, &config, "UTC").unwrap()),
            config: Some(config.clone()),
            baseline: Some(config),
            ..Default::default()
        };
        (temp, runner)
    }
    #[test]
    fn committed_project_switch_blocks_keyboard_save_but_monitoring_refresh_does_not() {
        let (_temp, mut runner) = fixture();
        runner
            .draft
            .as_mut()
            .unwrap()
            .document
            .operation
            .observed_horizon_days = Some(0);
        let path = runner.save_path().unwrap();
        let context = egui::Context::default();
        let modifiers = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        let input = || egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::S,
                physical_key: Some(egui::Key::S),
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        };
        let mut output = context.run_ui(input(), |_| runner.shortcuts(&context, true));
        output.textures_delta.clear();
        assert!(runner.dirty());
        assert!(runner.active.is_none());
        let absent = !path.exists();
        assert!(absent);
        let mut output = context.run_ui(input(), |_| runner.shortcuts(&context, false));
        output.textures_delta.clear();
        wait(&mut runner);
        assert!(path.is_file());
        let clean = !runner.dirty();
        assert!(clean);
    }
    #[test]
    fn current_hour_handles_folds_and_half_hour_gaps_without_panicking() {
        let fold = chrono::DateTime::parse_from_rfc3339("2026-11-01T05:30:00Z")
            .unwrap()
            .to_utc();
        assert!(reference_at("America/New_York", fold).is_err());
        let gap = chrono::DateTime::parse_from_rfc3339("2026-10-03T15:45:00Z")
            .unwrap()
            .to_utc();
        assert!(reference_at("Australia/Lord_Howe", gap).is_err());
        assert_eq!(reference_at("UTC", fold).unwrap(), "2026-11-01T05:00:00");
    }
    #[test]
    fn selection_controls_preserve_unset_and_explicit_empty_without_render_edits() {
        let context = egui::Context::default();
        let mut unset = None;
        let mut empty = Some(Vec::new());
        let mut instructions = Some(Vec::new());
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            list(ui, "Unset stations", &mut unset, false);
            list(ui, "Empty stations", &mut empty, false);
            ordered(ui, "No replacements", &mut instructions, &[], false);
        });
        output.textures_delta.clear();
        assert_eq!(unset, None);
        assert_eq!(empty, Some(Vec::new()));
        assert_eq!(instructions, Some(Vec::new()));
        assert!(parse_list("").is_empty());
        let expected = vec!["a", "b"];
        assert_eq!(parse_list(" a, b, "), expected);
    }
    #[test]
    fn catalog_choices_survive_an_unrelated_source_failure() {
        let (_temp, mut runner) = fixture();
        let draft = runner.draft.as_mut().unwrap();
        let root = draft.workspace.clone();
        let observations = root.join("history.sqlite");
        let mut store = mgb_ops::store::ObservationStore::create(&observations, "UTC").unwrap();
        store
            .write_batch(&mgb_ops::store::ObservationBatch {
                stations: vec![mgb_ops::store::Station {
                    station_id: "ana:1".into(),
                    provider_code: "ana".into(),
                    station_code: "1".into(),
                    name: "Station".into(),
                    latitude: None,
                    longitude: None,
                }],
                ..Default::default()
            })
            .unwrap();
        store
            .register_replacements(vec![mgb_ops::store::ObservationReplacement {
                replacement_id: "existing".into(),
                station_id: "ana:1".into(),
                variable: "rain".into(),
                kind: mgb_ops::store::ReplacementKind::Point,
                start_at: super::super::data::timestamp("2026-01-01T00:00:00").unwrap(),
                end_at: None,
                value: Some(0.),
                reason: String::new(),
            }])
            .unwrap();
        draft.document.operation.observed = Some(mgb_ops::app::DraftObserved {
            path: Some(observations),
            ..Default::default()
        });
        draft.document.operation.forecasts = Some(mgb_ops::app::DraftForecasts {
            catalog: Some("absent.sqlite".into()),
        });
        let choices = load_choices(draft);
        let expected = vec!["existing"];
        assert_eq!(choices.replacements, expected);
        assert_eq!(choices.findings.len(), 1);
        assert!(choices.findings[0].starts_with("Forecast catalog:"));
        let catalog = mgb_ops::store::ForecastCatalog::open(
            &root,
            Path::new("forecasts.sqlite"),
            Path::new("assets"),
        )
        .unwrap();
        drop(catalog);
        draft.document.operation.observed.as_mut().unwrap().path = Some("absent.sqlite".into());
        draft.document.operation.forecasts.as_mut().unwrap().catalog =
            Some("forecasts.sqlite".into());
        let choices = load_choices(draft);
        assert_eq!(choices.findings.len(), 1);
        assert!(choices.findings[0].starts_with("Observations:"));
    }
    #[test]
    fn one_worker_failed_preparation_and_cancel_keep_draft_and_sources() {
        let (_temp, mut runner) = fixture();
        let ctx = egui::Context::default();
        let before = runner.draft.as_ref().unwrap().document.clone();
        runner.start(Action::RunAll, &ctx);
        runner.start(Action::RunAll, &ctx);
        assert!(runner.status.contains("already active"));
        runner.active.as_ref().unwrap().control.cancel();
        wait(&mut runner);
        assert_eq!(runner.draft.as_ref().unwrap().document, before);
        {
            let unchanged = !(runner.status.starts_with("Completed"));
            assert!(unchanged);
        };
        {
            let unchanged = !(runner.busy());
            assert!(unchanged);
        };
    }
    #[test]
    fn save_collisions_and_failed_open_preserve_dirty_work() {
        let (_temp, mut runner) = fixture();
        let ctx = egui::Context::default();
        runner
            .draft
            .as_mut()
            .unwrap()
            .document
            .operation
            .forecast_horizon_days = Some(5);
        let before = runner.draft.as_ref().unwrap().document.clone();
        let path = runner.save_path().unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"keep destination").unwrap();
        runner.save(path.clone(), &ctx);
        wait(&mut runner);
        assert_eq!(runner.overwrite.as_ref(), Some(&path));
        assert_eq!(std::fs::read(&path).unwrap(), b"keep destination");
        assert!(runner.dirty());
        runner.overwrite = None;
        runner.start(Action::Open(path), &ctx);
        wait(&mut runner);
        assert_eq!(runner.draft.as_ref().unwrap().document, before);
    }
    #[test]
    fn independent_nonproject_zone_restores_only_its_current_hour() {
        let (_temp, mut runner) = fixture();
        let root = runner.draft.as_ref().unwrap().workspace.clone();
        let config = root.join("workspace.toml");
        mgb_ops::app::write_workspace_config(&config, runner.config.as_ref().unwrap()).unwrap();
        let mut selected =
            fresh(root.clone(), runner.config.as_ref().unwrap(), "Asia/Tokyo").unwrap();
        selected
            .document
            .operation
            .scenarios
            .push(mgb_ops::app::DraftScenario {
                id: Some("remembered".into()),
                ..Default::default()
            });
        let path = root.join("misleading-0300.json");
        selected.save(&path, false).unwrap();
        runner.start(
            Action::Initialize {
                workspace: root,
                config,
                zone: "Asia/Tokyo".into(),
                remembered: Some(path),
            },
            &egui::Context::default(),
        );
        wait(&mut runner);
        assert_eq!(
            runner.draft.as_ref().unwrap().document.operation.scenarios[0]
                .id
                .as_deref(),
            Some("remembered")
        );
        assert_eq!(
            runner
                .draft
                .as_ref()
                .unwrap()
                .document
                .operation
                .time_zone
                .as_deref(),
            Some("Asia/Tokyo")
        );
    }
    #[test]
    fn stale_remembered_file_does_not_determine_new_reference() {
        let (_temp, mut runner) = fixture();
        let root = runner.draft.as_ref().unwrap().workspace.clone();
        let config = root.join("workspace.toml");
        mgb_ops::app::write_workspace_config(&config, runner.config.as_ref().unwrap()).unwrap();
        let mut old = Draft::new_run(
            root.clone(),
            &root,
            runner.config.as_ref(),
            "2020-01-01T00:00:00",
            "UTC",
        )
        .unwrap();
        old.document
            .operation
            .scenarios
            .push(mgb_ops::app::DraftScenario {
                id: Some("stale".into()),
                ..Default::default()
            });
        let remembered = root.join("stale.json");
        old.save(&remembered, false).unwrap();
        runner.start(
            Action::Initialize {
                workspace: root,
                config,
                zone: "UTC".into(),
                remembered: Some(remembered.clone()),
            },
            &egui::Context::default(),
        );
        wait(&mut runner);
        let draft = runner.draft.as_ref().unwrap();
        assert!(draft.document.operation.scenarios.is_empty());
        assert_eq!(
            draft.document.operation.reference_time.as_deref(),
            Some(reference_now("UTC").unwrap().as_str())
        );
        {
            let unchanged = !(draft.is_dirty());
            assert!(unchanged);
        };
        runner.start(Action::Open(remembered), &egui::Context::default());
        wait(&mut runner);
        let opened = runner.draft.as_ref().unwrap();
        assert_eq!(opened.document, old.document);
    }
    #[test]
    #[ignore = "explicit synthetic engine fixture; set MGB_DESKTOP_CASE to a disposable workspace under runs"]
    fn controlled_worker_matches_cli_and_requires_explicit_acceptance() {
        let root = PathBuf::from(std::env::var_os("MGB_DESKTOP_CASE").expect("fixture workspace"));
        let root = root.canonicalize().unwrap();
        let ctx = egui::Context::default();
        let mut runner = Runners::default();
        runner.config =
            Some(mgb_ops::app::read_workspace_config(&root.join("config/workspace.toml")).unwrap());
        runner.baseline = runner.config.clone();
        runner.draft =
            Some(Draft::open(root.clone(), &root, &root.join("unaccepted.json")).unwrap());
        runner.start(Action::RunAll, &ctx);
        wait(&mut runner);
        {
            let unchanged = !(runner.status.starts_with("Completed"));
            assert!(unchanged);
        };
        assert!(
            runner
                .draft
                .as_ref()
                .unwrap()
                .document
                .sources
                .accepted
                .is_empty()
        );
        let operation = &mut runner.draft.as_mut().unwrap().document.operation;
        operation.observed.as_mut().unwrap().station_ids = Some(parse_list(""));
        operation.observed.as_mut().unwrap().replacement_ids = Some(Vec::new());
        operation.levels.as_mut().unwrap().station_ids = Some(parse_list(""));
        runner.start(Action::Accept, &ctx);
        wait(&mut runner);
        assert!(runner.status.starts_with("Completed"), "{}", runner.status);
        runner.start(Action::RunAll, &ctx);
        let completed = matches!(wait(&mut runner), Some(Outcome::Completed(_)));
        assert!(completed, "{}", runner.status);
    }
}
