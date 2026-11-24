use super::{layout::Form, text};
use chrono::NaiveDateTime;
use eframe::egui;
use mgb_ops::analysis::ReviewVariable;
use mgb_ops::app::{
    AutomationControl, AutomationJobStatus, BulletinPreview, BulletinRequest, CanonicalForcingKind,
    Draft, ForecastPreview, HydrographReport, ObservationPreview, ObservationReviewReport,
    ObservationReviewRequest, OperationControl, ReviewWindow, SpatialObservationQcReport,
};
use mgb_ops::store::{GridFrame, IdwSettings, ObservationSelection};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Intake {
    #[default]
    Observations,
    Inventory,
    Forecast,
    Reservoirs,
    Wind,
}

impl Intake {
    fn label(self, pt: bool) -> String {
        match self {
            Self::Observations => text(pt, "Observations", "Observações"),
            Self::Inventory => text(pt, "Station inventory", "Inventário de estações"),
            Self::Forecast => text(pt, "Forecast grid", "Grade de previsão"),
            Self::Reservoirs => text(pt, "Reservoir schedules", "Programação de reservatórios"),
            Self::Wind => text(pt, "Wind", "Vento"),
        }
    }
}

#[derive(Clone)]
pub enum Task {
    Init(PathBuf, String),
    Upgrade(PathBuf),
    ObservationBatch(PathBuf, PathBuf),
    Inventory(PathBuf, PathBuf),
    Forecast(PathBuf),
    Forcing(PathBuf, PathBuf, CanonicalForcingKind),
    Availability(ObservationReviewRequest),
    Spatial(ObservationReviewRequest, Option<String>, IdwSettings),
    Export(PathBuf, Option<PathBuf>, PathBuf),
    Import(PathBuf, PathBuf),
    Regenerate(PathBuf, PathBuf),
    Analyze(PathBuf, Option<PathBuf>),
    Report(BulletinRequest, bool),
    ReturnPeriod(PathBuf),
    Automation(AutomationTask),
}

#[derive(Clone)]
pub enum AutomationTask {
    Load(PathBuf),
    Save(PathBuf, String),
    Status,
    Serve(PathBuf),
    RunNow(PathBuf, String),
    Email {
        config: PathBuf,
        subject: String,
        body: String,
        attachment: PathBuf,
    },
}

pub enum Output {
    Notice(String),
    Observations(ObservationPreview),
    Forecast(ForecastPreview),
    Availability(ObservationReviewReport),
    Spatial(SpatialObservationQcReport),
    Analysis(HydrographReport),
    Bulletin(BulletinPreview),
    JobConfiguration(String),
    Jobs(Vec<AutomationJobStatus>),
}

pub fn json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_reader(file).map_err(|e| e.to_string())
}

fn workspace_path(workspace: &Path, path: &Path) -> PathBuf {
    workspace.join(path)
}

impl Task {
    pub fn needs_automation_control(&self) -> bool {
        matches!(
            self,
            Self::Automation(AutomationTask::Serve(_) | AutomationTask::RunNow(_, _))
        )
    }

