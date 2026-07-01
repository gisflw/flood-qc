from __future__ import annotations

from pathlib import Path

from mgb_ops.assets import databases as db_bootstrap


REPO_ROOT = Path(__file__).resolve().parents[1]
SQL_DIR = REPO_ROOT / "src" / "mgb_ops" / "assets" / "sql"
TEST_INVENTORY_CSV = REPO_ROOT / "tests" / "fixtures" / "history_station_inventory.csv"


def initialize_history_db(path: Path) -> Path:
    return db_bootstrap.initialize_history_db(path, TEST_INVENTORY_CSV, SQL_DIR / "history_schema.sql")


def initialize_run_db(run_id: str, path: Path) -> Path:
    return db_bootstrap.initialize_run_db(run_id, path, SQL_DIR / "run_schema.sql")
