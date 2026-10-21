from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
import hashlib
import json
from pathlib import Path
from typing import Iterable

from mgb_ops.adapters import (
    DEFAULT_FORECAST_ADAPTER,
    ForecastCycleUnavailableError,
    ForecastAdapter,
    get_forecast_adapter,
    list_forecast_adapter_codes,
)
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
    DEFAULT_FORECAST_CYCLE_HOURS,
    iter_forecast_cycle_candidates,
    resolve_forecast_cycle,
    resolve_reference_time,
)
from mgb_ops.assets.forecast_registry import build_relative_asset_path, register_forecast_asset
from mgb_ops.assets.forecast_registry import list_forecast_assets
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.config.runtime import RuntimeContext
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


def list_enabled_forecast_providers(database_path: Path) -> tuple[str, ...]:
    """Resolve enabled forecast providers from the persistent provider registry."""
    with HistoryRepository(Path(database_path)) as repository:
        rows = repository.list_enabled_providers("forecast")
    providers = tuple(str(row["provider_code"]).strip().lower() for row in rows)
    supported = set(list_forecast_adapter_codes())
    unsupported = sorted(set(providers).difference(supported))
    if unsupported:
        raise ValueError(
            "Enabled forecast providers have no registered adapter: "
            f"{unsupported}."
        )
    return providers


def _forecast_settings(settings: dict[str, object]) -> tuple[int, float]:
    forecast_settings = settings.get("forecast", {})
    if not isinstance(forecast_settings, dict):
        raise ValueError("forecast settings must be a mapping.")
    lookback_cycles = int(forecast_settings.get("lookback_cycles", 12))
    buffer_fraction = float(forecast_settings.get("buffer_fraction", FORECAST_BBOX_BUFFER_FRACTION))
    return lookback_cycles, buffer_fraction


def _metadata_value(grid, key: str) -> str:
    value = str(grid.metadata.get(key, "")).strip()
    if not value:
        raise ValueError(f"Forecast NetCDF is missing required metadata {key!r}.")
    return value


def _has_complete_native_schedule(grid, *, adapter: ForecastAdapter, cycle_time: datetime) -> bool:
    schedule = tuple(int(step) for step in adapter.product_config.step_schedule)
    if not schedule:
        return False
    cycle = cycle_time.replace(tzinfo=timezone.utc) if cycle_time.tzinfo is None else cycle_time.astimezone(timezone.utc)
    expected_hours = tuple(step for step in schedule if step > 0)
    expected_bounds = tuple(
        (cycle + timedelta(hours=start), cycle + timedelta(hours=end))
        for start, end in zip((0, *expected_hours[:-1]), expected_hours)
    )
    return grid.time_bounds_utc == expected_bounds


