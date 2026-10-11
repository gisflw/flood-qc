from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import xarray as xr

from mgb_ops.assets.model_outputs import validate_model_outputs_netcdf
from mgb_ops.assets.types import AnalysisWindow

SCENARIO_CACHE_DIRNAME = "forecast_scenarios"


@dataclass(frozen=True, slots=True)
class ScenarioCache:
    scenario_id: str
    label: str
    kind: str
    path: Path
    provider_code: str | None
    asset_id: str | None
    correction_id: int | None
    forecast_grid_path: Path | None = None
    reference_time: datetime | None = None
    window: AnalysisWindow | None = None


def scenario_cache_root(cache_dir: Path) -> Path:
    return Path(cache_dir) / SCENARIO_CACHE_DIRNAME


def read_scenario_cache(path: Path) -> ScenarioCache:
    """Read metadata from one validated published scenario cache."""
    source = Path(path)
    validation = validate_model_outputs_netcdf(source)
    with xr.open_dataset(source, decode_times=False) as dataset:
        attrs = dict(dataset.attrs)
    scenario_id = str(attrs.get("scenario_id") or "").strip()
    kind = str(attrs.get("scenario_kind") or "").strip()
    label = str(attrs.get("scenario_label") or scenario_id).strip()
    if not scenario_id or kind not in {"zero", "raw", "corrected"}:
        raise ValueError(f"Scenario cache has invalid scenario metadata: {source}")
    raw_correction_id = attrs.get("correction_id")
    raw_grid_path = str(attrs.get("forecast_grid_relative_path") or "").strip()
    raw_reference_time = str(attrs.get("reference_time") or "").strip()
    reference_time = (
        datetime.fromisoformat(raw_reference_time.replace("Z", "+00:00"))
        if raw_reference_time
        else None
    )
    root = next(
        (parent for parent in source.parents if parent.name == SCENARIO_CACHE_DIRNAME),
        source.parent,
    )
    forecast_grid_path = root / raw_grid_path if raw_grid_path else None
    if forecast_grid_path is not None and not forecast_grid_path.is_file():
        forecast_grid_path = None
    return ScenarioCache(
        scenario_id=scenario_id,
        label=label,
        kind=kind,
        path=source,
        provider_code=str(attrs["provider_code"]) if attrs.get("provider_code") else None,
        asset_id=str(attrs["source_forecast_asset_id"])
        if attrs.get("source_forecast_asset_id")
        else None,
        correction_id=int(raw_correction_id)
        if raw_correction_id not in (None, "")
        else None,
        forecast_grid_path=forecast_grid_path,
        reference_time=reference_time,
        window=validation["window"],
    )


def discover_latest_scenario_caches(cache_dir: Path) -> tuple[ScenarioCache, ...]:
    root = scenario_cache_root(cache_dir)
    if not root.is_dir():
        return ()

    caches = [read_scenario_cache(path) for path in sorted(root.glob("*.nc"))]
    order = {"zero": 0, "raw": 1, "corrected": 2}
    return tuple(sorted(caches, key=lambda item: (order[item.kind], item.scenario_id)))
