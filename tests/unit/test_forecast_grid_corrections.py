from __future__ import annotations

from datetime import datetime, timezone

import numpy as np

from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec, read_spatial_grid, write_spatial_grid
from mgb_ops.edit.forcing import ForecastCorrectionInstruction, apply_grid_correction
from mgb_ops.model.prepare_mgb_rainfall import _build_forecast_working_cache


def _grid() -> PrecipitationGrid:
    return PrecipitationGrid(
        values=np.array([[1.0, 2.0], [3.0, 4.0]]),
        latitudes=np.array([0.5, 1.5]),
        longitudes=np.array([10.5, 11.5]),
        bounds=(10.0, 0.0, 12.0, 2.0),
        start_time=datetime(2026, 1, 1),
        end_time=datetime(2026, 1, 1, 1),
    )


def test_grid_correction_translates_footprint_without_zero_padding() -> None:
    corrected = apply_grid_correction(
        _grid(),
        ForecastCorrectionInstruction("asset", 0, 1, shift_lat=1, shift_lon=-1),
    )

    np.testing.assert_allclose(corrected.values, [[1.0, 2.0], [3.0, 4.0]])
    np.testing.assert_allclose(corrected.latitudes, [1.5, 2.5])
    np.testing.assert_allclose(corrected.longitudes, [9.5, 10.5])
    assert corrected.bounds == (9.0, 1.0, 11.0, 3.0)


def test_grid_correction_rotation_expands_extent_and_marks_outer_cells_missing() -> None:
    corrected = apply_grid_correction(
        _grid(), ForecastCorrectionInstruction("asset", 0, 1, rotation_deg=45)
    )

    assert corrected.bounds == (9.0, -1.0, 13.0, 3.0)
    assert corrected.values.shape == (4, 4)
    assert np.isnan(corrected.values).any()
    assert np.isfinite(corrected.values).any()


def test_working_cache_transforms_native_grid_before_model_resampling(
    tmp_path, monkeypatch
) -> None:
    source_path = tmp_path / "forecast.nc"
    target_path = tmp_path / "working.nc"
    write_spatial_grid(
        source_path,
        variable="precipitation",
        grid_type="forecast",
        source="resampled_from_grid",
        providers=["test"],
        units="mm",
        bbox=(0.0, 0.0, 2.0, 2.0),
        resolution_degrees=1.0,
        times_utc=[datetime(2026, 1, 1, 1, tzinfo=timezone.utc)],
        latitudes=np.array([0.5, 1.5]),
        longitudes=np.array([0.5, 1.5]),
        values=np.array([[[1.0, 2.0], [3.0, 4.0]]]),
        timestep_hours=1,
    )
    monkeypatch.setattr(
        "mgb_ops.model.prepare_mgb_rainfall.build_forecast_start_time_utc",
        lambda _: datetime(2026, 1, 1, 1),
    )

    _build_forecast_working_cache(
        source_path,
        target_path,
        grid_spec=RegularGridSpec((1.0, 0.0, 3.0, 2.0), 1.0),
        forecast_start_time=datetime(2026, 1, 1),
        forecast_nt=1,
        timestep_hours=1,
        correction=ForecastCorrectionInstruction("asset", 0, 1, shift_lon=1),
    )

    np.testing.assert_allclose(read_spatial_grid(target_path).values[0], [[1.0, 2.0], [3.0, 4.0]])
