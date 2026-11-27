use super::{GridSeries, InputReference, digest_file, read_draft, read_grid_series, write_draft};
use chrono::{Duration, Timelike};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ImergObject {
    id: String,
    utc_day: chrono::NaiveDate,
    product: String,
    grid: InputReference,
}
fn valid(id: &str) -> bool {
    id.starts_with("imerg-")
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}
fn validate_day(series: &GridSeries) -> Result<chrono::NaiveDate, String> {
    series.validate().map_err(|e| e.to_string())?;
    let zone: chrono_tz::Tz = series
        .timeline
        .time_zone
        .parse()
        .map_err(|e: chrono_tz::ParseError| e.to_string())?;
    if zone != chrono_tz::UTC {
        return Err("IMERG registrations require UTC interval labels".into());
    }
    if series.timeline.bounds.len() != 24 {
        return Err("register a complete 24-hour regional UTC day".into());
    }
    let start = series.timeline.bounds[0].start;
    if start.time().num_seconds_from_midnight() != 0 {
        return Err("IMERG day must begin at UTC midnight".into());
    }
    for (index, bound) in series.timeline.bounds.iter().enumerate() {
        if bound.start != start + Duration::hours(index as i64)
            || bound.end != start + Duration::hours(index as i64 + 1)
        {
            return Err("IMERG day has missing or irregular hourly intervals".into());
        }
    }
    if series
        .provenance
        .processing
        .get("product")
        .and_then(|v| v.as_str())
        .is_none_or(|v| !v.starts_with("GPM_3IMERG"))
    {
        return Err("missing IMERG product provenance".into());
    }
    if series
        .values
        .iter()
        .zip(&series.valid)
        .any(|(v, ok)| *ok && *v < 0.)
    {
        return Err("negative IMERG precipitation".into());
    }
    Ok(start.date())
}
pub fn register_imerg_day(workspace: &Path, source: &Path) -> Result<String, String> {
    let series = read_grid_series(source).map_err(|e| e.to_string())?;
    let utc_day = validate_day(&series)?;
    let (sha256, size) = digest_file(source)?;
    let product = series.provenance.processing["product"]
        .as_str()
        .ok_or("missing product")?
        .to_owned();
    let label: String = product
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let id = format!(
        "imerg-{label}-{}-{}",
        utc_day.format("%Y%m%d"),
        &sha256[..24]
    );
    let root = workspace.join("assets/objects");
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let destination = root.join(&id);
    if !destination.exists() {
        super::publish_directory(&destination, |stage| {
            super::copy_staged_tree(source, &stage.join("grid.nc"))?;
            if digest_file(&stage.join("grid.nc"))? != (sha256.clone(), size) {
                return Err("IMERG source changed during registration".into());
            }
            write_draft(
                &stage.join("object.toml"),
                &ImergObject {
                    id: id.clone(),
                    utc_day,
                    product,
                    grid: InputReference {
                        path: "grid.nc".into(),
                        kind: "imerg".into(),
                        size,
                        sha256,
                    },
                },
            )
        })?;
    }
    registered_imerg(workspace, &id)?;
    Ok(id)
}
pub fn registered_imerg_path(workspace: &Path, id: &str) -> Result<PathBuf, String> {
    if !valid(id) {
        return Err("invalid IMERG object ID".into());
    }
    let root = workspace.join("assets/objects").join(id);
    let object: ImergObject = read_draft(&root.join("object.toml"))?;
    if object.id != id
        || object.grid.path != Path::new("grid.nc")
        || digest_file(&root.join("grid.nc"))? != (object.grid.sha256, object.grid.size)
    {
        return Err("IMERG registration failed integrity verification".into());
    }
    Ok(root.join("grid.nc"))
}
pub fn registered_imerg(workspace: &Path, id: &str) -> Result<GridSeries, String> {
    let grid =
        read_grid_series(&registered_imerg_path(workspace, id)?).map_err(|e| e.to_string())?;
    validate_day(&grid)?;
    Ok(grid)
}
pub fn registered_imerg_choices(workspace: &Path) -> Result<Vec<String>, String> {
    let root = workspace.join("assets/objects");
    if !root.exists() {
        return Ok(Vec::new());
    }
    let mut choices = Vec::new();
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let id = entry.file_name().to_string_lossy().to_string();
        if valid(&id) && entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            registered_imerg_path(workspace, &id)?;
            choices.push(id);
        }
    }
    choices.sort();
    Ok(choices)
}
