from __future__ import annotations

import os
import shlex
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping


def _normalize_env_value(value: object | None) -> str | None:
    if value is None:
        return None
    normalized = str(value).strip()
    return normalized or None


def parse_dotenv(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}

    values: dict[str, str] = {}
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("export "):
            stripped = stripped[len("export ") :].strip()
        if "=" not in stripped:
            raise ValueError(f"Invalid .env line {line_number} in {path}: expected KEY=VALUE.")
        key, raw_value = stripped.split("=", 1)
        key = key.strip()
        if not key:
            raise ValueError(f"Invalid .env line {line_number} in {path}: empty key.")
        try:
            parts = shlex.split(raw_value, comments=True, posix=True)
        except ValueError as exc:
            raise ValueError(f"Invalid .env value for {key!r} in {path}.") from exc
        values[key] = " ".join(parts) if parts else ""
    return values


def load_workspace_env(workspace: str | Path) -> dict[str, str]:
    return parse_dotenv(Path(workspace).expanduser().resolve() / ".env")


def resolve_env_value(
    name: str,
    *,
    explicit: object | None = None,
    env: Mapping[str, str] | None = None,
    dotenv_values: Mapping[str, str] | None = None,
    default: object | None = None,
) -> str | None:
    env_values = os.environ if env is None else env
    for candidate in (
        _normalize_env_value(explicit),
        _normalize_env_value(env_values.get(name)),
        _normalize_env_value((dotenv_values or {}).get(name)),
        _normalize_env_value(default),
    ):
        if candidate is not None:
            return candidate
    return None


def require_env_value(
    name: str,
    *,
    explicit: object | None = None,
    env: Mapping[str, str] | None = None,
    dotenv_values: Mapping[str, str] | None = None,
) -> str:
    value = resolve_env_value(name, explicit=explicit, env=env, dotenv_values=dotenv_values)
    if not value:
        raise RuntimeError(f"Missing required environment value. Set {name} in the environment or workspace .env.")
    return value


@dataclass(frozen=True, slots=True)
class RuntimeEnv:
    values: Mapping[str, str]

    def get(self, name: str, default: object | None = None) -> str | None:
        return resolve_env_value(name, dotenv_values=self.values, default=default)

    def require(self, name: str, explicit: object | None = None) -> str:
        return require_env_value(name, explicit=explicit, dotenv_values=self.values)