    pub fn run(
        self,
        workspace: &Path,
        control: &OperationControl,
        automation: Option<&AutomationControl>,
    ) -> Result<Output, String> {
        use mgb_ops::store::ObservationStore;
        let resolve = |path: &Path| workspace_path(workspace, path);
        let notice = match self {
            Self::Init(path, zone) => {
                ObservationStore::create(&resolve(&path), &zone).map_err(|e| e.to_string())?;
                resolve(&path).display().to_string()
            }
            Self::Upgrade(path) => {
                ObservationStore::open_writable(&resolve(&path))
                    .and_then(|mut db| db.upgrade_replacements())
                    .map_err(|e| e.to_string())?;
                resolve(&path).display().to_string()
            }
            Self::ObservationBatch(database, source) => {
                let batch: mgb_ops::store::ObservationBatch = json(&resolve(&source))?;
                ObservationStore::open_writable(&resolve(&database))
                    .and_then(|mut db| db.ingest_batch(&batch))
                    .map_err(|e| e.to_string())?;
                resolve(&database).display().to_string()
            }
            Self::Inventory(database, source) => {
                let stations: Vec<mgb_ops::store::Station> = json(&resolve(&source))?;
                ObservationStore::open_writable(&resolve(&database))
                    .and_then(|mut db| db.update_station_inventory(&stations))
                    .map_err(|e| e.to_string())?;
                resolve(&database).display().to_string()
            }
            Self::Forecast(request) => {
                mgb_ops::app::import_canonical_forecast(workspace, json(&resolve(&request))?)
                    .map_err(|e| e.to_string())?
                    .asset_id
            }
            Self::Forcing(source, destination, kind) => {
                let path = resolve(&destination);
                mgb_ops::app::save_canonical_forcing(&resolve(&source), &path, kind)?;
                path.display().to_string()
            }
            Self::Availability(mut request) => {
                request.database = resolve(&request.database);
                return mgb_ops::app::review_observations(&request)
                    .map(Output::Availability)
                    .map_err(|e| e.to_string());
            }
            Self::Spatial(mut request, station, idw) => {
                request.database = resolve(&request.database);
                return mgb_ops::app::spatial_observation_qc(&request, station.as_deref(), &idw)
                    .map(Output::Spatial)
                    .map_err(|e| e.to_string());
            }
            Self::Export(run, preparation, destination) => {
                let preparation = preparation.as_deref().map(resolve);
                mgb_ops::app::export_run(&run, preparation.as_deref(), &resolve(&destination))?
                    .root
                    .display()
                    .to_string()
            }
            Self::Import(bundle, destination) => {
                mgb_ops::app::import_run_to(workspace, &resolve(&bundle), &resolve(&destination))?
                    .root
                    .display()
                    .to_string()
            }
            Self::Regenerate(bundle, engine) => mgb_ops::app::regenerate_run(
                workspace,
                &resolve(&bundle),
                &resolve(&engine),
                control,
            )
            .map_err(|e| e.to_string())?
            .run_root
            .display()
            .to_string(),
            Self::Analyze(path, output) => {
                let request = json(&resolve(&path))?;
                let report = mgb_ops::app::analyze_hydrograph(&request)?;
                if let Some(path) = output {
                    mgb_ops::app::export_hydrograph(&report, &resolve(&path))?;
                }
                return Ok(Output::Analysis(report));
            }
            Self::Report(mut request, export) => {
                request.observations = resolve(&request.observations);
                request.geometry = resolve(&request.geometry);
                request.map_geometry = request.map_geometry.as_deref().map(resolve);
                request.profile = resolve(&request.profile);
                request.output = resolve(&request.output);
                if export {
                    mgb_ops::app::generate_bulletin(&request)?
                        .output
                        .display()
                        .to_string()
                } else {
                    return mgb_ops::app::preview_bulletin(&request).map(Output::Bulletin);
                }
            }
            Self::ReturnPeriod(path) => {
                mgb_ops::app::export_return_period(&json(&resolve(&path))?)?
                    .destination
                    .display()
                    .to_string()
            }
            Self::Automation(task) => return task.run(workspace, automation),
        };
        Ok(Output::Notice(notice))
    }
}

impl AutomationTask {
    fn run(self, workspace: &Path, control: Option<&AutomationControl>) -> Result<Output, String> {
        use mgb_ops::app::{
            automation_status, publish_automation_artifacts, read_automation_config,
            run_automation, run_automation_now, save_automation_config, send_automation_email,
        };
        match self {
            Self::Load(path) => std::fs::read_to_string(workspace.join(path))
                .map(Output::JobConfiguration)
                .map_err(|e| e.to_string()),
            Self::Save(path, text) => {
                save_automation_config(workspace, &path, &text)?;
                Ok(Output::Notice(workspace.join(path).display().to_string()))
            }
            Self::Status => automation_status(workspace).map(Output::Jobs),
            Self::Serve(path) => {
                let config = read_automation_config(workspace, &path)?;
                run_automation(
                    workspace,
                    &config,
                    control.ok_or("Missing scheduler control")?,
                    publish_automation_artifacts,
                )?;
                automation_status(workspace).map(Output::Jobs)
            }
            Self::RunNow(path, job) => {
                let config = read_automation_config(workspace, &path)?;
                let status = run_automation_now(
                    workspace,
                    &config,
                    &job,
                    control.ok_or("Missing scheduler control")?,
                    publish_automation_artifacts,
                )?;
                Ok(Output::Jobs(vec![status]))
            }
            Self::Email {
                config,
                subject,
                body,
                attachment,
            } => {
                let config = read_automation_config(workspace, &config)?;
                let smtp = config
                    .smtp
                    .as_ref()
                    .ok_or("Automation configuration requires [smtp]")?;
                let attachments: Vec<_> = (!attachment.as_os_str().is_empty())
                    .then_some(attachment)
                    .into_iter()
                    .collect();
                send_automation_email(workspace, smtp, &subject, &body, &attachments)?;
                Ok(Output::Notice(String::new()))
            }
        }
    }
}

#[derive(Default)]
pub struct Forms {
    intake: Intake,
    input: PathBuf,
    destination: PathBuf,
    database: PathBuf,
    station_id: String,
    end: String,
    variable: Option<ReviewVariable>,
    monthly: bool,
    bundle: PathBuf,
    import_parent: PathBuf,
    import_name: String,
    export: PathBuf,
    preparation: PathBuf,
    engine: PathBuf,
    preview_step: usize,
    preview: Option<Output>,
    notice: String,
    forcing_text: String,
    selection_notice: String,
    analysis_request: PathBuf,
    analysis_output: PathBuf,
    analysis: Option<HydrographReport>,
    bulletin_profile: PathBuf,
    bulletin_geometry: PathBuf,
    bulletin_map: PathBuf,
    bulletin_output: PathBuf,
    bulletin_scenario: String,
    bulletin: Option<BulletinPreview>,
    bulletin_textures: Vec<egui::TextureHandle>,
    gis_request: PathBuf,
    job_path: PathBuf,
    job_text: String,
    job_id: String,
    jobs: Vec<AutomationJobStatus>,
    email_subject: String,
    email_body: String,
    email_attachment: PathBuf,
}

