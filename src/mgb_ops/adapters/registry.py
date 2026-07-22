from __future__ import annotations

import logging
from dataclasses import dataclass
from datetime import date, datetime
from pathlib import Path
from typing import Any, Callable, Iterable, Mapping, Protocol

from mgb_ops.adapters import forecast_ecmwf, forecast_noaa, observed_ana, observed_inmet


class ObservationAdapter(Protocol):
    provider_code: str
    variable_codes: tuple[str, ...]
    credential_env_name: str | None
    default_product_code: str | None

    def normalize_station_code(self, station_code: str | None) -> str | None: ...

    def configure_logger(self, log_file: Path) -> logging.Logger: ...

    def fetch(
        self,
        stations: list[dict],
        *,
        request_dates_by_station: Mapping[str, Iterable[date]],
        downloads_dir: Path,
        run_id: str,
        base_url: str | None = None,
        timeout_seconds: float = 30.0,
        credential: str | None = None,
        product_code: str | None = None,
        logger: logging.Logger | None = None,
        fetch_window_days: int,
    ) -> Any: ...


class ForecastAdapter(Protocol):
    provider_code: str
    product_config: Any

    def asset_id(self, cycle_time: datetime) -> str: ...

    def store_grid(self, **kwargs: Any) -> Any: ...
    def download_grib(self, **kwargs: Any) -> Path: ...
    def process_grib(self, grib_path: Path, **kwargs: Any) -> Any: ...


@dataclass(slots=True)
class _ObservationAdapter:
    provider_code: str
    variable_codes: tuple[str, ...]
    default_base_url: str
    fetch_function: Callable[..., Any]
    normalize_function: Callable[[str | None], str | None]
    logger_function: Callable[[Path], logging.Logger]
    credential_env_name: str | None = None
    default_product_code: str | None = None

    def normalize_station_code(self, station_code: str | None) -> str | None:
        return self.normalize_function(station_code)

    def configure_logger(self, log_file: Path) -> logging.Logger:
        return self.logger_function(log_file)

    def fetch(
        self,
        stations: list[dict],
        *,
        request_dates_by_station: Mapping[str, Iterable[date]],
        downloads_dir: Path,
        run_id: str,
        base_url: str | None = None,
        timeout_seconds: float = 30.0,
        credential: str | None = None,
        product_code: str | None = None,
        logger: logging.Logger | None = None,
        fetch_window_days: int,
    ) -> Any:
        kwargs: dict[str, Any] = {
            "request_dates_by_station": request_dates_by_station,
            "downloads_dir": downloads_dir,
            "run_id": run_id,
            "base_url": base_url or self.default_base_url,
            "timeout_seconds": timeout_seconds,
            "logger": logger,
            "fetch_window_days": fetch_window_days,
        }
        if self.credential_env_name is not None:
            if not credential:
                raise ValueError(
                    f"A credential is required for provider {self.provider_code!r}; "
                    f"set {self.credential_env_name}."
                )
            kwargs["api_key"] = credential
        if self.default_product_code is not None:
            kwargs["product_code"] = product_code or self.default_product_code
        return self.fetch_function(stations, **kwargs)


@dataclass(frozen=True, slots=True)
class _ForecastAdapter:
    provider_code: str
    product_config: Any
    asset_id_function: Callable[..., str]
    store_grid_function: Callable[..., Any]
    download_grib_function: Callable[..., Path]
    process_grib_function: Callable[..., Any]

    def asset_id(self, cycle_time: datetime) -> str:
        return self.asset_id_function(cycle_time, self.product_config)

    def store_grid(self, **kwargs: Any) -> Any:
        return self.store_grid_function(
            **kwargs,
            product_config=self.product_config,
        )

    def download_grib(self, **kwargs: Any) -> Path:
        return self.download_grib_function(
            **kwargs, product_config=self.product_config
        )

    def process_grib(self, grib_path: Path, **kwargs: Any) -> Any:
        return self.process_grib_function(
            grib_path, **kwargs, product_config=self.product_config
        )


_OBSERVATION_ADAPTERS: dict[str, ObservationAdapter] = {
    "ana": _ObservationAdapter(
        provider_code="ana",
        variable_codes=observed_ana.OBSERVED_VARIABLES,
        default_base_url=observed_ana.DEFAULT_ANA_BASE_URL,
        fetch_function=observed_ana.fetch_observed_ana,
        normalize_function=observed_ana.normalize_ana_station_code,
        logger_function=observed_ana.configure_run_logger,
    ),
    "inmet": _ObservationAdapter(
        provider_code="inmet",
        variable_codes=observed_inmet.OBSERVED_VARIABLES,
        default_base_url=observed_inmet.DEFAULT_INMET_BASE_URL,
        fetch_function=observed_inmet.fetch_observed_inmet,
        normalize_function=observed_inmet.normalize_inmet_station_code,
        logger_function=observed_inmet.configure_run_logger,
        credential_env_name=observed_inmet.INMET_API_KEY_ENV,
        default_product_code=observed_inmet.DEFAULT_INMET_RAIN_PRODUCT,
    ),
}

DEFAULT_FORECAST_ADAPTER: ForecastAdapter = _ForecastAdapter(
    provider_code=forecast_ecmwf.ECMWF_FORECAST_PRODUCT.provider_code,
    product_config=forecast_ecmwf.ECMWF_FORECAST_PRODUCT,
    asset_id_function=forecast_ecmwf.build_asset_id,
    store_grid_function=forecast_ecmwf.store_normalized_forecast_grid,
    download_grib_function=forecast_ecmwf.download_forecast_grib,
    process_grib_function=forecast_ecmwf.process_forecast_grib,
)
NOAA_FORECAST_ADAPTER: ForecastAdapter = _ForecastAdapter(
    provider_code=forecast_noaa.GFS_FORECAST_PRODUCT.provider_code,
    product_config=forecast_noaa.GFS_FORECAST_PRODUCT,
    asset_id_function=forecast_noaa.build_asset_id,
    store_grid_function=forecast_noaa.store_normalized_forecast_grid,
    download_grib_function=forecast_noaa.download_forecast_grib,
    process_grib_function=forecast_noaa.process_forecast_grib,
)
_FORECAST_ADAPTERS: dict[str, ForecastAdapter] = {
    DEFAULT_FORECAST_ADAPTER.provider_code: DEFAULT_FORECAST_ADAPTER,
    NOAA_FORECAST_ADAPTER.provider_code: NOAA_FORECAST_ADAPTER,
}


def get_observation_adapter(provider_code: str) -> ObservationAdapter:
    normalized = provider_code.strip().lower()
    try:
        return _OBSERVATION_ADAPTERS[normalized]
    except KeyError as exc:
        raise ValueError(f"Unsupported observation provider_code {provider_code!r}.") from exc


def get_forecast_adapter(provider_code: str) -> ForecastAdapter:
    normalized = provider_code.strip().lower()
    try:
        return _FORECAST_ADAPTERS[normalized]
    except KeyError as exc:
        raise ValueError(f"Unsupported forecast provider_code {provider_code!r}.") from exc


def list_forecast_adapter_codes() -> tuple[str, ...]:
    """Return forecast providers supported by the in-process adapter registry."""
    return tuple(sorted(_FORECAST_ADAPTERS))


def register_observation_adapter(adapter: ObservationAdapter) -> None:
    _OBSERVATION_ADAPTERS[adapter.provider_code.strip().lower()] = adapter


def register_forecast_adapter(adapter: ForecastAdapter) -> None:
    _FORECAST_ADAPTERS[adapter.provider_code.strip().lower()] = adapter
