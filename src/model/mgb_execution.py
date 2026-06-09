from __future__ import annotations

import logging
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from common.models import CommandPlan, ModelOutput, RunMetadata
from common.paths import (
    logs_dir as default_logs_dir,
    mgb_executable_path as default_mgb_executable_path,
    mgb_input_dir as default_mgb_input_dir,
    mgb_output_dir as default_mgb_output_dir,
    mgb_remote_workspace_root,
    relative_to_repo,
)

LOGGER_NAME = "floodqc.model.mgb_execution"
MGB_EXECUTABLE_PATH = default_mgb_executable_path()
LOCAL_INPUT_DIR = default_mgb_input_dir()
LOCAL_OUTPUT_DIR = default_mgb_output_dir()
MGB_WORKSPACE_ROOT = mgb_remote_workspace_root()


def script_stem() -> str:
    return Path(__file__).stem


def build_execution_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


def configure_run_logger(log_file: Path) -> logging.Logger:
    log_file.parent.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger(LOGGER_NAME)
    logger.setLevel(logging.INFO)
    for handler in logger.handlers[:]:
        handler.close()
        logger.removeHandler(handler)
    logger.propagate = False

    formatter = logging.Formatter("%(asctime)s | %(levelname)s | %(message)s")

    file_handler = logging.FileHandler(log_file, mode="w", encoding="utf-8")
    file_handler.setFormatter(formatter)
    logger.addHandler(file_handler)

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    logger.addHandler(stream_handler)
    return logger


def _require_existing_file(path: Path, *, label: str) -> None:
    if not path.exists():
        raise FileNotFoundError(f"{label} not found: {path}")
    if not path.is_file():
        raise ValueError(f"{label} must be a file: {path}")


def _require_existing_directory(path: Path, *, label: str) -> None:
    if not path.exists():
        raise FileNotFoundError(f"{label} not found: {path}")
    if not path.is_dir():
        raise ValueError(f"{label} must be a directory: {path}")


def _ensure_directory(path: Path) -> None:
    path.mkdir(parents=True, exist_ok=True)


def _clear_directory_contents(path: Path) -> None:
    _ensure_directory(path)
    for child in path.iterdir():
        if child.is_dir():
            shutil.rmtree(child)
        else:
            child.unlink()


def _copy_directory_contents(source_dir: Path, destination_dir: Path) -> None:
    _require_existing_directory(source_dir, label="Diretorio de origem")
    _ensure_directory(destination_dir)
    for child in source_dir.iterdir():
        target = destination_dir / child.name
        if child.is_dir():
            shutil.copytree(child, target)
        else:
            shutil.copy2(child, target)


def _directory_has_files(path: Path) -> bool:
    return any(candidate.is_file() for candidate in path.rglob("*"))


def _collect_output_asset_refs(output_dir: Path) -> list[str]:
    asset_refs: list[str] = []
    for candidate in sorted(output_dir.rglob("*")):
        if candidate.is_file():
            asset_refs.append(relative_to_repo(candidate))
    return asset_refs


def _prepare_workspace(plan: CommandPlan, logger: logging.Logger) -> None:
    workspace_root = Path(plan.metadata["workspace_root"])
    remote_input_dir = Path(plan.metadata["remote_input_dir"])
    remote_output_dir = Path(plan.metadata["remote_output_dir"])
    local_input_dir = Path(plan.metadata["local_input_dir"])

    _ensure_directory(workspace_root)
    logger.info("mgb_workspace_prepare root=%s", workspace_root)
    _clear_directory_contents(workspace_root)
    _copy_directory_contents(local_input_dir, remote_input_dir)
    _ensure_directory(remote_output_dir)
    logger.info(
        "mgb_workspace_ready local_input=%s remote_input=%s remote_output=%s",
        local_input_dir,
        remote_input_dir,
        remote_output_dir,
    )


def _copy_output_back(plan: CommandPlan, logger: logging.Logger) -> list[str]:
    remote_output_dir = Path(plan.metadata["remote_output_dir"])
    local_output_dir = Path(plan.metadata["local_output_dir"])

    _clear_directory_contents(local_output_dir)
    _copy_directory_contents(remote_output_dir, local_output_dir)
    asset_refs = _collect_output_asset_refs(local_output_dir)
    logger.info(
        "mgb_output_copied remote_output=%s local_output=%s files=%s",
        remote_output_dir,
        local_output_dir,
        len(asset_refs),
    )
    return asset_refs


