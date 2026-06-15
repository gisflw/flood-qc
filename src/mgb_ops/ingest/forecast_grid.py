from __future__ import annotations

import logging
import tempfile
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
import sys

import numpy as np

from mgb_ops.common.models import DataState, RasterAsset, RunMetadata
from mgb_ops.common.time_utils import TIMEZONE, resolve_reference_time
from mgb_ops.storage.history_repository import HistoryRepository

LOGGER_NAME = "floodqc.ingest.forecast_grid"
ECMWF_ASSET_KIND = "forecast_grib_rs_buffered"
ECMWF_MODEL = "ifs"
ECMWF_PRODUCT_TYPE = "fc"
ECMWF_RESOLUTION = "0p25"
ECMWF_PARAM = "tp"
RS_BBOX = (-60.0, -35.0, -48.0, -26.0)  # west, south, east, north
BUFFER_FRACTION = 1.0


@dataclass(frozen=True, slots=True)
class ForecastProductConfig:
    provider_code: str
    asset_kind: str
    model: str
    product_type: str
    resolution: str
    param: str
    bbox: tuple[float, float, float, float]
    buffer_fraction: float
    step_schedule: tuple[int, ...]


ECMWF_FORECAST_PRODUCT = ForecastProductConfig(
    provider_code="ecmwf",
    asset_kind=ECMWF_ASSET_KIND,
    model=ECMWF_MODEL,
    product_type=ECMWF_PRODUCT_TYPE,
    resolution=ECMWF_RESOLUTION,
    param=ECMWF_PARAM,
    bbox=RS_BBOX,
    buffer_fraction=BUFFER_FRACTION,
    step_schedule=tuple([hour for hour in range(0, 144 + 3, 3)] + [hour for hour in range(150, 360 + 6, 6)]),
)


@dataclass(frozen=True, slots=True)
class ForecastGridSummary:
    run_id: str
    asset_id: str
    asset_path: Path
    valid_from: datetime
    valid_to: datetime


@dataclass(frozen=True, slots=True)
class TpGribMessage:
    valid_time: datetime
    step_hours: int
    latitudes: np.ndarray
    longitudes: np.ndarray
    values_mm: np.ndarray


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


def _require_eccodes():
    try:
        import eccodes
    except ModuleNotFoundError as exc:
        raise ModuleNotFoundError(
            "Missing dependency for GRIB2 reading/writing: install `eccodes` in the operational environment."
        ) from exc
    return eccodes


def _normalize_longitudes(values: np.ndarray) -> np.ndarray:
    normalized = np.asarray(values, dtype=np.float64).copy()
    normalized[normalized > 180.0] -= 360.0
    return normalized


def build_rs_bbox_with_buffer(
    *,
    buffer_fraction: float = BUFFER_FRACTION,
    bbox: tuple[float, float, float, float] = RS_BBOX,
) -> tuple[float, float, float, float]:
    west, south, east, north = bbox
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
    interim_root: Path,
    cycle_time: datetime,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> Path:
    directory = interim_root / product_config.provider_code
    file_name = f"{product_config.product_type}_{cycle_time.strftime('%Y-%m-%d')}_{cycle_time:%H}_{product_config.model.upper()}_rsbuf.grib2"
    return directory / file_name


def build_asset_id(cycle_time: datetime, product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT) -> str:
    return f"{product_config.provider_code}.{product_config.model}.{product_config.product_type}.{cycle_time.strftime('%Y%m%dT%H%M%SZ')}.rsbuf"


def _build_grid_arrays(gid) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    eccodes = _require_eccodes()
    grid_type = eccodes.codes_get(gid, "gridType")
    if grid_type != "regular_ll":
        raise ValueError(f"Unsupported GRIB gridType={grid_type!r}; expected 'regular_ll'.")

    ni = int(eccodes.codes_get_long(gid, "Ni"))
    nj = int(eccodes.codes_get_long(gid, "Nj"))
    if ni < 1 or nj < 1:
        raise ValueError(f"Invalid GRIB shape Ni={ni} Nj={nj}.")

    values = np.asarray(eccodes.codes_get_array(gid, "values"), dtype=np.float64).reshape(nj, ni)
    latitude_grid = np.asarray(eccodes.codes_get_array(gid, "latitudes"), dtype=np.float64).reshape(nj, ni)
    longitude_grid = _normalize_longitudes(
        np.asarray(eccodes.codes_get_array(gid, "longitudes"), dtype=np.float64)
    ).reshape(nj, ni)

    latitudes = latitude_grid[:, 0].copy()
    longitudes = longitude_grid[0, :].copy()

    if not np.allclose(latitude_grid, latitudes[:, None]):
        raise ValueError("Unexpected latitude layout in GRIB regular_ll grid.")
    if not np.allclose(longitude_grid, longitudes[None, :]):
        raise ValueError("Unexpected longitude layout in GRIB regular_ll grid.")

    lon_sort_idx = np.argsort(longitudes)
    longitudes = longitudes[lon_sort_idx]
    values = values[:, lon_sort_idx]

    return latitudes, longitudes, values


