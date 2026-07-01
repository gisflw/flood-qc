from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd

from mgb_ops.common.time_utils import DashboardWindow
from mgb_ops.assets.history_queries import read_station_catalog_tables, read_station_observed_tables
from mgb_ops.assets.model_outputs import (
    MGB_VARIABLE_METADATA,
    StaleModelOutputsError,
    TimeSegment,
    list_model_variables,
    load_mgb_series,
    load_weighted_mgb_series,
    validate_model_outputs_netcdf,
)

STATE_PRIORITY = {"approved": 0, "curated": 1, "raw": 2}

def select_preferred_series_rows(series: pd.DataFrame) -> pd.DataFrame:
    """Select one observed series per station/variable by state then recency."""
    if series.empty:
        return series.copy()
    required = {"station_id", "variable_code", "state"}
    missing = required.difference(series.columns)
    if missing:
        raise ValueError(f"Observed series table is missing columns: {sorted(missing)}")
    ranked = series.copy()
    ranked["_state_rank"] = ranked["state"].map(STATE_PRIORITY).fillna(len(STATE_PRIORITY))
    if "created_at" not in ranked:
        ranked["created_at"] = ""
    ranked["created_at"] = ranked["created_at"].fillna("")
    ranked = ranked.sort_values(
        ["station_id", "variable_code", "_state_rank", "created_at"],
        ascending=[True, True, True, False],
    )
    return (
        ranked.drop_duplicates(["station_id", "variable_code"])
        .drop(columns="_state_rank")
        .reset_index(drop=True)
    )


def derive_station_kind(variable_codes: Iterable[str]) -> str:
    codes = {str(value).strip().lower() for value in variable_codes}
    rain = "rain" in codes
    hydro = bool({"level", "flow"} & codes)
    if rain and hydro:
        return "mixed"
    if rain:
        return "rain"
    if hydro:
        return "level"
    return "no_data"


def summarize_station_status(values: pd.DataFrame) -> dict[str, object]:
    if values.empty:
        return {"status": "no_data", "status_reason": "no records in the dashboard window", "rows_recent": 0}
    valid = int(pd.to_numeric(values["value"], errors="coerce").notna().sum())
    if valid == 0:
        return {"status": "data_issue", "status_reason": "only null values in the period", "rows_recent": len(values)}
    return {"status": "ok", "status_reason": "", "rows_recent": len(values)}


def compute_rain_summary(
    values: pd.DataFrame,
    *,
    cutoff_time: datetime | pd.Timestamp,
) -> dict[str, float]:
    empty = {"rain_mean_mm_h": np.nan, "rain_acc_24h_mm": np.nan, "rain_p90_mm_h": np.nan}
    if values.empty:
        return empty
    frame = values.copy()
    frame["datetime"] = pd.to_datetime(frame["datetime"], errors="coerce")
    frame["value"] = pd.to_numeric(frame["value"], errors="coerce")
    frame = frame.dropna(subset=["datetime", "value"])
    if frame.empty:
        return empty
    cutoff = pd.Timestamp(cutoff_time)
    frame = frame[frame["datetime"] <= cutoff]
    if frame.empty:
        return empty
    accumulated = frame.loc[frame["datetime"] > cutoff - pd.Timedelta(hours=24), "value"].sum()
    return {
        "rain_mean_mm_h": float(frame["value"].mean()),
        "rain_acc_24h_mm": float(accumulated),
        "rain_p90_mm_h": float(frame["value"].quantile(0.9)),
    }


def load_station_catalog(
    database_path: Path,
    *,
    start_time: datetime,
    end_time: datetime,
) -> pd.DataFrame:
    """Load mapped stations and preferred-series availability in [start, end]."""
    if end_time < start_time:
        raise ValueError("end_time must be >= start_time.")
    stations, series, values = read_station_catalog_tables(
        database_path, start_time=start_time, end_time=end_time
    )
    columns = [
        "station_id", "station_code", "provider_code", "station_name", "mini_id",
        "lat", "lon", "kind", "status", "status_reason", "rows_recent",
        "rain_mean_mm_h", "rain_acc_24h_mm", "rain_p90_mm_h",
    ]
    if stations.empty:
        return pd.DataFrame(columns=columns)
    for frame in (stations, series, values):
        if "station_id" in frame:
            frame["station_id"] = frame["station_id"].astype(str)
    preferred = select_preferred_series_rows(series)
    preferred_ids = set(preferred["series_id"].astype(str))
    values = values[values["series_id"].astype(str).isin(preferred_ids)].copy()
    values["datetime"] = pd.to_datetime(values["datetime"], errors="coerce")
    coverage = preferred.groupby("station_id")["variable_code"].agg(list).map(derive_station_kind)
    rows: list[dict[str, object]] = []
    for station_id in stations["station_id"]:
        station_values = values[values["station_id"] == station_id]
        rain = station_values[station_values["variable_code"] == "rain"]
        rows.append({"station_id": station_id, **summarize_station_status(station_values), **compute_rain_summary(rain, cutoff_time=end_time)})
    result = stations.merge(coverage.rename("kind"), left_on="station_id", right_index=True, how="left")
    result = result.merge(pd.DataFrame(rows), on="station_id", how="left")
    result["kind"] = result["kind"].fillna("no_data")
    return result.reindex(columns=columns).sort_values(["provider_code", "station_code"]).reset_index(drop=True)