def _stream_process_output(process: subprocess.Popen[str], logger: logging.Logger) -> None:
    if process.stdout is None:
        return
    for raw_line in process.stdout:
        logger.info("mgb_process %s", raw_line.rstrip("\r\n"))


def prepare_mgb_execution(
    run: RunMetadata,
    executable_path: str | None = None,
    workdir: str | None = None,
    input_dir: str | None = None,
    output_dir: str | None = None,
    workspace_root: str | None = None,
) -> CommandPlan:
    """Prepare the real MGB execution plan on Windows."""
    del workdir

    local_executable_path = Path(executable_path) if executable_path is not None else MGB_EXECUTABLE_PATH
    local_input_dir = Path(input_dir) if input_dir is not None else LOCAL_INPUT_DIR
    local_output_dir = Path(output_dir) if output_dir is not None else LOCAL_OUTPUT_DIR
    remote_workspace_root = Path(workspace_root) if workspace_root is not None else MGB_WORKSPACE_ROOT
    remote_input_dir = remote_workspace_root / "Input"
    remote_output_dir = remote_workspace_root / "Output"

    _require_existing_file(local_executable_path, label="MGB executable")
    _require_existing_directory(local_input_dir, label="Local MGB Input directory")

    return CommandPlan(
        command=[str(local_executable_path)],
        working_directory=str(remote_workspace_root),
        metadata={
            "run_id": run.run_id,
            "reference_time": run.reference_time,
            "workspace_root": str(remote_workspace_root),
            "local_executable_path": str(local_executable_path),
            "local_input_dir": str(local_input_dir),
            "local_output_dir": str(local_output_dir),
            "remote_input_dir": str(remote_input_dir),
            "remote_output_dir": str(remote_output_dir),
        },
    )


def execute_mgb_plan(plan: CommandPlan, *, dry_run: bool = False, logs_dir: Path | None = None) -> ModelOutput:
    """Run the real MGB runner or return only the plan in dry-run mode."""
    if dry_run:
        plan.metadata["status"] = "dry_run"
        return ModelOutput(
            output_name="mgb_dry_run",
            description="MGB model dry-run; no simulation was executed.",
            asset_refs=[],
        )

    execution_id = build_execution_id()
    log_root = logs_dir or default_logs_dir()
    log_path = log_root / script_stem() / f"{execution_id}.log"
    logger = configure_run_logger(log_path)
    plan.metadata["log_path"] = str(log_path)

    logger.info(
        "mgb_execution_started run_id=%s executable=%s workspace=%s",
        plan.metadata["run_id"],
        plan.metadata["local_executable_path"],
        plan.metadata["workspace_root"],
    )

    _prepare_workspace(plan, logger)

    try:
        process = subprocess.Popen(
            plan.command,
            cwd=plan.working_directory,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
    except OSError as exc:
        raise RuntimeError(
            f"Falha ao iniciar o executavel do MGB: {plan.metadata['local_executable_path']} | log={log_path}"
        ) from exc

    _stream_process_output(process, logger)
    return_code = process.wait()
    plan.metadata["return_code"] = return_code

    if return_code != 0:
        logger.error("mgb_execution_failed exit_code=%s", return_code)
        raise RuntimeError(f"Execucao do MGB falhou com exit code {return_code}. Log: {log_path}")

    remote_output_dir = Path(plan.metadata["remote_output_dir"])
    if not _directory_has_files(remote_output_dir):
        logger.error("mgb_execution_failed output_dir_empty=%s", remote_output_dir)
        raise RuntimeError(f"Execucao do MGB terminou sem arquivos em Output. Log: {log_path}")

    asset_refs = _copy_output_back(plan, logger)
    plan.metadata["status"] = "success"
    logger.info("mgb_execution_finished exit_code=%s files=%s", return_code, len(asset_refs))
    return ModelOutput(
        output_name="mgb_output",
        description="Execucao real do MGB concluida com sucesso.",
        asset_refs=asset_refs,
    )
