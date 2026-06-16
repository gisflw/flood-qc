from __future__ import annotations

import csv
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Iterable

from mgb_ops.common.time_utils import TIMEZONE
from mgb_ops.storage.history_repository import HistoryRepository

NORMALIZED_OBSERVED_COLUMNS = (
    "station_id",
    "provider_code",
    "station_code",
    "observed_at",
    "variable_code",
    "value",
    "state",
)
ALLOWED_STATES = {"raw", "curated", "approved"}


@dataclass(frozen=True, slots=True)
class ObservedCsvImportSummary:
    files_total: int
    rows_total: int
    rows_imported: int
    values_by_variable: dict[str, int] = field(default_factory=dict)


def normalize_observed_timestamp(value: str, *, row_label: str) -> str:
    text = str(value or "").strip()
    if not text:
        raise ValueError(f"{row_label}: observed_at is required.")
    candidate = text[:-1] + "+00:00" if text.endswith("Z") else text
    try:
        observed_at = datetime.fromisoformat(candidate)
    except ValueError:
        for fmt in ("%Y-%m-%d %H:%M", "%Y-%m-%d %H:%M:%S"):
            try:
                observed_at = datetime.strptime(text, fmt)
                break
            except ValueError:
                continue
        else:
            raise ValueError(f"{row_label}: invalid observed_at {text!r}.")
    if observed_at.tzinfo is not None:
        observed_at = observed_at.astimezone(TIMEZONE).replace(tzinfo=None)
    return observed_at.replace(second=0, microsecond=0).strftime("%Y-%m-%d %H:%M")


def _load_catalogs(repository: HistoryRepository) -> tuple[dict[str, dict[str, str]], set[str]]:
    stations = {
        str(row["station_id"]): {
            "provider_code": str(row["provider_code"]),
            "station_code": str(row["station_code"]),
        }
        for row in repository.connection.execute(
            "SELECT station_id, provider_code, station_code FROM station"
        ).fetchall()
    }
    variables = {
        str(row["variable_code"])
        for row in repository.connection.execute("SELECT variable_code FROM variable").fetchall()
    }
    return stations, variables


def load_normalized_observed_csvs(
    database_path: Path,
    csv_paths: Iterable[Path],
) -> ObservedCsvImportSummary:
    paths = [Path(path) for path in csv_paths]
    if not paths:
        return ObservedCsvImportSummary(files_total=0, rows_total=0, rows_imported=0, values_by_variable={})

    rows_total = 0
    deduped: dict[tuple[str, str, str, str], float] = {}

    with HistoryRepository(database_path) as repository:
        stations, variables = _load_catalogs(repository)

        for path in paths:
            if not path.exists():
                raise FileNotFoundError(f"Normalized observed CSV not found: {path}")
            with path.open("r", encoding="utf-8", newline="") as handle:
                reader = csv.DictReader(handle)
                missing = set(NORMALIZED_OBSERVED_COLUMNS).difference(reader.fieldnames or [])
                if missing:
                    raise ValueError(f"Invalid normalized observed CSV at {path}: missing columns {sorted(missing)}")

                for row_number, row in enumerate(reader, start=2):
                    rows_total += 1
                    row_label = f"{path}:{row_number}"
                    station_id = str(row.get("station_id") or "").strip()
                    provider_code = str(row.get("provider_code") or "").strip().lower()
                    station_code = str(row.get("station_code") or "").strip()
                    variable_code = str(row.get("variable_code") or "").strip().lower()
                    state = str(row.get("state") or "").strip().lower()

                    station = stations.get(station_id)
                    if station is None:
                        raise ValueError(f"{row_label}: unknown station_id {station_id!r}.")
                    if provider_code != station["provider_code"] or station_code != station["station_code"]:
                        raise ValueError(
                            f"{row_label}: station_id {station_id!r} does not match "
                            f"provider_code={provider_code!r} station_code={station_code!r}."
                        )
                    if variable_code not in variables:
                        raise ValueError(f"{row_label}: unsupported variable_code {variable_code!r}.")
                    if state not in ALLOWED_STATES:
                        raise ValueError(f"{row_label}: unsupported state {state!r}.")

                    observed_at = normalize_observed_timestamp(row.get("observed_at", ""), row_label=row_label)
                    value_text = str(row.get("value") or "").strip()
                    if not value_text:
                        raise ValueError(f"{row_label}: value is required.")
                    try:
                        value = float(value_text)
                    except ValueError as exc:
                        raise ValueError(f"{row_label}: invalid numeric value {value_text!r}.") from exc

                    deduped[(station_id, variable_code, state, observed_at)] = value

        grouped: dict[tuple[str, str, str], list[tuple[str, float]]] = {}
        for (station_id, variable_code, state, observed_at), value in deduped.items():
            grouped.setdefault((station_id, variable_code, state), []).append((observed_at, value))

        values_by_variable: dict[str, int] = {}
        rows_imported = 0
        for (station_id, variable_code, state), values in sorted(grouped.items()):
            series_id = repository.ensure_observed_series(station_id, variable_code, state)
            ordered_values = sorted(values, key=lambda item: item[0])
            written = repository.upsert_observed_values(series_id, ordered_values)
            values_by_variable[variable_code] = values_by_variable.get(variable_code, 0) + written
            rows_imported += written

    return ObservedCsvImportSummary(
        files_total=len(paths),
        rows_total=rows_total,
        rows_imported=rows_imported,
        values_by_variable=values_by_variable,
    )


def import_normalized_observed_csvs(
    database_path: Path,
    csv_paths: Iterable[Path],
) -> ObservedCsvImportSummary:
    return load_normalized_observed_csvs(database_path, csv_paths)