def _validate_reusable_grid(
    path: Path,
    *,
    provider: str,
    adapter: ForecastAdapter,
    cycle_time: datetime,
    bbox: tuple[float, float, float, float],
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
        return (
            grid.grid_type == "forecast"
            and grid.variable == "precipitation"
            and grid.providers == (provider,)
            and _metadata_value(grid, "provider") == provider
            and _metadata_value(grid, "model") == adapter.product_config.model
            and _metadata_value(grid, "product_type") == adapter.product_config.product_type
            and _metadata_value(grid, "source_format") == "GRIB2"
            and _metadata_value(grid, "source_resolution") == adapter.product_config.resolution
            and _metadata_value(grid, "source_parameter") == adapter.product_config.param
            and _metadata_value(grid, "source_cycle_time") == expected_cycle
            and grid.timestep_hours is None
            and grid.source == "cropped_from_native_grid"
            and buffer_fraction == expected_buffer_fraction
            and model_bbox == list(bbox)
            and stored_buffered_bbox == list(buffered_bbox)
            and requested_bbox == list(buffered_bbox)
            and _has_complete_native_schedule(grid, adapter=adapter, cycle_time=cycle_time)
        )
    except ModuleNotFoundError:
        raise
    except Exception:
        return False


class ForecastProviderBatchError(RuntimeError):
    def __init__(
        self,
        failures: dict[str, Exception],
        summary: ForecastDownloadSummary | None = None,
    ) -> None:
        self.failures = dict(failures)
        self.summary = summary
        details = "; ".join(
            f"{provider}: {type(error).__name__}: {error}"
            for provider, error in sorted(failures.items())
        )
        super().__init__(f"Forecast provider update failed: {details}")



def _cycle_log_path(context: RuntimeContext, provider: str, cycle: datetime) -> Path:
    return context.paths.logs_dir / f"forecast_{provider}" / f"{cycle:%Y%m%dT%H%M%S}.log"


def _download_forecast_provider(
    context: RuntimeContext,
    provider: str,
    *,
    assets,
    reference: datetime,
    lookback_cycles: int,
    bbox: tuple[float, ...],
    forecast_bbox: tuple[float, ...],
    resolution: float,
    timestep: int,
    buffer_fraction: float,
) -> tuple[str, Path, Path | None]:
    adapter = get_forecast_adapter(provider)
    preflight = getattr(adapter, "preflight_dependencies", None)
    if callable(preflight):
        preflight()

    cycle_hours = getattr(adapter, "forecast_cycle_hours", DEFAULT_FORECAST_CYCLE_HOURS)
    target_cycle = resolve_forecast_cycle(reference, cycle_hours=cycle_hours)
    attempted_cycles: list[str] = []
    last_error: Exception | None = None
    for cycle in iter_forecast_cycle_candidates(
        target_cycle, lookback_cycles=lookback_cycles, cycle_hours=cycle_hours
    ):
        cycle_id = f"{cycle:%Y%m%dT%H%MZ}"
        attempted_cycles.append(cycle_id)
        attempt_number = len(attempted_cycles)
        logger = configure_run_logger(
            f"forecast_{provider}",
            _cycle_log_path(context, provider, cycle),
            console=False,
        )
        logger.info(
            "forecast_cycle_start provider=%s cycle=%s attempt=%d",
            provider,
            cycle_id,
            attempt_number,
        )
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
                expected_buffer_fraction=buffer_fraction,
            ):
                logger.info("forecast_cycle_reused provider=%s cycle=%s", provider, cycle_id)
                return "reused", path, None

        download_kwargs: dict[str, object] = {
            "cycle_time": cycle,
            "downloads_dir": context.paths.downloads_dir,
            "bbox": forecast_bbox if provider == "noaa" else bbox,
        }
        if provider == "noaa":
            download_kwargs["logger"] = logger
        try:
            grib = adapter.download_grib(**download_kwargs)
        except ForecastCycleUnavailableError as exc:
            logger.exception(
                "forecast_acquisition_unavailable provider=%s cycle=%s attempt=%d",
                provider, cycle_id, attempt_number,
            )
            last_error = exc
            continue
        except Exception:
            logger.exception(
                "forecast_acquisition_failed provider=%s cycle=%s attempt=%d",
                provider, cycle_id, attempt_number,
            )
            raise

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
            logger.exception("forecast_conversion_failed provider=%s cycle=%s", provider, cycle_id)
            raise

        published_path = Path(normalized.asset_path)
        logger.info("forecast_cycle_done provider=%s cycle=%s", provider, cycle_id)
        return "produced", published_path, grib

    attempted = ", ".join(attempted_cycles) or "none"
    detail = (
        f"{type(last_error).__name__}: {last_error}"
        if last_error is not None
        else "no eligible forecast cycle was available"
    )
    raise RuntimeError(
        f"Forecast lookback exhausted for provider {provider!r}; attempted cycles: {attempted}; "
        f"final underlying exception: {detail}"
    ) from last_error


