from __future__ import annotations

import csv
import logging
import sys
import time
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any, Iterable

import requests

from mgb_ops.common.time_utils import TIMEZONE
from mgb_ops.storage.observed_csv import NORMALIZED_OBSERVED_COLUMNS

DEFAULT_INMET_BASE_URL = "https://api-bndmet.decea.mil.br/v1"
DEFAULT_INMET_RAIN_PRODUCT = "I175"
INMET_API_KEY_ENV = "INMET_API_KEY"
OBSERVED_VARIABLES = ("rain",)
RETRY_ATTEMPTS = 5
RETRY_SLEEP_SECONDS = 5.0


@dataclass(frozen=True, slots=True)
class ObservedFetchStationSummary:
    station_id: str
    station_code: str
    request_start: date | None
    request_end: date | None
    rows_parsed: int
    csv_path: Path | None
    no_data: bool
    error: str | None = None


@dataclass(frozen=True, slots=True)
class ObservedFetchSummary:
    run_id: str
    provider_code: str
    stations: tuple[ObservedFetchStationSummary, ...]

    @property
    def csv_paths(self) -> list[Path]:
        return [station.csv_path for station in self.stations if station.csv_path is not None]

    def legacy_counts(self) -> dict[str, int | str]:
        stations_ok = sum(1 for station in self.stations if station.error is None and not station.no_data)
        stations_error = sum(1 for station in self.stations if station.error is not None)
        stations_no_data = sum(1 for station in self.stations if station.error is None and station.no_data)
        return {
            "run_id": self.run_id,
            "stations_total": len(self.stations),
            "stations_ok": stations_ok,
            "stations_no_data": stations_no_data,
            "stations_error": stations_error,
        }


def script_stem() -> str:
    return Path(__file__).stem


