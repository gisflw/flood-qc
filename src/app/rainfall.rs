use super::operation::ExecutionRecipe;
use super::request::{ScenarioDocument, timestamp};
use super::{OperationControl, OperationError};
use crate::analysis::{
    HourlyWindow, RainSummary, accumulated_valid_basin_forcing, accumulated_valid_grid_rain,
    resolve_hourly_window, signed_rainfall_window,
};
use crate::store::{
    BasinForcing, DecodedRainfallFile, GridSeries, InputReference, MiniTopology, RainfallCacheInfo,
    RegularGridSpec, StagedRainfallCache, TimeInterval, copy_tree, read_basin_forcing,
    read_grid_series, read_mini_topology, read_rainfall_cache, verify_inputs,
};
use std::path::{Path, PathBuf};

const OBSERVED_GRID: &str = "checkpoints/observed-grid.nc";
const OBSERVED_BASINS: &str = "forcing/observed.nc";
const MINI: &str = "baseline/Input/MINI.gtp";

fn working_grid(scenario: &str) -> PathBuf {
    Path::new("checkpoints/forecast")
        .join(scenario)
        .join("working.nc")
}

fn basin_path(scenario: &str) -> PathBuf {
    Path::new("forcing").join(format!("{scenario}.nc"))
}

#[derive(Clone, Debug)]
pub enum RainfallPeriod {
    SignedHours(i32),
    Explicit {
        start: chrono::NaiveDateTime,
        end: chrono::NaiveDateTime,
    },
}

impl RainfallPeriod {
    pub fn resolve(&self, info: &RainfallCacheInfo) -> Result<TimeInterval, String> {
        match self {
            Self::SignedHours(hours) => {
                signed_rainfall_window(&info.time_zone, info.reference, *hours)
                    .map(|(_, interval)| interval)
            }
            Self::Explicit { start, end } => resolve_hourly_window(
                &info.time_zone,
                &HourlyWindow::Explicit {
                    start: *start,
                    end: *end,
                },
            ),
        }
    }
}

#[derive(Debug)]
pub struct CachedRainfall {
    pub info: RainfallCacheInfo,
    pub scenario: String,
    pub window: TimeInterval,
    pub grid: RegularGridSpec,
    pub values: Vec<Option<f64>>,
    pub summary: Option<RainSummary>,
    pub summary_finding: Option<String>,
}

pub fn rainfall_cache_info(workspace: &Path) -> Result<Option<RainfallCacheInfo>, String> {
    crate::store::rainfall_cache_info(workspace)
}

/// Resolve rainfall strictly from the active workspace cache. The map and basin
/// summary share its generation, selected scenario and period.
pub fn query_cached_rainfall(
    workspace: &Path,
    scenario: &str,
    period: &RainfallPeriod,
    outlet: Option<i64>,
    upstream: bool,
) -> Result<Option<CachedRainfall>, String> {
    RainfallCacheReader::default().query(workspace, scenario, period, outlet, upstream)
}

/// A read-only reader for repeated queries of the active rainfall generation.
/// Retains the observed grid, the most recently used forecast grid and basin
/// forcing, and the frozen topology. Each query checks the active cache again.
#[derive(Default)]
pub struct RainfallCacheReader {
    active: Option<(PathBuf, RainfallCacheInfo)>,
    observed: Option<DecodedRainfallFile<GridSeries>>,
    forecast: Option<DecodedRainfallFile<GridSeries>>,
    basins: Option<DecodedRainfallFile<BasinForcing>>,
    topology: Option<DecodedRainfallFile<Vec<MiniTopology>>>,
}

impl RainfallCacheReader {
    pub fn query(
        &mut self,
        workspace: &Path,
        scenario: &str,
        period: &RainfallPeriod,
        outlet: Option<i64>,
        upstream: bool,
    ) -> Result<Option<CachedRainfall>, String> {
        let result = read_rainfall_cache(workspace, |info, files| {
            if self
                .active
                .as_ref()
                .is_none_or(|(last_workspace, last_info)| {
                    last_workspace != workspace || last_info != info
                })
            {
                *self = Self {
                    active: Some((workspace.to_owned(), info.clone())),
                    ..Self::default()
                };
            }
            self.query_generation(info, files, scenario, period, outlet, upstream)
        });
        if !matches!(&result, Ok(Some(_))) {
            *self = Self::default();
        }
        result
    }

