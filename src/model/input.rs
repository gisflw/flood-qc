use crate::store::{BasinForcing, BasinGeometry};
use chrono::{Datelike, NaiveDateTime, Timelike};
use std::{
    fs,
    io::{BufWriter, Write},
    path::Path,
};

enum InputFailure {
    Invalid(String),
    Io(String),
}
impl From<&str> for InputFailure {
    fn from(message: &str) -> Self {
        Self::Invalid(message.into())
    }
}
impl From<String> for InputFailure {
    fn from(message: String) -> Self {
        Self::Invalid(message)
    }
}
impl InputFailure {
    fn classify(self, invalid: fn(String) -> super::ModelError) -> super::ModelError {
        match self {
            Self::Invalid(message) => invalid(message),
            Self::Io(message) => super::ModelError::Io(message),
        }
    }
}

fn read_basin_geometry_inner(path: &Path) -> Result<BasinGeometry, InputFailure> {
    let content = fs::read_to_string(path).map_err(|error| InputFailure::Io(error.to_string()))?;
    crate::store::parse_basin_geometry(&content).map_err(InputFailure::Invalid)
}

fn write_rainfall_inner(path: &Path, forcing: &BasinForcing) -> Result<(), InputFailure> {
    forcing.validate().map_err(|error| error.to_string())?;
    if forcing
        .values
        .iter()
        .zip(&forcing.valid)
        .any(|(value, valid)| !valid || *value < 0.0 || !(*value as f32).is_finite())
    {
        return Err("invalid engine rainfall".into());
    }
    forcing
        .values
        .len()
        .checked_mul(4)
        .ok_or("rainfall length overflow")?;
    let file = fs::File::create(path).map_err(|error| InputFailure::Io(error.to_string()))?;
    let mut writer = BufWriter::with_capacity(64 * 1024, file);
    for value in &forcing.values {
        writer
            .write_all(&(*value as f32).to_le_bytes())
            .map_err(|error| InputFailure::Io(error.to_string()))?;
    }
    writer
        .flush()
        .and_then(|()| writer.get_ref().sync_all())
        .map_err(|error| InputFailure::Io(error.to_string()))
}

fn metadata_row(lines: &[&[u8]], headers: &[&[u8]]) -> Result<usize, String> {
    let mut matches = lines.iter().enumerate().filter(|(_, line)| {
        headers.iter().all(|header| {
            line.windows(header.len())
                .any(|word| word.eq_ignore_ascii_case(header))
        })
    });
    let (index, _) = matches.next().ok_or("missing metadata header")?;
    if matches.next().is_some() {
        return Err("duplicate metadata header".into());
    }
    (index + 1..lines.len())
        .find(|index| {
            let line = lines[*index].trim_ascii_start();
            !line.is_empty() && line[0] != b'!'
        })
        .ok_or_else(|| "missing metadata data row".into())
}

pub(super) fn metadata_fields<'a>(
    bytes: &'a [u8],
    headers: &[&[u8]],
) -> Result<Vec<&'a str>, String> {
    let lines: Vec<_> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    metadata_tokens(lines[metadata_row(&lines, headers)?])
}

fn metadata_tokens(row: &[u8]) -> Result<Vec<&str>, String> {
    let body = row.split(|byte| *byte == b'!').next().unwrap_or_default();
    Ok(std::str::from_utf8(body)
        .map_err(|_| "invalid metadata fields")?
        .split_ascii_whitespace()
        .collect())
}

fn edit_line(bytes: &[u8], headers: &[&[u8]], fields: &[String]) -> Result<Vec<u8>, String> {
    let mut lines: Vec<_> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    let index = metadata_row(&lines, headers)?;
    let original = lines[index];
    let tokens = metadata_tokens(original)?;
    if tokens.len() != fields.len() || tokens.iter().any(|token| token.parse::<f64>().is_err()) {
        return Err("invalid metadata fields".into());
    }
    let mut replacement = Vec::new();
    let mut position = 0;
    let mut field = 0;
    while position < original.len() {
        if original[position] == b'!' {
            replacement.extend_from_slice(&original[position..]);
            break;
        }
        if original[position].is_ascii_whitespace() {
            replacement.push(original[position]);
            position += 1;
            continue;
        }
        let start = position;
        while position < original.len()
            && !original[position].is_ascii_whitespace()
            && original[position] != b'!'
        {
            position += 1;
        }
        if field >= fields.len() {
            return Err("unexpected metadata field".into());
        }
        replacement.extend_from_slice(fields[field].as_bytes());
        field += 1;
        if position == start {
            return Err("invalid metadata row".into());
        }
    }
    if field != fields.len() {
        return Err("missing metadata field".into());
    }
    lines[index] = &replacement;
    Ok(lines.concat())
}