pub fn path_field(ui: &mut egui::Ui, label: String, path: &mut PathBuf, directory: bool, pt: bool) {
    ui.label(label);
    ui.horizontal(|ui| {
        let mut value = path.display().to_string();
        if ui.text_edit_singleline(&mut value).changed() {
            *path = value.into();
        }
        if ui.button(text(pt, "Browse…", "Procurar…")).clicked() {
            let dialog = rfd::FileDialog::new();
            let selected = if directory {
                dialog.pick_folder()
            } else {
                dialog.pick_file()
            };
            if let Some(selected) = selected {
                *path = selected;
            }
        }
    });
}

fn optional_path(
    ui: &mut egui::Ui,
    label: String,
    target: &mut Option<PathBuf>,
    directory: bool,
    pt: bool,
) {
    let mut path = target.clone().unwrap_or_default();
    path_field(ui, label, &mut path, directory, pt);
    *target = (!path.as_os_str().is_empty()).then_some(path);
}

impl Forms {
    pub fn receive(&mut self, response: Output, context: &egui::Context) {
        match response {
            Output::Notice(text) => {
                self.notice = text;
            }
            Output::Analysis(report) => {
                self.analysis = Some(report);
            }
            Output::Bulletin(report) => {
                match report
                    .content
                    .figures
                    .iter()
                    .map(|figure| {
                        png_image(&figure.png).map(|image| {
                            context.load_texture(
                                &figure.filename,
                                image,
                                egui::TextureOptions::LINEAR,
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(images) => self.bulletin_textures = images,
                    Err(error) => {
                        self.notice = error;
                        self.bulletin_textures.clear();
                    }
                }
                self.bulletin = Some(report);
            }
            Output::Jobs(jobs) => {
                self.jobs = jobs;
            }
            Output::JobConfiguration(text) => {
                self.job_text = text;
            }
            preview => {
                self.preview_step = 0;
                self.preview = Some(preview);
            }
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        form: Form,
        pt: bool,
        draft: &Draft,
        run: Option<&super::data::Run>,
    ) -> Result<Option<Task>, String> {
        ui.heading(form.label(pt));
        let default_db = draft
            .document
            .operation
            .observed
            .as_ref()
            .and_then(|s| s.path.as_ref())
            .cloned()
            .unwrap_or_else(|| "data/history.sqlite".into());
        if self.database.as_os_str().is_empty() {
            self.database = default_db;
        }
        let task = match form {
            Form::Intake => self.intake(ui, pt, draft),
            Form::Review => self.review(ui, pt, draft)?,
            Form::RunBundle => self.share(ui, pt, run.map(|run| run.root.as_path())),
            Form::Analysis => self.analysis(ui, pt),
            Form::Bulletin => self.bulletin(ui, pt, draft, run),
            Form::Automation => self.automation(ui, pt),
            _ => None,
        };
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        if form == Form::Review {
            self.show_preview(ui, pt, draft);
        }
        Ok(task)
    }

    fn analysis(&mut self, ui: &mut egui::Ui, pt: bool) -> Option<Task> {
        path_field(
            ui,
            text(
                pt,
                "Hydrograph request JSON",
                "Solicitação de hidrograma JSON",
            ),
            &mut self.analysis_request,
            false,
            pt,
        );
        destination(
            ui,
            text(
                pt,
                "CSV or PNG output (optional)",
                "Saída CSV ou PNG (opcional)",
            ),
            &mut self.analysis_output,
        );
        if ui
            .button(text(
                pt,
                "Analyze selected series",
                "Analisar séries selecionadas",
            ))
            .clicked()
        {
            let output = (!self.analysis_output.as_os_str().is_empty())
                .then(|| self.analysis_output.clone());
            return Some(Task::Analyze(self.analysis_request.clone(), output));
        }
        if let Some(report) = &self.analysis {
            hydrograph(ui, report);
            for curve in &report.curves {
                let label = [
                    curve.run_id.as_deref(),
                    curve.scenario_id.as_deref(),
                    Some(curve.variable.as_str()),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                let peak = curve
                    .peak
                    .as_ref()
                    .map(|point| format!("{} · {:.2} {}", point.timestamp, point.value, curve.unit))
                    .unwrap_or_else(|| "—".into());
                ui.label(format!("{label} · {peak}"));
            }
            for finding in &report.findings {
                ui.label(&finding.message);
            }
        }
        None
    }

    fn bulletin(
        &mut self,
        ui: &mut egui::Ui,
        pt: bool,
        draft: &Draft,
        run: Option<&super::data::Run>,
    ) -> Option<Task> {
        if let Some(run) = run {
            ui.label(run.root.display().to_string());
            if !run.scenarios.contains(&self.bulletin_scenario) {
                self.bulletin_scenario = run.scenarios.first().cloned().unwrap_or_default();
            }
            egui::ComboBox::from_id_salt("bulletin-scenario")
                .selected_text(&self.bulletin_scenario)
                .show_ui(ui, |ui| {
                    for scenario in &run.scenarios {
                        ui.selectable_value(
                            &mut self.bulletin_scenario,
                            scenario.clone(),
                            scenario,
                        );
                    }
                });
            path_field(
                ui,
                text(
                    pt,
                    "Regional bulletin profile",
                    "Perfil do boletim regional",
                ),
                &mut self.bulletin_profile,
                false,
                pt,
            );
            path_field(
                ui,
                text(pt, "Observation database", "Banco de observações"),
                &mut self.database,
                false,
                pt,
            );
            if self.bulletin_geometry.as_os_str().is_empty() {
                self.bulletin_geometry = draft
                    .document
                    .operation
                    .geometry
                    .as_ref()
                    .and_then(|item| item.mini_gtp.clone())
                    .unwrap_or_default();
            }
            path_field(
                ui,
                text(pt, "MINI topology", "Topologia MINI"),
                &mut self.bulletin_geometry,
                false,
                pt,
            );
            path_field(
                ui,
                text(
                    pt,
                    "Catchment geometry (optional)",
                    "Geometria das bacias (opcional)",
                ),
                &mut self.bulletin_map,
                false,
                pt,
            );
            destination(
                ui,
                text(pt, "New bulletin directory", "Novo diretório do boletim"),
                &mut self.bulletin_output,
            );
            let preview = ui
                .button(text(pt, "Preview bulletin", "Prévia do boletim"))
                .clicked();
            let export = ui
                .button(text(pt, "Export bulletin", "Exportar boletim"))
                .clicked();
            if preview || export {
                return Some(Task::Report(
                    BulletinRequest {
                        trial: run.root.clone(),
                        scenario_id: self.bulletin_scenario.clone(),
                        observations: self.database.clone(),
                        geometry: self.bulletin_geometry.clone(),
                        map_geometry: (!self.bulletin_map.as_os_str().is_empty())
                            .then(|| self.bulletin_map.clone()),
                        profile: self.bulletin_profile.clone(),
                        output: self.bulletin_output.clone(),
                    },
                    export,
                ));
            }
        } else {
            ui.label(text(
                pt,
                "Select a retained run",
                "Selecione uma execução retida",
            ));
        }
        if let Some(report) = &self.bulletin {
            ui.separator();
            ui.heading(&report.content.title);
            for detail in &report.content.details {
                ui.label(detail);
            }
            for table in &report.content.tables {
                ui.collapsing(&table.title, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt(&table.title)
                        .show(ui, |ui| {
                            egui::Grid::new(&table.title).striped(true).show(ui, |ui| {
                                for column in &table.columns {
                                    ui.strong(column);
                                }
                                ui.end_row();
                                for row in &table.rows {
                                    for value in row {
                                        ui.label(value);
                                    }
                                    ui.end_row();
                                }
                            });
                        });
                });
            }
            for (figure, texture) in report.content.figures.iter().zip(&self.bulletin_textures) {
                ui.collapsing(&figure.title, |ui| {
                    let size = texture.size_vec2();
                    let width = ui.available_width().min(size.x);
                    ui.add(
                        egui::Image::new(texture)
                            .fit_to_exact_size(egui::vec2(width, size.y * width / size.x)),
                    );
                });
            }
            for note in &report.content.notes {
                ui.label(note);
            }
        }
        ui.separator();
        path_field(
            ui,
            text(
                pt,
                "Return-period map request JSON",
                "Solicitação de mapa de período de retorno JSON",
            ),
            &mut self.gis_request,
            false,
            pt,
        );
        ui.button(text(
            pt,
            "Export return-period map",
            "Exportar mapa de período de retorno",
        ))
        .clicked()
        .then(|| Task::ReturnPeriod(self.gis_request.clone()))
    }

    fn automation(&mut self, ui: &mut egui::Ui, pt: bool) -> Option<Task> {
        if self.job_path.as_os_str().is_empty() {
            self.job_path = "config/automation.toml".into();
        }
        if self.job_text.is_empty() {
            self.job_text = "schema_version = 1\n".into();
        }
        path_field(
            ui,
            text(pt, "Local automation TOML", "TOML local de automação"),
            &mut self.job_path,
            false,
            pt,
        );
        ui.add(
            egui::TextEdit::multiline(&mut self.job_text)
                .desired_width(f32::INFINITY)
                .desired_rows(12)
                .code_editor(),
        );
        let action = ui
            .horizontal_wrapped(|ui| {
                if ui.button(text(pt, "Load", "Carregar")).clicked() {
                    Some(AutomationTask::Load(self.job_path.clone()))
                } else if ui
                    .button(text(
                        pt,
                        "Save validated settings",
                        "Salvar configurações validadas",
                    ))
                    .clicked()
                {
                    Some(AutomationTask::Save(
                        self.job_path.clone(),
                        self.job_text.clone(),
                    ))
                } else if ui
                    .button(text(pt, "Start scheduler", "Iniciar agendador"))
                    .clicked()
                {
                    Some(AutomationTask::Serve(self.job_path.clone()))
                } else if ui
                    .button(text(pt, "Refresh status", "Atualizar estado"))
                    .clicked()
                {
                    Some(AutomationTask::Status)
                } else {
                    None
                }
            })
            .inner;
        if let Some(action) = action {
            return Some(Task::Automation(action));
        }
        ui.label(text(pt, "Job ID", "ID da tarefa"));
        ui.text_edit_singleline(&mut self.job_id);
        if ui.button(text(pt, "Run now", "Executar agora")).clicked() {
            return Some(Task::Automation(AutomationTask::RunNow(
                self.job_path.clone(),
                self.job_id.clone(),
            )));
        }
        for job in self.jobs.iter().rev().take(30) {
            ui.collapsing(
                format!("{} · {} · {:?}", job.job_id, job.slot, job.state),
                |ui| {
                    for stage in &job.stages {
                        ui.label(format!("{:?} · {:?}", stage.stage, stage.state));
                        for artifact in &stage.artifacts {
                            ui.label(artifact.display().to_string());
                        }
                        if let Some(error) = &stage.error {
                            ui.label(error);
                        }
                    }
                    if let Some(error) = &job.error {
                        ui.label(error);
                    }
                },
            );
        }
        ui.separator();
        ui.strong(text(pt, "Email delivery", "Envio por e-mail"));
        ui.label(text(pt, "Subject", "Assunto"));
        ui.text_edit_singleline(&mut self.email_subject);
        ui.label(text(pt, "Message", "Mensagem"));
        ui.text_edit_multiline(&mut self.email_body);
        path_field(
            ui,
            text(pt, "Attachment (optional)", "Anexo (opcional)"),
            &mut self.email_attachment,
            false,
            pt,
        );
        ui.add_enabled(
            !self.email_subject.trim().is_empty(),
            egui::Button::new(text(
                pt,
                "Send configured email",
                "Enviar e-mail configurado",
            )),
        )
        .clicked()
        .then(|| {
            Task::Automation(AutomationTask::Email {
                config: self.job_path.clone(),
                subject: self.email_subject.clone(),
                body: self.email_body.clone(),
                attachment: self.email_attachment.clone(),
            })
        })
    }

    fn intake(&mut self, ui: &mut egui::Ui, pt: bool, draft: &Draft) -> Option<Task> {
        egui::ComboBox::from_id_salt("canonical-input-kind")
            .selected_text(self.intake.label(pt))
            .show_ui(ui, |ui| {
                for kind in [
                    Intake::Observations,
                    Intake::Inventory,
                    Intake::Forecast,
                    Intake::Reservoirs,
                    Intake::Wind,
                ] {
                    ui.selectable_value(&mut self.intake, kind, kind.label(pt));
                }
            });
        path_field(
            ui,
            text(
                pt,
                "Canonical input or forecast request JSON",
                "Entrada canônica ou solicitação de previsão JSON",
            ),
            &mut self.input,
            false,
            pt,
        );
        if matches!(self.intake, Intake::Observations | Intake::Inventory) {
            path_field(
                ui,
                text(pt, "Observation database", "Banco de observações"),
                &mut self.database,
                false,
                pt,
            );
            let zone = draft
                .document
                .operation
                .time_zone
                .as_deref()
                .unwrap_or("America/Sao_Paulo");
            if ui
                .button(text(pt, "Initialize database", "Inicializar banco"))
                .clicked()
            {
                return Some(Task::Init(self.database.clone(), zone.into()));
            }
            if ui
                .button(text(
                    pt,
                    "Enable flow replacements",
                    "Habilitar correções de vazão",
                ))
                .clicked()
            {
                return Some(Task::Upgrade(self.database.clone()));
            }
        }
        if matches!(self.intake, Intake::Reservoirs | Intake::Wind) {
            ui.label(text(pt, "New canonical file", "Novo arquivo canônico"));
            let mut path = self.destination.display().to_string();
            if ui.text_edit_singleline(&mut path).changed() {
                self.destination = path.into();
            }
        }
        ui.add_enabled(
            !self.input.as_os_str().is_empty(),
            egui::Button::new(text(
                pt,
                "Import selected data",
                "Importar dados selecionados",
            )),
        )
        .clicked()
        .then(|| match self.intake {
            Intake::Observations => {
                Task::ObservationBatch(self.database.clone(), self.input.clone())
            }
            Intake::Inventory => Task::Inventory(self.database.clone(), self.input.clone()),
            Intake::Forecast => Task::Forecast(self.input.clone()),
            Intake::Reservoirs => Task::Forcing(
                self.input.clone(),
                self.destination.clone(),
                CanonicalForcingKind::Reservoirs,
            ),
            Intake::Wind => Task::Forcing(
                self.input.clone(),
                self.destination.clone(),
                CanonicalForcingKind::Wind,
            ),
        })
    }

    fn review(
        &mut self,
        ui: &mut egui::Ui,
        pt: bool,
        draft: &Draft,
    ) -> Result<Option<Task>, String> {
        let operation = &draft.document.operation;
        path_field(
            ui,
            text(pt, "Observation database", "Banco de observações"),
            &mut self.database,
            false,
            pt,
        );
        if self.end.is_empty() {
            self.end = operation.reference_time.clone().unwrap_or_default();
        }
        ui.label(text(
            pt,
            "Window end (local hour)",
            "Fim do período (hora local)",
        ));
        ui.text_edit_singleline(&mut self.end);
        ui.label(text(
            pt,
            "Station ID (optional)",
            "ID da estação (opcional)",
        ));
        ui.text_edit_singleline(&mut self.station_id);
        let variable = self.variable.get_or_insert(ReviewVariable::Rain);
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(variable, ReviewVariable::Rain, text(pt, "Rain", "Chuva"));
            ui.selectable_value(variable, ReviewVariable::Flow, text(pt, "Flow", "Vazão"));
            ui.selectable_value(variable, ReviewVariable::Level, text(pt, "Level", "Nível"));
        });
        ui.checkbox(&mut self.monthly, text(pt, "Calendar month", "Mês civil"));
        let available = ui
            .button(text(pt, "Availability", "Disponibilidade"))
            .clicked();
        let spatial = ui
            .add_enabled(
                *variable == ReviewVariable::Rain,
                egui::Button::new(text(
                    pt,
                    "Spatial rainfall check",
                    "Consistência espacial da chuva",
                )),
            )
            .clicked();
        if !available && !spatial {
            return Ok(None);
        }
        let station =
            (!self.station_id.trim().is_empty()).then(|| self.station_id.trim().to_string());
        let observed = operation
            .observed
            .as_ref()
            .ok_or("Set the observation sources in the draft")?;
        let selection = match (&station, spatial) {
            (Some(station), false) => ObservationSelection {
                provider_codes: vec![],
                station_ids: vec![station.clone()],
            },
            _ => ObservationSelection {
                provider_codes: observed.provider_codes.clone().unwrap_or_default(),
                station_ids: observed.station_ids.clone().unwrap_or_default(),
            },
        };
        let request = ObservationReviewRequest {
            database: self.database.clone(),
            selection,
            series_ids: Vec::new(),
            variable: *variable,
            end: self
                .end
                .parse::<NaiveDateTime>()
                .map_err(|e| e.to_string())?,
            window: if self.monthly {
                ReviewWindow::Month
            } else {
                ReviewWindow::Recent72Hours
            },
            replacement_ids: observed.replacement_ids.clone().unwrap_or_default(),
            replacements: draft
                .document
                .instructions
                .replacements
                .as_ref()
                .map(|values| values.iter().map(|item| item.value.clone()).collect()),
        };
        Ok(Some(if available {
            Task::Availability(request)
        } else {
            let options = operation.interpolation.as_ref();
            Task::Spatial(
                request,
                station,
                IdwSettings {
                    nearest_stations: options.and_then(|o| o.nearest_stations).unwrap_or(5),
                    power: options.and_then(|o| o.power).unwrap_or(2.0),
                },
            )
        }))
    }

    fn share(&mut self, ui: &mut egui::Ui, pt: bool, run: Option<&Path>) -> Option<Task> {
        path_field(
            ui,
            text(pt, "Run bundle", "Pacote de execução"),
            &mut self.bundle,
            true,
            pt,
        );
        path_field(
            ui,
            text(
                pt,
                "Import location (outside managed storage)",
                "Local de importação (fora do armazenamento gerenciado)",
            ),
            &mut self.import_parent,
            true,
            pt,
        );
        ui.horizontal(|ui| {
            let label = ui.label(text(pt, "New folder name", "Nome da nova pasta"));
            ui.text_edit_singleline(&mut self.import_name)
                .labelled_by(label.id);
        });
        let name = Path::new(self.import_name.trim());
        let named_folder = matches!(
            (name.components().next(), name.components().nth(1)),
            (Some(std::path::Component::Normal(_)), None)
        );
        if ui
            .add_enabled(
                !self.bundle.as_os_str().is_empty()
                    && self.import_parent.is_absolute()
                    && named_folder,
                egui::Button::new(text(pt, "Import bundle", "Importar pacote")),
            )
            .clicked()
        {
            return Some(Task::Import(
                self.bundle.clone(),
                self.import_parent.join(name),
            ));
        }
        path_field(
            ui,
            text(
                pt,
                "Local matching engine",
                "Executável local correspondente",
            ),
            &mut self.engine,
            false,
            pt,
        );
        if ui
            .button(text(pt, "Regenerate bundle", "Regenerar pacote"))
            .clicked()
        {
            return Some(Task::Regenerate(self.bundle.clone(), self.engine.clone()));
        }
        let run = run?;
        ui.separator();
        ui.label(run.display().to_string());
        ui.label(text(pt, "New bundle directory", "Novo diretório do pacote"));
        let mut path = self.export.display().to_string();
        if ui.text_edit_singleline(&mut path).changed() {
            self.export = path.into();
        }
        path_field(
            ui,
            text(
                pt,
                "Matching retained preparation (optional)",
                "Preparação retida correspondente (opcional)",
            ),
            &mut self.preparation,
            true,
            pt,
        );
        ui.button(text(
            pt,
            "Export selected run",
            "Exportar execução selecionada",
        ))
        .clicked()
        .then(|| {
            let prepared =
                (!self.preparation.as_os_str().is_empty()).then(|| self.preparation.clone());
            Task::Export(run.into(), prepared, self.export.clone())
        })
    }

    pub fn forcing(&mut self, ui: &mut egui::Ui, pt: bool, draft: &mut Draft) {
        ui.heading(Form::Forcing.label(pt));
        ui.label(text(
            pt,
            "Canonical source files and engine mappings",
            "Arquivos canônicos e associações do modelo",
        ));
        if self.forcing_text.is_empty() {
            let selected = draft.document.operation.forcing.clone().unwrap_or_default();
            self.forcing_text = serde_json::to_string_pretty(&selected).unwrap_or_default();
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.forcing_text)
                .desired_width(f32::INFINITY)
                .desired_rows(10)
                .code_editor(),
        );
        if ui
            .button(text(
                pt,
                "Apply forcing selections",
                "Aplicar entradas do modelo",
            ))
            .clicked()
        {
            match serde_json::from_str(&self.forcing_text) {
                Ok(value) => {
                    draft.document.operation.forcing = Some(value);
                    draft.document.sources.accepted.remove("additional_forcing");
                    self.selection_notice = text(pt, "Updated draft", "Rascunho atualizado");
                }
                Err(error) => self.selection_notice = error.to_string(),
            }
        }
        if ui
            .button(text(pt, "Use baseline forcing", "Usar entradas da base"))
            .clicked()
        {
            draft.document.operation.forcing = None;
            self.forcing_text.clear();
        }
        ui.separator();
        ui.label(text(
            pt,
            "Hot-start save time (local hour)",
            "Hora de gravação do estado (hora local)",
        ));
        let selection = draft.document.operation.hot_start.get_or_insert_default();
        let mut date = selection.save_at.clone().unwrap_or_default();
        if ui.text_edit_singleline(&mut date).changed() {
            selection.save_at = (!date.trim().is_empty()).then_some(date);
        }
        optional_path(
            ui,
            text(
                pt,
                "Initial state bundle (optional)",
                "Pacote de estado inicial (opcional)",
            ),
            &mut selection.use_bundle,
            true,
            pt,
        );
        if selection.save_at.is_none() && selection.use_bundle.is_none() {
            draft.document.operation.hot_start = None;
        }
        let routing = &mut draft.document.operation.routing;
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                routing,
                None,
                text(pt, "Engine default", "Padrão do modelo"),
            );
            ui.selectable_value(
                routing,
                Some(mgb_ops::app::RoutingMode::Inertial),
                text(pt, "Inertial", "Inercial"),
            );
            ui.selectable_value(
                routing,
                Some(mgb_ops::app::RoutingMode::MuskingumCunge),
                "Muskingum–Cunge",
            );
        });
        ui.label(&self.selection_notice);
    }

    pub fn show_preview(&mut self, ui: &mut egui::Ui, pt: bool, draft: &Draft) {
        let Some(preview) = self.preview.as_ref() else {
            return;
        };
        ui.separator();
        match preview {
            Output::Availability(report) => {
                for station in &report.stations {
                    ui.collapsing(
                        format!(
                            "{} · {}/{}",
                            station.station.name,
                            station.valid,
                            station.hours.len()
                        ),
                        |ui| {
                            ui.label(format!(
                                "{} {} · {} {} · {} {}",
                                text(pt, "Absent", "Ausentes"),
                                station.absent,
                                text(pt, "Null", "Nulos"),
                                station.null,
                                text(pt, "Excluded", "Excluídos"),
                                station.excluded
                            ));
                            for hour in &station.hours {
                                let value = hour
                                    .effective_value
                                    .map(|v| format!("{v:.2}"))
                                    .unwrap_or_else(|| "—".into());
                                ui.label(format!(
                                    "{} · {:?} · {}",
                                    hour.observed_at, hour.state, value
                                ));
                            }
                        },
                    );
                }
            }
            Output::Spatial(report) => {
                for station in &report.stations {
                    let value = station
                        .residual_mm
                        .map(|v| format!("{v:.2} mm"))
                        .unwrap_or_else(|| "—".into());
                    ui.label(format!("{} · {}", station.station_id, value));
                }
            }
            Output::Observations(report) => {
                if !draft.preview_is_current(&report.generation) {
                    ui.label(text(
                        pt,
                        "Refresh preview after editing",
                        "Atualize a prévia após editar",
                    ));
                    return;
                }
                ui.label(format!(
                    "{} · {}",
                    text(pt, "Rainfall QC findings", "Avisos de chuva"),
                    report.qc.len()
                ));
                for finding in &report.qc {
                    ui.label(format!("{finding:?}"));
                }
            }
            Output::Forecast(report) => {
                if !draft.preview_is_current(&report.generation) {
                    ui.label(text(
                        pt,
                        "Refresh preview after editing",
                        "Atualize a prévia após editar",
                    ));
                    return;
                }
                if report.raw.is_empty() {
                    return;
                }
                ui.add(
                    egui::Slider::new(&mut self.preview_step, 0..=report.raw.len() - 1).text(text(
                        pt,
                        "Interval",
                        "Intervalo",
                    )),
                );
                let (before, after) = (
                    &report.raw[self.preview_step],
                    &report.corrected[self.preview_step],
                );
                ui.label(format!(
                    "{} → {}",
                    before.interval.start, before.interval.end
                ));
                let max = [before, after]
                    .into_iter()
                    .flat_map(|frame| frame.values.iter().zip(&frame.valid))
                    .filter_map(|(&value, &valid)| valid.then_some(value))
                    .fold(0.0_f64, f64::max);
                ui.columns(2, |columns| {
                    grid_preview(
                        &mut columns[0],
                        before,
                        max,
                        &text(pt, "Original", "Original"),
                    );
                    grid_preview(
                        &mut columns[1],
                        after,
                        max,
                        &text(pt, "Corrected", "Corrigida"),
                    );
                });
            }
            _ => {}
        }
    }
}

