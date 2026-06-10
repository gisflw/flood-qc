from __future__ import annotations

from pathlib import Path
import sys

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = REPO_ROOT / "src"
TEST_INVENTORY_CSV = REPO_ROOT / "tests" / "fixtures" / "history_station_inventory.csv"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))


@pytest.fixture(autouse=True)
def use_test_history_station_inventory(monkeypatch):
    from mgb_ops.storage import db_bootstrap

    monkeypatch.setattr(
        db_bootstrap,
        "history_station_inventory_csv_path",
        lambda: TEST_INVENTORY_CSV,
    )


@pytest.fixture(autouse=True)
def clear_mgb_ops_workspace_state():
    from mgb_ops.common.paths import clear_workspace

    clear_workspace()
    yield
    clear_workspace()
