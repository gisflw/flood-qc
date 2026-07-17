from __future__ import annotations

from copy import deepcopy
from datetime import datetime
from pathlib import Path
from typing import Any

import yaml

from mgb_ops.config.workspace import resolve_workspace
from mgb_ops.utils.time import validate_timestep_hours


DEFAULT_SETTINGS: dict[str, Any] = {
    "run": {
        "reference_time": "yesterday",
        "timestep_hours": 1,
    },
    "ingest": {
        "request_days": 90,
        "timeout_seconds": 15,
        "fetch_window_days": 30,
        "observed_aggregation": {
            "rain": "sum",
            "level": "mean",
            "flow": "mean",
        },
    },
    "forecast": {
        "provider": "ecmwf",
        "lookback_cycles": 12,
    },
    "spatial_grid": {
        "bbox": None,
        "resolution_degrees": 0.1,
    },
    "spatial": {
        "gpkg_path": "data/source/rs_hydro.gpkg",
    },
    "summaries": {
        "forecast_days": [1, 3, 7, 14],
        "accum_hours": [24, 72, 240, 720],
        "selected_mini_ids": [],
    },
    "mgb": {
        "input_days_before": 56,
        "output_days_before": 28,
        "forecast_horizon_days": 14,
        "use_forecast_data": True,
    },
    "rainfall_interpolation": {
        "nearest_stations": 5,
        "power": 2.0,
    },
}


