from __future__ import annotations

from common.models import ManualEdit, RunMetadata


def register_manual_review(run: RunMetadata, edits: list[ManualEdit]) -> None:
    """Register manual reviews for an already executed and materialized run.

    TODO:
    - persist the append-only log in the derived run database;
    - prevent in-place changes to the original automatic run;
    - optionally propagate approvals to history;
    - validate author, reason, and timestamps.
    """
    raise NotImplementedError("Manual review is not implemented yet.")
