use crate::store::{BasinForcing, ResultSeries, StationOffset, StationResults};
use std::collections::HashSet;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct DecodedEngineResults {
    pub forcing: BasinForcing,
    pub time_segment: Vec<u8>,
    pub flow: Vec<f64>,
    pub flow_valid: Vec<bool>,
    pub level_m: Vec<f64>,
    pub level_valid: Vec<bool>,
}

pub(super) fn read_stream_bytes(path: &Path, count: usize) -> Result<Vec<u8>, super::ModelError> {
    let incorrect_length = || {
        super::ModelError::MalformedOutput(format!("{}: incorrect stream length", path.display()))
    };
    let io = |error: std::io::Error| super::ModelError::Io(format!("{}: {error}", path.display()));
    let expected = count.checked_mul(4).ok_or_else(incorrect_length)?;
    let mut file = File::open(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            super::ModelError::MissingOutput(format!("{}: {error}", path.display()))
        } else {
            io(error)
        }
    })?;
    let metadata = file.metadata().map_err(io)?;
    if !metadata.is_file() || metadata.len() != expected as u64 {
        return Err(incorrect_length());
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take((expected as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(io)?;
    if bytes.len() != expected {
        return Err(incorrect_length());
    }
    Ok(bytes)
}

fn read_stream(path: &Path, count: usize) -> Result<Vec<f64>, super::ModelError> {
    read_stream_bytes(path, count)?
        .as_chunks::<4>()
        .0
        .iter()
        .map(|word| {
            let value = f64::from(f32::from_le_bytes(*word));
            if value.is_finite() {
                Ok(value)
            } else {
                Err(super::ModelError::Decode(format!(
                    "{}: nonfinite engine value",
                    path.display()
                )))
            }
        })
        .collect()
}

fn centimetres(value: f64) -> Result<i32, String> {
    let rounded = (value * 100.0).round_ties_even();
    if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
        return Err("level centimetres outside i32 range".into());
    }
    Ok(rounded as i32)
}

fn validate(raw: &DecodedEngineResults) -> Result<(), String> {
    raw.forcing.validate().map_err(|error| error.to_string())?;
    let count = raw.forcing.values.len();
    if raw.flow.len() != count
        || raw.flow_valid.len() != count
        || raw.level_m.len() != count
        || raw.level_valid.len() != count
        || raw.time_segment.len() != raw.forcing.timeline.bounds.len()
        || raw
            .flow
            .iter()
            .zip(&raw.flow_valid)
            .any(|(v, valid)| *valid && !v.is_finite())
        || raw
            .level_m
            .iter()
            .zip(&raw.level_valid)
            .any(|(v, valid)| *valid && !v.is_finite())
    {
        return Err("invalid decoded result shape, mask or value".into());
    }
    Ok(())
}

fn quantize_levels_inner(raw: &DecodedEngineResults) -> Result<ResultSeries, String> {
    validate(raw)?;
    let raw_level_cm = raw
        .level_m
        .iter()
        .zip(&raw.level_valid)
        .map(|(v, valid)| if *valid { centimetres(*v) } else { Ok(0) })
        .collect::<Result<Vec<_>, _>>()?;
    let result = ResultSeries {
        timeline: raw.forcing.timeline.clone(),
        basins: raw.forcing.basins.clone(),
        time_segment: raw.time_segment.clone(),
        precipitation: raw.forcing.values.clone(),
        precipitation_valid: raw.forcing.valid.clone(),
        flow: raw.flow.clone(),
        flow_valid: raw.flow_valid.clone(),
        raw_level_cm,
        raw_level_valid: raw.level_valid.clone(),
        provenance: raw.forcing.provenance.clone(),
    };
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}

fn reference_levels_inner(
    raw: &DecodedEngineResults,
    station_ids: &[String],
    offsets: &[StationOffset],
) -> Result<StationResults, String> {
    validate(raw)?;
    let mut seen = HashSet::new();
    if station_ids
        .iter()
        .any(|id| id.is_empty() || !seen.insert(id))
    {
        return Err("invalid station IDs".into());
    }
    let mut seen_offsets = HashSet::new();
    for offset in offsets {
        if !seen_offsets.insert(&offset.station_id)
            || !offset.offset_m.is_finite()
            || offset.mini_id <= 0
            || !raw.forcing.basins.mini_ids.contains(&offset.mini_id)
        {
            return Err("invalid station offset".into());
        }
    }
    let selected: Vec<_> = station_ids
        .iter()
        .map(|id| offsets.iter().find(|offset| &offset.station_id == id))
        .collect();
    let mini_ids = selected
        .iter()
        .map(|offset| offset.map(|v| v.mini_id))
        .collect();
    let offsets_m = selected
        .iter()
        .map(|offset| offset.map(|v| v.offset_m))
        .collect();
    let mut level_cm = Vec::new();
    let mut level_valid = Vec::new();
    let basins = raw.forcing.basins.mini_ids.len();
    for time in 0..raw.forcing.timeline.bounds.len() {
        for offset in &selected {
            let index = offset.and_then(|v| {
                raw.forcing
                    .basins
                    .mini_ids
                    .iter()
                    .position(|id| *id == v.mini_id)
            });
            if let (Some(offset), Some(index)) = (offset, index) {
                let index = time * basins + index;
                if raw.level_valid[index] {
                    level_cm.push(centimetres(raw.level_m[index] - offset.offset_m)?);
                    level_valid.push(true);
                    continue;
                }
            }
            level_cm.push(0);
            level_valid.push(false);
        }
    }
    let mut provenance = raw.forcing.provenance.clone();
    if let Some(processing) = provenance.processing.as_object_mut() {
        processing.insert(
            "station_offsets".into(),
            serde_json::to_value(offsets).map_err(|error| error.to_string())?,
        );
    }
    let result = StationResults {
        timeline: raw.forcing.timeline.clone(),
        time_segment: raw.time_segment.clone(),
        station_ids: station_ids.to_vec(),
        mini_ids,
        offsets_m,
        level_cm,
        level_valid,
        provenance,
    };
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}

pub(crate) struct HistoryResults {
    pub flow: Vec<f64>,
    pub level_m: Vec<f64>,
}

fn read_history_inner(output: &Path, count: usize) -> Result<HistoryResults, super::ModelError> {
    Ok(HistoryResults {
        flow: read_stream(&output.join("QTUDO_Inercial_Atual.MGB"), count)?,
        level_m: read_stream(&output.join("YTUDO.MGB"), count)?,
    })
}

fn decode_outputs_inner(
    output: &Path,
    forcing: BasinForcing,
    historical_count: usize,
    forecast_count: usize,
    history: &HistoryResults,
) -> Result<DecodedEngineResults, super::ModelError> {
    if forcing.timeline.bounds.len()
        != historical_count
            .checked_add(forecast_count)
            .ok_or_else(|| super::ModelError::Decode("result time count overflow".into()))?
        || historical_count == 0
        || forecast_count == 0
    {
        return Err(super::ModelError::Decode("result timeline mismatch".into()));
    }
    let basins = forcing.basins.mini_ids.len();
    let historical = historical_count
        .checked_mul(basins)
        .ok_or_else(|| super::ModelError::Decode("history length overflow".into()))?;
    let forecast = forecast_count
        .checked_mul(basins)
        .ok_or_else(|| super::ModelError::Decode("forecast length overflow".into()))?;
    if history.flow.len() != historical || history.level_m.len() != historical {
        return Err(super::ModelError::Decode("history shape mismatch".into()));
    }
    let mut flow = read_stream(&output.join("QTUDO_Inercial_Atual.MGB"), historical)?;
    let forecast_flow = read_stream(&output.join("QTUDO_Inercial_Prev.MGB"), forecast)?;
    let historical_level = read_stream(&output.join("YTUDO.MGB"), historical)?;
    let forecast_level = read_stream(&output.join("YTUDO_prev.MGB"), forecast)?;
    if flow != history.flow {
        return Err(super::ModelError::Decode(
            "historical flow differs from shared history".into(),
        ));
    }
    let overwritten = forecast.min(historical);
    if historical_level[overwritten..] != history.level_m[overwritten..] {
        return Err(super::ModelError::Decode(
            "historical level suffix differs from shared history".into(),
        ));
    }
    flow.extend(forecast_flow);
    let mut level_m = history.level_m.clone();
    level_m.extend(forecast_level);
    let count = flow.len();
    let mut time_segment = vec![0; historical_count];
    time_segment.extend(vec![1; forecast_count]);
    let raw = DecodedEngineResults {
        forcing,
        time_segment,
        flow,
        flow_valid: vec![true; count],
        level_m,
        level_valid: vec![true; count],
    };
    validate(&raw).map_err(super::ModelError::Decode)?;
    Ok(raw)
}

pub(crate) fn read_history(
    output: &Path,
    count: usize,
) -> Result<HistoryResults, super::ModelError> {
    read_history_inner(output, count)
}
pub(crate) fn decode_outputs(
    output: &Path,
    forcing: BasinForcing,
    historical_count: usize,
    forecast_count: usize,
    history: &HistoryResults,
) -> Result<DecodedEngineResults, super::ModelError> {
    decode_outputs_inner(output, forcing, historical_count, forecast_count, history)
}
pub(crate) fn quantize_levels(
    raw: &DecodedEngineResults,
) -> Result<ResultSeries, super::ModelError> {
    quantize_levels_inner(raw).map_err(super::ModelError::Decode)
}
pub(crate) fn reference_levels(
    raw: &DecodedEngineResults,
    ids: &[String],
    offsets: &[StationOffset],
) -> Result<StationResults, super::ModelError> {
    reference_levels_inner(raw, ids, offsets).map_err(super::ModelError::Decode)
}
#[cfg(test)]
mod tests {
    use super::{
        DecodedEngineResults, centimetres, quantize_levels, read_stream, reference_levels,
    };
    use crate::store::{
        BasinForcing, BasinGeometry, IntervalAxis, Provenance, StationOffset, TimeInterval,
    };
    #[test]
    fn levels_round_once_with_checked_ties() {
        assert_eq!(centimetres(0.125).unwrap(), 12);
        assert_eq!(centimetres(0.375).unwrap(), 38);
        assert_eq!(centimetres(-0.125).unwrap(), -12);
        assert_eq!(centimetres(-0.375).unwrap(), -38);
        let sample = f32::from_bits(1.0041_f32.to_bits()) as f64;
        assert_eq!(centimetres(sample).unwrap(), 100);
        assert_eq!(centimetres(sample - 0.609).unwrap(), 40);
        assert_eq!(centimetres(0.125 - 1e-12).unwrap(), 12);
        assert_eq!(centimetres(0.125 + 1e-12).unwrap(), 13);
        assert_eq!(centimetres(-0.125 + 1e-12).unwrap(), -12);
        assert_eq!(centimetres(-0.125 - 1e-12).unwrap(), -13);
        assert_eq!(centimetres(f64::from(i32::MAX) / 100.0).unwrap(), i32::MAX);
        assert_eq!(centimetres(f64::from(i32::MIN) / 100.0).unwrap(), i32::MIN);
        assert!(centimetres((f64::from(i32::MAX) + 1.0) / 100.0).is_err());
        assert!(centimetres((f64::from(i32::MIN) - 1.0) / 100.0).is_err());
        assert!(centimetres(f64::INFINITY).is_err());
        assert!(centimetres(f64::from(i32::MAX)).is_err());
    }
    #[test]
    fn references_unrounded_levels_and_keeps_unknown_station() {
        let start = chrono::NaiveDate::from_ymd_opt(2026, 7, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let provenance = Provenance {
            source_ids: vec![],
            processing: serde_json::Value::Null,
        };
        let forcing = BasinForcing {
            timeline: IntervalAxis {
                time_zone: "UTC".into(),
                bounds: vec![TimeInterval {
                    start,
                    end: start + chrono::Duration::hours(1),
                }],
            },
            basins: BasinGeometry {
                mini_ids: vec![1],
                longitudes: vec![0.0],
                latitudes: vec![0.0],
                source: provenance.clone(),
            },
            values: vec![0.0],
            valid: vec![true],
            provenance,
        };
        let raw = DecodedEngineResults {
            forcing,
            time_segment: vec![0],
            flow: vec![1.0],
            flow_valid: vec![true],
            level_m: vec![1.0041_f32 as f64],
            level_valid: vec![true],
        };
        let offsets = vec![
            StationOffset {
                station_id: "a".into(),
                mini_id: 1,
                offset_m: 0.609,
            },
            StationOffset {
                station_id: "b".into(),
                mini_id: 1,
                offset_m: 0.0,
            },
        ];
        let output =
            reference_levels(&raw, &["a".into(), "missing".into(), "b".into()], &offsets).unwrap();
        assert_eq!(output.level_cm, [40, 0, 100]);
        assert_eq!(output.level_valid, [true, false, true]);
        assert_eq!(output.mini_ids, [Some(1), None, Some(1)]);
        assert_eq!(quantize_levels(&raw).unwrap().raw_level_cm, [100]);
    }
    #[test]
    fn stream_requires_exact_length_and_finite_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stream");
        std::fs::write(&path, 1.25_f32.to_le_bytes()).unwrap();
        assert_eq!(read_stream(&path, 1).unwrap(), [1.25]);
        let is_expected = matches!(
            read_stream(&path, 2),
            Err(crate::model::ModelError::MalformedOutput(_))
        );
        assert!(is_expected);
        std::fs::write(&path, f32::NAN.to_le_bytes()).unwrap();
        let is_expected = matches!(
            read_stream(&path, 1),
            Err(crate::model::ModelError::Decode(_))
        );
        assert!(is_expected);
        std::fs::remove_file(&path).unwrap();
        let is_expected = matches!(
            read_stream(&path, 1),
            Err(crate::model::ModelError::MissingOutput(_))
        );
        assert!(is_expected);
    }
}
