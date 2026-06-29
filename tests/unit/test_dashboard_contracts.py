from __future__ import annotations

from datetime import datetime
from pathlib import Path
import sqlite3

import geopandas as gpd
from shapely.geometry import LineString, Polygon

from apps.ops_dashboard.services.deckgl import build_sqlite_version
from mgb_ops.analysis.spatial_layers import read_mini_layer, read_mini_layers
from mgb_ops.common.time_utils import DashboardWindow, resolve_dashboard_window
from mgb_ops.edit.forcing import ForecastCorrectionInstruction, validate_instruction


def test_resolve_dashboard_window_uses_reference_date_and_configured_horizon() -> None:
    settings = {
        "run": {"reference_time": "2026-03-17T12:00:00"},
        "mgb": {"output_days_before": 2, "forecast_horizon_days": 3},
    }

    assert resolve_dashboard_window(settings) == DashboardWindow(
        start_time=datetime(2026, 3, 15),
        cutoff_time=datetime(2026, 3, 17, 12),
        forecast_end_exclusive=datetime(2026, 3, 21),
    )


def test_read_mini_layers_normalizes_crs_ids_and_click_tokens(tmp_path: Path) -> None:
    path = tmp_path / "minis.gpkg"
    segments = gpd.GeoDataFrame(
        {"mini_id": ["7"]},
        geometry=[LineString([(-51.0, -30.0), (-50.5, -29.5)])],
        crs="EPSG:4326",
    )
    catchments = gpd.GeoDataFrame(
        {"mini_id": [7]},
        geometry=[Polygon([(-51, -30), (-50, -30), (-50, -29), (-51, -29)])],
        crs="EPSG:4326",
    ).to_crs("EPSG:4618")
    segments.to_file(path, layer="mini_segments", driver="GPKG")
    catchments.to_file(path, layer="mini_catchments", driver="GPKG")

    layers = read_mini_layers(path)

    assert layers.mini_segments.crs.to_epsg() == 4326
    assert layers.mini_catchments.crs.to_epsg() == 4326
    assert layers.mini_segments["mini_id"].tolist() == [7]
    assert layers.mini_catchments["click_id"].tolist() == ["MINI|7"]


def test_read_mini_layer_rejects_missing_mini_id(tmp_path: Path) -> None:
    path = tmp_path / "minis.gpkg"
    gpd.GeoDataFrame(
        {"other": [1]},
        geometry=[LineString([(0, 0), (1, 1)])],
        crs="EPSG:4326",
    ).to_file(path, layer="mini_segments", driver="GPKG")

    try:
        read_mini_layer(path, "mini_segments")
    except ValueError as exc:
        assert "mini_id" in str(exc)
    else:
        raise AssertionError("Expected missing mini_id validation failure.")


def test_forecast_instruction_validation_rejects_invalid_range_and_factor() -> None:
    try:
        validate_instruction(ForecastCorrectionInstruction("asset", 6, 3))
    except ValueError as exc:
        assert "t1_step" in str(exc)
    else:
        raise AssertionError("Expected invalid step range.")

    try:
        validate_instruction(
            ForecastCorrectionInstruction("asset", 0, 3, multiplication_factor=0)
        )
    except ValueError as exc:
        assert "multiplication_factor" in str(exc)
    else:
        raise AssertionError("Expected invalid multiplication factor.")


def test_sqlite_version_includes_wal_state(tmp_path: Path) -> None:
    path = tmp_path / "history.sqlite"
    with sqlite3.connect(path) as connection:
        connection.execute("PRAGMA journal_mode=WAL")
        connection.execute("CREATE TABLE sample (value INTEGER)")
        connection.commit()
        before = build_sqlite_version(path)
        connection.execute("INSERT INTO sample VALUES (1)")
        connection.commit()
        after = build_sqlite_version(path)

    assert before != after
