from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
import json
import os
from pathlib import Path
import tempfile
from typing import Iterable, Mapping

import numpy as np
import pandas as pd
import xarray as xr

from mgb_ops.utils.time import validate_timestep_hours


SPATIAL_GRID_ASSET_KIND = "spatial_grid"
SPATIAL_GRID_FORMAT = "NetCDF"
SPATIAL_GRID_SCHEMA_VERSION = "2.0"
SPATIAL_GRID_TIME_ZONE = "UTC"
SPATIAL_GRID_CRS = "EPSG:4326"
NETCDF_ZLIB_COMPLEVEL = 4
ALLOWED_GRID_TYPES = {"observed", "forecast"}
ALLOWED_GRID_SOURCES = {
    "interpolated_from_stations",
    "resampled_from_grid",
    "cropped_from_native_grid",
}


@dataclass(frozen=True, slots=True)
class SpatialGrid:
    variable: str
    grid_type: str
    source: str
    providers: tuple[str, ...]
    units: str
    latitudes: np.ndarray
    longitudes: np.ndarray
    times_utc: tuple[datetime, ...]
    time_bounds_utc: tuple[tuple[datetime, datetime], ...]
    values: np.ndarray
    timestep_hours: int | None
    metadata: dict[str, object]


def _coordinate_centers(lower: float, upper: float, resolution: float) -> np.ndarray:
    coordinates = np.arange(lower + resolution / 2.0, upper, resolution, dtype=float)
    if coordinates.size == 0:
        return np.array([(lower + upper) / 2.0], dtype=float)
    return coordinates


def _inclusive_touch_centers(lower: float, upper: float, resolution: float) -> np.ndarray:
    """Return cells whose closed footprints intersect [lower, upper]."""
    cell_count = int(np.floor((upper - lower) / resolution)) + 2
    return lower - resolution / 2.0 + np.arange(cell_count, dtype=float) * resolution


@dataclass(frozen=True, slots=True, init=False)
class RegularGridSpec:
    bbox: tuple[float, float, float, float]
    resolution: float
    include_boundary_cells: bool

    def __init__(
        self,
        bbox: tuple[float, float, float, float],
        resolution: float | None = None,
        *,
        resolution_degrees: float | None = None,
        include_boundary_cells: bool = False,
    ) -> None:
        if resolution is None:
            resolution = resolution_degrees
        elif resolution_degrees is not None and float(resolution) != float(resolution_degrees):
            raise ValueError("resolution and resolution_degrees must match when both are provided.")
        if resolution is None:
            raise TypeError("resolution is required.")
        west, south, east, north = (float(value) for value in bbox)
        if west >= east or south >= north:
            raise ValueError("bbox must satisfy west < east and south < north.")
        if not np.isfinite(resolution) or resolution <= 0:
            raise ValueError("resolution must be > 0.")
        object.__setattr__(self, "bbox", (west, south, east, north))
        object.__setattr__(self, "resolution", float(resolution))
        object.__setattr__(self, "include_boundary_cells", bool(include_boundary_cells))

    @property
    def resolution_degrees(self) -> float:
        return self.resolution

    @property
    def longitudes(self) -> np.ndarray:
        west, _, east, _ = self.bbox
        if self.include_boundary_cells:
            return _inclusive_touch_centers(west, east, self.resolution)
        return _coordinate_centers(west, east, self.resolution)

    @property
    def latitudes(self) -> np.ndarray:
        _, south, _, north = self.bbox
        if self.include_boundary_cells:
            return _inclusive_touch_centers(south, north, self.resolution)
        return _coordinate_centers(south, north, self.resolution)

    @property
    def effective_bbox(self) -> tuple[float, float, float, float]:
        if not self.include_boundary_cells:
            return self.bbox
        return (
            float(self.longitudes[0] - self.resolution / 2),
            float(self.latitudes[0] - self.resolution / 2),
            float(self.longitudes[-1] + self.resolution / 2),
            float(self.latitudes[-1] + self.resolution / 2),
        )

    @property
    def shape(self) -> tuple[int, int]:
        return (len(self.latitudes), len(self.longitudes))


