from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
from typing import Iterable

from mgb_ops.adapters import DEFAULT_FORECAST_ADAPTER, ForecastAdapter, get_forecast_adapter
from mgb_ops.adapters.forecast_ecmwf import (
    FORECAST_BBOX_BUFFER_FRACTION,
    build_bbox_with_buffer,
)
from mgb_ops.assets.spatial_grid import (
    SPATIAL_GRID_ASSET_KIND,
    SPATIAL_GRID_FORMAT,
    read_spatial_grid,
)
from mgb_ops.assets.types import DataState, RasterAsset, RunMetadata
from mgb_ops.utils.time import (
    iter_forecast_cycle_candidates,
    resolve_forecast_cycle,
    resolve_reference_time,
)
from mgb_ops.assets.forecast_registry import build_relative_asset_path, register_forecast_asset
from mgb_ops.assets.forecast_registry import list_forecast_assets
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.utils.time import TIMEZONE
from mgb_ops.workflows._providers import normalize_provider_codes
from mgb_ops.utils.logging import configure_run_logger


@dataclass(frozen=True, slots=True)
class ForecastGridSummary:
    run_id: str
    asset_id: str
    asset_path: Path
    valid_from: datetime
    valid_to: datetime


@dataclass(frozen=True, slots=True)
class ForecastDownloadSummary:
    reused_asset_paths: tuple[Path, ...]
    new_asset_paths: tuple[Path, ...]
    raw_grib_paths: tuple[Path, ...]


def _forecast_settings(settings: dict[str, object]) -> tuple[str, int, float]:
    forecast_settings = settings.get("forecast", {})
    if not isinstance(forecast_settings, dict):
        raise ValueError("forecast settings must be a mapping.")
    provider = str(forecast_settings.get("provider", DEFAULT_FORECAST_ADAPTER.provider_code)).strip().lower()
    lookback_cycles = int(forecast_settings.get("lookback_cycles", 12))
    buffer_fraction = float(forecast_settings.get("buffer_fraction", FORECAST_BBOX_BUFFER_FRACTION))
    return provider, lookback_cycles, buffer_fraction


def _metadata_value(grid, key: str) -> str:
    value = str(grid.metadata.get(key, "")).strip()
    if not value:
        raise ValueError(f"Forecast NetCDF is missing required metadata {key!r}.")
    return value


