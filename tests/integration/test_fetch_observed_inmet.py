from __future__ import annotations

import json
import sqlite3
from datetime import datetime
from pathlib import Path

import pandas as pd
import pytest

from mgb_ops.ingest import fetch_observed_inmet
from mgb_ops.storage.db_bootstrap import initialize_history_db


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



def test_ingest_observed_inmet_requires_explicit_api_key(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with pytest.raises(ValueError, match="api_key"):
        fetch_observed_inmet.ingest_observed_inmet(
            db_path,
            reference_time=datetime(2026, 3, 11, 13, 45, 0),
            request_days=1,
            timeout_seconds=5,
            api_key="",
            station_codes=["A801"],
            interim_dir=tmp_path / "interim",
            logs_dir=tmp_path / "logs",
            base_url="https://example.test/v1",
        )

def test_ingest_observed_inmet_persists_values_and_logs(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    session = FakeSession(responses=[FakeResponse(SAMPLE_INMET_PAYLOAD)])
    monkeypatch.setattr(fetch_observed_inmet.requests, "Session", lambda: session)

    summary = fetch_observed_inmet.ingest_observed_inmet(
        db_path,
        reference_time=datetime(2026, 3, 11, 13, 45, 0),
        request_days=1,
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
            "WHERE series_id = '2000001801.rain.raw' ORDER BY observed_at"
        ).fetchall()

    raw_json_files = list((tmp_path / "interim" / "inmet" / "A801").glob("*.json"))
    log_file = tmp_path / "logs" / "fetch_observed_inmet" / "20260311T134500.log"
    log_text = log_file.read_text(encoding="utf-8")

    assert summary == {
        "run_id": "20260311T134500",
        "stations_total": 1,
        "stations_ok": 1,
        "stations_no_data": 0,
        "stations_error": 0,
    }
    assert series_rows == [("2000001801.rain.raw", "rain")]
    assert rain_values == [("2026-03-10 21:00", 2.0), ("2026-03-10 23:00", 3.5)]
    assert len(raw_json_files) == 1
    assert json.loads(raw_json_files[0].read_text(encoding="utf-8")) == SAMPLE_INMET_PAYLOAD
    assert log_file.exists()
    assert "window_start=2026-03-11 window_end=2026-03-11" in log_text
    assert "raw_json=" in log_text


def test_ingest_observed_inmet_counts_no_data_when_payload_is_empty(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    monkeypatch.setattr(
        fetch_observed_inmet.requests,
        "Session",
        lambda: FakeSession(responses=[FakeResponse({"data": {"data": []}})]),
    )

    summary = fetch_observed_inmet.ingest_observed_inmet(
        db_path,
        reference_time=datetime(2026, 3, 11, 13, 45, 0),
        request_days=1,
        timeout_seconds=5,
        api_key="secret",
        station_codes=["A801"],
        interim_dir=tmp_path / "interim",
        logs_dir=tmp_path / "logs",
        base_url="https://example.test/v1",
    )

    assert summary["stations_no_data"] == 1
    assert summary["stations_ok"] == 0


def test_ingest_observed_inmet_marks_station_error_after_final_failure(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    monkeypatch.setattr(fetch_observed_inmet.time, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        fetch_observed_inmet.requests,
        "Session",
        lambda: FakeSession(side_effects=[fetch_observed_inmet.requests.Timeout("timeout")] * 5),
    )

    summary = fetch_observed_inmet.ingest_observed_inmet(
        db_path,
        reference_time=datetime(2026, 3, 11, 13, 45, 0),
        request_days=1,
        timeout_seconds=5,
        api_key="secret",
        station_codes=["A801"],
        interim_dir=tmp_path / "interim",
        logs_dir=tmp_path / "logs",
        base_url="https://example.test/v1",
    )

    assert summary["stations_error"] == 1
    assert summary["stations_ok"] == 0


def test_ingest_observed_inmet_rejects_unknown_station_filter(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with pytest.raises(ValueError, match="No INMET station found"):
        fetch_observed_inmet.ingest_observed_inmet(
            db_path,
            reference_time=datetime(2026, 3, 11, 13, 45, 0),
            request_days=1,
            timeout_seconds=5,
            api_key="secret",
            station_codes=["A999"],
            interim_dir=tmp_path / "interim",
            logs_dir=tmp_path / "logs",
            base_url="https://example.test/v1",
        )

