"""External data provider adapters."""
"""External data-provider integrations and their generic registry."""

from mgb_ops.adapters.registry import (
    DEFAULT_FORECAST_ADAPTER,
    ForecastAdapter,
    ObservationAdapter,
    get_forecast_adapter,
    get_observation_adapter,
    list_forecast_adapter_codes,
    register_forecast_adapter,
    register_observation_adapter,
)

__all__ = [
    "DEFAULT_FORECAST_ADAPTER",
    "ForecastAdapter",
    "ObservationAdapter",
    "get_forecast_adapter",
    "get_observation_adapter",
    "list_forecast_adapter_codes",
    "register_forecast_adapter",
    "register_observation_adapter",
]
