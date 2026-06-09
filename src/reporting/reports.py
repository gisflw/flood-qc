from __future__ import annotations

from common.models import ReportArtifact, RunMetadata


def build_run_reports(run: RunMetadata) -> list[ReportArtifact]:
    """Generate report artifacts for the run.

    TODO:
    - consolidate operational summaries;
    - generate outputs in agreed formats;
    - register reports in the run database.
    """
    raise NotImplementedError("Report generation is not implemented yet.")
