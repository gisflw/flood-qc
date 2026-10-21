from __future__ import annotations

import math
import logging
import time
import xml.etree.ElementTree as ET
from typing import Any

import requests


ANA_INVENTORY_URL = "https://telemetriaws1.ana.gov.br/ServiceANA.asmx/HidroInventario"
LOGGER = logging.getLogger(__name__)
ANA_INVENTORY_PARAMS = {
    "tpEst": "",
    "nmEst": "",
    "nmRio": "",
    "codSubBacia": "",
    "codBacia": "",
    "nmMunicipio": "",
    "nmEstado": "",
    "sgResp": "",
    "sgOper": "",
    "telemetrica": "",
}


def _normalized_code(value: object) -> str:
    code = str(value or "").strip()
    if not code or not code.isdigit():
        raise ValueError(f"Invalid ANA station code: {value!r}.")
    return code.lstrip("0") or "0"


def _local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def _fields(element: ET.Element) -> dict[str, str | None]:
    result: dict[str, str | None] = {}
    for child in element.iter():
        name = _local_name(child.tag)
        if child is element or not name:
            continue
        if name in result:
            raise ValueError(f"Malformed ANA inventory record: repeated field {name!r}.")
        value = child.text.strip() if child.text and child.text.strip() else None
        result[name] = value
    return result


def _optional_text(fields: dict[str, str | None], key: str) -> str | None:
    value = fields.get(key)
    return value.strip() if value else None


def _optional_float(fields: dict[str, str | None], key: str) -> float | None:
    value = _optional_text(fields, key)
    if value is None:
        return None
    try:
        parsed = float(value)
    except ValueError as exc:
        raise ValueError(f"Malformed ANA inventory value for {key}: {value!r}.") from exc
    if not math.isfinite(parsed):
        raise ValueError(f"Malformed ANA inventory value for {key}: {value!r}.")
    return parsed


def parse_station_metadata(xml_text: str, station_code: str) -> dict[str, Any]:
    requested = _normalized_code(station_code)
    try:
        root = ET.fromstring(xml_text)
    except ET.ParseError as exc:
        raise ValueError("Malformed ANA inventory XML response.") from exc

    records = [element for element in root.iter() if _local_name(element.tag) == "Table"]
    matches: list[dict[str, str | None]] = []
    for record in records:
        fields = _fields(record)
        code = fields.get("Codigo")
        if code is None:
            raise ValueError("Malformed ANA inventory record: missing Codigo.")
        if _normalized_code(code) == requested:
            matches.append(fields)
    if len(matches) != 1:
        raise ValueError(
            f"ANA inventory returned {len(matches)} records for requested station {requested}; expected exactly one."
        )

    fields = matches[0]
    name = _optional_text(fields, "Nome")
    if name is None:
        raise ValueError(f"ANA inventory record {requested} is missing Nome.")
    return {
        "station_code": requested,
        "station_name": name,
        "river_name": _optional_text(fields, "RioNome"),
        "drainage_area_km2": _optional_float(fields, "AreaDrenagem"),
        "latitude": _optional_float(fields, "Latitude"),
        "longitude": _optional_float(fields, "Longitude"),
        "altitude_m": _optional_float(fields, "Altitude"),
        "municipality": _optional_text(fields, "nmMunicipio"),
        "state": _optional_text(fields, "nmEstado"),
        "responsible_agency": _optional_text(fields, "ResponsavelSigla"),
        "operator": _optional_text(fields, "OperadoraSigla"),
    }


def fetch_station_metadata(
    station_code: str,
    *,
    session: requests.Session | None = None,
    timeout_seconds: float = 30.0,
) -> dict[str, Any]:
    requested = _normalized_code(station_code)
    padded = requested.zfill(8)
    params = {
        "codEstDE": padded,
        "codEstATE": padded,
        **ANA_INVENTORY_PARAMS,
    }
    requester = session or requests
    for attempt in range(5):
        response = requester.get(ANA_INVENTORY_URL, params=params, timeout=timeout_seconds)
        if getattr(response, "status_code", 200) != 429:
            break
        if attempt == 4:
            response.raise_for_status()
        try:
            delay = float(response.headers.get("Retry-After", 2**attempt))
        except (AttributeError, TypeError, ValueError):
            delay = float(2**attempt)
        if not math.isfinite(delay) or delay < 0:
            delay = float(2**attempt)
        delay = min(delay, 30.0)
        LOGGER.warning("ANA inventory rate-limited; retrying after %.1f seconds (%s/5).", delay, attempt + 1)
        time.sleep(delay)
    response.raise_for_status()
    return parse_station_metadata(response.text, requested)