@dataclass(frozen=True, slots=True)
class PrecipitationGrid:
    values: np.ndarray
    latitudes: np.ndarray
    longitudes: np.ndarray
    bounds: tuple[float, float, float, float]
    start_time: datetime | pd.Timestamp
    end_time: datetime | pd.Timestamp
    units: str = "mm"
    source: str = ""

    def __post_init__(self) -> None:
        values = np.asarray(self.values, dtype=float)
        latitudes = np.asarray(self.latitudes, dtype=float)
        longitudes = np.asarray(self.longitudes, dtype=float)
        if latitudes.ndim != 1 or longitudes.ndim != 1:
            raise ValueError("latitudes and longitudes must be 1-D.")
        if values.shape != (latitudes.size, longitudes.size):
            raise ValueError(
                f"values shape must be {(latitudes.size, longitudes.size)}, found {values.shape}."
            )
        if pd.Timestamp(self.end_time) <= pd.Timestamp(self.start_time):
            raise ValueError("end_time must be after start_time.")
        object.__setattr__(self, "values", values)
        object.__setattr__(self, "latitudes", latitudes)
        object.__setattr__(self, "longitudes", longitudes)

    @property
    def time_window(self) -> tuple[pd.Timestamp, pd.Timestamp]:
        return pd.Timestamp(self.start_time), pd.Timestamp(self.end_time)


def normalize_providers(providers: Iterable[str]) -> tuple[str, ...]:
    normalized = tuple(sorted({str(value).strip().lower() for value in providers if str(value).strip()}))
    if not normalized:
        raise ValueError("providers must contain at least one non-empty provider code.")
    return normalized


def _validate_identity(variable: str, grid_type: str, source: str) -> tuple[str, str, str]:
    variable = str(variable).strip()
    grid_type = str(grid_type).strip().lower()
    source = str(source).strip().lower()
    if not variable:
        raise ValueError("variable must be non-empty.")
    if grid_type not in ALLOWED_GRID_TYPES:
        raise ValueError(f"type must be one of {sorted(ALLOWED_GRID_TYPES)}.")
    if source not in ALLOWED_GRID_SOURCES:
        raise ValueError(f"source must be one of {sorted(ALLOWED_GRID_SOURCES)}.")
    return variable, grid_type, source


def _utc_index(
    values: Iterable[datetime | np.datetime64 | pd.Timestamp],
    *,
    name: str,
    require_aware: bool,
) -> pd.DatetimeIndex:
    raw = list(values)
    if not raw:
        raise ValueError(f"{name} must contain at least one timestamp.")
    if require_aware:
        for value in raw:
            timestamp = pd.Timestamp(value)
            if timestamp.tzinfo is None or timestamp.utcoffset() != timedelta(0):
                raise ValueError(f"{name} values must be timezone-aware UTC timestamps.")
    timestamps = pd.to_datetime(raw, utc=True, errors="coerce")
    if pd.isna(timestamps).any():
        raise ValueError(f"{name} contains an invalid timestamp.")
    return pd.DatetimeIndex(timestamps).tz_convert("UTC").tz_localize(None)


def _require_contiguous(times: pd.DatetimeIndex, timestep_hours: int) -> None:
    if len(times) > 1:
        expected = np.timedelta64(timestep_hours, "h")
        if not np.all(np.diff(times.values.astype("datetime64[ns]")) == expected):
            raise ValueError(f"times_utc must be a contiguous {timestep_hours}-hour UTC sequence.")


def _validate_coordinates(values: np.ndarray, *, name: str, resolution: float) -> None:
    if values.size < 1:
        raise ValueError(f"{name} must contain at least one coordinate.")
    if not np.all(np.isfinite(values)):
        raise ValueError(f"{name} must be finite.")
    if values.size == 1:
        return
    differences = np.diff(values)
    if not np.all(np.isfinite(differences)) or not np.all(differences > 0):
        raise ValueError(f"{name} must be finite and strictly increasing.")
    if not np.allclose(differences, resolution):
        raise ValueError(f"{name} must be regularly spaced.")


