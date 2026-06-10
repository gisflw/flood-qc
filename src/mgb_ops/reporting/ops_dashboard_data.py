from __future__ import annotations

import json
import re
import sqlite3
from datetime import datetime, timedelta
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd

from mgb_ops.common.paths import history_db_path, interim_dir, mgb_input_dir, mgb_output_dir, resolve_workspace_path
from mgb_ops.common.settings import load_settings
from mgb_ops.common.time_utils import resolve_reference_time
from mgb_ops.model.export_mgb_outputs import (
    build_export_window,
    compute_nt_current,
    infer_nt_from_binary,
    read_mini_ids,
    read_nc_from_parhig,
    read_time_settings_from_parhig,
)


STATE_PRIORITY = {"approved": 0, "curated": 1, "raw": 2}
ACCUM_RASTER_PATTERN = re.compile(r"^accum_(\d+)h\.tif$", re.IGNORECASE)
LEGACY_RIVERS_GEOJSON_PATH = resolve_workspace_path("data/legacy/app_layers/rios_mini.geojson")
DEFAULT_MGB_INPUT_DIR = mgb_input_dir()
DEFAULT_MGB_OUTPUT_DIR = mgb_output_dir()
DEFAULT_MGB_PARHIG_PATH = DEFAULT_MGB_INPUT_DIR / "PARHIG.hig"
DEFAULT_MGB_MINI_GTP_PATH = DEFAULT_MGB_INPUT_DIR / "MINI.gtp"
MGB_VARIABLE_METADATA = {
    "q": {
        "display_name": "QTUDO",
        "unit": "m3/s",
        "source_filename": "QTUDO_Inercial_Atual.MGB",
    },
    "y": {
        "display_name": "YTUDO",
        "unit": "m",
        "source_filename": "YTUDO.MGB",
    },
}


def _ensure_datetime_series(series: pd.Series) -> pd.Series:
    return pd.to_datetime(series, errors="coerce")


