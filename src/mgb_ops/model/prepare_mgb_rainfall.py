from __future__ import annotations

import argparse
import logging
import sqlite3
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

import numpy as np
import pandas as pd

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from mgb_ops.common.paths import history_db_path, logs_dir as default_logs_dir, mgb_input_dir, resolve_workspace_path
from mgb_ops.common.settings import load_settings
from mgb_ops.common.time_utils import TIMEZONE, resolve_reference_time
from mgb_ops.ingest.forecast_grid import ECMWF_ASSET_KIND, TpGribMessage, read_tp_grib_messages
from mgb_ops.model.export_mgb_outputs import read_nc_from_parhig
from mgb_ops.model.prepare_mgb_meta import (
    DEFAULT_PARHIG,
    build_mgb_window,
    read_time_settings_from_parhig,
)

DEFAULT_HISTORY_DB = history_db_path()
DEFAULT_MINI_GTP = mgb_input_dir() / "MINI.gtp"
DEFAULT_OUTPUT_PATH = mgb_input_dir() / "chuvabin.hig"
DEFAULT_CHUNK_HOURS = 720
LOGGER_NAME = "floodqc.model.prepare_mgb_rainfall"
STATE_PRIORITY = {"approved": 0, "curated": 1, "raw": 2}


@dataclass(frozen=True, slots=True)
class RainfallPreparationSummary:
    output_path: Path
    history_db_path: Path
    start_time: datetime
    end_time_exclusive: datetime
    nt: int
    nc: int
    station_count: int
    nearest_stations: int
    power: float
    used_hourly_normalization: bool
    forecast_hours: int


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger(LOGGER_NAME)
    logger.setLevel(logging.INFO)
    for handler in logger.handlers[:]:
        handler.close()
        logger.removeHandler(handler)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s | %(levelname)s | %(message)s")

    file_handler = logging.FileHandler(log_file, mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    return logger


def _connect_history_read_only(database_path: Path) -> sqlite3.Connection:
    connection = sqlite3.connect(f"{database_path.resolve().as_uri()}?mode=ro", uri=True, timeout=30.0)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA busy_timeout = 30000")
    connection.execute("PRAGMA query_only = ON")
    return connection


def _select_preferred_series_rows(series_df: pd.DataFrame) -> pd.DataFrame:
    if series_df.empty:
        return series_df.copy()

    ranked = series_df.copy()
    ranked["state_rank"] = ranked["state"].map(STATE_PRIORITY).fillna(len(STATE_PRIORITY)).astype(int)
    ranked["created_at"] = ranked["created_at"].fillna("")
    ranked = ranked.sort_values(["station_uid", "state_rank", "created_at"], ascending=[True, True, False])
    preferred = ranked.drop_duplicates(subset=["station_uid"], keep="first")
    return preferred.drop(columns=["state_rank"], errors="ignore").reset_index(drop=True)


def load_preferred_rain_stations(connection: sqlite3.Connection) -> pd.DataFrame:
    series = pd.read_sql_query(
        """
        SELECT
            os.series_id,
            os.station_uid,
            os.state,
            os.created_at,
            st.latitude AS lat,
            st.longitude AS lon
        FROM observed_series os
        JOIN station st ON st.station_uid = os.station_uid
        WHERE os.variable_code = 'rain'
          AND st.latitude IS NOT NULL
          AND st.longitude IS NOT NULL
        """,
        connection,
    )
    preferred = _select_preferred_series_rows(series)
    preferred["lat"] = pd.to_numeric(preferred["lat"], errors="coerce")
    preferred["lon"] = pd.to_numeric(preferred["lon"], errors="coerce")
    return preferred.dropna(subset=["lat", "lon"]).sort_values("station_uid").reset_index(drop=True)


def load_rain_values(
    connection: sqlite3.Connection,
    preferred_stations: pd.DataFrame,
    *,
    query_start: datetime,
    query_end_exclusive: datetime,
    batch_size: int = 400,
) -> pd.DataFrame:
    if preferred_stations.empty:
        return pd.DataFrame(columns=["station_uid", "observed_at", "value"])

    series_ids = preferred_stations["series_id"].astype(str).tolist()
    frames: list[pd.DataFrame] = []
    start_text = query_start.strftime("%Y-%m-%d %H:%M")
    end_text = query_end_exclusive.strftime("%Y-%m-%d %H:%M")

    for start_idx in range(0, len(series_ids), batch_size):
        chunk_ids = series_ids[start_idx : start_idx + batch_size]
        placeholders = ",".join("?" for _ in chunk_ids)
        frames.append(
            pd.read_sql_query(
                f"""
                SELECT
                    os.station_uid,
                    ov.observed_at,
                    ov.value
                FROM observed_value ov
                JOIN observed_series os ON os.series_id = ov.series_id
                WHERE ov.series_id IN ({placeholders})
                  AND ov.observed_at >= ?
                  AND ov.observed_at < ?
                ORDER BY ov.observed_at
                """,
                connection,
                params=(*chunk_ids, start_text, end_text),
            )
        )

    return pd.concat(frames, ignore_index=True) if frames else pd.DataFrame(columns=["station_uid", "observed_at", "value"])


def temporarily_normalize_rain_to_hourly(values_df: pd.DataFrame) -> tuple[pd.DataFrame, bool]:
    if values_df.empty:
        return pd.DataFrame(columns=["station_uid", "observed_at", "value"]), False

    frame = values_df.copy()
    frame["station_uid"] = pd.to_numeric(frame["station_uid"], errors="coerce")
    frame["observed_at"] = pd.to_datetime(frame["observed_at"], errors="coerce")
    frame["value"] = pd.to_numeric(frame["value"], errors="coerce")
    frame = frame.dropna(subset=["station_uid", "observed_at", "value"]).copy()
    if frame.empty:
        return pd.DataFrame(columns=["station_uid", "observed_at", "value"]), False

    frame["station_uid"] = frame["station_uid"].astype(np.int64)
    used_hourly_normalization = bool(
        (frame["observed_at"].dt.minute != 0).any()
        or (frame["observed_at"].dt.second != 0).any()
        or (frame["observed_at"].dt.microsecond != 0).any()
    )

    frame["observed_at"] = frame["observed_at"].dt.ceil("h")
    if not used_hourly_normalization:
        used_hourly_normalization = bool(frame.duplicated(subset=["station_uid", "observed_at"]).any())

    hourly = (
        frame.groupby(["station_uid", "observed_at"], as_index=False, sort=True)["value"]
        .sum()
        .sort_values(["station_uid", "observed_at"])
        .reset_index(drop=True)
    )
    return hourly, used_hourly_normalization


def read_mini_centroids(mini_gtp_path: Path, *, nc: int) -> pd.DataFrame:
    header: list[str] | None = None
    rows: list[tuple[int, float, float]] = []

    with mini_gtp_path.open("r", encoding="latin-1") as handle:
        for raw_line in handle:
            stripped = raw_line.strip()
            if not stripped:
                continue
            parts = stripped.split()
            if header is None:
                header = parts
                required = {"Mini", "Xcen", "Ycen"}
                if not required.issubset(header):
                    raise ValueError(f"MINI.gtp missing required columns {sorted(required)}: {mini_gtp_path}")
                continue

            mini_idx = header.index("Mini")
            xcen_idx = header.index("Xcen")
            ycen_idx = header.index("Ycen")
            if len(parts) <= max(mini_idx, xcen_idx, ycen_idx):
                raise ValueError(f"Invalid MINI.gtp row: {raw_line.rstrip()}")
            rows.append(
                (
                    int(float(parts[mini_idx].replace(",", "."))),
                    float(parts[xcen_idx].replace(",", ".")),
                    float(parts[ycen_idx].replace(",", ".")),
                )
            )
            if len(rows) == nc:
                break

    if len(rows) < nc:
        raise ValueError(f"MINI.gtp has {len(rows)} rows, smaller than NC={nc}")

    mini_df = pd.DataFrame(rows, columns=["mini_id", "lon", "lat"])
    duplicated = mini_df.loc[mini_df["mini_id"].duplicated(), "mini_id"].unique().tolist()
    if duplicated:
        raise ValueError(f"MINI.gtp has duplicated Mini ids (sample: {duplicated[:5]})")
    return mini_df.sort_values("mini_id").reset_index(drop=True)


def build_hourly_station_matrix(
    preferred_stations: pd.DataFrame,
    hourly_values: pd.DataFrame,
    *,
    time_index: pd.DatetimeIndex,
) -> tuple[pd.DataFrame, np.ndarray]:
    if hourly_values.empty:
        raise ValueError("No rainfall observations found in the requested simulation window.")

    pivoted = hourly_values.pivot_table(index="observed_at", columns="station_uid", values="value", aggfunc="sum").reindex(time_index)
    available_station_ids = [
        int(station_uid)
        for station_uid in preferred_stations["station_uid"].tolist()
        if station_uid in pivoted.columns and pivoted[station_uid].notna().any()
    ]
    if not available_station_ids:
        raise ValueError("No rain stations with valid values were found for the requested simulation window.")

    station_meta = preferred_stations.set_index("station_uid").loc[available_station_ids].reset_index()
    station_matrix = (
        pivoted.reindex(columns=available_station_ids)
        .fillna(0.0)
        .transpose()
        .to_numpy(dtype=np.float64, copy=False)
    )
    return station_meta, station_matrix


def _resolve_workspace_asset_path(raw_path: str, *, workspace: str | Path | None = None) -> Path:
    return resolve_workspace_path(raw_path, workspace)


def load_latest_ecmwf_asset_path(
    connection: sqlite3.Connection,
    *,
    reference_time: datetime,
    workspace: str | Path | None = None,
) -> Path:
    row = connection.execute(
        """
        SELECT relative_path
        FROM asset
        WHERE provider_code = 'ecmwf'
          AND asset_kind = ?
          AND valid_from IS NOT NULL
          AND valid_to IS NOT NULL
          AND valid_from <= ?
          AND valid_to >= ?
        ORDER BY valid_from DESC, created_at DESC
        LIMIT 1
        """,
        (
            ECMWF_ASSET_KIND,
            reference_time.isoformat(timespec="seconds"),
            reference_time.isoformat(timespec="seconds"),
        ),
    ).fetchone()
    if row is None:
        raise FileNotFoundError(
            "No ECMWF forecast asset was found in history for the requested forecast window. "
            "Run `mgb-ops ingest forecast-grid` first."
        )

    asset_path = _resolve_workspace_asset_path(str(row["relative_path"]), workspace=workspace)
    if not asset_path.exists():
        raise FileNotFoundError(f"ECMWF asset registered in history does not exist on disk: {asset_path}")
    return asset_path


def extend_station_matrix_with_forecast(
    station_matrix: np.ndarray,
    *,
    total_nt: int,
    forecast_nt: int,
    use_forecast_data: bool,
) -> np.ndarray:
    if forecast_nt < 0:
        raise ValueError("forecast_nt must be >= 0.")
    if total_nt < forecast_nt:
        raise ValueError("total_nt must be >= forecast_nt.")
    if station_matrix.shape[1] != total_nt:
        raise ValueError(f"station_matrix shape mismatch: expected {total_nt} columns, found {station_matrix.shape[1]}.")
    if forecast_nt == 0:
        return station_matrix
    if use_forecast_data:
        raise NotImplementedError("Forecast rainfall ingestion is not implemented yet. Set mgb.use_forecast_data=false.")

    observed_nt = total_nt - forecast_nt
    extended = np.array(station_matrix, dtype=np.float64, copy=True)
    extended[:, observed_nt:] = 0.0
    return extended


def interpolate_source_matrix(
    source_matrix: np.ndarray,
    *,
    nearest_idx: np.ndarray,
    weights: np.ndarray,
    chunk_hours: int,
) -> np.ndarray:
    if chunk_hours < 1:
        raise ValueError("chunk_hours must be >= 1.")
    total_hours = source_matrix.shape[1]
    out = np.empty((nearest_idx.shape[0], total_hours), dtype=np.float64)
    for start_idx in range(0, total_hours, chunk_hours):
        end_idx = min(start_idx + chunk_hours, total_hours)
        out[:, start_idx:end_idx] = interpolate_station_chunk(
            source_matrix[:, start_idx:end_idx],
            nearest_idx=nearest_idx,
            weights=weights,
        )
    return out


def build_idw_neighbors(
    mini_df: pd.DataFrame,
    station_df: pd.DataFrame,
    *,
    nearest_stations: int,
    power: float,
) -> tuple[np.ndarray, np.ndarray]:
    if station_df.empty:
        raise ValueError("At least one rain station is required for interpolation.")
    k = min(int(nearest_stations), len(station_df))
    if k < 1:
        raise ValueError("nearest_stations must be >= 1.")

    mini_lat = mini_df["lat"].to_numpy(dtype=np.float64)
    mini_lon = mini_df["lon"].to_numpy(dtype=np.float64)
    station_lat = station_df["lat"].to_numpy(dtype=np.float64)
    station_lon = station_df["lon"].to_numpy(dtype=np.float64)

    distances = np.hypot(mini_lat[:, None] - station_lat[None, :], mini_lon[:, None] - station_lon[None, :])
    nearest_idx = np.argsort(distances, axis=1)[:, :k]
    nearest_dist = np.take_along_axis(distances, nearest_idx, axis=1)
    safe_dist = np.where(nearest_dist == 0.0, 1e-12, nearest_dist)
    weights = 1.0 / np.power(safe_dist, float(power))
    return nearest_idx.astype(np.int32, copy=False), weights.astype(np.float64, copy=False)


def build_grid_idw_neighbors(
    mini_df: pd.DataFrame,
    *,
    latitudes: np.ndarray,
    longitudes: np.ndarray,
    nearest_points: int,
    power: float,
) -> tuple[np.ndarray, np.ndarray]:
    if latitudes.size < 1 or longitudes.size < 1:
        raise ValueError("Forecast grid must contain at least one latitude and one longitude.")

    lon_grid, lat_grid = np.meshgrid(longitudes, latitudes)
    point_df = pd.DataFrame(
        {
            "lat": lat_grid.reshape(-1),
            "lon": lon_grid.reshape(-1),
        }
    )
    return build_idw_neighbors(
        mini_df,
        point_df,
        nearest_stations=min(int(nearest_points), len(point_df)),
        power=power,
    )


def interpolate_station_chunk(
    station_chunk: np.ndarray,
    *,
    nearest_idx: np.ndarray,
    weights: np.ndarray,
) -> np.ndarray:
    n_mini, k = nearest_idx.shape
    chunk_size = station_chunk.shape[1]

    gathered = station_chunk[nearest_idx.reshape(-1), :].reshape(n_mini, k, chunk_size)
    valid = np.isfinite(gathered)
    weighted_values = np.where(valid, gathered * weights[:, :, None], 0.0)
    weight_sum = np.where(valid, weights[:, :, None], 0.0).sum(axis=1)
    if np.any(weight_sum <= 0):
        missing_positions = int((weight_sum <= 0).sum())
        raise ValueError(f"Interpolation left {missing_positions} mini/hour positions without rainfall coverage.")

    return np.divide(weighted_values.sum(axis=1), weight_sum)


def write_mini_rainfall_atomic(
    output_path: Path,
    *,
    mini_matrix: np.ndarray,
    chunk_hours: int,
) -> None:
    if chunk_hours < 1:
        raise ValueError("chunk_hours must be >= 1.")

    output_path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = output_path.with_suffix(output_path.suffix + ".tmp")
    try:
        with temp_path.open("wb") as handle:
            total_hours = mini_matrix.shape[1]
            for start_idx in range(0, total_hours, chunk_hours):
                end_idx = min(start_idx + chunk_hours, total_hours)
                mini_matrix[:, start_idx:end_idx].astype(np.float32, copy=False).reshape(-1, order="F").tofile(handle)
        temp_path.replace(output_path)
    finally:
        if temp_path.exists():
            temp_path.unlink(missing_ok=True)


def build_hourly_forecast_grid_series(
    grib_path: Path,
    *,
    forecast_start_time: datetime,
    forecast_nt: int,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    messages = read_tp_grib_messages(grib_path)
    first_message = messages[0]
    local_messages: list[TpGribMessage] = [
        TpGribMessage(
            valid_time=message.valid_time.replace(tzinfo=timezone.utc).astimezone(TIMEZONE).replace(tzinfo=None),
            step_hours=message.step_hours,
            latitudes=message.latitudes,
            longitudes=message.longitudes,
            values_mm=message.values_mm,
        )
        for message in messages
    ]
    cycle_time = min(message.valid_time - timedelta(hours=message.step_hours) for message in local_messages)
    prev_valid_time = cycle_time
    prev_cumulative = np.zeros_like(first_message.values_mm, dtype=np.float64)
    latitudes = first_message.latitudes
    longitudes = first_message.longitudes
    hourly_lookup: dict[datetime, np.ndarray] = {}

    for message in local_messages:
        if message.values_mm.shape != prev_cumulative.shape:
            raise ValueError("ECMWF GRIB contains inconsistent grid shapes across messages.")
        if not np.allclose(message.latitudes, latitudes) or not np.allclose(message.longitudes, longitudes):
            raise ValueError("ECMWF GRIB contains inconsistent grid coordinates across messages.")

        delta_seconds = int((message.valid_time - prev_valid_time).total_seconds())
        if delta_seconds < 0 or delta_seconds % 3600 != 0:
            raise ValueError(
                "ECMWF GRIB valid times are not monotonic hourly multiples; cannot harmonize to MGB hourly input."
            )

        delta_hours = delta_seconds // 3600
        increment = message.values_mm - prev_cumulative
        increment = np.where(np.isfinite(increment), increment, np.nan)
        increment[increment < 0.0] = 0.0
        if delta_hours > 0:
            per_hour = increment / float(delta_hours)
            for hour_offset in range(delta_hours):
                hourly_lookup[prev_valid_time + timedelta(hours=hour_offset + 1)] = per_hour

        prev_valid_time = message.valid_time
        prev_cumulative = message.values_mm

    required_times = [forecast_start_time + timedelta(hours=offset) for offset in range(forecast_nt)]
    missing_times = [dt for dt in required_times if dt not in hourly_lookup]
    if missing_times:
        raise ValueError(
            "ECMWF GRIB does not cover the full requested forecast window. "
            f"First missing hour: {missing_times[0].isoformat(timespec='seconds')}"
        )

    hourly_grids = np.stack([hourly_lookup[dt] for dt in required_times], axis=0)
    return latitudes, longitudes, hourly_grids


def prepare_mgb_rainfall(
    *,
    history_db: Path = DEFAULT_HISTORY_DB,
    parhig_path: Path = DEFAULT_PARHIG,
    mini_gtp_path: Path = DEFAULT_MINI_GTP,
    output_path: Path = DEFAULT_OUTPUT_PATH,
    nearest_stations: int,
    power: float,
    chunk_hours: int = DEFAULT_CHUNK_HOURS,
    logs_dir: Path = default_logs_dir(),
    workspace: str | Path | None = None,
) -> RainfallPreparationSummary:
    if not history_db.exists():
        raise FileNotFoundError(f"History database not found: {history_db}")
    if not parhig_path.exists():
        raise FileNotFoundError(f"PARHIG not found: {parhig_path}")
    if not mini_gtp_path.exists():
        raise FileNotFoundError(f"MINI.gtp not found: {mini_gtp_path}")

    execution_id = build_execution_id()
    logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")
    start_time, nt, dt_seconds = read_time_settings_from_parhig(parhig_path)
    if dt_seconds != 3600:
        raise ValueError(f"Only hourly rainfall input is currently supported; PARHIG DT={dt_seconds}.")

    settings = load_settings(workspace=workspace, require_custom=False if workspace is not None else None)
    reference_time = resolve_reference_time(settings["run"]["reference_time"])
    mgb_settings = settings["mgb"]
    window = build_mgb_window(
        reference_time,
        input_days_before=int(mgb_settings["input_days_before"]),
        forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    )
    use_forecast_data = bool(mgb_settings["use_forecast_data"])
    if start_time != window.start_time or nt != window.nt:
        raise ValueError(
            "PARHIG timing does not match current settings. "
            f"Expected start_time={window.start_time.isoformat(timespec='seconds')} nt={window.nt}, "
            f"found start_time={start_time.isoformat(timespec='seconds')} nt={nt}."
        )

    end_time_exclusive = start_time + timedelta(seconds=nt * dt_seconds)
    nc = read_nc_from_parhig(parhig_path)
    mini_df = read_mini_centroids(mini_gtp_path, nc=nc)
    query_start = start_time - timedelta(hours=1)
    observed_end_exclusive = window.forecast_start_time

    logger.info(
        "rainfall_prepare_start history_db=%s parhig=%s mini_gtp=%s output=%s start_time=%s nt=%s nc=%s nearest=%s power=%s reference_time=%s forecast_start_time=%s forecast_nt=%s use_forecast_data=%s",
        history_db,
        parhig_path,
        mini_gtp_path,
        output_path,
        start_time.isoformat(timespec="seconds"),
        nt,
        nc,
        nearest_stations,
        power,
        window.reference_time.isoformat(timespec="seconds"),
        window.forecast_start_time.isoformat(timespec="seconds"),
        window.forecast_nt,
        use_forecast_data,
    )

    observed_hours = nt - window.forecast_nt
    if observed_hours < 1:
        raise ValueError(f"Invalid observed window length calculated from nt={nt} and forecast_nt={window.forecast_nt}.")

    with _connect_history_read_only(history_db) as connection:
        preferred_stations = load_preferred_rain_stations(connection)
        raw_values = load_rain_values(
            connection,
            preferred_stations,
            query_start=query_start,
            query_end_exclusive=observed_end_exclusive,
        )
        forecast_asset_path = (
            load_latest_ecmwf_asset_path(
                connection,
                reference_time=window.forecast_start_time,
                workspace=workspace,
            )
            if use_forecast_data and window.forecast_nt > 0
            else None
        )

    hourly_values, used_hourly_normalization = temporarily_normalize_rain_to_hourly(raw_values)
    observed_time_index = pd.date_range(start=start_time, periods=observed_hours, freq="h")
    station_meta, station_matrix = build_hourly_station_matrix(
        preferred_stations,
        hourly_values,
        time_index=observed_time_index,
    )
    observed_nearest_idx, observed_weights = build_idw_neighbors(
        mini_df,
        station_meta,
        nearest_stations=nearest_stations,
        power=power,
    )
    observed_mini_matrix = interpolate_source_matrix(
        station_matrix,
        nearest_idx=observed_nearest_idx,
        weights=observed_weights,
        chunk_hours=chunk_hours,
    )

    if use_forecast_data and window.forecast_nt > 0:
        assert forecast_asset_path is not None
        forecast_latitudes, forecast_longitudes, forecast_hourly_grids = build_hourly_forecast_grid_series(
            forecast_asset_path,
            forecast_start_time=window.forecast_start_time,
            forecast_nt=window.forecast_nt,
        )
        forecast_grid_matrix = forecast_hourly_grids.reshape(window.forecast_nt, -1).transpose()
        forecast_nearest_idx, forecast_weights = build_grid_idw_neighbors(
            mini_df,
            latitudes=forecast_latitudes,
            longitudes=forecast_longitudes,
            nearest_points=nearest_stations,
            power=power,
        )
        forecast_mini_matrix = interpolate_source_matrix(
            forecast_grid_matrix,
            nearest_idx=forecast_nearest_idx,
            weights=forecast_weights,
            chunk_hours=chunk_hours,
        )
    else:
        forecast_mini_matrix = np.zeros((nc, window.forecast_nt), dtype=np.float64)

    mini_matrix = np.concatenate([observed_mini_matrix, forecast_mini_matrix], axis=1)
    if mini_matrix.shape != (nc, nt):
        raise ValueError(f"Final mini rainfall matrix shape mismatch: expected {(nc, nt)}, found {mini_matrix.shape}.")

    write_mini_rainfall_atomic(
        output_path,
        mini_matrix=mini_matrix,
        chunk_hours=chunk_hours,
    )

    logger.info(
        "rainfall_prepare_done output=%s station_count=%s nt=%s nc=%s forecast_nt=%s used_hourly_normalization=%s",
        output_path,
        len(station_meta),
        nt,
        nc,
        window.forecast_nt,
        used_hourly_normalization,
    )
    return RainfallPreparationSummary(
        output_path=output_path,
        history_db_path=history_db,
        start_time=start_time,
        end_time_exclusive=end_time_exclusive,
        nt=nt,
        nc=nc,
        station_count=len(station_meta),
        nearest_stations=min(int(nearest_stations), len(station_meta)),
        power=float(power),
        used_hourly_normalization=used_hourly_normalization,
        forecast_hours=window.forecast_nt,
    )


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Interpolate observed rainfall to MGB minis and write chuvabin.hig.")
    parser.add_argument("--history-db", type=Path, default=DEFAULT_HISTORY_DB, help="SQLite history database.")
    parser.add_argument("--parhig", type=Path, default=DEFAULT_PARHIG, help="PARHIG.hig file.")
    parser.add_argument("--mini-gtp", type=Path, default=DEFAULT_MINI_GTP, help="MINI.gtp file.")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT_PATH, help="Binary chuvabin.hig file.")
    parser.add_argument("--chunk-hours", type=int, default=DEFAULT_CHUNK_HOURS, help="Hours per interpolation/write chunk.")
    return parser


def main() -> int:
    args = build_parser().parse_args()
    settings = load_settings()
    rainfall_settings = settings["rainfall_interpolation"]
    summary = prepare_mgb_rainfall(
        history_db=args.history_db,
        parhig_path=args.parhig,
        mini_gtp_path=args.mini_gtp,
        output_path=args.output,
        nearest_stations=int(rainfall_settings["nearest_stations"]),
        power=float(rainfall_settings["power"]),
        chunk_hours=int(args.chunk_hours),
    )
    print(
        "chuvabin_ready "
        f"output={summary.output_path} "
        f"station_count={summary.station_count} "
        f"nt={summary.nt} "
        f"nc={summary.nc} "
        f"used_hourly_normalization={summary.used_hourly_normalization}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
