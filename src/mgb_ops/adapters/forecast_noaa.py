from __future__ import annotations

import os
import logging
import tempfile
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Iterable

import numpy as np
from tqdm import tqdm

from mgb_ops.adapters._grib2 import (
    TpGribMessage,
    read_precipitation_grib_messages,
)
from mgb_ops.adapters.forecast_ecmwf import (
    FORECAST_BBOX_BUFFER_FRACTION,
    ForecastProductConfig,
    NormalizedForecastGrid,
    build_bbox_with_buffer,
    build_execution_id,
    write_canonical_forecast_grid_from_grib,
)
from mgb_ops.assets.spatial_grid import SPATIAL_GRID_ASSET_KIND
from mgb_ops.utils.logging import configure_run_logger


GFS_ASSET_KIND = SPATIAL_GRID_ASSET_KIND
GFS_MODEL = "gfs"
GFS_PRODUCT_TYPE = "fc"
GFS_RESOLUTION = "0p25"
GFS_PARAM = "APCP"
GFS_NOMADS_FILTER_URL = "https://nomads.ncep.noaa.gov/cgi-bin/filter_gfs_0p25.pl"
GFS_FORECAST_PRODUCT = ForecastProductConfig(
    provider_code="noaa",
    asset_kind=GFS_ASSET_KIND,
    model=GFS_MODEL,
    product_type=GFS_PRODUCT_TYPE,
    resolution=GFS_RESOLUTION,
    param=GFS_PARAM,
    step_schedule=tuple([hour for hour in range(3, 240 + 3, 3)] + [hour for hour in range(252, 384 + 12, 12)]),
)


def build_gfs_steps(product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT) -> list[int]:
    return list(product_config.step_schedule)


