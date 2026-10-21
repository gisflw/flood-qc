from __future__ import annotations

import csv
from datetime import date
import math
import os
import re
import sqlite3
import tempfile
from pathlib import Path



def apply_schema(database_path: Path, schema_path: Path) -> None:
    database_path.parent.mkdir(parents=True, exist_ok=True)
    schema_sql = schema_path.read_text(encoding="utf-8")
    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute("PRAGMA busy_timeout = 5000")
        if Path(schema_path).name == "history_schema.sql":
            _remove_history_legacy_entities(connection)
        connection.executescript(schema_sql)
        connection.commit()


def _remove_history_legacy_entities(connection: sqlite3.Connection) -> None:
    """Destructively discard pre-artifact operational state from history."""
    for trigger in ("trg_manual_edit_no_overlap_insert", "trg_manual_edit_no_overlap_update"):
        connection.execute(f"DROP TRIGGER IF EXISTS {trigger}")
    for index in ("idx_qc_flag_scope", "idx_manual_edit_asset_step", "idx_run_catalog_status"):
        connection.execute(f"DROP INDEX IF EXISTS {index}")
    for table in ("qc_flag", "manual_edit", "run_catalog"):
        connection.execute(f"DROP TABLE IF EXISTS {table}")


def _normalize_station_code(provider_code: str, station_code: str) -> str:
    normalized = str(station_code).strip().upper()
    if not normalized:
        raise ValueError("Empty station_code is not supported.")
    return normalized.lstrip("0") or "0"


def _parse_nullable_int(value: str) -> int | None:
    normalized = str(value or "").strip()
    if not normalized:
        return None
    normalized = normalized.replace(",", "")
    return int(normalized)


def _parse_nullable_real(value: str, *, row_label: str, column: str) -> float | None:
    normalized = str(value or "").strip()
    if not normalized:
        return None
    try:
        parsed = float(normalized)
    except ValueError as exc:
        raise ValueError(f"{row_label}: {column} must be numeric.") from exc
    if not math.isfinite(parsed):
        raise ValueError(f"{row_label}: {column} must be finite.")
    return parsed



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
        "river_name",
        "drainage_area_km2",
        "municipality",
        "state",
        "responsible_agency",
        "operator",
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
            provider_code = str(raw_row["provider_code"] or "").strip().lower()
            if provider_code not in {"ana", "inmet"}:
                raise ValueError(f"{row_label}: unsupported observed provider_code {provider_code!r}.")
            station_code = _normalize_station_code(provider_code, str(raw_row["station_code"] or ""))
            station_name = str(raw_row["station_name"] or "").strip()
            if not station_name:
                raise ValueError(f"{row_label}: station_name is required.")
            mini_id = _parse_nullable_int(raw_row["mini_id"])
            latitude = _parse_nullable_real(raw_row["latitude"], row_label=row_label, column="latitude")
            longitude = _parse_nullable_real(raw_row["longitude"], row_label=row_label, column="longitude")
            altitude_m = _parse_nullable_real(raw_row["altitude_m"], row_label=row_label, column="altitude_m")
            drainage_area_km2 = _parse_nullable_real(
                raw_row["drainage_area_km2"], row_label=row_label, column="drainage_area_km2"
            )
            if latitude is not None and not -90 <= latitude <= 90:
                raise ValueError(f"{row_label}: latitude must be between -90 and 90.")
            if longitude is not None and not -180 <= longitude <= 180:
                raise ValueError(f"{row_label}: longitude must be between -180 and 180.")
            if drainage_area_km2 is not None and drainage_area_km2 < 0:
                raise ValueError(f"{row_label}: drainage_area_km2 must be >= 0.")
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
                    raw_row["river_name"] or None,
                    drainage_area_km2,
                    raw_row["municipality"] or None,
                    raw_row["state"] or None,
                    raw_row["responsible_agency"] or None,
                    raw_row["operator"] or None,
                )
            )
            capability_rows.extend((station_id, variable_code) for variable_code in observed_variables)

    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        existing_station_ids = {
            str(row[0]) for row in connection.execute("SELECT station_id FROM station")
        }
        removed_station_ids = existing_station_ids.difference(seen_station_ids)
        if removed_station_ids:
            connection.executemany(
                "DELETE FROM station WHERE station_id = ?",
                [(station_id,) for station_id in sorted(removed_station_ids)],
            )
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
                altitude_m,
                river_name,
                drainage_area_km2,
                municipality,
                state,
                responsible_agency,
                operator
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(provider_code, station_code) DO UPDATE SET
                station_name = excluded.station_name,
                mini_id = excluded.mini_id,
                latitude = excluded.latitude,
                longitude = excluded.longitude,
                altitude_m = excluded.altitude_m,
                river_name = excluded.river_name,
                drainage_area_km2 = excluded.drainage_area_km2,
                municipality = excluded.municipality,
                state = excluded.state,
                responsible_agency = excluded.responsible_agency,
                operator = excluded.operator
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

def _read_reference_csv(csv_path: Path, required_columns: set[str]) -> list[dict[str, str]]:
    if not csv_path.exists():
        raise FileNotFoundError(f"Reference CSV not found: {csv_path}")
    with csv_path.open("r", encoding="utf-8-sig", newline="") as handle:
        reader = csv.DictReader(handle)
        missing = required_columns.difference(reader.fieldnames or [])
        if missing:
            raise ValueError(f"Invalid reference CSV at {csv_path}: missing columns {sorted(missing)}")
        rows = list(reader)
    if any(None in row for row in rows):
        raise ValueError(f"Invalid reference CSV at {csv_path}: found rows with too many columns.")
    return rows


