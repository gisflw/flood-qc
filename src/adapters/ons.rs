use std::collections::{BTreeMap, BTreeSet};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use chrono_tz::{America::Sao_Paulo, Tz};
use serde::Deserialize;

use super::local_hour;

#[derive(Deserialize)]
struct Row<'a> {
    id_reservatorio: &'a str,
    din_instante: &'a str,
    val_vazaoafluente: &'a str,
    val_vazaodefluente: &'a str,
}

pub(crate) struct OnsHour {
    pub(crate) local_hour: NaiveDateTime,
    pub(crate) inflow: Option<f64>,
    pub(crate) outflow: Option<f64>,
}

pub(crate) struct OnsMonth {
    pub(crate) reservoirs: BTreeMap<String, Vec<OnsHour>>,
    pub(crate) non_hourly: usize,
}

fn flow(raw: &str, signed: bool) -> Result<Option<f64>, String> {
    if raw.is_empty() {
        return Ok(None);
    }
    raw.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && (signed || *v >= 0.0))
        .map(Some)
        .ok_or_else(|| "invalid ONS flow measurement".into())
}

pub(crate) fn parse_ons(
    bytes: &[u8],
    reservoirs: &BTreeSet<String>,
    start: NaiveDate,
    end: NaiveDate,
    zone: Tz,
) -> Result<OnsMonth, String> {
    let mut csv = csv::ReaderBuilder::new()
        .delimiter(b';')
        .trim(csv::Trim::All)
        .from_reader(bytes);
    for required in [
        "id_reservatorio",
        "din_instante",
        "val_vazaoafluente",
        "val_vazaodefluente",
    ] {
        if !csv
            .headers()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|name| name == required)
        {
            return Err(format!("ONS CSV is missing {required}"));
        }
    }
    let mut result = OnsMonth {
        reservoirs: BTreeMap::new(),
        non_hourly: 0,
    };
    let mut record = csv::StringRecord::new();
    let headers = csv.headers().map_err(|e| e.to_string())?.clone();
    while csv.read_record(&mut record).map_err(|e| e.to_string())? {
        let row: Row<'_> = record
            .deserialize(Some(&headers))
            .map_err(|e| e.to_string())?;
        if !reservoirs.contains(row.id_reservatorio) {
            continue;
        }
        let source =
            NaiveDateTime::parse_from_str(row.din_instante, "%Y-%m-%d %H:%M:%S").map_err(|e| {
                format!(
                    "ONS reservoir {}: invalid timestamp: {e}",
                    row.id_reservatorio
                )
            })?;
        if !(start..=end).contains(&source.date()) {
            continue;
        }
        let hour = if source.time() == NaiveTime::from_hms_opt(23, 59, 0).unwrap() {
            source
                .date()
                .succ_opt()
                .ok_or("ONS date overflow")?
                .and_time(NaiveTime::MIN)
        } else if source.minute() == 0 && source.second() == 0 {
            source
        } else {
            result.non_hourly += 1;
            continue;
        };
        let at = Sao_Paulo
            .from_local_datetime(&hour)
            .single()
            .ok_or_else(|| format!("ambiguous or nonexistent ONS hour {hour}"))?
            .to_utc();
        result
            .reservoirs
            .entry(row.id_reservatorio.into())
            .or_default()
            .push(OnsHour {
                local_hour: local_hour(at, zone)?,
                inflow: flow(row.val_vazaoafluente, true)?,
                outflow: flow(row.val_vazaodefluente, false)?,
            });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ons_normalizes_day_end_and_distinguishes_zero_from_missing() {
        let input = b"id_reservatorio;din_instante;val_vazaoafluente;val_vazaodefluente\n\
            GAR ;2026-10-31 23:59:00;-5;0\n\
            GAR;2026-10-31 22:57:00;20;25\n\
            GAR;2026-11-01 01:00:00;2;3\n\
            GAR;2026-10-31 04:00:00;2;\n";
        let date = "2026-10-31".parse().unwrap();
        let month = parse_ons(
            input,
            &BTreeSet::from(["GAR".into()]),
            date,
            date,
            Sao_Paulo,
        )
        .unwrap();
        assert_eq!(month.non_hourly, 1);
        let rows = &month.reservoirs["GAR"];
        assert_eq!(rows[0].local_hour, "2026-11-01T00:00:00".parse().unwrap());
        assert_eq!(rows[0].inflow, Some(-5.0));
        assert_eq!(rows[0].outflow, Some(0.0));
        assert_eq!(rows[1].outflow, None);
    }
}