def write_spatial_grid(
    netcdf_path: Path,
    *,
    variable: str,
    grid_type: str,
    source: str,
    providers: Iterable[str],
    units: str,
    bbox: tuple[float, float, float, float],
    resolution_degrees: float,
    times_utc: Iterable[datetime | np.datetime64 | pd.Timestamp],
    latitudes: np.ndarray,
    longitudes: np.ndarray,
    values: np.ndarray,
    timestep_hours: int | None = 1,
    time_bounds_utc: Iterable[
        tuple[datetime | np.datetime64 | pd.Timestamp, datetime | np.datetime64 | pd.Timestamp]
    ] | None = None,
    title: str | None = None,
    processing_metadata: Mapping[str, object] | None = None,
) -> Path:
    variable, grid_type, source = _validate_identity(variable, grid_type, source)
    provider_codes = normalize_providers(providers)
    units = str(units).strip()
    if not units:
        raise ValueError("units must be non-empty.")
    time_index = _utc_index(times_utc, name="times_utc", require_aware=True)
    if timestep_hours is not None:
        timestep_hours = validate_timestep_hours(timestep_hours)
        _require_contiguous(time_index, timestep_hours)

    latitude_values = np.asarray(latitudes, dtype=np.float64)
    longitude_values = np.asarray(longitudes, dtype=np.float64)
    payload = np.asarray(values, dtype=np.float64)
    if latitude_values.ndim != 1 or longitude_values.ndim != 1:
        raise ValueError("latitudes and longitudes must be 1-D coordinates.")
    resolution_degrees = float(resolution_degrees)
    if not np.isfinite(resolution_degrees) or resolution_degrees <= 0:
        raise ValueError("resolution_degrees must be > 0.")
    west, south, east, north = (float(value) for value in bbox)
    if west >= east or south >= north:
        raise ValueError("bbox must satisfy west < east and south < north.")
    _validate_coordinates(latitude_values, name="latitudes", resolution=resolution_degrees)
    _validate_coordinates(longitude_values, name="longitudes", resolution=resolution_degrees)
    expected_latitudes = np.arange(
        south + resolution_degrees / 2, north, resolution_degrees, dtype=float
    )
    expected_longitudes = np.arange(
        west + resolution_degrees / 2, east, resolution_degrees, dtype=float
    )
    if expected_latitudes.size == 0:
        expected_latitudes = np.array([(south + north) / 2])
    if expected_longitudes.size == 0:
        expected_longitudes = np.array([(west + east) / 2])
    if not np.allclose(latitude_values, expected_latitudes) or not np.allclose(
        longitude_values, expected_longitudes
    ):
        raise ValueError("latitude and longitude coordinates must be centers of bbox cells.")
    expected_shape = (len(time_index), latitude_values.size, longitude_values.size)
    if payload.shape != expected_shape:
        raise ValueError(f"values shape mismatch: expected {expected_shape}, found {payload.shape}.")

    end_times = time_index.values.astype("datetime64[ns]")
    if time_bounds_utc is None:
        if timestep_hours is None:
            raise ValueError("time_bounds_utc is required when timestep_hours is None.")
        start_times = end_times - np.timedelta64(timestep_hours, "h")
        time_bounds = np.stack([start_times, end_times], axis=1)
    else:
        raw_bounds = list(time_bounds_utc)
        if len(raw_bounds) != len(time_index):
            raise ValueError("time_bounds_utc length must match times_utc.")
        flat_bounds = _utc_index(
            [value for pair in raw_bounds for value in pair],
            name="time_bounds_utc",
            require_aware=True,
        ).values.astype("datetime64[ns]")
        time_bounds = flat_bounds.reshape(-1, 2)
        if not np.array_equal(time_bounds[:, 1], end_times):
            raise ValueError("Each time bound must end at its corresponding time.")
        if np.any(time_bounds[:, 0] >= time_bounds[:, 1]):
            raise ValueError("Each time bound must have positive duration.")
        if len(time_bounds) > 1 and not np.array_equal(time_bounds[:-1, 1], time_bounds[1:, 0]):
            raise ValueError("time_bounds_utc must be contiguous.")
        start_times = time_bounds[:, 0]
    created_at = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")

    attrs: dict[str, object] = {
        "Conventions": "CF-1.8, ACDD-1.3",
        "title": title or f"{variable} {grid_type} spatial grid",
        "spatial_grid_schema_version": SPATIAL_GRID_SCHEMA_VERSION,
        "variable": variable,
        "type": grid_type,
        "source": source,
        "providers": json.dumps(provider_codes),
        "time_zone": SPATIAL_GRID_TIME_ZONE,
        "crs": SPATIAL_GRID_CRS,
        "created_at": created_at,
        "valid_from": pd.Timestamp(start_times[0]).isoformat() + "Z",
        "valid_to": pd.Timestamp(end_times[-1]).isoformat() + "Z",
        "temporal_resolution": (
            f"PT{timestep_hours}H" if timestep_hours is not None else "irregular"
        ),
        "bbox": json.dumps([west, south, east, north]),
        "resolution_degrees": resolution_degrees,
    }
    if timestep_hours is not None:
        attrs["timestep_hours"] = timestep_hours
    for key, value in dict(processing_metadata or {}).items():
        if key in attrs:
            raise ValueError(f"processing_metadata cannot override required attribute {key!r}.")
        attrs[str(key)] = value if isinstance(value, (str, int, float, np.number)) else json.dumps(value)

    dataset = xr.Dataset(
        data_vars={
            variable: (
                ("time", "latitude", "longitude"),
                payload,
                {"long_name": variable.replace("_", " "), "units": units, "grid_mapping": "crs"},
            ),
            "time_bounds": (("time", "bounds"), time_bounds),
            "latitude_bounds": (
                ("latitude", "bounds"),
                np.column_stack([
                    latitude_values - resolution_degrees / 2,
                    latitude_values + resolution_degrees / 2,
                ]),
            ),
            "longitude_bounds": (
                ("longitude", "bounds"),
                np.column_stack([
                    longitude_values - resolution_degrees / 2,
                    longitude_values + resolution_degrees / 2,
                ]),
            ),
            "crs": (
                (),
                np.int32(0),
                {
                    "grid_mapping_name": "latitude_longitude",
                    "epsg_code": SPATIAL_GRID_CRS,
                    "semi_major_axis": 6378137.0,
                    "inverse_flattening": 298.257223563,
                },
            ),
        },
        coords={
            "time": (("time",), end_times, {"standard_name": "time", "bounds": "time_bounds"}),
            "latitude": (
                ("latitude",),
                latitude_values,
                {"standard_name": "latitude", "units": "degrees_north", "axis": "Y", "bounds": "latitude_bounds"},
            ),
            "longitude": (
                ("longitude",),
                longitude_values,
                {"standard_name": "longitude", "units": "degrees_east", "axis": "X", "bounds": "longitude_bounds"},
            ),
        },
        attrs=attrs,
    )
    encoding = {
        variable: {"zlib": True, "complevel": NETCDF_ZLIB_COMPLEVEL},
        "time": {"units": "hours since 1970-01-01 00:00:00", "calendar": "proleptic_gregorian"},
        "time_bounds": {
            "units": "hours since 1970-01-01 00:00:00",
            "calendar": "proleptic_gregorian",
            "zlib": True,
            "complevel": NETCDF_ZLIB_COMPLEVEL,
        },
        "latitude_bounds": {"zlib": True, "complevel": NETCDF_ZLIB_COMPLEVEL},
        "longitude_bounds": {"zlib": True, "complevel": NETCDF_ZLIB_COMPLEVEL},
    }
    target = Path(netcdf_path)
    target.parent.mkdir(parents=True, exist_ok=True)
    file_descriptor, temporary_name = tempfile.mkstemp(
        prefix=f".{target.stem}.", suffix=".tmp.nc", dir=target.parent
    )
    os.close(file_descriptor)
    temporary_path = Path(temporary_name)
    try:
        dataset.to_netcdf(temporary_path, engine="netcdf4", encoding=encoding)
        os.replace(temporary_path, target)
    finally:
        temporary_path.unlink(missing_ok=True)
    return target