    fn query_generation(
        &mut self,
        info: &RainfallCacheInfo,
        files: &crate::store::RainfallCacheFiles<'_>,
        scenario: &str,
        period: &RainfallPeriod,
        outlet: Option<i64>,
        upstream: bool,
    ) -> Result<CachedRainfall, String> {
        if !info.scenarios.iter().any(|id| id == scenario) {
            return Err(format!("Unknown active rainfall scenario {scenario}"));
        }
        let window = period.resolve(info)?;
        let observed = files.decode(Path::new(OBSERVED_GRID), &mut self.observed, read_grid)?;
        let observed_end = observed.timeline.bounds.last().map(|bound| bound.end);
        if observed.timeline.time_zone != info.time_zone
            || observed_end != Some(info.reference)
            || observed.timeline.bounds.first().map(|bound| bound.start)
                != Some(info.coverage.start)
        {
            return Err("Observed rainfall does not match the cache reference".into());
        }
        let bounds = |start, end| HourlyWindow::Explicit { start, end };
        let history = (window.start < info.reference)
            .then(|| {
                accumulated_valid_grid_rain(
                    observed,
                    &bounds(window.start, window.end.min(info.reference)),
                )
            })
            .transpose()?;
        let needs_forecast = window.end > info.reference;
        let zero = info.zero_scenarios.iter().any(|id| id == scenario);
        let basins = if needs_forecast && zero || outlet.is_some() {
            let path = if needs_forecast {
                basin_path(scenario)
            } else {
                OBSERVED_BASINS.into()
            };
            let forcing = files.decode(&path, &mut self.basins, read_forcing)?;
            if forcing.timeline.time_zone != info.time_zone
                || !forcing
                    .timeline
                    .bounds
                    .iter()
                    .any(|bound| bound.end == info.reference)
            {
                return Err("Basin rainfall does not match the cache reference".into());
            }
            Some(forcing)
        } else {
            None
        };
        let mut forecast = None;
        if needs_forecast {
            let start = window.start.max(info.reference);
            if zero {
                let forcing = basins.as_ref().ok_or("Missing zero rainfall support")?;
                let covered_hours: i64 = forcing
                    .timeline
                    .bounds
                    .iter()
                    .filter(|bound| bound.start >= start && bound.end <= window.end)
                    .map(|bound| (bound.end - bound.start).num_hours())
                    .sum();
                let cells = observed.grid.latitudes.len() * observed.grid.longitudes.len();
                forecast = Some(vec![
                    (covered_hours == (window.end - start).num_hours())
                        .then_some(0.0);
                    cells
                ]);
            } else {
                let grid = files.decode(&working_grid(scenario), &mut self.forecast, read_grid)?;
                if observed.grid != grid.grid
                    || grid.timeline.time_zone != info.time_zone
                    || grid.timeline.bounds.first().map(|bound| bound.start) != Some(info.reference)
                    || grid.timeline.bounds.last().map(|bound| bound.end) != Some(info.coverage.end)
                {
                    return Err(
                        "Forecast rainfall does not match the cache grid or reference".into(),
                    );
                }
                forecast =
                    Some(accumulated_valid_grid_rain(grid, &bounds(start, window.end))?.values);
            }
        }
        let values = match (history, forecast) {
            (Some(history), Some(forecast)) => history
                .values
                .into_iter()
                .zip(forecast)
                .map(|(before, after)| before.zip(after).map(|(a, b)| a + b))
                .collect(),
            (Some(history), None) => history.values,
            (None, Some(forecast)) => forecast,
            (None, None) => return Err("Empty rainfall window".into()),
        };
        if values.iter().flatten().any(|value| !value.is_finite()) {
            return Err("Rainfall accumulation overflow".into());
        }
        let summary = outlet.zip(basins).map(|(outlet, forcing)| {
            let topology = files.decode(Path::new(MINI), &mut self.topology, |path| {
                read_mini_topology(path)
                    .map(|(_, topology)| topology)
                    .map_err(|error| {
                        format!("Rainfall summary needs frozen MINI topology: {error}")
                    })
            })?;
            accumulated_valid_basin_forcing(
                forcing,
                topology,
                outlet,
                upstream,
                &bounds(window.start, window.end),
            )
        });
        let (summary, summary_finding) = match summary {
            Some(Ok(summary)) => (Some(summary), None),
            Some(Err(error)) => (None, Some(error)),
            None => (None, None),
        };
        Ok(CachedRainfall {
            info: info.clone(),
            scenario: scenario.into(),
            window,
            grid: observed.grid.clone(),
            values,
            summary,
            summary_finding,
        })
    }
}

