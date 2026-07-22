from __future__ import annotations

import json
import logging
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path

import numpy as np
import pandas as pd

from mgb_ops.assets.grid_transforms import (
    build_grid_idw_neighbors,
    build_idw_neighbors,
    interpolate_station_chunk,
    resample_regular_grid,
)
from mgb_ops.assets.observed_precipitation import build_observed_precipitation_cache
from mgb_ops.utils.time import TIMEZONE, build_horizon_window, validate_timestep_hours
from mgb_ops.utils.logging import configure_run_logger as _configure_run_logger
from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec, read_spatial_grid, write_spatial_grid
from mgb_ops.edit.forcing import ForecastCorrectionInstruction, apply_grid_correction
from mgb_ops.assets.history_queries import (
    open_history_read_only,
    read_observed_values,
    read_rain_series,
    select_preferred_series_rows,
)
from mgb_ops.model.export_mgb_outputs import read_nc_from_parhig
from mgb_ops.model.prepare_mgb_meta import read_time_settings_from_parhig

DEFAULT_CHUNK_HOURS = 720
MGB_OBSERVED_CACHE_FILENAME = "precipitations_mgb_observed.nc"
MGB_FORECAST_CACHE_FILENAME = "precipitations_mgb_forecast.nc"
LOGGER_NAME = "model.prepare_mgb_rainfall"


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
    return _configure_run_logger(LOGGER_NAME, log_file)


_connect_history_read_only = open_history_read_only


def load_preferred_rain_stations(connection) -> pd.DataFrame:
    series = read_rain_series(connection)
    preferred = select_preferred_series_rows(series)
    preferred["lat"] = pd.to_numeric(preferred["lat"], errors="coerce")
    preferred["lon"] = pd.to_numeric(preferred["lon"], errors="coerce")
    return preferred.dropna(subset=["lat", "lon"]).sort_values("station_id").reset_index(drop=True)


def load_rain_values(
    connection,
    preferred_stations: pd.DataFrame,
    *,
    query_start: datetime,
    query_end_exclusive: datetime,
    batch_size: int = 400,
) -> pd.DataFrame:
    if preferred_stations.empty:
        return pd.DataFrame(columns=["station_id", "observed_at", "value"])
    return read_observed_values(
        connection,
        preferred_stations["series_id"].astype(str).tolist(),
        start_time=query_start,
        end_time=query_end_exclusive,
        batch_size=batch_size,
    )[["station_id", "observed_at", "value"]]


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


