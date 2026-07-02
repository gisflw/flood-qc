from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping

from mgb_ops.common.env import RuntimeEnv, parse_dotenv, resolve_env_value
from mgb_ops.common.paths import (
    DEFAULT_REMOTE_WORKSPACE,
    MGB_OPS_WORKSPACE_ENV,
    MGB_REMOTE_WORKSPACE_ENV,
    RuntimePaths,
    resolve_workspace,
)
from mgb_ops.common.settings import load_settings


@dataclass(frozen=True, slots=True)
class RuntimeContext:
    paths: RuntimePaths
    settings: dict[str, object]
    env: RuntimeEnv

    @classmethod
    def from_workspace(
        cls,
        workspace: str | Path,
        *,
        dotenv_path: str | Path | None = None,
        env: Mapping[str, str] | None = None,
        remote_workspace: str | Path | None = None,
        require_custom_settings: bool = False,
    ) -> RuntimeContext:
        return build_runtime_context(
            workspace=workspace,
            dotenv_path=dotenv_path,
            env=env,
            remote_workspace=remote_workspace,
            require_custom_settings=require_custom_settings,
        )


def load_runtime_env(
    *,
    workspace: str | Path | None = None,
    dotenv_path: str | Path | None = None,
    env: Mapping[str, str] | None = None,
) -> tuple[Path, RuntimeEnv]:
    process_env = os.environ if env is None else env
    bootstrap_dotenv_path = Path(dotenv_path) if dotenv_path is not None else Path.cwd() / ".env"
    bootstrap_values = parse_dotenv(bootstrap_dotenv_path) if bootstrap_dotenv_path.exists() else {}

    resolved_workspace = resolve_workspace(workspace, env=process_env, dotenv_values=bootstrap_values)
    workspace_dotenv_path = Path(dotenv_path) if dotenv_path is not None else resolved_workspace / ".env"
    dotenv_values = dict(bootstrap_values)
    if workspace_dotenv_path.exists() and workspace_dotenv_path.resolve() != bootstrap_dotenv_path.resolve():
        dotenv_values.update(parse_dotenv(workspace_dotenv_path))
    elif workspace_dotenv_path.exists() and not bootstrap_values:
        dotenv_values.update(parse_dotenv(workspace_dotenv_path))

    return resolved_workspace, RuntimeEnv(dotenv_values)


def build_runtime_context(
    *,
    workspace: str | Path | None = None,
    dotenv_path: str | Path | None = None,
    env: Mapping[str, str] | None = None,
    remote_workspace: str | Path | None = None,
    require_custom_settings: bool = False,
) -> RuntimeContext:
    process_env = os.environ if env is None else env
    resolved_workspace, runtime_env = load_runtime_env(workspace=workspace, dotenv_path=dotenv_path, env=process_env)
    remote_root = resolve_env_value(
        MGB_REMOTE_WORKSPACE_ENV,
        explicit=remote_workspace,
        env=process_env,
        dotenv_values=runtime_env.values,
        default=DEFAULT_REMOTE_WORKSPACE,
    )
    paths = RuntimePaths(resolved_workspace, remote_workspace_root=Path(str(remote_root)))
    settings = load_settings(workspace=paths.workspace, require_custom=require_custom_settings)
    return RuntimeContext(paths=paths, settings=settings, env=runtime_env)


def resolve_workspace_from_runtime_env(
    *,
    workspace: str | Path | None = None,
    dotenv_path: str | Path | None = None,
    env: Mapping[str, str] | None = None,
) -> Path:
    resolved_workspace, _ = load_runtime_env(workspace=workspace, dotenv_path=dotenv_path, env=env)
    return resolved_workspace
