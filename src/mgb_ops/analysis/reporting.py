"""Read-only calculations and queries used to assemble operational reports."""
from __future__ import annotations

import math
from datetime import datetime
from pathlib import Path
from typing import Sequence

import pandas as pd

from mgb_ops.assets.history_queries import open_history_read_only
from mgb_ops.utils.time import TIMEZONE

def local_report_time(value: datetime | str | pd.Timestamp) -> pd.Timestamp:
    timestamp = pd.Timestamp(value)
    return timestamp.tz_convert(TIMEZONE).tz_localize(None) if timestamp.tzinfo else timestamp

def load_report_stations(history_path: Path, station_ids: Sequence[str]) -> dict[str, dict[str, object]]:
    if not station_ids:
        return {}
    with open_history_read_only(history_path) as connection:
        rows = connection.execute(
            "SELECT station_id, station_name, mini_id, latitude, longitude "
            "FROM station ORDER BY station_id"
        ).fetchall()
    all_rows = {
        str(row["station_id"]): {
            "station_id": str(row["station_id"]),
            "station_name": str(row["station_name"] or row["station_id"]),
            "mini_id": int(row["mini_id"]) if row["mini_id"] is not None else None,
            "lat": float(row["latitude"]) if row["latitude"] is not None else None,
            "lon": float(row["longitude"]) if row["longitude"] is not None else None,
        }
        for row in rows
    }
    missing = [station_id for station_id in station_ids if station_id not in all_rows]
    if missing:
        raise ValueError(f"Station IDs were not found in history: {missing}.")
    return {station_id: all_rows[station_id] for station_id in station_ids}

def clean_report_timeseries(frame: pd.DataFrame, *, time_col: str) -> pd.DataFrame:
    if frame.empty:
        return frame.copy()
    result = frame.copy()
    result[time_col] = pd.to_datetime(result[time_col], errors="coerce")
    result[time_col] = result[time_col].map(lambda value: local_report_time(value) if pd.notna(value) else pd.NaT)
    result["value"] = pd.to_numeric(result["value"], errors="coerce")
    result["value"] = result["value"].where(
        result["value"].map(lambda value: pd.notna(value) and math.isfinite(value))
    )
    return result.dropna(subset=[time_col]).sort_values(time_col).reset_index(drop=True)

def exact_series_value(frame: pd.DataFrame, *, variable: str, at: pd.Timestamp, time_col: str) -> float | None:
    if frame.empty or "variable_code" not in frame:
        return None
    rows = frame[
        (frame["variable_code"] == variable)
        & (frame[time_col] == at)
    ]
    if rows.empty:
        return None
    values = pd.to_numeric(rows["value"], errors="coerce").dropna()
    return float(values.iloc[-1]) if not values.empty else None

def evaluate_report_series(
    frame: pd.DataFrame,
    *,
    variable: str,
    reference: pd.Timestamp,
    horizon: int,
    operation: str,
    time_col: str,
    timestep_hours: int,
) -> float | None:
    end = reference + pd.Timedelta(hours=horizon)
    if operation == "value":
        return exact_series_value(frame, variable=variable, at=end, time_col=time_col)
    if operation == "change":
        if horizon == 0:
            return 0.0 if exact_series_value(frame, variable=variable, at=reference, time_col=time_col) is not None else None
        first, last = sorted((reference, end))
        earlier = exact_series_value(frame, variable=variable, at=first, time_col=time_col)
        later = exact_series_value(frame, variable=variable, at=last, time_col=time_col)
        return later - earlier if earlier is not None and later is not None else None
    if horizon == 0:
        return 0.0
    step = 1 if time_col == "datetime" else timestep_hours
    if abs(horizon) % step:
        return None
    start, stop = sorted((reference, end))
    expected = pd.date_range(start + pd.Timedelta(hours=step), stop, freq=f"{step}h")
    if not len(expected):
        return 0.0
    selected = frame[(frame[time_col] > start) & (frame[time_col] <= stop) & (frame["variable_code"] == variable)]
    values = selected.set_index(time_col)["value"].reindex(expected)
    if values.isna().any():
        return None
    return float(values.sum())


__all__ = [
    "clean_report_timeseries", "evaluate_report_series", "exact_series_value",
    "load_report_stations", "local_report_time",
]
