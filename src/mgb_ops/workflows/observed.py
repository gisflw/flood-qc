from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Iterable

from mgb_ops.utils.time import iter_observed_request_dates
from mgb_ops.adapters.observed_fetch_windows import DEFAULT_FETCH_WINDOW_DAYS
from mgb_ops.adapters import ObservationAdapter, get_observation_adapter
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.assets.observations import ObservedCsvImportSummary, load_normalized_observed_csvs
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.utils.time import resolve_reference_time
from mgb_ops.workflows._providers import normalize_provider_codes


@dataclass(frozen=True, slots=True)
class ObservedProviderDownloadSummary:
    provider_code: str
    normalized_csv_paths: tuple[Path, ...]
    requested_days: int
    skipped_days: int
    adapter_summary: object


@dataclass(frozen=True, slots=True)
class ObservedDownloadSummary:
    providers: tuple[ObservedProviderDownloadSummary, ...]

    @property
    def normalized_csv_paths(self) -> list[Path]:
        return [path for provider in self.providers for path in provider.normalized_csv_paths]


def _station_variable_codes(station: dict, provider_variable_codes: Iterable[str] | None) -> tuple[str, ...]:
    provider_variables = tuple(provider_variable_codes or ())
    if "observed_variable_codes" not in station:
        return provider_variables
    station_variables = tuple(str(code) for code in station.get("observed_variable_codes", ()))
    if not station_variables:
        return ()
    if not provider_variables:
        return station_variables
    station_variable_set = set(station_variables)
    return tuple(variable for variable in provider_variables if variable in station_variable_set)


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
) -> dict[str, list[date]]:
    variable_list = tuple(variable_codes) if variable_codes is not None else None
    request_dates: dict[str, list[date]] = {}
    for station in stations:
        station_id = str(station["station_id"])
        station_variables = _station_variable_codes(station, variable_list)
        if not station_variables:
            request_dates[station_id] = []
            continue
        coverage = repository.get_observed_calendar_days(
            station_id,
            start_date=window_start.date().isoformat(),
            end_date=window_end.date().isoformat(),
            state="raw",
            variable_codes=station_variables,
        )
        dates = list(iter_observed_request_dates(window_start, window_end))
        reference_date = window_end.date()
        request_dates[station_id] = [
            value for value in dates
            if value == reference_date
            or any(value.isoformat() not in coverage.get(code, set()) for code in station_variables)
        ]
    return request_dates


def _resume_dates_by_station(
    repository: HistoryRepository,
    stations: list[dict],
    *,
    window_start: datetime,
    window_end: datetime,
    variable_codes: Iterable[str] | None,
) -> dict[str, list[date]]:
    variable_list = tuple(variable_codes) if variable_codes is not None else None
    request_dates: dict[str, list[date]] = {}
    for station in stations:
        station_id = str(station["station_id"])
        station_variables = _station_variable_codes(station, variable_list)
        if not station_variables:
            request_dates[station_id] = []
            continue
        request_dates[station_id] = list(
            iter_observed_request_dates(
                window_start,
                window_end,
                latest_observed_at=repository.get_latest_observed_at(
                    station_id,
                    state="raw",
                    variable_codes=station_variables,
                ),
            )
        )
    return request_dates


def discover_observed_provider_csvs(
    downloads_dir: Path,
    provider_code: str,
    *,
    run_id: str | None = None,
    station_codes: Iterable[str] | None = None,
) -> list[Path]:
    provider = get_observation_adapter(provider_code).provider_code
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
    get_observation_adapter(provider_code)
    return load_normalized_observed_csvs(
        database_path,
        csv_paths,
        timestep_hours=timestep_hours,
        aggregation_by_variable=observed_aggregation,
    )


