pub(crate) mod parquet;
pub(crate) use self::geometry::parse_basin_geometry;
mod geometry;
pub use self::geometry::{
    MiniTopology, read_basin_geometry, read_mini_topology, validate_mini_topology,
};
mod forecast;
mod netcdf;
mod observations;
mod operation;
mod values;
pub(crate) use self::operation::PREPARED_ENGINE;

pub use self::observations::{
    LevelReference, ObservationBatch, ObservationError, ObservationInputs, ObservationReplacement,
    ObservationSample, ObservationSelection, ObservationSeries, ObservationStore, ObservationValue,
    ReplacementKind, Station, StationLocation, StationOffset, validate_replacement,
};
pub use self::values::{
    BasinForcing, BasinGeometry, CorrectionCoverage, ForecastCorrection, ForecastPreparation,
    ForecastProvider, ForecastSource, GridFrame, GridSeries, IdwSettings, IntervalAxis, Provenance,
    RegularGridSpec, ResultSeries, StationResults, TimeInterval, ValueError,
};

pub use self::netcdf::{
    StoreError, read_basin_forcing, read_grid_frame, read_grid_series, write_basin_forcing,
    write_grid_frame, write_grid_series,
};

pub(crate) use self::forecast::create_workspace_dir;
pub use self::forecast::{
    CanonicalForecastMetadata, ForecastAsset, ForecastAssetChoice, ForecastCatalog,
    ForecastSnapshotSelection, check_correction,
};

pub use self::operation::{
    HotStartManifest, InputReference, OperationManifest, ScenarioFailure, ScenarioRecord,
    collect_inputs, copy_tree, copy_working_inputs, digest_file, finalize_prepared,
    publish_directory, read_operation_manifest, read_retained_manifest, sync_directory, sync_tree,
    verify_inputs, write_run_manifest,
};
pub(crate) use self::operation::{copy_staged_tree, stage_working_inputs};

mod sharing;
pub(crate) use self::sharing::{BundleManifest, copy_member, payload_index, validate_bundle};

pub(crate) use self::netcdf::read_result_timeline;
pub use self::netcdf::{
    read_result_series, read_station_results, write_result_series, write_station_results,
};

mod map;
pub use self::map::{
    MapLayer, MapPolygon, MiniFlowExceedance, MiniMapFeature, read_map_layers, read_mini_map_layer,
    write_return_period_geojson,
};

mod draft;
pub use self::draft::{read_draft, write_atomic, write_draft, write_new_atomic};
mod forcing;
pub(crate) use self::forcing::whole_hour;
pub use self::forcing::{
    ForcingSelections, ObservedFlowMapping, PreparedAdditionalForcing, ReleaseExtrapolation,
    ReleaseExtrapolationMethod, ReservoirMapping, ReservoirScheduleSample, ReservoirSchedules,
    WindMapping, WindObservations, WindSample, WindVector, read_reservoir_schedules,
    read_wind_observations, write_reservoir_schedules, write_wind_observations,
};
mod calibration;
pub(crate) use self::calibration::save_calibration;
pub use self::calibration::{
    CalibrationGeometryRow, GeometryField, HydrologicClass, SoilWatershed,
};
mod automation;
pub(crate) use self::automation::{automation_documents, write_automation_document};

mod run_cache;
pub(crate) use self::run_cache::{
    RunAttemptManifest, active_run, collect_run_diagnostics, is_managed_run_path, new_run_stage,
    publish_run_checked, recover_run_cache, run_cache_root, run_stage_id, with_active_run,
    write_run_attempt, write_run_journal,
};
#[cfg(test)]
pub(crate) use self::run_cache::{new_run_stage_for, publish_run};

mod rainfall;
pub use self::rainfall::RainfallCacheInfo;
pub(crate) use self::rainfall::{
    DecodedRainfallFile, RainfallCacheFiles, rainfall_cache_info, read_rainfall_cache,
    stage_rainfall,
};

mod inundation;

pub use self::inundation::{
    FLOOD_MASK_FILE, InundationLevel, InundationMask, InundationMetadata, InundationProvenance,
    verify_inundation_run,
};
pub(crate) use self::inundation::{
    INUNDATION_TILE, InundationRasters, InundationStage, InundationWriter, cache_catchments,
};

mod flood_source;
pub(crate) use self::flood_source::{
    FLOOD_SOURCE_DIAGNOSTIC, FLOOD_SOURCE_FILE, FloodSource, read_flood_source, write_flood_source,
};

mod assistant_history;
pub use self::assistant_history::{
    ChatHistory, ChatHistoryStore, SavedActivity, SavedChat, SavedTurn,
};

mod input_sets;
pub use self::input_sets::{
    CurrentInputSet, InputSetManifest, current_input_root, current_input_set, read_input_set,
    save_input_set,
};
pub(crate) use self::input_sets::{
    clean_materialized_inputs, discard_current_inputs, publish_captured_input_set,
    save_current_inputs, stage_preparation, write_cache_json,
};

mod acquisition;
pub(crate) use self::acquisition::AnaPartialCoverage;
