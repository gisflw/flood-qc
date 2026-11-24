use chrono::NaiveDateTime;
#[cfg(test)]
use mgb_ops::analysis::HourlyWindow;
use mgb_ops::analysis::{RainSummary, upstream_minis};
use mgb_ops::app::{
    Location, RainfallCacheReader, RainfallPeriod, RunSummary, SeriesQuery, SeriesReport,
    SeriesSelection, Variable,
};
use mgb_ops::store::{
    self as canonical, LevelReference, MiniMapFeature, MiniTopology, StationLocation,
};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};

pub struct Project {
    pub fills: Arc<Vec<super::map::PreparedMini>>,
    pub config: PathBuf,
    pub workspace: PathBuf,
    pub session: mgb_ops::app::Session,
    pub topology: Vec<MiniTopology>,
    pub segments: Arc<Vec<super::map::RiverPaths>>,
    pub overview_segments: Arc<super::map::RiverOverviews>,
    pub catchments: Arc<Vec<MiniMapFeature>>,
    pub stations: Vec<StationLocation>,
    pub references: Vec<LevelReference>,
    pub series: Vec<(String, String, String)>,
    pub assets: Vec<String>,
    pub findings: Vec<String>,
    pub time_zone: String,
    pub retained_sources: Vec<PathBuf>,
    geometry_source: Option<PathBuf>,
    static_version: Option<Vec<SourceVersion>>,
}