def _require_contract(dataset: xr.Dataset, source_path: Path) -> tuple[str, str, str, tuple[str, ...]]:
    required_attrs = {
        "Conventions", "title", "spatial_grid_schema_version", "variable", "type", "source",
        "providers", "time_zone", "crs", "created_at", "valid_from", "valid_to",
        "bbox", "resolution_degrees", "temporal_resolution",
    }
    missing = sorted(required_attrs.difference(dataset.attrs))
    if missing:
        raise ValueError(f"Spatial-grid NetCDF missing required global attrs: {missing}: {source_path}")
    variable, grid_type, source = _validate_identity(
        dataset.attrs["variable"], dataset.attrs["type"], dataset.attrs["source"]
    )
    try:
        provider_payload = json.loads(dataset.attrs["providers"])
        if not isinstance(provider_payload, list):
            raise ValueError
        providers = normalize_providers(provider_payload)
    except (TypeError, ValueError, json.JSONDecodeError) as exc:
        raise ValueError("Spatial-grid NetCDF providers must be a JSON list of provider codes.") from exc
    for dimension in ("time", "latitude", "longitude", "bounds"):
        if dimension not in dataset.dims:
            raise ValueError(f"Spatial-grid NetCDF missing required dimension {dimension!r}: {source_path}")
    for name in (
        variable, "time_bounds", "latitude_bounds", "longitude_bounds",
        "time", "latitude", "longitude", "crs",
    ):
        if name not in dataset:
            raise ValueError(f"Spatial-grid NetCDF missing required variable {name!r}: {source_path}")
    if dataset[variable].dims != ("time", "latitude", "longitude"):
        raise ValueError(f"Spatial-grid variable {variable!r} must use (time, latitude, longitude).")
    if dataset["time_bounds"].dims != ("time", "bounds") or dataset.sizes["bounds"] != 2:
        raise ValueError("Spatial-grid time_bounds must use (time, bounds) with bounds length 2.")
    if dataset["latitude_bounds"].dims != ("latitude", "bounds") or dataset[
        "longitude_bounds"
    ].dims != ("longitude", "bounds"):
        raise ValueError("Spatial-grid coordinate bounds have invalid dimensions.")
    if dataset.attrs["time_zone"] != "UTC" or dataset.attrs["crs"] != SPATIAL_GRID_CRS:
        raise ValueError("Spatial-grid NetCDF must use UTC and EPSG:4326.")
    if dataset.attrs["spatial_grid_schema_version"] != SPATIAL_GRID_SCHEMA_VERSION:
        raise ValueError("Unsupported spatial-grid schema version.")
    if dataset["time"].attrs.get("standard_name") != "time" or dataset["time"].attrs.get(
        "bounds"
    ) != "time_bounds":
        raise ValueError("Spatial-grid time coordinate must declare its standard name and bounds.")
    if (
        dataset["latitude"].attrs.get("standard_name") != "latitude"
        or dataset["latitude"].attrs.get("units") != "degrees_north"
        or dataset["latitude"].attrs.get("bounds") != "latitude_bounds"
        or dataset["longitude"].attrs.get("standard_name") != "longitude"
        or dataset["longitude"].attrs.get("units") != "degrees_east"
        or dataset["longitude"].attrs.get("bounds") != "longitude_bounds"
    ):
        raise ValueError("Spatial-grid geographic coordinate metadata is invalid.")
    if dataset[variable].attrs.get("grid_mapping") != "crs":
        raise ValueError("Spatial-grid payload must reference the crs grid mapping.")
    if not str(dataset[variable].attrs.get("units", "")).strip():
        raise ValueError("Spatial-grid payload units must be non-empty.")
    try:
        bbox = json.loads(dataset.attrs["bbox"])
        if not isinstance(bbox, list) or len(bbox) != 4:
            raise ValueError
        resolution = float(dataset.attrs["resolution_degrees"])
        latitudes = np.asarray(dataset["latitude"].values, dtype=float)
        longitudes = np.asarray(dataset["longitude"].values, dtype=float)
        west, south, east, north = (float(value) for value in bbox)
        _validate_coordinates(latitudes, name="latitudes", resolution=resolution)
        _validate_coordinates(longitudes, name="longitudes", resolution=resolution)
        expected_latitudes = np.arange(south + resolution / 2, north, resolution)
        expected_longitudes = np.arange(west + resolution / 2, east, resolution)
        if expected_latitudes.size == 0:
            expected_latitudes = np.array([(south + north) / 2])
        if expected_longitudes.size == 0:
            expected_longitudes = np.array([(west + east) / 2])
        if not np.allclose(latitudes, expected_latitudes) or not np.allclose(
            longitudes, expected_longitudes
        ):
            raise ValueError
        expected_latitude_bounds = np.column_stack([
            latitudes - resolution / 2, latitudes + resolution / 2
        ])
        expected_longitude_bounds = np.column_stack([
            longitudes - resolution / 2, longitudes + resolution / 2
        ])
        if not np.allclose(dataset["latitude_bounds"].values, expected_latitude_bounds):
            raise ValueError
        if not np.allclose(dataset["longitude_bounds"].values, expected_longitude_bounds):
            raise ValueError
    except (TypeError, ValueError, json.JSONDecodeError) as exc:
        raise ValueError("Spatial-grid bbox, resolution, and coordinates are inconsistent.") from exc
    return variable, grid_type, source, providers


