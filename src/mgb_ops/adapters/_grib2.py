from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

import numpy as np


@dataclass(frozen=True, slots=True)
class TpGribMessage:
    valid_time: datetime
    step_hours: int
    latitudes: np.ndarray
    longitudes: np.ndarray
    values_mm: np.ndarray


def require_eccodes():
    try:
        import eccodes
    except ModuleNotFoundError as exc:
        raise ModuleNotFoundError(
            "Missing dependency for GRIB2 reading/writing: install `eccodes` in the operational environment."
        ) from exc
    return eccodes


def normalize_longitudes(values: np.ndarray) -> np.ndarray:
    normalized = np.asarray(values, dtype=np.float64).copy()
    normalized[normalized > 180.0] -= 360.0
    return normalized


def build_grid_arrays(gid) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    eccodes = require_eccodes()
    grid_type = eccodes.codes_get(gid, "gridType")
    if grid_type != "regular_ll":
        raise ValueError(f"Unsupported GRIB gridType={grid_type!r}; expected 'regular_ll'.")

    ni = int(eccodes.codes_get_long(gid, "Ni"))
    nj = int(eccodes.codes_get_long(gid, "Nj"))
    if ni < 1 or nj < 1:
        raise ValueError(f"Invalid GRIB shape Ni={ni} Nj={nj}.")

    values = np.asarray(eccodes.codes_get_array(gid, "values"), dtype=np.float64).reshape(nj, ni)
    latitude_grid = np.asarray(eccodes.codes_get_array(gid, "latitudes"), dtype=np.float64).reshape(nj, ni)
    longitude_grid = normalize_longitudes(
        np.asarray(eccodes.codes_get_array(gid, "longitudes"), dtype=np.float64)
    ).reshape(nj, ni)

    latitudes = latitude_grid[:, 0].copy()
    longitudes = longitude_grid[0, :].copy()

    if not np.allclose(latitude_grid, latitudes[:, None]):
        raise ValueError("Unexpected latitude layout in GRIB regular_ll grid.")
    if not np.allclose(longitude_grid, longitudes[None, :]):
        raise ValueError("Unexpected longitude layout in GRIB regular_ll grid.")

    lon_sort_idx = np.argsort(longitudes)
    longitudes = longitudes[lon_sort_idx]
    values = values[:, lon_sort_idx]

    return latitudes, longitudes, values


def set_regular_ll_grid(gid, *, latitudes: np.ndarray, longitudes: np.ndarray, values: np.ndarray) -> None:
    eccodes = require_eccodes()
    if latitudes.size < 1 or longitudes.size < 1:
        raise ValueError("GRIB grid cannot be empty.")

    grid_values = np.asarray(values, dtype=np.float64)
    lat_vec = np.asarray(latitudes, dtype=np.float64)
    lon_vec = np.asarray(longitudes, dtype=np.float64)

    if lat_vec[0] < lat_vec[-1]:
        lat_vec = lat_vec[::-1]
        grid_values = grid_values[::-1, :]

    eccodes.codes_set_long(gid, "Ni", int(lon_vec.size))
    eccodes.codes_set_long(gid, "Nj", int(lat_vec.size))
    eccodes.codes_set(gid, "latitudeOfFirstGridPointInDegrees", float(lat_vec[0]))
    eccodes.codes_set(gid, "latitudeOfLastGridPointInDegrees", float(lat_vec[-1]))
    eccodes.codes_set(gid, "longitudeOfFirstGridPointInDegrees", float(lon_vec[0]))
    eccodes.codes_set(gid, "longitudeOfLastGridPointInDegrees", float(lon_vec[-1]))
    if lon_vec.size > 1:
        eccodes.codes_set(gid, "iDirectionIncrementInDegrees", float(abs(lon_vec[1] - lon_vec[0])))
    if lat_vec.size > 1:
        eccodes.codes_set(gid, "jDirectionIncrementInDegrees", float(abs(lat_vec[1] - lat_vec[0])))
    eccodes.codes_set_long(gid, "iScansNegatively", 0)
    eccodes.codes_set_long(gid, "jScansPositively", 0)
    eccodes.codes_set_array(gid, "values", grid_values.reshape(-1))


def read_tp_grib_messages(grib_path: Path) -> list[TpGribMessage]:
    eccodes = require_eccodes()
    messages: list[TpGribMessage] = []

    with Path(grib_path).open("rb") as handle:
        while True:
            gid = eccodes.codes_grib_new_from_file(handle)
            if gid is None:
                break
            try:
                short_name = str(eccodes.codes_get(gid, "shortName"))
                if short_name != "tp":
                    continue

                valid_date = int(eccodes.codes_get_long(gid, "validityDate"))
                valid_time = int(eccodes.codes_get_long(gid, "validityTime"))
                step_hours = int(eccodes.codes_get_long(gid, "endStep"))
                valid_dt = datetime.strptime(f"{valid_date:08d}{valid_time:04d}", "%Y%m%d%H%M")
                latitudes, longitudes, values = build_grid_arrays(gid)
                messages.append(
                    TpGribMessage(
                        valid_time=valid_dt,
                        step_hours=step_hours,
                        latitudes=latitudes,
                        longitudes=longitudes,
                        values_mm=values * 1000.0,
                    )
                )
            finally:
                eccodes.codes_release(gid)

    if not messages:
        raise ValueError(f"No 'tp' messages found in {grib_path}.")
    return sorted(messages, key=lambda item: (item.valid_time, item.step_hours))
