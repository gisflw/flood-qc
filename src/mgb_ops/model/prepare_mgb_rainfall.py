from __future__ import annotations

import logging
import sqlite3
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.analysis.spatial import (
    build_grid_idw_neighbors,
    build_idw_neighbors,
    interpolate_station_chunk,
)
from mgb_ops.common.time_utils import TIMEZONE, build_horizon_window, validate_timestep_hours
from mgb_ops.adapters.forecast_ecmwf import (
    ECMWF_FORECAST_PRODUCT,
    ForecastProductConfig,
    build_asset_id,
    build_ecmwf_cycle,
)
from mgb_ops.model.export_mgb_outputs import read_nc_from_parhig
from mgb_ops.model.forecast_grid import load_forecast_precipitation_grid
from mgb_ops.model.prepare_mgb_meta import read_time_settings_from_parhig

DEFAULT_CHUNK_HOURS = 720
LOGGER_NAME = "model.prepare_mgb_rainfall"
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


@dataclass(frozen=True, slots=True)
class ForecastAssetMatch:
    asset_id: str
    asset_path: Path
    valid_from: datetime
    valid_to: datetime


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
    ranked = ranked.sort_values(["station_id", "state_rank", "created_at"], ascending=[True, True, False])
    preferred = ranked.drop_duplicates(subset=["station_id"], keep="first")
    return preferred.drop(columns=["state_rank"], errors="ignore").reset_index(drop=True)


def load_preferred_rain_stations(connection: sqlite3.Connection) -> pd.DataFrame:
    series = pd.read_sql_query(
        """
        SELECT
            os.series_id,
            os.station_id,
            os.state,
            os.created_at,
            st.latitude AS lat,
            st.longitude AS lon
        FROM observed_series os
        JOIN station st ON st.station_id = os.station_id
        WHERE os.variable_code = 'rain'
          AND st.latitude IS NOT NULL
          AND st.longitude IS NOT NULL
        """,
        connection,
    )
    preferred = _select_preferred_series_rows(series)
    preferred["lat"] = pd.to_numeric(preferred["lat"], errors="coerce")
    preferred["lon"] = pd.to_numeric(preferred["lon"], errors="coerce")
    return preferred.dropna(subset=["lat", "lon"]).sort_values("station_id").reset_index(drop=True)


def load_rain_values(
    connection: sqlite3.Connection,
    preferred_stations: pd.DataFrame,
    *,
    query_start: datetime,
    query_end_exclusive: datetime,
    batch_size: int = 400,
) -> pd.DataFrame:
    if preferred_stations.empty:
        return pd.DataFrame(columns=["station_id", "observed_at", "value"])

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
                    os.station_id,
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

    return pd.concat(frames, ignore_index=True) if frames else pd.DataFrame(columns=["station_id", "observed_at", "value"])


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

    preferred_stations = preferred_stations.copy()
    preferred_stations["station_id"] = preferred_stations["station_id"].astype(str)
    hourly_values = hourly_values.copy()
    hourly_values["station_id"] = hourly_values["station_id"].astype(str)
    hourly_values["observed_at"] = pd.to_datetime(hourly_values["observed_at"], errors="coerce")
    hourly_values["value"] = pd.to_numeric(hourly_values["value"], errors="coerce")
    hourly_values = hourly_values.dropna(subset=["station_id", "observed_at", "value"])
    pivoted = hourly_values.pivot_table(index="observed_at", columns="station_id", values="value", aggfunc="sum").reindex(time_index)
    available_station_ids = [
        station_id
        for station_id in preferred_stations["station_id"].astype(str).tolist()
        if station_id in pivoted.columns and pivoted[station_id].notna().any()
    ]
    if not available_station_ids:
        raise ValueError("No rain stations with valid values were found for the requested simulation window.")

    station_meta = preferred_stations.set_index("station_id").loc[available_station_ids].reset_index()
    station_matrix = (
        pivoted.reindex(columns=available_station_ids)
        .fillna(0.0)
        .transpose()
        .to_numpy(dtype=np.float64, copy=False)
    )
    return station_meta, station_matrix


