from __future__ import annotations

import csv
import sqlite3
import unicodedata
from decimal import Decimal, ROUND_DOWN
from pathlib import Path

from mgb_ops.common.paths import (
    SQL_DIR,
    build_run_db_path,
    history_station_inventory_csv_path,
)


PROVIDER_UID_BASES = {
    "ana": 1_000_000_000,
    "inmet": 2_000_000_000,
}


def apply_schema(database_path: Path, schema_path: Path) -> None:
    database_path.parent.mkdir(parents=True, exist_ok=True)
    schema_sql = schema_path.read_text(encoding="utf-8")
    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute("PRAGMA busy_timeout = 5000")
        connection.executescript(schema_sql)
        connection.commit()


def _normalize_station_name(name: str) -> str:
    normalized = unicodedata.normalize("NFKD", name.strip())
    return normalized.encode("ascii", "ignore").decode("ascii").upper()


def _normalize_station_code(provider_code: str, station_code: str) -> str:
    normalized = station_code.strip()
    if not normalized:
        raise ValueError("Empty station_code is not supported.")
    if provider_code == "ana":
        return normalized.lstrip("0") or "0"
    if provider_code == "inmet":
        return normalized.upper()
    raise ValueError(f"Unsupported provider_code for station_code: {provider_code!r}")


def _parse_nullable_int(value: str) -> int | None:
    normalized = value.strip()
    if not normalized:
        return None
    normalized = normalized.replace(",", "")
    return int(normalized)


def _parse_nullable_coordinate(value: str) -> float | None:
    normalized = value.strip()
    if not normalized:
        return None
    return float(Decimal(normalized).quantize(Decimal("0.0001"), rounding=ROUND_DOWN))


def _station_code_to_int(station_code: str) -> int:
    pieces: list[str] = []
    for char in station_code.strip().upper():
        if char.isdigit():
            pieces.append(char)
        elif "A" <= char <= "Z":
            pieces.append(str(ord(char) - ord("A") + 1))
        else:
            raise ValueError(f"Invalid character in station_code: {station_code!r}")
    if not pieces:
        raise ValueError("Empty station_code is not supported.")
    return int("".join(pieces))


def build_station_uid(provider_code: str, station_code: str) -> int:
    try:
        provider_base = PROVIDER_UID_BASES[provider_code]
    except KeyError as exc:
        raise ValueError(f"Unsupported provider_code for station_uid: {provider_code!r}") from exc
    return provider_base + _station_code_to_int(station_code)


def load_history_station_inventory(
    database_path: Path,
    inventory_csv_path: Path | None = None,
) -> int:
    inventory_path = inventory_csv_path or history_station_inventory_csv_path()
    if inventory_csv_path is None and not inventory_path.exists():
        development_inventory_path = (
            Path(__file__).resolve().parents[3]
            / "examples"
            / "rs_hydro"
            / "data"
            / "interim"
            / "history_station_inventory.csv"
        )
        if development_inventory_path.exists():
            inventory_path = development_inventory_path
    if not inventory_path.exists():
        raise FileNotFoundError(f"Inventory CSV not found: {inventory_path}")

    required_columns = {
        "provider_code",
        "station_code",
        "station_name",
        "latitude",
        "longitude",
        "altitude_m",
    }
    rows_to_insert: list[tuple[object, ...]] = []
    seen_keys: set[tuple[str, str]] = set()
    seen_uids: set[int] = set()

    with inventory_path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        missing_columns = required_columns.difference(reader.fieldnames or [])
        if missing_columns:
            raise ValueError(
                f"Invalid inventory CSV at {inventory_path}: missing columns {sorted(missing_columns)}"
            )

        for raw_row in reader:
            provider_code = raw_row["provider_code"].strip().lower()
            station_code = _normalize_station_code(provider_code, raw_row["station_code"])
            station_name = _normalize_station_name(raw_row["station_name"])
            latitude = _parse_nullable_coordinate(raw_row["latitude"])
            longitude = _parse_nullable_coordinate(raw_row["longitude"])
            altitude_m = _parse_nullable_int(raw_row["altitude_m"])

            row_key = (provider_code, station_code)
            if row_key in seen_keys:
                raise ValueError(f"Duplicate station in inventory CSV: {row_key}")
            seen_keys.add(row_key)

            station_uid = build_station_uid(provider_code, station_code)
            if station_uid in seen_uids:
                raise ValueError(f"Duplicate calculated station_uid for {row_key}: {station_uid}")
            seen_uids.add(station_uid)

            rows_to_insert.append(
                (
                    station_uid,
                    station_code,
                    station_name,
                    provider_code,
                    latitude,
                    longitude,
                    altitude_m,
                )
            )

    with sqlite3.connect(database_path) as connection:
        connection.executemany(
            """
            INSERT INTO station (
                station_uid,
                station_code,
                station_name,
                provider_code,
                latitude,
                longitude,
                altitude_m
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider_code, station_code) DO UPDATE SET
                station_name = excluded.station_name,
                latitude = excluded.latitude,
                longitude = excluded.longitude,
                altitude_m = excluded.altitude_m
            """,
            rows_to_insert,
        )
        connection.commit()

    return len(rows_to_insert)


def initialize_history_db(
    database_path: Path,
    inventory_csv_path: Path | None = None,
) -> Path:
    target = database_path
    apply_schema(target, SQL_DIR / "history_schema.sql")
    load_history_station_inventory(target, inventory_csv_path)
    return target


def initialize_run_db(run_id: str, database_path: Path | None = None) -> Path:
    target = database_path or build_run_db_path(run_id)
    apply_schema(target, SQL_DIR / "run_schema.sql")
    with sqlite3.connect(target) as connection:
        connection.execute(
            "INSERT OR IGNORE INTO run (run_id, reference_time, run_kind, status, parent_run_id, operator, note) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (run_id, run_id, "automatic", "draft", None, None, None),
        )
        connection.commit()
    return target
