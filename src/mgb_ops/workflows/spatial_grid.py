from __future__ import annotations

from datetime import datetime, timedelta, timezone
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.analysis.spatial import RegularGridSpec, interpolate_station_values
from mgb_ops.analysis.timeseries import select_preferred_series_rows
from mgb_ops.assets.history_queries import (
    open_history_read_only,
    read_observed_values,
    read_rain_series,
)
from mgb_ops.assets.spatial_grid import normalize_providers, write_spatial_grid
from mgb_ops.common.time_utils import TIMEZONE, validate_timestep_hours


OBSERVED_PRECIPITATION_CACHE_FILENAME = "precipitations_observed.nc"


def _require_utc(value: datetime, *, name: str) -> datetime:
    if value.tzinfo is None or value.utcoffset() != timedelta(0):
        raise ValueError(f"{name} must be a timezone-aware UTC datetime.")
    return value.astimezone(timezone.utc)


def build_observed_precipitation_cache(
    database_path: Path,
    cache_dir: Path,
    *,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    start_time_utc: datetime,
    end_time_utc: datetime,
    timestep_hours: int,
    providers: tuple[str, ...] | list[str],
    nearest_stations: int = 5,
    power: float = 2.0,
) -> Path:
    """Interpolate observed precipitation timesteps and atomically replace the dashboard cache."""
    start_utc = _require_utc(start_time_utc, name="start_time_utc")
    end_utc = _require_utc(end_time_utc, name="end_time_utc")
    timestep_hours = validate_timestep_hours(timestep_hours)
    if end_utc <= start_utc:
        raise ValueError("end_time_utc must be after start_time_utc.")
    step = timedelta(hours=timestep_hours)
    if (end_utc - start_utc) % step:
        raise ValueError("The UTC cache window must contain an exact number of timesteps.")
    if nearest_stations < 1:
        raise ValueError("nearest_stations must be >= 1.")
    if not np.isfinite(power) or power <= 0:
        raise ValueError("power must be > 0.")

    provider_codes = normalize_providers(providers)
    grid = RegularGridSpec(bbox=bbox, resolution_degrees=resolution_degrees)
    local_start = start_utc.astimezone(TIMEZONE).replace(tzinfo=None)
    local_end = end_utc.astimezone(TIMEZONE).replace(tzinfo=None)
    with open_history_read_only(Path(database_path)) as connection:
        series = read_rain_series(connection)
        series = series[series["provider_code"].astype(str).str.lower().isin(provider_codes)]
        preferred = select_preferred_series_rows(series)
        values = read_observed_values(
            connection,
            preferred["series_id"].astype(str).tolist(),
            start_time=local_start,
            end_time=local_end,
            end_inclusive=True,
        )

    ends_utc = pd.date_range(
        pd.Timestamp(start_utc) + pd.Timedelta(step),
        pd.Timestamp(end_utc),
        freq=pd.Timedelta(step),
    )
    local_labels = ends_utc.tz_convert(TIMEZONE).tz_localize(None)
    if values.empty:
        fields = np.full((len(ends_utc), *grid.shape), np.nan, dtype=float)
    else:
        values = values.copy()
        values["observed_at"] = pd.to_datetime(values["observed_at"], errors="coerce")
        values["value"] = pd.to_numeric(values["value"], errors="coerce")
        station_lookup = preferred.set_index("series_id")[["lat", "lon"]]
        fields_list: list[np.ndarray] = []
        for local_label in local_labels:
            timestep = values[values["observed_at"] == local_label].merge(
                station_lookup, left_on="series_id", right_index=True, how="inner"
            )
            timestep = timestep.dropna(subset=["value", "lat", "lon"])
            if timestep.empty:
                fields_list.append(np.full(grid.shape, np.nan, dtype=float))
            else:
                fields_list.append(
                    interpolate_station_values(
                        timestep,
                        grid,
                        nearest_stations=nearest_stations,
                        power=power,
                    )
                )
        fields = np.stack(fields_list)

    target = Path(cache_dir) / OBSERVED_PRECIPITATION_CACHE_FILENAME
    return write_spatial_grid(
        target,
        variable="precipitation",
        grid_type="observed",
        source="interpolated_from_stations",
        providers=provider_codes,
        units="mm",
        bbox=grid.bbox,
        resolution_degrees=grid.resolution,
        times_utc=[timestamp.to_pydatetime() for timestamp in ends_utc],
        latitudes=grid.latitudes,
        longitudes=grid.longitudes,
        values=fields,
        timestep_hours=timestep_hours,
        title="Observed precipitation interpolated from stations",
        processing_metadata={
            "interpolation_method": "inverse_distance_weighting",
            "nearest_stations": nearest_stations,
            "power": power,
        },
    )
