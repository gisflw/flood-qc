from __future__ import annotations

import logging
import tempfile
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
import sys

import numpy as np

from mgb_ops.common.grib2 import (
    TpGribMessage,
    build_grid_arrays,
    read_tp_grib_messages,
    require_eccodes,
    set_regular_ll_grid,
)
from mgb_ops.common.models import DataState, RasterAsset, RunMetadata
from mgb_ops.common.time_utils import TIMEZONE, resolve_reference_time
from mgb_ops.storage.history_repository import HistoryRepository

LOGGER_NAME = "adapters.forecast_ecmwf"
ECMWF_ASSET_KIND = "forecast_grib_buffered"
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


ECMWF_FORECAST_PRODUCT = ForecastProductConfig(
    provider_code="ecmwf",
    asset_kind=ECMWF_ASSET_KIND,
    model=ECMWF_MODEL,
    product_type=ECMWF_PRODUCT_TYPE,
    resolution=ECMWF_RESOLUTION,
    param=ECMWF_PARAM,
    step_schedule=tuple([hour for hour in range(0, 144 + 3, 3)] + [hour for hour in range(150, 360 + 6, 6)]),
)


@dataclass(frozen=True, slots=True)
class ForecastGridSummary:
    run_id: str
    asset_id: str
    asset_path: Path
    valid_from: datetime
    valid_to: datetime


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


def build_bbox_with_buffer(
    bbox: tuple[float, float, float, float],
    *,
    buffer_fraction: float,
) -> tuple[float, float, float, float]:
    west, south, east, north = bbox
    if west >= east or south >= north:
        raise ValueError("bbox must satisfy west < east and south < north.")
    if buffer_fraction < 0:
        raise ValueError("buffer_fraction must be >= 0.")
    width = east - west
    height = north - south
    return (
        west - width * buffer_fraction,
        south - height * buffer_fraction,
        east + width * buffer_fraction,
        north + height * buffer_fraction,
    )


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
    file_name = f"{product_config.product_type}_{cycle_time.strftime('%Y-%m-%d')}_{cycle_time:%H}_{product_config.model.upper()}_buffered.grib2"
    return directory / file_name


def build_asset_id(cycle_time: datetime, product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT) -> str:
    return f"{product_config.provider_code}.{product_config.model}.{product_config.product_type}.{cycle_time.strftime('%Y%m%dT%H%M%SZ')}.buffered"


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




def build_relative_asset_path(path: Path, *, asset_base_dir: Path) -> str:
    resolved_path = Path(path).resolve()
    resolved_base = Path(asset_base_dir).resolve()
    try:
        return resolved_path.relative_to(resolved_base).as_posix()
    except ValueError:
        return Path(path).as_posix()


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


def ingest_forecast_grids(
    database_path: Path,
    *,
    reference_time: datetime,
    bbox: tuple[float, float, float, float],
    buffer_fraction: float,
    downloads_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> ForecastGridSummary:
    if not Path(database_path).exists():
        raise FileNotFoundError(f"History database not found: {database_path}")

    execution_id = build_execution_id(reference_time)
    logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")
    cycle_time = build_ecmwf_cycle(reference_time)
    target_path = build_output_path(downloads_dir, cycle_time, product_config)
    buffered_bbox = build_bbox_with_buffer(
        bbox,
        buffer_fraction=buffer_fraction,
    )

    logger.info(
        "forecast_grid_start history_db=%s cycle_time=%s bbox=%s target=%s",
        database_path,
        cycle_time.strftime("%Y-%m-%dT%H:%M:%S"),
        buffered_bbox,
        target_path,
    )

    with tempfile.TemporaryDirectory(prefix="ecmwf_download_") as temp_dir_name:
        temp_dir = Path(temp_dir_name)
        temp_grib_path = temp_dir / "download.grib2"
        download_ecmwf_grib_to_path(temp_grib_path, reference_time=reference_time, product_config=product_config)
        crop_grib_to_bbox(temp_grib_path, target_path, bbox=buffered_bbox)

    valid_from, valid_to = extract_valid_time_bounds(target_path)
    relative_path = build_relative_asset_path(target_path, asset_base_dir=asset_base_dir)
    metadata = {
        "model": product_config.model,
        "product_type": product_config.product_type,
        "resolution": product_config.resolution,
        "reference_time": reference_time.isoformat(timespec="seconds"),
        "cycle_time": cycle_time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "bbox": list(buffered_bbox),
        "source_bbox": list(bbox),
        "buffer_fraction": buffer_fraction,
    }

    with HistoryRepository(database_path) as repository:
        asset = repository.upsert_asset(
            asset_id=build_asset_id(cycle_time, product_config),
            asset_kind=product_config.asset_kind,
            format="GRIB2",
            relative_path=relative_path,
            provider_code=product_config.provider_code,
            valid_from=valid_from.isoformat(timespec="seconds"),
            valid_to=valid_to.isoformat(timespec="seconds"),
            metadata=metadata,
        )

    logger.info(
        "forecast_grid_done asset_id=%s relative_path=%s valid_from=%s valid_to=%s",
        asset["asset_id"],
        asset["relative_path"],
        asset["valid_from"],
        asset["valid_to"],
    )
    return ForecastGridSummary(
        run_id=execution_id,
        asset_id=str(asset["asset_id"]),
        asset_path=target_path,
        valid_from=valid_from,
        valid_to=valid_to,
    )


def collect_forecast_grids(
    run: RunMetadata,
    *,
    history_db: Path,
    bbox: tuple[float, float, float, float],
    buffer_fraction: float,
    downloads_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
) -> list[RasterAsset]:
    reference_time = resolve_reference_time(run.reference_time)
    summary = ingest_forecast_grids(
        history_db,
        reference_time=reference_time,
        bbox=bbox,
        buffer_fraction=buffer_fraction,
        downloads_dir=downloads_dir,
        logs_dir=logs_dir,
        asset_base_dir=asset_base_dir,
    )
    return [
        RasterAsset(
            name=summary.asset_id,
            relative_path=build_relative_asset_path(summary.asset_path, asset_base_dir=asset_base_dir),
            format="GRIB2",
            state=DataState.RAW,
            crs="EPSG:4326",
        )
    ]
