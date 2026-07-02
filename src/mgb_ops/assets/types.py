"""Lightweight records shared across asset, workflow, and model boundaries."""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import Enum


class DataState(str, Enum):
    RAW = "raw"
    CURATED = "curated"
    APPROVED = "approved"


class RunKind(str, Enum):
    AUTOMATIC = "automatic"
    MANUAL = "manual"


class RunStatus(str, Enum):
    DRAFT = "draft"
    READY = "ready"
    EXECUTED = "executed"
    REVIEWED = "reviewed"
    PUBLISHED = "published"


@dataclass(slots=True)
class RunMetadata:
    run_id: str
    reference_time: str
    run_kind: RunKind = RunKind.AUTOMATIC
    status: RunStatus = RunStatus.DRAFT
    parent_run_id: str | None = None
    operator: str | None = None
    note: str | None = None


@dataclass(slots=True)
class TimeSeriesRecord:
    series_id: str
    station_code: str
    variable: str
    unit: str
    state: DataState = DataState.RAW
    source_path: str | None = None


@dataclass(slots=True)
class RasterAsset:
    name: str
    relative_path: str
    format: str = "GeoTIFF"
    state: DataState = DataState.RAW
    crs: str | None = None


@dataclass(frozen=True, slots=True)
class AnalysisWindow:
    start_time: datetime
    cutoff_time: datetime
    forecast_end_exclusive: datetime

    def cache_key(self) -> tuple[str, str, str]:
        return (
            self.start_time.isoformat(timespec="seconds"),
            self.cutoff_time.isoformat(timespec="seconds"),
            self.forecast_end_exclusive.isoformat(timespec="seconds"),
        )


__all__ = [
    "AnalysisWindow",
    "DataState",
    "RasterAsset",
    "RunKind",
    "RunMetadata",
    "RunStatus",
    "TimeSeriesRecord",
]
