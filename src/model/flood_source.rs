use super::input::metadata_fields;
use super::output::read_stream_bytes;
use super::{EnginePlan, GeometryFile};
use crate::store::{BasinForcing, FloodSource, TimeInterval, digest_file};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

const HAND_STREAM: &str = "HANDTUDO.MGB";

pub(crate) struct FloodHistory {
    timeline: Vec<TimeInterval>,
    mini_ids: Vec<i64>,
    depth: Vec<f32>,
    channel_depth: Vec<f32>,
    bed: Vec<f32>,
    hand_sha256: String,
    cell_sha256: String,
    cota_sha256: String,
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn start_time(year: &str, month: &str, day: &str, hour: &str) -> Option<NaiveDateTime> {
    NaiveDate::from_ymd_opt(year.parse().ok()?, month.parse().ok()?, day.parse().ok()?)?
        .and_hms_opt(hour.parse().ok()?, 0, 0)
}

fn match_hours(mut end: NaiveDateTime, bounds: &[TimeInterval]) -> Result<(), String> {
    for interval in bounds {
        let start = end
            .checked_sub_signed(Duration::hours(1))
            .ok_or("engine time overflow")?;
        if interval.start != start || interval.end != end {
            return Err(format!(
                "engine interval ending {end} differs from flood source timeline"
            ));
        }
        end = end
            .checked_add_signed(Duration::hours(1))
            .ok_or("engine time overflow")?;
    }
    Ok(())
}

fn check_timing(
    plan: &EnginePlan,
    forcing: &BasinForcing,
    historical_count: usize,
    history_only: bool,
) -> Result<(), String> {
    let bounds = &forcing.timeline.bounds;
    let (history, forecast) = bounds
        .split_at_checked(historical_count)
        .filter(|(h, f)| !h.is_empty() && !f.is_empty())
        .ok_or("invalid flood source timeline counts")?;
    if forcing.timeline.time_zone != plan.time_zone {
        return Err("engine timezone differs from flood source timeline".into());
    }
    let par = read(&plan.input.join("PARHIG.hig"))?;
    let fields = metadata_fields(&par, &[b"DIA", b"MES", b"ANO", b"HORA"])?;
    let [day, month, year, hour] = fields[..] else {
        return Err("invalid PARHIG.hig start".into());
    };
    let history_start = start_time(year, month, day, hour).ok_or("invalid PARHIG.hig start")?;
    let fields = metadata_fields(&par, &[b"NT", b"DT"])?;
    let [count, step] = fields[..] else {
        return Err("invalid PARHIG.hig count or step".into());
    };
    let expected = if history_only {
        history.len()
    } else {
        bounds.len()
    };
    if count.parse::<usize>().ok() != Some(expected) || step.parse::<f32>().ok() != Some(3600.0) {
        return Err("PARHIG.hig count or step differs from flood source timeline".into());
    }
    let meta = read(&plan.input.join("Previsao.meta"))?;
    let fields = metadata_fields(&meta, &[b"HORIZ(NT)"])?;
    let [year, month, day, hour, count] = fields[..] else {
        return Err("invalid Previsao.meta start or count".into());
    };
    let forecast_start = start_time(year, month, day, hour).ok_or("invalid Previsao.meta start")?;
    let expected = if history_only { 0 } else { forecast.len() };
    if count.parse::<usize>().ok() != Some(expected) {
        return Err("Previsao.meta horizon differs from flood source timeline".into());
    }
    match_hours(history_start, history)?;
    match_hours(forecast_start, forecast)?;
    if history.last().map(|interval| interval.end)
        != forecast.first().map(|interval| interval.start)
    {
        return Err("engine history and forecast intervals are not contiguous".into());
    }
    Ok(())
}

fn read_z0(bytes: &[u8], mini_ids: &[i64]) -> Result<Vec<f64>, String> {
    let mut lines = bytes.split(|byte| *byte == b'\n');
    let header = lines.next().unwrap_or_default();
    let header = std::str::from_utf8(header).map_err(|error| error.to_string())?;
    if !header
        .split_ascii_whitespace()
        .eq(["Minibacia", "Z0(m)", "Zfp(m)", "Areafp(km2)"])
    {
        return Err("invalid COTA_AREA.FLP header".into());
    }
    let mut current = None;
    let mut z0s = Vec::with_capacity(mini_ids.len());
    for line in lines {
        let line = std::str::from_utf8(line).map_err(|error| error.to_string())?;
        let mut fields = line.split_ascii_whitespace();
        let Some(mini) = fields.next() else { continue };
        let mini = mini
            .parse::<i64>()
            .map_err(|_| "invalid COTA_AREA.FLP MINI")?;
        let z0 = fields
            .next()
            .and_then(|token| token.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .ok_or("invalid COTA_AREA.FLP Z0")?;
        if fields.count() != 2 {
            return Err("invalid COTA_AREA.FLP row".into());
        }
        if current == Some(mini) {
            if z0s.last() != Some(&z0) {
                return Err(format!("COTA_AREA.FLP Z0 changes for MINI {mini}"));
            }
        } else {
            if mini_ids.get(z0s.len()) != Some(&mini) {
                return Err(format!("COTA_AREA.FLP order disagrees at MINI {mini}"));
            }
            z0s.push(z0);
            current = Some(mini);
        }
    }
    if z0s.len() != mini_ids.len() {
        return Err("COTA_AREA.FLP MINI count mismatch".into());
    }
    Ok(z0s)
}

fn read_hand(plan: &EnginePlan, rows: usize, minis: usize) -> Result<(Vec<f32>, String), String> {
    let samples = rows
        .checked_mul(minis)
        .ok_or("flood stream length overflow")?;
    let path = plan.output.join(HAND_STREAM);
    let bytes = read_stream_bytes(&path, samples)?;
    let depth: Vec<_> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect();
    if let Some(index) = depth
        .iter()
        .position(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(format!(
            "{}: invalid depth at row {}, MINI column {}",
            path.display(),
            index / minis + 1,
            index % minis + 1
        ));
    }
    Ok((depth, sha256(&bytes)))
}

fn check_levels(
    depth: &[f32],
    levels: &[f64],
    bed: &[f32],
    timeline: &[TimeInterval],
    mini_ids: &[i64],
) -> Result<(), String> {
    if bed.is_empty()
        || bed.len() != mini_ids.len()
        || timeline.len().checked_mul(bed.len()) != Some(depth.len())
        || levels.len() != depth.len()
    {
        return Err("HANDTUDO depth and raw YTUDO have different shapes".into());
    }
    for (time, (depth_row, level_row)) in depth
        .chunks_exact(bed.len())
        .zip(levels.chunks_exact(bed.len()))
        .enumerate()
    {
        for (mini, (&depth, (&level, &bed))) in
            depth_row.iter().zip(level_row.iter().zip(bed)).enumerate()
        {
            let expected = (level as f32 - bed).max(0.0);
            if !bed.is_finite() || !level.is_finite() || depth != expected {
                return Err(format!(
                    "HANDTUDO disagrees with raw YTUDO at {} for MINI {}",
                    timeline[time].end, mini_ids[mini]
                ));
            }
        }
    }
    Ok(())
}

pub(crate) fn capture_flood_history(
    plan: &EnginePlan,
    forcing: &BasinForcing,
    historical_count: usize,
    levels: &[f64],
) -> Result<FloodHistory, String> {
    let mini_ids = &forcing.basins.mini_ids;
    let (depth, hand_sha256) = read_hand(plan, historical_count, mini_ids.len())?;
    check_timing(plan, forcing, historical_count, true)?;
    let cell_path = plan.input.join("Cell.hig");
    let cell = read(&cell_path)?;
    let cell_sha256 = sha256(&cell);
    let cells =
        GeometryFile::parse(cell).map_err(|error| format!("{}: {error}", cell_path.display()))?;
    if !cells
        .rows()
        .iter()
        .map(|row| row.mini as i64)
        .eq(mini_ids.iter().copied())
    {
        return Err("Cell.hig MINI file order disagrees with canonical MINI order".into());
    }
    let channel_depth: Vec<_> = cells.rows().iter().map(|row| row.depth as f32).collect();
    let cota = read(&plan.input.join("COTA_AREA.FLP"))?;
    let cota_sha256 = sha256(&cota);
    let z0 = read_z0(&cota, mini_ids)?;
    let bed: Vec<_> = z0
        .iter()
        .zip(cells.rows())
        .map(|(z0, row)| (z0 - row.depth) as f32)
        .collect();
    let timeline = forcing.timeline.bounds[..historical_count].to_vec();
    check_levels(&depth, levels, &bed, &timeline, mini_ids)?;
    Ok(FloodHistory {
        timeline,
        mini_ids: mini_ids.clone(),
        depth,
        channel_depth,
        bed,
        hand_sha256,
        cell_sha256,
        cota_sha256,
    })
}

pub(crate) fn capture_flood_source(
    plan: &EnginePlan,
    scenario: &str,
    history: &FloodHistory,
    forcing: &BasinForcing,
    levels: &[f64],
) -> Result<FloodSource, String> {
    let historical_count = history.timeline.len();
    let bounds = &forcing.timeline.bounds;
    check_timing(plan, forcing, historical_count, false)?;
    let forecast = &bounds[historical_count..];
    if bounds[..historical_count] != history.timeline || forcing.basins.mini_ids != history.mini_ids
    {
        return Err("scenario timeline or MINI order disagrees with flood history".into());
    }
    let minis = history.mini_ids.len();
    let (scenario_depth, scenario_sha256) =
        read_hand(plan, historical_count.max(forecast.len()), minis)?;
    for (name, expected) in [
        ("Cell.hig", &history.cell_sha256),
        ("COTA_AREA.FLP", &history.cota_sha256),
    ] {
        if digest_file(&plan.input.join(name))?.0 != *expected {
            return Err(format!("scenario {name} differs from flood history"));
        }
    }
    // The forecast reuses the start of the direct-access stream. The remaining
    // historical rows must match the independent history-only run.
    let forecast_size = forecast.len() * minis;
    if !scenario_depth[forecast_size..]
        .iter()
        .map(|value| value.to_bits())
        .eq(history
            .depth
            .iter()
            .skip(forecast_size)
            .map(|value| value.to_bits()))
    {
        return Err("scenario HANDTUDO historical suffix differs from shared history".into());
    }
    let forecast_depth = &scenario_depth[..forecast_size];
    let forecast_levels = levels
        .get(history.depth.len()..)
        .ok_or("missing forecast raw levels")?;
    check_levels(
        forecast_depth,
        forecast_levels,
        &history.bed,
        forecast,
        &history.mini_ids,
    )?;

    let mut depth_m = history.depth.clone();
    depth_m.extend_from_slice(forecast_depth);
    let mut provenance = forcing.provenance.clone();
    let processing = provenance
        .processing
        .as_object_mut()
        .ok_or("invalid flood source provenance")?;
    processing.insert(
        "flood_source".into(),
        serde_json::json!({
            "scenario": scenario,
            "depth_datum": "model_channel_bed",
            "bed_elevation": "COTA_AREA.FLP Z0 minus Cell.hig Depth (float32)",
            "history_handtudo_sha256": history.hand_sha256,
            "scenario_handtudo_sha256": scenario_sha256,
            "cell_hig_sha256": history.cell_sha256,
            "cota_area_flp_sha256": history.cota_sha256,
        }),
    );
    Ok(FloodSource {
        timeline: forcing.timeline.clone(),
        mini_ids: history.mini_ids.clone(),
        depth_m,
        channel_depth_m: history.channel_depth.clone(),
        historical_count,
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::{check_levels, match_hours, read_z0, start_time};
    use crate::store::TimeInterval;
    use chrono::Duration;

    #[test]
    fn distinguishes_mini_order_and_conflicting_z0() {
        let cota = b" Minibacia Z0(m) Zfp(m) Areafp(km2)\r\n1 1122 1123 .91\r\n1 1122 1124 1.02\r\n2 10 11 1.00\r\n";
        assert_eq!(read_z0(cota, &[1, 2]).unwrap(), [1122.0, 10.0]);
        assert!(read_z0(cota, &[2, 1]).unwrap_err().contains("order"));
        assert!(read_z0(cota, &[1, 2, 3]).unwrap_err().contains("count"));
        let conflicting = b"Minibacia Z0(m) Zfp(m) Areafp(km2)\n1 10 11 1\n1 11 12 2\n";
        assert!(read_z0(conflicting, &[1]).unwrap_err().contains("changes"));
    }

    #[test]
    fn validates_canonical_hour_positions_and_signed_level_difference() {
        let hour = Duration::hours(1);
        let start = start_time("2026", "7", "20", "0").unwrap();
        let mut bounds = [
            TimeInterval {
                start,
                end: start + hour,
            },
            TimeInterval {
                start: start + hour,
                end: start + hour * 2,
            },
        ];
        match_hours(start + hour, &bounds).unwrap();
        check_levels(&[0.0, 2.5], &[1.0, 4.5], &[2.0], &bounds, &[4]).unwrap();
        assert!(
            check_levels(&[0.0, 2.49], &[1.0, 4.5], &[2.0], &bounds, &[4])
                .unwrap_err()
                .contains("2026-07-20 02:00:00 for MINI 4")
        );
        bounds[1].start += hour;
        bounds[1].end += hour;
        assert!(match_hours(start + hour, &bounds).is_err());
    }
}
