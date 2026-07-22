from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Iterable, Mapping

import numpy as np

from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec


@dataclass(frozen=True, slots=True)
class ForecastCorrectionInstruction:
    asset_id: str
    t0_step: int
    t1_step: int
    shift_lat: float = 0.0
    shift_lon: float = 0.0
    rotation_deg: float = 0.0
    multiplication_factor: float = 1.0
    editor: str | None = None
    reason: str = ""


def validate_instruction(
    instruction: ForecastCorrectionInstruction,
) -> ForecastCorrectionInstruction:
    if instruction.t1_step < instruction.t0_step:
        raise ValueError("t1_step must be >= t0_step.")
    numeric = (
        instruction.shift_lat,
        instruction.shift_lon,
        instruction.rotation_deg,
        instruction.multiplication_factor,
    )
    if not all(np.isfinite(float(value)) for value in numeric):
        raise ValueError("Correction parameters must be finite.")
    if instruction.multiplication_factor <= 0:
        raise ValueError("multiplication_factor must be > 0.")
    return instruction


def shift_pixels(values: np.ndarray, *, row_shift: int = 0, column_shift: int = 0, fill_value: float = 0.0) -> np.ndarray:
    """Legacy fixed-canvas pixel shift retained for array-only callers."""
    data = np.asarray(values, dtype=float)
    if data.ndim != 2:
        raise ValueError("values must be a 2-D grid.")
    out = np.full(data.shape, fill_value, dtype=float)
    rows, columns = data.shape
    source_row_start, source_row_end = max(0, -row_shift), min(rows, rows - row_shift)
    source_col_start, source_col_end = max(0, -column_shift), min(columns, columns - column_shift)
    if source_row_start >= source_row_end or source_col_start >= source_col_end:
        return out
    destination_row_start = max(0, row_shift)
    destination_col_start = max(0, column_shift)
    out[
        destination_row_start:destination_row_start + source_row_end - source_row_start,
        destination_col_start:destination_col_start + source_col_end - source_col_start,
    ] = data[source_row_start:source_row_end, source_col_start:source_col_end]
    return out


def rotate_nearest(values: np.ndarray, angle_degrees: float, *, fill_value: float = 0.0) -> np.ndarray:
    """Legacy fixed-canvas rotation retained for array-only callers."""
    data = np.asarray(values, dtype=float)
    if data.ndim != 2:
        raise ValueError("values must be a 2-D grid.")
    if abs(float(angle_degrees)) < 1e-12:
        return data.copy()
    rows, columns = data.shape
    output_rows, output_columns = np.indices(data.shape, dtype=float)
    center_row, center_column = (rows - 1) / 2.0, (columns - 1) / 2.0
    theta = np.deg2rad(float(angle_degrees))
    relative_row, relative_column = output_rows - center_row, output_columns - center_column
    source_row = center_row + relative_row * np.cos(theta) + relative_column * np.sin(theta)
    source_column = center_column - relative_row * np.sin(theta) + relative_column * np.cos(theta)
    source_row, source_column = np.rint(source_row).astype(int), np.rint(source_column).astype(int)
    valid = (source_row >= 0) & (source_row < rows) & (source_column >= 0) & (source_column < columns)
    out = np.full(data.shape, fill_value, dtype=float)
    out[valid] = data[source_row[valid], source_column[valid]]
    return out


def multiply_positive(values: np.ndarray, factor: float) -> np.ndarray:
    if not np.isfinite(factor) or factor <= 0:
        raise ValueError("multiplication factor must be > 0.")
    return np.asarray(values, dtype=float) * float(factor)


def _grid_resolution(grid: PrecipitationGrid) -> float:
    latitude_steps = np.diff(grid.latitudes)
    longitude_steps = np.diff(grid.longitudes)
    lat_resolution = float(np.median(np.abs(latitude_steps))) if latitude_steps.size else float(grid.bounds[3] - grid.bounds[1])
    lon_resolution = float(np.median(np.abs(longitude_steps))) if longitude_steps.size else float(grid.bounds[2] - grid.bounds[0])
    if lat_resolution <= 0 or lon_resolution <= 0 or not np.isclose(lat_resolution, lon_resolution):
        raise ValueError("Geographic corrections require a regular square grid.")
    return lat_resolution


def _instruction(value: Mapping[str, object] | object) -> ForecastCorrectionInstruction:
    getter = value.get if isinstance(value, Mapping) else lambda key, default: getattr(value, key, default)
    instruction = value if isinstance(value, ForecastCorrectionInstruction) else ForecastCorrectionInstruction(
        asset_id=str(getter("asset_id", "")),
        t0_step=int(getter("t0_step", 0)),
        t1_step=int(getter("t1_step", 0)),
        shift_lat=float(getter("shift_lat", 0.0)),
        shift_lon=float(getter("shift_lon", 0.0)),
        rotation_deg=float(getter("rotation_deg", 0.0)),
        multiplication_factor=float(getter("multiplication_factor", 1.0)),
    )
    return validate_instruction(instruction)


