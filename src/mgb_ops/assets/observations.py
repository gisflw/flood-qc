from __future__ import annotations

import csv
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from pathlib import Path
from typing import Iterable

from mgb_ops.common.time_utils import TIMEZONE
from mgb_ops.common.time_utils import validate_timestep_hours
from mgb_ops.assets.history import HistoryRepository

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
DEFAULT_OBSERVED_AGGREGATION = {
    "rain": "sum",
    "level": "mean",
    "flow": "mean",
}
ALLOWED_AGGREGATIONS = {"sum", "mean", "last"}



def write_normalized_observed_csv(
    frame,
    *,
    output_path: Path,
    station_id: str,
    provider_code: str,
    station_code: str,
    variable_columns: Iterable[str],
    state: str = "raw",
) -> Path:
    """Serialize provider-normalized values using the canonical observation CSV contract."""
    if state not in ALLOWED_STATES:
        raise ValueError(f"Unsupported observation state: {state!r}")
    target = Path(output_path)
    target.parent.mkdir(parents=True, exist_ok=True)
    with target.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=NORMALIZED_OBSERVED_COLUMNS)
        writer.writeheader()
        if frame.empty:
            return target
        for record in frame.sort_values("observed_at").to_dict("records"):
            observed_at = record["observed_at"].strftime("%Y-%m-%d %H:%M")
            for variable in variable_columns:
                value = record.get(variable)
                if value is None:
                    continue
                try:
                    if value != value:
                        continue
                except TypeError:
                    pass
                writer.writerow({
                    "station_id": station_id,
                    "provider_code": provider_code,
                    "station_code": station_code,
                    "observed_at": observed_at,
                    "variable_code": variable,
                    "value": float(value),
                    "state": state,
                })
    return target


@dataclass(frozen=True, slots=True)
class ObservedCsvImportSummary:
    files_total: int
    rows_total: int
    rows_imported: int
    values_by_variable: dict[str, int] = field(default_factory=dict)


def normalize_observed_timestamp(value: str, *, row_label: str) -> str:
    return _parse_observed_timestamp(value, row_label=row_label).strftime("%Y-%m-%d %H:%M")


def _parse_observed_timestamp(value: str, *, row_label: str) -> datetime:
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
    return observed_at.replace(second=0, microsecond=0)


def _normalize_aggregation_policy(aggregation_by_variable: dict[str, str] | None) -> dict[str, str]:
    policy = dict(DEFAULT_OBSERVED_AGGREGATION)
    if aggregation_by_variable is not None:
        policy.update({str(key).strip().lower(): str(value).strip().lower() for key, value in aggregation_by_variable.items()})
    for variable_code, method in policy.items():
        if method not in ALLOWED_AGGREGATIONS:
            raise ValueError(
                f"Unsupported aggregation method {method!r} for variable_code {variable_code!r}; "
                f"expected one of {sorted(ALLOWED_AGGREGATIONS)}."
            )
    return policy


def _ceil_to_timestep_end(observed_at: datetime, *, timestep_hours: int) -> datetime:
    timestep_hours = validate_timestep_hours(timestep_hours)
    day_start = observed_at.replace(hour=0, minute=0, second=0, microsecond=0)
    elapsed_minutes = int((observed_at - day_start).total_seconds() // 60)
    step_minutes = timestep_hours * 60
    bucket_minutes = ((elapsed_minutes + step_minutes - 1) // step_minutes) * step_minutes
    return day_start + timedelta(minutes=bucket_minutes)


def _aggregate_values(values: list[float], *, method: str, row_label: str) -> float:
    if not values:
        raise ValueError(f"{row_label}: no values to aggregate.")
    if method == "sum":
        return float(sum(values))
    if method == "mean":
        return float(sum(values) / len(values))
    if method == "last":
        return float(values[-1])
    raise ValueError(f"{row_label}: unsupported aggregation method {method!r}.")


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
    *,
    timestep_hours: int = 1,
    aggregation_by_variable: dict[str, str] | None = None,
) -> ObservedCsvImportSummary:
    timestep_hours = validate_timestep_hours(timestep_hours)
    aggregation_policy = _normalize_aggregation_policy(aggregation_by_variable)
    paths = [Path(path) for path in csv_paths]
    if not paths:
        return ObservedCsvImportSummary(files_total=0, rows_total=0, rows_imported=0, values_by_variable={})

    rows_total = 0
    grouped_values: dict[tuple[str, str, str, str], list[float]] = {}

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

                    method = aggregation_policy.get(variable_code)
                    if method is None:
                        raise ValueError(f"{row_label}: missing aggregation policy for variable_code {variable_code!r}.")

                    observed_at = _parse_observed_timestamp(row.get("observed_at", ""), row_label=row_label)
                    bucket_end = _ceil_to_timestep_end(observed_at, timestep_hours=timestep_hours)
                    observed_at_text = bucket_end.strftime("%Y-%m-%d %H:%M")
                    value_text = str(row.get("value") or "").strip()
                    if not value_text:
                        raise ValueError(f"{row_label}: value is required.")
                    try:
                        value = float(value_text)
                    except ValueError as exc:
                        raise ValueError(f"{row_label}: invalid numeric value {value_text!r}.") from exc

                    grouped_values.setdefault((station_id, variable_code, state, observed_at_text), []).append(value)

        grouped: dict[tuple[str, str, str], list[tuple[str, float]]] = {}
        for (station_id, variable_code, state, observed_at), values in grouped_values.items():
            row_label = f"{station_id} {variable_code} {state} {observed_at}"
            method = aggregation_policy[variable_code]
            grouped.setdefault((station_id, variable_code, state), []).append(
                (observed_at, _aggregate_values(values, method=method, row_label=row_label))
            )

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
    *,
    timestep_hours: int = 1,
    aggregation_by_variable: dict[str, str] | None = None,
) -> ObservedCsvImportSummary:
    return load_normalized_observed_csvs(
        database_path,
        csv_paths,
        timestep_hours=timestep_hours,
        aggregation_by_variable=aggregation_by_variable,
    )
