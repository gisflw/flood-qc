from __future__ import annotations

from datetime import datetime

from mgb_ops.common.models import RunMetadata


def build_run_id() -> str:
    return datetime.now().strftime("%Y%m%dT%H%M%S")


def build_run_metadata() -> RunMetadata:
    run_id = build_run_id()
    return RunMetadata(run_id=run_id, reference_time=run_id)


def build_summary(plan, result, *, dry_run: bool) -> dict[str, object]:
    return {
        "status": "dry_run" if dry_run else str(plan.metadata.get("status", "success")),
        "command": plan.command,
        "working_directory": plan.working_directory,
        "workspace_root": plan.metadata["workspace_root"],
        "local_input_dir": plan.metadata["local_input_dir"],
        "local_output_dir": plan.metadata["local_output_dir"],
        "remote_output_dir": plan.metadata["remote_output_dir"],
        "log_path": plan.metadata.get("log_path"),
        "description": result.description,
        "asset_refs": result.asset_refs,
    }
