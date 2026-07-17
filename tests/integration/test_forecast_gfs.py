from __future__ import annotations

from datetime import datetime
import logging
from pathlib import Path

import numpy as np
import pytest

from mgb_ops.adapters import forecast_gfs
from mgb_ops.adapters._grib2 import TpGribMessage
from mgb_ops.adapters.forecast_ecmwf import build_native_interval_precipitation_from_interval_messages
from mgb_ops.adapters.forecast_ecmwf import build_bbox_with_buffer


def test_build_gfs_url_uses_nomads_filter_shape() -> None:
    url, params = forecast_gfs.build_gfs_url(
        cycle_time=datetime(2026, 3, 18, 18),
        forecast_hour=3,
        bbox=(-75.0, -35.0, -32.0, 6.0),
        variables=["APCP"],
        levels=["surface"],
    )

    assert url == forecast_gfs.GFS_NOMADS_FILTER_URL
    assert params["file"] == "gfs.t18z.pgrb2.0p25.f003"
    assert params["dir"] == "/gfs.20260318/18/atmos"
    assert params["var_APCP"] == "on"
    assert params["lev_surface"] == "on"
    assert params["leftlon"] == "-75.0"
    assert params["toplat"] == "6.0"


def test_request_gfs_file_rejects_html_error_page(monkeypatch) -> None:
    class FakeResponse:
        content = b"<html>not found</html>"
        text = "<html>not found</html>"

        @staticmethod
        def raise_for_status() -> None:
            return None

    class FakeSession:
        @staticmethod
        def get(*args, **kwargs):
            return FakeResponse()

    with pytest.raises(RuntimeError, match="did not return a GRIB2"):
        forecast_gfs.request_gfs_file(
            cycle_time=datetime(2026, 3, 18, 18),
            forecast_hour=3,
            bbox=(-75.0, -35.0, -32.0, 6.0),
            session=FakeSession(),
        )


def test_gfs_interval_messages_preserve_native_accumulation_bounds() -> None:
    message = TpGribMessage(
        valid_time=datetime(2026, 3, 18, 21),
        step_hours=3,
        start_step_hours=0,
        latitudes=np.array([-30.0], dtype=np.float64),
        longitudes=np.array([-52.0], dtype=np.float64),
        values_mm=np.array([[12.0]], dtype=np.float64),
    )

    times, bounds, latitudes, longitudes, values = build_native_interval_precipitation_from_interval_messages([message])

    assert times == (datetime(2026, 3, 18, 21),)
    assert bounds == ((datetime(2026, 3, 18, 18), datetime(2026, 3, 18, 21)),)
    assert latitudes.tolist() == [-30.0]
    assert longitudes.tolist() == [-52.0]
    assert values[:, 0, 0].tolist() == [12.0]


def test_gfs_steps_stop_after_the_required_forecast_window() -> None:
    steps = forecast_gfs.build_required_gfs_steps(
        datetime(2026, 3, 18, 0),
        datetime(2026, 3, 28, 1),
    )

    assert steps[0] == 3
    assert steps[-2:] == [240, 252]
    assert len(steps) == 81


def test_gfs_request_uses_the_explicit_forecast_extent() -> None:
    model_bbox = (-52.0, -31.0, -50.0, -30.0)
    forecast_bbox = build_bbox_with_buffer(model_bbox, buffer_fraction=2.0)
    _, params = forecast_gfs.build_gfs_url(
        cycle_time=datetime(2026, 3, 18, 0),
        forecast_hour=3,
        bbox=forecast_bbox,
        variables=["APCP"],
        levels=["surface"],
    )

    assert forecast_bbox == (-56.0, -33.0, -46.0, -28.0)
    assert params["leftlon"] == "-56.0"
    assert params["rightlon"] == "-46.0"


def test_gfs_download_reports_each_request_to_the_run_log(tmp_path, monkeypatch, caplog) -> None:
    class FakeResponse:
        content = b"GRIB payload"
        text = ""

        @staticmethod
        def raise_for_status() -> None:
            return None

    class FakeSession:
        def __enter__(self):
            return self

        def __exit__(self, *args) -> None:
            return None

        @staticmethod
        def get(*args, **kwargs):
            return FakeResponse()

    class FakeRequests:
        @staticmethod
        def Session():
            return FakeSession()

    monkeypatch.setattr(forecast_gfs, "_require_requests", lambda: FakeRequests())
    logger = logging.getLogger("test.noaa.progress")
    with caplog.at_level(logging.INFO, logger=logger.name):
        forecast_gfs.download_gfs_grib_to_path(
            Path(tmp_path) / "forecast.grib2",
            cycle_time=datetime(2026, 3, 18, 0),
            bbox=(-56.0, -33.0, -46.0, -28.0),
            forecast_hours=[3, 6],
            logger=logger,
            pause_seconds=0,
        )

    assert "forecast_hour=003" in caplog.text
    assert "forecast_hour=006" in caplog.text
