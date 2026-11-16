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

pub use self::forecast::{
    ForecastAsset, ForecastAssetChoice, ForecastCatalog, ForecastSnapshotSelection,
    check_correction,
};

pub use self::operation::{
    InputReference, OperationManifest, ScenarioFailure, ScenarioRecord, collect_inputs, copy_tree,
    copy_working_inputs, digest_file, finalize_prepared, read_operation_manifest,
    read_retained_manifest, sync_tree, verify_inputs, write_run_manifest,
};

pub use self::netcdf::{
    read_result_series, read_station_results, write_result_series, write_station_results,
};

mod map;
pub use self::map::{MapLayer, MiniMapFeature, read_map_layers, read_mini_map_layer};

mod draft;
pub use self::draft::{read_draft, write_atomic, write_draft};
