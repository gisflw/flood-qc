from __future__ import annotations

from pathlib import Path
from typing import Any, Iterable, Mapping

from mgb_ops.qc.checks import check_correction_overlaps, check_correction_window
from mgb_ops.storage.history_repository import HistoryRepository


def list_forecast_corrections(database_path: Path, asset_id: str) -> list[dict[str, Any]]:
    with HistoryRepository(Path(database_path)) as repository:
        return repository.list_forecast_manual_edits(str(asset_id))


def replace_forecast_corrections(
    database_path: Path,
    asset_id: str,
    corrections: Iterable[Mapping[str, Any]],
) -> list[dict[str, Any]]:
    """Atomically replace persisted instructions without modifying source assets."""
    rows = [dict(row) for row in corrections]
    for row in rows:
        result = check_correction_window(int(row["t0_step"]), int(row["t1_step"]))
        if not result.passed:
            raise ValueError(result.message)
    overlap = check_correction_overlaps(rows)
    if not overlap.passed:
        raise ValueError(overlap.message)
    with HistoryRepository(Path(database_path)) as repository:
        return repository.replace_forecast_manual_edits(str(asset_id), rows)


list_corrections = list_forecast_corrections
replace_corrections = replace_forecast_corrections
