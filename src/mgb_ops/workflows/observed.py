from __future__ import annotations

from datetime import datetime
from pathlib import Path
from typing import Iterable

from mgb_ops.common.time_utils import iter_observed_request_dates
from mgb_ops.adapters.observed_fetch_windows import DEFAULT_FETCH_WINDOW_DAYS
from mgb_ops.adapters.observed_ana import (
    DEFAULT_ANA_BASE_URL,
    ObservedFetchSummary as AnaFetchSummary,
    build_run_id,
    configure_run_logger as configure_ana_logger,
    fetch_observed_ana,
)
from mgb_ops.adapters.observed_inmet import (
    DEFAULT_INMET_BASE_URL,
    DEFAULT_INMET_RAIN_PRODUCT,
    ObservedFetchSummary as InmetFetchSummary,
    configure_run_logger as configure_inmet_logger,
    fetch_observed_inmet,
)
from mgb_ops.storage.history_repository import HistoryRepository
from mgb_ops.storage.observed_csv import ObservedCsvImportSummary, load_normalized_observed_csvs


def _filter_stations(stations: list[dict], station_codes: Iterable[str] | None, *, provider_code: str) -> list[dict]:
    if station_codes is None:
        return stations

    requested = {str(station_code).strip().upper() for station_code in station_codes if str(station_code).strip()}
    selected = [station for station in stations if str(station["station_code"]).upper() in requested]
    if not selected:
        raise ValueError(f"No {provider_code.upper()} station found for station_codes={sorted(requested)}.")
    return selected


def _request_dates_by_station(
    repository: HistoryRepository,
    stations: list[dict],
    *,
    window_start: datetime,
    window_end: datetime,
    variable_codes: Iterable[str] | None,
) -> dict[str, list]:
    variable_list = list(variable_codes) if variable_codes is not None else None
    request_dates: dict[str, list] = {}
    for station in stations:
        station_id = str(station["station_id"])
        latest_observed_at = repository.get_latest_observed_at(
            station_id,
            state="raw",
            variable_codes=variable_list,
        )
        request_dates[station_id] = list(
            iter_observed_request_dates(
                window_start,
                window_end,
                latest_observed_at=latest_observed_at,
            )
        )
    return request_dates


def _normalize_provider(provider_code: str) -> str:
    provider = provider_code.strip().lower()
    if provider not in {"ana", "inmet"}:
        raise ValueError(f"Unsupported observed provider_code {provider_code!r}.")
    return provider


def discover_observed_provider_csvs(
    downloads_dir: Path,
    provider_code: str,
    *,
    run_id: str | None = None,
    station_codes: Iterable[str] | None = None,
) -> list[Path]:
    provider = _normalize_provider(provider_code)
    provider_dir = Path(downloads_dir) / provider
    if not provider_dir.exists():
        return []

    requested_stations = None
    if station_codes is not None:
        requested_stations = {
            str(station_code).strip().upper()
            for station_code in station_codes
            if str(station_code).strip()
        }

    run_dirs = [provider_dir / run_id] if run_id is not None else sorted(path for path in provider_dir.iterdir() if path.is_dir())
    csv_paths: list[Path] = []
    for run_dir in run_dirs:
        if not run_dir.is_dir():
            continue
        for station_dir in sorted(path for path in run_dir.iterdir() if path.is_dir()):
            if requested_stations is not None and station_dir.name.upper() not in requested_stations:
                continue
            csv_path = station_dir / "observed.csv"
            if csv_path.exists():
                csv_paths.append(csv_path)
    return csv_paths


def load_observed_provider_csvs(
    provider_code: str,
    *,
    database_path: Path,
    csv_paths: Iterable[Path],
    timestep_hours: int = 1,
    observed_aggregation: dict[str, str] | None = None,
) -> ObservedCsvImportSummary:
    _normalize_provider(provider_code)
    return load_normalized_observed_csvs(
        database_path,
        csv_paths,
        timestep_hours=timestep_hours,
        aggregation_by_variable=observed_aggregation,
    )


def fetch_observed_provider(
    provider_code: str,
    *,
    database_path: Path,
    window_start: datetime,
    window_end: datetime,
    downloads_dir: Path,
    logs_dir: Path | None = None,
    run_id: str | None = None,
    station_codes: Iterable[str] | None = None,
    base_url: str | None = None,
    timeout_seconds: float = 30.0,
    api_key: str | None = None,
    product_code: str = DEFAULT_INMET_RAIN_PRODUCT,
    fetch_window_days: int = DEFAULT_FETCH_WINDOW_DAYS,
) -> AnaFetchSummary | InmetFetchSummary:
    provider = _normalize_provider(provider_code)
    run_id = run_id or build_run_id(window_end)
    logger = None
    if logs_dir is not None:
        if provider == "ana":
            logger = configure_ana_logger(Path(logs_dir) / "observed_ana" / f"{run_id}.log")
        else:
            logger = configure_inmet_logger(Path(logs_dir) / "observed_inmet" / f"{run_id}.log")

    variable_codes = ("rain", "level", "flow") if provider == "ana" else ("rain",)
    with HistoryRepository(database_path) as repository:
        stations = _filter_stations(
            repository.get_provider_stations(provider),
            station_codes,
            provider_code=provider,
        )
        request_dates = _request_dates_by_station(
            repository,
            stations,
            window_start=window_start,
            window_end=window_end,
            variable_codes=variable_codes,
        )

    if provider == "ana":
        return fetch_observed_ana(
            stations,
            request_dates_by_station=request_dates,
            downloads_dir=downloads_dir,
            run_id=run_id,
            base_url=base_url or DEFAULT_ANA_BASE_URL,
            timeout_seconds=timeout_seconds,
            logger=logger,
            fetch_window_days=fetch_window_days,
        )
    else:
        if not api_key:
            raise ValueError("api_key is required for INMET/BNDMET observed ingestion.")
        return fetch_observed_inmet(
            stations,
            request_dates_by_station=request_dates,
            downloads_dir=downloads_dir,
            run_id=run_id,
            api_key=api_key,
            base_url=base_url or DEFAULT_INMET_BASE_URL,
            timeout_seconds=timeout_seconds,
            product_code=product_code,
            logger=logger,
            fetch_window_days=fetch_window_days,
        )
