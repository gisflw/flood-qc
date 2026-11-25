use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeDelta, TimeZone, Timelike, Utc};
use chrono_tz::{America::Sao_Paulo, Tz};
use serde_json::Value;

use super::local_hour;
use crate::store::{WindSample, WindVector};

#[derive(Clone, Copy)]
pub(crate) enum BndmetProduct {
    Rain,
    Speed,
    Direction,
}

impl BndmetProduct {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Rain => "I175",
            Self::Speed => "I111",
            Self::Direction => "I113",
        }
    }
}

type Readings = BTreeMap<DateTime<Utc>, Option<f64>>;

fn timestamp(value: &Value) -> Result<DateTime<Utc>, String> {
    if let Some(text) = value.as_str() {
        let text = text.trim();
        if let Ok(time) = DateTime::parse_from_rfc3339(text) {
            return Ok(time.to_utc());
        }
        for format in [
            "%Y-%m-%dT%H:%M:%S%.f",
            "%Y-%m-%d %H:%M:%S%.f",
            "%Y-%m-%dT%H:%M",
            "%Y-%m-%d %H:%M",
            "%d/%m/%Y %H:%M",
        ] {
            if let Ok(local) = NaiveDateTime::parse_from_str(text, format) {
                return Sao_Paulo
                    .from_local_datetime(&local)
                    .single()
                    .map(|at| at.to_utc())
                    .ok_or_else(|| format!("ambiguous or nonexistent BNDMET local time {local}"));
            }
        }
    }
    value
        .as_i64()
        .and_then(DateTime::from_timestamp_millis)
        .ok_or_else(|| "invalid BNDMET timestamp".into())
}

fn measurement(value: &Value, product: BndmetProduct) -> Result<Option<f64>, String> {
    if value.is_null() || value.as_str().is_some_and(|s| matches!(s.trim(), "" | "-")) {
        return Ok(None);
    }
    let number = value
        .as_f64()
        .or_else(|| value.as_str()?.trim().parse().ok())
        .filter(|n| n.is_finite() && *n >= 0.0)
        .filter(|n| !matches!(product, BndmetProduct::Direction) || *n <= 360.0)
        .ok_or_else(|| format!("invalid BNDMET {} measurement", product.code()))?;
    Ok(Some(number))
}

pub(crate) fn parse_bndmet(
    bytes: &[u8],
    station: &str,
    product: BndmetProduct,
    start: NaiveDate,
    end: NaiveDate,
) -> Result<Readings, String> {
    let payload: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let rows = payload
        .as_array()
        .or_else(|| payload.get("data")?.as_array())
        .or_else(|| payload.pointer("/data/data")?.as_array())
        .ok_or("unexpected BNDMET response schema")?;
    let mut readings = Readings::new();
    for row in rows {
        let (at, raw) = if let Some(pair) = row.as_array() {
            let [at, value, ..] = pair.as_slice() else {
                return Err("short BNDMET timestamp/value pair".into());
            };
            (at, value)
        } else {
            if let Some(code) = row.get("codigo").or_else(|| row.get("station_code"))
                && !code
                    .as_str()
                    .is_some_and(|s| s.trim().eq_ignore_ascii_case(station))
            {
                return Err("BNDMET response contains a different station".into());
            }
            let at = [
                "timestamp",
                "time",
                "datetime",
                "observed_at",
                "dataHora",
                "date",
            ]
            .iter()
            .find_map(|key| row.get(key).filter(|v| !v.is_null()))
            .ok_or("missing BNDMET timestamp")?;
            let raw = row
                .get("value")
                .or_else(|| {
                    matches!(product, BndmetProduct::Rain)
                        .then(|| row.get("chuva").or_else(|| row.get("rain")))
                        .flatten()
                })
                .ok_or("missing BNDMET measurement field")?;
            (at, raw)
        };
        let at = timestamp(at)?;
        if !(start..=end).contains(&at.with_timezone(&Sao_Paulo).date_naive()) {
            continue;
        }
        let value = measurement(raw, product)?;
        if readings.insert(at, value).is_some_and(|old| old != value) {
            return Err(format!("conflicting BNDMET readings at {at}"));
        }
    }
    Ok(readings)
}

