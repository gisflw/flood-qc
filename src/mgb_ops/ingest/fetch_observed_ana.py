from __future__ import annotations

import csv
import logging
import sys
import xml.etree.ElementTree as ET
from datetime import date, datetime, timedelta
from pathlib import Path

import requests

from mgb_ops.common.time_utils import TIMEZONE, resolve_reference_time
from mgb_ops.ingest.observed_csv import NORMALIZED_OBSERVED_COLUMNS, import_normalized_observed_csvs

DEFAULT_ANA_BASE_URL = "http://telemetriaws1.ana.gov.br/serviceana.asmx/DadosHidrometeorologicos"
OBSERVED_VARIABLES = ("rain", "level", "flow")


def script_stem() -> str:
    return Path(__file__).stem


def build_run_id(reference_time: datetime) -> str:
    return reference_time.strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("ingest.ana")
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
    request_date: date,
    base_url: str,
    timeout_seconds: float,
) -> str:
    params = {
        "codEstacao": station_code,
        "dataInicio": request_date.strftime("%d/%m/%Y"),
        "dataFim": request_date.strftime("%d/%m/%Y"),
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
    request_date: date,
) -> Path:
    station_dir = ana_root_dir / station_code
    station_dir.mkdir(parents=True, exist_ok=True)
    file_stamp = request_date.strftime("%Y%m%d")
    file_path = station_dir / f"{file_stamp}__{file_stamp}.xml"
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
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with output_path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=NORMALIZED_OBSERVED_COLUMNS)
        writer.writeheader()
        if frame.empty:
            return output_path
        for record in frame.sort_values("observed_at").to_dict("records"):
            observed_at = record["observed_at"].strftime("%Y-%m-%d %H:%M")
            for variable in OBSERVED_VARIABLES:
                value = record.get(variable)
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
                        "observed_at": observed_at,
                        "variable_code": variable,
                        "value": float(value),
                        "state": state,
                    }
                )
    return output_path


def build_normalized_csv_path(ana_root_dir: Path, *, station_code: str, request_date: date) -> Path:
    station_dir = ana_root_dir / station_code
    file_stamp = request_date.strftime("%Y%m%d")
    return station_dir / f"{file_stamp}__{file_stamp}.csv"