fn read_grid(path: &Path) -> Result<GridSeries, String> {
    read_grid_series(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn read_forcing(path: &Path) -> Result<BasinForcing, String> {
    read_basin_forcing(path).map_err(|error| format!("{}: {error}", path.display()))
}

pub(super) fn stage(
    workspace: &Path,
    prepared: &Path,
    recipe: &ExecutionRecipe,
    inputs: &[InputReference],
    control: &OperationControl,
) -> Result<StagedRainfallCache, OperationError> {
    stage_from_prepared(workspace, prepared, recipe, inputs, control).map_err(|message| {
        OperationError::Store {
            run_root: workspace.join("data/cache/rainfall"),
            message,
        }
    })
}

fn stage_from_prepared(
    workspace: &Path,
    prepared: &Path,
    recipe: &ExecutionRecipe,
    inputs: &[InputReference],
    control: &OperationControl,
) -> Result<StagedRainfallCache, String> {
    let field = |key: &str| {
        recipe.document[key]
            .as_str()
            .ok_or_else(|| format!("Missing rainfall recipe {key}"))
    };
    let reference =
        timestamp("reference_time", field("reference_time")?).map_err(|e| e.to_string())?;
    let time_zone = field("time_zone")?;
    let coverage = TimeInterval {
        start: timestamp("historical_first_end", field("historical_first_end")?)
            .map_err(|error| error.to_string())?
            .checked_sub_signed(chrono::Duration::hours(1))
            .ok_or("Rainfall coverage overflow")?,
        end: timestamp("forecast_last_end", field("forecast_last_end")?)
            .map_err(|error| error.to_string())?,
    };
    let scenarios: Vec<ScenarioDocument> =
        serde_json::from_value(recipe.document["scenarios"].clone()).map_err(|e| e.to_string())?;
    let ids = scenarios
        .iter()
        .map(|scenario| scenario.id.clone())
        .collect();
    let zero = scenarios
        .iter()
        .filter(|scenario| scenario.kind == "zero")
        .map(|scenario| scenario.id.clone())
        .collect();
    StagedRainfallCache::build(
        workspace,
        reference,
        time_zone.into(),
        coverage.clone(),
        ids,
        zero,
        |root| {
            let copy = |relative: &Path| -> Result<PathBuf, String> {
                control.check().map_err(|error| error.to_string())?;
                let entry = inputs
                    .iter()
                    .find(|entry| entry.path == relative)
                    .ok_or_else(|| {
                        format!("Missing prepared rainfall member {}", relative.display())
                    })?;
                let destination = root.join(relative);
                std::fs::create_dir_all(
                    destination
                        .parent()
                        .ok_or("Missing rainfall member directory")?,
                )
                .map_err(|error| error.to_string())?;
                copy_tree(&prepared.join(relative), &destination)?;
                verify_inputs(root, std::slice::from_ref(entry))?;
                Ok(destination)
            };
            let observed = read_grid(&copy(Path::new(OBSERVED_GRID))?)?;
            let historical = read_forcing(&copy(Path::new(OBSERVED_BASINS))?)?;
            copy(Path::new(MINI))?;
            if observed.timeline != historical.timeline
                || observed.timeline.time_zone != time_zone
                || observed.timeline.bounds.len() != recipe.historical_count
                || observed.timeline.bounds.last().map(|bound| bound.end) != Some(reference)
                || observed.timeline.bounds.first().map(|bound| bound.start) != Some(coverage.start)
            {
                return Err("Prepared observed rainfall does not match recipe".into());
            }
            for scenario in scenarios {
                let combined = read_forcing(&copy(&basin_path(&scenario.id))?)?;
                let forecast = combined
                    .timeline
                    .bounds
                    .get(recipe.historical_count..)
                    .ok_or("Missing prepared forecast bounds")?;
                if combined.basins != historical.basins
                    || combined.timeline.time_zone != time_zone
                    || !combined
                        .timeline
                        .bounds
                        .starts_with(&historical.timeline.bounds)
                    || forecast.len() != recipe.forecast_count
                    || forecast.first().map(|bound| bound.start) != Some(reference)
                    || forecast.last().map(|bound| bound.end) != Some(coverage.end)
                    || !combined.valid.starts_with(&historical.valid)
                    || !combined.values.starts_with(&historical.values)
                {
                    return Err("Prepared basin rainfall does not match recipe".into());
                }
                if scenario.kind == "zero" {
                    let offset = historical.values.len();
                    if combined.values[offset..].iter().any(|value| *value != 0.0)
                        || combined.valid[offset..].iter().any(|valid| !*valid)
                    {
                        return Err("Prepared zero scenario lacks valid zero rainfall".into());
                    }
                } else {
                    let grid = read_grid(&copy(&working_grid(&scenario.id))?)?;
                    if grid.grid != observed.grid
                        || grid.timeline.time_zone != time_zone
                        || grid.timeline.bounds != forecast
                    {
                        return Err("Prepared forecast rainfall does not match recipe".into());
                    }
                }
            }
            Ok(())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{IntervalAxis, Provenance, write_grid_series};

    #[test]
    fn crossing_accumulation_preserves_masks_and_whole_native_intervals() {
        std::fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        let start = timestamp("test", "2026-01-01T00:00:00").unwrap();
        let reference = start + chrono::Duration::hours(2);
        let end = reference + chrono::Duration::hours(3);
        let grid = RegularGridSpec::new([0., 0., 1., 1.], 1.).unwrap();
        let cells = grid.latitudes.len() * grid.longitudes.len();
        let scenario = "rain";
        let build = |forecast_amount: f64| {
            StagedRainfallCache::build(
                root.path(),
                reference,
                "UTC".into(),
                TimeInterval { start, end },
                vec![scenario.into()],
                vec![],
                |dir| {
                    for (path, interval, amount, masked) in [
                        (
                            PathBuf::from(OBSERVED_GRID),
                            TimeInterval {
                                start,
                                end: reference,
                            },
                            0.,
                            0,
                        ),
                        (
                            working_grid(scenario),
                            TimeInterval {
                                start: reference,
                                end,
                            },
                            forecast_amount,
                            1,
                        ),
                    ] {
                        let path = dir.join(path);
                        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                        let mut valid = vec![true; cells];
                        valid[masked] = false;
                        write_grid_series(
                            &path,
                            &GridSeries {
                                timeline: IntervalAxis {
                                    time_zone: "UTC".into(),
                                    bounds: vec![interval],
                                },
                                grid: grid.clone(),
                                values: vec![amount; cells],
                                valid,
                                provenance: Provenance {
                                    source_ids: vec![],
                                    processing: serde_json::Value::Null,
                                },
                            },
                        )
                        .unwrap();
                    }
                    Ok(())
                },
            )
            .unwrap()
        };
        let generation = build(4.).publish().unwrap();
        let query = |period| {
            query_cached_rainfall(root.path(), scenario, &period, None, false)
                .unwrap()
                .unwrap()
        };
        assert_eq!(
            &query(RainfallPeriod::SignedHours(-2)).values[..3],
            &[None, Some(0.), Some(0.)]
        );
        let crossing = query(RainfallPeriod::Explicit { start, end });
        assert_eq!(crossing.info, generation);
        assert_eq!(&crossing.values[..3], &[None, None, Some(4.)]);
        let missing = vec![None; cells];
        assert_eq!(query(RainfallPeriod::SignedHours(2)).values, missing);

        let mut reader = RainfallCacheReader::default();
        let period = RainfallPeriod::Explicit { start, end };
        for _ in 0..2 {
            let reused = reader
                .query(root.path(), scenario, &period, None, false)
                .unwrap()
                .unwrap();
            assert_eq!(reused.info, crossing.info);
            assert_eq!(reused.values, crossing.values);
        }
        let replacement = build(8.).publish().unwrap();
        let refreshed = reader
            .query(root.path(), scenario, &period, None, false)
            .unwrap()
            .unwrap();
        assert_eq!(refreshed.info, replacement);
        assert_eq!(&refreshed.values[..3], &[None, None, Some(8.)]);
    }
}
