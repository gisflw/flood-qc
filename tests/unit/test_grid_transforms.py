from __future__ import annotations

import numpy as np
import pandas as pd
import pytest

from mgb_ops.assets.grid_transforms import (
    bilinear_resample,
    build_grid_idw_neighbors,
    build_idw_neighbors,
    idw_interpolate,
    interpolate_station_chunk,
    interpolate_station_values,
)
from mgb_ops.assets.spatial_grid import RegularGridSpec


def test_regular_grid_spec_builds_cell_centers_and_validates_geometry() -> None:
    grid = RegularGridSpec((-2.0, -1.0, 2.0, 1.0), resolution_degrees=1.0)

    np.testing.assert_allclose(grid.longitudes, [-1.5, -0.5, 0.5, 1.5])
    np.testing.assert_allclose(grid.latitudes, [-0.5, 0.5])
    assert grid.shape == (2, 4)

    with pytest.raises(ValueError, match="west < east"):
        RegularGridSpec((1.0, -1.0, 1.0, 1.0), 1.0)
    with pytest.raises(ValueError, match="resolution"):
        RegularGridSpec((-1.0, -1.0, 1.0, 1.0), 0.0)


def test_idw_uses_exact_point_and_ignores_missing_sources() -> None:
    result = idw_interpolate(
        np.array([0.0, 1.0, 2.0]),
        np.array([0.0, 0.0, 0.0]),
        np.array([4.0, np.nan, 8.0]),
        np.array([0.0, 1.0]),
        np.array([0.0, 0.0]),
        nearest_stations=2,
        power=2.0,
    )

    assert result[0] == pytest.approx(4.0)
    assert np.isfinite(result[1])


def test_reusable_idw_neighbors_apply_per_time_validity() -> None:
    sources = pd.DataFrame({"lat": [0.0, 0.0], "lon": [0.0, 2.0]})
    targets = pd.DataFrame({"lat": [0.0], "lon": [1.0]})
    nearest, weights = build_idw_neighbors(
        targets, sources, nearest_stations=2, power=2.0
    )

    result = interpolate_station_chunk(
        np.array([[2.0, np.nan], [6.0, 8.0]]),
        nearest_idx=nearest,
        weights=weights,
    )

    np.testing.assert_allclose(result, [[4.0, 8.0]])


def test_grid_neighbor_builder_and_station_grid_interpolation_shapes() -> None:
    targets = pd.DataFrame({"lat": [0.0], "lon": [0.0]})
    nearest, weights = build_grid_idw_neighbors(
        targets,
        latitudes=np.array([-0.5, 0.5]),
        longitudes=np.array([-0.5, 0.5]),
        nearest_points=4,
        power=2.0,
    )
    assert nearest.shape == weights.shape == (1, 4)

    grid = RegularGridSpec((-1.0, -1.0, 1.0, 1.0), 1.0)
    stations = pd.DataFrame({"lat": [0.0], "lon": [0.0], "value": [3.0]})
    np.testing.assert_allclose(interpolate_station_values(stations, grid), 3.0)


def test_bilinear_resampling_validates_shape_and_interpolates() -> None:
    source = np.array([[0.0, 2.0], [2.0, 4.0]])
    result = bilinear_resample(
        source,
        np.array([0.0, 2.0]),
        np.array([0.0, 2.0]),
        np.array([1.0]),
        np.array([1.0]),
    )
    np.testing.assert_allclose(result, [[2.0]])

    with pytest.raises(ValueError, match="shape"):
        bilinear_resample(
            np.ones((3, 2)),
            np.array([0.0, 2.0]),
            np.array([0.0, 2.0]),
            np.array([1.0]),
            np.array([1.0]),
        )