def _connect(database_path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(database_path)
    connection.row_factory = sqlite3.Row
    return connection


def select_preferred_series_rows(series_df: pd.DataFrame) -> pd.DataFrame:
    if series_df.empty:
        return series_df.copy()

    ranked = series_df.copy()
    ranked["state_rank"] = ranked["state"].map(STATE_PRIORITY).fillna(len(STATE_PRIORITY)).astype(int)
    if "created_at" not in ranked.columns:
        ranked["created_at"] = ""
    else:
        ranked["created_at"] = ranked["created_at"].fillna("")

    ranked = ranked.sort_values(
        ["station_uid", "variable_code", "state_rank", "created_at"],
        ascending=[True, True, True, False],
    )
    preferred = ranked.drop_duplicates(subset=["station_uid", "variable_code"], keep="first")
    return preferred.drop(columns=["state_rank"], errors="ignore").reset_index(drop=True)


def derive_station_kind(variable_codes: Iterable[str]) -> str:
    codes = {str(code).strip().lower() for code in variable_codes if str(code).strip()}
    has_rain = "rain" in codes
    has_stage = bool({"level", "flow"} & codes)

    if has_rain and has_stage:
        return "mixed"
    if has_rain:
        return "rain"
    if has_stage:
        return "level"
    return "no_data"


def summarize_station_status(values_df: pd.DataFrame, *, days: int) -> dict[str, object]:
    if values_df.empty:
        return {
            "status": "no_data",
            "status_reason": f"sem registros nos ultimos {days} dias",
            "rows_recent": 0,
        }

    non_null_count = int(values_df["value"].notna().sum())
    if non_null_count == 0:
        return {
            "status": "data_issue",
            "status_reason": "somente valores nulos no periodo",
            "rows_recent": int(len(values_df)),
        }

    return {
        "status": "ok",
        "status_reason": "",
        "rows_recent": int(len(values_df)),
    }


def compute_rain_summary(rain_df: pd.DataFrame) -> dict[str, float]:
    if rain_df.empty:
        return {
            "rain_mean_mm_h": float("nan"),
            "rain_acc_24h_mm": float("nan"),
            "rain_p90_mm_h": float("nan"),
        }

    ordered = rain_df.sort_values("datetime").copy()
    ordered["value"] = pd.to_numeric(ordered["value"], errors="coerce")
    valid = ordered.dropna(subset=["value"])
    if valid.empty:
        return {
            "rain_mean_mm_h": float("nan"),
            "rain_acc_24h_mm": float("nan"),
            "rain_p90_mm_h": float("nan"),
        }

    latest_time = valid["datetime"].max()
    rain_24h = valid.loc[valid["datetime"] >= latest_time - timedelta(hours=24), "value"].sum(min_count=1)
    return {
        "rain_mean_mm_h": float(valid["value"].mean()),
        "rain_acc_24h_mm": float(rain_24h) if pd.notna(rain_24h) else float("nan"),
        "rain_p90_mm_h": float(valid["value"].quantile(0.9)),
    }


def load_station_catalog(
    database_path: Path | None = None,
    *,
    days: int = 30,
    now: datetime | None = None,
) -> pd.DataFrame:
    history_path = database_path or history_db_path()
    cutoff = (now or datetime.utcnow()) - timedelta(days=days)
    cutoff_text = cutoff.strftime("%Y-%m-%d %H:%M:%S")

    with _connect(history_path) as connection:
        stations = pd.read_sql_query(
            """
            SELECT
                station_uid,
                station_code,
                provider_code,
                station_name,
                latitude AS lat,
                longitude AS lon
            FROM station
            WHERE latitude IS NOT NULL
              AND longitude IS NOT NULL
            ORDER BY provider_code, station_code
            """,
            connection,
        )
        series = pd.read_sql_query(
            """
            SELECT
                series_id,
                station_uid,
                variable_code,
                state,
                created_at
            FROM observed_series
            """,
            connection,
        )
        recent_values = pd.read_sql_query(
            """
            SELECT
                os.series_id,
                os.station_uid,
                os.variable_code,
                ov.observed_at,
                ov.value
            FROM observed_series os
            JOIN observed_value ov ON ov.series_id = os.series_id
            WHERE ov.observed_at >= ?
            """,
            connection,
            params=(cutoff_text,),
        )

    if stations.empty:
        return pd.DataFrame(
            columns=[
                "station_uid",
                "station_code",
                "provider_code",
                "station_name",
                "lat",
                "lon",
                "kind",
                "status",
                "status_reason",
            ]
        )

    preferred_series = select_preferred_series_rows(series)
    preferred_ids = set(preferred_series["series_id"].tolist())
    recent_values = recent_values[recent_values["series_id"].isin(preferred_ids)].copy()
    recent_values["datetime"] = _ensure_datetime_series(recent_values["observed_at"])
    recent_values["value"] = pd.to_numeric(recent_values["value"], errors="coerce")
    recent_values = recent_values.dropna(subset=["datetime"])

    coverage = (
        preferred_series.groupby("station_uid")["variable_code"]
        .agg(list)
        .reset_index(name="variable_codes")
    )
    coverage["kind"] = coverage["variable_codes"].apply(derive_station_kind)

    metrics_rows: list[dict[str, object]] = []
    for station_uid, station_values in recent_values.groupby("station_uid", sort=False):
        status_summary = summarize_station_status(station_values, days=days)
        rain_summary = compute_rain_summary(station_values[station_values["variable_code"] == "rain"])
        metrics_rows.append(
            {
                "station_uid": int(station_uid),
                **status_summary,
                **rain_summary,
            }
        )

    metrics = pd.DataFrame(
        metrics_rows,
        columns=["station_uid", "status", "status_reason", "rows_recent", "rain_mean_mm_h", "rain_acc_24h_mm", "rain_p90_mm_h"],
    )
    merged = stations.merge(coverage[["station_uid", "kind"]], on="station_uid", how="left")
    merged = merged.merge(metrics, on="station_uid", how="left")
    merged["kind"] = merged["kind"].fillna("no_data")
    merged["status"] = merged["status"].fillna("no_data")
    merged["status_reason"] = merged["status_reason"].fillna(f"sem registros nos ultimos {days} dias")
    merged["rows_recent"] = merged["rows_recent"].fillna(0).astype(int)

    for column in ("rain_mean_mm_h", "rain_acc_24h_mm", "rain_p90_mm_h"):
        if column not in merged:
            merged[column] = np.nan

    return merged.sort_values(["provider_code", "station_code"]).reset_index(drop=True)


def load_observed_series(
    station_uid: int,
    database_path: Path | None = None,
    *,
    days: int = 30,
    now: datetime | None = None,
) -> pd.DataFrame:
    history_path = database_path or history_db_path()
    cutoff = (now or datetime.utcnow()) - timedelta(days=days)

    with _connect(history_path) as connection:
        series = pd.read_sql_query(
            """
            SELECT
                series_id,
                station_uid,
                variable_code,
                state,
                created_at
            FROM observed_series
            WHERE station_uid = ?
            """,
            connection,
            params=(int(station_uid),),
        )

        if series.empty:
            return pd.DataFrame(columns=["datetime", "variable_code", "value"])

        preferred = select_preferred_series_rows(series)
        placeholders = ",".join("?" for _ in preferred["series_id"])
        values = pd.read_sql_query(
            f"""
            SELECT
                os.variable_code,
                ov.observed_at AS datetime,
                ov.value
            FROM observed_value ov
            JOIN observed_series os ON os.series_id = ov.series_id
            WHERE ov.series_id IN ({placeholders})
              AND ov.observed_at >= ?
            ORDER BY ov.observed_at
            """,
            connection,
            params=(*preferred["series_id"].tolist(), cutoff.strftime("%Y-%m-%d %H:%M:%S")),
        )

    values["datetime"] = _ensure_datetime_series(values["datetime"])
    values["value"] = pd.to_numeric(values["value"], errors="coerce")
    values = values.dropna(subset=["datetime"]).sort_values(["datetime", "variable_code"])
    return values.reset_index(drop=True)


def compute_observed_metrics(observed_df: pd.DataFrame) -> dict[str, object]:
    if observed_df.empty:
        return {
            "latest_time": None,
            "rain_12h": float("nan"),
            "rain_24h": float("nan"),
            "rain_72h": float("nan"),
            "level_current": float("nan"),
            "flow_current": float("nan"),
        }

    ordered = observed_df.copy()
    ordered["datetime"] = _ensure_datetime_series(ordered["datetime"])
    ordered["value"] = pd.to_numeric(ordered["value"], errors="coerce")
    ordered = ordered.dropna(subset=["datetime"]).sort_values("datetime")
    if ordered.empty:
        return {
            "latest_time": None,
            "rain_12h": float("nan"),
            "rain_24h": float("nan"),
            "rain_72h": float("nan"),
            "level_current": float("nan"),
            "flow_current": float("nan"),
        }

    latest_time = ordered["datetime"].max()
    rain_df = ordered[ordered["variable_code"] == "rain"].dropna(subset=["value"])

    def accumulate(hours: int) -> float:
        if rain_df.empty:
            return float("nan")
        value = rain_df.loc[rain_df["datetime"] >= latest_time - timedelta(hours=hours), "value"].sum(min_count=1)
        return float(value) if pd.notna(value) else float("nan")

    def latest_variable(variable_code: str) -> float:
        subset = ordered[(ordered["variable_code"] == variable_code) & ordered["value"].notna()]
        if subset.empty:
            return float("nan")
        return float(subset.sort_values("datetime").iloc[-1]["value"])

    return {
        "latest_time": latest_time,
        "rain_12h": accumulate(12),
        "rain_24h": accumulate(24),
        "rain_72h": accumulate(72),
        "level_current": latest_variable("level"),
        "flow_current": latest_variable("flow"),
    }


def _canonical_variable_code(variable_code: str) -> str:
    return str(variable_code).strip().lower()


def _resolve_mgb_output_path(variable_code: str) -> Path:
    canonical = _canonical_variable_code(variable_code)
    metadata = MGB_VARIABLE_METADATA.get(canonical)
    if metadata is None:
        raise ValueError(f"Unsupported MGB variable_code={variable_code!r}. Expected one of {sorted(MGB_VARIABLE_METADATA)}.")
    return DEFAULT_MGB_OUTPUT_DIR / str(metadata["source_filename"])


def _build_mgb_mini_index(*, mini_gtp_path: Path, nc: int) -> dict[int, int]:
    mini_ids = read_mini_ids(mini_gtp_path, nc=nc)
    return {int(mini_id): row_index for row_index, mini_id in enumerate(mini_ids)}


def _require_mgb_paths(variable_code: str) -> tuple[Path, Path, Path]:
    parhig_path = DEFAULT_MGB_PARHIG_PATH
    mini_gtp_path = DEFAULT_MGB_MINI_GTP_PATH
    output_path = _resolve_mgb_output_path(variable_code)
    missing = [path for path in (parhig_path, mini_gtp_path, output_path) if not path.exists()]
    if missing:
        missing_text = ", ".join(str(path) for path in missing)
        raise FileNotFoundError(f"Required MGB dashboard inputs were not found: {missing_text}")
    return parhig_path, mini_gtp_path, output_path


def _load_mgb_runtime_settings() -> tuple[datetime, int, int]:
    settings = load_settings()
    reference_time = resolve_reference_time(settings["run"]["reference_time"])
    mgb_settings = settings["mgb"]
    output_days_before = int(mgb_settings["output_days_before"])
    forecast_horizon_days = int(mgb_settings["forecast_horizon_days"])
    return reference_time, output_days_before, forecast_horizon_days


def load_model_metadata(database_path: Path | None = None) -> dict[str, object]:
    """Load MGB output metadata directly from canonical binaries."""
    del database_path
    available_paths = [DEFAULT_MGB_OUTPUT_DIR / str(meta["source_filename"]) for meta in MGB_VARIABLE_METADATA.values()]
    if not DEFAULT_MGB_PARHIG_PATH.exists() or not DEFAULT_MGB_MINI_GTP_PATH.exists() or not any(
        path.exists() for path in available_paths
    ):
        return {}

    nc = read_nc_from_parhig(DEFAULT_MGB_PARHIG_PATH)
    start_time, dt_seconds = read_time_settings_from_parhig(DEFAULT_MGB_PARHIG_PATH)
    nt_values = {
        variable_code: infer_nt_from_binary(path, nc=nc)
        for variable_code, meta in MGB_VARIABLE_METADATA.items()
        for path in [DEFAULT_MGB_OUTPUT_DIR / str(meta["source_filename"])]
        if path.exists()
    }
    nt_set = set(nt_values.values())
    if len(nt_set) != 1:
        raise ValueError(f"Inconsistent NT across MGB binary outputs: {nt_values}")

    nt_total = nt_set.pop()
    reference_time, output_days_before, forecast_horizon_days = _load_mgb_runtime_settings()
    nt_current, nt_forecast = compute_nt_current(
        start_time=start_time,
        dt_seconds=dt_seconds,
        reference_time=reference_time,
        nt_total=nt_total,
    )
    export_window = build_export_window(
        reference_time,
        output_days_before=output_days_before,
        forecast_horizon_days=forecast_horizon_days,
    )
    return {
        "reference_time": pd.Timestamp(reference_time),
        "reference_date": pd.Timestamp(export_window.reference_date),
        "window_start": pd.Timestamp(export_window.window_start),
        "window_end_exclusive": pd.Timestamp(export_window.window_end_exclusive),
        "dt_seconds": dt_seconds,
        "nc": nc,
        "nt_current": nt_current,
        "nt_forecast": nt_forecast,
    }


def list_model_variables(database_path: Path | None = None) -> pd.DataFrame:
    """List MGB variables supported by the dashboard."""
    del database_path
    return pd.DataFrame(
        [
            {
                "variable_code": variable_code,
                "display_name": str(metadata["display_name"]),
                "unit": str(metadata["unit"]),
            }
            for variable_code, metadata in sorted(MGB_VARIABLE_METADATA.items())
        ]
    )


def load_mgb_series(
    mini_id: int,
    variable_code: str,
    database_path: Path | None = None,
    *,
    days_window: int = 30,
) -> pd.DataFrame:
    """Read an MGB series directly from canonical runner binaries."""
    del database_path
    canonical = _canonical_variable_code(variable_code)
    metadata = MGB_VARIABLE_METADATA.get(canonical)
    if metadata is None:
        return pd.DataFrame(columns=["dt", "prev_flag", "value", "variable_code", "display_name", "unit"])

    parhig_path, mini_gtp_path, output_path = _require_mgb_paths(canonical)
    nc = read_nc_from_parhig(parhig_path)
    start_time, dt_seconds = read_time_settings_from_parhig(parhig_path)
    row_lookup = _build_mgb_mini_index(mini_gtp_path=mini_gtp_path, nc=nc)
    row_index = row_lookup.get(int(mini_id))
    if row_index is None:
        raise ValueError(f"Mini {mini_id} was not found in {mini_gtp_path}.")

    nt_total = infer_nt_from_binary(output_path, nc=nc)
    reference_time, _, _ = _load_mgb_runtime_settings()
    nt_current, nt_forecast = compute_nt_current(
        start_time=start_time,
        dt_seconds=dt_seconds,
        reference_time=reference_time,
        nt_total=nt_total,
    )

    matrix = np.memmap(output_path, dtype=np.float32, mode="r", shape=(nc, nt_total))
    try:
        values = np.asarray(matrix[row_index, :], dtype=np.float32)
    finally:
        del matrix

    dt_index = pd.date_range(start=start_time, periods=nt_total, freq=pd.to_timedelta(dt_seconds, unit="s"))
    prev_flags = np.concatenate(
        [
            np.zeros(nt_current, dtype=np.int8),
            np.ones(nt_forecast, dtype=np.int8),
        ]
    )
    df = pd.DataFrame(
        {
            "dt": dt_index,
            "prev_flag": prev_flags,
            "value": values,
            "variable_code": canonical,
            "display_name": str(metadata["display_name"]),
            "unit": str(metadata["unit"]),
        }
    )

    current_df = df[df["prev_flag"] == 0].copy()
    forecast_df = df[df["prev_flag"] == 1].copy()

    if not current_df.empty and days_window > 0:
        cutoff = current_df["dt"].max() - timedelta(days=days_window)
        current_df = current_df[current_df["dt"] >= cutoff]

    out = pd.concat([current_df, forecast_df], ignore_index=True)
    return out.sort_values("dt").reset_index(drop=True)


def list_accumulation_rasters(base_dir: Path | None = None) -> list[dict[str, object]]:
    search_dir = base_dir or interim_dir()
    rasters: list[dict[str, object]] = []
    for path in sorted(search_dir.glob("accum_*h.tif")):
        match = ACCUM_RASTER_PATTERN.match(path.name)
        if not match:
            continue
        horizon_hours = int(match.group(1))
        rasters.append(
            {
                "name": path.stem,
                "path": path,
                "horizon_hours": horizon_hours,
                "horizon_label": f"{horizon_hours}h",
            }
        )
    return sorted(rasters, key=lambda item: int(item["horizon_hours"]))


def load_raster_data(path: Path, *, max_size: int = 600) -> tuple[np.ndarray, tuple[float, float, float, float]]:
    try:
        import rasterio
        from rasterio.enums import Resampling
    except ModuleNotFoundError as exc:
        raise RuntimeError(
            "Leitura de raster requer rasterio. Instale as dependencias de UI/geo antes de abrir o dashboard."
        ) from exc

    with rasterio.open(path) as src:
        scale = min(max_size / src.height, max_size / src.width, 1.0)
        out_h = max(1, int(src.height * scale))
        out_w = max(1, int(src.width * scale))
        data = src.read(
            1,
            out_shape=(out_h, out_w),
            resampling=Resampling.bilinear,
        ).astype("float32")
        data[data <= 0] = np.nan
        data[data <= -1e20] = np.nan
        bounds = src.bounds
    return data, (bounds.left, bounds.bottom, bounds.right, bounds.top)


def load_rivers_layer_geojson(path: Path | None = None) -> dict | None:
    target_path = path or LEGACY_RIVERS_GEOJSON_PATH
    if not target_path.exists():
        return None
    payload = json.loads(target_path.read_text(encoding="utf-8"))
    if payload.get("type") != "FeatureCollection":
        return None

    for feature in payload.get("features", []):
        props = feature.setdefault("properties", {})
        mini_raw = props.get("mini_id")
        try:
            mini_id = int(mini_raw)
        except (TypeError, ValueError):
            props["click_id"] = "MINI|"
            continue
        props["mini_id"] = mini_id
        props["click_id"] = f"MINI|{mini_id}"
    return payload
