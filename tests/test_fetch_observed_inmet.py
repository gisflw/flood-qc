from __future__ import annotations

import json
import sqlite3
from datetime import datetime
from pathlib import Path

import pandas as pd
import pytest

from ingest import fetch_observed_inmet
from storage.db_bootstrap import initialize_history_db


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


def test_require_api_key_raises_clear_error_when_missing(monkeypatch) -> None:
    monkeypatch.delenv(fetch_observed_inmet.INMET_API_KEY_ENV, raising=False)
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", Path("/nonexistent/.env"))

    with pytest.raises(RuntimeError, match=fetch_observed_inmet.INMET_API_KEY_ENV):
        fetch_observed_inmet.require_api_key()


def test_require_api_key_reads_local_env_file(tmp_path, monkeypatch) -> None:
    env_path = tmp_path / ".env"
    env_path.write_text('INMET_API_KEY="secret-from-file"\n', encoding="utf-8")
    monkeypatch.delenv(fetch_observed_inmet.INMET_API_KEY_ENV, raising=False)
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", env_path)

    assert fetch_observed_inmet.require_api_key() == "secret-from-file"


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


def test_ingest_observed_inmet_persists_values_and_logs(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    session = FakeSession(responses=[FakeResponse(SAMPLE_INMET_PAYLOAD)])
    monkeypatch.setenv(fetch_observed_inmet.INMET_API_KEY_ENV, "secret")
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", tmp_path / ".env")
    monkeypatch.setattr(fetch_observed_inmet.requests, "Session", lambda: session)

    summary = fetch_observed_inmet.ingest_observed_inmet(
        db_path,
        reference_time=datetime(2026, 3, 11, 13, 45, 0),
        request_days=1,
        timeout_seconds=5,
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

    monkeypatch.setenv(fetch_observed_inmet.INMET_API_KEY_ENV, "secret")
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", tmp_path / ".env")
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

    monkeypatch.setenv(fetch_observed_inmet.INMET_API_KEY_ENV, "secret")
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", tmp_path / ".env")
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
    monkeypatch.setenv(fetch_observed_inmet.INMET_API_KEY_ENV, "secret")
    monkeypatch.setattr(fetch_observed_inmet, "LOCAL_ENV_PATH", tmp_path / ".env")

    with pytest.raises(ValueError, match="No INMET station found"):
        fetch_observed_inmet.ingest_observed_inmet(
            db_path,
            reference_time=datetime(2026, 3, 11, 13, 45, 0),
            request_days=1,
            timeout_seconds=5,
            station_codes=["A999"],
            interim_dir=tmp_path / "interim",
            logs_dir=tmp_path / "logs",
            base_url="https://example.test/v1",
        )


def test_fetch_observed_inmet_main_uses_config_only(tmp_path, monkeypatch) -> None:
    captured: dict[str, object] = {}

    monkeypatch.setattr(
        fetch_observed_inmet,
        "load_settings",
        lambda: {
            "run": {"reference_time": "2026-03-11T00:00:00"},
            "ingest": {"request_days": 3, "timeout_seconds": 20},
            "summaries": {
                "forecast_days": [1, 3],
                "accum_hours": [24, 72],
                "selected_mini_ids": ["7601"],
            },
            "mgb": {
                "input_days_before": 56,
                "output_days_before": 28,
                "forecast_horizon_days": 14,
                "use_forecast_data": True,
            },
            "rainfall_interpolation": {"nearest_stations": 5, "power": 2.0},
        },
    )
    monkeypatch.setattr(fetch_observed_inmet, "history_db_path", lambda: tmp_path / "history.sqlite")
    monkeypatch.setattr(fetch_observed_inmet, "default_interim_dir", lambda: tmp_path / "interim")
    monkeypatch.setattr(fetch_observed_inmet, "default_logs_dir", lambda: tmp_path / "logs")

    def fake_ingest(database_path, **kwargs):
        captured["database_path"] = database_path
        captured.update(kwargs)
        return {"run_id": "20260311T000000"}

    monkeypatch.setattr(fetch_observed_inmet, "ingest_observed_inmet", fake_ingest)

    result = fetch_observed_inmet.main()

    assert result == 0
    assert captured["database_path"] == tmp_path / "history.sqlite"
    assert captured["reference_time"] == datetime(2026, 3, 11, 0, 0, 0)
    assert captured["request_days"] == 3
    assert captured["timeout_seconds"] == 20.0
    assert captured["station_codes"] is None
    assert captured["interim_dir"] == tmp_path / "interim"
    assert captured["logs_dir"] == tmp_path / "logs"
