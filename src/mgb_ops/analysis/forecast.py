from __future__ import annotations

import json
import sqlite3
from datetime import datetime, timedelta
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.analysis.spatial import PrecipitationGrid, RegularGridSpec, resample_regular_grid
from mgb_ops.adapters.forecast_ecmwf import build_ecmwf_cycle
from mgb_ops.common.time_utils import DashboardWindow, TIMEZONE
from mgb_ops.model.forecast_grid import (
    FORECAST_PRECIPITATION_GRID_ASSET_KIND,
    ForecastPrecipitationGrid,
    read_forecast_precipitation_grid,
)


class ForecastIntegrityError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


def _metadata_cycle(metadata_json: object) -> datetime | None:
    try:
        metadata = json.loads(str(metadata_json)) if metadata_json else {}
        raw = metadata.get("cycle_time") or metadata.get("source_cycle_time")
        if not raw:
            return None
        timestamp = pd.Timestamp(raw)
        if timestamp.tzinfo is None:
            return timestamp.to_pydatetime()
        return timestamp.tz_convert("UTC").tz_localize(None).to_pydatetime()
    except (TypeError, ValueError, json.JSONDecodeError):
        return None


def resolve_expected_ecmwf_asset(
    database_path: Path,
    *,
    workspace_path: Path,
    reference_time: datetime,
) -> tuple[dict[str, object], Path]:
    expected_cycle = build_ecmwf_cycle(reference_time)
    assets = list_forecast_assets(database_path, workspace_path=workspace_path)
    if not assets.empty:
        assets = assets[assets["provider_code"] == "ecmwf"]
    matching = (
        assets[assets["metadata_json"].map(_metadata_cycle).map(lambda value: value == expected_cycle)]
        if not assets.empty
        else assets
    )
    if matching.empty:
        raise ForecastIntegrityError(
            "unregistered_cycle",
            "ECMWF integrity error: canonical NetCDF for expected cycle "
            f"{expected_cycle.isoformat(timespec='seconds')}Z is not registered in history.sqlite.",
        )
    row = matching.iloc[0].to_dict()
    path = Path(row["asset_path"])
    if not path.exists():
        raise ForecastIntegrityError(
            "missing_registered_file",
            "ECMWF integrity error: expected cycle "
            f"{expected_cycle.isoformat(timespec='seconds')}Z is registered as "
            f"{row['relative_path']!r}, but that canonical NetCDF is missing.",
        )
    return row, path


def list_expected_ecmwf_assets(
    database_path: Path,
    *,
    workspace_path: Path,
    reference_time: datetime,
) -> pd.DataFrame:
    row, _ = resolve_expected_ecmwf_asset(
        database_path,
        workspace_path=workspace_path,
        reference_time=reference_time,
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


def list_forecast_assets(database_path: Path, *, workspace_path: Path | None = None) -> pd.DataFrame:
    """List registered canonical forecast NetCDF assets.

    Paths are resolved only from the explicit database/workspace arguments.
    """
    database = Path(database_path)
    if not database.exists():
        raise FileNotFoundError(f"History database not found: {database}")
    with sqlite3.connect(f"{database.resolve().as_uri()}?mode=ro", uri=True) as connection:
        frame = pd.read_sql_query(
            """SELECT asset_id, asset_kind, format, relative_path, provider_code,
                      checksum, valid_from, valid_to, metadata_json, created_at
               FROM asset WHERE asset_kind = ?
               ORDER BY COALESCE(valid_from, created_at) DESC, created_at DESC""",
            connection,
            params=(FORECAST_PRECIPITATION_GRID_ASSET_KIND,),
        )
    if frame.empty:
        return frame.assign(asset_path=pd.Series(dtype=object), display_label=pd.Series(dtype=str))
    root = Path(workspace_path).resolve() if workspace_path is not None else database.parent.parent.resolve()
    frame["asset_path"] = frame["relative_path"].map(
        lambda value: Path(value) if Path(value).is_absolute() else root / Path(value)
    )
    frame["metadata"] = frame["metadata_json"].map(
        lambda value: json.loads(value) if value else {}
    )
    frame["cycle_time"] = frame["metadata"].map(
        lambda value: value.get("cycle_time") if isinstance(value, dict) else None
    )
    frame["display_label"] = frame.apply(
        lambda row: f"{row['asset_id']} | cycle {row['cycle_time'] or row['valid_from'] or 'unknown'}",
        axis=1,
    )
    return frame.drop(columns="metadata")


def resolve_forecast_asset(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path | None = None,
) -> tuple[dict[str, object], Path]:
    assets = list_forecast_assets(database_path, workspace_path=workspace_path)
    selected = assets[assets["asset_id"] == asset_id]
    if selected.empty:
        raise ValueError(f"Canonical forecast asset {asset_id!r} was not found.")
    row = selected.iloc[0].to_dict()
    path = Path(row["asset_path"])
    if not path.exists():
        raise FileNotFoundError(f"Forecast NetCDF registered for {asset_id!r} was not found: {path}")
    return row, path


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