def download_forecast_data(
    context: RuntimeContext,
    providers: str | Iterable[str] | None = None,
    *,
    reference_time: datetime | None = None,
) -> ForecastDownloadSummary:
    settings = context.settings
    lookback_cycles, buffer_fraction = _forecast_settings(settings)
    provider_selection = (
        providers
        if providers is not None
        else list_enabled_forecast_providers(context.paths.history_db)
    )
    provider_codes = normalize_provider_codes(provider_selection, get_forecast_adapter)
    if not provider_codes:
        raise ValueError("No enabled forecast providers were found in the provider registry.")
    reference = reference_time or resolve_reference_time(str(settings["run"]["reference_time"]))
    timestep = int(settings["run"]["timestep_hours"])
    bbox_value = settings["spatial_grid"]["bbox"]
    if bbox_value is None:
        raise ValueError("spatial_grid.bbox must be configured.")
    bbox = tuple(float(value) for value in bbox_value)
    resolution = float(settings["spatial_grid"]["resolution_degrees"])
    forecast_bbox = build_bbox_with_buffer(bbox, buffer_fraction=buffer_fraction)
    reused: list[Path] = []
    produced: list[Path] = []
    raw: list[Path] = []
    assets = list_forecast_assets(
        context.paths.history_db, workspace_path=context.paths.workspace
    )
    failures: dict[str, Exception] = {}
    for provider in provider_codes:
        try:
            status, path, grib = _download_forecast_provider(
                context,
                provider,
                assets=assets,
                reference=reference,
                lookback_cycles=lookback_cycles,
                bbox=bbox,
                forecast_bbox=forecast_bbox,
                resolution=resolution,
                timestep=timestep,
                buffer_fraction=buffer_fraction,
            )
            if status == "reused":
                reused.append(path)
            else:
                produced.append(path)
                if grib is not None:
                    raw.append(grib)
        except Exception as exc:
            failures[provider] = exc
    if failures:
        if len(provider_codes) == 1:
            raise next(iter(failures.values()))
        raise ForecastProviderBatchError(failures)
    return ForecastDownloadSummary(tuple(reused), tuple(produced), tuple(raw))


def update_enabled_forecast_providers(
    context: RuntimeContext,
    *,
    reference_time: datetime | None = None,
) -> ForecastDownloadSummary:
    """Download and register every enabled forecast provider, then report failures."""
    providers = list_enabled_forecast_providers(context.paths.history_db)
    reused: list[Path] = []
    produced: list[Path] = []
    raw: list[Path] = []
    failures: dict[str, Exception] = {}
    for provider in providers:
        try:
            summary = download_forecast_data(
                context,
                providers=(provider,),
                reference_time=reference_time,
            )
            reused.extend(summary.reused_asset_paths)
            raw.extend(summary.raw_grib_paths)
            produced.extend(summary.new_asset_paths)
            for path in (*summary.reused_asset_paths, *summary.new_asset_paths):
                ingest_forecast_asset(context, path)
        except Exception as exc:
            failures[provider] = exc
    summary = ForecastDownloadSummary(tuple(reused), tuple(produced), tuple(raw))
    if failures:
        raise ForecastProviderBatchError(failures, summary)
    return summary


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
    if (
        buffered_bbox != list(build_bbox_with_buffer(tuple(model_bbox), buffer_fraction=buffer_fraction))
    ):
        raise ValueError("Forecast asset bbox buffer metadata is inconsistent.")
    cycle_text = _metadata_value(grid, "source_cycle_time")
    cycle = datetime.fromisoformat(cycle_text.replace("Z", "+00:00")).astimezone(timezone.utc).replace(tzinfo=None)
    asset_id = adapter.asset_id(cycle)
    checksum = hashlib.sha256(target.read_bytes()).hexdigest()
    valid_from = grid.time_bounds_utc[0][0].isoformat(timespec="seconds")
    valid_to = grid.time_bounds_utc[-1][1].isoformat(timespec="seconds")
    metadata = json.loads(
        json.dumps(
            dict(grid.metadata),
            sort_keys=True,
            ensure_ascii=True,
            default=lambda value: value.item() if hasattr(value, "item") else str(value),
        )
    )
    metadata["cycle_time"] = cycle_text
    with HistoryRepository(context.paths.history_db) as repository:
        return repository.upsert_asset(
            asset_id=asset_id,
            asset_kind=SPATIAL_GRID_ASSET_KIND,
            format=SPATIAL_GRID_FORMAT,
            relative_path=relative_path,
            provider_code=provider,
            checksum=checksum,
            valid_from=valid_from,
            valid_to=valid_to,
            metadata=metadata,
        )


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
