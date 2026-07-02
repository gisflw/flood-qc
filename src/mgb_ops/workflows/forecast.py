from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

from mgb_ops.adapters import DEFAULT_FORECAST_ADAPTER, ForecastAdapter
from mgb_ops.assets.spatial_grid import SPATIAL_GRID_FORMAT
from mgb_ops.common.models import DataState, RasterAsset, RunMetadata
from mgb_ops.common.time_utils import resolve_reference_time
from mgb_ops.assets.forecast_registry import build_relative_asset_path, register_forecast_asset


@dataclass(frozen=True, slots=True)
class ForecastGridSummary:
    run_id: str
    asset_id: str
    asset_path: Path
    valid_from: datetime
    valid_to: datetime


def ingest_forecast_grids(
    database_path: Path,
    *,
    reference_time: datetime,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    downloads_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
    timestep_hours: int = 1,
    adapter: ForecastAdapter = DEFAULT_FORECAST_ADAPTER,
) -> ForecastGridSummary:
    if not Path(database_path).exists():
        raise FileNotFoundError(f"History database not found: {database_path}")

    normalized = adapter.store_grid(
        reference_time=reference_time,
        bbox=bbox,
        resolution_degrees=resolution_degrees,
        downloads_dir=downloads_dir,
        logs_dir=logs_dir,
        timestep_hours=timestep_hours,
    )
    product_config = adapter.product_config
    asset_id = adapter.asset_id(normalized.cycle_time)
    metadata = {
        "provider": product_config.provider_code,
        "model": product_config.model,
        "product_type": product_config.product_type,
        "resolution": product_config.resolution,
        "param": product_config.param,
        "reference_time": reference_time.isoformat(timespec="seconds"),
        "source_cycle_time": normalized.cycle_time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "cycle_time": normalized.cycle_time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "source_format": "GRIB2",
        "variable": "precipitation",
        "type": "forecast",
        "source": "resampled_from_grid",
        "providers": [product_config.provider_code],
        "bbox": list(normalized.bbox),
        "resolution_degrees": resolution_degrees,
        "timestep_hours": timestep_hours,
    }
    asset = register_forecast_asset(
        database_path,
        asset_id=asset_id,
        asset_kind=product_config.asset_kind,
        format=SPATIAL_GRID_FORMAT,
        path=normalized.asset_path,
        asset_base_dir=asset_base_dir,
        provider_code=product_config.provider_code,
        valid_from=normalized.valid_from,
        valid_to=normalized.valid_to,
        metadata=metadata,
    )
    return ForecastGridSummary(
        run_id=normalized.run_id,
        asset_id=str(asset["asset_id"]),
        asset_path=normalized.asset_path,
        valid_from=normalized.valid_from,
        valid_to=normalized.valid_to,
    )


def collect_forecast_grids(
    run: RunMetadata,
    *,
    history_db: Path,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    downloads_dir: Path,
    logs_dir: Path,
    asset_base_dir: Path,
    timestep_hours: int = 1,
) -> list[RasterAsset]:
    reference_time = resolve_reference_time(run.reference_time)
    summary = ingest_forecast_grids(
        history_db,
        reference_time=reference_time,
        bbox=bbox,
        resolution_degrees=resolution_degrees,
        downloads_dir=downloads_dir,
        logs_dir=logs_dir,
        asset_base_dir=asset_base_dir,
        timestep_hours=timestep_hours,
    )
    return [
        RasterAsset(
            name=summary.asset_id,
            relative_path=build_relative_asset_path(summary.asset_path, asset_base_dir=asset_base_dir),
            format=SPATIAL_GRID_FORMAT,
            state=DataState.RAW,
            crs="EPSG:4326",
        )
    ]