fn write_metadata_inner(
    input: &Path,
    first: NaiveDateTime,
    forecast_first: NaiveDateTime,
    total: usize,
    horizon: usize,
    basins: &BasinGeometry,
) -> Result<(), InputFailure> {
    if first.minute() != 0
        || first.second() != 0
        || forecast_first.minute() != 0
        || forecast_first.second() != 0
        || total == 0
        || horizon >= total
    {
        return Err("invalid engine timing".into());
    }
    let mini = read_basin_geometry_inner(&input.join("MINI.gtp"))?;
    if mini.mini_ids != basins.mini_ids
        || mini.longitudes != basins.longitudes
        || mini.latitudes != basins.latitudes
    {
        return Err("MINI geometry mismatch".into());
    }
    let par_path = input.join("PARHIG.hig");
    let par = fs::read(&par_path).map_err(|error| InputFailure::Io(error.to_string()))?;
    let par = edit_line(
        &par,
        &[b"DIA", b"MES", b"ANO", b"HORA"],
        &[
            first.day().to_string(),
            first.month().to_string(),
            first.year().to_string(),
            first.hour().to_string(),
        ],
    )?;
    let par = edit_line(&par, &[b"NT", b"DT"], &[total.to_string(), "3600.".into()])?;
    let meta_path = input.join("Previsao.meta");
    let meta = fs::read(&meta_path).map_err(|error| InputFailure::Io(error.to_string()))?;
    let lines: Vec<_> = meta.split(|byte| *byte == b'\n').collect();
    let matches: Vec<_> = lines
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line.windows(b"NUMERO DE MINI-BACIAS".len())
                .any(|word| word.eq_ignore_ascii_case(b"NUMERO DE MINI-BACIAS"))
        })
        .map(|(index, _)| index)
        .collect();
    if matches.len() != 1 {
        return Err("invalid forecast basin header".into());
    }
    let basin_line = lines
        .iter()
        .skip(matches[0] + 1)
        .find(|line| {
            let value = line.trim_ascii();
            !value.is_empty() && value[0] != b'!'
        })
        .ok_or("missing forecast basin count")?;
    let basin_count = std::str::from_utf8(basin_line.trim_ascii())
        .map_err(|_| "invalid forecast basin count")?
        .parse::<usize>()
        .map_err(|_| "invalid forecast basin count")?;
    if basin_count != basins.mini_ids.len() {
        return Err("forecast basin count mismatch".into());
    }
    let meta = edit_line(
        &meta,
        &[b"HORIZ(NT)"],
        &[
            forecast_first.year().to_string(),
            forecast_first.month().to_string(),
            forecast_first.day().to_string(),
            forecast_first.hour().to_string(),
            horizon.to_string(),
        ],
    )?;
    fs::write(par_path, par).map_err(|error| InputFailure::Io(error.to_string()))?;
    fs::write(meta_path, meta).map_err(|error| InputFailure::Io(error.to_string()))?;
    Ok(())
}

pub(crate) fn read_basin_geometry(path: &Path) -> Result<BasinGeometry, super::ModelError> {
    read_basin_geometry_inner(path)
        .map_err(|error| error.classify(super::ModelError::InvalidGeometry))
}
pub(crate) fn write_rainfall(path: &Path, forcing: &BasinForcing) -> Result<(), super::ModelError> {
    write_rainfall_inner(path, forcing)
        .map_err(|error| error.classify(super::ModelError::InvalidForcing))
}
pub(crate) fn write_metadata(
    input: &Path,
    first: NaiveDateTime,
    forecast_first: NaiveDateTime,
    total: usize,
    horizon: usize,
    basins: &BasinGeometry,
) -> Result<(), super::ModelError> {
    write_metadata_inner(input, first, forecast_first, total, horizon, basins)
        .map_err(|error| error.classify(super::ModelError::InvalidTiming))
}

#[cfg(test)]
mod tests {
    use super::edit_line;
    #[test]
    fn rainfall_is_time_major_little_endian_and_rejects_invalid_values() {
        use crate::store::{BasinForcing, BasinGeometry, IntervalAxis, Provenance, TimeInterval};
        let start = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let provenance = Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        };
        let mut forcing = BasinForcing {
            timeline: IntervalAxis {
                time_zone: "UTC".into(),
                bounds: (0..2)
                    .map(|hour| TimeInterval {
                        start: start + chrono::Duration::hours(hour),
                        end: start + chrono::Duration::hours(hour + 1),
                    })
                    .collect(),
            },
            basins: BasinGeometry {
                mini_ids: vec![1, 2],
                longitudes: vec![0.0, 1.0],
                latitudes: vec![0.0, 1.0],
                source: provenance.clone(),
            },
            values: vec![1.0, 2.0, 3.0, 4.0],
            valid: vec![true; 4],
            provenance,
        };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHUVABIN.hig");
        super::write_rainfall(&path, &forcing).unwrap();
        let expected: Vec<u8> = [1.0_f32, 2.0, 3.0, 4.0]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        assert_eq!(std::fs::read(&path).unwrap(), expected);
        forcing.valid[0] = false;
        let is_expected = matches!(
            super::write_rainfall(&path, &forcing),
            Err(crate::model::ModelError::InvalidForcing(_))
        );
        assert!(is_expected);
        forcing.valid[0] = true;
        forcing.values[0] = f64::MAX;
        let is_expected = matches!(
            super::write_rainfall(&path, &forcing),
            Err(crate::model::ModelError::InvalidForcing(_))
        );
        assert!(is_expected);
    }
    #[test]
    fn missing_geometry_is_an_io_error() {
        let dir = tempfile::tempdir().unwrap();
        let is_expected = matches!(
            super::read_basin_geometry(&dir.path().join("missing")),
            Err(crate::model::ModelError::Io(_))
        );
        assert!(is_expected);
    }
    #[test]
    fn edits_only_ascii_data_preserving_comments_and_encoding() {
        let bytes = b"!\xff\r\n DIA MES ANO HORA\r\n 01 02 2000 00\r\n!tail\r\n";
        let edited = edit_line(
            bytes,
            &[b"DIA", b"MES", b"ANO", b"HORA"],
            &["10".into(), "7".into(), "2026".into(), "0".into()],
        )
        .unwrap();
        assert_eq!(
            edited,
            b"!\xff\r\n DIA MES ANO HORA\r\n 10 7 2026 0\r\n!tail\r\n"
        );
        assert!(edit_line(bytes, &[b"missing"], &[]).is_err());
    }
}
