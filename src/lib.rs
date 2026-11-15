pub mod adapters;
pub mod analysis;
pub mod app;
mod model;
pub mod store;

pub use app::{
    FindingCode, FindingStatus, InspectionReport, InspectionRequest, example_config, inspect,
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
    BatchStatus, ExecutionReport, ExecutionRequest, OperationError, PreparationReport,
    PreparationRequest, ScenarioExecution, execute_prepared, prepare,
};
pub use app::{
    NormalizeForecastError, NormalizeForecastReport, NormalizeForecastRequest, normalize_forecast,
};
