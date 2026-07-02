from datetime import datetime, timezone
import sqlite3

import numpy as np

from db_helpers import initialize_history_db
from mgb_ops.assets.spatial_grid import read_spatial_grid
from mgb_ops.assets.observed_precipitation import build_observed_precipitation_cache


def test_observed_precipitation_cache_combines_requested_providers_in_utc(tmp_path):
    database = initialize_history_db(tmp_path / "history.sqlite")
    with sqlite3.connect(database) as connection:
        connection.executemany(
            """INSERT INTO station
               (station_id, station_code, station_name, provider_code, latitude, longitude)
               VALUES (?, ?, ?, ?, ?, ?)""",
            [
                ("ana:a", "a", "ANA", "ana", -30.75, -51.75),
                ("inmet:b", "b", "INMET", "inmet", -30.25, -51.25),
            ],
        )
        connection.executemany(
            """INSERT INTO observed_series (series_id, station_id, variable_code, state)
               VALUES (?, ?, 'rain', 'raw')""",
            [("ana:a.rain.raw", "ana:a"), ("inmet:b.rain.raw", "inmet:b")],
        )
        connection.executemany(
            "INSERT INTO observed_value (series_id, observed_at, value) VALUES (?, ?, ?)",
            [
                ("ana:a.rain.raw", "2026-03-11 01:00", 2.0),
                ("inmet:b.rain.raw", "2026-03-11 01:00", 4.0),
                ("ana:a.rain.raw", "2026-03-11 02:00", 3.0),
                ("inmet:b.rain.raw", "2026-03-11 02:00", 5.0),
            ],
        )

    path = build_observed_precipitation_cache(
        database,
        tmp_path / "cache",
        bbox=(-52.0, -31.0, -51.0, -30.0),
        resolution_degrees=0.5,
        start_time_utc=datetime(2026, 3, 11, 3, tzinfo=timezone.utc),
        end_time_utc=datetime(2026, 3, 11, 5, tzinfo=timezone.utc),
        timestep_hours=1,
        providers=["inmet", "ana"],
        nearest_stations=2,
        power=2.0,
    )

    assert path.name == "precipitations_observed.nc"
    grid = read_spatial_grid(path)
    assert grid.providers == ("ana", "inmet")
    assert grid.grid_type == "observed"
    assert grid.source == "interpolated_from_stations"
    assert grid.times_utc == (
        datetime(2026, 3, 11, 4, tzinfo=timezone.utc),
        datetime(2026, 3, 11, 5, tzinfo=timezone.utc),
    )
    assert np.isfinite(grid.values).all()


def test_observed_precipitation_cache_rejects_naive_api_times(tmp_path):
    database = initialize_history_db(tmp_path / "history.sqlite")
    try:
        build_observed_precipitation_cache(
            database,
            tmp_path / "cache",
            bbox=(-52.0, -31.0, -51.0, -30.0),
            resolution_degrees=0.5,
            start_time_utc=datetime(2026, 3, 11, 3),
            end_time_utc=datetime(2026, 3, 11, 5, tzinfo=timezone.utc),
            timestep_hours=1,
            providers=["ana"],
        )
    except ValueError as exc:
        assert "timezone-aware UTC" in str(exc)
    else:
        raise AssertionError("Expected naïve UTC API input to be rejected.")
