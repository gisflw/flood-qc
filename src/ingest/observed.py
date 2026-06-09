from __future__ import annotations

from common.models import RunMetadata, TimeSeriesRecord


def collect_observed_timeseries(run: RunMetadata) -> list[TimeSeriesRecord]:
    """Collect observed series for the given run.

    TODO:
    - implement real connectors for ANA, INMET, and other sources;
    - persist raw files in `data/interim/`;
    - register metadata and lineage in the history database.
    """
    raise NotImplementedError("Observation collection is not implemented yet.")
