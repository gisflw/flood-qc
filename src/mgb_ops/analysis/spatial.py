from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.analysis.timeseries import select_preferred_series_rows
from mgb_ops.assets.history_queries import open_history_read_only, read_observed_values, read_rain_series


@dataclass(frozen=True, slots=True, init=False)
class RegularGridSpec:
    bbox: tuple[float, float, float, float]
    resolution: float

    def __init__(
        self,
        bbox: tuple[float, float, float, float],
        resolution: float | None = None,
        *,
        resolution_degrees: float | None = None,
    ) -> None:
        if resolution is None:
            resolution = resolution_degrees
        elif resolution_degrees is not None and float(resolution) != float(resolution_degrees):
            raise ValueError("resolution and resolution_degrees must match when both are provided.")
        if resolution is None:
            raise TypeError("resolution is required.")
        west, south, east, north = (float(value) for value in bbox)
        if west >= east or south >= north:
            raise ValueError("bbox must satisfy west < east and south < north.")
        if not np.isfinite(resolution) or resolution <= 0:
            raise ValueError("resolution must be > 0.")
        object.__setattr__(self, "bbox", (west, south, east, north))
        object.__setattr__(self, "resolution", float(resolution))

    @property
    def resolution_degrees(self) -> float:
        return self.resolution

    @property
    def longitudes(self) -> np.ndarray:
        west, _, east, _ = self.bbox
        return _coordinate_centers(west, east, self.resolution)

    @property
    def latitudes(self) -> np.ndarray:
        _, south, _, north = self.bbox
        return _coordinate_centers(south, north, self.resolution)

    @property
    def shape(self) -> tuple[int, int]:
        return (len(self.latitudes), len(self.longitudes))


def _coordinate_centers(lower: float, upper: float, resolution: float) -> np.ndarray:
    coordinates = np.arange(lower + resolution / 2.0, upper, resolution, dtype=float)
    if coordinates.size == 0:
        return np.array([(lower + upper) / 2.0], dtype=float)
    return coordinates


@dataclass(frozen=True, slots=True)
class PrecipitationGrid:
    values: np.ndarray
    latitudes: np.ndarray
    longitudes: np.ndarray
    bounds: tuple[float, float, float, float]
    start_time: datetime | pd.Timestamp
    end_time: datetime | pd.Timestamp
    units: str = "mm"
    source: str = ""

    def __post_init__(self) -> None:
        values = np.asarray(self.values, dtype=float)
        latitudes = np.asarray(self.latitudes, dtype=float)
        longitudes = np.asarray(self.longitudes, dtype=float)
        if latitudes.ndim != 1 or longitudes.ndim != 1:
            raise ValueError("latitudes and longitudes must be 1-D.")
        if values.shape != (latitudes.size, longitudes.size):
            raise ValueError(
                f"values shape must be {(latitudes.size, longitudes.size)}, found {values.shape}."
            )
        if pd.Timestamp(self.end_time) <= pd.Timestamp(self.start_time):
            raise ValueError("end_time must be after start_time.")
        object.__setattr__(self, "values", values)
        object.__setattr__(self, "latitudes", latitudes)
        object.__setattr__(self, "longitudes", longitudes)

    @property
    def time_window(self) -> tuple[pd.Timestamp, pd.Timestamp]:
        return pd.Timestamp(self.start_time), pd.Timestamp(self.end_time)


def idw_interpolate(
    station_longitudes: np.ndarray,
    station_latitudes: np.ndarray,
    station_values: np.ndarray,
    target_longitudes: np.ndarray,
    target_latitudes: np.ndarray,
    *,
    nearest_stations: int = 5,
    power: float = 2.0,
) -> np.ndarray:
    """Interpolate point values to targets using deterministic nearest-k IDW."""
    x = np.asarray(station_longitudes, dtype=float).reshape(-1)
    y = np.asarray(station_latitudes, dtype=float).reshape(-1)
    z = np.asarray(station_values, dtype=float).reshape(-1)
    tx, ty = np.broadcast_arrays(np.asarray(target_longitudes, dtype=float), np.asarray(target_latitudes, dtype=float))
    valid = np.isfinite(x) & np.isfinite(y) & np.isfinite(z)
    x, y, z = x[valid], y[valid], z[valid]
    if len(z) == 0:
        return np.full(tx.shape, np.nan, dtype=float)
    if nearest_stations < 1:
        raise ValueError("nearest_stations must be >= 1.")
    if power <= 0:
        raise ValueError("power must be > 0.")
    points = np.column_stack([tx.ravel(), ty.ravel()])
    distances = np.hypot(points[:, None, 0] - x[None, :], points[:, None, 1] - y[None, :])
    k = min(int(nearest_stations), len(z))
    nearest = np.argpartition(distances, kth=k - 1, axis=1)[:, :k]
    selected_distances = np.take_along_axis(distances, nearest, axis=1)
    selected_values = z[nearest]
    result = np.empty(len(points), dtype=float)
    exact = selected_distances == 0
    has_exact = exact.any(axis=1)
    if has_exact.any():
        result[has_exact] = np.array([
            values_row[mask].mean()
            for values_row, mask in zip(selected_values[has_exact], exact[has_exact])
        ])
    remaining = ~has_exact
    if remaining.any():
        weights = 1.0 / np.power(selected_distances[remaining], power)
        result[remaining] = np.sum(weights * selected_values[remaining], axis=1) / weights.sum(axis=1)
    return result.reshape(tx.shape)


