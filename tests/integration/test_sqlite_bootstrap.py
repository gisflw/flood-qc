from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from mgb_ops.assets.databases import initialize_history_db, initialize_run_db, load_history_station_inventory

REPO_ROOT = Path(__file__).resolve().parents[2]
SQL_DIR = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql"
TEST_INVENTORY_CSV = REPO_ROOT / "tests" / "fixtures" / "history_station_inventory.csv"


def _init_history(database_path: Path) -> Path:
    return initialize_history_db(database_path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")


def _init_run(run_id: str, database_path: Path) -> Path:
    return initialize_run_db(run_id, database_path, SQL_DIR / "run_schema.sql")


def _list_tables(database_path) -> set[str]:
    with sqlite3.connect(database_path) as connection:
        rows = connection.execute(
            "SELECT name FROM sqlite_master WHERE type = 'table'"
        ).fetchall()
    return {row[0] for row in rows}


def _list_columns(database_path, table_name: str) -> set[str]:
    with sqlite3.connect(database_path) as connection:
        rows = connection.execute(f"PRAGMA table_info({table_name})").fetchall()
    return {row[1] for row in rows}


def _list_triggers(database_path) -> set[str]:
    with sqlite3.connect(database_path) as connection:
        rows = connection.execute("SELECT name FROM sqlite_master WHERE type = 'trigger'").fetchall()
    return {row[0] for row in rows}


def test_initialize_history_db(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    _init_history(db_path)
    tables = _list_tables(db_path)
    assert {
        "provider",
        "variable",
        "station",
        "station_observed_variable",
        "asset",
        "observed_series",
        "observed_value",
        "manual_edit",
        "run_catalog",
    }.issubset(tables)
    assert "ingest_batch" not in tables

    with sqlite3.connect(db_path) as connection:
        providers = {
            row[0] for row in connection.execute("SELECT provider_code FROM provider").fetchall()
        }
        variables = {
            row[0] for row in connection.execute("SELECT variable_code FROM variable").fetchall()
        }
        station_variable_rows = connection.execute(
            "SELECT station_id, variable_code FROM station_observed_variable ORDER BY station_id, variable_code"
        ).fetchall()

    station_columns = _list_columns(db_path, "station")
    assert {
        "station_id",
        "station_code",
        "station_name",
        "provider_code",
        "mini_id",
        "latitude",
        "longitude",
        "altitude_m",
        "created_at",
    }.issubset(station_columns)
    with sqlite3.connect(db_path) as connection:
        altitude_type = connection.execute(
            "SELECT type FROM pragma_table_info('station') WHERE name = 'altitude_m'"
        ).fetchone()[0]
        mini_id_type = connection.execute(
            "SELECT type FROM pragma_table_info('station') WHERE name = 'mini_id'"
        ).fetchone()[0]
    assert altitude_type == "INTEGER"
    assert mini_id_type == "INTEGER"

    assert {"ana", "inmet", "ecmwf", "noaa"}.issubset(providers)
    assert {"rain", "level", "flow"}.issubset(variables)
    assert ("ana:74100000", "flow") in station_variable_rows
    assert ("ana:74100000", "level") in station_variable_rows
    assert ("ana:74100000", "rain") in station_variable_rows
    assert ("ana:2650035", "rain") in station_variable_rows
    assert not any(row[0] == "ana:74320000" for row in station_variable_rows)

    observed_series_columns = _list_columns(db_path, "observed_series")
    assert {"series_id", "station_id", "variable_code", "state", "created_at"}.issubset(observed_series_columns)
    assert "provider_code" not in observed_series_columns
    assert "unit" not in observed_series_columns
    assert "source_asset_id" not in observed_series_columns
    assert "ingest_batch_id" not in observed_series_columns

    manual_edit_columns = _list_columns(db_path, "manual_edit")
    assert {
        "manual_edit_id",
        "asset_id",
        "t0_step",
        "t1_step",
        "shift_lat",
        "shift_lon",
        "rotation_deg",
        "multiplication_factor",
        "editor",
        "reason",
        "metadata_json",
        "created_at",
    }.issubset(manual_edit_columns)
    assert "edit_kind" not in manual_edit_columns
    triggers = _list_triggers(db_path)
    assert {"trg_manual_edit_no_overlap_insert", "trg_manual_edit_no_overlap_update"}.issubset(triggers)


def test_manual_edit_overlap_trigger_blocks_conflicts(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    _init_history(db_path)

    with sqlite3.connect(db_path) as connection:
        connection.execute(
            """
            INSERT INTO asset (
                asset_id, asset_kind, format, relative_path, provider_code
            ) VALUES (?, ?, ?, ?, ?)
            """,
            (
                "ecmwf.ifs.fc.20260311T000000Z.precipitation_grid",
                "forecast_precipitation_grid",
                "NetCDF",
                "data/downloads/ecmwf/fc_2026-03-11_00_IFS_precipitation_grid.nc",
                "ecmwf",
            ),
        )
        connection.execute(
            """
            INSERT INTO manual_edit (
                asset_id, t0_step, t1_step, shift_lat, shift_lon, rotation_deg, multiplication_factor, reason, metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            ("ecmwf.ifs.fc.20260311T000000Z.precipitation_grid", 0, 24, 0.0, 0.0, 0.0, 1.0, "primeira", "{}"),
        )
        connection.execute(
            """
            INSERT INTO manual_edit (
                asset_id, t0_step, t1_step, shift_lat, shift_lon, rotation_deg, multiplication_factor, reason, metadata_json
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            ("ecmwf.ifs.fc.20260311T000000Z.precipitation_grid", 24, 48, 0.0, 0.0, 0.0, 1.0, "encostada", "{}"),
        )
        with pytest.raises(sqlite3.IntegrityError, match="manual_edit overlap"):
            connection.execute(
                """
                INSERT INTO manual_edit (
                    asset_id, t0_step, t1_step, shift_lat, shift_lon, rotation_deg, multiplication_factor, reason, metadata_json
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                ("ecmwf.ifs.fc.20260311T000000Z.precipitation_grid", 12, 36, 0.0, 0.0, 0.0, 1.0, "sobreposta", "{}"),
            )


def test_history_station_inventory_csv_loads(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    _init_history(db_path)

    with sqlite3.connect(db_path) as connection:
        total = connection.execute("SELECT COUNT(*) FROM station").fetchone()[0]
        ana_total = connection.execute(
            "SELECT COUNT(*) FROM station WHERE provider_code = 'ana'"
        ).fetchone()[0]
        inmet_total = connection.execute(
            "SELECT COUNT(*) FROM station WHERE provider_code = 'inmet'"
        ).fetchone()[0]
        distinct_id = connection.execute(
            "SELECT COUNT(DISTINCT station_id) FROM station"
        ).fetchone()[0]
        distinct_station = connection.execute(
            "SELECT COUNT(DISTINCT provider_code || '|' || station_code) FROM station"
        ).fetchone()[0]
        ana_sample = connection.execute(
            "SELECT station_name, mini_id, typeof(mini_id), latitude, longitude, altitude_m, typeof(altitude_m) FROM station "
            "WHERE provider_code = 'ana' AND station_code = '2650035'"
        ).fetchone()
        fallback_sample = connection.execute(
            "SELECT station_name, mini_id, typeof(mini_id), latitude, longitude, altitude_m, typeof(altitude_m) FROM station "
            "WHERE provider_code = 'ana' AND station_code = '74320000'"
        ).fetchone()
        mapped_sample = connection.execute(
            "SELECT mini_id, typeof(mini_id) FROM station "
            "WHERE provider_code = 'ana' AND station_code = '74100000'"
        ).fetchone()
        inmet_sample = connection.execute(
            "SELECT station_name, mini_id, typeof(mini_id), latitude, longitude, altitude_m, typeof(altitude_m) FROM station "
            "WHERE provider_code = 'inmet' AND station_code = 'A840'"
        ).fetchone()
        computed_ids = dict(
            connection.execute(
                "SELECT station_code, station_id FROM station "
                "WHERE station_code IN ('71200000', '2650035', 'A801', 'B807')"
            ).fetchall()
        )
        padded_code = connection.execute(
            "SELECT COUNT(*) FROM station WHERE station_code IN ('02650035', '0A801')"
        ).fetchone()[0]

    assert total == 7
    assert ana_total == 4
    assert inmet_total == 3
    assert distinct_id == total
    assert distinct_station == total
    assert ana_sample == ("UHE ITA CACADOR PLU", None, "null", -26.8192, -50.9856, 960, "integer")
    assert fallback_sample == ("PONTE DO SARGENTO", 6693, "integer", -26.6822, -53.2861, None, "null")
    assert mapped_sample == (8504, "integer")
    assert inmet_sample == ("BENTO GONCALVES", None, "null", -29.1645, -51.5342, 623, "integer")
    assert computed_ids == {
        "71200000": "ana:71200000",
        "2650035": "ana:2650035",
        "A801": "inmet:A801",
        "B807": "inmet:B807",
    }
    assert padded_code == 0


def test_history_station_inventory_requires_mini_id_column(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    inventory_path = tmp_path / "inventory_without_mini_id.csv"
    inventory_path.write_text(
        "\n".join(
            [
                "provider_code,station_code,station_name,latitude,longitude,altitude_m,observed_variables",
                "ana,74100000,ANA TEST STATION,-29.1234,-51.1234,10,rain",
            ]
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="missing columns \\['mini_id'\\]"):
        initialize_history_db(db_path, inventory_path, SQL_DIR / "history_schema.sql")


def test_history_station_inventory_requires_observed_variables_column(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    inventory_path = tmp_path / "inventory_without_observed_variables.csv"
    inventory_path.write_text(
        "\n".join(
            [
                "provider_code,station_code,station_name,mini_id,latitude,longitude,altitude_m",
                "ana,74100000,ANA TEST STATION,8504,-29.1234,-51.1234,10",
            ]
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match="missing columns \\['observed_variables'\\]"):
        initialize_history_db(db_path, inventory_path, SQL_DIR / "history_schema.sql")


@pytest.mark.parametrize(
    ("observed_variables", "expected_error"),
    [
        ("", "observed_variables is required"),
        ("wind", "unsupported"),
        ("rain,rain", "duplicates"),
        ("rain,", "empty value"),
    ],
)
def test_history_station_inventory_rejects_invalid_observed_variables(
    tmp_path, observed_variables: str, expected_error: str
) -> None:
    db_path = tmp_path / "history.sqlite"
    initialize_history_db(db_path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")
    inventory_path = tmp_path / "inventory_invalid_observed_variables.csv"
    inventory_path.write_text(
        "\n".join(
            [
                "provider_code,station_code,station_name,mini_id,latitude,longitude,altitude_m,observed_variables",
                f'ana,74100000,ANA TEST STATION,8504,-29.1234,-51.1234,10,"{observed_variables}"',
            ]
        ),
        encoding="utf-8",
    )

    with pytest.raises(ValueError, match=expected_error):
        load_history_station_inventory(db_path, inventory_path)


def test_initialize_history_db_migrates_station_variables_without_touching_observed_values(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    _init_history(db_path)
    with sqlite3.connect(db_path) as connection:
        connection.execute(
            "INSERT INTO observed_series (series_id, station_id, variable_code, state) VALUES (?, ?, ?, ?)",
            ("ana:74100000.rain.raw", "ana:74100000", "rain", "raw"),
        )
        connection.execute(
            "INSERT INTO observed_value (series_id, observed_at, value) VALUES (?, ?, ?)",
            ("ana:74100000.rain.raw", "2026-03-10 00:00", 4.2),
        )
        before_series = connection.execute("SELECT COUNT(*) FROM observed_series").fetchone()[0]
        before_values = connection.execute("SELECT COUNT(*) FROM observed_value").fetchone()[0]
        before_sample = connection.execute(
            "SELECT series_id, observed_at, value FROM observed_value ORDER BY series_id, observed_at"
        ).fetchall()
        connection.execute("DROP TABLE station_observed_variable")
        connection.commit()

    initialize_history_db(db_path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")

    with sqlite3.connect(db_path) as connection:
        after_series = connection.execute("SELECT COUNT(*) FROM observed_series").fetchone()[0]
        after_values = connection.execute("SELECT COUNT(*) FROM observed_value").fetchone()[0]
        after_sample = connection.execute(
            "SELECT series_id, observed_at, value FROM observed_value ORDER BY series_id, observed_at"
        ).fetchall()
        station_variables = connection.execute(
            "SELECT variable_code FROM station_observed_variable WHERE station_id = 'ana:74100000' ORDER BY variable_code"
        ).fetchall()

    assert after_series == before_series
    assert after_values == before_values
    assert after_sample == before_sample
    assert station_variables == [("flow",), ("level",), ("rain",)]


def test_initialize_history_db_rejects_partial_existing_database_without_repair(tmp_path) -> None:
    db_path = tmp_path / "history.sqlite"
    with sqlite3.connect(db_path) as connection:
        connection.execute(
            """
            CREATE TABLE station (
                station_id TEXT PRIMARY KEY,
                station_code TEXT NOT NULL,
                station_name TEXT NOT NULL,
                provider_code TEXT NOT NULL,
                latitude REAL,
                longitude REAL,
                altitude_m INTEGER,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (provider_code, station_code)
            )
            """
        )

    original_bytes = db_path.read_bytes()
    with pytest.raises(RuntimeError, match="incompatible"):
        initialize_history_db(db_path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")

    with sqlite3.connect(db_path) as connection:
        assert connection.execute(
            "SELECT COUNT(*) FROM pragma_table_info('station') WHERE name = 'mini_id'"
        ).fetchone()[0] == 0
    assert db_path.read_bytes() == original_bytes


def test_initialize_run_db(tmp_path) -> None:
    db_path = tmp_path / "20260310T120000.sqlite"
    _init_run("20260310T120000", db_path)
    tables = _list_tables(db_path)
    assert {
        "run",
        "run_input_series",
        "run_input_value",
        "run_asset",
        "derived_series",
        "derived_value",
        "model_execution",
        "mgb_output_series",
        "mgb_output_value",
        "report_artifact",
    }.issubset(tables)

    with sqlite3.connect(db_path) as connection:
        row = connection.execute("SELECT run_id FROM run").fetchone()
    assert row[0] == "20260310T120000"