fn destination(ui: &mut egui::Ui, label: String, path: &mut PathBuf) {
    ui.label(label);
    let mut text = path.display().to_string();
    if ui.text_edit_singleline(&mut text).changed() {
        *path = text.into();
    }
}

fn png_image(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut bytes = vec![
        0;
        reader
            .output_buffer_size()
            .ok_or("Invalid bulletin image size")?
    ];
    let info = reader.next_frame(&mut bytes).map_err(|e| e.to_string())?;
    let size = [info.width as usize, info.height as usize];
    let pixels = &bytes[..info.buffer_size()];
    match info.color_type {
        png::ColorType::Rgba => Ok(egui::ColorImage::from_rgba_unmultiplied(size, pixels)),
        png::ColorType::Rgb => Ok(egui::ColorImage::from_rgb(size, pixels)),
        _ => Err("Unexpected bulletin image color type".into()),
    }
}

fn hydrograph(ui: &mut egui::Ui, report: &HydrographReport) {
    let mut values = report
        .curves
        .iter()
        .flat_map(|curve| curve.points.iter())
        .filter_map(|point| point.value);
    let Some(first) = values.next() else {
        return;
    };
    let (min, max) = values.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 160.0),
        egui::Sense::hover(),
    );
    let seconds = (report.window.end - report.window.start)
        .num_seconds()
        .max(1) as f64;
    let range = if max > min { max - min } else { 1.0 };
    let palette = [
        egui::Color32::from_rgb(54, 142, 218),
        egui::Color32::from_rgb(227, 110, 53),
        egui::Color32::from_rgb(83, 167, 119),
        egui::Color32::from_rgb(150, 117, 206),
    ];
    for (index, curve) in report.curves.iter().enumerate() {
        for pair in curve.points.windows(2) {
            let (Some(left), Some(right)) = (pair[0].value, pair[1].value) else {
                continue;
            };
            if pair[0].segment != pair[1].segment {
                continue;
            }
            let at = |time, value| {
                let offset: chrono::TimeDelta = time - report.window.start;
                let x = (offset.num_seconds() as f64 / seconds).clamp(0.0, 1.0) as f32;
                let y = ((value - min) / range) as f32;
                egui::pos2(
                    rect.left() + x * rect.width(),
                    rect.bottom() - y * rect.height(),
                )
            };
            ui.painter().line_segment(
                [at(pair[0].timestamp, left), at(pair[1].timestamp, right)],
                egui::Stroke::new(1.5, palette[index % palette.len()]),
            );
        }
    }
}

fn grid_preview(ui: &mut egui::Ui, frame: &GridFrame, maximum: f64, title: &str) {
    ui.strong(title);
    let width = frame.grid.longitudes.len();
    let height = frame.grid.latitudes.len();
    if width == 0 || height == 0 {
        return;
    }
    let cols = width.min(64);
    let rows = height.min(64);
    let size = egui::vec2(ui.available_width(), 100.0);
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let cell = egui::vec2(size.x / cols as f32, size.y / rows as f32);
    for y in 0..rows {
        for x in 0..cols {
            let i = ((rows - 1 - y) * height / rows) * width + x * width / cols;
            let color = if frame.valid[i] {
                let intensity = if maximum > 0.0 {
                    (255.0 * frame.values[i] / maximum).round() as u8
                } else {
                    0
                };
                egui::Color32::from_rgb(245 - intensity.min(245), 246 - intensity / 2, 250)
            } else {
                ui.visuals().faint_bg_color
            };
            let pos = rect.min + egui::vec2(x as f32 * cell.x, y as f32 * cell.y);
            ui.painter()
                .rect_filled(egui::Rect::from_min_size(pos, cell), 0.0, color);
        }
    }
}