def _set_cropped_grid(gid, *, latitudes: np.ndarray, longitudes: np.ndarray, values: np.ndarray) -> None:
    eccodes = _require_eccodes()
    if latitudes.size < 1 or longitudes.size < 1:
        raise ValueError("Cropped GRIB produced an empty grid.")

    grid_values = np.asarray(values, dtype=np.float64)
    lat_vec = np.asarray(latitudes, dtype=np.float64)
    lon_vec = np.asarray(longitudes, dtype=np.float64)

    if lat_vec[0] < lat_vec[-1]:
        lat_vec = lat_vec[::-1]
        grid_values = grid_values[::-1, :]

    eccodes.codes_set_long(gid, "Ni", int(lon_vec.size))
    eccodes.codes_set_long(gid, "Nj", int(lat_vec.size))
    eccodes.codes_set(gid, "latitudeOfFirstGridPointInDegrees", float(lat_vec[0]))
    eccodes.codes_set(gid, "latitudeOfLastGridPointInDegrees", float(lat_vec[-1]))
    eccodes.codes_set(gid, "longitudeOfFirstGridPointInDegrees", float(lon_vec[0]))
    eccodes.codes_set(gid, "longitudeOfLastGridPointInDegrees", float(lon_vec[-1]))
    if lon_vec.size > 1:
        eccodes.codes_set(gid, "iDirectionIncrementInDegrees", float(abs(lon_vec[1] - lon_vec[0])))
    if lat_vec.size > 1:
        eccodes.codes_set(gid, "jDirectionIncrementInDegrees", float(abs(lat_vec[1] - lat_vec[0])))
    eccodes.codes_set_long(gid, "iScansNegatively", 0)
    eccodes.codes_set_long(gid, "jScansPositively", 0)
    eccodes.codes_set_array(gid, "values", grid_values.reshape(-1))


def crop_grib_to_bbox(
    source_path: Path,
    target_path: Path,
    *,
    bbox: tuple[float, float, float, float],
) -> None:
    eccodes = _require_eccodes()
    west, south, east, north = bbox
    target_path.parent.mkdir(parents=True, exist_ok=True)

    wrote_any = False
    with source_path.open("rb") as src_handle, target_path.open("wb") as dst_handle:
        while True:
            gid = eccodes.codes_grib_new_from_file(src_handle)
            if gid is None:
                break
            try:
                latitudes, longitudes, values = _build_grid_arrays(gid)
                lat_mask = (latitudes >= south) & (latitudes <= north)
                lon_mask = (longitudes >= west) & (longitudes <= east)
                if not lat_mask.any() or not lon_mask.any():
                    raise ValueError(
                        f"Requested bbox {bbox} does not intersect GRIB grid in {source_path}."
                    )

                cropped_latitudes = latitudes[lat_mask]
                cropped_longitudes = longitudes[lon_mask]
                cropped_values = values[np.ix_(lat_mask, lon_mask)]
                _set_cropped_grid(
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


def read_tp_grib_messages(grib_path: Path) -> list[TpGribMessage]:
    eccodes = _require_eccodes()
    messages: list[TpGribMessage] = []

    with grib_path.open("rb") as handle:
        while True:
            gid = eccodes.codes_grib_new_from_file(handle)
            if gid is None:
                break
            try:
                short_name = str(eccodes.codes_get(gid, "shortName"))
                if short_name != ECMWF_PARAM:
                    continue

                valid_date = int(eccodes.codes_get_long(gid, "validityDate"))
                valid_time = int(eccodes.codes_get_long(gid, "validityTime"))
                step_hours = int(eccodes.codes_get_long(gid, "endStep"))
                valid_dt = datetime.strptime(f"{valid_date:08d}{valid_time:04d}", "%Y%m%d%H%M")
                latitudes, longitudes, values = _build_grid_arrays(gid)
                messages.append(
                    TpGribMessage(
                        valid_time=valid_dt,
                        step_hours=step_hours,
                        latitudes=latitudes,
                        longitudes=longitudes,
                        values_mm=values * 1000.0,
                    )
                )
            finally:
                eccodes.codes_release(gid)

    if not messages:
        raise ValueError(f"No '{ECMWF_PARAM}' messages found in {grib_path}.")
    return sorted(messages, key=lambda item: (item.valid_time, item.step_hours))


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
    interim_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
    product_config: ForecastProductConfig = ECMWF_FORECAST_PRODUCT,
) -> ForecastGridSummary:
    if not Path(database_path).exists():
        raise FileNotFoundError(f"History database not found: {database_path}")

    execution_id = build_execution_id(reference_time)
    logger = configure_run_logger(logs_dir / script_stem() / f"{execution_id}.log")
    cycle_time = build_ecmwf_cycle(reference_time)
    target_path = build_output_path(interim_dir, cycle_time, product_config)
    bbox = build_rs_bbox_with_buffer(
        buffer_fraction=product_config.buffer_fraction,
        bbox=product_config.bbox,
    )

    logger.info(
        "forecast_grid_start history_db=%s cycle_time=%s bbox=%s target=%s",
        database_path,
        cycle_time.strftime("%Y-%m-%dT%H:%M:%S"),
        bbox,
        target_path,
    )

    with tempfile.TemporaryDirectory(prefix="ecmwf_download_") as temp_dir_name:
        temp_dir = Path(temp_dir_name)
        temp_grib_path = temp_dir / "download.grib2"
        download_ecmwf_grib_to_path(temp_grib_path, reference_time=reference_time, product_config=product_config)
        crop_grib_to_bbox(temp_grib_path, target_path, bbox=bbox)

    valid_from, valid_to = extract_valid_time_bounds(target_path)
    relative_path = build_relative_asset_path(target_path, asset_base_dir=asset_base_dir)
    metadata = {
        "model": product_config.model,
        "product_type": product_config.product_type,
        "resolution": product_config.resolution,
        "reference_time": reference_time.isoformat(timespec="seconds"),
        "cycle_time": cycle_time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "bbox": list(bbox),
        "buffer_fraction": product_config.buffer_fraction,
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
    interim_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
) -> list[RasterAsset]:
    reference_time = resolve_reference_time(run.reference_time)
    summary = ingest_forecast_grids(
        history_db,
        reference_time=reference_time,
        interim_dir=interim_dir,
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
