from __future__ import annotations

from pathlib import Path
import sys

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))


@pytest.fixture(autouse=True)
def clear_mgb_ops_workspace_state():
    from mgb_ops.common.paths import clear_workspace

    clear_workspace()
    yield
    clear_workspace()
