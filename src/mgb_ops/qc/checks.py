from __future__ import annotations

from pathlib import Path
from typing import Iterable, Mapping

import numpy as np
import pandas as pd

from mgb_ops.assets.model_outputs import validate_model_outputs_netcdf
from mgb_ops.assets.spatial_grid import read_spatial_grid
from mgb_ops.qc.rules import (
    CORRECTION_OVERLAP,
    CORRECTION_WINDOW,
    NETCDF_CONTRACT,
    PRECIPITATION_VALID,
    STATION_AVAILABLE,
    QCResult,
)


def check_station_availability(
    values: pd.DataFrame,
    *,
    required_variables: Iterable[str] = (),
) -> QCResult:
    if values.empty:
        return QCResult(STATION_AVAILABLE, False, "warning", "No station observations are available.")
    present = set(values.loc[pd.to_numeric(values["value"], errors="coerce").notna(), "variable_code"].astype(str))
    missing = sorted(set(required_variables).difference(present))
    if missing:
        return QCResult(STATION_AVAILABLE, False, "warning", f"Missing station variables: {missing}.")
    return QCResult(STATION_AVAILABLE, True, "info", "Station observations are available.")


def check_netcdf_contract(path: Path, *, contract: str) -> QCResult:
    try:
        if contract == "mgb":
            validate_model_outputs_netcdf(path)
        elif contract in {"forecast", "spatial_grid"}:
            grid = read_spatial_grid(path)
            if contract == "forecast" and grid.grid_type != "forecast":
                raise ValueError("Expected a forecast spatial grid.")
        else:
            raise ValueError("contract must be 'mgb', 'forecast', or 'spatial_grid'.")
    except (FileNotFoundError, OSError, ValueError) as exc:
        return QCResult(NETCDF_CONTRACT, False, "error", str(exc))
    return QCResult(NETCDF_CONTRACT, True, "info", f"Valid {contract} NetCDF contract.")


def check_precipitation(values: np.ndarray) -> QCResult:
    data = np.asarray(values, dtype=float)
    if data.size == 0:
        return QCResult(PRECIPITATION_VALID, False, "error", "Precipitation grid is empty.")
    if np.isinf(data).any():
        return QCResult(PRECIPITATION_VALID, False, "error", "Precipitation contains infinite values.")
    if np.any(data[np.isfinite(data)] < 0):
        return QCResult(PRECIPITATION_VALID, False, "error", "Precipitation contains negative values.")
    if not np.isfinite(data).any():
        return QCResult(PRECIPITATION_VALID, False, "warning", "Precipitation has no finite values.")
    return QCResult(PRECIPITATION_VALID, True, "info", "Precipitation values are valid.")


def check_correction_window(t0_step: int, t1_step: int, *, available_steps: Iterable[int] | None = None) -> QCResult:
    if int(t0_step) < 0 or int(t1_step) <= int(t0_step):
        return QCResult(CORRECTION_WINDOW, False, "error", "Correction window must satisfy 0 <= t0_step < t1_step.")
    if available_steps is not None:
        steps = set(int(value) for value in available_steps)
        if int(t0_step) not in steps or int(t1_step) not in steps:
            return QCResult(CORRECTION_WINDOW, False, "error", "Correction window is outside available forecast boundaries.")
    return QCResult(CORRECTION_WINDOW, True, "info", "Correction window is valid.")


def check_correction_overlaps(rows: Iterable[Mapping[str, object]]) -> QCResult:
    ordered = sorted(
        ((int(row["t0_step"]), int(row["t1_step"])) for row in rows),
        key=lambda value: (value[0], value[1]),
    )
    for previous, current in zip(ordered, ordered[1:]):
        if current[0] < previous[1]:
            return QCResult(
                CORRECTION_OVERLAP, False, "error",
                f"Correction windows overlap: {previous} and {current}.",
            )
    return QCResult(CORRECTION_OVERLAP, True, "info", "Correction windows do not overlap.")


validate_station_availability = check_station_availability
validate_precipitation = check_precipitation
validate_correction_window = check_correction_window
validate_correction_overlaps = check_correction_overlaps
