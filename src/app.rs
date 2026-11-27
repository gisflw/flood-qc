mod observation_review;
pub use self::observation_review::{
    AvailabilityHour, ObservationReviewReport, ObservationReviewRequest, ReviewWindow,
    SpatialObservationQcReport, StationAvailability, review_observations, spatial_observation_qc,
};
mod series;
pub use self::series::{
    ChartSeries, Location, QueryFinding, SeriesPoint, SeriesQuery, SeriesReport, SeriesSelection,
    Variable, export_comparison_csv, export_series_csv, query_comparison, query_series,
};
mod references;
pub use self::references::{ReferenceImportReport, import_level_references};
mod migration_export;
mod operation;
mod operational_run;
mod rainfall;
pub use self::operational_run::{
    ForecastAdjustment, ForecastPreview, OperationalRun, OperationalRunReport, RunParameters,
    RunScenario, ScenarioBase,
};
pub use self::rainfall::{
    CachedRainfall, RainfallCacheReader, RainfallPeriod, query_cached_rainfall, rainfall_cache_info,
};
mod request;
mod sharing;
pub use self::migration_export::{
    HydrographBatchItem, export_hydrograph_batch, publish_automation_artifacts,
};
pub use self::operation::{
    BatchStatus, ExecutionReport, ExecutionRequest, OperationControl, OperationError,
    OperationEvent, OperationLogStream, OperationProgress, OperationStage, PreparationReport,
    PreparationRequest, ScenarioExecution, ScenarioProgress, TrialCache, discover_trials,
    execute_prepared, execute_prepared_controlled, read_trial, tail_log, with_active_run,
};
pub use self::request::{
    CalibrationDocument, EngineDocument, ForcingDocument, ForecastsDocument, GeometryDocument,
    GridDocument, HistoricalRainfall, HotStartDocument, InterpolationDocument, LevelsDocument,
    ObservedDocument, PreparationDocument, RoutingMode, ScenarioDocument, retained_day_horizons,
};
pub use self::sharing::{RunBundleReport, export_run, import_run, import_run_to, regenerate_run};
mod acquisition;
mod forecast;
pub use self::acquisition::{
    AcquisitionKind, AcquisitionReport, AcquisitionRequest, AnaAcquisitionReport,
    AnaStationOutcome, DownloadEvent, ForecastAcquisitionOutcome, acquire_inputs,
    export_workspace_inventory, import_ana_csv, import_workspace_inventory,
};
pub use self::forecast::{
    CanonicalForecastDocument, NormalizeForecastDocument, NormalizeForecastError,
    NormalizeForecastReport, NormalizeForecastRequest, import_canonical_forecast,
    import_workspace_forecast, normalize_forecast, normalize_forecast_document,
    normalize_workspace_forecast,
};

mod additional;
mod prepare;
pub use self::additional::{CanonicalForcingKind, save_canonical_forcing};
pub use self::prepare::{
    ObservedInputs, ObservedRequest, TypedPreparationRequest, prepare, prepare_controlled,
    prepare_observations, prepare_typed,
};

mod edits;
pub use self::edits::{
    ObservationEdit, export_workspace_edits, import_workspace_edits,
    save_workspace_observation_edits, workspace_observation_edits,
};
mod workspace;
use self::workspace::absolute;
pub use self::workspace::{
    Finding, FindingCode, FindingStatus, InspectionReport, InspectionRequest, WorkspaceAcquisition,
    WorkspaceConfig, WorkspaceEngine, WorkspaceRun, WorkspaceSpatial, example_config,
    forecast_assets, initialize_workspace, inspect, observation_assets, read_workspace_config,
    workspace_config_path, write_workspace_config,
};

mod calibration;
pub use self::calibration::{
    CalibrationGeometry, CalibrationGeometryRow, ChannelDimension, GeometryEdit, GeometryField,
    HydrologicClass, SoilCalibration, SoilEdit, SoilWatershed, validate_selected_calibration,
};
mod automation;
pub use self::automation::{
    AutomationConfig, AutomationControl, AutomationJob, AutomationJobStatus, AutomationPostAction,
    AutomationReportConfiguration, AutomationStage, AutomationStageStatus, AutomationState,
    AutomationStep, AutomationStepKind, SmtpConfiguration, SmtpTls, automation_status,
    read_automation_config, run_automation, run_automation_due, run_automation_now,
    save_automation_config, send_automation_email,
};
mod return_period;
pub use self::return_period::{ReturnPeriodExport, ReturnPeriodRequest, export_return_period};
mod bulletin;
pub use self::bulletin::{
    BulletinExport, BulletinPreview, BulletinProfile, BulletinRequest, BulletinStation,
    generate_bulletin, preview_bulletin,
};
mod hydrograph;
pub use self::hydrograph::{
    HydrographCurve, HydrographReport, HydrographRequest, analyze_hydrograph, export_hydrograph,
};

mod inundation;
pub use self::inundation::{
    InundationProcessReport, InundationProcessRequest, InundationProcessStatus, process_inundation,
    process_workspace_inundation,
};

mod session;
pub use self::session::{
    RunSelection, RunSetupRevision, RunSetupSnapshot, RunSnapshot, RunSummary, Session,
    WorkspaceJob, read_run_summary,
};

mod assistant;
pub use self::assistant::{
    AgentPreset, AgentSetting, AgentSettings, AssistantEvent, AssistantStage, Conversation,
    FieldChange, PermissionOption, ProposalReview, SETUP_NOTICE, SettingChoice, SettingKind,
    assistant_mcp_stdio,
};

mod source_acquisition;
pub use self::source_acquisition::{
    AcquisitionSource, SourceAcquisitionReport, SourceAcquisitionRequest, acquire_source,
};

mod input_sets;
pub use self::input_sets::{
    capture_run_inputs, capture_workspace_inputs, current_workspace_inputs,
    discard_workspace_inputs, execute_input_set, save_workspace_inputs,
};
mod registry;
pub use self::registry::register_imerg_acquisition;
pub use self::registry::{
    imerg_choices, import_registered_observations, keep_workspace_results,
    register_workspace_imerg, retained_workspace_results,
};

mod legacy_conversion;
pub use self::legacy_conversion::{convert_saved_inputs, convert_workspace};
