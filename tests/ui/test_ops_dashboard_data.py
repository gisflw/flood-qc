from __future__ import annotations

import sqlite3
from datetime import datetime
from pathlib import Path

import numpy as np
import pandas as pd
import pytest
import xarray as xr

from mgb_ops.analysis import timeseries as ops_dashboard_data
from mgb_ops.storage.db_bootstrap import apply_schema
from mgb_ops.common.time_utils import DashboardWindow
from mgb_ops.analysis.timeseries import StaleModelOutputsError


REPO_ROOT = Path(__file__).resolve().parents[2]
HISTORY_SCHEMA_PATH = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql" / "history_schema.sql"


def initialize_history_db(path: Path) -> Path:
    apply_schema(path, HISTORY_SCHEMA_PATH)
    return path


def insert_station(connection: sqlite3.Connection, *, station_id: int, station_code: str, station_name: str) -> None:
    connection.execute(
        """
        INSERT INTO station (
            station_id,
            station_code,
            station_name,
            provider_code,
            latitude,
            longitude,
            altitude_m
        ) VALUES (?, ?, ?, 'ana', -29.5, -53.5, 10)
        """,
        (station_id, station_code, station_name),
    )


def insert_observed_series(
    connection: sqlite3.Connection,
    *,
    series_id: str,
    station_id: int,
    variable_code: str,
    state: str,
    created_at: str = "2026-03-17 12:00:00",
) -> None:
    connection.execute(
        """
        INSERT INTO observed_series (series_id, station_id, variable_code, state, created_at)
        VALUES (?, ?, ?, ?, ?)
        """,
        (series_id, station_id, variable_code, state, created_at),
    )


def insert_observed_value(connection: sqlite3.Connection, *, series_id: str, observed_at: str, value: float | None) -> None:
    connection.execute(
        "INSERT INTO observed_value (series_id, observed_at, value) VALUES (?, ?, ?)",
        (series_id, observed_at, value),
    )


def test_select_preferred_series_rows_uses_state_precedence() -> None:
    series = pd.DataFrame(
        [
            {"series_id": "rain.raw", "station_id": 1, "variable_code": "rain", "state": "raw", "created_at": "2026-01-01 00:00:00"},
            {"series_id": "rain.curated", "station_id": 1, "variable_code": "rain", "state": "curated", "created_at": "2026-01-02 00:00:00"},
            {"series_id": "rain.approved", "station_id": 1, "variable_code": "rain", "state": "approved", "created_at": "2026-01-03 00:00:00"},
            {"series_id": "level.raw", "station_id": 1, "variable_code": "level", "state": "raw", "created_at": "2026-01-01 00:00:00"},
        ]
    )

    preferred = ops_dashboard_data.select_preferred_series_rows(series)

    assert preferred["series_id"].tolist() == ["level.raw", "rain.approved"]


def test_derive_station_kind_from_variable_coverage() -> None:
    assert ops_dashboard_data.derive_station_kind(["rain"]) == "rain"
    assert ops_dashboard_data.derive_station_kind(["level"]) == "level"
    assert ops_dashboard_data.derive_station_kind(["flow"]) == "level"
    assert ops_dashboard_data.derive_station_kind(["rain", "flow"]) == "mixed"


def test_load_station_catalog_classifies_status_from_observed_values(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_id=1001, station_code="1001", station_name="OK")
        insert_station(connection, station_id=1002, station_code="1002", station_name="ISSUE")
        insert_station(connection, station_id=1003, station_code="1003", station_name="NODATA")

        insert_observed_series(connection, series_id="1001.rain.raw", station_id=1001, variable_code="rain", state="raw")
        insert_observed_series(connection, series_id="1002.rain.raw", station_id=1002, variable_code="rain", state="raw")
        insert_observed_series(connection, series_id="1003.rain.raw", station_id=1003, variable_code="rain", state="raw")

        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-03-16 00:00:00", value=5.0)
        insert_observed_value(connection, series_id="1002.rain.raw", observed_at="2026-03-16 00:00:00", value=None)
        insert_observed_value(connection, series_id="1003.rain.raw", observed_at="2026-01-01 00:00:00", value=2.0)
        connection.commit()

    catalog = ops_dashboard_data.load_station_catalog(
        db_path, start_time=datetime(2026, 2, 15), end_time=now
    )
    status_by_station = dict(zip(catalog["station_id"], catalog["status"]))

    assert status_by_station == {
        "1001": "ok",
        "1002": "data_issue",
        "1003": "no_data",
    }
    assert set(catalog.columns).issuperset(
        {"station_id", "station_code", "provider_code", "station_name", "lat", "lon", "kind", "status", "status_reason"}
    )


