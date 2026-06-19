from __future__ import annotations

import shutil
import sqlite3
import csv
from datetime import date, datetime, timedelta

from mgb_ops.common import time_utils
from mgb_ops.adapters import observed_ana
from mgb_ops.workflows import observed as observed_workflow
from db_helpers import initialize_history_db
from mgb_ops.storage.history_repository import HistoryRepository, build_observed_series_id


SAMPLE_ANA_XML = """\
<root>
  <DadosHidrometereologicos>
    <CodEstacao>74100000</CodEstacao>
    <DataHora>2026-03-10 00:00:00</DataHora>
    <Nivel>100</Nivel>
    <Chuva>1.0</Chuva>
    <Vazao>10.0</Vazao>
  </DadosHidrometereologicos>
  <DadosHidrometereologicos>
    <CodEstacao>74100000</CodEstacao>
    <DataHora>2026-03-10 00:00:00</DataHora>
    <Nivel>101</Nivel>
    <Chuva>2.0</Chuva>
    <Vazao>11.0</Vazao>
  </DadosHidrometereologicos>
  <DadosHidrometereologicos>
    <CodEstacao>74100000</CodEstacao>
    <DataHora>2026-03-10 02:00:00</DataHora>
    <Nivel>105</Nivel>
    <Chuva></Chuva>
    <Vazao>12.0</Vazao>
  </DadosHidrometereologicos>
</root>
"""


class FakeResponse:
    def __init__(self, text: str) -> None:
        self.text = text

    def raise_for_status(self) -> None:
        return None


class FakeDateTime(datetime):
    @classmethod
    def now(cls, tz=None):
        current = cls(2026, 3, 19, 11, 35, 42)
        if tz is not None:
            return current.replace(tzinfo=tz)
        return current


