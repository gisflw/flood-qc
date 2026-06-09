from __future__ import annotations

from copy import deepcopy
from datetime import datetime
from pathlib import Path
from typing import Any

import yaml

from common.paths import CONFIG_DIR, runtime_paths


def _load_yaml(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(f"Arquivo de config nao encontrado: {path}")
    with path.open("r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle) or {}
    if not isinstance(data, dict):
        raise ValueError(f"Config invalido em {path}: esperado um objeto YAML.")
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
        raise ValueError(f"{context} deve ser um objeto YAML.")
    return value


def _validate_reference_time(value: Any, context: str) -> None:
    if not isinstance(value, str):
        raise ValueError(f"{context} deve ser string ISO, 'now' ou 'yesterday'.")
    normalized = value.strip()
    if not normalized:
        raise ValueError(f"{context} nao pode ser vazio.")
    if normalized in {"now", "yesterday"}:
        return
    if normalized.endswith("Z"):
        normalized = normalized[:-1] + "+00:00"
    try:
        datetime.fromisoformat(normalized)
    except ValueError as exc:
        raise ValueError(f"{context} deve ser string ISO valida, 'now' ou 'yesterday'.") from exc


def _validate_positive_int(value: Any, context: str) -> None:
    if not isinstance(value, int) or isinstance(value, bool) or value < 1:
        raise ValueError(f"{context} deve ser inteiro >= 1.")


def _validate_positive_number(value: Any, context: str) -> None:
    if not isinstance(value, (int, float)) or isinstance(value, bool) or value <= 0:
        raise ValueError(f"{context} deve ser numero > 0.")


def _validate_bool(value: Any, context: str) -> None:
    if not isinstance(value, bool):
        raise ValueError(f"{context} deve ser booleano.")


def _validate_positive_int_list(value: Any, context: str) -> None:
    if not isinstance(value, list):
        raise ValueError(f"{context} deve ser lista de inteiros >= 1.")
    for item in value:
        if not isinstance(item, int) or isinstance(item, bool) or item < 1:
            raise ValueError(f"{context} deve conter apenas inteiros >= 1.")


def _validate_selected_mini_ids(value: Any, context: str) -> None:
    if not isinstance(value, list):
        raise ValueError(f"{context} deve ser lista.")
    for item in value:
        if not isinstance(item, (str, int)) or isinstance(item, bool):
            raise ValueError(f"{context} deve conter apenas strings ou inteiros.")
        if not str(item).strip():
            raise ValueError(f"{context} nao pode conter valores vazios.")


def _validate_section(config: dict[str, Any], schema: dict[str, Any], context: str) -> None:
    extra_keys = sorted(set(config) - set(schema))
    missing_keys = sorted(set(schema) - set(config))
    if extra_keys:
        raise ValueError(f"{context} contem chaves nao suportadas: {extra_keys}")
    if missing_keys:
        raise ValueError(f"{context} nao contem chaves obrigatorias: {missing_keys}")

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
        },
        "ingest": {
            "request_days": _validate_positive_int,
            "timeout_seconds": _validate_positive_number,
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
        default_path = config_dir / "default.yaml"
        custom_path = config_dir / "custom.yaml"
        custom_required = False if require_custom is None else require_custom
    else:
        default_path = CONFIG_DIR / "default.yaml"
        workspace_custom_path = runtime_paths(workspace).config_dir / "custom.yaml"
        custom_path = workspace_custom_path if workspace_custom_path.exists() else CONFIG_DIR / "custom.yaml"
        custom_required = False if require_custom is None else require_custom

    settings = _load_yaml(default_path)
    if custom_path.exists() or custom_required:
        settings = _deep_merge(settings, _load_yaml(custom_path))
    _validate_settings(settings)
    return settings
