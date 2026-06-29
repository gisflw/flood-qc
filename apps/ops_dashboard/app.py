"""Thin ``panel serve`` entry point for the operations dashboard."""
from __future__ import annotations

import argparse
from pathlib import Path
import sys

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
for import_root in (REPO_ROOT, SRC_DIR):
    if str(import_root) not in sys.path:
        sys.path.insert(0, str(import_root))

import panel as pn

from apps.ops_dashboard.dashboard import create_dashboard
from mgb_ops.common.runtime import resolve_workspace_from_runtime_env


def _workspace_from_argv(argv: list[str]) -> Path:
    parser = argparse.ArgumentParser(add_help=False)
    parser.add_argument("--workspace")
    args, _ = parser.parse_known_args(argv)
    return resolve_workspace_from_runtime_env(workspace=args.workspace)


def main() -> None:
    workspace = _workspace_from_argv(sys.argv[1:])
    pn.serve(
        lambda: create_dashboard(workspace),
        title="Operational Hydrology",
        show=True,
    )


if pn.state.served:
    create_dashboard(_workspace_from_argv(sys.argv[1:])).servable()
elif __name__ == "__main__":
    main()


__all__ = ["create_dashboard", "main"]
