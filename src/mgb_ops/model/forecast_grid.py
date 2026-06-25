from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Iterable

import numpy as np
import pandas as pd
import xarray as xr


FORECAST_PRECIPITATION_GRID_ASSET_KIND = "forecast_precipitation_grid"
FORECAST_GRID_FORMAT = "NetCDF"
FORECAST_TIME_ZONE = "UTC"
OPERATIONAL_TIME_ZONE = "America/Sao_Paulo"
NETCDF_ZLIB_COMPLEVEL = 4


@dataclass(frozen=True, slots=True)
class ForecastPrecipitationGrid:
    latitudes: np.ndarray
    longitudes: np.ndarray
    times_utc: tuple[datetime, ...]
    time_bounds_utc: tuple[tuple[datetime, datetime], ...]
    hourly_grids: np.ndarray


def _utc_datetime_index(values: Iterable[datetime | np.datetime64 | pd.Timestamp], *, name: str) -> pd.DatetimeIndex:
    raw_values = list(values)
    if not raw_values:
        raise ValueError(f"{name} must contain at least one timestamp.")
    timestamps = pd.to_datetime(raw_values, utc=True, errors="coerce")
    if pd.isna(timestamps).any():
        raise ValueError(f"{name} contains an invalid timestamp.")
    return pd.DatetimeIndex(timestamps).tz_convert(FORECAST_TIME_ZONE).tz_localize(None)


def _naive_utc_datetimes(values: Iterable[datetime | np.datetime64 | pd.Timestamp], *, name: str) -> tuple[datetime, ...]:
    return tuple(ts.to_pydatetime().replace(tzinfo=None) for ts in _utc_datetime_index(values, name=name))


def _require_hourly_sequence(times: pd.DatetimeIndex, *, name: str) -> None:
    if len(times) < 2:
        return
    deltas = np.diff(times.values.astype("datetime64[ns]"))
    if not np.all(deltas == np.timedelta64(1, "h")):
        raise ValueError(f"{name} must be a contiguous hourly UTC sequence.")


def write_forecast_precipitation_grid(
    netcdf_path: Path,
    *,
    times_utc: Iterable[datetime | np.datetime64 | pd.Timestamp],
    latitudes: np.ndarray,
    longitudes: np.ndarray,
    precipitation_mm: np.ndarray,
    provider_code: str,
    source_format: str,
    source_cycle_time: datetime | str,
    title: str = "Canonical hourly forecast precipitation grid",
) -> Path:
    time_index = _utc_datetime_index(times_utc, name="times_utc")
    _require_hourly_sequence(time_index, name="times_utc")

    latitude_values = np.asarray(latitudes, dtype=np.float64)
    longitude_values = np.asarray(longitudes, dtype=np.float64)
    precipitation_values = np.asarray(precipitation_mm, dtype=np.float64)
    if latitude_values.ndim != 1:
        raise ValueError("latitudes must be a 1-D coordinate.")
    if longitude_values.ndim != 1:
        raise ValueError("longitudes must be a 1-D coordinate.")
    expected_shape = (len(time_index), latitude_values.size, longitude_values.size)
    if precipitation_values.shape != expected_shape:
        raise ValueError(
            f"precipitation_mm shape mismatch: expected {expected_shape}, found {precipitation_values.shape}."
        )

    end_times = time_index.values.astype("datetime64[ns]")
    start_times = end_times - np.timedelta64(1, "h")
    time_bounds = np.stack([start_times, end_times], axis=1)

    source_cycle_text = (
        _naive_utc_datetimes([source_cycle_time], name="source_cycle_time")[0].isoformat(timespec="seconds") + "Z"
        if not isinstance(source_cycle_time, str)
        else source_cycle_time
    )

    dataset = xr.Dataset(
        data_vars={
            "precipitation": (
                ("time", "latitude", "longitude"),
                precipitation_values,
                {
                    "long_name": "Hourly precipitation amount",
                    "units": "mm",
                },
            ),
            "time_bounds": (("time", "bounds"), time_bounds),
        },
        coords={
            "time": (
                ("time",),
                end_times,
                {
                    "standard_name": "time",
                    "bounds": "time_bounds",
                },
            ),
            "latitude": (
                ("latitude",),
                latitude_values,
                {
                    "standard_name": "latitude",
                    "units": "degrees_north",
                    "axis": "Y",
                },
            ),
            "longitude": (
                ("longitude",),
                longitude_values,
                {
                    "standard_name": "longitude",
                    "units": "degrees_east",
                    "axis": "X",
                },
            ),
        },
        attrs={
            "Conventions": "CF-1.8",
            "title": title,
            "provider_code": provider_code,
            "source_format": source_format,
            "source_cycle_time": source_cycle_text,
            "time_zone": FORECAST_TIME_ZONE,
            "operational_time_zone": OPERATIONAL_TIME_ZONE,
        },
    )

    encoding = {
        "precipitation": {"zlib": True, "complevel": NETCDF_ZLIB_COMPLEVEL},
        "time": {"units": "hours since 1970-01-01 00:00:00", "calendar": "proleptic_gregorian"},
        "time_bounds": {
            "units": "hours since 1970-01-01 00:00:00",
            "calendar": "proleptic_gregorian",
            "zlib": True,
            "complevel": NETCDF_ZLIB_COMPLEVEL,
        },
    }
    netcdf_path.parent.mkdir(parents=True, exist_ok=True)
    dataset.to_netcdf(netcdf_path, engine="netcdf4", encoding=encoding)
    return netcdf_path


