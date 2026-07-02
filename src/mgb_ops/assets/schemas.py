"""Locations of schemas owned by the asset persistence layer."""

from pathlib import Path


SQL_DIR = Path(__file__).resolve().parent / "sql"
HISTORY_SCHEMA_PATH = SQL_DIR / "history_schema.sql"
RUN_SCHEMA_PATH = SQL_DIR / "run_schema.sql"

__all__ = ["HISTORY_SCHEMA_PATH", "RUN_SCHEMA_PATH", "SQL_DIR"]
