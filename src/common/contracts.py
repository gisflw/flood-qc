from __future__ import annotations

from typing import Iterable, Protocol

from common.models import (
    CommandPlan,
    ManualEdit,
    ModelInput,
    ModelOutput,
    QcFlag,
    RasterAsset,
    ReportArtifact,
    RunMetadata,
    TimeSeriesRecord,
)


class ObservationCollector(Protocol):
    """Contract for observed series collectors."""

    def collect(self, run: RunMetadata) -> Iterable[TimeSeriesRecord]:
        ...


class ForecastGridCollector(Protocol):
    """Contract for gridded forecast collectors."""

    def collect(self, run: RunMetadata) -> Iterable[RasterAsset]:
        ...


class AutomaticQcProcessor(Protocol):
    """Contract for automatic quality assessment before model execution."""

    def run(self, run: RunMetadata) -> Iterable[QcFlag]:
        ...


class ManualReviewService(Protocol):
    """Contract for registering manual adjustments on an already materialized run."""

    def apply(self, run: RunMetadata, edits: Iterable[ManualEdit]) -> None:
        ...


class RunAssembler(Protocol):
    """Contract for materializing the operational run from selected inputs and outputs."""

    def build(self, run: RunMetadata) -> Iterable[ModelInput]:
        ...


class ModelExecutor(Protocol):
    """Contract for preparing and executing the external model from input files."""

    def prepare(self, run: RunMetadata) -> CommandPlan:
        ...

    def execute(self, plan: CommandPlan, *, dry_run: bool = False) -> ModelOutput:
        ...


class PostProcessor(Protocol):
    """Contract for exporting complete output and preparing the operational run subset."""

    def process(self, run: RunMetadata) -> Iterable[ModelOutput]:
        ...


class ReportBuilder(Protocol):
    """Contract for report artifact generation."""

    def build(self, run: RunMetadata) -> Iterable[ReportArtifact]:
        ...
