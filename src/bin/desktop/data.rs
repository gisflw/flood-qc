use chrono::NaiveDateTime;
use mgb_ops::analysis::{
    HourlyWindow, RainSummary, accumulated_grid_rain, accumulated_rain, upstream_minis,
};
use mgb_ops::app::{Location, SeriesQuery, SeriesReport, SeriesSelection, Variable};
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
    pub topology: Vec<MiniTopology>,
    pub segments: Arc<Vec<super::map::RiverPaths>>,
    pub overview_segments: Arc<Vec<super::map::RiverPaths>>,
    pub catchments: Arc<Vec<MiniMapFeature>>,
    pub stations: Vec<StationLocation>,
    pub references: Vec<LevelReference>,
    pub series: Vec<(String, String, String)>,
    pub runs: Vec<PathBuf>,
    pub grids: Vec<PathBuf>,
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
    let settings = mgb_ops::app::read_workspace_config(&config)
        .map_err(|e| format!("{}: {e}", config.display()))?;
    let previous = previous.filter(|p| p.config == config && p.workspace == workspace);
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
        topology: vec![],
        segments: Arc::new(vec![]),
        overview_segments: Arc::new(vec![]),
        catchments: Arc::new(vec![]),
        stations: vec![],
        references: vec![],
        series: vec![],
        runs: vec![],
        grids: vec![],
        assets: vec![],
        findings: vec![],
        time_zone: "America/Sao_Paulo".into(),
        retained_sources: vec![],
        geometry_source,
        static_version,
    };
    let inspection = mgb_ops::app::inspect(mgb_ops::app::InspectionRequest {
        workspace: project.workspace.clone(),
        invocation_dir: std::env::current_dir().map_err(|e| e.to_string())?,
        config: Some(project.config.clone()),
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
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
                        project.overview_segments =
                            Arc::new(super::map::overview_segments(&features));
                        project.segments = Arc::new(super::map::river_segments(&features));
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
    match mgb_ops::app::discover_trials(&project.workspace) {
        Ok(runs) => project.runs = runs,
        Err(error) => {
            project.findings.push(error.to_string());
            if let Some(previous) = previous {
                project.runs = previous.runs.clone();
                project
                    .retained_sources
                    .push(project.workspace.join("data/runs"));
            }
        }
    }
    match mgb_ops::app::discover_rainfall_grids(&project.workspace) {
        Ok(grids) => project.grids = grids,
        Err(error) => {
            project.findings.push(error);
            if let Some(previous) = previous {
                project.grids = previous.grids.clone();
                project
                    .retained_sources
                    .push(project.workspace.join("data/cache/preparations"));
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

pub struct Run {
    pub root: PathBuf,
    pub manifest: canonical::OperationManifest,
    pub scenarios: Vec<String>,
    pub findings: Vec<String>,
    pub reference: String,
    pub start: String,
    pub end: String,
    pub time_zone: String,
}
pub fn read_run(root: PathBuf) -> Result<Arc<Run>, String> {
    let manifest =
        canonical::read_retained_manifest(&root).map_err(|e| format!("{}: {e}", root.display()))?;
    if manifest.scenarios.iter().any(|s| {
        s.id.is_empty()
            || !s
                .id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    }) {
        return Err("invalid scenario ID in retained manifest".into());
    }
    let mut scenarios = Vec::new();
    let mut findings = Vec::new();
    let mut first = None;
    for scenario in manifest.scenarios.iter().filter(|s| s.state == "completed") {
        match canonical::read_result_series(
            &root.join("results").join(&scenario.id).join("basin.nc"),
        ) {
            Ok(basin) => {
                scenarios.push(scenario.id.clone());
                if first.is_none() {
                    first = Some(basin);
                }
            }
            Err(error) => findings.push(format!("{}: {error}", scenario.id)),
        }
    }
    let basin = first.ok_or_else(|| {
        format!(
            "run has no available successful scenario results: {}",
            findings.join("; ")
        )
    })?;
    Ok(Arc::new(Run {
        root,
        manifest,
        scenarios,
        findings,
        reference: basin
            .provenance
            .processing
            .get("reference_time")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .into(),
        start: basin
            .timeline
            .bounds
            .first()
            .ok_or("empty result timeline")?
            .start
            .to_string()
            .replace(' ', "T"),
        end: basin
            .timeline
            .bounds
            .last()
            .ok_or("empty result timeline")?
            .end
            .to_string()
            .replace(' ', "T"),
        time_zone: basin.timeline.time_zone,
    }))
}

#[derive(Clone)]
pub struct ViewRequest {
    pub project: Arc<Project>,
    pub run: Option<Arc<Run>>,
    pub location: Option<Location>,
    pub overlays: Vec<String>,
    pub observations: bool,
    pub map_scenario: String,
    pub grid: Option<PathBuf>,
    pub zone: String,
    pub window: HourlyWindow,
    pub chart_window: HourlyWindow,
    pub upstream: bool,
}
pub struct Rain {
    pub zone: String,
    pub grid: canonical::RegularGridSpec,
    pub values: Vec<Option<f64>>,
    pub window: canonical::TimeInterval,
    pub source: PathBuf,
}
pub struct View {
    pub zone: String,
    pub run: Option<Arc<Run>>,
    pub map_scenario: String,
    pub query: Option<SeriesQuery>,
    pub series: Option<SeriesReport>,
    pub rainfall: Option<Arc<Rain>>,
    pub summary: Option<RainSummary>,
    pub upstream: Vec<i64>,
    pub area_km2: Option<f64>,
    pub findings: Vec<String>,
}
type RainCache = Option<(
    PathBuf,
    u64,
    std::time::SystemTime,
    canonical::TimeInterval,
    Arc<Rain>,
)>;
fn load_rain(
    path: PathBuf,
    zone: &str,
    window: &HourlyWindow,
    cache: &mut RainCache,
) -> Result<Arc<Rain>, String> {
    let metadata = std::fs::metadata(&path).map_err(|e| {
        format!(
            "{}: {e}. Select an available rainfall checkpoint or refresh sources.",
            path.display()
        )
    })?;
    let modified = metadata.modified().map_err(|e| e.to_string())?;
    let interval = mgb_ops::analysis::resolve_hourly_window(zone, window)?;
    if let Some((old, length, version, bounds, rain)) = cache
        && old == &path
        && *length == metadata.len()
        && *version == modified
        && bounds == &interval
        && rain.zone == zone
    {
        return Ok(rain.clone());
    }
    let grid = canonical::read_grid_series(&path).map_err(|e| e.to_string())?;
    if grid.timeline.time_zone != zone {
        return Err("rainfall timezone differs from selection".into());
    }
    let accumulated = accumulated_grid_rain(&grid, window)?;
    let rain = Arc::new(Rain {
        zone: zone.into(),
        grid: grid.grid,
        values: accumulated.values,
        window: accumulated.window,
        source: path.clone(),
    });
    *cache = Some((path, metadata.len(), modified, interval, rain.clone()));
    Ok(rain)
}
fn view(request: ViewRequest, cache: &mut RainCache) -> Result<View, String> {
    let mut result = View {
        zone: request.zone.clone(),
        run: request.run.clone(),
        map_scenario: request.map_scenario.clone(),
        query: None,
        series: None,
        rainfall: None,
        summary: None,
        upstream: vec![],
        area_km2: None,
        findings: vec![],
    };
    let window = mgb_ops::analysis::resolve_hourly_window(&request.zone, &request.window)?;
    if let Some(path) = request.grid {
        match load_rain(path, &request.zone, &request.window, cache) {
            Ok(rainfall) => result.rainfall = Some(rainfall),
            Err(error) => result.findings.push(error),
        }
    }
    if let Some(location) = request.location {
        let mini = match &location {
            Location::Mini(id) => Some(*id),
            Location::Station(id) => request
                .project
                .stations
                .iter()
                .find(|s| s.station.station_id == *id)
                .and_then(|s| s.offset.as_ref().map(|o| o.mini_id)),
        };
        if let Some(mini) = mini {
            match upstream_minis(&request.project.topology, mini) {
                Ok(ids) => result.upstream = if request.upstream { ids } else { vec![mini] },
                Err(error) => result.findings.push(error),
            }
            match mgb_ops::analysis::mini_area(&request.project.topology, &result.upstream) {
                Ok(area) => result.area_km2 = Some(area),
                Err(error) => result.findings.push(error),
            }
            if let Some(run) = &request.run {
                match canonical::read_result_series(
                    &run.root
                        .join("results")
                        .join(&request.map_scenario)
                        .join("basin.nc"),
                )
                .map_err(|e| e.to_string())
                .and_then(|series| {
                    if series.timeline.time_zone != request.zone {
                        return Err("result timezone differs from selection".into());
                    }
                    accumulated_rain(
                        &series,
                        &request.project.topology,
                        mini,
                        request.upstream,
                        &HourlyWindow::Explicit {
                            start: window.start,
                            end: window.end,
                        },
                    )
                }) {
                    Ok(summary) => result.summary = Some(summary),
                    Err(error) => result.findings.push(error),
                }
            }
        }
        let mut selections = Vec::new();
        for variable in [Variable::Rain, Variable::Flow, Variable::Level] {
            if let Some(run) = &request.run {
                for scenario in &request.overlays {
                    selections.push(SeriesSelection::Model {
                        root: run.root.clone(),
                        scenario_id: scenario.clone(),
                        variable: variable.clone(),
                    });
                }
            }
            if let Location::Station(id) = &location
                && request.observations
            {
                let name = match variable {
                    Variable::Rain => "rain",
                    Variable::Flow => "flow",
                    Variable::Level => "level",
                };
                for (series, station, kind) in &request.project.series {
                    if station == id && kind == name {
                        selections.push(SeriesSelection::Observation {
                            database: request.project.workspace.join("data/history.sqlite"),
                            series_id: series.clone(),
                            variable: variable.clone(),
                            effective: false,
                            replacement_ids: vec![],
                            replacements: None,
                        });
                    }
                }
            }
        }
        let query = SeriesQuery {
            location,
            time_zone: request.zone,
            window: request.chart_window,
            selections,
        };
        match mgb_ops::app::query_series(&query) {
            Ok(series) => {
                result
                    .findings
                    .extend(series.findings.iter().map(|f| f.message.clone()));
                result.series = Some(series);
            }
            Err(error) => result.findings.push(error),
        }
        result.query = Some(query);
    }
    Ok(result)
}

pub fn timestamp(value: &str) -> Result<NaiveDateTime, String> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .map_err(|_| "Use YYYY-MM-DDTHH:MM:SS".into())
}

pub enum Job {
    Open(PathBuf, PathBuf),
    Run(PathBuf),
    View(ViewRequest),
    Local(Vec<PathBuf>),
    Export(SeriesQuery, PathBuf),
    Png(Box<super::export::Export>, PathBuf),
}
impl Job {
    fn slot(&self) -> usize {
        match self {
            Self::Open(..) => 0,
            Self::Run(..) => 1,
            Self::View(..) => 2,
            Self::Local(..) => 3,
            Self::Export(..) | Self::Png(..) => 4,
        }
    }
}
pub enum Loaded {
    Project(Arc<Project>),
    Run(Arc<Run>),
    View(Box<View>),
    Local(Vec<super::map::LocalLayer>, Vec<String>),
    Export(PathBuf),
}
pub struct Reply {
    pub slot: usize,
    pub generation: u64,
    pub result: Result<Loaded, String>,
}
pub struct Worker {
    sender: Option<mpsc::SyncSender<(u64, Job)>>,
    receiver: mpsc::Receiver<Reply>,
    pending: [Option<(u64, Job)>; 5],
    pub generations: [u64; 5],
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn new(context: eframe::egui::Context) -> Self {
        let (sender, jobs) = mpsc::sync_channel::<(u64, Job)>(1);
        let (replies, receiver) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let thread = thread::spawn(move || {
            let mut rain_cache = None;
            let mut last_project: Option<Arc<Project>> = None;
            while let Ok((generation, job)) = jobs.recv() {
                if stopped.load(Ordering::Acquire) {
                    break;
                }
                let slot = job.slot();
                let result = match job {
                    Job::Open(config, workspace) => {
                        if last_project
                            .as_ref()
                            .is_none_or(|p| p.config != config || p.workspace != workspace)
                        {
                            rain_cache = None;
                        }
                        open(config, workspace, last_project.as_deref()).map(|project| {
                            last_project = Some(project.clone());
                            Loaded::Project(project)
                        })
                    }
                    Job::Run(root) => read_run(root).map(Loaded::Run),
                    Job::View(request) => {
                        view(request, &mut rain_cache).map(|v| Loaded::View(Box::new(v)))
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
                    Job::Export(query, destination) => {
                        mgb_ops::app::export_series_csv(&query, &destination)
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
            pending: [None, None, None, None, None],
            generations: [0; 5],
            stop,
            thread: Some(thread),
        }
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
        for pending in &mut self.pending {
            if let Some(job) = pending.take() {
                match self.sender.as_ref().unwrap().try_send(job) {
                    Ok(()) => {}
                    Err(mpsc::TrySendError::Full(job)) => {
                        *pending = Some(job);
                        break;
                    }
                    Err(mpsc::TrySendError::Disconnected(_)) => break,
                }
            }
        }
        self.receiver
            .try_iter()
            .filter(|reply| reply.generation == self.generations[reply.slot])
            .collect()
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
            station_ids: vec!["s".into()],
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
        let config = std::path::absolute(root.join("config/workspace.toml")).unwrap();
        let workspace = std::path::absolute(root).unwrap();
        let first = open(config.clone(), workspace.clone(), None).unwrap();
        let second = open(config, workspace, Some(&first)).unwrap();
        assert_eq!(first.topology.len(), 8649);
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
    fn rainfall_summary_rejects_a_different_model_timezone() {
        let dir = tempfile::tempdir().unwrap();
        saved_trial(dir.path(), 0., 0.);
        let run = read_run(dir.path().into()).unwrap();
        let config = dir.path().join("workspace.toml");
        std::fs::write(&config, "[spatial]\ngpkg_path = \"missing.gpkg\"\n").unwrap();
        let mut project = open(config, dir.path().into(), None).unwrap();
        Arc::get_mut(&mut project).unwrap().topology = vec![MiniTopology {
            mini_id: 1,
            downstream: None,
            area_km2: 1.,
        }];
        let window = HourlyWindow::Explicit {
            start: timestamp(&run.start).unwrap(),
            end: timestamp(&run.end).unwrap(),
        };
        let result = view(
            ViewRequest {
                project,
                run: Some(run),
                location: Some(Location::Mini(1)),
                overlays: vec!["good".into()],
                observations: false,
                map_scenario: "good".into(),
                grid: None,
                zone: "America/Sao_Paulo".into(),
                window: window.clone(),
                chart_window: window,
                upstream: false,
            },
            &mut None,
        )
        .unwrap();
        assert!(result.summary.is_none());
        assert!(
            result
                .findings
                .iter()
                .any(|f| f.contains("timezone differs"))
        );
        assert_eq!(result.area_km2, Some(1.));
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
        old.segments = Arc::new(Vec::from([super::super::map::RiverPaths {
            mini_id: 1,
            paths: vec![Vec::from([(0., 0.), (1., 1.)])],
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
        assert!(project.runs.is_empty());
        assert!(project.grids.is_empty());
        let window = HourlyWindow::Explicit {
            start,
            end: start + chrono::Duration::hours(1),
        };
        let result = view(
            ViewRequest {
                project,
                run: None,
                location: Some(Location::Station("no-coordinate".into())),
                overlays: vec![],
                observations: true,
                map_scenario: String::new(),
                grid: None,
                zone: "UTC".into(),
                window: window.clone(),
                chart_window: window,
                upstream: true,
            },
            &mut None,
        )
        .unwrap();
        assert_eq!(result.series.unwrap().series[0].points[0].value, Some(-2.));
        assert_eq!(std::fs::read(config).unwrap(), original);
        let untouched = !root.join("data/runs").exists();
        assert!(untouched);
    }
    #[test]
    fn rainfall_reuses_the_same_version_and_respects_masks_and_timezone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("grid.nc");
        let start = timestamp("2026-01-01T00:00:00").unwrap();
        let end = start + chrono::Duration::hours(1);
        let spec = canonical::RegularGridSpec::new([0., 0., 1., 1.], 1.).unwrap();
        let cells = spec.latitudes.len() * spec.longitudes.len();
        let mut grid = canonical::GridSeries {
            timeline: canonical::IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![canonical::TimeInterval { start, end }],
            },
            grid: spec,
            values: vec![0.; cells],
            valid: vec![true; cells],
            provenance: canonical::Provenance {
                source_ids: vec!["test".into()],
                processing: serde_json::Value::Null,
            },
        };
        grid.valid[0] = false;
        canonical::write_grid_series(&path, &grid).unwrap();
        let window = HourlyWindow::Explicit { start, end };
        let mut cache = None;
        let first = load_rain(path.clone(), "UTC", &window, &mut cache).unwrap();
        let second = load_rain(path.clone(), "UTC", &window, &mut cache).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(first.values[0], None);
        assert_eq!(first.values[1], Some(0.));
        assert!(load_rain(path, "America/Sao_Paulo", &window, &mut cache).is_err());
        let source =
            super::super::map::cell_at(&first, (grid.grid.longitudes[0], grid.grid.latitudes[0]))
                .unwrap();
        assert_eq!(source.2, None);
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
    fn newer_requests_replace_pending_work_and_old_replies() {
        let mut worker = Worker::new(eframe::egui::Context::default());
        worker.submit(Job::Run("first".into()));
        worker.submit(Job::Run("second".into()));
        assert_eq!(worker.generations[1], 2);
        let latest =
            matches!(&worker.pending[1], Some((2, Job::Run(path))) if path == Path::new("second"));
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
