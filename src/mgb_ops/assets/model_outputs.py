from __future__ import annotations

from pathlib import Path
from typing import Literal

import numpy as np
import pandas as pd
import xarray as xr

from mgb_ops.common.time_utils import DashboardWindow

NETCDF_ZLIB_COMPLEVEL = 4
NETCDF_PACKING_SCALE_FACTOR = 0.01
NETCDF_PACKED_FILL_VALUE = np.iinfo(np.int32).min
MGB_VARIABLE_METADATA = {
    "precipitation": {"display_name": "Precipitation", "unit": "mm"},
    "level": {"display_name": "Level", "unit": "m"},
    "flow": {"display_name": "Flow", "unit": "m3/s"},
}
TimeSegment = Literal["all", "current", "forecast"]


class StaleModelOutputsError(ValueError):
    """Raised when canonical model output metadata differs from the requested run."""


def _required_model_time_attr(dataset: xr.Dataset, name: str) -> pd.Timestamp:
    raw = dataset.attrs.get(name)
    if raw in (None, ""):
        raise ValueError(f"MGB NetCDF missing required global attribute {name!r}.")
    try:
        value = pd.Timestamp(raw)
    except Exception as exc:
        raise ValueError(f"MGB NetCDF has invalid global attribute {name!r}: {raw!r}.") from exc
    if value.tzinfo is not None:
        value = value.tz_convert(None)
    return value


def validate_model_outputs_netcdf(
    path: Path,
    *,
    expected_window: DashboardWindow | None = None,
) -> dict[str, object]:
    """Validate the canonical MGB model-output NetCDF contract."""
    source = Path(path)
    if not source.exists():
        raise FileNotFoundError(f"Canonical MGB NetCDF not found: {source}")
    with xr.open_dataset(source, decode_times=True) as dataset:
        missing_dims = {"time", "mini"}.difference(dataset.dims)
        if missing_dims:
            raise ValueError(f"MGB NetCDF missing required dimensions: {sorted(missing_dims)}")
        missing = {"mini_id", "time_segment"}.difference(dataset.variables)
        if missing:
            raise ValueError(f"MGB NetCDF missing required variables: {sorted(missing)}")
        present = [code for code in MGB_VARIABLE_METADATA if code in dataset]
        if not present:
            raise ValueError(
                "MGB NetCDF must contain at least one model variable: "
                "precipitation, level, or flow."
            )
        if dataset["mini_id"].dims != ("mini",):
            raise ValueError("MGB NetCDF mini_id must use dimension ('mini',).")
        if dataset["time_segment"].dims != ("time",):
            raise ValueError("MGB NetCDF time_segment must use dimension ('time',).")
        for code in present:
            if dataset[code].dims != ("time", "mini"):
                raise ValueError(f"MGB NetCDF {code} must use dimensions ('time', 'mini').")
        mini_ids = np.asarray(dataset["mini_id"].values)
        if len(np.unique(mini_ids)) != len(mini_ids):
            raise ValueError("MGB NetCDF mini_id values must be unique.")
        segments = set(np.asarray(dataset["time_segment"].values).astype(int).tolist())
        if not segments.issubset({0, 1}):
            raise ValueError("MGB NetCDF time_segment may contain only 0 (current) and 1 (forecast).")
        times = pd.to_datetime(dataset["time"].values, errors="coerce")
        if pd.isna(times).any() or not pd.DatetimeIndex(times).is_monotonic_increasing:
            raise ValueError("MGB NetCDF time must be valid and monotonically increasing.")
        model_window = DashboardWindow(
            start_time=_required_model_time_attr(dataset, "window_start").to_pydatetime(),
            cutoff_time=_required_model_time_attr(dataset, "reference_time").to_pydatetime(),
            forecast_end_exclusive=_required_model_time_attr(dataset, "window_end_exclusive").to_pydatetime(),
        )
        if expected_window is not None and model_window != expected_window:
            raise StaleModelOutputsError(
                "Stale model_outputs.nc metadata: "
                f"expected start={expected_window.start_time.isoformat()}, "
                f"reference={expected_window.cutoff_time.isoformat()}, "
                f"end_exclusive={expected_window.forecast_end_exclusive.isoformat()}; "
                f"actual start={model_window.start_time.isoformat()}, "
                f"reference={model_window.cutoff_time.isoformat()}, "
                f"end_exclusive={model_window.forecast_end_exclusive.isoformat()}."
            )
        return {
            "path": source,
            "mini_count": int(dataset.sizes["mini"]),
            "time_count": int(dataset.sizes["time"]),
            "variables": tuple(present),
            "mini_ids": tuple(int(value) for value in mini_ids),
            "start_time": pd.Timestamp(times[0]) if len(times) else None,
            "end_time": pd.Timestamp(times[-1]) if len(times) else None,
            "window": model_window,
        }