def ingest_from_csv(
    context: RuntimeContext,
    csv_path: Path,
    state: str = "raw",
    *,
    timestep_hours: int | None = None,
    observed_aggregation: dict[str, str] | None = None,
) -> ObservedCsvImportSummary:
    settings = context.settings
    return load_normalized_observed_csvs(
        context.paths.history_db,
        [csv_path],
        timestep_hours=int(timestep_hours or settings["run"]["timestep_hours"]),
        aggregation_by_variable=observed_aggregation
        or dict(settings["ingest"]["observed_aggregation"]),
        state=state,
        provider_variables={
            code: set(get_observation_adapter(code).variable_codes)
            for code in ("ana", "inmet")
        },
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
    credential: str | None = None,
    product_code: str | None = None,
    fetch_window_days: int = DEFAULT_FETCH_WINDOW_DAYS,
    request_dates_by_station: dict[str, list[date]] | None = None,
) -> object:
    adapter: ObservationAdapter = get_observation_adapter(provider_code)
    provider = adapter.provider_code
    run_id = run_id or window_end.strftime("%Y%m%dT%H%M%S")
    logger = None
    if logs_dir is not None:
        logger = adapter.configure_logger(Path(logs_dir) / f"observed_{provider}" / f"{run_id}.log")

    with HistoryRepository(database_path) as repository:
        stations = _filter_stations(
            repository.get_provider_stations(provider),
            station_codes,
            provider_code=provider,
        )
        request_dates = (
            request_dates_by_station
            if request_dates_by_station is not None
            else _resume_dates_by_station(
                repository,
                stations,
                window_start=window_start,
                window_end=window_end,
                variable_codes=adapter.variable_codes,
            )
        )

    return adapter.fetch(
        stations,
        request_dates_by_station=request_dates,
        downloads_dir=downloads_dir,
        run_id=run_id,
        base_url=base_url,
        timeout_seconds=timeout_seconds,
        credential=credential,
        product_code=product_code,
        logger=logger,
        fetch_window_days=fetch_window_days,
    )


def download_observed_data(
    context: RuntimeContext,
    providers: str | Iterable[str],
    *,
    reference_time: datetime | None = None,
    station_codes_by_provider: dict[str, Iterable[str]] | None = None,
) -> ObservedDownloadSummary:
    provider_codes = normalize_provider_codes(providers, get_observation_adapter)
    settings = context.settings
    reference = reference_time or resolve_reference_time(str(settings["run"]["reference_time"]))
    request_days = int(settings["ingest"]["request_days"])
    window_start = datetime.combine(reference.date() - timedelta(days=request_days - 1), datetime.min.time())
    results: list[ObservedProviderDownloadSummary] = []
    for provider in provider_codes:
        adapter = get_observation_adapter(provider)
        credential = (
            context.env.get(adapter.credential_env_name)
            if adapter.credential_env_name is not None
            else None
        )
        with HistoryRepository(context.paths.history_db) as repository:
            stations = _filter_stations(
                repository.get_provider_stations(provider),
                (station_codes_by_provider or {}).get(provider),
                provider_code=provider,
            )
            requests = _request_dates_by_station(
                repository,
                stations,
                window_start=window_start,
                window_end=reference,
                variable_codes=adapter.variable_codes,
            )
        requested = sum(len(values) for values in requests.values())
        total = len(stations) * request_days
        summary = fetch_observed_provider(
            provider,
            database_path=context.paths.history_db,
            window_start=window_start,
            window_end=reference,
            downloads_dir=context.paths.downloads_dir,
            logs_dir=context.paths.logs_dir,
            station_codes=(station_codes_by_provider or {}).get(provider),
            timeout_seconds=float(settings["ingest"]["timeout_seconds"]),
            credential=credential,
            fetch_window_days=int(settings["ingest"]["fetch_window_days"]),
            request_dates_by_station=requests,
        )
        results.append(
            ObservedProviderDownloadSummary(
                provider_code=provider,
                normalized_csv_paths=tuple(Path(path) for path in summary.csv_paths),
                requested_days=requested,
                skipped_days=max(total - requested, 0),
                adapter_summary=summary,
            )
        )
    return ObservedDownloadSummary(tuple(results))
