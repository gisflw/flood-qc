from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Iterable, Mapping

import numpy as np

from mgb_ops.analysis.spatial import PrecipitationGrid


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


def apply_forcing_correction(
    values: np.ndarray,
    *,
    shift_lat: float = 0.0,
    shift_lon: float = 0.0,
    rotation_deg: float = 0.0,
    multiplication_factor: float = 1.0,
) -> np.ndarray:
    corrected = np.where(np.isfinite(np.asarray(values, dtype=float)), values, 0.0)
    corrected = shift_pixels(
        corrected,
        row_shift=int(round(float(shift_lat))),
        column_shift=int(round(float(shift_lon))),
    )
    corrected = rotate_nearest(corrected, float(rotation_deg))
    corrected = multiply_positive(corrected, float(multiplication_factor))
    return np.maximum(corrected, 0.0)


def apply_corrections(
    grid: PrecipitationGrid,
    corrections: Iterable[Mapping[str, object] | object],
) -> PrecipitationGrid:
    values = grid.values.copy()
    for correction in corrections:
        getter = correction.get if isinstance(correction, Mapping) else lambda key, default: getattr(correction, key, default)
        instruction = (
            correction
            if isinstance(correction, ForecastCorrectionInstruction)
            else ForecastCorrectionInstruction(
                asset_id=str(getter("asset_id", "")),
                t0_step=int(getter("t0_step", 0)),
                t1_step=int(getter("t1_step", 0)),
                shift_lat=float(getter("shift_lat", 0.0)),
                shift_lon=float(getter("shift_lon", 0.0)),
                rotation_deg=float(getter("rotation_deg", 0.0)),
                multiplication_factor=float(getter("multiplication_factor", 1.0)),
            )
        )
        instruction = validate_instruction(instruction)
        values = apply_forcing_correction(
            values,
            shift_lat=instruction.shift_lat,
            shift_lon=instruction.shift_lon,
            rotation_deg=instruction.rotation_deg,
            multiplication_factor=instruction.multiplication_factor,
        )
    return replace(grid, values=values, source=f"{grid.source}:corrected")


pixel_shift = shift_pixels
nearest_neighbor_rotation = rotate_nearest
positive_multiply = multiply_positive
