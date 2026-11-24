use super::ModelError;
use crate::store::{IntervalAxis, PreparedAdditionalForcing};
use chrono::{Datelike, Timelike};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{self, BufWriter, Write},
    path::Path,
};

const QOBS_FILE: &str = "QOBS_hora.PRN";
const RESERVOIR_FILE: &str = "QSUBST_hora.PRN";

fn columns(par: &[u8], file: &str) -> Result<Vec<i64>, ModelError> {
    let invalid = || ModelError::InvalidForcing(format!("invalid {file} columns in PARHIG.hig"));
    let mut rows = par.split(|byte| *byte == b'\n').filter_map(|line| {
        let data = line.split(|byte| *byte == b'!').next()?.trim_ascii();
        (!data.is_empty()).then_some(data)
    });
    let count = loop {
        let row = rows.next().ok_or_else(invalid)?;
        let fields: Vec<_> = row
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
            .collect();
        if fields.get(1) == Some(&file.as_bytes()) {
            let count = std::str::from_utf8(fields[0])
                .ok()
                .and_then(|field| field.parse::<usize>().ok())
                .filter(|count| *count > 0 && *count <= 300)
                .ok_or_else(invalid)?;
            break count;
        }
    };
    let parse_id = |field| {
        std::str::from_utf8(field)
            .ok()
            .and_then(|field| field.parse::<i64>().ok())
            .filter(|id| *id > 0)
            .ok_or_else(invalid)
    };
    let mut slots = Vec::with_capacity(count);
    if file == QOBS_FILE {
        let row = rows.next().ok_or_else(invalid)?;
        for field in row
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
        {
            slots.push(parse_id(field)?);
        }
    } else {
        for _ in 0..count {
            let row = rows.next().ok_or_else(invalid)?;
            let field = row
                .split(|byte| byte.is_ascii_whitespace())
                .next()
                .ok_or_else(invalid)?;
            slots.push(parse_id(field)?);
        }
        // The RS engine overwrites these two configured substitution cells in flood_discharge.
        if slots.get(2) != Some(&8468) || slots.get(12) != Some(&8181) {
            return Err(invalid());
        }
    }
    let unique: HashSet<_> = slots.iter().collect();
    if slots.len() != count
        || unique.len() != count
        || rows.any(|row| {
            row.split(|byte| byte.is_ascii_whitespace())
                .any(|field| field == file.as_bytes())
        })
    {
        return Err(invalid());
    }
    Ok(slots)
}

fn write_table(
    mut file: impl Write,
    timeline: &IntervalAxis,
    columns: &[i64],
    flows: &BTreeMap<i64, Vec<Option<f64>>>,
) -> io::Result<()> {
    write!(file, "   Dia   Mes   Ano  Hora")?;
    for mini_id in columns {
        write!(file, "{mini_id:>10}")?;
    }
    writeln!(file)?;
    for (step, interval) in timeline.bounds.iter().enumerate() {
        let date = interval.end;
        write!(
            file,
            "{:6}{:6}{:6}{:6}",
            date.day(),
            date.month(),
            date.year(),
            date.hour()
        )?;
        for mini_id in columns {
            let value = flows
                .get(mini_id)
                .and_then(|values| values[step])
                .unwrap_or(-1.0);
            // The engine reads list-directed values, retaining sub-tenth observations.
            write!(file, " {value}")?;
        }
        writeln!(file)?;
    }
    Ok(())
}

fn io_at(path: &Path, error: io::Error) -> ModelError {
    ModelError::Io(format!("{}: {error}", path.display()))
}

