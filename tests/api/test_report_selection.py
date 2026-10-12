from __future__ import annotations

import sqlite3
from pathlib import Path

import geopandas as gpd
import pandas as pd
import pytest
from shapely.geometry import Point, Polygon

from mgb_ops.analysis.basins import select_stations_near_basin
from mgb_ops.analysis.reporting import load_report_stations
from mgb_ops.assets.report import normalize_report_columns
from mgb_ops.assets.databases import apply_schema
from mgb_ops.assets.schemas import SQL_DIR


@pytest.fixture
def history(tmp_path: Path) -> Path:
    path = tmp_path / "history.sqlite"
    apply_schema(path, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(path) as connection:
        connection.execute(
            "INSERT OR IGNORE INTO provider (provider_code, provider_name, provider_type) VALUES ('ana', 'ANA', 'government')"
        )
        connection.executemany(
            "INSERT INTO station (station_id, station_code, station_name, provider_code, latitude, longitude) "
            "VALUES (?, ?, ?, 'ana', ?, ?)",
            [
                ("ana:b", "b", "B", -30.0, -52.0),
                ("ana:a", "a", "A", -30.0, -52.1),
            ],
        )
    return path


def test_report_station_catalog_keeps_requested_order_and_can_load_every_station(history):
    assert list(load_report_stations(history)) == ["ana:a", "ana:b"]
    assert list(load_report_stations(history, ["ana:b", "ana:a"])) == ["ana:b", "ana:a"]
    assert load_report_stations(history, []) == {}


def test_basin_buffer_includes_boundary_and_skips_unlocated_stations():
    basin = gpd.GeoSeries(
        [Polygon([(-52.1, -30.1), (-51.9, -30.1), (-51.9, -29.9), (-52.1, -29.9)])],
        crs="EPSG:4326",
    )
    stations = pd.DataFrame([
        {"station_id": "inside", "lat": -30.0, "lon": -52.0},
        {"station_id": "edge", "lat": -30.0, "lon": -51.9},
        {"station_id": "near", "lat": -30.0, "lon": -51.895},
        {"station_id": "far", "lat": -30.0, "lon": -51.8},
        {"station_id": "unlocated", "lat": None, "lon": None},
    ])

    zero = select_stations_near_basin(basin, stations, 0)
    buffered = select_stations_near_basin(basin, stations, 1)

    assert zero.station_ids == ("inside", "edge")
    assert buffered.station_ids == ("inside", "edge", "near")
    assert buffered.buffered_geometry.iloc[0].covers(Point(-51.9, -30.0))


@pytest.mark.parametrize("distance", [-1, float("nan"), float("inf")])
def test_basin_buffer_rejects_invalid_distance(distance):
    basin = gpd.GeoSeries([Point(-52, -30).buffer(0.1)], crs="EPSG:4326")
    with pytest.raises(ValueError, match="buffer_km"):
        select_stations_near_basin(basin, pd.DataFrame(columns=["station_id", "lat", "lon"]), distance)


def test_custom_report_columns_accept_threshold_rows_without_numeric_fields():
    columns = normalize_report_columns([
        {"label": "Flood threshold", "type": "threshold"},
        {"label": "Level +3h", "type": "forecast", "variable": "level", "time_horizon": 3, "operation": "value"},
    ])
    assert columns == (
        {"label": "Flood threshold", "type": "threshold"},
        {"label": "Level +3h", "type": "forecast", "variable": "level", "time_horizon": 3, "operation": "value"},
    )
