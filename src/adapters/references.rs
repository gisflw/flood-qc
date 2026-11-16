use chrono::NaiveDate;
use std::{collections::HashSet, path::Path};

#[derive(Clone, Debug)]
pub struct ReferenceRow {
    pub code: String,
    pub kind: String,
    pub date: Option<NaiveDate>,
    pub level_cm: f64,
}

pub fn normalize_station_code(code: &str) -> Result<String, String> {
    let value = code.trim().to_uppercase();
    if value.is_empty() {
        return Err("empty station code".into());
    }
    let stripped = value.trim_start_matches('0');
    Ok(if stripped.is_empty() { "0" } else { stripped }.into())
}

pub struct ParsedReferences {
    pub rows: Vec<ReferenceRow>,
    pub codes: HashSet<String>,
}

pub fn read_reference_csvs(thresholds: &Path, history: &Path) -> Result<ParsedReferences, String> {
    let mut result = Vec::new();
    let mut keys = HashSet::new();
    let mut codes = HashSet::new();
    for (path, historical) in [(thresholds, false), (history, true)] {
        let mut reader = csv::Reader::from_path(path).map_err(|e| e.to_string())?;
        let headers = reader.headers().map_err(|e| e.to_string())?.clone();
        let col = |name| {
            let mut matches = headers
                .iter()
                .enumerate()
                .filter(|(_, h)| h.trim_start_matches('\u{feff}') == name);
            let index = matches
                .next()
                .map(|(i, _)| i)
                .ok_or_else(|| format!("{}: missing {name}", path.display()))?;
            if matches.next().is_some() {
                return Err(format!("{}: duplicate {name} header", path.display()));
            }
            Ok(index)
        };
        let code = col("station_code")?;
        let fields = if historical {
            vec![("historical", col("level_registered")?)]
        } else {
            let _ = col("station_name")?;
            vec![
                ("attention", col("level_atention")?),
                ("alert", col("level_alert")?),
                ("flood", col("level_flood")?),
                ("severe", col("level_severe")?),
            ]
        };
        let date = if historical { Some(col("date")?) } else { None };
        for (i, row) in reader.records().enumerate() {
            let row = row.map_err(|e| format!("{}:{}: {e}", path.display(), i + 2))?;
            let station = normalize_station_code(row.get(code).ok_or("missing station code")?)?;
            codes.insert(station.clone());
            let date = date
                .map(|n| {
                    let value = row.get(n).unwrap_or("").trim();
                    let parsed = NaiveDate::parse_from_str(value, "%Y-%m-%d")
                        .map_err(|_| format!("invalid date {value}"))?;
                    if parsed.format("%Y-%m-%d").to_string() != value {
                        return Err(format!("invalid date {value}"));
                    }
                    Ok(parsed)
                })
                .transpose()?;
            for (kind, index) in &fields {
                let value = row.get(*index).unwrap_or("").trim();
                if value.is_empty() {
                    if historical {
                        return Err("missing historical level".into());
                    }
                    continue;
                }
                let level_cm: f64 = value
                    .parse()
                    .map_err(|_| format!("invalid reference level {value}"))?;
                if !level_cm.is_finite() {
                    return Err(format!("invalid reference level {value}"));
                }
                let key = (
                    station.clone(),
                    *kind,
                    date,
                    if historical && level_cm != 0.0 {
                        level_cm.to_bits()
                    } else {
                        0
                    },
                );
                if !keys.insert(key) {
                    return Err(format!("duplicate reference for {station}"));
                }
                result.push(ReferenceRow {
                    code: station.clone(),
                    kind: (*kind).into(),
                    date,
                    level_cm,
                });
            }
        }
    }
    Ok(ParsedReferences {
        rows: result,
        codes,
    })
}