def _parse_reference_level(value: str, *, row_label: str, column: str) -> float | None:
    text = str(value or "").strip()
    if not text:
        return None
    try:
        level = float(text)
    except ValueError as exc:
        raise ValueError(f"{row_label}: {column} must be numeric.") from exc
    if not math.isfinite(level):
        raise ValueError(f"{row_label}: {column} must be finite.")
    return level


def load_station_reference_levels(
    database_path: Path, boundaries_csv_path: Path, historical_floods_csv_path: Path,
) -> tuple[int, int]:
    """Replace station reference levels from immutable ANA source CSVs."""
    boundaries_path, floods_path = Path(boundaries_csv_path), Path(historical_floods_csv_path)
    boundary_source = _read_reference_csv(boundaries_path, {
        "station_code", "station_name", "level_atention", "level_alert", "level_flood", "level_severe",
    })
    flood_source = _read_reference_csv(floods_path, {"station_code", "level_registered", "date"})
    station_codes = {
        _normalize_station_code("ana", str(row["station_code"]))
        for row in [*boundary_source, *flood_source]
    }
    with sqlite3.connect(database_path) as connection:
        station_ids = {
            str(code): str(station_id)
            for code, station_id in connection.execute(
                "SELECT station_code, station_id FROM station WHERE provider_code = 'ana'"
            )
        }

    boundary_rows: list[tuple[str, str, float]] = []
    fields = {"attention": "level_atention", "alert": "level_alert", "flood": "level_flood", "severe": "level_severe"}
    seen_boundaries: set[tuple[str, str]] = set()
    for number, row in enumerate(boundary_source, start=2):
        station_code = _normalize_station_code("ana", row["station_code"])
        if station_code not in station_ids:
            continue
        for code, column in fields.items():
            level = _parse_reference_level(row[column], row_label=f"{boundaries_path}:{number}", column=column)
            if level is None:
                continue
            key = (station_code, code)
            if key in seen_boundaries:
                raise ValueError(f"{boundaries_path}:{number}: duplicate {code} boundary for station {station_code}.")
            seen_boundaries.add(key)
            boundary_rows.append((station_ids[station_code], code, level))

    flood_rows: list[tuple[str, float, str]] = []
    seen_floods: set[tuple[str, float, str]] = set()
    for number, row in enumerate(flood_source, start=2):
        station_code = _normalize_station_code("ana", row["station_code"])
        if station_code not in station_ids:
            continue
        label = f"{floods_path}:{number}"
        level = _parse_reference_level(row["level_registered"], row_label=label, column="level_registered")
        if level is None:
            raise ValueError(f"{label}: level_registered is required.")
        try:
            event_date = date.fromisoformat(str(row["date"]).strip()).isoformat()
        except ValueError as exc:
            raise ValueError(f"{label}: date must be an ISO YYYY-MM-DD value.") from exc
        item = (station_ids[station_code], level, event_date)
        if item in seen_floods:
            raise ValueError(f"{label}: duplicate historical flood record.")
        seen_floods.add(item)
        flood_rows.append(item)

    with sqlite3.connect(database_path) as connection:
        connection.execute("PRAGMA foreign_keys = ON")
        connection.execute("DELETE FROM station_level_reference")
        connection.execute("DELETE FROM historical_flood_level")
        connection.executemany(
            "INSERT INTO station_level_reference (station_id, reference_code, level_cm) VALUES (?, ?, ?)", boundary_rows,
        )
        connection.executemany(
            "INSERT INTO historical_flood_level (station_id, level_cm, event_date) VALUES (?, ?, ?)", flood_rows,
        )
        connection.commit()
    return len(boundary_rows), len(flood_rows)



def initialize_history_db(
    database_path: Path,
    inventory_csv_path: Path,
    schema_path: Path,
    boundaries_csv_path: Path | None = None,
    historical_floods_csv_path: Path | None = None,
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
        HistoryRepository.validate_database(target)
        apply_schema(target, Path(schema_path))
        load_history_station_inventory(target, inventory_csv_path)
        if boundaries_csv_path is not None or historical_floods_csv_path is not None:
            if boundaries_csv_path is None or historical_floods_csv_path is None:
                raise ValueError("Both reference-level CSV paths must be provided together.")
            load_station_reference_levels(target, boundaries_csv_path, historical_floods_csv_path)
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
        if boundaries_csv_path is not None or historical_floods_csv_path is not None:
            if boundaries_csv_path is None or historical_floods_csv_path is None:
                raise ValueError("Both reference-level CSV paths must be provided together.")
            load_station_reference_levels(temporary, boundaries_csv_path, historical_floods_csv_path)
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
    """Create a fresh current artifact database; legacy contents are discarded."""
    target = Path(database_path)
    del run_id
    if target.exists():
        target.unlink()
    apply_schema(target, Path(schema_path))
    with sqlite3.connect(target) as connection:
        connection.execute("INSERT INTO current_execution (singleton, status) VALUES (1, 'never')")
        connection.commit()
    return target