def build_required_gfs_steps(
    cycle_time: datetime,
    required_end: datetime,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> list[int]:
    """Return native GFS steps needed to cover the requested forecast end."""
    cycle = cycle_time.replace(tzinfo=timezone.utc) if cycle_time.tzinfo is None else cycle_time.astimezone(timezone.utc)
    end = required_end.replace(tzinfo=timezone.utc) if required_end.tzinfo is None else required_end.astimezone(timezone.utc)
    required_hours = max(0, (end - cycle).total_seconds() / 3600)
    available = build_gfs_steps(product_config)
    steps = [step for step in available if step <= required_hours]
    following = next((step for step in available if step > required_hours), None)
    if following is not None:
        steps.append(following)
    if not steps:
        raise ValueError("Requested forecast end is before the first available GFS interval.")
    return steps


def build_asset_id(cycle_time: datetime, product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT) -> str:
    return (
        f"{product_config.provider_code}.{product_config.model}.{product_config.product_type}."
        f"{cycle_time:%Y%m%dT%H%M%SZ}.precipitation_grid"
    )


def build_grib_path(
    downloads_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> Path:
    return (
        Path(downloads_root)
        / product_config.provider_code
        / f"{product_config.model}_{product_config.product_type}_{cycle_time:%Y%m%dT%H%M%SZ}.grib2"
    )


def build_asset_path(
    assets_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> Path:
    return Path(assets_root) / product_config.provider_code / f"{build_asset_id(cycle_time, product_config)}.nc"


def build_gfs_url(
    *,
    cycle_time: datetime,
    forecast_hour: int,
    bbox: tuple[float, float, float, float],
    variables: Iterable[str],
    levels: Iterable[str],
) -> tuple[str, dict[str, str]]:
    if cycle_time.tzinfo is not None:
        cycle_time = cycle_time.astimezone(timezone.utc).replace(tzinfo=None)
    if forecast_hour < 0 or forecast_hour > 384:
        raise ValueError("forecast_hour must be between 0 and 384.")
    west, south, east, north = bbox
    if west >= east or south >= north:
        raise ValueError("bbox must satisfy west < east and south < north.")
    params = {
        "file": f"gfs.t{cycle_time:%H}z.pgrb2.0p25.f{forecast_hour:03d}",
        "dir": f"/gfs.{cycle_time:%Y%m%d}/{cycle_time:%H}/atmos",
        "subregion": "",
        "leftlon": str(west),
        "rightlon": str(east),
        "bottomlat": str(south),
        "toplat": str(north),
    }
    for variable in variables:
        params[f"var_{variable}"] = "on"
    for level in levels:
        params[f"lev_{level}"] = "on"
    return GFS_NOMADS_FILTER_URL, params


def is_grib2(content: bytes) -> bool:
    return len(content) >= 4 and content[:4] == b"GRIB"


def _require_requests():
    try:
        import requests
    except ModuleNotFoundError as exc:
        raise ModuleNotFoundError(
            "Missing dependency for GFS ingestion: install `requests` in the operational environment."
        ) from exc
    return requests


def request_gfs_file(
    *,
    cycle_time: datetime,
    forecast_hour: int,
    bbox: tuple[float, float, float, float],
    timeout_seconds: float = 120.0,
    session=None,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
):
    requests = _require_requests()
    http = session or requests.Session()
    url, params = build_gfs_url(
        cycle_time=cycle_time,
        forecast_hour=forecast_hour,
        bbox=bbox,
        variables=[product_config.param],
        levels=["surface"],
    )
    response = http.get(
        url,
        params=params,
        timeout=timeout_seconds,
        headers={"User-Agent": "mgb-ops-gfs-downloader/1.0"},
    )
    response.raise_for_status()
    if not is_grib2(response.content):
        preview = response.text[:300].replace("\n", " ")
        raise RuntimeError(
            "NOMADS did not return a GRIB2 file. "
            f"cycle_time={cycle_time:%Y-%m-%dT%H:%M:%SZ} "
            f"forecast_hour={forecast_hour}; response preview={preview!r}"
        )
    return response


def read_gfs_precipitation_messages(grib_path: Path) -> list[TpGribMessage]:
    """Read the authoritative, cumulative APCP series from a GFS GRIB file.

    NOMADS responses can contain both cumulative APCP fields and rolling APCP
    fields for the same forecast output. Only ``startStep == 0`` represents
    the cumulative series used to construct canonical native intervals.
    """
    messages = read_precipitation_grib_messages(
        grib_path,
        short_names=("tp", "apcp"),
        values_multiplier=1.0,
    )
    cumulative_messages = [message for message in messages if message.start_step_hours == 0]
    if not cumulative_messages:
        raise ValueError("NOAA GFS GRIB2 contains no cumulative APCP messages (startStep=0).")

    unique_messages: dict[tuple[datetime, int, int], TpGribMessage] = {}
    for message in cumulative_messages:
        cycle_time = message.valid_time - timedelta(hours=message.step_hours)
        interval = (cycle_time, message.start_step_hours, message.step_hours)
        previous = unique_messages.get(interval)
        if previous is None:
            unique_messages[interval] = message
            continue
        same_coordinates = (
            np.array_equal(previous.latitudes, message.latitudes)
            and np.array_equal(previous.longitudes, message.longitudes)
        )
        same_values = np.array_equal(previous.values_mm, message.values_mm, equal_nan=True)
        if not same_coordinates or not same_values:
            raise ValueError(
                "NOAA GFS GRIB2 has conflicting duplicate cumulative APCP messages "
                f"for forecast interval {message.start_step_hours}-{message.step_hours} hours: "
                "coordinates or values differ."
            )
    return sorted(unique_messages.values(), key=lambda item: (item.valid_time, item.step_hours))


def download_gfs_grib_to_path(
    target_path: Path,
    *,
    cycle_time: datetime,
    bbox: tuple[float, float, float, float],
    forecast_hours: Iterable[int] | None = None,
    logger: logging.Logger | None = None,
    pause_seconds: float = 0.2,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> None:
    requests = _require_requests()
    steps = list(forecast_hours or build_gfs_steps(product_config))
    target_path.parent.mkdir(parents=True, exist_ok=True)
    with requests.Session() as session, target_path.open("wb") as handle, tqdm(
        steps, desc="NOAA GFS", unit="step"
    ) as progress:
        for forecast_hour in progress:
            progress.set_postfix_str(f"f{forecast_hour:03d}")
            if logger is not None:
                logger.info("noaa_request_start cycle=%s forecast_hour=%03d", cycle_time.isoformat(), forecast_hour)
            response = request_gfs_file(
                cycle_time=cycle_time,
                forecast_hour=forecast_hour,
                bbox=bbox,
                session=session,
                product_config=product_config,
            )
            handle.write(response.content)
            if logger is not None:
                logger.info("noaa_request_done cycle=%s forecast_hour=%03d bytes=%d", cycle_time.isoformat(), forecast_hour, len(response.content))
            if pause_seconds > 0:
                time.sleep(pause_seconds)


def download_forecast_grib(
    *,
    cycle_time: datetime,
    downloads_dir: Path,
    bbox: tuple[float, float, float, float],
    required_end: datetime | None = None,
    logger: logging.Logger | None = None,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> Path:
    target = build_grib_path(downloads_dir, cycle_time, product_config)
    steps = build_required_gfs_steps(cycle_time, required_end, product_config) if required_end is not None else build_gfs_steps(product_config)
    target.parent.mkdir(parents=True, exist_ok=True)
    if target.exists():
        messages = read_gfs_precipitation_messages(target)
        found_cycle = min(message.valid_time - timedelta(hours=message.step_hours) for message in messages)
        found_steps = {message.step_hours for message in messages}
        if found_cycle == cycle_time and set(steps).issubset(found_steps):
            if logger is not None:
                logger.info("noaa_download_reused cycle=%s path=%s", cycle_time.isoformat(), target)
            return target
        target.unlink()

    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target.name}.", suffix=".tmp", dir=target.parent
    )
    os.close(descriptor)
    temporary = Path(temporary_name)
    try:
        if logger is not None:
            logger.info("noaa_download_start cycle=%s bbox=%s steps=%s", cycle_time.isoformat(), bbox, steps)
        download_gfs_grib_to_path(
            temporary,
            cycle_time=cycle_time,
            bbox=bbox,
            forecast_hours=steps,
            logger=logger,
            product_config=product_config,
        )
        messages = read_gfs_precipitation_messages(temporary)
        found_cycle = min(message.valid_time - timedelta(hours=message.step_hours) for message in messages)
        found_steps = {message.step_hours for message in messages}
        if found_cycle != cycle_time or not set(steps).issubset(found_steps):
            raise ValueError("Downloaded GFS GRIB2 has an unexpected cycle or incomplete steps.")
        os.replace(temporary, target)
        if logger is not None:
            logger.info("noaa_download_done cycle=%s path=%s", cycle_time.isoformat(), target)
    finally:
        temporary.unlink(missing_ok=True)
    return target


def process_forecast_grib(
    grib_path: Path,
    *,
    cycle_time: datetime,
    assets_dir: Path,
    bbox: tuple[float, float, float, float],
    forecast_bbox: tuple[float, float, float, float] | None = None,
    buffer_fraction: float = FORECAST_BBOX_BUFFER_FRACTION,
    resolution_degrees: float,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> NormalizedForecastGrid:
    target = build_asset_path(assets_dir, cycle_time, product_config)
    target.parent.mkdir(parents=True, exist_ok=True)
    buffered_bbox = forecast_bbox or build_bbox_with_buffer(bbox, buffer_fraction=buffer_fraction)
    valid_from, valid_to = write_canonical_forecast_grid_from_grib(
        grib_path,
        target,
        cycle_time=cycle_time,
        bbox=buffered_bbox,
        model_bbox=bbox,
        buffer_fraction=buffer_fraction,
        resolution_degrees=resolution_degrees,
        timestep_hours=timestep_hours,
        product_config=product_config,
        messages_reader=read_gfs_precipitation_messages,
        accumulation_semantics="cumulative",
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
    cycle_time: datetime,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    downloads_dir: Path,
    logs_dir: Path,
    timestep_hours: int = 1,
    product_config: ForecastProductConfig = GFS_FORECAST_PRODUCT,
) -> NormalizedForecastGrid:
    forecast_bbox = build_bbox_with_buffer(bbox)
    logger = configure_run_logger("forecast_noaa", logs_dir / "forecast_noaa" / f"{build_execution_id(cycle_time)}.log", console=False)
    logger.info("noaa_cycle_start cycle=%s model_bbox=%s forecast_bbox=%s", cycle_time.isoformat(), bbox, forecast_bbox)
    grib_path = download_forecast_grib(
        cycle_time=cycle_time,
        downloads_dir=downloads_dir,
        bbox=forecast_bbox,
        logger=logger,
        product_config=product_config,
    )
    return process_forecast_grib(
        grib_path,
        cycle_time=cycle_time,
        assets_dir=downloads_dir,
        bbox=bbox,
        resolution_degrees=resolution_degrees,
        timestep_hours=timestep_hours,
        product_config=product_config,
    )
