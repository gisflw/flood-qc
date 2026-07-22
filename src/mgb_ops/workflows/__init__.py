"""Reusable orchestration workflows."""

from mgb_ops.workflows.observed import (
    ObservedDownloadSummary,
    download_observed_data,
    ingest_from_csv,
)
from mgb_ops.workflows.forecast import (
    ForecastDownloadSummary,
    ForecastProviderBatchError,
    download_forecast_data,
    ingest_forecast_asset,
    list_enabled_forecast_providers,
    update_enabled_forecast_providers,
)

__all__ = [
    "ForecastDownloadSummary",
    "ForecastProviderBatchError",
    "ObservedDownloadSummary",
    "download_forecast_data",
    "download_observed_data",
    "ingest_forecast_asset",
    "ingest_from_csv",
    "list_enabled_forecast_providers",
    "update_enabled_forecast_providers",
]

from mgb_ops.workflows.scenarios import ForecastScenario, ScenarioKind, derive_forecast_scenarios
from mgb_ops.workflows.scenario_orchestrator import (
    ScenarioBatchError,
    ScenarioBatchResult,
    ScenarioRunResult,
    execute_forecast_scenarios,
)

__all__ += [
    "ForecastScenario",
    "ScenarioKind",
    "ScenarioBatchError",
    "ScenarioBatchResult",
    "ScenarioRunResult",
    "derive_forecast_scenarios",
    "execute_forecast_scenarios",
]
