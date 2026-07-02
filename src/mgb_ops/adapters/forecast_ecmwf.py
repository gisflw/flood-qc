from __future__ import annotations

import logging
import os
import tempfile
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
import sys

import numpy as np

from mgb_ops.adapters._grib2 import (
    TpGribMessage,
    build_grid_arrays,
    read_tp_grib_messages,
    require_eccodes,
    set_regular_ll_grid,
)
from mgb_ops.analysis.spatial import RegularGridSpec, resample_regular_grid
from mgb_ops.assets.spatial_grid import SPATIAL_GRID_ASSET_KIND, write_spatial_grid
from mgb_ops.common.time_utils import TIMEZONE, validate_timestep_hours

LOGGER_NAME = "adapters.forecast_ecmwf"
ECMWF_ASSET_KIND = SPATIAL_GRID_ASSET_KIND
ECMWF_MODEL = "ifs"
ECMWF_PRODUCT_TYPE = "fc"
ECMWF_RESOLUTION = "0p25"
ECMWF_PARAM = "tp"


@dataclass(frozen=True, slots=True)
class ForecastProductConfig:
    provider_code: str
    asset_kind: str
    model: str
    product_type: str
    resolution: str
    param: str
    step_schedule: tuple[int, ...]


@dataclass(frozen=True, slots=True)
class NormalizedForecastGrid:
    run_id: str
    asset_path: Path
    cycle_time: datetime
    valid_from: datetime
    valid_to: datetime
    bbox: tuple[float, float, float, float]


ECMWF_FORECAST_PRODUCT = ForecastProductConfig(
    provider_code="ecmwf",
    asset_kind=ECMWF_ASSET_KIND,
    model=ECMWF_MODEL,
    product_type=ECMWF_PRODUCT_TYPE,
    resolution=ECMWF_RESOLUTION,
    param=ECMWF_PARAM,
    step_schedule=tuple([hour for hour in range(0, 144 + 3, 3)] + [hour for hour in range(150, 360 + 6, 6)]),
)


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id(reference_time: datetime) -> str:
    return reference_time.strftime("%Y%m%dT%H%M%S")


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


def _require_opendata_client():
    try:
        from ecmwf.opendata import Client
    except ModuleNotFoundError as exc:
        raise ModuleNotFoundError(
            "Missing dependency for ECMWF ingestion: install `ecmwf-opendata` in the operational environment."
        ) from exc
    return Client


def build_ecmwf_cycle(reference_time: datetime) -> datetime:
    # `reference_time` arrives in local time (America/Sao_Paulo) as the measurement cutoff.
    # The MGB forecast starts on the next hour, so resolve the ECMWF cycle from that
    # forecast start converted to UTC.
    forecast_start_local = reference_time + timedelta(hours=1)
    forecast_start_utc = forecast_start_local.replace(tzinfo=TIMEZONE).astimezone(timezone.utc)
    return datetime(forecast_start_utc.year, forecast_start_utc.month, forecast_start_utc.day, 0, 0, 0)


def build_ecmwf_steps(product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT) -> list[int]:
    return list(product_config.step_schedule)