def require_observed_values_aligned_to_timestep(values_df: pd.DataFrame, *, timestep_hours: int) -> None:
    timestep_hours = validate_timestep_hours(timestep_hours)
    if values_df.empty:
        return
    observed_at = pd.to_datetime(values_df["observed_at"], errors="coerce")
    if observed_at.isna().any():
        raise ValueError("Observed rainfall values contain invalid timestamps.")
    off_grid = (
        (observed_at.dt.minute != 0)
        | (observed_at.dt.second != 0)
        | (observed_at.dt.microsecond != 0)
        | (observed_at.dt.hour % timestep_hours != 0)
    )
    if off_grid.any():
        first_bad = observed_at[off_grid].iloc[0]
        raise ValueError(
            "Observed rainfall values must already be normalized to run.timestep_hours before MGB preparation. "
            f"First off-grid timestamp: {first_bad.isoformat()}"
        )


def load_latest_ecmwf_asset_path(
    connection: sqlite3.Connection,
    *,
    reference_time: datetime,
    asset_base_dir: Path,
) -> Path:
    reference_time_utc = reference_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc).replace(tzinfo=None)
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
            ECMWF_FORECAST_PRODUCT.asset_kind,
            reference_time_utc.isoformat(timespec="seconds"),
            reference_time_utc.isoformat(timespec="seconds"),
        ),
    ).fetchone()
    if row is None:
        raise FileNotFoundError(
            "No ECMWF forecast asset was found in history for the requested forecast window. "
            "Ingest forecast grids with `mgb_ops.adapters.forecast_ecmwf` first."
        )

    registered_path = Path(str(row["relative_path"]))
    asset_path = registered_path if registered_path.is_absolute() else asset_base_dir / registered_path
    if not asset_path.exists():
        raise FileNotFoundError(f"ECMWF asset registered in history does not exist on disk: {asset_path}")
    return asset_path


def find_required_ecmwf_asset(
    connection: sqlite3.Connection,
    *,
    reference_time: datetime,
    input_days_before: int,
    forecast_horizon_days: int,
    asset_base_dir: Path,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> ForecastAssetMatch | None:
    window = build_horizon_window(
        reference_time,
        days_before=input_days_before,
        horizon_days=forecast_horizon_days,
        timestep_hours=timestep_hours,
    )
    cycle_time = build_ecmwf_cycle(reference_time)
    expected_asset_id = build_asset_id(cycle_time, product_config)
    forecast_end_time = window.forecast_start_time + timedelta(hours=timestep_hours * (window.forecast_nt - 1))
    forecast_start_utc = (
        window.forecast_start_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc).replace(tzinfo=None)
    )
    forecast_end_utc = (
        forecast_end_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc).replace(tzinfo=None)
    )

    row = connection.execute(
        """
        SELECT asset_id, relative_path, valid_from, valid_to
        FROM asset
        WHERE asset_id = ?
          AND provider_code = ?
          AND asset_kind = ?
          AND valid_from IS NOT NULL
          AND valid_to IS NOT NULL
          AND valid_from <= ?
          AND valid_to >= ?
        LIMIT 1
        """,
        (
            expected_asset_id,
            product_config.provider_code,
            product_config.asset_kind,
            forecast_start_utc.isoformat(timespec="seconds"),
            forecast_end_utc.isoformat(timespec="seconds"),
        ),
    ).fetchone()
    if row is None:
        return None

    registered_path = Path(str(row[1]))
    asset_path = registered_path if registered_path.is_absolute() else Path(asset_base_dir) / registered_path
    if not asset_path.exists():
        raise FileNotFoundError(f"ECMWF asset registered in history does not exist on disk: {asset_path}")

    return ForecastAssetMatch(
        asset_id=str(row[0]),
        asset_path=asset_path,
        valid_from=datetime.fromisoformat(str(row[2])),
        valid_to=datetime.fromisoformat(str(row[3])),
    )


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