def load_forecast_precipitation_grid(
    netcdf_path: Path,
    *,
    forecast_start_time_utc: datetime,
    forecast_nt: int,
    timestep_hours: int = 1,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    timestep_hours = validate_timestep_hours(timestep_hours)
    if forecast_nt < 0:
        raise ValueError("forecast_nt must be >= 0.")
    grid = read_spatial_grid(netcdf_path)
    if grid.variable != "precipitation" or grid.grid_type != "forecast":
        raise ValueError("Expected a forecast precipitation spatial grid.")
    if grid.timestep_hours != timestep_hours:
        raise ValueError(
            f"Forecast NetCDF timestep_hours={grid.timestep_hours} "
            f"does not match requested timestep_hours={timestep_hours}."
        )
    if forecast_nt == 0:
        return grid.latitudes, grid.longitudes, grid.values[:0]

    required_times = tuple(
        forecast_start_time_utc + timedelta(hours=timestep_hours * offset)
        for offset in range(forecast_nt)
    )
    required_times = tuple(value.replace(tzinfo=timezone.utc) for value in required_times)
    index_by_time = {valid_time: idx for idx, valid_time in enumerate(grid.times_utc)}
    missing_times = [valid_time for valid_time in required_times if valid_time not in index_by_time]
    if missing_times:
        raise ValueError(
            "Forecast NetCDF does not cover the full requested UTC forecast window. "
            f"First missing timestep: {missing_times[0].isoformat(timespec='seconds')}"
        )

    selected_indices = [index_by_time[valid_time] for valid_time in required_times]
    return grid.latitudes, grid.longitudes, grid.values[selected_indices, :, :]


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


def _build_forecast_working_cache(
    source_path: Path,
    target_path: Path,
    *,
    grid_spec: RegularGridSpec,
    forecast_start_time: datetime,
    forecast_nt: int,
    timestep_hours: int,
    correction: ForecastCorrectionInstruction | None = None,
) -> Path:
    source = read_spatial_grid(source_path)
    if source.variable != "precipitation" or source.grid_type != "forecast":
        raise ValueError("Expected a forecast precipitation spatial grid.")
    target_ends = [
        build_forecast_start_time_utc(forecast_start_time)
        + timedelta(hours=index * timestep_hours)
        for index in range(forecast_nt)
    ]
    target_bounds = [
        (end - timedelta(hours=timestep_hours), end) for end in target_ends
    ]
    fields: list[np.ndarray] = []
    for target_start, target_end in target_bounds:
        matches = [
            index
            for index, (source_start, source_end) in enumerate(source.time_bounds_utc)
            if source_start <= target_start.replace(tzinfo=timezone.utc)
            and source_end >= target_end.replace(tzinfo=timezone.utc)
        ]
        if len(matches) != 1:
            raise ValueError(
                "Forecast asset does not uniquely cover MGB timestep "
                f"({target_start.isoformat()}, {target_end.isoformat()}]."
            )
        source_index = matches[0]
        source_start, source_end = source.time_bounds_utc[source_index]
        source_hours = (source_end - source_start).total_seconds() / 3600.0
        if source_hours % timestep_hours:
            raise ValueError("Forecast native interval cannot be split into MGB timesteps.")
        source_grid = PrecipitationGrid(
            values=source.values[source_index],
            latitudes=source.latitudes,
            longitudes=source.longitudes,
            bounds=tuple(float(value) for value in json.loads(str(source.metadata["bbox"]))),
            start_time=source_start,
            end_time=source_end,
            units=source.units,
            source="forecast",
        )
        if correction is not None:
            cycle_start = source.time_bounds_utc[0][0]
            correction_start = cycle_start + timedelta(hours=correction.t0_step)
            correction_end = cycle_start + timedelta(hours=correction.t1_step)
            if source_start >= correction_start and source_end <= correction_end:
                source_grid = apply_grid_correction(source_grid, correction)
        per_step = source_grid.values / (source_hours / timestep_hours)
        fields.append(
            resample_regular_grid(
                per_step,
                source_grid.latitudes,
                source_grid.longitudes,
                grid_spec,
            )
        )
    values = np.stack(fields)
    if not np.isfinite(values).all():
        raise ValueError("Forecast asset does not cover the complete MGB working grid.")
    return write_spatial_grid(
        target_path,
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=source.providers,
        units=source.units,
        bbox=grid_spec.effective_bbox,
        resolution_degrees=grid_spec.resolution,
        times_utc=[value.replace(tzinfo=timezone.utc) for value in target_ends],
        latitudes=grid_spec.latitudes,
        longitudes=grid_spec.longitudes,
        values=values,
        timestep_hours=timestep_hours,
        title="Forecast precipitation prepared for MGB",
        processing_metadata={
            "model_role": "forecast_working_grid",
            "source_asset": str(source_path),
            "spatial_resampling_method": "bilinear",
            "temporal_resampling_method": "uniform_interval_split",
            "requested_bbox": list(grid_spec.bbox),
            "effective_bbox": list(grid_spec.effective_bbox),
            "boundary_cell_policy": "closed_footprint_intersects_bbox",
        },
    )


def _grid_to_mini_matrix(
    grid,
    mini_df: pd.DataFrame,
    *,
    nearest_points: int,
    power: float,
    chunk_hours: int,
) -> np.ndarray:
    """IDW-interpolate a time-varying regular grid to MINI.gtp centroids."""
    source_matrix = grid.values.reshape(len(grid.times_utc), -1).transpose()
    nearest_idx, weights = build_grid_idw_neighbors(
        mini_df,
        latitudes=grid.latitudes,
        longitudes=grid.longitudes,
        nearest_points=nearest_points,
        power=power,
    )
    return interpolate_source_matrix(
        source_matrix,
        nearest_idx=nearest_idx,
        weights=weights,
        chunk_hours=chunk_hours,
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
    forecast_correction: ForecastCorrectionInstruction | None = None,
    cache_dir: Path | None = None,
    spatial_bbox: tuple[float, float, float, float] | None = None,
    spatial_resolution_degrees: float | None = None,
    observed_providers: tuple[str, ...] | list[str] | None = None,
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

    cache_grid_mode = any(
        value is not None
        for value in (cache_dir, spatial_bbox, spatial_resolution_degrees)
    )
    if cache_grid_mode:
        if (
            cache_dir is None
            or spatial_bbox is None
            or spatial_resolution_degrees is None
            or observed_providers is None
        ):
            raise ValueError(
                "cache_dir, spatial_bbox, spatial_resolution_degrees, and "
                "observed_providers are all required for cache-grid rainfall preparation."
            )
        grid_spec = RegularGridSpec(
            bbox=spatial_bbox,
            resolution_degrees=spatial_resolution_degrees,
            include_boundary_cells=True,
        )
        cache_dir.mkdir(parents=True, exist_ok=True)
        observed_cache_path = build_observed_precipitation_cache(
            history_db,
            cache_dir,
            bbox=spatial_bbox,
            resolution_degrees=grid_spec.resolution,
            start_time_utc=(
                start_time - timedelta(hours=timestep_hours)
            ).replace(tzinfo=TIMEZONE).astimezone(timezone.utc),
            end_time_utc=window.reference_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc),
            timestep_hours=timestep_hours,
            providers=observed_providers,
            nearest_stations=nearest_stations,
            power=power,
            filename=MGB_OBSERVED_CACHE_FILENAME,
            include_boundary_cells=True,
            processing_metadata={
                "model_role": "observed_working_grid",
                "requested_bbox": list(grid_spec.bbox),
                "effective_bbox": list(grid_spec.effective_bbox),
                "boundary_cell_policy": "closed_footprint_intersects_bbox",
            },
        )
        observed_grid = read_spatial_grid(observed_cache_path)
        if not np.isfinite(observed_grid.values).all():
            raise ValueError("Observed data does not cover the complete MGB working grid and window.")
        observed_mini_matrix = _grid_to_mini_matrix(
            observed_grid,
            mini_df,
            nearest_points=nearest_stations,
            power=power,
            chunk_hours=chunk_hours,
        )
        with _connect_history_read_only(history_db) as connection:
            station_count = len(load_preferred_rain_stations(connection))
        if use_forecast_data and window.forecast_nt > 0:
            if forecast_asset_path is None:
                raise ValueError("forecast_asset_path is required when use_forecast_data is true.")
            forecast_cache_path = _build_forecast_working_cache(
                forecast_asset_path,
                cache_dir / MGB_FORECAST_CACHE_FILENAME,
                grid_spec=grid_spec,
                forecast_start_time=window.forecast_start_time,
                forecast_nt=window.forecast_nt,
                timestep_hours=timestep_hours,
                correction=forecast_correction,
            )
            forecast_grid = read_spatial_grid(forecast_cache_path)
            forecast_mini_matrix = _grid_to_mini_matrix(
                forecast_grid,
                mini_df,
                nearest_points=nearest_stations,
                power=power,
                chunk_hours=chunk_hours,
            )
        else:
            forecast_mini_matrix = np.zeros((nc, window.forecast_nt), dtype=np.float64)
        used_hourly_normalization = False
    else:
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
        station_count = len(station_meta)
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
        station_count,
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
        station_count=station_count,
        nearest_stations=min(int(nearest_stations), station_count),
        power=float(power),
        used_hourly_normalization=used_hourly_normalization,
        forecast_hours=window.forecast_nt,
    )