def write_model_outputs_netcdf(
    *,
    path: Path,
    variables: dict[str, np.ndarray],
    variable_attrs: dict[str, dict[str, str]],
    time_values: np.ndarray,
    time_segment: np.ndarray,
    mini_ids: list[int],
    global_attrs: dict[str, object],
) -> Path:
    """Build and serialize the canonical model-output NetCDF structure."""
    unknown = set(variables).difference(MGB_VARIABLE_METADATA)
    if unknown or not variables:
        raise ValueError(
            "Model-output variables must be precipitation/level/flow; "
            f"found {sorted(variables)}"
        )
    expected_shape = (len(time_values), len(mini_ids))
    for code, values in variables.items():
        if np.asarray(values).shape != expected_shape:
            raise ValueError(f"Model-output {code} must have shape {expected_shape}.")
    if np.asarray(time_segment).shape != (len(time_values),):
        raise ValueError("Model-output time_segment must match the time coordinate.")
    dataset = xr.Dataset(
        data_vars={
            code: (("time", "mini"), values, variable_attrs[code])
            for code, values in variables.items()
        }
        | {
            "time_segment": (
                ("time",),
                time_segment,
                {
                    "long_name": "MGB output time segment",
                    "flag_values": np.array([0, 1], dtype=np.int8),
                    "flag_meanings": "current_simulation forecast",
                },
            )
        },
        coords={
            "time": (("time",), time_values, {"long_name": "time"}),
            "mini": (("mini",), np.arange(len(mini_ids), dtype=np.int32), {"long_name": "mini-basin index"}),
            "mini_id": (("mini",), np.asarray(mini_ids, dtype=np.int32), {"long_name": "MGB mini-basin identifier"}),
        },
        attrs=global_attrs,
    )
    packed_min = np.iinfo(np.int32).min + 1
    packed_max = np.iinfo(np.int32).max
    for code, values in variables.items():
        finite = np.asarray(values, dtype=np.float64)
        finite = finite[np.isfinite(finite)]
        packed = np.rint(finite / NETCDF_PACKING_SCALE_FACTOR)
        if packed.size and (packed.min() < packed_min or packed.max() > packed_max):
            raise OverflowError(
                f"Model-output {code} exceeds the int32 packing range "
                f"for scale_factor={NETCDF_PACKING_SCALE_FACTOR}."
            )
    encoding = {
        code: {
            "dtype": "i4",
            "_FillValue": NETCDF_PACKED_FILL_VALUE,
            "scale_factor": NETCDF_PACKING_SCALE_FACTOR,
            "zlib": True,
            "complevel": NETCDF_ZLIB_COMPLEVEL,
        }
        for code in variables
    }
    encoding["time_segment"] = {"dtype": "i1"}
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    try:
        dataset.to_netcdf(target, engine="netcdf4", encoding=encoding)
    finally:
        dataset.close()
    return target


def list_model_variables(path: Path | None = None) -> pd.DataFrame:
    available = set(MGB_VARIABLE_METADATA)
    if path is not None:
        available = set(validate_model_outputs_netcdf(path)["variables"])
    return pd.DataFrame([
        {"variable_code": code, **MGB_VARIABLE_METADATA[code]}
        for code in sorted(available)
    ])


def load_mgb_series(
    path: Path,
    *,
    mini_id: int,
    variable_code: str,
    time_segment: TimeSegment | int | None = "all",
    window: DashboardWindow | None = None,
) -> pd.DataFrame:
    """Read one mini/variable from the canonical model-output artifact."""
    validate_model_outputs_netcdf(path, expected_window=window)
    code = str(variable_code).strip().lower()
    if code not in MGB_VARIABLE_METADATA:
        raise ValueError("variable_code must be 'precipitation', 'level', or 'flow'.")
    with xr.open_dataset(path, decode_times=True) as dataset:
        if code not in dataset:
            raise ValueError(f"MGB NetCDF does not contain variable {code!r}.")
        matches = np.flatnonzero(np.asarray(dataset["mini_id"].values) == int(mini_id))
        if len(matches) == 0:
            raise ValueError(f"Mini {mini_id} was not found in {path}.")
        frame = pd.DataFrame({
            "dt": pd.to_datetime(dataset["time"].values),
            "prev_flag": np.asarray(dataset["time_segment"].values, dtype=np.int8),
            "value": np.asarray(dataset[code].isel(mini=int(matches[0])).values, dtype=float),
        })
    segment_map = {"current": 0, "forecast": 1}
    if time_segment not in (None, "all"):
        normalized = str(time_segment).lower()
        try:
            flag = segment_map[normalized] if normalized in segment_map else int(time_segment)
        except (TypeError, ValueError) as exc:
            raise ValueError("time_segment must be 'all', 'current', 'forecast', 0, or 1.") from exc
        if flag not in (0, 1):
            raise ValueError("time_segment must be 'all', 'current', 'forecast', 0, or 1.")
        frame = frame[frame["prev_flag"] == flag]
    if window is not None:
        frame = frame[
            (frame["dt"] >= pd.Timestamp(window.start_time))
            & (frame["dt"] < pd.Timestamp(window.forecast_end_exclusive))
        ]
    meta = MGB_VARIABLE_METADATA[code]
    frame["variable_code"] = code
    frame["display_name"] = meta["display_name"]
    frame["unit"] = meta["unit"]
    return frame.sort_values("dt").reset_index(drop=True)


read_model_outputs = validate_model_outputs_netcdf
select_mgb_series = load_mgb_series
