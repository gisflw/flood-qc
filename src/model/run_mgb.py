from __future__ import annotations

import argparse
import json
from datetime import datetime
from pathlib import Path
import sys

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

from common.models import RunMetadata
from model.mgb_execution import execute_mgb_plan, prepare_mgb_execution


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Windows-only MGB runner.")
    parser.add_argument("--dry-run", action="store_true", help="Do not execute the real binary.")
    return parser


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


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    metadata = build_run_metadata()
    plan = prepare_mgb_execution(metadata)
    result = execute_mgb_plan(plan, dry_run=args.dry_run)
    print(json.dumps(build_summary(plan, result, dry_run=args.dry_run), indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
