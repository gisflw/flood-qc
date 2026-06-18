from __future__ import annotations

import logging
import os
import shutil
import subprocess
import sys
from datetime import datetime
from pathlib import Path

from mgb_ops.common.models import CommandPlan, ModelOutput, RunMetadata

LOGGER_NAME = "model.mgb_execution"


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


def _directory_has_files(path: Path) -> bool:
    return any(candidate.is_file() for candidate in path.rglob("*"))


def _relative_asset_ref(path: Path, *, asset_base_dir: Path) -> str:
    resolved_path = Path(path).resolve()
    resolved_base = Path(asset_base_dir).resolve()
    try:
        return resolved_path.relative_to(resolved_base).as_posix()
    except ValueError:
        return Path(path).as_posix()


def _collect_output_asset_refs(output_dir: Path, *, asset_base_dir: Path) -> list[str]:
    asset_refs: list[str] = []
    for candidate in sorted(output_dir.rglob("*")):
        if candidate.is_file():
            asset_refs.append(_relative_asset_ref(candidate, asset_base_dir=asset_base_dir))
    return asset_refs


def _prepare_output_directory(plan: CommandPlan, logger: logging.Logger) -> Path:
    output_dir = Path(plan.metadata["output_dir"])
    _clear_directory_contents(output_dir)
    logger.info("mgb_output_ready output=%s", output_dir)
    return output_dir


def _stream_process_output(process: subprocess.Popen[str], logger: logging.Logger) -> None:
    if process.stdout is None:
        return
    for raw_line in process.stdout:
        logger.info("mgb_process %s", raw_line.rstrip("\r\n"))


def prepare_mgb_execution(
    run: RunMetadata,
    executable_path: str,
    input_dir: str | Path,
    output_dir: str | Path,
    workspace_root: str | Path,
    asset_base_dir: str | Path,
    workdir: str | None = None,
) -> CommandPlan:
    """Prepare a direct MGB execution plan for the configured runner paths."""
    del workdir

    executable = Path(executable_path)
    input_path = Path(input_dir)
    output_path = Path(output_dir)
    workspace = Path(workspace_root)
    asset_base_path = Path(asset_base_dir)

    _require_existing_file(executable, label="MGB executable")
    _require_existing_directory(input_path, label="MGB Input directory")
    _ensure_directory(output_path)

    return CommandPlan(
        command=[str(executable)],
        working_directory=str(workspace),
        metadata={
            "run_id": run.run_id,
            "reference_time": run.reference_time,
            "workspace_root": str(workspace),
            "executable_path": str(executable),
            "input_dir": str(input_path),
            "output_dir": str(output_path),
            "asset_base_dir": str(asset_base_path),
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
    if logs_dir is None:
        raise ValueError("logs_dir is required when executing an MGB plan.")
    log_root = logs_dir
    log_path = log_root / script_stem() / f"{execution_id}.log"
    logger = configure_run_logger(log_path)
    plan.metadata["log_path"] = str(log_path)

    logger.info(
        "mgb_execution_started run_id=%s executable=%s workspace=%s",
        plan.metadata["run_id"],
        plan.metadata["executable_path"],
        plan.metadata["workspace_root"],
    )

    output_dir = _prepare_output_directory(plan, logger)
    process_env = os.environ.copy()
    process_env["MGB_INPUT_DIR"] = plan.metadata["input_dir"]
    process_env["MGB_OUTPUT_DIR"] = plan.metadata["output_dir"]

    try:
        process = subprocess.Popen(
            plan.command,
            cwd=plan.working_directory,
            env=process_env,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,
        )
    except OSError as exc:
        raise RuntimeError(
            f"Falha ao iniciar o executavel do MGB: {plan.metadata['executable_path']} | log={log_path}"
        ) from exc

    _stream_process_output(process, logger)
    return_code = process.wait()
    plan.metadata["return_code"] = return_code

    if return_code != 0:
        logger.error("mgb_execution_failed exit_code=%s", return_code)
        raise RuntimeError(f"Execucao do MGB falhou com exit code {return_code}. Log: {log_path}")

    if not _directory_has_files(output_dir):
        logger.error("mgb_execution_failed output_dir_empty=%s", output_dir)
        raise RuntimeError(f"Execucao do MGB terminou sem arquivos em Output. Log: {log_path}")

    asset_refs = _collect_output_asset_refs(output_dir, asset_base_dir=Path(plan.metadata["asset_base_dir"]))
    plan.metadata["status"] = "success"
    logger.info("mgb_execution_finished exit_code=%s files=%s", return_code, len(asset_refs))
    return ModelOutput(
        output_name="mgb_output",
        description="Execucao real do MGB concluida com sucesso.",
        asset_refs=asset_refs,
    )