def _require_dataset_contract(dataset: xr.Dataset, *, source_path: Path) -> None:
    for dimension in ("time", "latitude", "longitude", "bounds"):
        if dimension not in dataset.dims:
            raise ValueError(f"Forecast NetCDF missing required dimension {dimension!r}: {source_path}")
    for variable in ("precipitation", "time_bounds", "time", "latitude", "longitude"):
        if variable not in dataset:
            raise ValueError(f"Forecast NetCDF missing required variable {variable!r}: {source_path}")

    if dataset["latitude"].dims != ("latitude",):
        raise ValueError("Forecast NetCDF latitude coordinate must be 1-D.")
    if dataset["longitude"].dims != ("longitude",):
        raise ValueError("Forecast NetCDF longitude coordinate must be 1-D.")
    if dataset["precipitation"].dims != ("time", "latitude", "longitude"):
        raise ValueError("Forecast NetCDF precipitation must use dimensions (time, latitude, longitude).")
    if dataset["time_bounds"].dims != ("time", "bounds") or int(dataset.sizes["bounds"]) != 2:
        raise ValueError("Forecast NetCDF time_bounds must use dimensions (time, bounds) with bounds length 2.")

    required_attrs = {
        "Conventions",
        "title",
        "provider_code",
        "source_format",
        "source_cycle_time",
        "time_zone",
        "operational_time_zone",
    }
    missing_attrs = sorted(required_attrs.difference(dataset.attrs))
    if missing_attrs:
        raise ValueError(f"Forecast NetCDF missing required global attrs: {missing_attrs}.")
    if dataset.attrs.get("time_zone") != FORECAST_TIME_ZONE:
        raise ValueError("Forecast NetCDF time_zone must be 'UTC'.")
    if dataset.attrs.get("operational_time_zone") != OPERATIONAL_TIME_ZONE:
        raise ValueError("Forecast NetCDF operational_time_zone must be 'America/Sao_Paulo'.")

    time_attrs = dataset["time"].attrs
    if time_attrs.get("standard_name") != "time" or time_attrs.get("bounds") != "time_bounds":
        raise ValueError("Forecast NetCDF time coordinate must declare standard_name='time' and bounds='time_bounds'.")
    if dataset["latitude"].attrs.get("standard_name") != "latitude":
        raise ValueError("Forecast NetCDF latitude coordinate must declare standard_name='latitude'.")
    if dataset["latitude"].attrs.get("units") != "degrees_north":
        raise ValueError("Forecast NetCDF latitude units must be 'degrees_north'.")
    if dataset["longitude"].attrs.get("standard_name") != "longitude":
        raise ValueError("Forecast NetCDF longitude coordinate must declare standard_name='longitude'.")
    if dataset["longitude"].attrs.get("units") != "degrees_east":
        raise ValueError("Forecast NetCDF longitude units must be 'degrees_east'.")
    if dataset["precipitation"].attrs.get("units") != "mm":
        raise ValueError("Forecast NetCDF precipitation units must be 'mm'.")


def read_forecast_precipitation_grid(netcdf_path: Path) -> ForecastPrecipitationGrid:
    with xr.open_dataset(netcdf_path, decode_times=True) as dataset:
        dataset = dataset.load()
    _require_dataset_contract(dataset, source_path=Path(netcdf_path))

    times_utc = _naive_utc_datetimes(dataset["time"].values, name="time")
    bounds_flat = _naive_utc_datetimes(dataset["time_bounds"].values.reshape(-1), name="time_bounds")
    time_bounds_utc = tuple(
        (bounds_flat[idx], bounds_flat[idx + 1])
        for idx in range(0, len(bounds_flat), 2)
    )
    for idx, (start_time, end_time) in enumerate(time_bounds_utc):
        if end_time != times_utc[idx]:
            raise ValueError("Forecast NetCDF time_bounds end must equal the corresponding time value.")
        if end_time - start_time != timedelta(hours=1):
            raise ValueError("Forecast NetCDF time_bounds intervals must be exactly one hour.")

    _require_hourly_sequence(pd.DatetimeIndex(times_utc), name="time")
    return ForecastPrecipitationGrid(
        latitudes=np.asarray(dataset["latitude"].values, dtype=np.float64),
        longitudes=np.asarray(dataset["longitude"].values, dtype=np.float64),
        times_utc=times_utc,
        time_bounds_utc=time_bounds_utc,
        hourly_grids=np.asarray(dataset["precipitation"].values, dtype=np.float64),
    )


def load_forecast_precipitation_grid(
    netcdf_path: Path,
    *,
    forecast_start_time_utc: datetime,
    forecast_nt: int,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    if forecast_nt < 0:
        raise ValueError("forecast_nt must be >= 0.")
    grid = read_forecast_precipitation_grid(netcdf_path)
    if forecast_nt == 0:
        return grid.latitudes, grid.longitudes, grid.hourly_grids[:0]

    start_utc = _naive_utc_datetimes([forecast_start_time_utc], name="forecast_start_time_utc")[0]
    required_times = tuple(start_utc + timedelta(hours=offset) for offset in range(forecast_nt))
    index_by_time = {valid_time: idx for idx, valid_time in enumerate(grid.times_utc)}
    missing_times = [valid_time for valid_time in required_times if valid_time not in index_by_time]
    if missing_times:
        raise ValueError(
            "Forecast NetCDF does not cover the full requested UTC forecast window. "
            f"First missing hour: {missing_times[0].isoformat(timespec='seconds')}"
        )

    selected_indices = [index_by_time[valid_time] for valid_time in required_times]
    return grid.latitudes, grid.longitudes, grid.hourly_grids[selected_indices, :, :]
