from __future__ import annotations

import numpy as np
import pandas as pd

from mgb_ops.assets.spatial_grid import RegularGridSpec


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


def resample_regular_grid(
    values: np.ndarray,
    source_latitudes: np.ndarray,
    source_longitudes: np.ndarray,
    grid: RegularGridSpec,
) -> np.ndarray:
    return bilinear_resample(
        values,
        source_latitudes,
        source_longitudes,
        grid.latitudes,
        grid.longitudes,
    )
