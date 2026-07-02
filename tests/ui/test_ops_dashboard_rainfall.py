from datetime import datetime, timedelta, timezone

import numpy as np
import pytest

from apps.ops_dashboard.services import loaders
from mgb_ops.assets.spatial_grid import write_spatial_grid
from mgb_ops.assets.types import AnalysisWindow


def _cache(path):
    times = [
        datetime(2026, 1, 2, hour, tzinfo=timezone.utc)
        for hour in range(4, 24)
    ] + [
        datetime(2026, 1, 3, hour, tzinfo=timezone.utc)
        for hour in range(0, 4)
    ]
    return write_spatial_grid(
        path,
        variable="precipitation",
        grid_type="observed",
        source="interpolated_from_stations",
        providers=["ana"],
        units="mm",
        bbox=(-52.5, -31.5, -50.5, -29.5),
        resolution_degrees=0.5,
        times_utc=times,
        latitudes=np.array([-31.25, -30.75, -30.25, -29.75]),
        longitudes=np.array([-52.25, -51.75, -51.25, -50.75]),
        values=np.ones((24, 4, 4)),
    )


def test_accumulation_raster_reads_cache(tmp_path):
    cache = _cache(tmp_path / "precipitations_mgb_observed.nc")
    window = AnalysisWindow(
        start_time=datetime(2026, 1, 1),
        cutoff_time=datetime(2026, 1, 3),
        forecast_end_exclusive=datetime(2026, 1, 4),
    )
    raster = loaders._accumulation_raster(
        str(cache), str(tmp_path), "v1", window,
        (-52.0, -31.0, -51.0, -30.0), 0.5, 24,
    )
    assert np.all(raster["grid"].values == 24)
    assert raster["name"] == "observed_accum_24h"
    assert raster["rainfall_mode"] == "observed"


def test_forecast_accumulation_starts_at_reference_time(tmp_path):
    reference_utc = datetime(2026, 1, 3, 3, tzinfo=timezone.utc)
    times = [reference_utc + timedelta(hours=offset) for offset in (1, 2, 3)]
    cache = write_spatial_grid(
        tmp_path / "precipitations_mgb_forecast.nc",
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=["ecmwf"],
        units="mm",
        bbox=(-52.5, -31.5, -50.5, -29.5),
        resolution_degrees=0.5,
        times_utc=times,
        latitudes=np.array([-31.25, -30.75, -30.25, -29.75]),
        longitudes=np.array([-52.25, -51.75, -51.25, -50.75]),
        values=np.stack(
            [np.full((4, 4), value) for value in (2.0, 3.0, 4.0)]
        ),
    )
    window = AnalysisWindow(
        start_time=datetime(2026, 1, 1),
        cutoff_time=datetime(2026, 1, 3),
        forecast_end_exclusive=datetime(2026, 1, 4),
    )

    raster = loaders._accumulation_raster(
        str(cache),
        str(tmp_path),
        "v1",
        window,
        (-52.0, -31.0, -51.0, -30.0),
        0.5,
        3,
        rainfall_mode="forecast",
    )

    assert np.all(raster["grid"].values == 9)
    assert raster["grid"].start_time == reference_utc
    assert raster["grid"].end_time == reference_utc + timedelta(hours=3)
    assert raster["name"] == "forecast_accum_3h"


def test_forecast_accumulation_requires_complete_coverage(tmp_path):
    reference_utc = datetime(2026, 1, 3, 3, tzinfo=timezone.utc)
    cache = write_spatial_grid(
        tmp_path / "precipitations_mgb_forecast.nc",
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=["ecmwf"],
        units="mm",
        bbox=(-52.5, -31.5, -50.5, -29.5),
        resolution_degrees=0.5,
        times_utc=[reference_utc + timedelta(hours=2)],
        time_bounds_utc=[
            (
                reference_utc + timedelta(hours=1),
                reference_utc + timedelta(hours=2),
            )
        ],
        latitudes=np.array([-31.25, -30.75, -30.25, -29.75]),
        longitudes=np.array([-52.25, -51.75, -51.25, -50.75]),
        values=np.ones((1, 4, 4)),
    )
    window = AnalysisWindow(
        start_time=datetime(2026, 1, 1),
        cutoff_time=datetime(2026, 1, 3),
        forecast_end_exclusive=datetime(2026, 1, 4),
    )

    with pytest.raises(ValueError, match="incompletely covers"):
        loaders._accumulation_raster(
            str(cache),
            str(tmp_path),
            "v1",
            window,
            (-52.0, -31.0, -51.0, -30.0),
            0.5,
            2,
            rainfall_mode="forecast",
        )


@pytest.mark.parametrize("hours", [0, -1, 1.5, True])
def test_accumulation_raster_rejects_invalid_hours(tmp_path, hours):
    window = AnalysisWindow(datetime(2026, 1, 1), datetime(2026, 1, 3), datetime(2026, 1, 4))
    with pytest.raises(ValueError, match="positive integer"):
        loaders._accumulation_raster(
            str(tmp_path / "missing.nc"), str(tmp_path), "v1", window,
            (-52.0, -31.0, -51.0, -30.0), 0.5, hours,
        )


def test_accumulation_raster_rejects_invalid_mode(tmp_path):
    window = AnalysisWindow(
        datetime(2026, 1, 1),
        datetime(2026, 1, 3),
        datetime(2026, 1, 4),
    )
    with pytest.raises(ValueError, match="Rainfall mode"):
        loaders._accumulation_raster(
            str(tmp_path / "missing.nc"),
            str(tmp_path),
            "v1",
            window,
            (-52.0, -31.0, -51.0, -30.0),
            0.5,
            1,
            rainfall_mode="radar",
        )
