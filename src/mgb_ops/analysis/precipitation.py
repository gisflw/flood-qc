from __future__ import annotations

from datetime import datetime
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.assets.grid_transforms import interpolate_station_values
from mgb_ops.assets.history_queries import (
    open_history_read_only,
    read_observed_values,
    read_rain_series,
    select_preferred_series_rows,
)
from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec


def accumulate_observed_rainfall(
    database_path: Path,
    *,
    start_time: datetime,
    end_time: datetime,
) -> pd.DataFrame:
    """Accumulate preferred rainfall series over the half-open interval [start, end)."""
    if end_time <= start_time:
        raise ValueError("end_time must be after start_time.")
    with open_history_read_only(database_path) as connection:
        series = read_rain_series(connection)
        preferred = select_preferred_series_rows(series)
        if preferred.empty:
            return pd.DataFrame(columns=["station_id", "lat", "lon", "value"])
        values = read_observed_values(
            connection,
            preferred["series_id"].astype(str).tolist(),
            start_time=start_time,
            end_time=end_time,
        )
    values["value"] = pd.to_numeric(values["value"], errors="coerce")
    totals = values.groupby("series_id", as_index=False)["value"].sum(min_count=1)
    result = preferred.merge(totals, on="series_id", how="left")
    return result[["station_id", "lat", "lon", "value"]].reset_index(drop=True)


def observed_rainfall_grid(
    database_path: Path,
    *,
    grid: RegularGridSpec,
    start_time: datetime,
    end_time: datetime,
    nearest_stations: int = 5,
    power: float = 2.0,
) -> PrecipitationGrid:
    stations = accumulate_observed_rainfall(
        database_path, start_time=start_time, end_time=end_time
    )
    values = (
        interpolate_station_values(
            stations, grid, nearest_stations=nearest_stations, power=power
        )
        if not stations.empty
        else np.full(grid.shape, np.nan)
    )
    return PrecipitationGrid(
        values=values,
        latitudes=grid.latitudes,
        longitudes=grid.longitudes,
        bounds=grid.bbox,
        start_time=start_time,
        end_time=end_time,
        units="mm",
        source="observed",
    )


__all__ = ["accumulate_observed_rainfall", "observed_rainfall_grid"]