def load_observed_series(
    station_id: str,
    database_path: Path,
    *,
    start_time: datetime,
    end_time: datetime,
) -> pd.DataFrame:
    """Load preferred observed rainfall, level, and flow values for a station."""
    if end_time < start_time:
        raise ValueError("end_time must be >= start_time.")
    series, values = read_station_observed_tables(
        str(station_id), database_path, start_time=start_time, end_time=end_time
    )
    preferred = select_preferred_series_rows(series)
    if preferred.empty:
        return pd.DataFrame(columns=["datetime", "variable_code", "value"])
    preferred_ids = set(preferred["series_id"].astype(str))
    values = values[values["series_id"].astype(str).isin(preferred_ids)].drop(columns="series_id")
    values["datetime"] = pd.to_datetime(values["datetime"], errors="coerce")
    values["value"] = pd.to_numeric(values["value"], errors="coerce")
    return values.dropna(subset=["datetime"]).reset_index(drop=True)


def compute_observed_metrics(values: pd.DataFrame, *, cutoff_time: datetime | pd.Timestamp) -> dict[str, object]:
    result: dict[str, object] = {
        "latest_time": None, "rain_12h": np.nan, "rain_24h": np.nan,
        "rain_72h": np.nan, "level_current": np.nan, "flow_current": np.nan,
    }
    if values.empty:
        return result
    frame = values.copy()
    frame["datetime"] = pd.to_datetime(frame["datetime"], errors="coerce")
    frame["value"] = pd.to_numeric(frame["value"], errors="coerce")
    frame = frame.dropna(subset=["datetime"]).sort_values("datetime")
    if frame.empty:
        return result
    cutoff = pd.Timestamp(cutoff_time)
    frame = frame[frame["datetime"] <= cutoff]
    if frame.empty:
        return result
    latest = frame["datetime"].max()
    result["latest_time"] = latest
    rain = frame[(frame["variable_code"] == "rain") & frame["value"].notna()]
    for hours in (12, 24, 72):
        if not rain.empty:
            result[f"rain_{hours}h"] = float(rain.loc[rain["datetime"] > cutoff - pd.Timedelta(hours=hours), "value"].sum())
    for code in ("level", "flow"):
        selected = frame[(frame["variable_code"] == code) & frame["value"].notna()]
        if not selected.empty:
            result[f"{code}_current"] = float(selected.iloc[-1]["value"])
    return result


def summarize_mini_peaks(
    values: pd.DataFrame,
    *,
    cutoff_time: datetime | pd.Timestamp,
    forecast_end_exclusive: datetime | pd.Timestamp,
) -> dict[str, object]:
    """Summarize current value and current/forecast peaks for a selected mini."""
    empty = {"current_value": np.nan, "current_time": None, "current_peak": np.nan, "forecast_peak": np.nan}
    if values.empty:
        return empty
    frame = values.copy()
    frame["dt"] = pd.to_datetime(frame["dt"], errors="coerce")
    cutoff = pd.Timestamp(cutoff_time)
    forecast_end = pd.Timestamp(forecast_end_exclusive)
    current = frame[(frame["prev_flag"] == 0) & (frame["dt"] <= cutoff)].dropna(subset=["dt", "value"]).sort_values("dt")
    forecast = frame[
        (frame["prev_flag"] == 1) & (frame["dt"] > cutoff) & (frame["dt"] < forecast_end)
    ].dropna(subset=["dt", "value"]).sort_values("dt")
    if current.empty:
        return empty
    end = current.iloc[-1]
    return {
        "current_value": float(end["value"]),
        "current_time": pd.Timestamp(end["dt"]),
        "current_peak": float(current["value"].max()),
        "forecast_peak": float(forecast["value"].max()) if not forecast.empty else np.nan,
    }


def summarize_network_peaks(
    path: Path,
    *,
    variable_code: str = "flow",
    mini_ids: Iterable[int] | None = None,
    window: DashboardWindow,
) -> pd.DataFrame:
    """Return current and forecast peak summaries for every requested mini."""
    metadata = validate_model_outputs_netcdf(path)
    selected = list(mini_ids) if mini_ids is not None else list(metadata["mini_ids"])
    rows = []
    for mini_id in selected:
        series = load_mgb_series(path, mini_id=int(mini_id), variable_code=variable_code, window=window)
        rows.append({
            "mini_id": int(mini_id),
            **summarize_mini_peaks(
                series,
                cutoff_time=window.cutoff_time,
                forecast_end_exclusive=window.forecast_end_exclusive,
            ),
        })
    return pd.DataFrame(rows)


def load_basin_precipitation(
    path: Path,
    *,
    mini_ids: Iterable[int],
    weights: Iterable[float],
    window: DashboardWindow | None = None,
) -> pd.DataFrame:
    """Load an area-weighted precipitation series for a set of mini basins."""
    selected_ids = [int(value) for value in mini_ids]
    selected_weights = np.asarray(list(weights), dtype=float)
    if not selected_ids:
        raise ValueError("Basin precipitation requires at least one mini ID.")
    if len(set(selected_ids)) != len(selected_ids):
        raise ValueError("Basin precipitation mini IDs must be unique.")
    if selected_weights.shape != (len(selected_ids),):
        raise ValueError("Basin precipitation weights must match the mini IDs.")
    if not np.isfinite(selected_weights).all() or (selected_weights <= 0).any():
        raise ValueError("Basin precipitation weights must be finite and positive.")

    frame = load_weighted_mgb_series(
        path,
        mini_ids=selected_ids,
        weights=selected_weights,
        variable_code="precipitation",
        window=window,
    )
    frame["display_name"] = "Basin precipitation"
    return frame


# Clear aliases for callers that prefer noun-specific API names.
read_model_outputs = validate_model_outputs_netcdf
select_mgb_series = load_mgb_series
compute_mini_peak_summary = summarize_mini_peaks
compute_network_peak_summary = summarize_network_peaks
