from __future__ import annotations

import sqlite3

import geopandas as gpd
import pandas as pd
import pytest
from shapely.geometry import LineString, Polygon

from apps.ops_dashboard.state import DashboardState
import apps.ops_dashboard.state as dashboard_state
from mgb_ops.analysis.basins import basin_spatial_data
from mgb_ops.assets.databases import apply_schema
from mgb_ops.assets.schemas import SQL_DIR


def test_report_basin_load_seeds_stations_and_failed_load_preserves_selection(tmp_path, monkeypatch):
    history = tmp_path / "data" / "history.sqlite"
    apply_schema(history, SQL_DIR / "history_schema.sql")
    with sqlite3.connect(history) as connection:
        connection.execute(
            "INSERT OR IGNORE INTO provider (provider_code, provider_name, provider_type) VALUES ('ana', 'ANA', 'government')"
        )
        connection.executemany(
            "INSERT INTO station (station_id, station_code, station_name, provider_code, mini_id, latitude, longitude) "
            "VALUES (?, ?, ?, 'ana', ?, ?, ?)",
            [
                ("ana:inside", "inside", "Inside", 1, -30.0, -52.0),
                ("ana:outside", "outside", "Outside", 2, -30.0, -51.8),
            ],
        )
    catchments = gpd.GeoDataFrame(
        {"mini_id": [1], "mini_jus": [99], "mini_area": [50.0]},
        geometry=[Polygon([(-52.1, -30.1), (-51.9, -30.1), (-51.9, -29.9), (-52.1, -29.9)])],
        crs="EPSG:4326",
    )
    monkeypatch.setattr(
        dashboard_state, "_basin_spatial_data",
        lambda mini_id, *_: basin_spatial_data(catchments, mini_id),
    )
    monkeypatch.setattr(
        dashboard_state, "_mini_segment_paths",
        lambda *_: pd.DataFrame([{"mini_id": 1, "path": [[-52.0, -30.0], [-51.95, -29.95]]}]),
    )
    state = DashboardState(tmp_path)

    selected = state.load_report_basin(1, 0)
    assert selected == ("ana:inside",)
    assert state.report_map_station_ids == ["ana:inside"]
    assert "report-basin-buffer" in [layer["id"] for layer in state.report_map_artifacts.spec["layers"]]

    state.report_map_station_ids = ["ana:outside"]
    state.rebuild_report_map()
    station_layer = next(layer for layer in state.report_map_artifacts.spec["layers"] if layer["id"] == "stations")
    assert station_layer["data"]["features"][0]["properties"]["station_id"] == "ana:outside"

    with pytest.raises(ValueError, match="Target ID 99"):
        state.load_report_basin(99, 0)
    assert state.report_map_station_ids == ["ana:outside"]