def _load_yaml(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(f"Config file not found: {path}")
    with path.open("r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle) or {}
    if not isinstance(data, dict):
        raise ValueError(f"Invalid config at {path}: expected a YAML object.")
    return data


def _deep_merge(base: dict[str, Any], override: dict[str, Any]) -> dict[str, Any]:
    merged = deepcopy(base)
    for key, value in override.items():
        current = merged.get(key)
        if isinstance(current, dict) and isinstance(value, dict):
            merged[key] = _deep_merge(current, value)
        else:
            merged[key] = deepcopy(value)
    return merged


def _require_mapping(value: Any, context: str) -> dict[str, Any]:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be a YAML object.")
    return value


def _validate_reference_time(value: Any, context: str) -> None:
    if not isinstance(value, str):
        raise ValueError(f"{context} must be an ISO string, 'now', or 'yesterday'.")
    normalized = value.strip()
    if not normalized:
        raise ValueError(f"{context} cannot be empty.")
    if normalized in {"now", "yesterday"}:
        return
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    try:
        datetime.fromisoformat(normalized)
    except ValueError as exc:
        raise ValueError(f"{context} must be a valid ISO string, 'now', or 'yesterday'.") from exc


def _validate_positive_int(value: Any, context: str) -> None:
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        raise ValueError(f"{context} must be an integer >= 1.")


def _validate_timestep_hours(value: Any, context: str) -> None:
    try:
        validate_timestep_hours(value)
    except ValueError as exc:
        raise ValueError(f"{context} {exc}") from exc


def _validate_positive_number(value: Any, context: str) -> None:
    if not isinstance(value, (int, float)) or isinstance(value, bool) or value <= 0:
        raise ValueError(f"{context} must be a number > 0.")


def _validate_optional_nonnegative_number(value: Any, context: str) -> None:
    if value is None:
        return
    if not isinstance(value, (int, float)) or isinstance(value, bool) or value < 0:
        raise ValueError(f"{context} must be null or a number >= 0.")


def _validate_bool(value: Any, context: str) -> None:
    if not isinstance(value, bool):
        raise ValueError(f"{context} must be boolean.")


def _validate_forecast_provider(value: Any, context: str) -> None:
    if not isinstance(value, str) or value.strip().lower() not in {"ecmwf", "noaa"}:
        raise ValueError(f"{context} must be one of ['ecmwf', 'noaa'].")


def _validate_nonempty_path(value: Any, context: str) -> None:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{context} must be a non-empty path string.")


def _validate_optional_bbox(value: Any, context: str) -> None:
    if value is None:
        return
    if not isinstance(value, list) or len(value) != 4:
        raise ValueError(f"{context} must be null or a list of four numbers: [west, south, east, north].")
    if any(not isinstance(item, (int, float)) or isinstance(item, bool) for item in value):
        raise ValueError(f"{context} must contain only numbers.")
    west, south, east, north = (float(item) for item in value)
    if west >= east or south >= north:
        raise ValueError(f"{context} must satisfy west < east and south < north.")


def _validate_positive_int_list(value: Any, context: str) -> None:
    if not isinstance(value, list):
        raise ValueError(f"{context} must be a list of integers >= 1.")
    for item in value:
        if not isinstance(item, int) or isinstance(item, bool) or item < 1:
            raise ValueError(f"{context} must contain only integers >= 1.")


def _validate_selected_mini_ids(value: Any, context: str) -> None:
    if not isinstance(value, list):
        raise ValueError(f"{context} must be a list.")
    for item in value:
        if not isinstance(item, (str, int)) or isinstance(item, bool):
            raise ValueError(f"{context} must contain only strings or integers.")
        if not str(item).strip():
            raise ValueError(f"{context} cannot contain empty values.")


def _validate_observed_aggregation(value: Any, context: str) -> None:
    if not isinstance(value, dict):
        raise ValueError(f"{context} must be a YAML object.")
    required_variables = {"rain", "level", "flow"}
    extra_keys = sorted(set(value) - required_variables)
    missing_keys = sorted(required_variables - set(value))
    if extra_keys:
        raise ValueError(f"{context} contains unsupported keys: {extra_keys}")
    if missing_keys:
        raise ValueError(f"{context} is missing required keys: {missing_keys}")
    allowed_methods = {"sum", "mean", "last"}
    for variable_code, method in value.items():
        if not isinstance(method, str) or method.strip().lower() not in allowed_methods:
            raise ValueError(
                f"{context}.{variable_code} must be one of {sorted(allowed_methods)}."
            )


def _validate_section(config: dict[str, Any], schema: dict[str, Any], context: str) -> None:
    extra_keys = sorted(set(config) - set(schema))
    missing_keys = sorted(set(schema) - set(config))
    if extra_keys:
        raise ValueError(f"{context} contains unsupported keys: {extra_keys}")
    if missing_keys:
        raise ValueError(f"{context} is missing required keys: {missing_keys}")

    for key, validator in schema.items():
        value = config[key]
        key_context = f"{context}.{key}"
        if isinstance(validator, dict):
            _validate_section(_require_mapping(value, key_context), validator, key_context)
            continue
        validator(value, key_context)


def _validate_settings(settings: dict[str, Any]) -> None:
    schema: dict[str, Any] = {
        "run": {
            "reference_time": _validate_reference_time,
            "timestep_hours": _validate_timestep_hours,
        },
        "ingest": {
            "request_days": _validate_positive_int,
            "timeout_seconds": _validate_positive_number,
            "fetch_window_days": _validate_positive_int,
            "observed_aggregation": _validate_observed_aggregation,
        },
        "forecast": {
            "provider": _validate_forecast_provider,
            "lookback_cycles": _validate_positive_int,
        },
        "spatial_grid": {
            "bbox": _validate_optional_bbox,
            "resolution_degrees": _validate_positive_number,
        },
        "spatial": {
            "gpkg_path": _validate_nonempty_path,
        },
        "summaries": {
            "forecast_days": _validate_positive_int_list,
            "accum_hours": _validate_positive_int_list,
            "selected_mini_ids": _validate_selected_mini_ids,
        },
        "mgb": {
            "input_days_before": _validate_positive_int,
            "output_days_before": _validate_positive_int,
            "forecast_horizon_days": _validate_positive_int,
            "use_forecast_data": _validate_bool,
        },
        "rainfall_interpolation": {
            "nearest_stations": _validate_positive_int,
            "power": _validate_positive_number,
        },
    }
    _validate_section(_require_mapping(settings, "config"), schema, "config")


def load_settings(
    *,
    config_dir: Path | None = None,
    workspace: str | Path | None = None,
    require_custom: bool | None = None,
) -> dict[str, Any]:
    if config_dir is not None:
        raise ValueError("config_dir is no longer supported; use <workspace>/config/custom.yaml.")

    custom_path = resolve_workspace(workspace) / "config" / "custom.yaml"
    custom_required = False if require_custom is None else require_custom

    settings = deepcopy(DEFAULT_SETTINGS)
    if custom_path.exists() or custom_required:
        settings = _deep_merge(settings, _load_yaml(custom_path))
    _validate_settings(settings)
    return settings