fn replace_file(
    path: &Path,
    writer: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> Result<(), ModelError> {
    let mut file = BufWriter::new(File::create(path).map_err(|e| io_at(path, e))?);
    writer(&mut file)
        .and_then(|()| file.flush())
        .and_then(|()| file.get_ref().sync_all())
        .map_err(|e| io_at(path, e))
}

pub(crate) fn write_additional_forcing(
    input: &Path,
    forcing: &PreparedAdditionalForcing,
) -> Result<(), ModelError> {
    forcing
        .validate()
        .map_err(|error| ModelError::InvalidForcing(error.to_string()))?;
    if forcing
        .timeline
        .bounds
        .iter()
        .any(|interval| !(1..=9999).contains(&interval.end.year()))
    {
        return Err(ModelError::InvalidTiming(
            "forcing year must fit the four-digit engine timestamp".into(),
        ));
    }
    let tables = [
        (QOBS_FILE, &forcing.qobs, !forcing.qobs.is_empty()),
        (
            RESERVOIR_FILE,
            &forcing.reservoirs,
            forcing.disable_reservoirs || !forcing.reservoirs.is_empty(),
        ),
    ];
    if tables.iter().any(|table| table.2) {
        let path = input.join("PARHIG.hig");
        let par = fs::read(&path).map_err(|e| io_at(&path, e))?;
        let mut layouts = Vec::new();
        for (name, flows, _) in tables.into_iter().filter(|table| table.2) {
            let slots = columns(&par, name)?;
            if let Some(mini) = flows.keys().find(|mini| !slots.contains(mini)) {
                return Err(ModelError::InvalidForcing(format!(
                    "{name} has no column for mini-basin {mini}"
                )));
            }
            layouts.push((name, flows, slots));
        }
        for (name, flows, slots) in layouts {
            replace_file(&input.join(name), |file| {
                write_table(file, &forcing.timeline, &slots, flows)
            })?;
        }
    }
    if let Some(wind) = &forcing.wind {
        replace_file(&input.join("vento.txt"), |file| {
            for value in wind {
                match value {
                    Some(value) => {
                        writeln!(file, "{} {}", value.speed_mps, value.direction_degrees)?
                    }
                    None => writeln!(file, "-1 -1")?,
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{TimeInterval, WindVector};
    use chrono::Duration;

    #[test]
    fn serializers_keep_column_order_time_labels_and_missing_distinct_from_zero() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        fs::write(
            path.join("PARHIG.hig"),
            b"2 QOBS_hora.PRN\r\n! Cells\r\n43 12\r\n",
        )
        .unwrap();
        fs::write(path.join(RESERVOIR_FILE), b"baseline").unwrap();
        let first: chrono::NaiveDateTime = "2026-07-31T23:00:00".parse().unwrap();
        let mut prepared = PreparedAdditionalForcing {
            timeline: IntervalAxis {
                time_zone: "UTC".into(),
                bounds: (0..2)
                    .map(|hour| TimeInterval {
                        start: first + Duration::hours(hour - 1),
                        end: first + Duration::hours(hour),
                    })
                    .collect(),
            },
            qobs: [(12, vec![Some(0.0), Some(7.25)])].into(),
            reservoirs: BTreeMap::new(),
            wind: Some(vec![
                Some(WindVector {
                    speed_mps: 0.0,
                    direction_degrees: 90.0,
                }),
                None,
            ]),
            disable_reservoirs: false,
        };
        write_additional_forcing(path, &prepared).unwrap();
        let qobs = fs::read_to_string(path.join(QOBS_FILE)).unwrap();
        let mut rows = qobs
            .lines()
            .map(|row| row.split_whitespace().collect::<Vec<_>>());
        assert_eq!(
            rows.next().unwrap(),
            ["Dia", "Mes", "Ano", "Hora", "43", "12"]
        );
        assert_eq!(rows.next().unwrap(), ["31", "7", "2026", "23", "-1", "0"]);
        assert_eq!(rows.next().unwrap(), ["1", "8", "2026", "0", "-1", "7.25"]);
        assert_eq!(
            fs::read_to_string(path.join("vento.txt")).unwrap(),
            "0 90\n-1 -1\n"
        );
        assert_eq!(fs::read(path.join(RESERVOIR_FILE)).unwrap(), b"baseline");
        prepared.qobs.insert(100, vec![None, None]);
        assert!(write_additional_forcing(path, &prepared).is_err());
    }
}