def build_output_path(
    downloads_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> Path:
    directory = downloads_root / product_config.provider_code
    file_name = (
        f"{product_config.product_type}_{cycle_time.strftime('%Y-%m-%d')}_{cycle_time:%H}_"
        f"{product_config.model.upper()}_precipitation_grid.nc"
    )
    return directory / file_name


def build_grib_path(
    downloads_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> Path:
    return (
        Path(downloads_root)
        / product_config.provider_code
        / f"{product_config.model}_{product_config.product_type}_{cycle_time:%Y%m%dT%H%M%SZ}.grib2"
    )


def build_asset_path(
    assets_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> Path:
    return Path(assets_root) / product_config.provider_code / f"{build_asset_id(cycle_time, product_config)}.nc"


def build_asset_id(cycle_time: datetime, product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT) -> str:
    return (
        f"{product_config.provider_code}.{product_config.model}.{product_config.product_type}."
        f"{cycle_time.strftime('%Y%m%dT%H%M%SZ')}.precipitation_grid"
    )


def crop_grib_to_bbox(
    source_path: Path,
    target_path: Path,
    *,
    bbox: tuple[float, float, float, float],
) -> None:
    eccodes = require_eccodes()
    west, south, east, north = bbox
    target_path.parent.mkdir(parents=True, exist_ok=True)

    wrote_any = False
    with source_path.open("rb") as src_handle, target_path.open("wb") as dst_handle:
        while True:
            gid = eccodes.codes_grib_new_from_file(src_handle)
            if gid is None:
                break
            try:
                latitudes, longitudes, values = build_grid_arrays(gid)
                lat_mask = (latitudes >= south) & (latitudes <= north)
                lon_mask = (longitudes >= west) & (longitudes <= east)
                if not lat_mask.any() or not lon_mask.any():
                    raise ValueError(
                        f"Requested bbox {bbox} does not intersect GRIB grid in {source_path}."
                    )

                cropped_latitudes = latitudes[lat_mask]
                cropped_longitudes = longitudes[lon_mask]
                cropped_values = values[np.ix_(lat_mask, lon_mask)]
                set_regular_ll_grid(
                    gid,
                    latitudes=cropped_latitudes,
                    longitudes=cropped_longitudes,
                    values=cropped_values,
                )
                eccodes.codes_write(gid, dst_handle)
                wrote_any = True
            finally:
                eccodes.codes_release(gid)

    if not wrote_any:
        raise ValueError(f"No GRIB messages were written to {target_path}.")


def extract_valid_time_bounds(grib_path: Path) -> tuple[datetime, datetime]:
    messages = read_tp_grib_messages(grib_path)
    return messages[0].valid_time, messages[-1].valid_time


def build_hourly_precipitation_from_cumulative_messages(
    messages: list[TpGribMessage],
) -> tuple[tuple[datetime, ...], np.ndarray, np.ndarray, np.ndarray]:
    if not messages:
        raise ValueError("No ECMWF precipitation messages were provided.")

    ordered_messages = sorted(messages, key=lambda item: (item.valid_time, item.step_hours))
    first_message = ordered_messages[0]
    cycle_time = min(message.valid_time - timedelta(hours=message.step_hours) for message in ordered_messages)
    prev_valid_time = cycle_time
    prev_cumulative = np.zeros_like(first_message.values_mm, dtype=np.float64)
    latitudes = first_message.latitudes
    longitudes = first_message.longitudes
    hourly_times: list[datetime] = []
    hourly_grids: list[np.ndarray] = []

    for message in ordered_messages:
        if message.values_mm.shape != prev_cumulative.shape:
            raise ValueError("ECMWF GRIB contains inconsistent grid shapes across messages.")
        if not np.allclose(message.latitudes, latitudes) or not np.allclose(message.longitudes, longitudes):
            raise ValueError("ECMWF GRIB contains inconsistent grid coordinates across messages.")

        delta_seconds = int((message.valid_time - prev_valid_time).total_seconds())
        if delta_seconds < 0 or delta_seconds % 3600 != 0:
            raise ValueError(
                "ECMWF GRIB valid times are not monotonic hourly multiples; cannot build canonical hourly grid."
            )

        delta_hours = delta_seconds // 3600
        increment = message.values_mm - prev_cumulative
        increment = np.where(np.isfinite(increment), increment, np.nan)
        increment[increment < 0.0] = 0.0
        if delta_hours > 0:
            per_hour = increment / float(delta_hours)
            for hour_offset in range(delta_hours):
                hourly_times.append(prev_valid_time + timedelta(hours=hour_offset + 1))
                hourly_grids.append(per_hour.copy())

        prev_valid_time = message.valid_time
        prev_cumulative = np.asarray(message.values_mm, dtype=np.float64)

    if not hourly_grids:
        raise ValueError("ECMWF GRIB did not contain any positive-length accumulation interval.")
    return tuple(hourly_times), latitudes, longitudes, np.stack(hourly_grids, axis=0)


def aggregate_hourly_precipitation_to_timestep(
    times_utc: tuple[datetime, ...] | list[datetime],
    precipitation_mm: np.ndarray,
    *,
    timestep_hours: int,
) -> tuple[tuple[datetime, ...], np.ndarray]:
    timestep_hours = validate_timestep_hours(timestep_hours)
    times_utc = tuple(times_utc)
    if not times_utc:
        raise ValueError("times_utc must contain at least one timestamp.")
    for previous, current in zip(times_utc, times_utc[1:]):
        if current - previous != timedelta(hours=1):
            raise ValueError("times_utc must be a contiguous 1-hour UTC sequence.")

    precipitation_values = np.asarray(precipitation_mm, dtype=np.float64)
    if precipitation_values.shape[0] != len(times_utc):
        raise ValueError(
            f"precipitation_mm time dimension mismatch: expected {len(times_utc)}, "
            f"found {precipitation_values.shape[0]}."
        )
    if timestep_hours == 1:
        return times_utc, precipitation_values

    full_bucket_count = len(times_utc) // timestep_hours
    if full_bucket_count < 1:
        raise ValueError("Not enough hourly precipitation values for one full timestep bucket.")
    usable_count = full_bucket_count * timestep_hours
    bucket_times = tuple(times_utc[(idx + 1) * timestep_hours - 1] for idx in range(full_bucket_count))
    bucket_values = precipitation_values[:usable_count].reshape(
        full_bucket_count,
        timestep_hours,
        *precipitation_values.shape[1:],
    ).sum(axis=1)
    return bucket_times, bucket_values


def write_canonical_forecast_grid_from_grib(
    grib_path: Path,
    netcdf_path: Path,
    *,
    cycle_time: datetime,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> tuple[datetime, datetime]:
    hourly_times, latitudes, longitudes, hourly_grids = build_hourly_precipitation_from_cumulative_messages(
        read_tp_grib_messages(grib_path)
    )
    timestep_times, timestep_grids = aggregate_hourly_precipitation_to_timestep(
        hourly_times,
        hourly_grids,
        timestep_hours=timestep_hours,
    )
    target_grid = RegularGridSpec(bbox=bbox, resolution_degrees=resolution_degrees)
    resampled = np.stack(
        [
            resample_regular_grid(field, latitudes, longitudes, target_grid)
            for field in timestep_grids
        ]
    )
    cycle_time_utc = cycle_time.replace(tzinfo=timezone.utc) if cycle_time.tzinfo is None else cycle_time.astimezone(timezone.utc)
    write_spatial_grid(
        netcdf_path,
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=[product_config.provider_code],
        units="mm",
        bbox=target_grid.bbox,
        resolution_degrees=target_grid.resolution,
        times_utc=[
            value.replace(tzinfo=timezone.utc) if value.tzinfo is None else value.astimezone(timezone.utc)
            for value in timestep_times
        ],
        latitudes=target_grid.latitudes,
        longitudes=target_grid.longitudes,
        values=resampled,
        timestep_hours=timestep_hours,
        title="ECMWF IFS precipitation forecast grid",
        processing_metadata={
            "provider": product_config.provider_code,
            "source_format": "GRIB2",
            "source_cycle_time": cycle_time_utc.isoformat(timespec="seconds").replace("+00:00", "Z"),
            "resampling_method": "bilinear",
            "model": product_config.model,
            "product_type": product_config.product_type,
            "source_resolution": product_config.resolution,
            "source_parameter": product_config.param,
        },
    )
    return (
        timestep_times[0] - timedelta(hours=timestep_hours),
        timestep_times[-1],
    )

def download_ecmwf_grib_to_path(
    target_path: Path,
    *,
    reference_time: datetime,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> None:
    Client = _require_opendata_client()
    cycle_time = build_ecmwf_cycle(reference_time)
    client = Client()
    client.retrieve(
        date=cycle_time.strftime("%Y-%m-%d"),
        model=product_config.model,
        time=cycle_time.hour,
        step=build_ecmwf_steps(product_config),
        resol=product_config.resolution,
        type=product_config.product_type,
        levtype="sfc",
        param=[product_config.param],
        target=str(target_path),
    )


def download_forecast_grib(
    *,
    reference_time: datetime,
    downloads_dir: Path,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> Path:
    cycle_time = build_ecmwf_cycle(reference_time)
    target = build_grib_path(downloads_dir, cycle_time, product_config)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        messages = read_tp_grib_messages(target)
        found_cycle = min(message.valid_time - timedelta(hours=message.step_hours) for message in messages)
        found_steps = {message.step_hours for message in messages}
        if found_cycle == cycle_time and set(build_ecmwf_steps(product_config)).issubset(found_steps):
            return target
        target.unlink()
    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target.name}.", suffix=".tmp", dir=target.parent
    )
    os.close(descriptor)
    temporary = Path(temporary_name)
    try:
        download_ecmwf_grib_to_path(
            temporary, reference_time=reference_time, product_config=product_config
        )
        messages = read_tp_grib_messages(temporary)
        found_cycle = min(message.valid_time - timedelta(hours=message.step_hours) for message in messages)
        found_steps = {message.step_hours for message in messages}
        if found_cycle != cycle_time or not set(build_ecmwf_steps(product_config)).issubset(found_steps):
            raise ValueError("Downloaded ECMWF GRIB2 has an unexpected cycle or incomplete steps.")
        os.replace(temporary, target)
    finally:
        temporary.unlink(missing_ok=True)
    return target


def process_forecast_grib(
    grib_path: Path,
    *,
    cycle_time: datetime,
    assets_dir: Path,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> NormalizedForecastGrid:
    target = build_asset_path(assets_dir, cycle_time, product_config)
    target.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="ecmwf_crop_") as temp_dir_name:
        cropped = Path(temp_dir_name) / "cropped.grib2"
        crop_grib_to_bbox(grib_path, cropped, bbox=bbox)
        valid_from, valid_to = write_canonical_forecast_grid_from_grib(
            cropped,
            target,
            cycle_time=cycle_time,
            bbox=bbox,
            resolution_degrees=resolution_degrees,
            timestep_hours=timestep_hours,
            product_config=product_config,
        )
    return NormalizedForecastGrid(
        run_id=build_execution_id(cycle_time),
        asset_path=target,
        cycle_time=cycle_time,
        valid_from=valid_from,
        valid_to=valid_to,
        bbox=bbox,
    )


def store_normalized_forecast_grid(
    *,
    reference_time: datetime,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    downloads_dir: Path,
    logs_dir: Path,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> NormalizedForecastGrid:
    execution_id = build_execution_id(reference_time)
    logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")
    cycle_time = build_ecmwf_cycle(reference_time)
    target_path = build_output_path(downloads_dir, cycle_time, product_config)
    RegularGridSpec(bbox=bbox, resolution_degrees=resolution_degrees)

    logger.info(
        "forecast_grid_start cycle_time=%s bbox=%s target=%s",
        cycle_time.strftime("%Y-%m-%dT%H:%M:%S"),
        bbox,
        target_path,
    )

    with tempfile.TemporaryDirectory(prefix="ecmwf_download_") as temp_dir_name:
        temp_dir = Path(temp_dir_name)
        temp_grib_path = temp_dir / "download.grib2"
        cropped_grib_path = temp_dir / "cropped.grib2"
        download_ecmwf_grib_to_path(temp_grib_path, reference_time=reference_time, product_config=product_config)
        crop_grib_to_bbox(temp_grib_path, cropped_grib_path, bbox=bbox)
        valid_from, valid_to = write_canonical_forecast_grid_from_grib(
            cropped_grib_path,
            target_path,
            cycle_time=cycle_time,
            bbox=bbox,
            resolution_degrees=resolution_degrees,
            timestep_hours=timestep_hours,
            product_config=product_config,
        )

    logger.info(
        "forecast_grid_done path=%s valid_from=%s valid_to=%s",
        target_path,
        valid_from.isoformat(timespec="seconds"),
        valid_to.isoformat(timespec="seconds"),
    )
    return NormalizedForecastGrid(
        run_id=execution_id,
        asset_path=target_path,
        cycle_time=cycle_time,
        valid_from=valid_from,
        valid_to=valid_to,
        bbox=bbox,
    )
