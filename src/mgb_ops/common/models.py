from __future__ import annotations

from dataclasses import dataclass, field
from enum import Enum
from typing import Any


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
class RunLineage:
    run_id: str
    parent_run_id: str
    relation_type: str = "derived_from"


@dataclass(slots=True)
class StationRecord:
    station_code: str
    name: str
    provider_code: str
    latitude: float | None = None
    longitude: float | None = None
    altitude_m: int | None = None


@dataclass(slots=True)
class TimeSeriesRecord:
    series_id: str
    station_code: str
    variable: str
    unit: str
    state: DataState = DataState.RAW
    source_path: str | None = None


@dataclass(slots=True)
class QcFlag:
    scope: str
    reference: str
    rule_code: str
    severity: str
    message: str
    state: str = "open"


@dataclass(slots=True)
class ManualEdit:
    entity_type: str
    entity_id: str
    field_name: str
    old_value: str | None
    new_value: str | None
    reason: str
    editor: str | None = None


@dataclass(slots=True)
class RasterAsset:
    name: str
    relative_path: str
    format: str = "GeoTIFF"
    state: DataState = DataState.RAW
    crs: str | None = None


@dataclass(slots=True)
class ModelInput:
    input_name: str
    description: str
    source_refs: list[str] = field(default_factory=list)


@dataclass(slots=True)
class ModelOutput:
    output_name: str
    description: str
    asset_refs: list[str] = field(default_factory=list)


@dataclass(slots=True)
class ReportArtifact:
    name: str
    relative_path: str
    format: str
    description: str = ""


@dataclass(slots=True)
class CommandPlan:
    command: list[str]
    working_directory: str | None = None
    environment: dict[str, str] = field(default_factory=dict)
    metadata: dict[str, Any] = field(default_factory=dict)
