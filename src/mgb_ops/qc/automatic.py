from __future__ import annotations

from mgb_ops.common.models import QcFlag, RunMetadata


def apply_automatic_qc(run: RunMetadata) -> list[QcFlag]:
    """Run automatic QC rules before model execution.

    TODO:
    - implement checks by variable;
    - register flags in history and, when useful, in the operational run context;
    - promote data between `raw`, `curated`, and `approved`;
    - distinguish severities and blocking criteria before releasing model inputs.
    """
    raise NotImplementedError("Automatic QC is not implemented yet.")
