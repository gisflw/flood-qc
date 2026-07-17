from __future__ import annotations

import csv
import sqlite3
from datetime import date, datetime
from pathlib import Path

from db_helpers import initialize_history_db
from mgb_ops.adapters.observed_ana import ObservedFetchStationSummary, ObservedFetchSummary
from mgb_ops.workflows import observed as observed_workflow
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.assets.observations import NORMALIZED_OBSERVED_COLUMNS


def _write_station_csv(path, rows: list[tuple[str, str]] | None = None, *, observed_at: str = "2026-03-10 00:00", value: str = "1.0") -> None:
    rows = rows or [(observed_at, value)]
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=NORMALIZED_OBSERVED_COLUMNS)
        writer.writeheader()
        for row_observed_at, row_value in rows:
            writer.writerow(
                {
                    "station_id": "ana:74100000",
                    "provider_code": "ana",
                    "station_code": "74100000",
                    "observed_at": row_observed_at,
                    "variable_code": "rain",
                    "value": row_value,
                    "state": "raw",
                }
            )


def test_fetch_observed_provider_empty_db_starts_at_window_start_without_import(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    captured_dates = {}
    captured_kwargs = {}

    def fake_fetch(stations, *, request_dates_by_station, downloads_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        captured_kwargs.update(kwargs)
        csv_path = tmp_path / "downloads" / "ana" / run_id / "74100000" / "observed.csv"
        _write_station_csv(csv_path, observed_at="2026-03-10 00:00")
        return ObservedFetchSummary(
            run_id=run_id,
            provider_code="ana",
            stations=(
                ObservedFetchStationSummary(
                    station_id="ana:74100000",
                    station_code="74100000",
                    request_start=date(2026, 3, 10),
                    request_end=date(2026, 3, 12),
                    rows_parsed=1,
                    csv_path=csv_path,
                    no_data=False,
                ),
            ),
        )

    monkeypatch.setattr(
        observed_workflow.get_observation_adapter("ana"),
        "fetch_function",
        fake_fetch,
    )

    summary = observed_workflow.fetch_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        downloads_dir=tmp_path / "downloads",
        station_codes=["74100000"],
        run_id="run",
        fetch_window_days=7,
    )

    with sqlite3.connect(db_path) as connection:
        values_total = connection.execute("SELECT COUNT(*) FROM observed_value").fetchone()[0]

    assert captured_dates["ana:74100000"] == [date(2026, 3, 10), date(2026, 3, 11), date(2026, 3, 12)]
    assert captured_kwargs["fetch_window_days"] == 7
    assert summary.csv_paths == [tmp_path / "downloads" / "ana" / "run" / "74100000" / "observed.csv"]
    assert values_total == 0


def test_fetch_observed_provider_uses_station_variable_capabilities(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    with HistoryRepository(db_path) as repository:
        series_id = repository.ensure_observed_series("ana:2650035", "rain")
        repository.upsert_observed_values(
            series_id,
            [("2026-03-10 00:00", 1.0), ("2026-03-11 00:00", 1.0)],
        )

    captured_dates = {}

    def fake_fetch(stations, *, request_dates_by_station, downloads_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        return ObservedFetchSummary(run_id=run_id, provider_code="ana", stations=())

    monkeypatch.setattr(
        observed_workflow.get_observation_adapter("ana"),
        "fetch_function",
        fake_fetch,
    )

    observed_workflow.fetch_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        downloads_dir=tmp_path / "downloads",
        station_codes=["2650035"],
        run_id="run",
    )

    assert captured_dates["ana:2650035"] == [date(2026, 3, 11), date(2026, 3, 12)]


def test_fetch_observed_provider_skips_stations_without_observed_variables(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    captured_dates = {}

    def fake_fetch(stations, *, request_dates_by_station, downloads_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        return ObservedFetchSummary(run_id=run_id, provider_code="ana", stations=())

    monkeypatch.setattr(
        observed_workflow.get_observation_adapter("ana"),
        "fetch_function",
        fake_fetch,
    )

    observed_workflow.fetch_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        downloads_dir=tmp_path / "downloads",
        station_codes=["74320000"],
        run_id="run",
    )

    assert captured_dates["ana:74320000"] == []


def test_request_dates_by_station_ignores_unsupported_missing_variables(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    with HistoryRepository(db_path) as repository:
        series_id = repository.ensure_observed_series("ana:2650035", "rain")
        repository.upsert_observed_values(
            series_id,
            [("2026-03-10 00:00", 1.0), ("2026-03-11 00:00", 1.0)],
        )
        stations = [
            station
            for station in repository.get_provider_stations("ana")
            if station["station_id"] == "ana:2650035"
        ]
        requests = observed_workflow._request_dates_by_station(
            repository,
            stations,
            window_start=datetime(2026, 3, 10, 0),
            window_end=datetime(2026, 3, 12, 23),
            variable_codes=("rain", "level", "flow"),
        )

    assert requests["ana:2650035"] == [date(2026, 3, 12)]


def test_fetch_observed_provider_resumes_from_latest_day_with_overlap(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    with HistoryRepository(db_path) as repository:
        series_id = repository.ensure_observed_series("ana:74100000", "rain")
        repository.upsert_observed_values(series_id, [("2026-03-11 15:00", 1.0)])

    captured_dates = {}

    def fake_fetch(stations, *, request_dates_by_station, downloads_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        csv_path = tmp_path / "downloads" / "ana" / run_id / "74100000" / "observed.csv"
        _write_station_csv(csv_path, observed_at="2026-03-11 15:00", value="2.0")
        return ObservedFetchSummary(
            run_id=run_id,
            provider_code="ana",
            stations=(
                ObservedFetchStationSummary(
                    station_id="ana:74100000",
                    station_code="74100000",
                    request_start=date(2026, 3, 11),
                    request_end=date(2026, 3, 12),
                    rows_parsed=1,
                    csv_path=csv_path,
                    no_data=False,
                ),
            ),
        )

    monkeypatch.setattr(
        observed_workflow.get_observation_adapter("ana"),
        "fetch_function",
        fake_fetch,
    )

    observed_workflow.fetch_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        downloads_dir=tmp_path / "downloads",
        station_codes=["74100000"],
        run_id="run",
    )

    with HistoryRepository(db_path) as repository:
        latest = repository.get_latest_observed_at("ana:74100000", variable_codes=["rain"])

    assert captured_dates["ana:74100000"] == [date(2026, 3, 11), date(2026, 3, 12)]
    assert latest == datetime(2026, 3, 11, 15, 0)


def test_load_observed_provider_csvs_imports_existing_csvs_with_timestep_aggregation(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    csv_path = tmp_path / "downloads" / "ana" / "run" / "74100000" / "observed.csv"
    _write_station_csv(
        csv_path,
        [
            ("2026-03-10 01:00", "1.5"),
            ("2026-03-10 02:00", "2.5"),
        ],
    )

    summary = observed_workflow.load_observed_provider_csvs(
        "ana",
        database_path=db_path,
        csv_paths=[csv_path],
        timestep_hours=3,
        observed_aggregation={"rain": "sum", "level": "mean", "flow": "mean"},
    )

    with sqlite3.connect(db_path) as connection:
        rain_values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.rain.raw' ORDER BY observed_at"
        ).fetchall()

    assert summary.files_total == 1
    assert summary.rows_total == 2
    assert summary.rows_imported == 1
    assert rain_values == [("2026-03-10 03:00", 4.0)]


def test_discover_observed_provider_csvs_filters_run_and_station(tmp_path) -> None:
    paths = [
        tmp_path / "downloads" / "ana" / "run-a" / "74100000" / "observed.csv",
        tmp_path / "downloads" / "ana" / "run-a" / "74200000" / "observed.csv",
        tmp_path / "downloads" / "ana" / "run-b" / "74100000" / "observed.csv",
        tmp_path / "downloads" / "inmet" / "run-a" / "A801" / "observed.csv",
    ]
    for path in paths:
        _write_station_csv(path)

    discovered = observed_workflow.discover_observed_provider_csvs(
        tmp_path / "downloads",
        "ana",
        run_id="run-a",
        station_codes=["74100000"],
    )
    all_ana = observed_workflow.discover_observed_provider_csvs(tmp_path / "downloads", "ana")
    missing = observed_workflow.discover_observed_provider_csvs(Path(tmp_path / "missing"), "ana")

    assert discovered == [tmp_path / "downloads" / "ana" / "run-a" / "74100000" / "observed.csv"]
    assert all_ana == sorted(paths[:3])
    assert missing == []
