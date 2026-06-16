from __future__ import annotations

import csv
from datetime import date, datetime

from db_helpers import initialize_history_db
from mgb_ops.ingest import observed_workflow
from mgb_ops.ingest.fetch_observed_ana import ObservedFetchStationSummary, ObservedFetchSummary
from mgb_ops.storage.history_repository import HistoryRepository
from mgb_ops.storage.observed_csv import NORMALIZED_OBSERVED_COLUMNS


def _write_station_csv(path, *, observed_at: str, value: str = "1.0") -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=NORMALIZED_OBSERVED_COLUMNS)
        writer.writeheader()
        writer.writerow(
            {
                "station_id": "ana:74100000",
                "provider_code": "ana",
                "station_code": "74100000",
                "observed_at": observed_at,
                "variable_code": "rain",
                "value": value,
                "state": "raw",
            }
        )


def test_fetch_and_load_observed_provider_empty_db_starts_at_window_start(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    captured_dates = {}

    def fake_fetch(stations, *, request_dates_by_station, interim_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        csv_path = tmp_path / "interim" / "ana" / run_id / "74100000" / "observed.csv"
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

    monkeypatch.setattr(observed_workflow, "fetch_observed_ana", fake_fetch)

    summary = observed_workflow.fetch_and_load_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        interim_dir=tmp_path / "interim",
        station_codes=["74100000"],
        run_id="run",
    )

    assert captured_dates["ana:74100000"] == [date(2026, 3, 10), date(2026, 3, 11), date(2026, 3, 12)]
    assert summary.import_summary.rows_imported == 1


def test_fetch_and_load_observed_provider_resumes_from_latest_day_with_overlap(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    with HistoryRepository(db_path) as repository:
        series_id = repository.ensure_observed_series("ana:74100000", "rain")
        repository.upsert_observed_values(series_id, [("2026-03-11 15:00", 1.0)])

    captured_dates = {}

    def fake_fetch(stations, *, request_dates_by_station, interim_dir, run_id, **kwargs):
        captured_dates.update(request_dates_by_station)
        csv_path = tmp_path / "interim" / "ana" / run_id / "74100000" / "observed.csv"
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

    monkeypatch.setattr(observed_workflow, "fetch_observed_ana", fake_fetch)

    observed_workflow.fetch_and_load_observed_provider(
        "ana",
        database_path=db_path,
        window_start=datetime(2026, 3, 10, 0),
        window_end=datetime(2026, 3, 12, 23),
        interim_dir=tmp_path / "interim",
        station_codes=["74100000"],
        run_id="run",
    )

    with HistoryRepository(db_path) as repository:
        latest = repository.get_latest_observed_at("ana:74100000", variable_codes=["rain"])

    assert captured_dates["ana:74100000"] == [date(2026, 3, 11), date(2026, 3, 12)]
    assert latest == datetime(2026, 3, 11, 15, 0)