def test_history_repository_observed_series_and_values(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        station = repository.get_provider_stations("ana")[0]
        series_id = repository.ensure_observed_series(station["station_id"], "rain")
        repeated_series_id = repository.ensure_observed_series(station["station_id"], "rain")
        written = repository.upsert_observed_values(
            series_id,
            [("2026-03-10 00:00", 1.0), ("2026-03-10 01:00", 2.0)],
        )
        updated = repository.upsert_observed_values(
            series_id,
            [("2026-03-10 00:00", 3.5)],
        )

    with sqlite3.connect(db_path) as connection:
        series_total = connection.execute("SELECT COUNT(*) FROM observed_series").fetchone()[0]
        values = connection.execute(
            "SELECT observed_at, value FROM observed_value WHERE series_id = ? ORDER BY observed_at",
            (series_id,),
        ).fetchall()

    assert series_id == repeated_series_id
    assert series_id == build_observed_series_id(station["station_id"], "rain")
    assert written == 2
    assert updated == 1
    assert series_total == 1
    assert values == [("2026-03-10 00:00", 3.5), ("2026-03-10 01:00", 2.0)]


def test_fetch_observed_ana_resolve_reference_time_accepts_yesterday(monkeypatch) -> None:
    monkeypatch.setattr(time_utils, "datetime", FakeDateTime)

    reference_time = observed_ana.resolve_reference_time("yesterday")

    assert reference_time == datetime(2026, 3, 18, 23, 0, 0)


def test_fetch_observed_ana_resolve_reference_time_date_only_assumes_last_hour() -> None:
    reference_time = observed_ana.resolve_reference_time("2026-03-18")

    assert reference_time == datetime(2026, 3, 18, 23, 0, 0)


def test_history_repository_rebuild_assumption_uses_canonical_series_id(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        station_id = repository.get_provider_stations("ana")[0]["station_id"]
        series_id = repository.ensure_observed_series(station_id, "rain")

    assert series_id == f"{station_id}.rain.raw"


def test_fetch_observed_ana_writes_one_station_csv_without_sqlite_writes(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)
    requested_params: list[dict[str, str]] = []

    def fake_get(url, params=None, timeout=None):
        requested_params.append(params)
        day = params["dataInicio"][:2]
        return FakeResponse(
            f"""\
<root>
  <DadosHidrometereologicos>
    <CodEstacao>74100000</CodEstacao>
    <DataHora>2026-03-{day} 00:00:00</DataHora>
    <Chuva>{int(day)}</Chuva>
  </DadosHidrometereologicos>
</root>
"""
        )

    monkeypatch.setattr("mgb_ops.adapters.observed_ana.requests.get", fake_get)

    summary = observed_ana.fetch_observed_ana(
        [{"station_id": "ana:74100000", "station_code": "74100000"}],
        request_dates_by_station={"ana:74100000": [date(2026, 3, 10), date(2026, 3, 11)]},
        downloads_dir=tmp_path / "downloads",
        run_id="run",
        base_url="http://example.test/ana",
        timeout_seconds=5,
    )

    with summary.csv_paths[0].open("r", encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    with sqlite3.connect(db_path) as connection:
        series_total = connection.execute("SELECT COUNT(*) FROM observed_series").fetchone()[0]

    assert requested_params == [
        {"codEstacao": "74100000", "dataInicio": "10/03/2026", "dataFim": "10/03/2026"},
        {"codEstacao": "74100000", "dataInicio": "11/03/2026", "dataFim": "11/03/2026"},
    ]
    assert summary.csv_paths == [tmp_path / "downloads" / "ana" / "run" / "74100000" / "observed.csv"]
    assert [row["observed_at"] for row in rows] == ["2026-03-10 00:00", "2026-03-11 00:00"]
    assert series_total == 0


def test_history_repository_get_latest_observed_at_per_station(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        station_id = repository.get_provider_stations("ana")[0]["station_id"]
        rain_series_id = repository.ensure_observed_series(station_id, "rain")
        level_series_id = repository.ensure_observed_series(station_id, "level")
        repository.upsert_observed_values(rain_series_id, [("2026-03-10 01:00", 1.0)])
        repository.upsert_observed_values(level_series_id, [("2026-03-10 03:00", 100.0)])

        latest_any = repository.get_latest_observed_at(station_id)
        latest_rain = repository.get_latest_observed_at(station_id, variable_codes=["rain"])
        latest_missing = repository.get_latest_observed_at("ana:missing")

    assert latest_any == datetime(2026, 3, 10, 3, 0)
    assert latest_rain == datetime(2026, 3, 10, 1, 0)
    assert latest_missing is None


def test_fetch_and_load_observed_ana_persists_values_and_logs(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path)

    requested_params: list[dict[str, str]] = []

    def fake_get(url, params=None, timeout=None):
        requested_params.append(params)
        return FakeResponse(SAMPLE_ANA_XML)

    monkeypatch.setattr("mgb_ops.adapters.observed_ana.requests.get", fake_get)

    stale_root = tmp_path / "downloads" / "ana"
    stale_station_dir = stale_root / "99999999"
    stale_station_dir.mkdir(parents=True, exist_ok=True)
    (stale_station_dir / "old.xml").write_text("obsolete", encoding="utf-8")

    summary = observed_workflow.fetch_and_load_observed_provider(
        "ana",
        database_path=db_path,
        base_url="http://example.test/ana",
        window_start=datetime(2026, 3, 11, 0, 0, 0),
        window_end=datetime(2026, 3, 11, 13, 45, 0),
        timeout_seconds=5,
        station_codes=["74100000"],
        downloads_dir=tmp_path / "downloads",
        logs_dir=tmp_path / "logs",
    )

    with sqlite3.connect(db_path) as connection:
        series_rows = connection.execute(
            "SELECT series_id, variable_code FROM observed_series ORDER BY variable_code"
        ).fetchall()
        rain_values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.rain.raw' ORDER BY observed_at"
        ).fetchall()
        level_values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.level.raw' ORDER BY observed_at"
        ).fetchall()
        flow_values = connection.execute(
            "SELECT observed_at, value FROM observed_value "
            "WHERE series_id = 'ana:74100000.flow.raw' ORDER BY observed_at"
        ).fetchall()

    raw_xml_files = list((tmp_path / "downloads" / "ana" / "20260311T134500" / "74100000").glob("*.xml"))
    normalized_csv_files = list((tmp_path / "downloads" / "ana" / "20260311T134500" / "74100000").glob("*.csv"))
    log_file = tmp_path / "logs" / "observed_ana" / "20260311T134500.log"
    log_text = log_file.read_text(encoding="utf-8")

    assert summary.fetch_summary.legacy_counts() == {
        "run_id": "20260311T134500",
        "stations_total": 1,
        "stations_ok": 1,
        "stations_no_data": 0,
        "stations_error": 0,
    }
    assert requested_params == [{"codEstacao": "74100000", "dataInicio": "11/03/2026", "dataFim": "11/03/2026"}]
    assert series_rows == [
        ("ana:74100000.flow.raw", "flow"),
        ("ana:74100000.level.raw", "level"),
        ("ana:74100000.rain.raw", "rain"),
    ]
    assert rain_values == [("2026-03-10 00:00", 2.0)]
    assert level_values == [("2026-03-10 00:00", 101.0), ("2026-03-10 02:00", 105.0)]
    assert flow_values == [("2026-03-10 00:00", 11.0), ("2026-03-10 02:00", 12.0)]
    assert len(raw_xml_files) == 1
    assert len(normalized_csv_files) == 1
    assert raw_xml_files[0].name == "20260311__20260311.xml"
    assert normalized_csv_files[0].name == "observed.csv"
    assert raw_xml_files[0].read_text(encoding="utf-8") == SAMPLE_ANA_XML
    assert stale_station_dir.exists()
    assert log_file.exists()
    assert "window_start=2026-03-11 window_end=2026-03-11" in log_text
    assert "raw_xml=" in log_text
    assert "normalized_csv=" in log_text
    assert not (tmp_path / "downloads" / "ana" / "raw").exists()
    assert not (tmp_path / "reports").exists()



def test_history_repository_rejects_old_observed_schema(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    with sqlite3.connect(db_path) as connection:
        connection.executescript(
            """
            CREATE TABLE observed_series (
                series_id TEXT PRIMARY KEY,
                station_id TEXT NOT NULL,
                provider_code TEXT NOT NULL,
                variable_code TEXT NOT NULL,
                unit TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'raw',
                source_asset_id TEXT,
                ingest_batch_id TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE variable (
                variable_code TEXT PRIMARY KEY,
                variable_name TEXT NOT NULL,
                default_unit TEXT NOT NULL,
                description TEXT
            );
            INSERT INTO variable (variable_code, variable_name, default_unit, description) VALUES
                ('rain', 'Observed precipitation', 'mm', ''),
                ('level', 'Observed level', 'cm', '');
            """
        )

    try:
        HistoryRepository(db_path)
    except RuntimeError as exc:
        assert "History database is incompatible" in str(exc)
        assert "mgb_ops.storage.db_bootstrap" in str(exc)
    else:
        raise AssertionError("Expected an error for the old observed_series schema.")
