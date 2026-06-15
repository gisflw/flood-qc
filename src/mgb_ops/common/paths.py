from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path
from typing import Mapping


REPO_ROOT = Path(__file__).resolve().parents[2]
try:
    import mgb_ops.assets as _mgb_ops_assets

    ASSETS_DIR = Path(_mgb_ops_assets.__file__).resolve().parent
except ModuleNotFoundError:
    ASSETS_DIR = REPO_ROOT

SQL_DIR = ASSETS_DIR / "sql"
MGB_OPS_WORKSPACE_ENV = "MGB_OPS_WORKSPACE"
MGB_REMOTE_WORKSPACE_ENV = "MGB_OPS_REMOTE_WORKSPACE"
DEFAULT_MGB_EXECUTABLE_NAME = "MGB_Inercial_PrevRS_FORTRAN.exe"
DEFAULT_REMOTE_WORKSPACE = Path("C:/mgb-hora")

_WORKSPACE_ROOT: Path | None = None


@dataclass(frozen=True, slots=True)
class RuntimePaths:
    workspace: Path
    remote_workspace_root: Path = DEFAULT_REMOTE_WORKSPACE

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
    def interim_dir(self) -> Path:
        return self.data_dir / "interim"

    @property
    def timeseries_dir(self) -> Path:
        return self.data_dir / "timeseries"

    @property
    def spatial_dir(self) -> Path:
        return self.data_dir / "spatial"

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


def set_workspace(workspace: str | Path | None) -> Path:
    global _WORKSPACE_ROOT
    _WORKSPACE_ROOT = resolve_workspace(workspace)
    return _WORKSPACE_ROOT


def clear_workspace() -> None:
    global _WORKSPACE_ROOT
    _WORKSPACE_ROOT = None


def get_workspace() -> Path:
    return _WORKSPACE_ROOT or resolve_workspace()


def runtime_paths(workspace: str | Path | None = None) -> RuntimePaths:
    remote_workspace = Path(os.getenv(MGB_REMOTE_WORKSPACE_ENV, str(DEFAULT_REMOTE_WORKSPACE)))
    return RuntimePaths(resolve_workspace(workspace) if workspace is not None else get_workspace(), remote_workspace)


def history_db_path(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).history_db


def runs_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).runs_dir


def interim_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).interim_dir


def logs_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).logs_dir


def history_station_inventory_csv_path(workspace: str | Path | None = None) -> Path:
    return interim_dir(workspace) / "history_station_inventory.csv"


def timeseries_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).timeseries_dir


def spatial_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).spatial_dir


def mgb_runner_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).mgb_runner_dir


def mgb_input_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).mgb_input_dir


def mgb_output_dir(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).mgb_output_dir


def mgb_executable_path(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).mgb_executable_path


def mgb_remote_workspace_root(workspace: str | Path | None = None) -> Path:
    return runtime_paths(workspace).remote_workspace_root


def build_run_db_path(run_id: str, workspace: str | Path | None = None) -> Path:
    return runs_dir(workspace) / f"{run_id}.sqlite"


def ensure_standard_dirs(workspace: str | Path | None = None) -> None:
    for path in (
        interim_dir(workspace),
        timeseries_dir(workspace),
        spatial_dir(workspace),
        runs_dir(workspace),
        logs_dir(workspace),
    ):
        path.mkdir(parents=True, exist_ok=True)


def relative_to_repo(path: Path) -> str:
    path = Path(path)
    try:
        return path.resolve().relative_to(get_workspace().resolve()).as_posix()
    except ValueError:
        pass
    try:
        return path.resolve().relative_to(REPO_ROOT.resolve()).as_posix()
    except ValueError:
        return path.as_posix()


def resolve_workspace_path(relative_or_absolute: str | Path, workspace: str | Path | None = None) -> Path:
    candidate = Path(relative_or_absolute)
    if candidate.is_absolute():
        return candidate
    return runtime_paths(workspace).workspace / candidate