type SourceVersion = (PathBuf, PathBuf, u64, std::time::SystemTime);
fn source_version(path: &Path) -> Option<SourceVersion> {
    let metadata = std::fs::metadata(path).ok()?;
    if !metadata.is_file() {
        return None;
    }
    let _file = std::fs::File::open(path).ok()?;
    Some((
        path.into(),
        std::fs::canonicalize(path).ok()?,
        metadata.len(),
        metadata.modified().ok()?,
    ))
}
fn geometry_versions(mini: &Path, geometry: &Path) -> Option<Vec<SourceVersion>> {
    let geometry = source_version(geometry)?;
    let mut wal = geometry.1.as_os_str().to_os_string();
    wal.push("-wal");
    let wal = PathBuf::from(wal);
    let mut versions = vec![source_version(mini)?, geometry];
    if wal.exists() {
        versions.push(source_version(&wal)?);
    }
    Some(versions)
}
fn retain_geometry(project: &mut Project, previous: &Project) {
    project.topology = previous.topology.clone();
    project.segments = previous.segments.clone();
    project.catchments = previous.catchments.clone();
    project.overview_segments = previous.overview_segments.clone();
    project.fills = previous.fills.clone();
    project.geometry_source = previous.geometry_source.clone();
    project.static_version = previous.static_version.clone();
}
pub fn open(
    config: PathBuf,
    workspace: PathBuf,
    previous: Option<&Project>,
) -> Result<Arc<Project>, String> {
    let session = mgb_ops::app::Session::new(mgb_ops::InspectionRequest {
        workspace,
        invocation_dir: std::env::current_dir().map_err(|error| error.to_string())?,
        config: Some(config),
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    let config = session.config().to_path_buf();
    let workspace = session.workspace().to_path_buf();
    let settings = mgb_ops::app::read_workspace_config(&config)
        .map_err(|e| format!("{}: {e}", config.display()))?;
    let previous = previous.filter(|p| p.config == config && p.workspace == workspace);
    let session = previous.map_or(session, |project| project.session.clone());
    let input = settings
        .engine
        .and_then(|e| e.input_dir)
        .unwrap_or_else(|| "mgb_runner/Input".into());
    let mini = workspace.join(input).join("MINI.gtp");
    let geometry_source = settings
        .spatial
        .and_then(|s| s.gpkg_path)
        .map(|p| workspace.join(p));
    let static_version = geometry_source
        .as_ref()
        .and_then(|path| geometry_versions(&mini, path));
    let mut project = Project {
        fills: Arc::new(vec![]),
        config,
        workspace,
        session,
        topology: vec![],
        segments: Arc::new(vec![]),
        overview_segments: Arc::default(),
        catchments: Arc::new(vec![]),
        stations: vec![],
        references: vec![],
        series: vec![],
        assets: vec![],
        findings: vec![],
        time_zone: "America/Sao_Paulo".into(),
        retained_sources: vec![],
        geometry_source,
        static_version,
    };
    let inspection = project.session.inspect();
    for finding in inspection.findings {
        if finding.status == mgb_ops::app::FindingStatus::Fail {
            project
                .findings
                .push(format!("{}: {}", finding.check, finding.message));
        }
    }
    if let Some(previous) = previous
        .filter(|p| project.static_version.is_some() && project.static_version == p.static_version)
    {
        retain_geometry(&mut project, previous);
    } else {
        match canonical::read_mini_topology(&mini) {
            Ok((_, topology)) => project.topology = topology,
            Err(error) => project
                .findings
                .push(format!("{}: {error}", mini.display())),
        }
        if let Some(path) = &project.geometry_source {
            for name in ["mini_segments", "mini_catchments"] {
                match canonical::read_mini_map_layer(path, name, &project.topology) {
                    Ok(features) if name == "mini_segments" => {
                        project.segments = Arc::new(super::map::river_segments(&features));
                        project.overview_segments =
                            Arc::new(super::map::overview_segments(&project.segments));
                    }
                    Ok(features) => project.catchments = Arc::new(features),
                    Err(error) => project.findings.push(format!("{name}: {error}")),
                }
            }
        }
        match super::map::prepare_catchments(&project.catchments) {
            Ok(fills) => project.fills = Arc::new(fills),
            Err(error) => project.findings.push(format!("catchment fill: {error}")),
        }
        if project.topology.is_empty()
            || project.segments.is_empty()
            || project.catchments.is_empty()
            || project.fills.is_empty()
        {
            project.static_version = None;
        }
        if let Some(previous) = previous
            && ((project.topology.is_empty() && !previous.topology.is_empty())
                || (project.segments.is_empty() && !previous.segments.is_empty())
                || (project.catchments.is_empty() && !previous.catchments.is_empty())
                || (project.fills.is_empty() && !previous.fills.is_empty()))
        {
            // Topology and its matching geometry form one display snapshot.
            retain_geometry(&mut project, previous);
            project.retained_sources.push(
                previous
                    .geometry_source
                    .clone()
                    .unwrap_or_else(|| previous.config.clone()),
            );
        }
    }
    let history = project.workspace.join("data/history.sqlite");
    match canonical::ObservationStore::open_read_only(&history) {
        Ok(observations) => {
            match observations.time_zone() {
                Ok(zone) => project.time_zone = zone,
                Err(error) => {
                    project.findings.push(error.to_string());
                    if let Some(previous) = previous {
                        project.time_zone = previous.time_zone.clone();
                        project.retained_sources.push(history.clone());
                    }
                }
            }
            match observations.station_locations() {
                Ok(stations) => project.stations = stations,
                Err(error) => {
                    project.findings.push(error.to_string());
                    if let Some(previous) = previous.filter(|p| !p.stations.is_empty()) {
                        project.stations = previous.stations.clone();
                        project.retained_sources.push(history.clone());
                    }
                }
            }
            match observations.series_choices() {
                Ok(series) => project.series = series,
                Err(error) => {
                    project.findings.push(error.to_string());
                    if let Some(previous) = previous.filter(|p| !p.series.is_empty()) {
                        project.series = previous.series.clone();
                        project.retained_sources.push(history.clone());
                    }
                }
            }
            match observations.read_level_references() {
                Ok(references) => project.references = references,
                Err(error) => {
                    project.findings.push(error.to_string());
                    if let Some(previous) = previous.filter(|p| !p.references.is_empty()) {
                        project.references = previous.references.clone();
                        project.retained_sources.push(history.clone());
                    }
                }
            }
        }
        Err(error) => {
            project
                .findings
                .push(format!("{}: {error}", history.display()));
            if let Some(previous) = previous.filter(|p| {
                !p.stations.is_empty() || !p.series.is_empty() || !p.references.is_empty()
            }) {
                project.stations = previous.stations.clone();
                project.series = previous.series.clone();
                project.references = previous.references.clone();
                project.time_zone = previous.time_zone.clone();
                project.retained_sources.push(history);
            }
        }
    }
    let catalog = project.workspace.join("data/forecasts.sqlite");
    if catalog.exists() {
        match canonical::ForecastCatalog::open_read_only(&project.workspace, &catalog)
            .and_then(|c| c.asset_choices())
        {
            Ok(assets) => {
                project.assets = assets
                    .into_iter()
                    .map(|a| {
                        format!(
                            "{} · {} {} · {}",
                            a.asset_id, a.provider, a.model, a.cycle_utc
                        )
                    })
                    .collect()
            }
            Err(error) => {
                project.findings.push(error.to_string());
                if let Some(previous) = previous {
                    project.assets = previous.assets.clone();
                    project.retained_sources.push(catalog);
                }
            }
        }
    } else if let Some(previous) = previous.filter(|p| !p.assets.is_empty()) {
        project.assets = previous.assets.clone();
        project.retained_sources.push(catalog.clone());
        project
            .findings
            .push(format!("{}: catalog unavailable", catalog.display()));
    }
    project.retained_sources.sort();
    project.retained_sources.dedup();
    Ok(Arc::new(project))
}

/// The exact generation selected in the UI, held against retirement while files are read.
#[derive(Clone, PartialEq, Eq)]
pub struct SelectedRun {
    pub snapshot: mgb_ops::app::RunSnapshot,
    pub root: PathBuf,
}

impl SelectedRun {
    pub fn read<T>(&self, read: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        self.snapshot
            .read(Some(&self.root), |_, _| read())?
            .ok_or_else(|| "Active run is no longer available".into())
    }
}

#[cfg(test)]
fn run_snapshot() -> mgb_ops::app::RunSnapshot {
    mgb_ops::app::Session::new(mgb_ops::InspectionRequest {
        workspace: "unused".into(),
        invocation_dir: ".".into(),
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    })
    .run_snapshot()
}

#[cfg(test)]
fn read_run(root: PathBuf) -> Result<Arc<RunSummary>, String> {
    mgb_ops::app::read_run_summary(&root).map(Arc::new)
}

#[derive(Clone)]
pub struct ViewRequest {
    pub project: Arc<Project>,
    pub location: Option<Location>,
    pub scenario: String,
    pub period: Option<RainfallPeriod>,
}
pub struct Rain {
    pub grid: canonical::RegularGridSpec,
    pub values: Vec<Option<f64>>,
    pub generation: String,
    pub scenario: String,
    pub window: canonical::TimeInterval,
}
pub struct View {
    pub rainfall: Option<Arc<Rain>>,
    pub summary: Option<RainSummary>,
    pub rainfall_finding: Option<String>,
    pub summary_finding: Option<String>,
    pub area_km2: Option<f64>,
    pub findings: Vec<String>,
}

pub fn location_mini(location: &Location, stations: &[StationLocation]) -> Option<i64> {
    match location {
        Location::Mini(id) => Some(*id),
        Location::Station(id) => stations
            .iter()
            .find(|s| s.station.station_id == *id)
            .and_then(|s| s.offset.as_ref().map(|o| o.mini_id)),
    }
}

fn view(request: ViewRequest, rainfall_reader: &mut RainfallCacheReader) -> View {
    let mini = request
        .location
        .as_ref()
        .and_then(|location| location_mini(location, &request.project.stations));
    let mut result = View {
        rainfall: None,
        summary: None,
        rainfall_finding: None,
        summary_finding: None,
        area_km2: None,
        findings: vec![],
    };
    if let Some(period) = request.period {
        match rainfall_reader.query(
            &request.project.workspace,
            &request.scenario,
            &period,
            mini,
            true,
        ) {
            Ok(Some(rainfall)) => {
                result.rainfall = Some(Arc::new(Rain {
                    grid: rainfall.grid,
                    values: rainfall.values,
                    generation: rainfall.info.generation,
                    scenario: rainfall.scenario,
                    window: rainfall.window,
                }));
                result.summary = rainfall.summary;
                result.summary_finding = rainfall.summary_finding;
            }
            Ok(None) => (),
            Err(error) => result.rainfall_finding = Some(error),
        }
    }
    if let Some(summary) = &result.summary {
        result.area_km2 = Some(summary.total_area_km2);
    } else if let Some(mini) = mini {
        let area = upstream_minis(&request.project.topology, mini)
            .and_then(|ids| mgb_ops::analysis::mini_area(&request.project.topology, &ids));
        match area {
            Ok(area) => result.area_km2 = Some(area),
            Err(error) => result.findings.push(error),
        }
    }
    result
}

pub struct ChartRequest {
    pub run: Option<SelectedRun>,
    pub query: SeriesQuery,
    pub comparison_station: Option<String>,
    pub notices: Vec<String>,
}

impl ChartRequest {
    pub fn load(self) -> Result<Loaded, String> {
        let query = || match &self.comparison_station {
            Some(station) => mgb_ops::app::query_comparison(&self.query, station),
            None => mgb_ops::app::query_series(&self.query),
        };
        let report = if let Some(run) = &self.run {
            run.read(query)?
        } else {
            query()?
        };
        let labels = self
            .query
            .selections
            .into_iter()
            .map(|selection| match selection {
                SeriesSelection::Observation { series_id, .. } => series_id,
                SeriesSelection::Model { scenario_id, .. } => scenario_id,
            })
            .collect();
        Ok(Loaded::Chart {
            report,
            station: self.comparison_station,
            labels,
            notices: self.notices,
        })
    }
}

pub fn chart_selections(
    project: &Project,
    run: Option<&RunSummary>,
    overlays: &[String],
    station: Option<&str>,
) -> Vec<SeriesSelection> {
    let mut selections = Vec::new();
    if let Some(run) = run {
        for scenario in overlays {
            for variable in [Variable::Rain, Variable::Flow, Variable::Level] {
                selections.push(SeriesSelection::Model {
                    root: run.root.clone(),
                    scenario_id: scenario.clone(),
                    variable,
                });
            }
        }
    }
    if let Some(id) = station {
        for (series, station, kind) in &project.series {
            if station != id {
                continue;
            }
            let variable = match kind.as_str() {
                "rain" => Variable::Rain,
                "flow" => Variable::Flow,
                "level" => Variable::Level,
                _ => continue,
            };
            selections.push(SeriesSelection::Observation {
                database: project.workspace.join("data/history.sqlite"),
                series_id: series.clone(),
                variable,
                effective: false,
                replacement_ids: vec![],
                replacements: None,
            });
        }
    }
    selections
}

pub fn timestamp(value: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| "Use YYYY-MM-DDTHH:MM:SS".into())
}

pub enum Job {
    Open(PathBuf, PathBuf),
    Run(mgb_ops::app::RunSnapshot, Option<PathBuf>),
    View(RainSide, ViewRequest),
    Local(Vec<PathBuf>),
    Export(SeriesReport, Option<String>, PathBuf),
    Upstream(Arc<Project>, i64),
    Chart(ChartRequest),
    Png(Box<super::export::Export>, PathBuf),
}
impl Job {
    pub fn slot(&self) -> usize {
        match self {
            Self::Open(..) => 0,
            Self::Run(..) => RUN_SLOT,
            Self::View(side, _) => side.slot(),
            Self::Local(..) => 3,
            Self::Export(..) | Self::Png(..) => 4,
            Self::Upstream(..) => UPSTREAM_SLOT,
            Self::Chart(..) => CHART_SLOT,
        }
    }
}
pub enum Loaded {
    Project(Arc<Project>),
    Run(Arc<RunSummary>),
    View(Box<View>),
    Local(Vec<super::map::LocalLayer>, Vec<String>),
    Export(PathBuf),
    Upstream {
        mini: i64,
        paths: super::map::BasinPaths,
    },
    Chart {
        report: SeriesReport,
        station: Option<String>,
        labels: Vec<String>,
        notices: Vec<String>,
    },
}
pub struct Reply {
    pub slot: usize,
    pub generation: u64,
    pub result: Result<Loaded, String>,
}
pub const RUN_SLOT: usize = 1;
pub const UPSTREAM_SLOT: usize = 5;
pub const CHART_SLOT: usize = 6;
pub const SLOTS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RainSide {
    Left,
    Right,
}

impl RainSide {
    pub const BOTH: [Self; 2] = [Self::Left, Self::Right];

    pub fn slot(self) -> usize {
        match self {
            Self::Left => 2,
            Self::Right => 7,
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
        }
    }

    pub fn from_slot(slot: usize) -> Option<Self> {
        Self::BOTH.into_iter().find(|side| side.slot() == slot)
    }
}

pub struct Worker {
    sender: Option<mpsc::Sender<(u64, Job)>>,
    receiver: mpsc::Receiver<Reply>,
    pending: [Option<(u64, Job)>; SLOTS],
    pub generations: [u64; SLOTS],
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    in_flight: Option<(usize, u64)>,
}
impl Worker {
    pub fn new(context: eframe::egui::Context) -> Self {
        let (sender, jobs) = mpsc::channel::<(u64, Job)>();
        let (replies, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = thread::spawn(move || {
            let mut last_project: Option<Arc<Project>> = None;
            let mut rainfall_reader = RainfallCacheReader::default();
            while let Ok((generation, job)) = jobs.recv() {
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let slot = job.slot();
                let result = match job {
                    Job::Open(config, workspace) => {
                        open(config, workspace, last_project.as_deref()).map(|project| {
                            last_project = Some(project.clone());
                            rainfall_reader = RainfallCacheReader::default();
                            Loaded::Project(project)
                        })
                    }
                    Job::Run(selection, expected) => selection
                        .read(expected.as_deref(), |root, _| {
                            mgb_ops::app::read_run_summary(root)
                        })
                        .and_then(|run| {
                            run.map(|run| Loaded::Run(Arc::new(run)))
                                .ok_or_else(|| "No active run".into())
                        }),
                    Job::View(_, request) => {
                        Ok(Loaded::View(Box::new(view(request, &mut rainfall_reader))))
                    }
                    Job::Local(paths) => {
                        let mut loaded = Vec::new();
                        let mut findings = Vec::new();
                        for path in paths {
                            match canonical::read_map_layers(&path) {
                                Ok(layers) => {
                                    for layer in layers {
                                        match super::map::prepare_fill(&layer.polygons) {
                                            Ok(fill) => loaded.push(super::map::LocalLayer {
                                                path: path.clone(),
                                                name: layer.name,
                                                projected_paths: super::map::project_paths(
                                                    &layer.paths,
                                                ),
                                                fill,
                                            }),
                                            Err(error) => findings
                                                .push(format!("{}: {error}", path.display())),
                                        }
                                    }
                                }
                                Err(error) => findings.push(format!("{}: {error}", path.display())),
                            }
                        }
                        Ok(Loaded::Local(loaded, findings))
                    }
                    Job::Png(export, destination) => super::export::write(*export, &destination)
                        .map(|_| Loaded::Export(destination)),
                    Job::Upstream(project, mini) => upstream_minis(&project.topology, mini)
                        .and_then(|ids| {
                            let paths = super::map::upstream_boundary(&project, &ids);
                            if paths.is_empty() {
                                return Err("No catchment geometry for upstream basin".into());
                            }
                            Ok(Loaded::Upstream {
                                mini,
                                paths: super::map::prepare_basin(paths),
                            })
                        }),
                    Job::Chart(request) => request.load(),
                    Job::Export(report, station, destination) => {
                        mgb_ops::app::export_comparison_csv(
                            &report,
                            station.as_deref(),
                            &destination,
                        )
                        .map(|_| Loaded::Export(destination))
                    }
                };
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                if replies
                    .send(Reply {
                        slot,
                        generation,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
                context.request_repaint();
            }
        });
        Self {
            sender: Some(sender),
            receiver,
            pending: std::array::from_fn(|_| None),
            generations: [0; SLOTS],
            stop,
            thread: Some(thread),
            in_flight: None,
        }
    }
    pub fn is_busy(&self) -> bool {
        self.pending.iter().any(Option::is_some) || self.in_flight.is_some()
    }
    pub fn submit(&mut self, job: Job) {
        let slot = job.slot();
        self.generations[slot] += 1;
        self.pending[slot] = Some((self.generations[slot], job));
    }
    pub fn invalidate(&mut self, slot: usize) {
        self.generations[slot] += 1;
        self.pending[slot] = None;
    }
    pub fn poll(&mut self) -> Vec<Reply> {
        let mut replies = Vec::new();
        loop {
            match self.receiver.try_recv() {
                Ok(reply) => {
                    if self.in_flight == Some((reply.slot, reply.generation)) {
                        self.in_flight = None;
                    }
                    replies.push(reply);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    if let Some((slot, generation)) = self.in_flight.take() {
                        replies.push(Reply {
                            slot,
                            generation,
                            result: Err("Monitoring worker has stopped".into()),
                        });
                    }
                    break;
                }
            }
        }
        if self.in_flight.is_none()
            && let Some((slot, pending)) = self
                .pending
                .iter_mut()
                .enumerate()
                .find(|(_, job)| job.is_some())
        {
            let (generation, job) = pending.take().unwrap();
            if self
                .sender
                .as_ref()
                .unwrap()
                .send((generation, job))
                .is_ok()
            {
                self.in_flight = Some((slot, generation));
            } else {
                replies.push(Reply {
                    slot,
                    generation,
                    result: Err("Monitoring worker has stopped".into()),
                });
            }
        }
        replies.retain(|reply| reply.generation == self.generations[reply.slot]);
        replies
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn infer_workspace(config: &Path) -> Option<PathBuf> {
    (config.file_name()?.to_str()? == "workspace.toml"
        && config.parent()?.file_name()?.to_str()? == "config")
        .then(|| config.parent().unwrap().parent().unwrap().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chart_worker_completes_latest_request_while_map_lane_is_busy() {
        let mut chart = Worker::new(eframe::egui::Context::default());
        let mut map = Worker::new(eframe::egui::Context::default());
        let (sender, in_flight) = mpsc::channel();
        map.sender = Some(sender); // Hold the map request in flight until the chart reply is checked.
        let start = timestamp("2026-01-01T00:00:00").unwrap();
        let request = |id| {
            Job::Chart(ChartRequest {
                run: None,
                query: SeriesQuery {
                    location: Location::Mini(id),
                    time_zone: "UTC".into(),
                    window: HourlyWindow::Explicit {
                        start,
                        end: start + chrono::Duration::hours(1),
                    },
                    selections: vec![],
                },
                comparison_station: Some("002".into()),
                notices: vec![],
            })
        };
        map.submit(Job::Run(run_snapshot(), Some("slow-map-lane".into())));
        assert!(map.poll().is_empty());
        assert!(in_flight.try_recv().is_ok());
        chart.submit(request(1));
        chart.poll();
        chart.submit(request(2));
        chart.submit(request(3));
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(reply) = chart.poll().pop() {
                assert_eq!(reply.generation, 3);
                let Loaded::Chart {
                    report, station, ..
                } = reply.result.unwrap()
                else {
                    panic!("expected chart reply");
                };
                let latest = matches!(report.location, Location::Mini(3));
                assert!(latest);
                assert_eq!(station.as_deref(), Some("002"));
                assert!(report.series.is_empty());
                break;
            }
            assert!(std::time::Instant::now() < limit);
            thread::yield_now();
        }
        let idle = !chart.is_busy();
        assert!(idle);
        assert!(map.is_busy());
    }

    #[test]
    fn invalidated_inflight_monitoring_remains_busy_until_the_read_stops() {
        let (sender, jobs) = mpsc::channel::<(u64, Job)>();
        let (replies, receiver) = mpsc::channel();
        let (started, start) = mpsc::channel();
        let (release, blocked) = mpsc::channel();
        let thread = thread::spawn(move || {
            let (generation, _job) = jobs.recv().unwrap();
            started.send(()).unwrap();
            blocked.recv().unwrap();
            replies
                .send(Reply {
                    slot: 1,
                    generation,
                    result: Err("delayed read".into()),
                })
                .unwrap();
        });
        let mut worker = Worker {
            sender: Some(sender),
            receiver,
            pending: std::array::from_fn(|_| None),
            generations: [0; SLOTS],
            stop: Arc::new(AtomicBool::new(false)),
            thread: Some(thread),
            in_flight: None,
        };
        worker.submit(Job::Run(run_snapshot(), Some("delayed".into())));
        assert!(worker.is_busy());
        assert!(worker.poll().is_empty());
        start.recv().unwrap();
        worker.invalidate(1);
        assert!(worker.is_busy());
        assert!(worker.poll().is_empty());
        release.send(()).unwrap();
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while worker.is_busy() {
            assert!(worker.poll().is_empty());
            assert!(std::time::Instant::now() < limit);
            thread::yield_now();
        }
        assert!(worker.poll().is_empty());
        drop(worker);
    }
    fn saved_trial(root: &Path, longitude: f64, latitude: f64) {
        let start = timestamp("2026-01-01T00:00:00").unwrap();
        let timeline = canonical::IntervalAxis {
            time_zone: "UTC".into(),
            bounds: (0..4)
                .map(|h| canonical::TimeInterval {
                    start: start + chrono::Duration::hours(h),
                    end: start + chrono::Duration::hours(h + 1),
                })
                .collect(),
        };
        let provenance = canonical::Provenance {
            source_ids: vec!["synthetic".into()],
            processing: serde_json::from_str(r#"{"reference_time":"2026-01-01T02:00:00","engine_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","snapshot_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"}"#).unwrap(),
        };
        let basin = canonical::ResultSeries {
            timeline: timeline.clone(),
            basins: canonical::BasinGeometry {
                mini_ids: vec![1],
                longitudes: vec![longitude],
                latitudes: vec![latitude],
                source: provenance.clone(),
            },
            time_segment: vec![0, 0, 1, 1],
            precipitation: vec![0., 2., 4., 6.],
            precipitation_valid: vec![true; 4],
            flow: vec![0., 1., 2., 3.],
            flow_valid: vec![true; 4],
            raw_level_cm: vec![-2, -1, 0, 1],
            raw_level_valid: vec![true; 4],
            provenance: provenance.clone(),
        };
        let station = canonical::StationResults {
            timeline,
            time_segment: vec![0, 0, 1, 1],
            station_ids: vec!["002".into()],
            mini_ids: vec![Some(1)],
            offsets_m: vec![Some(0.)],
            level_cm: vec![-2, -1, 0, 1],
            level_valid: vec![true; 4],
            provenance,
        };
        std::fs::create_dir_all(root.join("results/good")).unwrap();
        canonical::write_result_series(&root.join("results/good/basin.nc"), &basin).unwrap();
        canonical::write_station_results(&root.join("results/good/station.nc"), &station).unwrap();
        let scenario = |id: &str, state: &str| canonical::ScenarioRecord {
            id: id.into(),
            state: state.into(),
            started_at: None,
            finished_at: None,
            exit_code: None,
            signal: None,
            error: None,
            stdout: None,
            stderr: None,
            basin_result: None,
            station_result: None,
        };
        let manifest = canonical::OperationManifest {
            schema_version: 1,
            preparation_sha256: None,
            state: "failed".into(),
            recipe: serde_json::Value::Null,
            inputs: vec![],
            scenarios: vec![
                scenario("broken", "completed"),
                scenario("good", "completed"),
                scenario("failed", "failed"),
            ],
            created_at: "2026-01-01T04:00:00".into(),
            finished_at: None,
            error: Some("synthetic sibling failure".into()),
        };
        std::fs::write(
            root.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }
    #[test]
    fn partial_runs_keep_usable_siblings_and_reject_unsafe_scenario_paths() {
        let dir = tempfile::tempdir().unwrap();
        saved_trial(dir.path(), 0., 0.);
        let run = read_run(dir.path().into()).unwrap();
        assert_eq!(run.scenarios, ["good"]);
        assert_eq!(run.findings.len(), 1);
        assert_eq!(run.manifest.state, "failed");
        std::fs::remove_file(dir.path().join("results/good/basin.nc")).unwrap();
        let no_results = read_run(dir.path().into()).unwrap();
        assert_eq!(no_results.manifest.state, "failed");
        assert!(no_results.scenarios.is_empty());
        assert!(no_results.start.is_empty());
        assert_eq!(no_results.findings.len(), 2);

        let mut manifest = run.manifest.clone();
        manifest.scenarios[0].id = "../outside".into();
        std::fs::write(
            dir.path().join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(read_run(dir.path().into()).is_err());
    }
    #[test]
    #[ignore = "requires read-only regional geometry; creates a disposable synthetic display fixture under runs"]
    fn regional_virtual_display_fixture() {
        let root = Path::new("runs/desktop-qualification/region");
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();
        std::fs::write(
            root.join("config/workspace.toml"),
            r#"[spatial]
gpkg_path = "/workspace/rs_hydro_forecast/data/source/rs_hydro.gpkg"
[engine]
input_dir = "/workspace/rs_hydro_forecast/mgb_runner/Input"
executable = "/workspace/rs_hydro_forecast/mgb_runner/MGB_Inercial_Prev_RSH"
"#,
        )
        .unwrap();
        let geometry = canonical::read_basin_geometry(Path::new(
            "/workspace/rs_hydro_forecast/mgb_runner/Input/MINI.gtp",
        ))
        .unwrap();
        let synthetic = root.join("data/runs/synthetic");
        if synthetic.exists() {
            std::fs::remove_dir_all(&synthetic).unwrap();
        }
        saved_trial(&synthetic, geometry.longitudes[0], geometry.latitudes[0]);
        let history = root.join("data/history.sqlite");
        if !history.exists() {
            let mut database = canonical::ObservationStore::create(&history, "UTC").unwrap();
            let station_id = "002".to_owned();
            let series_id = "fixture-level".to_owned();
            let start = timestamp("2026-01-01T00:00:00").unwrap();
            database
                .write_batch(&canonical::ObservationBatch {
                    stations: vec![canonical::Station {
                        station_id: station_id.clone(),
                        provider_code: "fixture".into(),
                        station_code: station_id.clone(),
                        name: "Fixture station".into(),
                        latitude: Some(-30.),
                        longitude: Some(-52.),
                    }],
                    offsets: vec![canonical::StationOffset {
                        station_id: station_id.clone(),
                        mini_id: 1,
                        offset_m: 0.,
                    }],
                    series: vec![canonical::ObservationSeries {
                        series_id: series_id.clone(),
                        station_id,
                        variable: "level".into(),
                        unit: "cm".into(),
                        state: "raw".into(),
                        interval_seconds: None,
                        created_at: start,
                    }],
                    samples: vec![canonical::ObservationSample {
                        series_id,
                        observed_at: start,
                        value: Some(canonical::ObservationValue::LevelCm(-1)),
                    }],
                    ..Default::default()
                })
                .unwrap();
        }
        let config = std::path::absolute(root.join("config/workspace.toml")).unwrap();
        let workspace = std::path::absolute(root).unwrap();
        let first = open(config.clone(), workspace.clone(), None).unwrap();
        let second = open(config, workspace, Some(&first)).unwrap();
        assert_eq!(first.topology.len(), 8649);
        assert_eq!(first.stations[0].station.station_id, "002");
        assert!(Arc::ptr_eq(&first.segments, &second.segments));
        assert!(Arc::ptr_eq(&first.fills, &second.fills));
    }
    #[test]
    fn geometry_versions_track_wal_and_reject_nonfiles() {
        let dir = tempfile::tempdir().unwrap();
        let mini = dir.path().join("MINI.gtp");
        let geometry = dir.path().join("geometry.gpkg");
        std::fs::write(&mini, "mini").unwrap();
        std::fs::write(&geometry, "geometry").unwrap();
        let first = geometry_versions(&mini, &geometry).unwrap();
        let wal = dir.path().join("geometry.gpkg-wal");
        std::fs::write(&wal, "new committed frame").unwrap();
        assert_ne!(first, geometry_versions(&mini, &geometry).unwrap());
        let second = geometry_versions(&mini, &geometry).unwrap();
        std::fs::write(&wal, "a different committed frame").unwrap();
        assert_ne!(second, geometry_versions(&mini, &geometry).unwrap());
        assert!(source_version(dir.path()).is_none());
    }
    #[test]
    fn saved_trial_does_not_supply_rainfall_without_an_active_cache() {
        let dir = tempfile::tempdir().unwrap();
        saved_trial(dir.path(), 0., 0.);
        let root = dir.path();
        let config = root.join("workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let mut project = open(config, root.into(), None).unwrap();
        Arc::get_mut(&mut project).unwrap().topology = vec![
            MiniTopology {
                mini_id: 1,
                downstream: None,
                area_km2: 1.,
            },
            MiniTopology {
                mini_id: 2,
                downstream: Some(1),
                area_km2: 3.,
            },
        ];
        let result = view(
            ViewRequest {
                project,
                location: Some(Location::Mini(1)),
                scenario: "good".into(),
                period: Some(RainfallPeriod::SignedHours(-1)),
            },
            &mut RainfallCacheReader::default(),
        );
        assert!(result.rainfall.is_none());
        assert!(result.summary.is_none());
        assert_eq!(result.area_km2, Some(4.));
        let cache_missing = !root.join("data/cache/rainfall").exists();
        assert!(cache_missing);
    }
    #[test]
    fn failed_geometry_refresh_retains_snapshot_and_loads_new_observations() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = root.join("workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let mut previous = open(config.clone(), root.into(), None).unwrap();
        let old = Arc::get_mut(&mut previous).unwrap();
        old.topology = vec![MiniTopology {
            mini_id: 1,
            downstream: None,
            area_km2: 1.,
        }];
        let path = super::super::map::ProjectedPath::new(vec![(0., 0.), (1., 1.)]);
        old.segments = Arc::new(Vec::from([super::super::map::RiverPaths {
            mini_id: 1,
            paths: vec![path],
        }]));
        std::fs::create_dir_all(root.join("data")).unwrap();
        let mut database =
            canonical::ObservationStore::create(&root.join("data/history.sqlite"), "UTC").unwrap();
        database
            .write_batch(&canonical::ObservationBatch {
                stations: vec![canonical::Station {
                    station_id: "new".into(),
                    provider_code: "p".into(),
                    station_code: "1".into(),
                    name: "New station".into(),
                    latitude: None,
                    longitude: None,
                }],
                ..Default::default()
            })
            .unwrap();
        drop(database);
        let refreshed = open(config, root.into(), Some(&previous)).unwrap();
        assert!(Arc::ptr_eq(&previous.segments, &refreshed.segments));
        assert!(Arc::ptr_eq(&previous.fills, &refreshed.fills));
        assert_eq!(refreshed.topology, previous.topology);
        assert_eq!(refreshed.stations[0].station.station_id, "new");
        assert!(
            refreshed
                .retained_sources
                .contains(&root.join("missing.gpkg"))
        );
        assert!(previous.stations.is_empty());
    }
    #[test]
    fn observations_remain_usable_without_geometry_engine_or_caches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("config")).unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();
        let config = root.join("config/workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let original = std::fs::read(&config).unwrap();
        let mut database =
            canonical::ObservationStore::create(&root.join("data/history.sqlite"), "UTC").unwrap();
        let start = timestamp("2026-01-01T00:00:00").unwrap();
        database
            .write_batch(&canonical::ObservationBatch {
                stations: vec![canonical::Station {
                    station_id: "no-coordinate".into(),
                    provider_code: "p".into(),
                    station_code: "1".into(),
                    name: "Sem coordenadas".into(),
                    latitude: None,
                    longitude: None,
                }],
                series: vec![canonical::ObservationSeries {
                    series_id: "level".into(),
                    station_id: "no-coordinate".into(),
                    variable: "level".into(),
                    unit: "cm".into(),
                    state: "raw".into(),
                    interval_seconds: None,
                    created_at: start,
                }],
                samples: vec![canonical::ObservationSample {
                    series_id: "level".into(),
                    observed_at: start,
                    value: Some(canonical::ObservationValue::LevelCm(-2)),
                }],
                ..Default::default()
            })
            .unwrap();
        drop(database);
        let project = open(config.clone(), root.into(), None).unwrap();
        assert_eq!(project.stations.len(), 1);
        let window = HourlyWindow::Explicit {
            start,
            end: start + chrono::Duration::hours(1),
        };
        let result = mgb_ops::app::query_series(&SeriesQuery {
            selections: chart_selections(&project, None, &[], Some("no-coordinate")),
            location: Location::Station("no-coordinate".into()),
            time_zone: "UTC".into(),
            window,
        })
        .unwrap();
        assert_eq!(result.series[0].points[0].value, Some(-2.));
        assert_eq!(std::fs::read(config).unwrap(), original);
        let untouched = !root.join("data/runs").exists();
        assert!(untouched);
    }
    #[test]
    fn standard_config_association_only() {
        assert_eq!(
            infer_workspace(Path::new("/region/config/workspace.toml")),
            Some(PathBuf::from("/region"))
        );
        assert_eq!(infer_workspace(Path::new("/region/custom.toml")), None);
    }
    #[test]
    fn rainfall_request_invalidation_is_independent_by_side() {
        let mut worker = Worker::new(eframe::egui::Context::default());
        let (sender, receiver) = mpsc::channel();
        worker.receiver = receiver;
        for side in RainSide::BOTH {
            assert_eq!(RainSide::from_slot(side.slot()), Some(side));
            sender
                .send(Reply {
                    slot: side.slot(),
                    generation: 0,
                    result: Err("synthetic reply".into()),
                })
                .unwrap();
        }
        worker.invalidate(RainSide::Left.slot());
        let replies = worker.poll();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].slot, RainSide::Right.slot());
    }

    #[test]
    fn newer_requests_replace_pending_work_and_old_replies() {
        let mut worker = Worker::new(eframe::egui::Context::default());
        worker.submit(Job::Run(run_snapshot(), Some("first".into())));
        worker.submit(Job::Run(run_snapshot(), Some("second".into())));
        assert_eq!(worker.generations[1], 2);
        let latest = matches!(&worker.pending[1], Some((2, Job::Run(_, Some(path)))) if path == Path::new("second"));
        assert!(latest);
        let (sender, receiver) = mpsc::channel();
        worker.receiver = receiver;
        sender
            .send(Reply {
                slot: 1,
                generation: 1,
                result: Err("old".into()),
            })
            .unwrap();
        sender
            .send(Reply {
                slot: 1,
                generation: 2,
                result: Err("current".into()),
            })
            .unwrap();
        let replies = worker.poll();
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].generation, 2);
        worker.invalidate(1);
        assert!(worker.pending[1].is_none());
        assert_eq!(worker.generations[1], 3);
    }
}
