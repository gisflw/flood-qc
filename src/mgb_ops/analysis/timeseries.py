from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path
from typing import Iterable, Literal

import numpy as np
import pandas as pd
import xarray as xr

from mgb_ops.common.time_utils import DashboardWindow

STATE_PRIORITY = {"approved": 0, "curated": 1, "raw": 2}
MGB_VARIABLE_METADATA = {
    "q": {"display_name": "QTUDO", "unit": "m3/s"},
    "y": {"display_name": "YTUDO", "unit": "m"},
}
TimeSegment = Literal["all", "current", "forecast"]


def _connect_read_only(database_path: Path) -> sqlite3.Connection:
    path = Path(database_path)
    if not path.exists():
        raise FileNotFoundError(f"History database not found: {path}")
    connection = sqlite3.connect(f"{path.resolve().as_uri()}?mode=ro", uri=True)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA query_only = ON")
    return connection


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


class StaleModelOutputsError(ValueError):
    """Raised when canonical model output metadata differs from the requested run."""


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
    with _connect_read_only(database_path) as connection:
        stations = pd.read_sql_query(
            """SELECT station_id, station_code, provider_code, station_name,
                      mini_id, latitude AS lat, longitude AS lon
               FROM station WHERE latitude IS NOT NULL AND longitude IS NOT NULL
               ORDER BY provider_code, station_code""",
            connection,
        )
        series = pd.read_sql_query(
            "SELECT series_id, station_id, variable_code, state, created_at FROM observed_series",
            connection,
        )
        values = pd.read_sql_query(
            """SELECT os.series_id, os.station_id, os.variable_code,
                      ov.observed_at AS datetime, ov.value
               FROM observed_series os JOIN observed_value ov USING (series_id)
               WHERE ov.observed_at >= ? AND ov.observed_at <= ?""",
            connection,
            params=(
                start_time.strftime("%Y-%m-%d %H:%M:%S"),
                end_time.strftime("%Y-%m-%d %H:%M:%S"),
            ),
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
    with _connect_read_only(database_path) as connection:
        series = pd.read_sql_query(
            """SELECT series_id, station_id, variable_code, state, created_at
               FROM observed_series WHERE station_id = ?""",
            connection,
            params=(str(station_id),),
        )
        preferred = select_preferred_series_rows(series)
        if preferred.empty:
            return pd.DataFrame(columns=["datetime", "variable_code", "value"])
        ids = preferred["series_id"].astype(str).tolist()
        placeholders = ",".join("?" for _ in ids)
        values = pd.read_sql_query(
            f"""SELECT os.variable_code, ov.observed_at AS datetime, ov.value
                FROM observed_value ov JOIN observed_series os USING (series_id)
                WHERE ov.series_id IN ({placeholders})
                  AND ov.observed_at >= ? AND ov.observed_at <= ?
                ORDER BY ov.observed_at, os.variable_code""",
            connection,
            params=(
                *ids,
                start_time.strftime("%Y-%m-%d %H:%M:%S"),
                end_time.strftime("%Y-%m-%d %H:%M:%S"),
            ),
        )
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


def _required_model_time_attr(dataset: xr.Dataset, name: str) -> pd.Timestamp:
    raw = dataset.attrs.get(name)
    if raw in (None, ""):
        raise ValueError(f"MGB NetCDF missing required global attribute {name!r}.")
    try:
        value = pd.Timestamp(raw)
    except Exception as exc:
        raise ValueError(f"MGB NetCDF has invalid global attribute {name!r}: {raw!r}.") from exc
    if value.tzinfo is not None:
        value = value.tz_convert(None)
    return value


def validate_model_outputs_netcdf(
    path: Path,
    *,
    expected_window: DashboardWindow | None = None,
) -> dict[str, object]:
    """Validate the canonical MGB dashboard contract and return its metadata."""
    source = Path(path)
    if not source.exists():
        raise FileNotFoundError(f"Canonical MGB NetCDF not found: {source}")
    with xr.open_dataset(source, decode_times=True) as dataset:
        required_dims = {"time", "mini"}
        missing_dims = required_dims.difference(dataset.dims)
        if missing_dims:
            raise ValueError(f"MGB NetCDF missing required dimensions: {sorted(missing_dims)}")
        required = {"mini_id", "time_segment"}
        missing = required.difference(dataset.variables)
        if missing:
            raise ValueError(f"MGB NetCDF missing required variables: {sorted(missing)}")
        present = [code for code in MGB_VARIABLE_METADATA if code in dataset]
        if not present:
            raise ValueError("MGB NetCDF must contain at least one model variable: q or y.")
        if dataset["mini_id"].dims != ("mini",):
            raise ValueError("MGB NetCDF mini_id must use dimension ('mini',).")
        if dataset["time_segment"].dims != ("time",):
            raise ValueError("MGB NetCDF time_segment must use dimension ('time',).")
        for code in present:
            if dataset[code].dims != ("time", "mini"):
                raise ValueError(f"MGB NetCDF {code} must use dimensions ('time', 'mini').")
        mini_ids = np.asarray(dataset["mini_id"].values)
        if len(np.unique(mini_ids)) != len(mini_ids):
            raise ValueError("MGB NetCDF mini_id values must be unique.")
        segments = set(np.asarray(dataset["time_segment"].values).astype(int).tolist())
        if not segments.issubset({0, 1}):
            raise ValueError("MGB NetCDF time_segment may contain only 0 (current) and 1 (forecast).")
        times = pd.to_datetime(dataset["time"].values, errors="coerce")
        if pd.isna(times).any() or not pd.DatetimeIndex(times).is_monotonic_increasing:
            raise ValueError("MGB NetCDF time must be valid and monotonically increasing.")
        model_window = DashboardWindow(
            start_time=_required_model_time_attr(dataset, "window_start").to_pydatetime(),
            cutoff_time=_required_model_time_attr(dataset, "reference_time").to_pydatetime(),
            forecast_end_exclusive=_required_model_time_attr(dataset, "window_end_exclusive").to_pydatetime(),
        )
        if expected_window is not None and model_window != expected_window:
            raise StaleModelOutputsError(
                "Stale model_outputs.nc metadata: "
                f"expected start={expected_window.start_time.isoformat()}, "
                f"reference={expected_window.cutoff_time.isoformat()}, "
                f"end_exclusive={expected_window.forecast_end_exclusive.isoformat()}; "
                f"actual start={model_window.start_time.isoformat()}, "
                f"reference={model_window.cutoff_time.isoformat()}, "
                f"end_exclusive={model_window.forecast_end_exclusive.isoformat()}."
            )
        return {
            "path": source,
            "mini_count": int(dataset.sizes["mini"]),
            "time_count": int(dataset.sizes["time"]),
            "variables": tuple(present),
            "mini_ids": tuple(int(value) for value in mini_ids),
            "start_time": pd.Timestamp(times[0]) if len(times) else None,
            "end_time": pd.Timestamp(times[-1]) if len(times) else None,
            "window": model_window,
        }


def list_model_variables(path: Path | None = None) -> pd.DataFrame:
    available = set(MGB_VARIABLE_METADATA)
    if path is not None:
        available = set(validate_model_outputs_netcdf(path)["variables"])
    return pd.DataFrame([
        {"variable_code": code, **MGB_VARIABLE_METADATA[code]}
        for code in sorted(available)
    ])


def load_mgb_series(
    path: Path,
    *,
    mini_id: int,
    variable_code: str,
    time_segment: TimeSegment | int | None = "all",
    window: DashboardWindow | None = None,
) -> pd.DataFrame:
    """Select one mini/variable from canonical model_outputs.nc."""
    validate_model_outputs_netcdf(path, expected_window=window)
    code = str(variable_code).strip().lower()
    if code not in MGB_VARIABLE_METADATA:
        raise ValueError("variable_code must be 'q' or 'y'.")
    with xr.open_dataset(path, decode_times=True) as dataset:
        if code not in dataset:
            raise ValueError(f"MGB NetCDF does not contain variable {code!r}.")
        matches = np.flatnonzero(np.asarray(dataset["mini_id"].values) == int(mini_id))
        if len(matches) == 0:
            raise ValueError(f"Mini {mini_id} was not found in {path}.")
        values = np.asarray(dataset[code].isel(mini=int(matches[0])).values, dtype=float)
        frame = pd.DataFrame({
            "dt": pd.to_datetime(dataset["time"].values),
            "prev_flag": np.asarray(dataset["time_segment"].values, dtype=np.int8),
            "value": values,
        })
    segment_map = {"current": 0, "forecast": 1}
    if time_segment not in (None, "all"):
        normalized_segment = str(time_segment).lower()
        if normalized_segment in segment_map:
            flag = segment_map[normalized_segment]
        else:
            try:
                flag = int(time_segment)
            except (TypeError, ValueError) as exc:
                raise ValueError("time_segment must be 'all', 'current', 'forecast', 0, or 1.") from exc
        if flag not in (0, 1):
            raise ValueError("time_segment must be 'all', 'current', 'forecast', 0, or 1.")
        frame = frame[frame["prev_flag"] == flag]
    if window is not None:
        frame = frame[
            (frame["dt"] >= pd.Timestamp(window.start_time))
            & (frame["dt"] < pd.Timestamp(window.forecast_end_exclusive))
        ]
    meta = MGB_VARIABLE_METADATA[code]
    frame["variable_code"] = code
    frame["display_name"] = meta["display_name"]
    frame["unit"] = meta["unit"]
    return frame.sort_values("dt").reset_index(drop=True)


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
    variable_code: str = "q",
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


# Clear aliases for callers that prefer noun-specific API names.
read_model_outputs = validate_model_outputs_netcdf
select_mgb_series = load_mgb_series
compute_mini_peak_summary = summarize_mini_peaks
compute_network_peak_summary = summarize_network_peaks
