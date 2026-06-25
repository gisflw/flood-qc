from __future__ import annotations

from datetime import datetime

import numpy as np
import pytest
import xarray as xr

from mgb_ops.model.forecast_grid import (
    read_forecast_precipitation_grid,
    write_forecast_precipitation_grid,
)


def test_forecast_precipitation_grid_writes_canonical_netcdf_and_round_trips(tmp_path) -> None:
    netcdf_path = tmp_path / "forecast.nc"
    times = [datetime(2026, 3, 11, 1, 0, 0), datetime(2026, 3, 11, 2, 0, 0)]
    latitudes = np.array([-29.5, -30.5], dtype=np.float64)
    longitudes = np.array([-52.0, -51.0, -50.0], dtype=np.float64)
    precipitation = np.arange(12, dtype=np.float64).reshape(2, 2, 3)

    write_forecast_precipitation_grid(
        netcdf_path,
        times_utc=times,
        latitudes=latitudes,
        longitudes=longitudes,
        precipitation_mm=precipitation,
        provider_code="ecmwf",
        source_format="GRIB2",
        source_cycle_time=datetime(2026, 3, 11, 0, 0, 0),
    )

    with xr.open_dataset(netcdf_path) as dataset:
        assert set(dataset.dims) == {"time", "latitude", "longitude", "bounds"}
        assert dataset["precipitation"].dims == ("time", "latitude", "longitude")
        assert dataset["time_bounds"].dims == ("time", "bounds")
        assert dataset.attrs["time_zone"] == "UTC"
        assert dataset.attrs["operational_time_zone"] == "America/Sao_Paulo"
        assert dataset.attrs["source_format"] == "GRIB2"
        assert dataset["time"].attrs["standard_name"] == "time"
        assert dataset["time"].attrs["bounds"] == "time_bounds"
        assert dataset["precipitation"].attrs["units"] == "mm"

    grid = read_forecast_precipitation_grid(netcdf_path)

    assert grid.times_utc == tuple(times)
    assert grid.time_bounds_utc[0] == (datetime(2026, 3, 11, 0, 0, 0), datetime(2026, 3, 11, 1, 0, 0))
    assert grid.time_bounds_utc[1] == (datetime(2026, 3, 11, 1, 0, 0), datetime(2026, 3, 11, 2, 0, 0))
    assert np.array_equal(grid.latitudes, latitudes)
    assert np.array_equal(grid.longitudes, longitudes)
    assert np.array_equal(grid.hourly_grids, precipitation)


def test_forecast_precipitation_grid_rejects_missing_precipitation(tmp_path) -> None:
    netcdf_path = tmp_path / "forecast.nc"
    dataset = xr.Dataset(
        data_vars={
            "time_bounds": (
                ("time", "bounds"),
                np.array([["2026-03-11T00:00:00", "2026-03-11T01:00:00"]], dtype="datetime64[ns]"),
            )
        },
        coords={
            "time": np.array(["2026-03-11T01:00:00"], dtype="datetime64[ns]"),
            "latitude": np.array([-29.5]),
            "longitude": np.array([-51.5]),
        },
        attrs={
            "Conventions": "CF-1.8",
            "title": "broken",
            "provider_code": "ecmwf",
            "source_format": "GRIB2",
            "source_cycle_time": "2026-03-11T00:00:00Z",
            "time_zone": "UTC",
            "operational_time_zone": "America/Sao_Paulo",
        },
    )
    dataset["time"].attrs.update({"standard_name": "time", "bounds": "time_bounds"})
    dataset["latitude"].attrs.update({"standard_name": "latitude", "units": "degrees_north", "axis": "Y"})
    dataset["longitude"].attrs.update({"standard_name": "longitude", "units": "degrees_east", "axis": "X"})
    dataset.to_netcdf(netcdf_path, engine="netcdf4")

    with pytest.raises(ValueError, match="missing required variable 'precipitation'"):
        read_forecast_precipitation_grid(netcdf_path)


def test_forecast_precipitation_grid_rejects_non_hourly_bounds(tmp_path) -> None:
    netcdf_path = tmp_path / "forecast.nc"
    write_forecast_precipitation_grid(
        netcdf_path,
        times_utc=[datetime(2026, 3, 11, 1, 0, 0)],
        latitudes=np.array([-29.5]),
        longitudes=np.array([-51.5]),
        precipitation_mm=np.array([[[1.0]]]),
        provider_code="ecmwf",
        source_format="GRIB2",
        source_cycle_time=datetime(2026, 3, 11, 0, 0, 0),
    )
    with xr.open_dataset(netcdf_path) as dataset:
        edited = dataset.load()
    edited["time_bounds"] = (
        ("time", "bounds"),
        np.array([["2026-03-10T23:00:00", "2026-03-11T01:00:00"]], dtype="datetime64[ns]"),
    )
    edited.to_netcdf(netcdf_path, engine="netcdf4")

    with pytest.raises(ValueError, match="intervals must be exactly one hour"):
        read_forecast_precipitation_grid(netcdf_path)
