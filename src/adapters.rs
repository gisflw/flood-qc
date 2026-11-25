use chrono::{DateTime, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;

mod references;
pub use self::references::{
    ParsedReferences, ReferenceRow, normalize_station_code, read_reference_csvs,
};
mod ana;
mod forecast;
mod provider_http;

pub(crate) use self::ana::{
    AnaAggregation, AnaReading, AnaStation, AnaVariable, aggregate_ana, ana_csv, ana_station_code,
    parse_ana_xml, read_ana_csv, read_ana_inventory,
};
pub(crate) use self::provider_http::{
    ProviderClient, ProviderError, SourceClient, cycle_hours, forecast_leads,
};

pub use self::forecast::{
    ForecastMessageEvidence, NormalizationError, NormalizationRequest, NormalizedForecast,
    normalize_saved_forecast,
};

mod bndmet;
pub(crate) use self::bndmet::{
    BndmetProduct, hourly_bndmet_rain, hourly_bndmet_wind, parse_bndmet,
};
mod ons;
pub(crate) use self::ons::parse_ons;
mod imerg;
pub(crate) use self::imerg::{imerg_catalog, imerg_hour_depth, read_imerg_granule};

fn local_hour(at: DateTime<Utc>, zone: Tz) -> Result<NaiveDateTime, String> {
    let local = at.with_timezone(&zone).naive_local();
    if local.minute() != 0
        || local.second() != 0
        || local.nanosecond() != 0
        || zone.from_local_datetime(&local).single().is_none()
    {
        return Err(format!(
            "{at} does not have an unambiguous hour label in {zone}"
        ));
    }
    Ok(local)
}

const OBSERVATION_CSV_HEADER: [&str; 5] = [
    "provider_code",
    "station_code",
    "observed_at",
    "variable_code",
    "value",
];

pub(crate) fn observation_csv<'a>(
    rows: impl IntoIterator<Item = (&'a str, &'a str, NaiveDateTime, &'a str, Option<f64>)>,
) -> Result<Vec<u8>, String> {
    let mut csv = csv::Writer::from_writer(Vec::new());
    csv.write_record(OBSERVATION_CSV_HEADER)
        .map_err(|e| e.to_string())?;
    for (provider, station, at, variable, value) in rows {
        csv.serialize((
            provider,
            station,
            at.format("%Y-%m-%dT%H:%M:%S").to_string(),
            variable,
            value,
        ))
        .map_err(|e| e.to_string())?;
    }
    csv.into_inner().map_err(|e| e.to_string())
}
