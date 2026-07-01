from __future__ import annotations

import logging
import sys
import xml.etree.ElementTree as ET
from dataclasses import dataclass
from datetime import date, datetime, timedelta
from pathlib import Path
from typing import Iterable

import requests

from mgb_ops.common.time_utils import TIMEZONE, resolve_reference_time
from mgb_ops.adapters.observed_fetch_windows import DEFAULT_FETCH_WINDOW_DAYS, iter_fetch_date_windows
from mgb_ops.assets.observations import write_normalized_observed_csv

DEFAULT_ANA_BASE_URL = "http://telemetriaws1.ana.gov.br/serviceana.asmx/DadosHidrometeorologicos"
OBSERVED_VARIABLES = ("rain", "level", "flow")


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
    logger = logging.getLogger("adapters.observed_ana")
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


def parse_float(value: str | None) -> float | None:
    if value is None:
        return None
    normalized = value.strip().replace(",", ".")
    if not normalized or normalized == "-":
        return None
    try:
        return float(normalized)
    except ValueError:
        return None


def normalize_ana_station_code(station_code: str | None) -> str | None:
    if station_code is None:
        return None
    normalized = str(station_code).strip()
    if not normalized:
        return None
    return normalized.lstrip("0") or "0"


def iter_data_nodes(root: ET.Element):
    for element in root.iter():
        if element.tag.endswith("DadosHidrometereologicos"):
            yield element


def parse_response(text: str):
    import pandas as pd

    root = ET.fromstring(text)
    records: list[dict] = []
    for data in iter_data_nodes(root):
        station_code = normalize_ana_station_code(data.findtext("CodEstacao"))
        observed_text = data.findtext("DataHora")
        if not station_code or not observed_text:
            continue

        try:
            observed_at = pd.to_datetime(observed_text.strip(), errors="raise")
        except (TypeError, ValueError):
            continue

        if getattr(observed_at, "tzinfo", None) is not None:
            observed_at = observed_at.tz_convert(TIMEZONE).tz_localize(None)

        record = {
            "station_code": station_code,
            "observed_at": observed_at,
            "rain": parse_float(data.findtext("Chuva")),
            "level": parse_float(data.findtext("Nivel")),
            "flow": parse_float(data.findtext("Vazao")),
        }
        if any(record[variable] is not None for variable in OBSERVED_VARIABLES):
            records.append(record)

    if not records:
        return pd.DataFrame(columns=["station_code", "observed_at", *OBSERVED_VARIABLES])

    frame = pd.DataFrame.from_records(records)
    frame["observed_at"] = pd.to_datetime(frame["observed_at"]).dt.floor("min")
    return frame


def fetch_station_xml(
    station_code: str,
    *,
    request_date: date | None = None,
    request_start_date: date | None = None,
    request_end_date: date | None = None,
    base_url: str,
    timeout_seconds: float,
) -> str:
    if request_date is not None:
        request_start_date = request_date
        request_end_date = request_date
    if request_start_date is None or request_end_date is None:
        raise ValueError("request_start_date and request_end_date are required.")
    if request_end_date < request_start_date:
        raise ValueError("request_end_date must be >= request_start_date.")

    params = {
        "codEstacao": station_code,
        "dataInicio": request_start_date.strftime("%d/%m/%Y"),
        "dataFim": request_end_date.strftime("%d/%m/%Y"),
    }
    response = requests.get(base_url, params=params, timeout=timeout_seconds)
    response.raise_for_status()
    return response.text


def iter_request_dates(reference_time: datetime, request_days: int):
    reference_date = reference_time.date()
    start_date = reference_date - timedelta(days=request_days - 1)

    for offset in range(request_days):
        yield start_date + timedelta(days=offset)


