mod summary;
pub use self::summary::{
    GridAccumulation, HourlyWindow, RainSummary, RainfallSource, accumulated_basin_forcing,
    accumulated_grid_frame, accumulated_grid_rain, accumulated_rain, resolve_hourly_window,
    signed_rainfall_window,
};
pub(crate) use self::summary::{accumulated_valid_basin_forcing, accumulated_valid_grid_rain};
mod topology;
pub use self::topology::{mini_area, upstream_minis};
mod forecast;
mod observations;
mod spatial;

pub use self::observations::{
    AppliedReplacement, AvailabilityState, EffectiveObservation, ObservedPreparation,
    PrecipitationQcFinding, PrecipitationQcPolicy, ReplacementDiagnostic, ReviewVariable,
    SelectedObservation, SpatialQcHour, SpatialQcStation, detect_suspect_precipitation,
    effective_flow_observations, effective_observations, effective_review_observations,
    prepare_observed_forcing, select_observation_series, spatial_precipitation_qc,
};
pub use self::spatial::{
    GridIdwNeighbors, SpatialError, bilinear_resample, build_grid_idw_neighbors,
    interpolate_grid_to_basins, interpolate_stations,
};

pub(crate) use self::forecast::forecast_interval_selection;
pub use self::forecast::{
    ForecastPreparationError, ForecastPreparationRequest, corrected_forecast_frames,
    prepare_forecast, prepare_zero_forecast,
};

mod forcing;
pub use self::forcing::prepare_additional_forcing;
mod calibration;
pub(crate) use self::calibration::{channel_dimension, channel_table};
mod return_period;
pub use self::return_period::{FlowReturnPeriod, forecast_day_exceedance};
mod bulletin;
pub use self::bulletin::{
    BulletinContent, BulletinFigure, BulletinTable, bulletin_html, bulletin_markdown, bulletin_pdf,
};
mod report_graphics;
pub use self::report_graphics::{render_hydrograph_png, render_rain_map_png};
mod hydrograph;
pub use self::hydrograph::{
    HydrographAdjustment, HydrographPeak, HydrographPoint, HydrographThreshold, ObservationBias,
    RatingPoint, analyze_model_hydrograph, rating_value, threshold_status, validate_hydrograph,
};

#[cfg(feature = "inundation-write")]
mod inundation;
#[cfg(feature = "inundation-write")]
pub(crate) use self::inundation::{classify, wet_fraction};
