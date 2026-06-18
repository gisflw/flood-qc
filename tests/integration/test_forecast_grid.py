from __future__ import annotations

import shutil
import sqlite3
from datetime import datetime
from pathlib import Path

import numpy as np

from mgb_ops.ingest import forecast_grid
from db_helpers import initialize_history_db


class FakeTemporaryDirectory:
    def __init__(self, path: Path) -> None:
        self.path = path

    def __enter__(self) -> str:
        self.path.mkdir(parents=True, exist_ok=True)
        return str(self.path)

    def __exit__(self, exc_type, exc, tb) -> None:
        shutil.rmtree(self.path, ignore_errors=True)


def test_build_ecmwf_cycle_uses_next_local_hour_and_converts_to_utc() -> None:
    cycle_time = forecast_grid.build_ecmwf_cycle(datetime(2026, 3, 18, 23, 0, 0))

    assert cycle_time == datetime(2026, 3, 19, 0, 0, 0)


def test_ingest_forecast_grids_stores_only_cropped_asset(tmp_path, monkeypatch) -> None:
    history_db = tmp_path / "history.sqlite"
    initialize_history_db(history_db)
    temp_dir = tmp_path / "temp_download"

    monkeypatch.setattr(
        forecast_grid.tempfile,
        "TemporaryDirectory",
        lambda prefix="": FakeTemporaryDirectory(temp_dir),
    )

    def fake_download(target_path: Path, *, reference_time: datetime, product_config=None) -> None:
        target_path.write_bytes(b"raw-grib")

    def fake_crop(source_path: Path, target_path: Path, *, bbox) -> None:
        assert source_path.exists()
        assert bbox == (-72.0, -44.0, -36.0, -17.0)
        target_path.parent.mkdir(parents=True, exist_ok=True)
        target_path.write_bytes(b"cropped-grib")

    monkeypatch.setattr(forecast_grid, "download_ecmwf_grib_to_path", fake_download)
    monkeypatch.setattr(forecast_grid, "crop_grib_to_bbox", fake_crop)
    monkeypatch.setattr(
        forecast_grid,
        "extract_valid_time_bounds",
        lambda _: (datetime(2026, 3, 11, 3, 0, 0), datetime(2026, 3, 26, 0, 0, 0)),
    )

    summary = forecast_grid.ingest_forecast_grids(
        history_db,
        reference_time=datetime(2026, 3, 11, 23, 0, 0),
        bbox=(-60.0, -35.0, -48.0, -26.0),
        buffer_fraction=1.0,
        downloads_dir=tmp_path / "data" / "downloads",
        logs_dir=tmp_path / "logs",
        asset_base_dir=tmp_path,
    )

    assert summary.asset_path.exists()
    assert not temp_dir.exists()

    with sqlite3.connect(history_db) as connection:
        row = connection.execute(
            """
            SELECT asset_kind, format, provider_code, relative_path, valid_from, valid_to
            FROM asset
            WHERE provider_code = 'ecmwf'
            """
        ).fetchone()

    assert row == (
        forecast_grid.ECMWF_ASSET_KIND,
        "GRIB2",
        "ecmwf",
        summary.asset_path.relative_to(tmp_path).as_posix(),
        "2026-03-11T03:00:00",
        "2026-03-26T00:00:00",
    )


def test_build_grid_arrays_uses_coordinate_arrays_for_wrapped_global_grid(monkeypatch) -> None:
    class FakeEccodes:
        @staticmethod
        def codes_get(gid, key):
            if key == "gridType":
                return "regular_ll"
            raise KeyError(key)

        @staticmethod
        def codes_get_long(gid, key):
            if key == "Ni":
                return 4
            if key == "Nj":
                return 2
            raise KeyError(key)

        @staticmethod
        def codes_get_array(gid, key):
            if key == "values":
                return np.array([1.0, 2.0, 3.0, 4.0, 10.0, 20.0, 30.0, 40.0], dtype=np.float64)
            if key == "latitudes":
                return np.array([1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0], dtype=np.float64)
            if key == "longitudes":
                return np.array([180.0, 180.25, 180.5, 180.75, 180.0, 180.25, 180.5, 180.75], dtype=np.float64)
            raise KeyError(key)

    monkeypatch.setattr(forecast_grid, "_require_eccodes", lambda: FakeEccodes())

    latitudes, longitudes, values = forecast_grid._build_grid_arrays(object())

    assert latitudes.tolist() == [1.0, -1.0]
    assert longitudes.tolist() == [-179.75, -179.5, -179.25, 180.0]
    assert values.tolist() == [[2.0, 3.0, 4.0, 1.0], [20.0, 30.0, 40.0, 10.0]]
