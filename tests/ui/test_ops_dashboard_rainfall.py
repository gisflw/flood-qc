from datetime import datetime, timezone

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
        bbox=(-52.0, -31.0, -51.0, -30.0),
        resolution_degrees=0.5,
        times_utc=times,
        latitudes=np.array([-30.75, -30.25]),
        longitudes=np.array([-51.75, -51.25]),
        values=np.ones((24, 2, 2)),
    )


def test_accumulation_raster_reads_cache(tmp_path):
    cache = _cache(tmp_path / "precipitations_observed.nc")
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
    assert raster["name"] == "accum_24h"


@pytest.mark.parametrize("hours", [0, -1, 1.5, True])
def test_accumulation_raster_rejects_invalid_hours(tmp_path, hours):
    window = AnalysisWindow(datetime(2026, 1, 1), datetime(2026, 1, 3), datetime(2026, 1, 4))
    with pytest.raises(ValueError, match="positive integer"):
        loaders._accumulation_raster(
            str(tmp_path / "missing.nc"), str(tmp_path), "v1", window,
            (-52.0, -31.0, -51.0, -30.0), 0.5, hours,
        )
