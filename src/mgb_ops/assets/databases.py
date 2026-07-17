from __future__ import annotations

import csv
import os
import re
import sqlite3
import tempfile
import unicodedata
from decimal import Decimal, ROUND_DOWN
from pathlib import Path



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
    normalized = str(station_code).strip().upper()
    if not normalized:
        raise ValueError("Empty station_code is not supported.")
    return normalized.lstrip("0") or "0"


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



ALLOWED_OBSERVED_VARIABLES = {"rain", "level", "flow"}
OBSERVED_VARIABLES_NONE = "none"


def parse_observed_variables(value: str, *, row_label: str) -> tuple[str, ...]:
    text = str(value or "").strip().lower()
    if not text:
        raise ValueError(f"{row_label}: observed_variables is required.")
    if text == OBSERVED_VARIABLES_NONE:
        return ()

    values = tuple(part.strip().lower() for part in re.split(r"[,;|]", text))
    if any(not part for part in values):
        raise ValueError(f"{row_label}: observed_variables contains an empty value.")

    seen: set[str] = set()
    duplicates: list[str] = []
    unsupported: list[str] = []
    for variable_code in values:
        if variable_code in seen and variable_code not in duplicates:
            duplicates.append(variable_code)
        seen.add(variable_code)
        if variable_code not in ALLOWED_OBSERVED_VARIABLES:
            unsupported.append(variable_code)

    if duplicates:
        raise ValueError(f"{row_label}: observed_variables contains duplicates: {sorted(duplicates)}")
    if unsupported:
        raise ValueError(
            f"{row_label}: observed_variables contains unsupported values: {sorted(set(unsupported))}"
        )
    return values


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
        "observed_variables",
    }
    station_rows: list[tuple[object, ...]] = []
    capability_rows: list[tuple[str, str]] = []
    inventory_station_ids: list[tuple[str]] = []
    seen_keys: set[tuple[str, str]] = set()
    seen_station_ids: set[str] = set()

    with inventory_path.open("r", encoding="utf-8", newline="") as handle:
        reader = csv.DictReader(handle)
        missing_columns = required_columns.difference(reader.fieldnames or [])
        if missing_columns:
            raise ValueError(
                f"Invalid inventory CSV at {inventory_path}: missing columns {sorted(missing_columns)}"
            )

        for row_number, raw_row in enumerate(reader, start=2):
            row_label = f"{inventory_path}:{row_number}"
            if None in raw_row:
                raise ValueError(
                    f"{row_label}: too many columns; quote observed_variables values containing commas."
                )
            provider_code = raw_row["provider_code"].strip().lower()
            station_code = _normalize_station_code(provider_code, raw_row["station_code"])
            station_name = _normalize_station_name(raw_row["station_name"])
            mini_id = _parse_nullable_int(raw_row["mini_id"])
            latitude = _parse_nullable_coordinate(raw_row["latitude"])
            longitude = _parse_nullable_coordinate(raw_row["longitude"])
            altitude_m = _parse_nullable_int(raw_row["altitude_m"])
            observed_variables = parse_observed_variables(
                raw_row["observed_variables"],
                row_label=row_label,
            )

            row_key = (provider_code, station_code)
            if row_key in seen_keys:
                raise ValueError(f"Duplicate station in inventory CSV: {row_key}")
            seen_keys.add(row_key)

            station_id = build_station_id(provider_code, station_code)
            if station_id in seen_station_ids:
                raise ValueError(f"Duplicate station_id in inventory CSV for {row_key}: {station_id}")
            seen_station_ids.add(station_id)
            inventory_station_ids.append((station_id,))

            station_rows.append(
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
            capability_rows.extend((station_id, variable_code) for variable_code in observed_variables)

    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
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
            station_rows,
        )
        connection.executemany(
            "DELETE FROM station_observed_variable WHERE station_id = ?",
            inventory_station_ids,
        )
        connection.executemany(
            """
            INSERT INTO station_observed_variable (station_id, variable_code)
            VALUES (?, ?)
            """,
            capability_rows,
        )
        connection.commit()

    return len(station_rows)


def initialize_history_db(
    database_path: Path,
    inventory_csv_path: Path,
    schema_path: Path,
) -> Path:
    target = Path(database_path)
    target.parent.mkdir(parents=True, exist_ok=True)
    user_tables: set[str] = set()
    if target.exists() and target.stat().st_size:
        try:
            with sqlite3.connect(f"{target.resolve().as_uri()}?mode=ro", uri=True) as connection:
                user_tables = {
                    str(row[0])
                    for row in connection.execute(
                        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
                    )
                }
        except sqlite3.DatabaseError as exc:
            raise RuntimeError(f"History database is not a valid SQLite database: {target}") from exc
    if user_tables:
        from mgb_ops.assets.history import HistoryRepository
        HistoryRepository.validate_database(target, allow_missing_station_observed_variable=True)
        apply_schema(target, Path(schema_path))
        load_history_station_inventory(target, inventory_csv_path)
        HistoryRepository.validate_database(target)
        return target

    descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target.name}.", suffix=".tmp", dir=target.parent
    )
    os.close(descriptor)
    temporary = Path(temporary_name)
    try:
        apply_schema(temporary, Path(schema_path))
        load_history_station_inventory(temporary, inventory_csv_path)
        from mgb_ops.assets.history import HistoryRepository
        HistoryRepository.validate_database(temporary)
        packed = temporary.with_suffix(temporary.suffix + ".packed")
        with sqlite3.connect(temporary) as source, sqlite3.connect(packed) as destination:
            source.backup(destination)
        os.replace(packed, target)
    finally:
        temporary.unlink(missing_ok=True)
        temporary.with_suffix(temporary.suffix + ".packed").unlink(missing_ok=True)
        Path(f"{temporary}-wal").unlink(missing_ok=True)
        Path(f"{temporary}-shm").unlink(missing_ok=True)
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