def test_load_station_catalog_handles_all_stations_without_recent_values(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_id=1001, station_code="1001", station_name="NODATA")
        insert_observed_series(connection, series_id="1001.rain.raw", station_id=1001, variable_code="rain", state="raw")
        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-01-01 00:00:00", value=2.0)
        connection.commit()

    catalog = ops_dashboard_data.load_station_catalog(
        db_path, start_time=datetime(2026, 2, 15), end_time=now
    )

    assert catalog["station_id"].tolist() == ["1001"]
    assert catalog["kind"].tolist() == ["rain"]
    assert catalog["status"].tolist() == ["no_data"]
    assert catalog["rows_recent"].tolist() == [0]


def test_load_observed_series_returns_only_preferred_state_for_station(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    now = datetime(2026, 3, 17, 12, 0, 0)

    with sqlite3.connect(db_path) as connection:
        insert_station(connection, station_id=1001, station_code="1001", station_name="TESTE")
        insert_observed_series(
            connection,
            series_id="1001.rain.raw",
            station_id=1001,
            variable_code="rain",
            state="raw",
            created_at="2026-03-10 00:00:00",
        )
        insert_observed_series(
            connection,
            series_id="1001.rain.curated",
            station_id=1001,
            variable_code="rain",
            state="curated",
            created_at="2026-03-11 00:00:00",
        )
        insert_observed_series(connection, series_id="1001.level.raw", station_id=1001, variable_code="level", state="raw")

        insert_observed_value(connection, series_id="1001.rain.raw", observed_at="2026-03-16 01:00:00", value=1.0)
        insert_observed_value(connection, series_id="1001.rain.curated", observed_at="2026-03-16 01:00:00", value=2.5)
        insert_observed_value(connection, series_id="1001.level.raw", observed_at="2026-03-16 01:00:00", value=120.0)
        connection.commit()

    observed = ops_dashboard_data.load_observed_series(
        1001, db_path, start_time=datetime(2026, 2, 15), end_time=now
    )

    assert observed.to_dict(orient="records") == [
        {"datetime": pd.Timestamp("2026-03-16 01:00:00"), "variable_code": "level", "value": 120.0},
        {"datetime": pd.Timestamp("2026-03-16 01:00:00"), "variable_code": "rain", "value": 2.5},
    ]


def write_model_outputs(path: Path) -> Path:
    times = pd.date_range("2026-02-01", periods=72, freq="h")
    dataset = xr.Dataset(
        data_vars={
            "q": (("time", "mini"), np.column_stack([np.arange(72), 1000 + np.arange(72)])),
            "y": (("time", "mini"), np.column_stack([2000 + np.arange(72), 3000 + np.arange(72)])),
            "time_segment": (("time",), np.r_[np.zeros(48, dtype=np.int8), np.ones(24, dtype=np.int8)]),
        },
        coords={"time": times, "mini": [0, 1], "mini_id": ("mini", [101, 539])},
        attrs={
            "window_start": "2026-02-01T00:00:00",
            "reference_time": "2026-02-02T23:00:00",
            "window_end_exclusive": "2026-02-04T00:00:00",
        },
    )
    dataset.to_netcdf(path)
    return path


def test_load_mgb_series_splits_current_and_forecast(tmp_path) -> None:
    source = write_model_outputs(tmp_path / "model_outputs.nc")
    series = ops_dashboard_data.load_mgb_series(source, mini_id=539, variable_code="q")

    assert series["prev_flag"].tolist() == ([0] * 48) + ([1] * 24)
    assert series["value"].tolist()[0] == 1000.0
    assert series["value"].tolist()[-1] == 1071.0
    assert series["display_name"].tolist()[0] == "QTUDO"
    assert series["unit"].tolist()[0] == "m3/s"
    assert series["dt"].iloc[0] == pd.Timestamp("2026-02-01 00:00:00")
    assert series["dt"].iloc[47] == pd.Timestamp("2026-02-02 23:00:00")
    assert series["dt"].iloc[48] == pd.Timestamp("2026-02-03 00:00:00")


def test_list_model_variables_returns_static_mgb_catalog() -> None:
    variables = ops_dashboard_data.list_model_variables()

    assert variables.to_dict(orient="records") == [
        {"variable_code": "q", "display_name": "QTUDO", "unit": "m3/s"},
        {"variable_code": "y", "display_name": "YTUDO", "unit": "m"},
    ]


def test_load_mgb_series_rejects_unknown_mini_id(tmp_path) -> None:
    source = write_model_outputs(tmp_path / "model_outputs.nc")
    with pytest.raises(ValueError, match="Mini 999 was not found"):
        ops_dashboard_data.load_mgb_series(source, mini_id=999, variable_code="q")


def test_model_outputs_metadata_mismatch_is_blocked(tmp_path) -> None:
    source = write_model_outputs(tmp_path / "model_outputs.nc")
    expected = DashboardWindow(
        start_time=datetime(2026, 2, 1),
        cutoff_time=datetime(2026, 2, 3),
        forecast_end_exclusive=datetime(2026, 2, 4),
    )

    with pytest.raises(StaleModelOutputsError, match="expected.*actual"):
        ops_dashboard_data.validate_model_outputs_netcdf(
            source, expected_window=expected
        )