def save_raw_xml(
    xml_text: str,
    *,
    ana_root_dir: Path,
    station_code: str,
    request_date: date | None = None,
    request_start_date: date | None = None,
    request_end_date: date | None = None,
) -> Path:
    if request_date is not None:
        request_start_date = request_date
        request_end_date = request_date
    if request_start_date is None or request_end_date is None:
        raise ValueError("request_start_date and request_end_date are required.")

    station_dir = ana_root_dir / station_code
    station_dir.mkdir(parents=True, exist_ok=True)
    start_stamp = request_start_date.strftime("%Y%m%d")
    end_stamp = request_end_date.strftime("%Y%m%d")
    file_path = station_dir / f"{start_stamp}__{end_stamp}.xml"
    file_path.write_text(xml_text, encoding="utf-8")
    return file_path


def write_normalized_csv(
    frame,
    *,
    output_path: Path,
    station_id: str,
    provider_code: str,
    station_code: str,
    state: str = "raw",
) -> Path:
    return write_normalized_observed_csv(
        frame,
        output_path=output_path,
        station_id=station_id,
        provider_code=provider_code,
        station_code=station_code,
        variable_columns=OBSERVED_VARIABLES,
        state=state,
    )


def build_normalized_csv_path(ana_root_dir: Path, *, station_code: str, request_date: date | None = None) -> Path:
    return ana_root_dir / station_code / "observed.csv"


def fetch_observed_ana(
    stations: Iterable[dict],
    *,
    request_dates_by_station: dict[str, Iterable[date]],
    downloads_dir: Path,
    run_id: str,
    base_url: str = DEFAULT_ANA_BASE_URL,
    timeout_seconds: float = 30.0,
    logger: logging.Logger | None = None,
    save_raw: bool = True,
    fetch_window_days: int = DEFAULT_FETCH_WINDOW_DAYS,
) -> ObservedFetchSummary:
    import pandas as pd

    if fetch_window_days < 1:
        raise ValueError("fetch_window_days must be >= 1.")

    ana_root_dir = Path(downloads_dir) / "ana" / run_id
    station_summaries: list[ObservedFetchStationSummary] = []

    for station in stations:
        station_id = str(station["station_id"])
        station_code = str(station["station_code"])
        request_dates = list(request_dates_by_station.get(station_id, []))
        csv_path = build_normalized_csv_path(ana_root_dir, station_code=station_code)

        if not request_dates:
            write_normalized_csv(
                pd.DataFrame(columns=["station_code", "observed_at", *OBSERVED_VARIABLES]),
                output_path=csv_path,
                station_id=station_id,
                provider_code="ana",
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
            for request_start_date, request_end_date in iter_fetch_date_windows(
                request_dates,
                fetch_window_days=fetch_window_days,
            ):
                xml_text = fetch_station_xml(
                    station_code,
                    request_start_date=request_start_date,
                    request_end_date=request_end_date,
                    base_url=base_url,
                    timeout_seconds=timeout_seconds,
                )
                if save_raw:
                    raw_xml_path = save_raw_xml(
                        xml_text,
                        ana_root_dir=ana_root_dir,
                        station_code=station_code,
                        request_start_date=request_start_date,
                        request_end_date=request_end_date,
                    )
                else:
                    raw_xml_path = None
                frame = parse_response(xml_text)
                if not frame.empty:
                    frame = frame[frame["station_code"].astype(str) == normalize_ana_station_code(station_code)]
                    frames.append(frame)
                if logger is not None:
                    logger.info(
                        "station_window_fetched station_id=%s station_code=%s window_start=%s window_end=%s raw_xml=%s rows=%s",
                        station_id,
                        station_code,
                        request_start_date.isoformat(),
                        request_end_date.isoformat(),
                        raw_xml_path,
                        len(frame),
                    )

            combined = (
                pd.concat(frames, ignore_index=True)
                if frames
                else pd.DataFrame(columns=["station_code", "observed_at", *OBSERVED_VARIABLES])
            )
            write_normalized_csv(
                combined,
                output_path=csv_path,
                station_id=station_id,
                provider_code="ana",
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

    return ObservedFetchSummary(run_id=run_id, provider_code="ana", stations=tuple(station_summaries))
