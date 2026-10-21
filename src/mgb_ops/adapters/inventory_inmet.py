from __future__ import annotations

import math
from typing import Any

import requests


INMET_INVENTORY_URL = "https://api-bndmet.decea.mil.br/v1/estacoes"


def _normalized_code(value: object) -> str:
    code = str(value or "").strip().upper()
    if not code:
        raise ValueError("INMET inventory record is missing codEstacao.")
    return code.lstrip("0") or "0"


def _optional_text(record: dict[str, Any], key: str) -> str | None:
    value = record.get(key)
    if value is None or not str(value).strip():
        return None
    return str(value).strip()


def _optional_float(record: dict[str, Any], key: str) -> float | None:
    value = record.get(key)
    if value is None or (isinstance(value, str) and not value.strip()):
        return None
    try:
        parsed = float(value)
    except (TypeError, ValueError) as exc:
        raise ValueError(f"Malformed INMET inventory value for {key}: {value!r}.") from exc
    if not math.isfinite(parsed):
        raise ValueError(f"Malformed INMET inventory value for {key}: {value!r}.")
    return parsed


def parse_inventory(payload: Any) -> dict[str, dict[str, Any]]:
    records = payload.get("data") if isinstance(payload, dict) else None
    if not isinstance(records, list):
        raise ValueError("Malformed INMET inventory response: expected data to be a list.")

    result: dict[str, dict[str, Any]] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("Malformed INMET inventory response: station records must be objects.")
        code = _normalized_code(record.get("codEstacao"))
        name = _optional_text(record, "nome")
        if name is None:
            raise ValueError(f"INMET inventory record {code} is missing nome.")
        if code in result:
            raise ValueError(f"INMET inventory returned conflicting duplicate station {code}.")
        result[code] = {
            "station_code": code,
            "station_name": name,
            "river_name": None,
            "drainage_area_km2": None,
            "latitude": _optional_float(record, "latitude"),
            "longitude": _optional_float(record, "longitude"),
            "altitude_m": _optional_float(record, "altitudeEmMetros"),
            "municipality": None,
            "state": _optional_text(record, "estado"),
            "responsible_agency": _optional_text(record, "entidadeResponsavel"),
            "operator": None,
        }
    return result


def fetch_inventory_metadata(
    api_key: str,
    *,
    session: requests.Session | None = None,
    timeout_seconds: float = 30.0,
) -> dict[str, dict[str, Any]]:
    if not api_key.strip():
        raise ValueError("INMET API key is required.")
    response = (session or requests).get(
        INMET_INVENTORY_URL,
        params={"tipo": "automatica"},
        headers={"x-api-key": api_key, "accept": "application/json"},
        timeout=timeout_seconds,
    )
    response.raise_for_status()
    try:
        payload = response.json()
    except ValueError as exc:
        raise ValueError("Malformed INMET inventory JSON response.") from exc
    return parse_inventory(payload)
