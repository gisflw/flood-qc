from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np

from mgb_ops.ingest.forecast_grid import _require_eccodes, read_tp_grib_messages


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


@dataclass(frozen=True, slots=True)
class CorrectedGribSummary:
    source_path: Path
    target_path: Path
    instruction_count: int
    corrected_step_count: int


def validate_instruction(instruction: ForecastCorrectionInstruction) -> ForecastCorrectionInstruction:
    if instruction.t1_step < instruction.t0_step:
        raise ValueError("t1_step must be >= t0_step.")
    if instruction.multiplication_factor <= 0:
        raise ValueError("multiplication_factor must be > 0.")
    return instruction


def _shift_with_fill(data: np.ndarray, row_shift: int, col_shift: int) -> np.ndarray:
    out = np.zeros_like(data, dtype=np.float64)
    rows, cols = data.shape

    src_row_start = max(0, -row_shift)
    src_row_end = min(rows, rows - row_shift)
    src_col_start = max(0, -col_shift)
    src_col_end = min(cols, cols - col_shift)

    dst_row_start = max(0, row_shift)
    dst_row_end = dst_row_start + max(0, src_row_end - src_row_start)
    dst_col_start = max(0, col_shift)
    dst_col_end = dst_col_start + max(0, src_col_end - src_col_start)

    if src_row_end <= src_row_start or src_col_end <= src_col_start:
        return out

    out[dst_row_start:dst_row_end, dst_col_start:dst_col_end] = data[src_row_start:src_row_end, src_col_start:src_col_end]
    return out


def _rotate_nearest(data: np.ndarray, rotation_deg: float) -> np.ndarray:
    if abs(float(rotation_deg)) < 1e-9:
        return np.asarray(data, dtype=np.float64).copy()

    rows, cols = data.shape
    row_coords = np.arange(rows, dtype=np.float64)
    col_coords = np.arange(cols, dtype=np.float64)
    col_grid, row_grid = np.meshgrid(col_coords, row_coords)

    center_row = (rows - 1) / 2.0
    center_col = (cols - 1) / 2.0
    theta = np.deg2rad(float(rotation_deg))
    cos_theta = float(np.cos(theta))
    sin_theta = float(np.sin(theta))

    row_rel = row_grid - center_row
    col_rel = col_grid - center_col
    src_row = center_row + row_rel * cos_theta + col_rel * sin_theta
    src_col = center_col - row_rel * sin_theta + col_rel * cos_theta

    src_row_idx = np.rint(src_row).astype(int)
    src_col_idx = np.rint(src_col).astype(int)
    valid = (
        (src_row_idx >= 0)
        & (src_row_idx < rows)
        & (src_col_idx >= 0)
        & (src_col_idx < cols)
    )

    out = np.zeros_like(data, dtype=np.float64)
    out[valid] = np.asarray(data, dtype=np.float64)[src_row_idx[valid], src_col_idx[valid]]
    return out


def apply_grid_correction(
    data: np.ndarray,
    *,
    shift_lat: float,
    shift_lon: float,
    rotation_deg: float,
    multiplication_factor: float,
) -> np.ndarray:
    if multiplication_factor <= 0:
        raise ValueError("multiplication_factor must be > 0.")

    corrected = np.asarray(data, dtype=np.float64).copy()
    corrected = np.where(np.isfinite(corrected), corrected, 0.0)
    corrected = _shift_with_fill(corrected, int(round(float(shift_lat))), int(round(float(shift_lon))))
    corrected = _rotate_nearest(corrected, float(rotation_deg))
    corrected *= float(multiplication_factor)
    corrected[corrected < 0.0] = 0.0
    return corrected


def apply_correction_sequence(
    data: np.ndarray,
    instructions: Iterable[ForecastCorrectionInstruction],
) -> np.ndarray:
    corrected = np.asarray(data, dtype=np.float64).copy()
    for instruction in instructions:
        validated = validate_instruction(instruction)
        corrected = apply_grid_correction(
            corrected,
            shift_lat=validated.shift_lat,
            shift_lon=validated.shift_lon,
            rotation_deg=validated.rotation_deg,
            multiplication_factor=validated.multiplication_factor,
        )
    return corrected


def build_corrected_cumulative_fields(
    source_path: Path,
    instructions: Iterable[ForecastCorrectionInstruction],
) -> list[np.ndarray]:
    messages = read_tp_grib_messages(source_path)
    validated = sorted((validate_instruction(item) for item in instructions), key=lambda item: (item.t0_step, item.t1_step))
    if not validated:
        return [np.asarray(message.values_mm, dtype=np.float64).copy() for message in messages]

    corrected_cumulative_fields: list[np.ndarray] = []
    previous_step = 0
    previous_cumulative = np.zeros_like(messages[0].values_mm, dtype=np.float64)
    corrected_cumulative = np.zeros_like(messages[0].values_mm, dtype=np.float64)

    for message in messages:
        if message.step_hours < previous_step:
            raise ValueError("GRIB tp messages are not ordered by step_hours.")
        increment = np.asarray(message.values_mm, dtype=np.float64) - previous_cumulative
        increment = np.where(np.isfinite(increment), increment, 0.0)
        increment[increment < 0.0] = 0.0

        applicable = [item for item in validated if item.t0_step < message.step_hours <= item.t1_step]
        if applicable:
            increment = apply_correction_sequence(increment, applicable)

        corrected_cumulative = corrected_cumulative + increment
        corrected_cumulative_fields.append(corrected_cumulative.copy())
        previous_step = int(message.step_hours)
        previous_cumulative = np.asarray(message.values_mm, dtype=np.float64)

    return corrected_cumulative_fields


def write_corrected_grib2(
    source_path: Path,
    target_path: Path,
    instructions: Iterable[ForecastCorrectionInstruction],
) -> CorrectedGribSummary:
    normalized_instructions = [validate_instruction(item) for item in instructions]
    corrected_fields = build_corrected_cumulative_fields(source_path, normalized_instructions)
    if not corrected_fields:
        raise ValueError(f"No tp messages found in {source_path}.")

    corrected_fields_iter = iter(corrected_fields)
    eccodes = _require_eccodes()
    target_path.parent.mkdir(parents=True, exist_ok=True)

    corrected_step_count = 0
    with Path(source_path).open("rb") as src_handle, Path(target_path).open("wb") as dst_handle:
        while True:
            gid = eccodes.codes_grib_new_from_file(src_handle)
            if gid is None:
                break
            try:
                short_name = str(eccodes.codes_get(gid, "shortName"))
                if short_name == "tp":
                    corrected_mm = next(corrected_fields_iter)
                    corrected_m = np.asarray(corrected_mm, dtype=np.float64) / 1000.0
                    eccodes.codes_set_array(gid, "values", corrected_m.reshape(-1))
                    corrected_step_count += 1
                eccodes.codes_write(gid, dst_handle)
            finally:
                eccodes.codes_release(gid)

    return CorrectedGribSummary(
        source_path=Path(source_path),
        target_path=Path(target_path),
        instruction_count=len(normalized_instructions),
        corrected_step_count=corrected_step_count,
    )
