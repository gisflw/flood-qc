pub mod adapters;
pub mod analysis;
pub mod app;
mod model;
pub mod store;

pub use app::{
    FindingCode, FindingStatus, InspectionReport, InspectionRequest, WorkspaceConfig,
    WorkspaceEngine, WorkspaceSpatial, example_config, inspect, read_workspace_config,
    write_workspace_config,
};

pub use analysis::{
    AppliedReplacement, EffectiveObservation, ObservedPreparation, PrecipitationQcFinding,
    PrecipitationQcPolicy, ReplacementDiagnostic, SelectedObservation,
    detect_suspect_precipitation, effective_observations, prepare_observed_forcing,
    select_observation_series,
};
pub use store::{
    BasinForcing, BasinGeometry, GridFrame, GridSeries, IdwSettings, IntervalAxis,
    ObservationBatch, ObservationError, ObservationInputs, ObservationReplacement,
    ObservationSample, ObservationSelection, ObservationSeries, ObservationStore, ObservationValue,
    Provenance, RegularGridSpec, ReplacementKind, Station, StationOffset, TimeInterval, ValueError,
};

pub use app::{
    BatchStatus, EngineDocument, ExecutionReport, ExecutionRequest, ForecastsDocument,
    GeometryDocument, GridDocument, InterpolationDocument, LevelsDocument, ObservedDocument,
    OperationControl, OperationError, OperationProgress, OperationStage, OperationalRun,
    PreparationDocument, PreparationReport, PreparationRequest, ScenarioDocument,
    ScenarioExecution, ScenarioProgress, TrialCache, TypedPreparationRequest, discover_trials,
    execute_prepared, execute_prepared_controlled, prepare, prepare_controlled, prepare_typed,
    read_trial, run_draft, tail_log,
};
pub use app::{
    NormalizeForecastDocument, NormalizeForecastError, NormalizeForecastReport,
    NormalizeForecastRequest, normalize_forecast, normalize_forecast_document,
};

pub use app::{
    Draft, DraftAction, DraftCorrection, DraftDocument, DraftEngine, DraftFinding, DraftForecasts,
    DraftGeometry, DraftGrid, DraftInstructions, DraftInterpolation, DraftLevels, DraftObserved,
    DraftOperation, DraftQc, DraftReplacement, DraftScenario, DraftSources, DraftVisualization,
    ForecastPreview, ObservationPreview, RegistrationCorrectionSelection, RegistrationMapping,
    RegistrationOutcome, RegistrationReport, RegistrationSelection,
};

pub use app::{ReferenceImportReport, import_level_references, upgrade_level_references};
pub use app::{RunBundleReport, export_run, import_run, import_run_to, regenerate_run, replay_run};

pub use analysis::{
    GridAccumulation, HourlyWindow, RainSummary, RainfallSource, accumulated_grid_frame,
    accumulated_grid_rain, accumulated_rain, resolve_hourly_window, signed_rainfall_window,
};
pub use app::{
    ChartSeries, Location, QueryFinding, SeriesPoint, SeriesQuery, SeriesReport, SeriesSelection,
    Variable, export_series_csv, query_series,
};