fn hour_end(at: DateTime<Utc>) -> Result<DateTime<Utc>, String> {
    let floor = at
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap();
    if floor == at {
        Ok(at)
    } else {
        floor
            .checked_add_signed(TimeDelta::hours(1))
            .ok_or_else(|| "rain hour overflow".into())
    }
}

pub(crate) fn hourly_bndmet_rain(
    readings: Readings,
    zone: Tz,
) -> Result<Vec<(NaiveDateTime, Option<f64>)>, String> {
    let mut hours = BTreeMap::<_, Option<f64>>::new();
    for (at, value) in readings {
        let slot = hours.entry(hour_end(at)?).or_default();
        if let Some(value) = value {
            let total = slot.unwrap_or_default() + value;
            if !total.is_finite() {
                return Err("BNDMET hourly rainfall overflow".into());
            }
            *slot = Some(total);
        }
    }
    hours
        .into_iter()
        .map(|(at, value)| Ok((local_hour(at, zone)?, value)))
        .collect()
}

pub(crate) fn hourly_bndmet_wind(
    station: &str,
    speed: &Readings,
    direction: &Readings,
    zone: Tz,
) -> Result<Vec<WindSample>, String> {
    let hours: std::collections::BTreeSet<_> = speed.keys().chain(direction.keys()).collect();
    hours
        .into_iter()
        .map(|at| {
            let wind = speed
                .get(at)
                .copied()
                .flatten()
                .zip(direction.get(at).copied().flatten())
                .map(|(speed_mps, direction_degrees)| WindVector {
                    speed_mps,
                    direction_degrees,
                });
            Ok(WindSample {
                series_id: format!("inmet:{station}"),
                at: local_hour(*at, zone)?,
                wind,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_replays_legacy_schema_without_double_counting_or_filling_missing() {
        let day = "2026-11-22".parse().unwrap();
        let raw = br#"{"data":{"data":[["2026-11-22T11:30:00Z",1.25],
            ["2026-11-22T08:30:00",1.25],{"timestamp":"2026-11-22T09:00:00-03:00","chuva":"0"},
            ["22/11/2026 10:00","-"]]}}"#;
        let readings = parse_bndmet(raw, "A801", BndmetProduct::Rain, day, day).unwrap();
        let result = hourly_bndmet_rain(readings, Sao_Paulo).unwrap();
        assert_eq!(
            result,
            [
                ("2026-11-22T09:00:00".parse().unwrap(), Some(1.25)),
                ("2026-11-22T10:00:00".parse().unwrap(), None)
            ]
        );
        assert!(
            parse_bndmet(
                br#"{"message":"denied"}"#,
                "A801",
                BndmetProduct::Rain,
                day,
                day
            )
            .is_err()
        );
    }

    #[test]
    fn wind_pairs_by_instant_and_keeps_unpaired_values_missing() {
        let day = "2026-11-22".parse().unwrap();
        let speed = parse_bndmet(
            br#"[["2026-11-22T09:00:00",0],["2026-11-22T10:00:00",2]]"#,
            "A801",
            BndmetProduct::Speed,
            day,
            day,
        )
        .unwrap();
        let direction = parse_bndmet(
            br#"{"data":[["2026-11-22T12:00:00Z",360]]}"#,
            "A801",
            BndmetProduct::Direction,
            day,
            day,
        )
        .unwrap();
        let rows = hourly_bndmet_wind("A801", &speed, &direction, Sao_Paulo).unwrap();
        assert_eq!(rows[0].wind.unwrap().speed_mps, 0.0);
        assert_eq!(rows[0].wind.unwrap().direction_degrees, 360.0);
        assert_eq!(rows[1].wind, None);
    }
}
