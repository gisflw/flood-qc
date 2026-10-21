from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from mgb_ops.assets.current_run import CurrentRunArtifact, ForecastScenarioReference, create_current_artifact, load_current_artifact
from mgb_ops.assets.databases import initialize_history_db, initialize_run_db, load_history_station_inventory

REPO_ROOT = Path(__file__).resolve().parents[2]
SQL_DIR = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql"
TEST_INVENTORY_CSV = REPO_ROOT / "tests" / "fixtures" / "history_station_inventory.csv"


def _tables(path: Path) -> set[str]:
    with sqlite3.connect(path) as connection:
        return {str(row[0]) for row in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")}


def test_initialize_history_db_contains_only_history_entities(tmp_path: Path) -> None:
    path = initialize_history_db(tmp_path / "history.sqlite", TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")
    assert _tables(path) == {
        "provider", "variable", "station", "station_observed_variable", "station_level_reference",
        "historical_flood_level", "asset", "observed_series", "observed_value",
    }
    with sqlite3.connect(path) as connection:
        assert {row[0] for row in connection.execute("SELECT provider_code FROM provider")} == {"ana", "inmet", "ecmwf", "noaa"}
        assert {row[0] for row in connection.execute("SELECT variable_code FROM variable")} == {"rain", "level", "flow"}
        triggers = list(connection.execute("SELECT name FROM sqlite_master WHERE type='trigger'"))
    assert triggers == []


def test_history_bootstrap_discards_legacy_operational_entities(tmp_path: Path) -> None:
    path = initialize_history_db(tmp_path / "history.sqlite", TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(path) as connection:
        connection.execute("CREATE TABLE qc_flag (id INTEGER)")
        connection.execute("CREATE TABLE manual_edit (id INTEGER)")
        connection.execute("CREATE TABLE run_catalog (id INTEGER)")
        connection.execute("CREATE TRIGGER trg_manual_edit_no_overlap_insert AFTER INSERT ON manual_edit BEGIN SELECT 1; END")
    initialize_history_db(path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")
    assert not {"qc_flag", "manual_edit", "run_catalog"}.intersection(_tables(path))
    with sqlite3.connect(path) as connection:
        assert list(connection.execute("SELECT name FROM sqlite_master WHERE type='trigger'")) == []


def test_initialize_run_db_replaces_legacy_database_with_empty_artifact_schema(tmp_path: Path) -> None:
    path = tmp_path / "legacy.sqlite"
    with sqlite3.connect(path) as connection:
        connection.execute("CREATE TABLE run (run_id TEXT)")
        connection.execute("INSERT INTO run VALUES ('legacy')")
    initialize_run_db("ignored", path, SQL_DIR / "run_schema.sql")
    assert _tables(path) == {
        "current_run", "forecast_scenario", "forecast_correction", "observed_replacement",
        "station_exclusion", "current_execution", "published_cache_reference",
    }
    with sqlite3.connect(path) as connection:
        assert connection.execute("SELECT status FROM current_execution").fetchone() == ("never",)


def test_current_artifact_create_and_load(tmp_path: Path) -> None:
    path = tmp_path / "current_run.sqlite"
    artifact = CurrentRunArtifact(
        "2026-03-12T00:00:00", "2026-03-26T00:00:00", 1,
        {"forecast_horizon_days": 14}, {"bbox": [-53, -31, -50, -29]}, {"nearest_stations": 5},
        {"start": "2026-03-01T00:00:00", "end": "2026-03-12T00:00:00"}, ("ana", "inmet"),
        scenarios=(ForecastScenarioReference("zero", 0, "zero", "Zero-rain horizon"),),
    )
    create_current_artifact(path, artifact)
    loaded = load_current_artifact(path)
    assert loaded.reference_time == artifact.reference_time
    with sqlite3.connect(path) as connection:
        columns = {row[1] for row in connection.execute("PRAGMA table_info(current_run)")}
    assert "window_start" not in columns
    assert loaded.scenarios[0].kind == "zero"


@pytest.mark.parametrize("observed_variables,error", [("", "required"), ("wind", "unsupported"), ("rain,rain", "duplicates")])
def test_history_station_inventory_rejects_invalid_observed_variables(tmp_path: Path, observed_variables: str, error: str) -> None:
    inventory = tmp_path / "inventory.csv"
    inventory.write_text(
        "provider_code,station_code,station_name,mini_id,latitude,longitude,altitude_m,observed_variables,river_name,drainage_area_km2,municipality,state,responsible_agency,operator\n"
        f'ana,1,ONE,1,-29,-51,1,"{observed_variables}",,,,,,\n',
        encoding="utf-8",
    )
    with pytest.raises(ValueError, match=error):
        initialize_history_db(tmp_path / "history.sqlite", inventory, SQL_DIR / "history_schema.sql")


def test_inventory_refresh_deletes_removed_station_and_observations(tmp_path: Path) -> None:
    path = initialize_history_db(tmp_path / "history.sqlite", TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(path) as connection:
        connection.execute("INSERT INTO observed_series VALUES ('ana:74100000.level.raw', 'ana:74100000', 'level', 'raw', CURRENT_TIMESTAMP)")
        connection.execute("INSERT INTO observed_value VALUES ('ana:74100000.level.raw', '2026-03-10 00:00', 10)")
    inventory = tmp_path / "without_station.csv"
    inventory.write_text(
        "provider_code,station_code,station_name,mini_id,latitude,longitude,altitude_m,observed_variables,river_name,drainage_area_km2,municipality,state,responsible_agency,operator\n"
        "ana,2650035,UHE ITA CACADOR PLU,,-26.8192,-50.9856,960,rain,,,,,,\n",
        encoding="utf-8",
    )
    initialize_history_db(path, inventory, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(path) as connection:
        assert connection.execute("SELECT COUNT(*) FROM observed_value").fetchone()[0] == 0