def _validate_reusable_grid(
    path: Path,
    *,
    provider: str,
    adapter: ForecastAdapter,
    cycle_time: datetime,
    bbox: tuple[float, float, float, float],
    resolution: float,
    timestep_hours: int,
    required_start: datetime,
    required_end: datetime,
    expected_buffer_fraction: float,
) -> bool:
    try:
        grid = read_spatial_grid(path)
        expected_cycle = cycle_time.replace(tzinfo=timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
        buffered_bbox = build_bbox_with_buffer(bbox, buffer_fraction=expected_buffer_fraction)
        buffer_fraction = float(grid.metadata.get("buffer_fraction", -1))
        model_bbox = json.loads(str(grid.metadata["model_bbox"]))
        stored_buffered_bbox = json.loads(str(grid.metadata["buffered_bbox"]))
        requested_bbox = json.loads(str(grid.metadata["requested_bbox"]))
    except (KeyError, TypeError, ValueError, json.JSONDecodeError):
        return False
    return (
        grid.grid_type == "forecast"
        and grid.variable == "precipitation"
        and grid.providers == (provider,)
        and _metadata_value(grid, "provider") == provider
        and _metadata_value(grid, "model") == adapter.product_config.model
        and _metadata_value(grid, "product_type") == adapter.product_config.product_type
        and _metadata_value(grid, "source_cycle_time") == expected_cycle
        and grid.timestep_hours is None
        and grid.source == "cropped_from_native_grid"
        and buffer_fraction == expected_buffer_fraction
        and model_bbox == list(bbox)
        and stored_buffered_bbox == list(buffered_bbox)
        and requested_bbox == list(buffered_bbox)
        and grid.time_bounds_utc[0][0] <= required_start
        and grid.time_bounds_utc[-1][1] >= required_end
    )


def download_forecast_data(
    context: RuntimeContext,
    providers: str | Iterable[str] | None = None,
    *,
    reference_time: datetime | None = None,
) -> ForecastDownloadSummary:
    settings = context.settings
    configured_provider, lookback_cycles, buffer_fraction = _forecast_settings(settings)
    provider_codes = normalize_provider_codes(providers or configured_provider, get_forecast_adapter)
    reference = reference_time or resolve_reference_time(str(settings["run"]["reference_time"]))
    timestep = int(settings["run"]["timestep_hours"])
    bbox_value = settings["spatial_grid"]["bbox"]
    if bbox_value is None:
        raise ValueError("spatial_grid.bbox must be configured.")
    bbox = tuple(float(value) for value in bbox_value)
    resolution = float(settings["spatial_grid"]["resolution_degrees"])
    forecast_bbox = build_bbox_with_buffer(bbox, buffer_fraction=buffer_fraction)
    required_start_local = reference + timedelta(hours=timestep)
    required_end_local = required_start_local + timedelta(
        days=int(settings["mgb"]["forecast_horizon_days"])
    )
    required_start = required_start_local.replace(tzinfo=TIMEZONE).astimezone(timezone.utc)
    required_end = required_end_local.replace(tzinfo=TIMEZONE).astimezone(timezone.utc)
    reused: list[Path] = []
    produced: list[Path] = []
    raw: list[Path] = []
    assets = list_forecast_assets(
        context.paths.history_db, workspace_path=context.paths.workspace
    )
    for provider in provider_codes:
        adapter = get_forecast_adapter(provider)
        target_cycle = resolve_forecast_cycle(reference)
        last_error: Exception | None = None
        for cycle in iter_forecast_cycle_candidates(target_cycle, lookback_cycles=lookback_cycles):
            asset_id = adapter.asset_id(cycle)
            matches = assets[assets["asset_id"] == asset_id] if not assets.empty else assets
            if not matches.empty:
                path = Path(matches.iloc[0]["asset_path"])
                if not path.exists():
                    raise FileNotFoundError(f"Registered forecast asset is missing: {path}")
                if _validate_reusable_grid(
                    path,
                    provider=provider,
                    adapter=adapter,
                    cycle_time=cycle,
                    bbox=bbox,
                    resolution=resolution,
                    timestep_hours=timestep,
                    required_start=required_start,
                    required_end=required_end,
                    expected_buffer_fraction=buffer_fraction,
                ):
                    reused.append(path)
                    break
            logger = None
            download_kwargs: dict[str, object] = {
                "cycle_time": cycle,
                "downloads_dir": context.paths.downloads_dir,
                "bbox": forecast_bbox if provider == "noaa" else bbox,
            }
            if provider == "noaa":
                logger = configure_run_logger(
                    "forecast_noaa",
                    context.paths.logs_dir / "forecast_noaa" / f"{cycle:%Y%m%dT%H%M%S}.log",
                    console=False,
                )
                logger.info("noaa_cycle_start cycle=%s model_bbox=%s forecast_bbox=%s", cycle.isoformat(), bbox, forecast_bbox)
                download_kwargs["required_end"] = required_end
                download_kwargs["logger"] = logger
            try:
                grib = adapter.download_grib(**download_kwargs)
            except Exception as exc:
                if logger is not None:
                    logger.exception("noaa_acquisition_failed cycle=%s", cycle.isoformat())
                last_error = exc
                continue

            try:
                normalized = adapter.process_grib(
                    grib,
                    cycle_time=cycle,
                    assets_dir=context.paths.assets_dir,
                    bbox=bbox,
                    forecast_bbox=forecast_bbox,
                    buffer_fraction=buffer_fraction,
                    resolution_degrees=resolution,
                    timestep_hours=timestep,
                )
            except Exception:
                if logger is not None:
                    logger.exception("noaa_conversion_failed cycle=%s", cycle.isoformat())
                raise

            required_start_naive = required_start.replace(tzinfo=None)
            required_end_naive = required_end.replace(tzinfo=None)
            try:
                if (
                    normalized.valid_from > required_start_naive
                    or normalized.valid_to < required_end_naive
                ):
                    raise ValueError(
                        "Downloaded forecast asset does not cover the required forecast window."
                    )
            except Exception:
                if logger is not None:
                    logger.exception("noaa_validation_failed cycle=%s", cycle.isoformat())
                raise

            raw.append(grib)
            produced.append(normalized.asset_path)
            break
        else:
            raise RuntimeError(
                f"No usable forecast asset found for provider {provider!r} "
                f"within {lookback_cycles} cycle(s)."
            ) from last_error
    return ForecastDownloadSummary(tuple(reused), tuple(produced), tuple(raw))


def ingest_forecast_asset(context: RuntimeContext, path: Path) -> dict[str, object]:
    target = Path(path).resolve()
    assets_root = context.paths.assets_dir.resolve()
    try:
        relative_path = target.relative_to(context.paths.workspace.resolve()).as_posix()
        target.relative_to(assets_root)
    except ValueError as exc:
        raise ValueError(f"Forecast asset must be inside {assets_root}: {target}") from exc
    grid = read_spatial_grid(target)
    if grid.grid_type != "forecast" or grid.variable != "precipitation" or len(grid.providers) != 1:
        raise ValueError("Forecast asset must contain precipitation for exactly one provider.")
    provider = grid.providers[0]
    adapter = get_forecast_adapter(provider)
    if _metadata_value(grid, "provider") != provider:
        raise ValueError("Forecast provider metadata conflicts with providers.")
    expected_metadata = {
        "model": adapter.product_config.model,
        "product_type": adapter.product_config.product_type,
        "source_format": "GRIB2",
        "source_resolution": adapter.product_config.resolution,
        "source_parameter": adapter.product_config.param,
    }
    for key, expected in expected_metadata.items():
        if _metadata_value(grid, key) != expected:
            raise ValueError(
                f"Forecast metadata {key!r} conflicts with provider product identity."
            )
    try:
        buffer_fraction = float(grid.metadata["buffer_fraction"])
        model_bbox = json.loads(str(grid.metadata["model_bbox"]))
        buffered_bbox = json.loads(str(grid.metadata["buffered_bbox"]))
    except (KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        raise ValueError("Forecast asset is missing valid bbox buffer metadata.") from exc
    _, _, expected_buffer_fraction = _forecast_settings(context.settings)
    if (
        buffer_fraction != expected_buffer_fraction
        or buffered_bbox != list(build_bbox_with_buffer(tuple(model_bbox), buffer_fraction=expected_buffer_fraction))
    ):
        raise ValueError("Forecast asset bbox buffer metadata is inconsistent.")
    cycle_text = _metadata_value(grid, "source_cycle_time")
    cycle = datetime.fromisoformat(cycle_text.replace("Z", "+00:00")).astimezone(timezone.utc).replace(tzinfo=None)
    asset_id = adapter.asset_id(cycle)
    checksum = hashlib.sha256(target.read_bytes()).hexdigest()
    valid_from = grid.time_bounds_utc[0][0].isoformat(timespec="seconds")
    valid_to = grid.time_bounds_utc[-1][1].isoformat(timespec="seconds")
    metadata = dict(grid.metadata)
    metadata["cycle_time"] = cycle_text
    metadata_json = json.dumps(
        metadata,
        sort_keys=True,
        ensure_ascii=True,
        default=lambda value: value.item() if hasattr(value, "item") else str(value),
    )
    immutable = {
        "asset_id": asset_id,
        "asset_kind": SPATIAL_GRID_ASSET_KIND,
        "format": SPATIAL_GRID_FORMAT,
        "relative_path": relative_path,
        "provider_code": provider,
        "checksum": checksum,
        "valid_from": valid_from,
        "valid_to": valid_to,
        "metadata_json": metadata_json,
    }
    with HistoryRepository(context.paths.history_db) as repository:
        by_id = repository.get_asset_by_id(asset_id)
        by_path = repository.get_asset_by_relative_path(relative_path)
        existing = by_id or by_path
        if existing is not None:
            if any(str(existing[key]) != str(value) for key, value in immutable.items()):
                try:
                    existing_metadata = json.loads(str(existing["metadata_json"] or "{}"))
                    existing_buffer = float(existing_metadata.get("buffer_fraction", -1))
                except (TypeError, ValueError, json.JSONDecodeError):
                    existing_buffer = -1
                if existing_buffer == FORECAST_BBOX_BUFFER_FRACTION:
                    raise ValueError(f"Forecast asset conflicts with registered immutable metadata: {target}")
                if str(existing["asset_id"]) != asset_id or str(existing["provider_code"]) != provider:
                    raise ValueError(f"Obsolete forecast asset identity conflicts with replacement: {target}")
                try:
                    repository.connection.execute(
                        """
                        UPDATE asset
                        SET asset_kind = ?, format = ?, relative_path = ?, provider_code = ?,
                            checksum = ?, valid_from = ?, valid_to = ?, metadata_json = ?
                        WHERE asset_id = ?
                        """,
                        (
                            immutable["asset_kind"],
                            immutable["format"],
                            immutable["relative_path"],
                            immutable["provider_code"],
                            immutable["checksum"],
                            immutable["valid_from"],
                            immutable["valid_to"],
                            immutable["metadata_json"],
                            asset_id,
                        ),
                    )
                    repository.connection.commit()
                except Exception:
                    repository.connection.rollback()
                    raise
                replaced = repository.get_asset_by_id(asset_id)
                if replaced is None:
                    raise RuntimeError(f"Failed to replace obsolete forecast asset {asset_id}.")
                return replaced
            return existing
        try:
            repository.connection.execute(
                """
                INSERT INTO asset (
                    asset_id, asset_kind, format, relative_path, provider_code,
                    checksum, valid_from, valid_to, metadata_json
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                tuple(immutable.values()),
            )
            repository.connection.commit()
        except Exception:
            repository.connection.rollback()
            raise
        result = repository.get_asset_by_id(asset_id)
        if result is None:
            raise RuntimeError(f"Failed to register forecast asset {asset_id}.")
        return result


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
        cycle_time=resolve_forecast_cycle(reference_time),
        bbox=bbox,
        resolution_degrees=resolution_degrees,
        downloads_dir=downloads_dir,
        logs_dir=logs_dir,
        timestep_hours=timestep_hours,
    )
    product_config = adapter.product_config
    asset_id = adapter.asset_id(normalized.cycle_time)
    native_grid = read_spatial_grid(normalized.asset_path)
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
        "source": "cropped_from_native_grid",
        "providers": [product_config.provider_code],
        "bbox": json.loads(str(native_grid.metadata["bbox"])),
        "requested_bbox": list(bbox),
        "resolution_degrees": float(native_grid.metadata["resolution_degrees"]),
        "temporal_resolution": "irregular",
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