def build_run_id(reference_time: datetime) -> str:
    return reference_time.strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("adapters.observed_inmet")
    logger.setLevel(logging.INFO)
    for handler in logger.handlers[:]:
        handler.close()
        logger.removeHandler(handler)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s | %(levelname)s | %(message)s")

    file_handler = logging.FileHandler(log_file, mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    return logger


def normalize_inmet_station_code(station_code: str | None) -> str | None:
    if station_code is None:
        return None
    normalized = str(station_code).strip().upper()
    if not normalized:
        return None
    return normalized



def iter_request_dates(reference_time: datetime, request_days: int):
    reference_date = reference_time.date()
    start_date = reference_date - timedelta(days=request_days - 1)

    for offset in range(request_days):
        yield start_date + timedelta(days=offset)


def build_station_request_url(
    station_code: str,
    *,
    base_url: str,
    product_code: str = DEFAULT_INMET_RAIN_PRODUCT,
) -> str:
    return f"{base_url.rstrip('/')}/estacoes/{station_code}/fenomenos/{product_code}"


def fetch_station_payload(
    station_code: str,
    *,
    request_date: date,
    base_url: str,
    timeout_seconds: float,
    api_key: str,
    product_code: str = DEFAULT_INMET_RAIN_PRODUCT,
    session: requests.Session | None = None,
    retry_attempts: int = RETRY_ATTEMPTS,
    retry_sleep_seconds: float = RETRY_SLEEP_SECONDS,
) -> Any:
    params = {
        "dataInicio": request_date.isoformat(),
        "dataFinal": request_date.isoformat(),
    }
    session = session or requests.Session()
    session.headers.update(
        {
            "accept": "application/json",
            "x-api-key": api_key,
        }
    )
    url = build_station_request_url(station_code, base_url=base_url, product_code=product_code)

    last_exc: Exception | None = None
    for attempt in range(retry_attempts):
        try:
            response = session.get(url, params=params, timeout=timeout_seconds)
            response.raise_for_status()
            return response.json()
        except (requests.Timeout, requests.RequestException, ValueError) as exc:
            last_exc = exc
            if attempt == retry_attempts - 1:
                break
            time.sleep(retry_sleep_seconds)

    assert last_exc is not None
    raise RuntimeError(
        f"Falha ao consultar INMET/BNDMET para station_code={station_code} "
        f"date={request_date.isoformat()} apos {retry_attempts} tentativas."
    ) from last_exc


def _extract_data_rows(payload: Any) -> list[Any]:
    if isinstance(payload, dict):
        data = payload.get("data")
        if isinstance(data, dict):
            nested = data.get("data")
            if isinstance(nested, list):
                return nested
        if isinstance(data, list):
            return data
    if isinstance(payload, list):
        return payload
    return []


def _parse_timestamp(raw_value: Any) -> datetime | None:
    if raw_value is None:
        return None
    if isinstance(raw_value, datetime):
        observed_at = raw_value
        if observed_at.tzinfo is not None:
            observed_at = observed_at.astimezone(TIMEZONE).replace(tzinfo=None)
        return observed_at.replace(second=0, microsecond=0)
    if isinstance(raw_value, str):
        text = raw_value.strip()
        if not text:
            return None
        if text.endswith("Z"):
            text = text[:-1] + "+00:00"
        try:
            observed_at = datetime.fromisoformat(text)
        except ValueError:
            try:
                observed_at = datetime.strptime(text, "%d/%m/%Y %H:%M")
            except ValueError:
                return None
        if observed_at.tzinfo is not None:
            observed_at = observed_at.astimezone(TIMEZONE).replace(tzinfo=None)
        return observed_at.replace(second=0, microsecond=0)
    try:
        timestamp_ms = int(raw_value)
    except (TypeError, ValueError):
        return None

    observed_at = datetime.fromtimestamp(timestamp_ms / 1000.0, tz=TIMEZONE)
    return observed_at.replace(tzinfo=None, second=0, microsecond=0)


def _parse_rain_value(raw_value: Any) -> float | None:
    if raw_value in (None, "", "-"):
        return None
    try:
        return float(raw_value)
    except (TypeError, ValueError):
        return None


def _parse_row_as_pair(row: Any) -> tuple[datetime | None, float | None]:
    if not isinstance(row, (list, tuple)) or len(row) < 2:
        return None, None
    return _parse_timestamp(row[0]), _parse_rain_value(row[1])


def _parse_row_as_dict(row: dict[str, Any]) -> tuple[datetime | None, float | None]:
    timestamp = (
        row.get("timestamp")
        or row.get("time")
        or row.get("datetime")
        or row.get("observed_at")
        or row.get("dataHora")
        or row.get("date")
    )
    value = row.get("value")
    if value is None:
        value = row.get("chuva")
    if value is None:
        value = row.get("rain")
    return _parse_timestamp(timestamp), _parse_rain_value(value)


def parse_payload(payload: Any, *, station_code: str):
    import pandas as pd

    normalized_station_code = normalize_inmet_station_code(station_code)
    if normalized_station_code is None:
        raise ValueError("Invalid INMET station_code.")

    records: list[dict[str, Any]] = []
    for row in _extract_data_rows(payload):
        observed_at: datetime | None
        rain_value: float | None

        if isinstance(row, dict):
            observed_at, rain_value = _parse_row_as_dict(row)
            row_station_code = normalize_inmet_station_code(row.get("codigo") or row.get("station_code") or station_code)
        else:
            observed_at, rain_value = _parse_row_as_pair(row)
            row_station_code = normalized_station_code

        if observed_at is None:
            continue
        if row_station_code != normalized_station_code:
            raise ValueError(
                f"Resposta do INMET/BNDMET retornou station_code inesperado para {normalized_station_code}: {row_station_code}"
            )
        records.append(
            {
                "station_code": normalized_station_code,
                "observed_at": observed_at,
                "rain": rain_value,
            }
        )

    if not records:
        return pd.DataFrame(columns=["station_code", "observed_at", "rain"])

    frame = pd.DataFrame.from_records(records)
    frame["observed_at"] = pd.to_datetime(frame["observed_at"], errors="coerce").dt.floor("min")
    frame["rain"] = pd.to_numeric(frame["rain"], errors="coerce")
    frame = frame.dropna(subset=["observed_at"])
    return frame.sort_values("observed_at").reset_index(drop=True)



def write_normalized_csv(
    frame,
    *,
    output_path: Path,
    station_id: str,
    provider_code: str,
    station_code: str,
    state: str = "raw",
) -> Path:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=NORMALIZED_OBSERVED_COLUMNS)
        writer.writeheader()
        if frame.empty:
            return output_path
        for record in frame.sort_values("observed_at").to_dict("records"):
            value = record.get("rain")
            if value is None:
                continue
            try:
                if value != value:
                    continue
            except TypeError:
                pass
            writer.writerow(
                {
                    "station_id": station_id,
                    "provider_code": provider_code,
                    "station_code": station_code,
                    "observed_at": record["observed_at"].strftime("%Y-%m-%d %H:%M"),
                    "variable_code": "rain",
                    "value": float(value),
                    "state": state,
                }
            )
    return output_path


