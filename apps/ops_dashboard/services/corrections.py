"""Correction-frame normalization, row creation, and persistence shapes."""
from __future__ import annotations

import json
from typing import Any

import numpy as np
import pandas as pd

from apps.ops_dashboard.services import forecast as dashboard_forecast
from mgb_ops.edit.forcing import ForecastCorrectionInstruction


FORECAST_EDIT_COLUMNS = [
    "manual_edit_id",
    "asset_id",
    "t0_step",
    "t1_step",
    "shift_lat",
    "shift_lon",
    "rotation_deg",
    "multiplication_factor",
    "editor",
    "reason",
    "metadata_json",
    "created_at",
    "remove",
]
FORECAST_EDIT_NUMERIC_COLUMNS = [
    "t0_step",
    "t1_step",
    "shift_lat",
    "shift_lon",
    "rotation_deg",
    "multiplication_factor",
]


def normalize_forecast_edit_frame(frame: pd.DataFrame | None) -> pd.DataFrame:
    normalized = pd.DataFrame() if frame is None else frame.copy()
    for column in FORECAST_EDIT_COLUMNS:
        if column not in normalized.columns:
            normalized[column] = pd.NA
    normalized["manual_edit_id"] = pd.to_numeric(
        normalized["manual_edit_id"], errors="coerce"
    ).astype("Int64")
    for column in FORECAST_EDIT_NUMERIC_COLUMNS:
        normalized[column] = pd.to_numeric(normalized[column], errors="coerce")
    for column in ("asset_id", "editor", "reason", "created_at"):
        normalized[column] = normalized[column].fillna("").astype(str)
    normalized["metadata_json"] = (
        normalized["metadata_json"].fillna("{}").astype(str)
    )
    normalized["remove"] = normalized["remove"].fillna(False).astype(bool)
    return normalized[FORECAST_EDIT_COLUMNS]


def empty_forecast_edit_frame() -> pd.DataFrame:
    return normalize_forecast_edit_frame(None)


def build_forecast_edit_row(
    *,
    asset_id: str,
    t0_step: int,
    t1_step: int,
    shift_lat: float,
    shift_lon: float,
    rotation_deg: float,
    multiplication_factor: float,
    editor: str,
    reason: str,
    metadata: dict[str, Any] | None = None,
) -> dict[str, object]:
    return {
        "manual_edit_id": pd.NA,
        "asset_id": asset_id,
        "t0_step": int(t0_step),
        "t1_step": int(t1_step),
        "shift_lat": float(shift_lat),
        "shift_lon": float(shift_lon),
        "rotation_deg": float(rotation_deg),
        "multiplication_factor": float(multiplication_factor),
        "editor": str(editor),
        "reason": str(reason),
        "metadata_json": json.dumps(metadata or {}, sort_keys=True, ensure_ascii=True),
        "created_at": "",
        "remove": False,
    }


def validate_forecast_edit_draft(
    asset_id: str, frame: pd.DataFrame
) -> list[dict[str, Any]]:
    normalized = normalize_forecast_edit_frame(frame)
    active = normalized.loc[~normalized["remove"]].reset_index(drop=True)
    rows: list[dict[str, Any]] = []
    for row_index, row in enumerate(active.itertuples(index=False), start=1):
        if pd.isna(row.t0_step) or pd.isna(row.t1_step):
            raise ValueError(f"Row {row_index}: t0_step and t1_step are required.")
        if pd.isna(row.multiplication_factor):
            raise ValueError(
                f"Row {row_index}: multiplication_factor is required."
            )
        t0_step, t1_step = int(row.t0_step), int(row.t1_step)
        if t1_step < t0_step:
            raise ValueError(f"Row {row_index}: t1_step must be >= t0_step.")
        factor = float(row.multiplication_factor)
        if not np.isfinite(factor) or factor <= 0:
            raise ValueError(
                f"Row {row_index}: multiplication_factor must be > 0."
            )
        reason = str(row.reason or "").strip()
        if not reason:
            raise ValueError(f"Row {row_index}: correction reason is required.")
        metadata_json = str(row.metadata_json or "{}").strip() or "{}"
        try:
            metadata = json.loads(metadata_json)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Row {row_index}: invalid metadata_json.") from exc
        rows.append(
            {
                "asset_id": asset_id,
                "t0_step": t0_step,
                "t1_step": t1_step,
                "shift_lat": float(
                    0.0 if pd.isna(row.shift_lat) else row.shift_lat
                ),
                "shift_lon": float(
                    0.0 if pd.isna(row.shift_lon) else row.shift_lon
                ),
                "rotation_deg": float(
                    0.0 if pd.isna(row.rotation_deg) else row.rotation_deg
                ),
                "multiplication_factor": factor,
                "editor": str(row.editor or "").strip() or None,
                "reason": reason,
                "metadata": metadata,
            }
        )
    rows.sort(key=lambda item: (item["t0_step"], item["t1_step"]))
    for previous, current in zip(rows, rows[1:]):
        if current["t0_step"] < previous["t1_step"]:
            raise ValueError(
                "Overlapping grid corrections: "
                f"[{previous['t0_step']}, {previous['t1_step']}] x "
                f"[{current['t0_step']}, {current['t1_step']}]."
            )
    return rows


def build_forecast_instruction_from_request(
    request: dashboard_forecast.ForecastPreviewRequest,
) -> ForecastCorrectionInstruction:
    return ForecastCorrectionInstruction(
        asset_id=request.asset_id,
        t0_step=request.t0_step,
        t1_step=request.t1_step,
        shift_lat=request.shift_lat,
        shift_lon=request.shift_lon,
        rotation_deg=request.rotation_deg,
        multiplication_factor=request.multiplication_factor,
    )


__all__ = [
    "FORECAST_EDIT_COLUMNS",
    "build_forecast_edit_row",
    "build_forecast_instruction_from_request",
    "empty_forecast_edit_frame",
    "normalize_forecast_edit_frame",
    "validate_forecast_edit_draft",
]
