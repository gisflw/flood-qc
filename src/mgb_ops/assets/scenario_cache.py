from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import xarray as xr

from mgb_ops.assets.model_outputs import validate_model_outputs_netcdf

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


def scenario_cache_root(cache_dir: Path) -> Path:
    return Path(cache_dir) / SCENARIO_CACHE_DIRNAME


def discover_latest_scenario_caches(cache_dir: Path) -> tuple[ScenarioCache, ...]:
    root = scenario_cache_root(cache_dir)
    if not root.is_dir():
        return ()

    caches: list[ScenarioCache] = []
    for path in sorted(root.glob("*.nc")):
        validate_model_outputs_netcdf(path)
        with xr.open_dataset(path, decode_times=False) as dataset:
            attrs = dict(dataset.attrs)
        scenario_id = str(attrs.get("scenario_id") or "").strip()
        kind = str(attrs.get("scenario_kind") or "").strip()
        label = str(attrs.get("scenario_label") or scenario_id).strip()
        if not scenario_id or kind not in {"zero", "raw", "corrected"}:
            raise ValueError(f"Scenario cache has invalid scenario metadata: {path}")
        raw_correction_id = attrs.get("correction_id")
        caches.append(
            ScenarioCache(
                scenario_id=scenario_id,
                label=label,
                kind=kind,
                path=path,
                provider_code=str(attrs["provider_code"]) if attrs.get("provider_code") else None,
                asset_id=str(attrs["source_forecast_asset_id"])
                if attrs.get("source_forecast_asset_id")
                else None,
                correction_id=int(raw_correction_id)
                if raw_correction_id not in (None, "")
                else None,
            )
        )
    order = {"zero": 0, "raw": 1, "corrected": 2}
    return tuple(sorted(caches, key=lambda item: (order[item.kind], item.scenario_id)))