def build_normalized_csv_path(inmet_root_dir: Path, *, station_code: str, request_date: date | None = None) -> Path:
    return inmet_root_dir / station_code / "observed.csv"


def fetch_observed_inmet(
    stations: Iterable[dict],
    *,
    request_dates_by_station: dict[str, Iterable[date]],
    downloads_dir: Path,
    run_id: str,
    api_key: str,
    base_url: str = DEFAULT_INMET_BASE_URL,
    timeout_seconds: float = 30.0,
    product_code: str = DEFAULT_INMET_RAIN_PRODUCT,
    retry_attempts: int = RETRY_ATTEMPTS,
    retry_sleep_seconds: float = RETRY_SLEEP_SECONDS,
    logger: logging.Logger | None = None,
) -> ObservedFetchSummary:
    import pandas as pd

    if not api_key:
        raise ValueError("api_key is required for INMET/BNDMET observed ingestion.")

    inmet_root_dir = Path(downloads_dir) / "inmet" / run_id
    station_summaries: list[ObservedFetchStationSummary] = []

    with requests.Session() as session:
        for station in stations:
            station_id = str(station["station_id"])
            station_code = str(station["station_code"])
            request_dates = list(request_dates_by_station.get(station_id, []))
            csv_path = build_normalized_csv_path(inmet_root_dir, station_code=station_code)

            if not request_dates:
                write_normalized_csv(
                    pd.DataFrame(columns=["station_code", "observed_at", "rain"]),
                    output_path=csv_path,
                    station_id=station_id,
                    provider_code="inmet",
                    station_code=station_code,
                )
                station_summaries.append(
                    ObservedFetchStationSummary(
                        station_id=station_id,
                        station_code=station_code,
                        request_start=None,
                        request_end=None,
                        rows_parsed=0,
                        csv_path=csv_path,
                        no_data=True,
                    )
                )
                continue

            frames = []
            try:
                for request_date in request_dates:
                    payload = fetch_station_payload(
                        station_code,
                        request_date=request_date,
                        base_url=base_url,
                        timeout_seconds=timeout_seconds,
                        api_key=api_key,
                        product_code=product_code,
                        session=session,
                        retry_attempts=retry_attempts,
                        retry_sleep_seconds=retry_sleep_seconds,
                    )
                    frame = parse_payload(payload, station_code=station_code)
                    if not frame.empty:
                        frames.append(frame)
                    if logger is not None:
                        logger.info(
                            "station_day_fetched station_id=%s station_code=%s request_date=%s rows=%s",
                            station_id,
                            station_code,
                            request_date.isoformat(),
                            len(frame),
                        )

                combined = (
                    pd.concat(frames, ignore_index=True)
                    if frames
                    else pd.DataFrame(columns=["station_code", "observed_at", "rain"])
                )
                write_normalized_csv(
                    combined,
                    output_path=csv_path,
                    station_id=station_id,
                    provider_code="inmet",
                    station_code=station_code,
                )
                station_summaries.append(
                    ObservedFetchStationSummary(
                        station_id=station_id,
                        station_code=station_code,
                        request_start=min(request_dates),
                        request_end=max(request_dates),
                        rows_parsed=len(combined),
                        csv_path=csv_path,
                        no_data=combined.empty,
                    )
                )
                if logger is not None:
                    logger.info(
                        "station_complete station_id=%s station_code=%s window_start=%s window_end=%s rows_parsed=%s normalized_csv=%s no_data=%s",
                        station_id,
                        station_code,
                        min(request_dates).isoformat(),
                        max(request_dates).isoformat(),
                        len(combined),
                        csv_path,
                        combined.empty,
                    )
            except Exception as exc:
                station_summaries.append(
                    ObservedFetchStationSummary(
                        station_id=station_id,
                        station_code=station_code,
                        request_start=min(request_dates),
                        request_end=max(request_dates),
                        rows_parsed=0,
                        csv_path=None,
                        no_data=False,
                        error=str(exc),
                    )
                )
                if logger is not None:
                    logger.exception(
                        "station_error station_id=%s station_code=%s window_start=%s window_end=%s",
                        station_id,
                        station_code,
                        min(request_dates).isoformat(),
                        max(request_dates).isoformat(),
                    )

    return ObservedFetchSummary(run_id=run_id, provider_code="inmet", stations=tuple(station_summaries))
