from __future__ import annotations

from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.analysis.spatial import PrecipitationGrid, RegularGridSpec, resample_regular_grid
from mgb_ops.adapters import DEFAULT_FORECAST_ADAPTER, ForecastAdapter
from mgb_ops.common.time_utils import DashboardWindow, TIMEZONE
from mgb_ops.assets.forecast_grid import (
    ForecastPrecipitationGrid,
    read_forecast_precipitation_grid,
)
from mgb_ops.storage.forecast_assets import (
    find_forecast_asset_by_cycle,
    list_forecast_assets,
    resolve_forecast_asset,
)


class ForecastIntegrityError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def resolve_expected_forecast_asset(
    database_path: Path,
    *,
    workspace_path: Path,
    reference_time: datetime,
    adapter: ForecastAdapter = DEFAULT_FORECAST_ADAPTER,
) -> tuple[dict[str, object], Path]:
    expected_cycle = adapter.cycle_time(reference_time)
    match = find_forecast_asset_by_cycle(
        database_path,
        workspace_path=workspace_path,
        provider_code=adapter.provider_code,
        cycle_time=expected_cycle,
    )
    if match is None:
        raise ForecastIntegrityError(
            "unregistered_cycle",
            f"Forecast integrity error for {adapter.provider_code}: canonical NetCDF for expected cycle "
            f"{expected_cycle.isoformat(timespec='seconds')}Z is not registered in history.sqlite.",
        )
    row, path = match
    if not path.exists():
        raise ForecastIntegrityError(
            "missing_registered_file",
            f"Forecast integrity error for {adapter.provider_code}: expected cycle "
            f"{expected_cycle.isoformat(timespec='seconds')}Z is registered as "
            f"{row['relative_path']!r}, but that canonical NetCDF is missing.",
        )
    return row, path


def list_expected_forecast_assets(
    database_path: Path,
    *,
    workspace_path: Path,
    reference_time: datetime,
    adapter: ForecastAdapter = DEFAULT_FORECAST_ADAPTER,
) -> pd.DataFrame:
    row, _ = resolve_expected_forecast_asset(
        database_path,
        workspace_path=workspace_path,
        reference_time=reference_time,
        adapter=adapter,
    )
    return pd.DataFrame([row])


def _local_naive_to_utc(value: datetime) -> pd.Timestamp:
    return pd.Timestamp(value.replace(tzinfo=TIMEZONE)).tz_convert("UTC").tz_localize(None)


def list_dashboard_forecast_intervals(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path,
    window: DashboardWindow,
) -> pd.DataFrame:
    frame = list_forecast_intervals(
        asset_id,
        database_path=database_path,
        workspace_path=workspace_path,
    )
    if frame.empty:
        return frame
    cutoff_utc = _local_naive_to_utc(window.cutoff_time)
    end_utc = _local_naive_to_utc(window.forecast_end_exclusive)
    starts = pd.to_datetime(frame["start_time"]).dt.tz_localize(None)
    ends = pd.to_datetime(frame["end_time"]).dt.tz_localize(None)
    return frame[(starts >= cutoff_utc) & (ends <= end_utc)].reset_index(drop=True)


