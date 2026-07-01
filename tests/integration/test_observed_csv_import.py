from __future__ import annotations

import csv
import sqlite3

import pytest

from db_helpers import initialize_history_db
from mgb_ops.assets.observations import NORMALIZED_OBSERVED_COLUMNS, load_normalized_observed_csvs


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

    summary = load_normalized_observed_csvs(db_path, [csv_path])

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
        ("ana:74100000.rain.raw", "2026-03-10 00:00", 3.5),
    ]


def test_import_normalized_observed_csv_rejects_bad_station_id(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(station_id="ana:missing")])

    with pytest.raises(ValueError, match="unknown station_id"):
        load_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_missing_columns(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [{"station_id": "ana:74100000"}], columns=["station_id"])

    with pytest.raises(ValueError, match="missing columns"):
        load_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_bad_timestamp(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(observed_at="not-a-time")])

    with pytest.raises(ValueError, match="invalid observed_at"):
        load_normalized_observed_csvs(db_path, [csv_path])


def test_import_normalized_observed_csv_rejects_unsupported_variable(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(csv_path, [valid_row(variable_code="wind")])

    with pytest.raises(ValueError, match="unsupported variable_code"):
        load_normalized_observed_csvs(db_path, [csv_path])


def test_load_normalized_observed_csv_upserts_missing_hours_and_duplicates(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    first_csv_path = tmp_path / "first.csv"
    second_csv_path = tmp_path / "second.csv"
    write_csv(
        first_csv_path,
        [
            valid_row(observed_at="2026-03-10 00:00", value="1.0"),
            valid_row(observed_at="2026-03-10 02:00", value="3.0"),
        ],
    )
    write_csv(
        second_csv_path,
        [
            valid_row(observed_at="2026-03-10 01:00", value="2.0"),
            valid_row(observed_at="2026-03-10 02:00", value="4.0"),
        ],
    )

    summary = load_normalized_observed_csvs(db_path, [first_csv_path, second_csv_path])

    with sqlite3.connect(db_path) as connection:
        values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.rain.raw' ORDER BY observed_at"
        ).fetchall()

    assert summary.rows_total == 4
    assert summary.rows_imported == 3
    assert values == [
        ("2026-03-10 00:00", 1.0),
        ("2026-03-10 01:00", 2.0),
        ("2026-03-10 02:00", 7.0),
    ]


def test_load_normalized_observed_csv_normalizes_to_configured_timestep(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(
        csv_path,
        [
            valid_row(observed_at="2026-03-10 00:10", value="1.0"),
            valid_row(observed_at="2026-03-10 02:55", value="2.0"),
            valid_row(observed_at="2026-03-10 03:00", value="4.0"),
        ],
    )

    summary = load_normalized_observed_csvs(db_path, [csv_path], timestep_hours=3)

    with sqlite3.connect(db_path) as connection:
        values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.rain.raw' ORDER BY observed_at"
        ).fetchall()

    assert summary.rows_total == 3
    assert summary.rows_imported == 1
    assert values == [("2026-03-10 03:00", 7.0)]


def test_load_normalized_observed_csv_averages_level_and_flow_by_policy(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "observed.csv"
    write_csv(
        csv_path,
        [
            valid_row(observed_at="2026-03-10 00:10", variable_code="level", value="10.0"),
            valid_row(observed_at="2026-03-10 00:20", variable_code="level", value="20.0"),
            valid_row(observed_at="2026-03-10 00:10", variable_code="flow", value="100.0"),
            valid_row(observed_at="2026-03-10 00:20", variable_code="flow", value="140.0"),
        ],
    )

    summary = load_normalized_observed_csvs(db_path, [csv_path], timestep_hours=1)

    with sqlite3.connect(db_path) as connection:
        values = connection.execute(
            "SELECT series_id, observed_at, value FROM observed_value ORDER BY series_id, observed_at"
        ).fetchall()

    assert summary.values_by_variable == {"flow": 1, "level": 1}
    assert values == [
        ("ana:74100000.flow.raw", "2026-03-10 01:00", 120.0),
        ("ana:74100000.level.raw", "2026-03-10 01:00", 15.0),
    ]
