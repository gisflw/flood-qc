from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping


MGB_OPS_WORKSPACE_ENV = "MGB_OPS_WORKSPACE"
MGB_REMOTE_WORKSPACE_ENV = "MGB_OPS_REMOTE_WORKSPACE"
DEFAULT_MGB_EXECUTABLE_NAME = "MGB_Inercial_PrevRS_FORTRAN.exe"
DEFAULT_REMOTE_WORKSPACE = Path("C:/mgb-hora")


@dataclass(frozen=True, slots=True)
class RuntimePaths:
    workspace: Path
    remote_workspace_root: Path = DEFAULT_REMOTE_WORKSPACE

    def __post_init__(self) -> None:
        object.__setattr__(self, "workspace", Path(self.workspace).expanduser().resolve())
        object.__setattr__(self, "remote_workspace_root", Path(self.remote_workspace_root))

    @property
    def config_dir(self) -> Path:
        return self.workspace / "config"

    @property
    def data_dir(self) -> Path:
        return self.workspace / "data"

    @property
    def history_db(self) -> Path:
        return self.data_dir / "history.sqlite"

    @property
    def runs_dir(self) -> Path:
        return self.data_dir / "runs"

    @property
    def source_dir(self) -> Path:
        return self.data_dir / "source"

    @property
    def downloads_dir(self) -> Path:
        return self.data_dir / "downloads"

    @property
    def assets_dir(self) -> Path:
        return self.data_dir / "assets"

    @property
    def cache_dir(self) -> Path:
        return self.data_dir / "cache"

    @property
    def processed_dir(self) -> Path:
        return self.data_dir / "processed"

    @property
    def reports_dir(self) -> Path:
        return self.data_dir / "reports"

    @property
    def station_inventory_csv_path(self) -> Path:
        return self.source_dir / "history_station_inventory.csv"

    @property
    def station_level_reference_csv_path(self) -> Path:
        return self.source_dir / "station_level_reference.csv"

    @property
    def registered_floods_csv_path(self) -> Path:
        return self.source_dir / "registered_floods.csv"

    @property
    def logs_dir(self) -> Path:
        return self.workspace / "logs"

    @property
    def mgb_runner_dir(self) -> Path:
        return self.workspace / "mgb_runner"

    @property
    def mgb_input_dir(self) -> Path:
        return self.mgb_runner_dir / "Input"

    @property
    def mgb_output_dir(self) -> Path:
        return self.mgb_runner_dir / "Output"

    @property
    def mgb_executable_path(self) -> Path:
        return self.mgb_runner_dir / DEFAULT_MGB_EXECUTABLE_NAME

    def run_db_path(self, run_id: str) -> Path:
        return self.runs_dir / f"{run_id}.sqlite"

    def ensure_standard_dirs(self) -> None:
        for path in (
            self.source_dir,
            self.downloads_dir,
            self.assets_dir,
            self.cache_dir,
            self.processed_dir,
            self.reports_dir,
            self.runs_dir,
            self.logs_dir,
        ):
            path.mkdir(parents=True, exist_ok=True)

    def resolve_path(self, configured_path: str | Path) -> Path:
        return resolve_workspace_path(self.workspace, configured_path)


def _env_value(values: Mapping[str, str] | None, name: str) -> str:
    if values is None:
        return os.getenv(name, "").strip()
    return str(values.get(name, "")).strip()


def resolve_workspace(
    workspace: str | Path | None = None,
    *,
    env: Mapping[str, str] | None = None,
    dotenv_values: Mapping[str, str] | None = None,
) -> Path:
    if workspace is not None:
        return Path(workspace).expanduser().resolve()
    env_workspace = _env_value(env, MGB_OPS_WORKSPACE_ENV)
    if env_workspace:
        return Path(env_workspace).expanduser().resolve()
    dotenv_workspace = _env_value(dotenv_values, MGB_OPS_WORKSPACE_ENV)
    if dotenv_workspace:
        return Path(dotenv_workspace).expanduser().resolve()
    return Path.cwd().resolve()


def resolve_workspace_path(workspace: str | Path, configured_path: str | Path) -> Path:
    candidate = Path(configured_path).expanduser()
    if candidate.is_absolute():
        return candidate.resolve()
    return (Path(workspace).expanduser().resolve() / candidate).resolve()


__all__ = [
    "DEFAULT_MGB_EXECUTABLE_NAME",
    "DEFAULT_REMOTE_WORKSPACE",
    "MGB_OPS_WORKSPACE_ENV",
    "MGB_REMOTE_WORKSPACE_ENV",
    "RuntimePaths",
    "resolve_workspace",
    "resolve_workspace_path",
]
