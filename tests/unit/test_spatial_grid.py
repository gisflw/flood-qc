from datetime import datetime, timezone
import json

import numpy as np
import pytest
import xarray as xr
from netCDF4 import Dataset

from mgb_ops.assets.spatial_grid import NETCDF_ZLIB_COMPLEVEL, read_spatial_grid, write_spatial_grid


def _write(path, **overrides):
    kwargs = {
        "variable": "precipitation",
        "grid_type": "observed",
        "source": "interpolated_from_stations",
        "providers": ["INMET", "ana"],
        "units": "mm",
        "bbox": (-52.5, -31.0, -49.5, -29.0),
        "resolution_degrees": 1.0,
        "times_utc": [datetime(2026, 3, 11, 1, tzinfo=timezone.utc)],
        "latitudes": np.array([-30.5, -29.5]),
        "longitudes": np.array([-52.0, -51.0, -50.0]),
        "values": np.arange(6, dtype=float).reshape(1, 2, 3),
    }
    kwargs.update(overrides)
    return write_spatial_grid(path, **kwargs)


def test_spatial_grid_round_trip_and_metadata(tmp_path):
    path = _write(tmp_path / "grid.nc")
    with xr.open_dataset(path) as dataset:
        assert dataset.attrs["type"] == "observed"
        assert json.loads(dataset.attrs["providers"]) == ["ana", "inmet"]
        assert dataset["precipitation"].attrs["grid_mapping"] == "crs"
        assert dataset["crs"].attrs["epsg_code"] == "EPSG:4326"
    with Dataset(path) as dataset:
        filters = dataset.variables["precipitation"].filters()
        assert filters["zlib"] is True
        assert filters["complevel"] == NETCDF_ZLIB_COMPLEVEL
    grid = read_spatial_grid(path)
    assert grid.providers == ("ana", "inmet")
    assert grid.times_utc[0].tzinfo is timezone.utc
    assert grid.values.shape == (1, 2, 3)


def test_spatial_grid_supports_future_variable(tmp_path):
    grid = read_spatial_grid(
        _write(
            tmp_path / "temperature.nc",
            variable="air_temperature",
            grid_type="forecast",
            source="resampled_from_grid",
            providers=["ecmwf"],
            units="K",
        )
    )
    assert (grid.variable, grid.units) == ("air_temperature", "K")


def test_spatial_grid_rejects_naive_times(tmp_path):
    with pytest.raises(ValueError, match="timezone-aware UTC"):
        _write(tmp_path / "bad.nc", times_utc=[datetime(2026, 3, 11, 1)])