def build_idw_neighbors(
    target_points: pd.DataFrame,
    source_points: pd.DataFrame,
    *,
    nearest_stations: int,
    power: float,
) -> tuple[np.ndarray, np.ndarray]:
    """Precompute source indices and IDW weights for repeated fields."""
    if source_points.empty:
        raise ValueError("At least one source point is required for interpolation.")
    if nearest_stations < 1:
        raise ValueError("nearest_stations must be >= 1.")
    if power <= 0:
        raise ValueError("power must be > 0.")
    k = min(int(nearest_stations), len(source_points))
    target_lat = target_points["lat"].to_numpy(dtype=float)
    target_lon = target_points["lon"].to_numpy(dtype=float)
    source_lat = source_points["lat"].to_numpy(dtype=float)
    source_lon = source_points["lon"].to_numpy(dtype=float)
    distances = np.hypot(
        target_lat[:, None] - source_lat[None, :],
        target_lon[:, None] - source_lon[None, :],
    )
    nearest = np.argsort(distances, axis=1)[:, :k]
    nearest_distances = np.take_along_axis(distances, nearest, axis=1)
    safe_distances = np.where(nearest_distances == 0, 1e-12, nearest_distances)
    weights = 1.0 / np.power(safe_distances, float(power))
    return nearest.astype(np.int32), weights


def build_grid_idw_neighbors(
    target_points: pd.DataFrame,
    *,
    latitudes: np.ndarray,
    longitudes: np.ndarray,
    nearest_points: int,
    power: float,
) -> tuple[np.ndarray, np.ndarray]:
    if np.asarray(latitudes).size < 1 or np.asarray(longitudes).size < 1:
        raise ValueError("Source grid must contain at least one latitude and one longitude.")
    longitude_grid, latitude_grid = np.meshgrid(longitudes, latitudes)
    source_points = pd.DataFrame({
        "lat": latitude_grid.reshape(-1),
        "lon": longitude_grid.reshape(-1),
    })
    return build_idw_neighbors(
        target_points,
        source_points,
        nearest_stations=min(int(nearest_points), len(source_points)),
        power=power,
    )


def interpolate_station_chunk(
    source_chunk: np.ndarray,
    *,
    nearest_idx: np.ndarray,
    weights: np.ndarray,
) -> np.ndarray:
    """Apply precomputed IDW neighbors to a source-by-time value matrix."""
    target_count, neighbor_count = nearest_idx.shape
    time_count = source_chunk.shape[1]
    gathered = source_chunk[nearest_idx.reshape(-1), :].reshape(
        target_count, neighbor_count, time_count
    )
    valid = np.isfinite(gathered)
    weighted = np.where(valid, gathered * weights[:, :, None], 0.0)
    weight_sum = np.where(valid, weights[:, :, None], 0.0).sum(axis=1)
    if np.any(weight_sum <= 0):
        count = int((weight_sum <= 0).sum())
        raise ValueError(f"Interpolation left {count} target/time positions without coverage.")
    return np.divide(weighted.sum(axis=1), weight_sum)


def interpolate_station_values(
    stations: pd.DataFrame,
    grid: RegularGridSpec,
    *,
    value_column: str = "value",
    nearest_stations: int = 5,
    power: float = 2.0,
) -> np.ndarray:
    lon_grid, lat_grid = np.meshgrid(grid.longitudes, grid.latitudes)
    return idw_interpolate(
        stations["lon"].to_numpy(), stations["lat"].to_numpy(), stations[value_column].to_numpy(),
        lon_grid, lat_grid, nearest_stations=nearest_stations, power=power,
    )


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
        ids = preferred["series_id"].astype(str).tolist()
        values = read_observed_values(
            connection, ids, start_time=start_time, end_time=end_time
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
    stations = accumulate_observed_rainfall(database_path, start_time=start_time, end_time=end_time)
    values = interpolate_station_values(
        stations, grid, nearest_stations=nearest_stations, power=power
    ) if not stations.empty else np.full(grid.shape, np.nan)
    return PrecipitationGrid(
        values=values, latitudes=grid.latitudes, longitudes=grid.longitudes,
        bounds=grid.bbox, start_time=start_time, end_time=end_time,
        units="mm", source="observed",
    )


def bilinear_resample(
    values: np.ndarray,
    source_latitudes: np.ndarray,
    source_longitudes: np.ndarray,
    target_latitudes: np.ndarray,
    target_longitudes: np.ndarray,
) -> np.ndarray:
    """Bilinearly resample a rectilinear regular grid without GIS dependencies."""
    data = np.asarray(values, dtype=float)
    src_y = np.asarray(source_latitudes, dtype=float)
    src_x = np.asarray(source_longitudes, dtype=float)
    dst_y = np.asarray(target_latitudes, dtype=float)
    dst_x = np.asarray(target_longitudes, dtype=float)
    if data.shape != (len(src_y), len(src_x)):
        raise ValueError("values shape does not match source coordinates.")
    if len(src_y) == 0 or len(src_x) == 0:
        raise ValueError("source coordinates cannot be empty.")
    if src_y[0] > src_y[-1]:
        src_y, data = src_y[::-1], data[::-1, :]
    if src_x[0] > src_x[-1]:
        src_x, data = src_x[::-1], data[:, ::-1]
    intermediate = np.vstack([
        np.interp(dst_x, src_x, row, left=np.nan, right=np.nan) for row in data
    ])
    return np.vstack([
        np.interp(dst_y, src_y, intermediate[:, column], left=np.nan, right=np.nan)
        for column in range(len(dst_x))
    ]).T


def resample_regular_grid(values: np.ndarray, source_latitudes: np.ndarray, source_longitudes: np.ndarray, grid: RegularGridSpec) -> np.ndarray:
    return bilinear_resample(values, source_latitudes, source_longitudes, grid.latitudes, grid.longitudes)


# Compatibility names that describe the algorithms explicitly.
interpolate_idw = idw_interpolate
resample_bilinear = bilinear_resample
build_observed_precipitation_grid = observed_rainfall_grid
