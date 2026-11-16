mod references;
pub use self::references::{
    ParsedReferences, ReferenceRow, normalize_station_code, read_reference_csvs,
};
mod forecast;

pub use self::forecast::{
    ForecastMessageEvidence, NormalizationError, NormalizationRequest, NormalizedForecast,
    normalize_saved_forecast,
};
