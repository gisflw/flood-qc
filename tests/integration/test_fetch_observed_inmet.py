from __future__ import annotations

import json
import sqlite3
import csv
from datetime import date, datetime
from pathlib import Path

import pandas as pd
import pytest

from mgb_ops.ingest import fetch_observed_inmet, observed_workflow
from db_helpers import initialize_history_db


SAMPLE_INMET_PAYLOAD = {
    "data": {
        "data": [
            [1773187200000, 1.0],
            [1773187200000, 2.0],
            [1773190800000, None],
            [1773194400000, 3.5],
        ]
    }
}


class FakeResponse:
    def __init__(self, payload) -> None:
        self._payload = payload

    def raise_for_status(self) -> None:
        return None

    def json(self):
        return self._payload


class FakeSession:
    def __init__(self, responses=None, side_effects=None) -> None:
        self.headers = {}
        self.responses = list(responses or [])
        self.side_effects = list(side_effects or [])
        self.requests: list[dict[str, object]] = []

    def get(self, url, params=None, timeout=None):
        self.requests.append({"url": url, "params": params, "timeout": timeout, "headers": dict(self.headers)})
        if self.side_effects:
            effect = self.side_effects.pop(0)
            if isinstance(effect, Exception):
                raise effect
        if not self.responses:
            raise AssertionError("Nenhuma resposta fake restante para a sessao.")
        return self.responses.pop(0)

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        return None


def test_parse_payload_accepts_pair_rows_and_deduplicates() -> None:
    frame = fetch_observed_inmet.parse_payload(SAMPLE_INMET_PAYLOAD, station_code="a801")

    assert frame["station_code"].tolist() == ["A801", "A801", "A801", "A801"]
    assert frame["observed_at"].tolist() == [
        pd.Timestamp("2026-03-10 21:00:00"),
        pd.Timestamp("2026-03-10 21:00:00"),
        pd.Timestamp("2026-03-10 22:00:00"),
        pd.Timestamp("2026-03-10 23:00:00"),
    ]
    assert frame["rain"].iloc[0] == 1.0
    assert frame["rain"].iloc[1] == 2.0
    assert pd.isna(frame["rain"].iloc[2])
    assert frame["rain"].iloc[3] == 3.5


def test_parse_payload_accepts_dict_rows() -> None:
    payload = {
        "data": {
            "data": [
                {"timestamp": 1773187200000, "value": "1.5", "codigo": "A801"},
                {"timestamp": 1773190800000, "chuva": "2.5", "codigo": "A801"},
            ]
        }
    }

    frame = fetch_observed_inmet.parse_payload(payload, station_code="A801")

    assert frame["station_code"].tolist() == ["A801", "A801"]
    assert frame["rain"].tolist() == [1.5, 2.5]



def test_fetch_station_payload_retries_and_preserves_headers(monkeypatch) -> None:
    session = FakeSession(
        responses=[FakeResponse(SAMPLE_INMET_PAYLOAD)],
        side_effects=[fetch_observed_inmet.requests.Timeout("timeout")],
    )
    sleep_calls: list[float] = []
    monkeypatch.setattr(fetch_observed_inmet.time, "sleep", lambda seconds: sleep_calls.append(seconds))

    payload = fetch_observed_inmet.fetch_station_payload(
        "A801",
        request_date=datetime(2026, 3, 11).date(),
        base_url="https://example.test/v1",
        timeout_seconds=7,
        api_key="secret",
        session=session,
        retry_attempts=2,
        retry_sleep_seconds=0.1,
    )

    assert payload == SAMPLE_INMET_PAYLOAD
    assert sleep_calls == [0.1]
    assert session.requests[0]["headers"]["x-api-key"] == "secret"
    assert session.requests[0]["params"] == {"dataInicio": "2026-03-11", "dataFinal": "2026-03-11"}