def forecast_interval_boundaries(
    source: Path | ForecastPrecipitationGrid,
) -> pd.DataFrame:
    grid = read_forecast_precipitation_grid(source) if isinstance(source, (str, Path)) else source
    rows = []
    cycle_start = grid.time_bounds_utc[0][0]
    for index, (start, end) in enumerate(grid.time_bounds_utc):
        rows.append({
            "index": index,
            "start_time": pd.Timestamp(start),
            "end_time": pd.Timestamp(end),
            "start_step_hours": int((start - cycle_start).total_seconds() // 3600),
            "end_step_hours": int((end - cycle_start).total_seconds() // 3600),
            "label": f"{pd.Timestamp(start):%d/%m %H:%M} – {pd.Timestamp(end):%d/%m %H:%M}",
        })
    return pd.DataFrame(rows)


def list_forecast_intervals(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path | None = None,
) -> pd.DataFrame:
    _, path = resolve_forecast_asset(asset_id, database_path=database_path, workspace_path=workspace_path)
    return forecast_interval_boundaries(path)


def accumulate_forecast_precipitation(
    source: Path | ForecastPrecipitationGrid,
    *,
    start_time: datetime | pd.Timestamp,
    end_time: datetime | pd.Timestamp,
) -> PrecipitationGrid:
    """Sum canonical timestep fields fully contained in [start_time, end_time)."""
    grid = read_forecast_precipitation_grid(source) if isinstance(source, (str, Path)) else source
    start = pd.Timestamp(start_time)
    end = pd.Timestamp(end_time)
    if end <= start:
        raise ValueError("end_time must be after start_time.")
    indices = [
        index for index, (left, right) in enumerate(grid.time_bounds_utc)
        if pd.Timestamp(left) >= start and pd.Timestamp(right) <= end
    ]
    if not indices:
        raise ValueError("Selected interval contains no complete forecast timesteps.")
    first_start = pd.Timestamp(grid.time_bounds_utc[indices[0]][0])
    last_end = pd.Timestamp(grid.time_bounds_utc[indices[-1]][1])
    if first_start != start or last_end != end:
        raise ValueError(
            "Selected accumulation boundaries must align with forecast interval boundaries."
        )
    values = np.nansum(grid.hourly_grids[indices, :, :], axis=0)
    return PrecipitationGrid(
        values=values,
        latitudes=grid.latitudes,
        longitudes=grid.longitudes,
        bounds=(
            float(np.min(grid.longitudes)), float(np.min(grid.latitudes)),
            float(np.max(grid.longitudes)), float(np.max(grid.latitudes)),
        ),
        start_time=start,
        end_time=end,
        units="mm",
        source=str(source) if isinstance(source, (str, Path)) else "forecast",
    )


def accumulate_forecast_steps(
    source: Path | ForecastPrecipitationGrid,
    *,
    t0_step: int,
    t1_step: int,
) -> PrecipitationGrid:
    grid = read_forecast_precipitation_grid(source) if isinstance(source, (str, Path)) else source
    if t0_step < 0 or t1_step <= t0_step:
        raise ValueError("Forecast steps must satisfy 0 <= t0_step < t1_step.")
    cycle_start = grid.time_bounds_utc[0][0]
    return accumulate_forecast_precipitation(
        grid,
        start_time=cycle_start + timedelta(hours=int(t0_step)),
        end_time=cycle_start + timedelta(hours=int(t1_step)),
    )


def resample_forecast_grid(
    precipitation: PrecipitationGrid,
    target: RegularGridSpec,
) -> PrecipitationGrid:
    values = resample_regular_grid(
        precipitation.values,
        precipitation.latitudes,
        precipitation.longitudes,
        target,
    )
    return PrecipitationGrid(
        values=values,
        latitudes=target.latitudes,
        longitudes=target.longitudes,
        bounds=target.bbox,
        start_time=precipitation.start_time,
        end_time=precipitation.end_time,
        units=precipitation.units,
        source=precipitation.source,
    )


def build_forecast_grid(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path | None = None,
    t0_step: int,
    t1_step: int,
    target_grid: RegularGridSpec | None = None,
) -> PrecipitationGrid:
    _, path = resolve_forecast_asset(
        asset_id, database_path=database_path, workspace_path=workspace_path
    )
    accumulated = accumulate_forecast_steps(path, t0_step=t0_step, t1_step=t1_step)
    return resample_forecast_grid(accumulated, target_grid) if target_grid else accumulated


list_registered_forecast_assets = list_forecast_assets
get_interval_boundaries = forecast_interval_boundaries
accumulate_precipitation = accumulate_forecast_precipitation
resample_to_analysis_grid = resample_forecast_grid
