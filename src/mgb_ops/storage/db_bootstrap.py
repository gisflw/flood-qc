from __future__ import annotations

import csv
import sqlite3
import unicodedata
from decimal import Decimal, ROUND_DOWN
from pathlib import Path

from mgb_ops.adapters import get_observation_adapter


def apply_schema(database_path: Path, schema_path: Path) -> None:
    database_path.parent.mkdir(parents=True, exist_ok=True)
    schema_sql = schema_path.read_text(encoding="utf-8")
    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute("PRAGMA busy_timeout = 5000")
        connection.executescript(schema_sql)
        _migrate_station_mini_id(connection)
        connection.commit()


def _migrate_station_mini_id(connection: sqlite3.Connection) -> None:
    station_columns = {
        row[1]
        for row in connection.execute("PRAGMA table_info(station)").fetchall()
    }
    if station_columns and "mini_id" not in station_columns:
        connection.execute("ALTER TABLE station ADD COLUMN mini_id INTEGER")


def _normalize_station_name(name: str) -> str:
    normalized = unicodedata.normalize("NFKD", name.strip())
    return normalized.encode("ascii", "ignore").decode("ascii").upper()


def _normalize_station_code(provider_code: str, station_code: str) -> str:
    normalized = get_observation_adapter(provider_code).normalize_station_code(station_code)
    if normalized is None:
        raise ValueError("Empty station_code is not supported.")
    return normalized


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



def build_station_id(provider_code: str, station_code: str) -> str:
    normalized_provider = provider_code.strip().lower()
    normalized_station_code = _normalize_station_code(normalized_provider, station_code)
    return f"{normalized_provider}:{normalized_station_code}"


def load_history_station_inventory(
    database_path: Path,
    inventory_csv_path: Path,
) -> int:
    inventory_path = Path(inventory_csv_path)
    if not inventory_path.exists():
        raise FileNotFoundError(f"Inventory CSV not found: {inventory_path}")

    required_columns = {
        "provider_code",
        "station_code",
        "station_name",
        "mini_id",
        "latitude",
        "longitude",
        "altitude_m",
    }
    rows_to_insert: list[tuple[object, ...]] = []
    seen_keys: set[tuple[str, str]] = set()
    seen_station_ids: set[str] = set()

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
            mini_id = _parse_nullable_int(raw_row["mini_id"])
            latitude = _parse_nullable_coordinate(raw_row["latitude"])
            longitude = _parse_nullable_coordinate(raw_row["longitude"])
            altitude_m = _parse_nullable_int(raw_row["altitude_m"])

            row_key = (provider_code, station_code)
            if row_key in seen_keys:
                raise ValueError(f"Duplicate station in inventory CSV: {row_key}")
            seen_keys.add(row_key)

            station_id = build_station_id(provider_code, station_code)
            if station_id in seen_station_ids:
                raise ValueError(f"Duplicate station_id in inventory CSV for {row_key}: {station_id}")
            seen_station_ids.add(station_id)

            rows_to_insert.append(
                (
                    station_id,
                    station_code,
                    station_name,
                    provider_code,
                    mini_id,
                    latitude,
                    longitude,
                    altitude_m,
                )
            )

    with sqlite3.connect(database_path) as connection:
        connection.executemany(
            """
            INSERT INTO station (
                station_id,
                station_code,
                station_name,
                provider_code,
                mini_id,
                latitude,
                longitude,
                altitude_m
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider_code, station_code) DO UPDATE SET
                station_name = excluded.station_name,
                mini_id = excluded.mini_id,
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
    inventory_csv_path: Path,
    schema_path: Path,
) -> Path:
    target = Path(database_path)
    apply_schema(target, Path(schema_path))
    load_history_station_inventory(target, inventory_csv_path)
    return target


def initialize_run_db(run_id: str, database_path: Path, schema_path: Path) -> Path:
    target = Path(database_path)
    apply_schema(target, Path(schema_path))
    with sqlite3.connect(target) as connection:
        connection.execute(
            "INSERT OR IGNORE INTO run (run_id, reference_time, run_kind, status, parent_run_id, operator, note) "
            "VALUES (?, ?, ?, ?, ?, ?, ?)",
            (run_id, run_id, "automatic", "draft", None, None, None),
        )
        connection.commit()
    return target