def test_fetch_observed_inmet_writes_one_station_csv_without_sqlite_writes(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    session = FakeSession(
        responses=[
            FakeResponse({"data": {"data": [{"timestamp": "2026-03-10T00:00:00", "value": 1.0, "codigo": "A801"}]}}),
            FakeResponse({"data": {"data": [{"timestamp": "2026-03-11T00:00:00", "value": 2.0, "codigo": "A801"}]}}),
        ]
    )
    monkeypatch.setattr(fetch_observed_inmet.requests, "Session", lambda: session)

    summary = fetch_observed_inmet.fetch_observed_inmet(
        [{"station_id": "inmet:A801", "station_code": "A801"}],
        request_dates_by_station={"inmet:A801": [date(2026, 3, 10), date(2026, 3, 11)]},
        interim_dir=tmp_path / "interim",
        run_id="run",
        base_url="https://example.test/v1",
        timeout_seconds=5,
        api_key="secret",
    )

    with summary.csv_paths[0].open("r", encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    with sqlite3.connect(db_path) as connection:
        series_total = connection.execute("SELECT COUNT(*) FROM observed_series").fetchone()[0]

    assert [request["params"] for request in session.requests] == [
        {"dataInicio": "2026-03-10", "dataFinal": "2026-03-10"},
        {"dataInicio": "2026-03-11", "dataFinal": "2026-03-11"},
    ]
    assert summary.csv_paths == [tmp_path / "interim" / "inmet" / "run" / "A801" / "observed.csv"]
    assert [row["observed_at"] for row in rows] == ["2026-03-10 00:00", "2026-03-11 00:00"]
    assert series_total == 0



def test_fetch_and_load_observed_inmet_requires_explicit_api_key(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with pytest.raises(ValueError, match="api_key"):
        observed_workflow.fetch_and_load_observed_provider(
            "inmet",
            database_path=db_path,
            window_start=datetime(2026, 3, 11, 0, 0, 0),
            window_end=datetime(2026, 3, 11, 13, 45, 0),
            timeout_seconds=5,
            api_key="",
            station_codes=["A801"],
            interim_dir=tmp_path / "interim",
            logs_dir=tmp_path / "logs",
            base_url="https://example.test/v1",
        )

def test_fetch_and_load_observed_inmet_persists_values_and_logs(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    session = FakeSession(responses=[FakeResponse(SAMPLE_INMET_PAYLOAD)])
    monkeypatch.setattr(fetch_observed_inmet.requests, "Session", lambda: session)

    summary = observed_workflow.fetch_and_load_observed_provider(
        "inmet",
        database_path=db_path,
        window_start=datetime(2026, 3, 11, 0, 0, 0),
        window_end=datetime(2026, 3, 11, 13, 45, 0),
        timeout_seconds=5,
        api_key="secret",
        station_codes=["a801"],
        interim_dir=tmp_path / "interim",
        logs_dir=tmp_path / "logs",
        base_url="https://example.test/v1",
    )

    with sqlite3.connect(db_path) as connection:
        series_rows = connection.execute(
            "SELECT series_id, variable_code FROM observed_series ORDER BY variable_code"
        ).fetchall()
        rain_values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'inmet:A801.rain.raw' ORDER BY observed_at"
        ).fetchall()

    normalized_csv_files = list((tmp_path / "interim" / "inmet" / "20260311T134500" / "A801").glob("*.csv"))
    log_file = tmp_path / "logs" / "fetch_observed_inmet" / "20260311T134500.log"
    log_text = log_file.read_text(encoding="utf-8")

    assert summary.fetch_summary.legacy_counts() == {
        "run_id": "20260311T134500",
        "stations_total": 1,
        "stations_ok": 1,
        "stations_no_data": 0,
        "stations_error": 0,
    }
    assert series_rows == [("inmet:A801.rain.raw", "rain")]
    assert rain_values == [("2026-03-10 21:00", 2.0), ("2026-03-10 23:00", 3.5)]
    assert len(normalized_csv_files) == 1
    assert normalized_csv_files[0].name == "observed.csv"
    assert log_file.exists()
    assert "window_start=2026-03-11 window_end=2026-03-11" in log_text
    assert "normalized_csv=" in log_text


def test_fetch_and_load_observed_inmet_counts_no_data_when_payload_is_empty(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    monkeypatch.setattr(
        fetch_observed_inmet.requests,
        "Session",
        lambda: FakeSession(responses=[FakeResponse({"data": {"data": []}})]),
    )

    summary = observed_workflow.fetch_and_load_observed_provider(
        "inmet",
        database_path=db_path,
        window_start=datetime(2026, 3, 11, 0, 0, 0),
        window_end=datetime(2026, 3, 11, 13, 45, 0),
        timeout_seconds=5,
        api_key="secret",
        station_codes=["A801"],
        interim_dir=tmp_path / "interim",
        logs_dir=tmp_path / "logs",
        base_url="https://example.test/v1",
    )

    assert summary.fetch_summary.legacy_counts()["stations_no_data"] == 1
    assert summary.fetch_summary.legacy_counts()["stations_ok"] == 0


def test_fetch_and_load_observed_inmet_marks_station_error_after_final_failure(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    monkeypatch.setattr(fetch_observed_inmet.time, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        fetch_observed_inmet.requests,
        "Session",
        lambda: FakeSession(side_effects=[fetch_observed_inmet.requests.Timeout("timeout")] * 5),
    )

    summary = observed_workflow.fetch_and_load_observed_provider(
        "inmet",
        database_path=db_path,
        window_start=datetime(2026, 3, 11, 0, 0, 0),
        window_end=datetime(2026, 3, 11, 13, 45, 0),
        timeout_seconds=5,
        api_key="secret",
        station_codes=["A801"],
        interim_dir=tmp_path / "interim",
        logs_dir=tmp_path / "logs",
        base_url="https://example.test/v1",
    )

    assert summary.fetch_summary.legacy_counts()["stations_error"] == 1
    assert summary.fetch_summary.legacy_counts()["stations_ok"] == 0


def test_fetch_and_load_observed_inmet_rejects_unknown_station_filter(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with pytest.raises(ValueError, match="No INMET station found"):
        observed_workflow.fetch_and_load_observed_provider(
            "inmet",
            database_path=db_path,
            window_start=datetime(2026, 3, 11, 0, 0, 0),
            window_end=datetime(2026, 3, 11, 13, 45, 0),
            timeout_seconds=5,
            api_key="secret",
            station_codes=["A999"],
            interim_dir=tmp_path / "interim",
            logs_dir=tmp_path / "logs",
            base_url="https://example.test/v1",
        )
