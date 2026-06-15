from __future__ import annotations

import csv
import logging
import sys
import time
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Any

import requests

from mgb_ops.common.time_utils import TIMEZONE
from mgb_ops.ingest.observed_csv import NORMALIZED_OBSERVED_COLUMNS, import_normalized_observed_csvs
from mgb_ops.storage.history_repository import HistoryRepository

DEFAULT_INMET_BASE_URL = "https://api-bndmet.decea.mil.br/v1"
DEFAULT_INMET_RAIN_PRODUCT = "I175"
INMET_API_KEY_ENV = "INMET_API_KEY"
OBSERVED_VARIABLES = ("rain",)
RETRY_ATTEMPTS = 5
RETRY_SLEEP_SECONDS = 5.0


def script_stem() -> str:
    return Path(__file__).stem


def build_run_id(reference_time: datetime) -> str:
    return reference_time.strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("floodqc.ingest.inmet")
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


def build_normalized_csv_path(inmet_root_dir: Path, *, station_code: str, request_date: date) -> Path:
    station_dir = inmet_root_dir / station_code
    file_stamp = request_date.strftime("%Y%m%d")
    return station_dir / f"{file_stamp}__{file_stamp}.csv"

def ingest_observed_inmet(
    database_path: Path,
    *,
    reference_time: datetime,
    request_days: int,
    timeout_seconds: float,
    api_key: str,
    station_codes: list[str] | None = None,
    interim_dir: Path,
    logs_dir: Path,
    base_url: str = DEFAULT_INMET_BASE_URL,
    product_code: str = DEFAULT_INMET_RAIN_PRODUCT,
) -> dict[str, object]:
    if request_days < 1:
        raise ValueError("request_days must be >= 1.")
    if not Path(database_path).exists():
        raise FileNotFoundError(f"History database not found: {database_path}")

    api_key = api_key.strip()
    if not api_key:
        raise ValueError("api_key is required for INMET ingestion.")

    run_id = build_run_id(reference_time)
    logger = configure_run_logger(logs_dir / script_stem() / f"{run_id}.log")
    product_code = product_code.strip() or DEFAULT_INMET_RAIN_PRODUCT
    inmet_root_dir = interim_dir / "inmet" / run_id
    inmet_root_dir.mkdir(parents=True, exist_ok=True)

    with HistoryRepository(database_path) as repository:
        stations = repository.get_provider_stations("inmet")
        if station_codes:
            allowed_codes = {normalize_inmet_station_code(code) for code in station_codes}
            stations = [station for station in stations if station["station_code"] in allowed_codes]
        if not stations:
            raise ValueError("No INMET station found for ingestion.")

        summary = {
            "run_id": run_id,
            "stations_total": len(stations),
            "stations_ok": 0,
            "stations_no_data": 0,
            "stations_error": 0,
        }

        for station in stations:
            station_code = normalize_inmet_station_code(station["station_code"])
            station_id = station["station_id"]
            assert station_code is not None
            station_written = {"rain": 0}
            station_error = False

            with requests.Session() as session:
                for request_date in iter_request_dates(reference_time, request_days):
                    try:
                        payload = fetch_station_payload(
                            station_code,
                            request_date=request_date,
                            base_url=base_url,
                            timeout_seconds=timeout_seconds,
                            api_key=api_key,
                            product_code=product_code,
                            session=session,
                        )
                        frame = parse_payload(payload, station_code=station_code)
                        csv_path = write_normalized_csv(
                            frame,
                            output_path=build_normalized_csv_path(inmet_root_dir, station_code=station_code, request_date=request_date),
                            station_id=station_id,
                            provider_code="inmet",
                            station_code=station_code,
                        )
                        import_summary = import_normalized_observed_csvs(database_path, [csv_path])
                        counts = {"rain": import_summary.values_by_variable.get("rain", 0)}
                        station_written["rain"] += counts["rain"]
                        logger.info(
                            "station=%s station_id=%s window_start=%s window_end=%s records=%s rain=%s normalized_csv=%s product_code=%s",
                            station_code,
                            station_id,
                            request_date.strftime("%Y-%m-%d"),
                            request_date.strftime("%Y-%m-%d"),
                            len(frame),
                            counts["rain"],
                            csv_path,
                            product_code,
                        )
                    except (requests.RequestException, ValueError, RuntimeError) as exc:
                        station_error = True
                        logger.error(
                            "station=%s station_id=%s window_start=%s window_end=%s error=%s",
                            station_code,
                            station_id,
                            request_date.strftime("%Y-%m-%d"),
                            request_date.strftime("%Y-%m-%d"),
                            exc,
                        )
                        break

            total_written = station_written["rain"]
            if station_error:
                summary["stations_error"] += 1
            elif total_written == 0:
                summary["stations_no_data"] += 1
            else:
                summary["stations_ok"] += 1

        logger.info(
            "run_id=%s stations_total=%s stations_ok=%s stations_no_data=%s stations_error=%s",
            summary["run_id"],
            summary["stations_total"],
            summary["stations_ok"],
            summary["stations_no_data"],
            summary["stations_error"],
        )
        return summary
