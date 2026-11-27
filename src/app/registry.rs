use crate::store::ObservationStore;
use std::{fs, path::Path};

pub fn register_workspace_imerg(workspace: &Path, source: &Path) -> Result<String, String> {
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    crate::store::register_imerg_day(workspace, source)
}
pub fn imerg_choices(workspace: &Path) -> Result<Vec<String>, String> {
    crate::store::registered_imerg_choices(workspace)
}
pub fn import_registered_observations(workspace: &Path, interchange: &Path) -> Result<i64, String> {
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    let target = super::observation_assets(workspace);
    ObservationStore::import_interchange(&target, interchange)
        .and_then(|store| store.revision())
        .map_err(|e| e.to_string())
}
pub fn keep_workspace_results(workspace: &Path, id: &str, keep: bool) -> Result<(), String> {
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    crate::store::keep_run_results(workspace, id, keep)
}
pub fn retained_workspace_results(workspace: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    crate::store::retained_run_results(workspace)
}

/// Register complete UTC days from canonical acquisition outputs, preserving originals.
pub fn register_imerg_acquisition(
    workspace: &Path,
    paths: &[std::path::PathBuf],
) -> Result<Vec<String>, String> {
    use chrono::TimeZone;
    let _lock = super::operation::lock_workspace_job(workspace).map_err(|e| e.to_string())?;
    let work = workspace.join("work");
    fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let temporary = tempfile::Builder::new()
        .prefix(".imerg-registration-")
        .tempdir_in(&work)
        .map_err(|e| e.to_string())?;
    let mut days = Vec::new();
    for (file_index, path) in paths.iter().enumerate() {
        let native = crate::store::read_grid_series(path).map_err(|e| e.to_string())?;
        let zone: chrono_tz::Tz = native
            .timeline
            .time_zone
            .parse()
            .map_err(|e: chrono_tz::ParseError| e.to_string())?;
        let count = native.grid.latitudes.len() * native.grid.longitudes.len();
        let mut grouped =
            std::collections::BTreeMap::<chrono::NaiveDate, crate::store::GridSeries>::new();
        for (index, bound) in native.timeline.bounds.iter().enumerate() {
            let start = zone
                .from_local_datetime(&bound.start)
                .single()
                .ok_or("ambiguous IMERG start")?
                .naive_utc();
            let end = zone
                .from_local_datetime(&bound.end)
                .single()
                .ok_or("ambiguous IMERG end")?
                .naive_utc();
            let day = grouped
                .entry(start.date())
                .or_insert_with(|| crate::store::GridSeries {
                    timeline: crate::store::IntervalAxis {
                        time_zone: "UTC".into(),
                        bounds: Vec::new(),
                    },
                    grid: native.grid.clone(),
                    values: Vec::new(),
                    valid: Vec::new(),
                    provenance: native.provenance.clone(),
                });
            day.timeline
                .bounds
                .push(crate::store::TimeInterval { start, end });
            day.values
                .extend_from_slice(&native.values[index * count..(index + 1) * count]);
            day.valid
                .extend_from_slice(&native.valid[index * count..(index + 1) * count]);
        }
        for (date, day) in grouped {
            if day.timeline.bounds.len() != 24
                || day.timeline.bounds[0].start
                    != date.and_hms_opt(0, 0, 0).ok_or("invalid UTC day")?
                || day.timeline.bounds.last().ok_or("empty day")?.end
                    != date
                        .succ_opt()
                        .ok_or("UTC date overflow")?
                        .and_hms_opt(0, 0, 0)
                        .ok_or("invalid UTC end")?
            {
                return Err(
                    "IMERG acquisition must cover complete UTC days before registration".into(),
                );
            }
            let target = temporary.path().join(format!("{file_index}-{date}.nc"));
            crate::store::write_grid_series(&target, &day).map_err(|e| e.to_string())?;
            days.push(target);
        }
    }
    if days.is_empty() {
        return Err("no canonical IMERG days to register".into());
    }
    days.iter()
        .map(|path| crate::store::register_imerg_day(workspace, path))
        .collect()
}