def read_spatial_grid(netcdf_path: Path) -> SpatialGrid:
    path = Path(netcdf_path)
    with xr.open_dataset(path, decode_times=True) as dataset:
        dataset = dataset.load()
    variable, grid_type, source, providers = _require_contract(dataset, path)
    raw_timestep = dataset.attrs.get("timestep_hours")
    timestep_hours = (
        validate_timestep_hours(int(raw_timestep)) if raw_timestep is not None else None
    )
    time_index = _utc_index(dataset["time"].values, name="time", require_aware=False)
    if timestep_hours is not None:
        _require_contiguous(time_index, timestep_hours)
    bounds_index = _utc_index(
        dataset["time_bounds"].values.reshape(-1), name="time_bounds", require_aware=False
    )
    times = tuple(value.to_pydatetime().replace(tzinfo=timezone.utc) for value in time_index)
    flat_bounds = tuple(value.to_pydatetime().replace(tzinfo=timezone.utc) for value in bounds_index)
    bounds = tuple((flat_bounds[index], flat_bounds[index + 1]) for index in range(0, len(flat_bounds), 2))
    for timestamp, (start, end) in zip(times, bounds, strict=True):
        if end != timestamp or end <= start:
            raise ValueError("Spatial-grid time bounds must end at time and have positive duration.")
        if timestep_hours is not None and end - start != timedelta(hours=timestep_hours):
            raise ValueError("Spatial-grid time bounds must match timestep_hours.")
    if any(left[1] != right[0] for left, right in zip(bounds, bounds[1:])):
        raise ValueError("Spatial-grid time bounds must be contiguous.")
    valid_from = pd.Timestamp(dataset.attrs["valid_from"])
    valid_to = pd.Timestamp(dataset.attrs["valid_to"])
    if valid_from.tzinfo is None or valid_to.tzinfo is None:
        raise ValueError("Spatial-grid valid interval metadata must be timezone-aware UTC.")
    if valid_from.tz_convert("UTC") != pd.Timestamp(bounds[0][0]) or valid_to.tz_convert(
        "UTC"
    ) != pd.Timestamp(bounds[-1][1]):
        raise ValueError("Spatial-grid valid interval metadata must match time_bounds.")
    return SpatialGrid(
        variable=variable,
        grid_type=grid_type,
        source=source,
        providers=providers,
        units=str(dataset[variable].attrs.get("units", "")),
        latitudes=np.asarray(dataset["latitude"].values, dtype=np.float64),
        longitudes=np.asarray(dataset["longitude"].values, dtype=np.float64),
        times_utc=times,
        time_bounds_utc=bounds,
        values=np.asarray(dataset[variable].values, dtype=np.float64),
        timestep_hours=timestep_hours,
        metadata=dict(dataset.attrs),
    )
