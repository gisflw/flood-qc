from __future__ import annotations

import csv
import sqlite3

import pytest

from db_helpers import initialize_history_db
from mgb_ops.ingest.observed_csv import NORMALIZED_OBSERVED_COLUMNS, import_normalized_observed_csvs


def write_csv(path, rows, *, columns=NORMALIZED_OBSERVED_COLUMNS):
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def valid_row(**overrides):
    row = {
        "station_id": "ana:74100000",
        "provider_code": "ana",
        "station_code": "74100000",
        "observed_at": "2026-03-10 00:00",
        "variable_code": "rain",
        "value": "1.0",
        "state": "raw",
    }
    row.update(overrides)
    return row


def test_import_normalized_observed_csv_imports_series_and_values(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(
        csv_path,
        [
            valid_row(value="1.0"),
            valid_row(value="2.5"),
            valid_row(observed_at="2026-03-10 01:00", variable_code="level", value="100"),
        ],
    )

    summary = import_normalized_observed_csvs(db_path, [csv_path])

    with sqlite3.connect(db_path) as connection:
        series = connection.execute(
            "SELECT series_id, station_id, variable_code FROM observed_series ORDER BY variable_code"
        ).fetchall()
        values = connection.execute(
            "SELECT series_id, observed_at, value FROM observed_value ORDER BY series_id, observed_at"
        ).fetchall()

    assert summary.rows_total == 3
    assert summary.rows_imported == 2
    assert summary.values_by_variable == {"level": 1, "rain": 1}
    assert series == [
        ("ana:74100000.level.raw", "ana:74100000", "level"),
        ("ana:74100000.rain.raw", "ana:74100000", "rain"),
    ]
    assert values == [
        ("ana:74100000.level.raw", "2026-03-10 01:00", 100.0),
        ("ana:74100000.rain.raw", "2026-03-10 00:00", 2.5),
    ]


def test_import_normalized_observed_csv_rejects_bad_station_id(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(station_id="ana:missing")])

    with pytest.raises(ValueError, match="unknown station_id"):
        import_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_missing_columns(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [{"station_id": "ana:74100000"}], columns=["station_id"])

    with pytest.raises(ValueError, match="missing columns"):
        import_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_bad_timestamp(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(observed_at="not-a-time")])

    with pytest.raises(ValueError, match="invalid observed_at"):
        import_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_unsupported_variable(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(variable_code="wind")])

    with pytest.raises(ValueError, match="unsupported variable_code"):
        import_normalized_observed_csvs(db_path, [csv_path])
