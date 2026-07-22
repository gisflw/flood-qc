"""Canonical persistence contracts, schemas, repositories, validation, and I/O."""

from mgb_ops.assets.scenario_cache import (
    ScenarioCache,
    discover_latest_scenario_caches,
)

__all__ = list(globals().get("__all__", [])) + [
    "ScenarioCache",
    "discover_latest_scenario_caches",
]