def build_forecast_start_time_utc(forecast_start_time: datetime) -> datetime:
    return forecast_start_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc).replace(tzinfo=None)


def load_required_forecast_precipitation_grid(
    netcdf_path: Path,
    *,
    forecast_start_time: datetime,
    forecast_nt: int,
    timestep_hours: int = 1,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    return load_forecast_precipitation_grid(
        netcdf_path,
        forecast_start_time_utc=build_forecast_start_time_utc(forecast_start_time),
        forecast_nt=forecast_nt,
        timestep_hours=timestep_hours,
    )


def prepare_mgb_rainfall(
    *,
    history_db: Path,
    parhig_path: Path,
    mini_gtp_path: Path,
    output_path: Path,
    reference_time: datetime,
    input_days_before: int,
    forecast_horizon_days: int,
    use_forecast_data: bool,
    nearest_stations: int,
    power: float,
    timestep_hours: int = 1,
    chunk_hours: int = DEFAULT_CHUNK_HOURS,
    forecast_asset_path: Path | None = None,
    logs_dir: Path | None = None,
    logger: logging.Logger | None = None,
) -> RainfallPreparationSummary:
    timestep_hours = validate_timestep_hours(timestep_hours)
    if not history_db.exists():
        raise FileNotFoundError(f"History database not found: {history_db}")
    if not parhig_path.exists():
        raise FileNotFoundError(f"PARHIG not found: {parhig_path}")
    if not mini_gtp_path.exists():
        raise FileNotFoundError(f"MINI.gtp not found: {mini_gtp_path}")

    run_logger = logger
    if run_logger is None and logs_dir is not None:
        execution_id = build_execution_id()
        run_logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")
    if run_logger is None:
        run_logger = logging.getLogger(LOGGER_NAME)

    start_time, nt, dt_seconds = read_time_settings_from_parhig(parhig_path)
    expected_dt_seconds = timestep_hours * 3600
    if dt_seconds != expected_dt_seconds:
        raise ValueError(
            f"PARHIG DT={dt_seconds} does not match run.timestep_hours={timestep_hours} "
            f"(expected {expected_dt_seconds})."
        )

    window = build_horizon_window(
        reference_time,
        days_before=input_days_before,
        horizon_days=forecast_horizon_days,
        timestep_hours=timestep_hours,
    )
    if start_time != window.start_time or nt != window.nt:
        raise ValueError(
            "PARHIG timing does not match current settings. "
            f"Expected start_time={window.start_time.isoformat(timespec='seconds')} nt={window.nt}, "
            f"found start_time={start_time.isoformat(timespec='seconds')} nt={nt}."
        )

    end_time_exclusive = start_time + timedelta(seconds=nt * dt_seconds)
    nc = read_nc_from_parhig(parhig_path)
    mini_df = read_mini_centroids(mini_gtp_path, nc=nc)
    query_start = start_time
    observed_end_exclusive = window.forecast_start_time

    run_logger.info(
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

    require_observed_values_aligned_to_timestep(raw_values, timestep_hours=timestep_hours)
    used_hourly_normalization = False
    observed_time_index = pd.date_range(start=start_time, periods=observed_hours, freq=f"{timestep_hours}h")
    station_meta, station_matrix = build_hourly_station_matrix(
        preferred_stations,
        raw_values,
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
        if forecast_asset_path is None:
            raise ValueError("forecast_asset_path is required when use_forecast_data is true.")
        forecast_latitudes, forecast_longitudes, forecast_hourly_grids = load_required_forecast_precipitation_grid(
            forecast_asset_path,
            forecast_start_time=window.forecast_start_time,
            forecast_nt=window.forecast_nt,
            timestep_hours=timestep_hours,
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

    run_logger.info(
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
