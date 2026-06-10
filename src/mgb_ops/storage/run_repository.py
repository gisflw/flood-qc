from __future__ import annotations

from pathlib import Path


class RunRepository:
    """Future access layer for a run SQLite file.

    TODO:
    - register run lineage;
    - persist inputs, outputs, flags, and edits;
    - register associated assets and reports.
    """

    def __init__(self, database_path: Path) -> None:
        self.database_path = database_path

    def connect(self) -> None:
        raise NotImplementedError("Run repository is not implemented yet.")