def _rotated_bounds(
    grid: PrecipitationGrid, *, shift_lat: float, shift_lon: float, angle_degrees: float
) -> tuple[float, float, float, float]:
    west, south, east, north = grid.bounds
    center_x, center_y = (west + east) / 2.0, (south + north) / 2.0
    corners = np.array([[west, south], [west, north], [east, south], [east, north]], dtype=float)
    corners[:, 0] += shift_lon
    corners[:, 1] += shift_lat
    theta = np.deg2rad(angle_degrees)
    relative = corners - np.array([center_x, center_y])
    rotation = np.array([[np.cos(theta), -np.sin(theta)], [np.sin(theta), np.cos(theta)]])
    transformed = relative @ rotation.T + np.array([center_x, center_y])
    return (
        float(np.min(transformed[:, 0])),
        float(np.min(transformed[:, 1])),
        float(np.max(transformed[:, 0])),
        float(np.max(transformed[:, 1])),
    )


def apply_grid_correction(
    grid: PrecipitationGrid, instruction: ForecastCorrectionInstruction
) -> PrecipitationGrid:
    """Apply a correction as a geographic transform, preserving its footprint."""
    instruction = validate_instruction(instruction)
    resolution = _grid_resolution(grid)
    shift_lat = int(round(instruction.shift_lat)) * resolution
    shift_lon = int(round(instruction.shift_lon)) * resolution
    angle = float(instruction.rotation_deg)
    values = np.asarray(grid.values, dtype=float)
    if abs(angle) < 1e-12:
        corrected = np.where(
            np.isfinite(values),
            np.maximum(values * instruction.multiplication_factor, 0.0),
            np.nan,
        )
        bounds = tuple(
            float(value + offset)
            for value, offset in zip(grid.bounds, (shift_lon, shift_lat, shift_lon, shift_lat), strict=True)
        )
        return replace(
            grid,
            values=corrected,
            latitudes=grid.latitudes + shift_lat,
            longitudes=grid.longitudes + shift_lon,
            bounds=bounds,
        )

    west, south, east, north = _rotated_bounds(
        grid, shift_lat=shift_lat, shift_lon=shift_lon, angle_degrees=angle
    )
    # Align outward to source cell edges so the transformed footprint is never clipped.
    west = np.floor(west / resolution) * resolution
    south = np.floor(south / resolution) * resolution
    east = np.ceil(east / resolution) * resolution
    north = np.ceil(north / resolution) * resolution
    target = RegularGridSpec((west, south, east, north), resolution)
    output_latitudes, output_longitudes = np.meshgrid(target.latitudes, target.longitudes, indexing="ij")
    center_x, center_y = (grid.bounds[0] + grid.bounds[2]) / 2.0, (grid.bounds[1] + grid.bounds[3]) / 2.0
    theta = np.deg2rad(angle)
    relative_x = output_longitudes - center_x
    relative_y = output_latitudes - center_y
    # Inverse rotation, then undo the preceding translation.
    source_x = center_x + relative_x * np.cos(theta) + relative_y * np.sin(theta) - shift_lon
    source_y = center_y - relative_x * np.sin(theta) + relative_y * np.cos(theta) - shift_lat
    source_rows = np.rint((source_y - grid.latitudes[0]) / resolution).astype(int)
    source_columns = np.rint((source_x - grid.longitudes[0]) / resolution).astype(int)
    valid = (
        (source_rows >= 0)
        & (source_rows < len(grid.latitudes))
        & (source_columns >= 0)
        & (source_columns < len(grid.longitudes))
    )
    corrected = np.full(target.shape, np.nan, dtype=float)
    corrected[valid] = values[source_rows[valid], source_columns[valid]]
    corrected = np.where(
        np.isfinite(corrected),
        np.maximum(corrected * instruction.multiplication_factor, 0.0),
        np.nan,
    )
    return PrecipitationGrid(
        values=corrected,
        latitudes=target.latitudes,
        longitudes=target.longitudes,
        bounds=target.bbox,
        start_time=grid.start_time,
        end_time=grid.end_time,
        units=grid.units,
        source=grid.source,
    )


def apply_forcing_correction(
    values: np.ndarray,
    *,
    shift_lat: float = 0.0,
    shift_lon: float = 0.0,
    rotation_deg: float = 0.0,
    multiplication_factor: float = 1.0,
) -> np.ndarray:
    """Legacy array-only correction with fixed-canvas behavior."""
    corrected = np.where(np.isfinite(np.asarray(values, dtype=float)), values, 0.0)
    corrected = shift_pixels(corrected, row_shift=int(round(float(shift_lat))), column_shift=int(round(float(shift_lon))))
    corrected = rotate_nearest(corrected, float(rotation_deg))
    corrected = multiply_positive(corrected, float(multiplication_factor))
    return np.maximum(corrected, 0.0)


def apply_corrections(
    grid: PrecipitationGrid,
    corrections: Iterable[Mapping[str, object] | object],
) -> PrecipitationGrid:
    corrected = grid
    for correction in corrections:
        corrected = apply_grid_correction(corrected, _instruction(correction))
    return replace(corrected, source=f"{grid.source}:corrected")


pixel_shift = shift_pixels
nearest_neighbor_rotation = rotate_nearest
positive_multiply = multiply_positive
