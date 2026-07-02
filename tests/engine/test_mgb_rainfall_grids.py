from datetime import datetime, timezone

import numpy as np
import pandas as pd

from mgb_ops.assets.spatial_grid import RegularGridSpec, read_spatial_grid, write_spatial_grid
from mgb_ops.model.prepare_mgb_rainfall import (
    _build_forecast_working_cache,
    _grid_to_mini_matrix,
)


def test_forecast_working_cache_splits_native_intervals_and_preserves_totals(tmp_path):
    source = tmp_path / "native.nc"
    bounds = [
        (
            datetime(2026, 3, 12, 0, tzinfo=timezone.utc),
            datetime(2026, 3, 12, 3, tzinfo=timezone.utc),
        ),
        (
            datetime(2026, 3, 12, 3, tzinfo=timezone.utc),
            datetime(2026, 3, 12, 6, tzinfo=timezone.utc),
        ),
    ]
    write_spatial_grid(
        source,
        variable="precipitation",
        grid_type="forecast",
        source="cropped_from_native_grid",
        providers=["ecmwf"],
        units="mm",
        bbox=(-52.0, -31.0, -51.0, -30.0),
        resolution_degrees=0.5,
        times_utc=[end for _, end in bounds],
        time_bounds_utc=bounds,
        latitudes=np.array([-30.75, -30.25]),
        longitudes=np.array([-51.75, -51.25]),
        values=np.stack([np.full((2, 2), 6.0), np.full((2, 2), 9.0)]),
        timestep_hours=None,
    )

    target = _build_forecast_working_cache(
        source,
        tmp_path / "working.nc",
        grid_spec=RegularGridSpec((-52.0, -31.0, -51.0, -30.0), 0.5),
        forecast_start_time=datetime(2026, 3, 12, 0),
        forecast_nt=4,
        timestep_hours=1,
    )

    grid = read_spatial_grid(target)
    np.testing.assert_allclose(grid.values[:, 0, 0], [2.0, 3.0, 3.0, 3.0])
    assert grid.grid_type == "forecast"
    assert grid.metadata["temporal_resampling_method"] == "uniform_interval_split"


def test_grid_to_mini_matrix_uses_mini_gtp_centroid_order(tmp_path):
    path = write_spatial_grid(
        tmp_path / "working.nc",
        variable="precipitation",
        grid_type="observed",
        source="interpolated_from_stations",
        providers=["ana"],
        units="mm",
        bbox=(0.0, 0.0, 2.0, 1.0),
        resolution_degrees=1.0,
        times_utc=[datetime(2026, 3, 12, 1, tzinfo=timezone.utc)],
        latitudes=np.array([0.5]),
        longitudes=np.array([0.5, 1.5]),
        values=np.array([[[2.0, 6.0]]]),
    )
    minis = pd.DataFrame({
        "mini_id": [2, 1],
        "lon": [1.5, 0.5],
        "lat": [0.5, 0.5],
    })

    values = _grid_to_mini_matrix(
        read_spatial_grid(path),
        minis,
        nearest_points=1,
        power=2.0,
        chunk_hours=24,
    )

    np.testing.assert_allclose(values, [[6.0], [2.0]])


def test_inclusive_working_grid_keeps_all_bbox_touching_cells():
    grid = RegularGridSpec(
        (0.0, 0.0, 2.0, 1.0),
        1.0,
        include_boundary_cells=True,
    )

    np.testing.assert_allclose(grid.longitudes, [-0.5, 0.5, 1.5, 2.5])
    np.testing.assert_allclose(grid.latitudes, [-0.5, 0.5, 1.5])
    assert grid.effective_bbox == (-1.0, -1.0, 3.0, 2.0)
