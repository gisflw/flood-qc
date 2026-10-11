"""Session-local dashboard parameters and state transitions."""
from __future__ import annotations

import sqlite3
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import threading
import math
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any, Mapping

import pandas as pd
import param

from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.services import forecast as dashboard_forecast
from apps.ops_dashboard.i18n import translate
from apps.ops_dashboard.services.corrections import (
    build_forecast_edit_row,
    build_forecast_instruction_from_request,
    empty_forecast_edit_frame,
    normalize_forecast_edit_frame,
    validate_forecast_edit_draft,
)
from apps.ops_dashboard.services.loaders import (
    _accumulation_raster,
    _basin_precipitation,
    _basin_spatial_data,
    _forecast_assets,
    _forecast_preview,
    _forecast_steps,
    _mgb_series,
    _mini_segment_paths,
    _prepared_mgb_level,
    _model_variables,
    parse_signed_rainfall_period,
    _observed_series,
    _station_rainfall_accumulations,
    _station_reference_levels,
    _station_catalog,
    BasinSpatialData,
)
from mgb_ops.analysis import timeseries as dashboard_data
from mgb_ops.analysis.windows import build_analysis_window
from mgb_ops.assets.model_outputs import validate_model_outputs_netcdf
from mgb_ops.assets.scenario_cache import discover_latest_scenario_caches
from mgb_ops.assets.types import AnalysisWindow
from mgb_ops.assets.spatial_grid import RegularGridSpec
from mgb_ops.config.runtime import RuntimeContext, build_runtime_context
from mgb_ops.config.workspace import resolve_workspace_path
from mgb_ops.utils.time import resolve_reference_time
from mgb_ops.model.prepare_mgb_rainfall import MGB_OBSERVED_CACHE_FILENAME
from mgb_ops.assets.current_run import (
    CurrentRunArtifact, ForecastScenarioReference, ObservedReplacement, StationExclusion,
    create_current_artifact, load_current_artifact, save_current_artifact, update_current_artifact,
)
from mgb_ops.analysis.observations import load_effective_observations, load_preferred_rainfall_observations
from mgb_ops.analysis.precipitation import ObservationPolicyDraft, accumulated_leave_one_out_analysis
from mgb_ops.qc.precipitation import PrecipitationQCPolicy, detect_suspect_precipitation
from mgb_ops.workflows.scenario_orchestrator import execute_current_artifact
from mgb_ops.edit.forcing import ForecastCorrectionInstruction
from mgb_ops.workflows.forecast import list_enabled_forecast_providers


class CurrentRunArtifactError(ValueError):
    """Raised when dashboard startup cannot load a valid current-run artifact."""


@dataclass(frozen=True, slots=True)
class DashboardSources:
    history: str
    spatial: str
    model: str


class DashboardState(param.Parameterized):
    """All mutable dashboard state; one instance is created per Panel session."""

    language = param.Selector(default="en", objects={"English": "en", "Português": "pt-BR"})
    station_id = param.String(default=None, allow_None=True)
    mini_id = param.Integer(default=None, allow_None=True)
    selected_raster = param.Selector(default=None, objects=[None])
    rainfall_period = param.Integer(default=-24, bounds=(-999, 999))
    draft_basin_mini = param.String(default="")
    applied_basin_mini_id = param.Integer(default=None, allow_None=True)
    rainfall_mode = param.Selector(
        default="observed", objects=["observed", "forecast"], precedence=-1
    )
    rainfall_hours = param.Integer(default=24, bounds=(1, None), precedence=-1)
    forecast_rainfall_hours = param.Integer(default=24, bounds=(1, None), precedence=-1)
    summary_previous_hours = param.Integer(default=24, bounds=(1, None))
    summary_forecast_hours = param.Integer(default=24, bounds=(1, None))
    raster_opacity = param.Number(default=0.25, bounds=(0, 1), step=0.05)
    show_selected_basin = param.Boolean(default=False)
    raster_inspection = param.Parameter(default=None, allow_None=True)
    last_refresh_at = param.String(default="")
    source_versions = param.Dict(default={})
    warnings = param.List(default=[])
    message = param.String(default="")
    message_kind = param.Selector(
        default="info", objects=["info", "success", "warning", "danger"]
    )

    stations = param.DataFrame(precedence=-1)
    model_variables = param.DataFrame(precedence=-1)
    accumulation_rasters = param.List(default=[], precedence=-1)
    map_artifacts = param.Parameter(default=None, precedence=-1)

    forecast_assets = param.DataFrame(precedence=-1)
    forecast_asset_id = param.String(default=None, allow_None=True)
    forecast_steps = param.DataFrame(precedence=-1)
    forecast_t0_step = param.Integer(default=0, bounds=(0, None))
    forecast_t1_step = param.Integer(default=0, bounds=(0, None))
    forecast_shift_lat = param.Number(default=0)
    forecast_shift_lon = param.Number(default=0)
    forecast_rotation_deg = param.Number(default=0)
    forecast_multiplication_factor = param.Number(default=1, bounds=(0.000001, None))
    forecast_opacity = param.Number(default=0.7, bounds=(0, 1))
    applied_preview_request = param.Parameter(default=None, allow_None=True)
    forecast_preview = param.Parameter(default=None, allow_None=True)
    forecast_map_artifacts = param.Parameter(default=None, allow_None=True)
    forecast_view_state = param.Dict(default={})
    forecast_draft = param.DataFrame(default=empty_forecast_edit_frame())

    scenario_id = param.String(default=None, allow_None=True)
    comparison_scenario_ids = param.List(default=[])
    scenario_caches = param.List(default=[], precedence=-1)

    start_time = param.Date(default=None, allow_None=True)
    review_window_start = param.Date(default=None, allow_None=True)
    review_window_end = param.Date(default=None, allow_None=True)
    qc_threshold_mm = param.Number(default=50.0, bounds=(0, None))
    qc_sequence_min_length = param.Integer(default=5, bounds=(1, None))
    qc_sequence_lower_mm = param.Number(default=0.0, bounds=(0, None))
    qc_sequence_upper_mm = param.Number(default=1.0, bounds=(0, None))
    detected_flags = param.DataFrame(default=pd.DataFrame())
    observed_replacement_draft = param.DataFrame(default=pd.DataFrame(columns=["station_id", "observed_at", "value"]))
    station_exclusion_draft = param.DataFrame(default=pd.DataFrame(columns=["station_id", "start_time", "end_time"]))
    qc_selected_station = param.String(default=None, allow_None=True)
    qc_selected_sequence = param.String(default=None, allow_None=True)
    qc_diagnostic_result = param.Parameter(default=None, allow_None=True)
    qc_map_artifacts = param.Parameter(default=None, allow_None=True)
    responsible_person = param.String(default="")
    update_reason = param.String(default="")
    save_run_id = param.String(default="")
    save_run_description = param.String(default="")
    execution_active = param.Boolean(default=False)
    execution_progress = param.Integer(default=0, bounds=(0, 100))
    execution_status = param.String(default="idle")
    execution_error = param.String(default="")
    execution_generation = param.String(default="")
    reload_requested = param.Boolean(default=False)

    def __init__(
        self,
        workspace: str | Path | None = None,
        *,
        context: RuntimeContext | None = None,
        **params: Any,
    ) -> None:
        self.context = context or build_runtime_context(
            workspace=workspace, require_custom_settings=False
        )
        self.workspace = self.context.paths.workspace
        self._artifact_path = self.context.paths.current_run_db
        self._current_run_artifact = self._load_required_current_artifact()
        try:
            artifact_reference_time = resolve_reference_time(
                self._current_run_artifact.reference_time
            )
            artifact_mgb_settings = self._current_run_artifact.mgb_settings
            self.window = build_analysis_window(
                artifact_reference_time,
                observed_horizon_days=int(artifact_mgb_settings["observed_horizon_days"]),
                forecast_horizon_days=int(artifact_mgb_settings["forecast_horizon_days"]),
            )
            artifact_forecast_end = resolve_reference_time(
                self._current_run_artifact.forecast_end_exclusive
            )
        except (KeyError, TypeError, ValueError, OverflowError) as exc:
            raise CurrentRunArtifactError(
                f"Current-run artifact {self._artifact_path} has invalid timing or MGB horizon settings. "
                "Rebuild the current run artifact before opening the dashboard."
            ) from exc
        if artifact_forecast_end != self.window.forecast_end_exclusive:
            raise CurrentRunArtifactError(
                f"Current-run artifact {self._artifact_path} has inconsistent forecast timing. "
                "Rebuild the current run artifact before opening the dashboard."
            )
        self._runtime_reference_time = self.window.cutoff_time
        self._model_output_error: str | None = None
        self.history_path = self.context.paths.history_db
        try:
            initial_caches = list(
                discover_latest_scenario_caches(self.context.paths.cache_dir)
            )
            self._scenario_cache_error = None
        except (FileNotFoundError, OSError, ValueError) as exc:
            initial_caches = []
            self._scenario_cache_error = str(exc)
        initial_caches = self._filter_runtime_scenario_caches(initial_caches)
        self._scenario_cache_by_id = {item.scenario_id: item for item in initial_caches}
        initial = next((item for item in initial_caches if item.kind == "raw"), None)
        self.model_path = (
            initial.path
            if initial is not None
            else self.context.paths.processed_dir / "model_outputs.nc"
        )
        params.setdefault("scenario_caches", initial_caches)
        params.setdefault("scenario_id", initial.scenario_id if initial else None)
        params.setdefault("comparison_scenario_ids", [cache.scenario_id for cache in initial_caches])
        self.window = self._resolve_dashboard_window(self.window)
        params.setdefault("start_time", self.window.start_time)
        self.observed_precipitation_path = (
            self.context.paths.cache_dir / MGB_OBSERVED_CACHE_FILENAME
        )
        self.gpkg_path = resolve_workspace_path(
            self.workspace, self.context.settings["spatial"]["gpkg_path"]
        )
        default_hours = int(self.context.settings["summaries"]["accum_hours"][0])
        params.setdefault("rainfall_period", -default_hours)
        params.setdefault("rainfall_hours", default_hours)
        params.setdefault("forecast_rainfall_hours", default_hours)
        params.setdefault("summary_previous_hours", default_hours)
        params.setdefault("summary_forecast_hours", default_hours)
        restored_artifact = self._current_run_artifact
        review = restored_artifact.review_window
        try:
            params.setdefault("review_window_start", pd.Timestamp(review.get("start", self.window.start_time)).to_pydatetime())
            params.setdefault("review_window_end", pd.Timestamp(review.get("end", self.window.cutoff_time)).to_pydatetime())
            qc = PrecipitationQCPolicy.from_mapping(restored_artifact.precipitation_qc_settings)
        except (TypeError, ValueError, OverflowError) as exc:
            raise CurrentRunArtifactError(
                f"Current-run artifact {self._artifact_path} has invalid review or QC settings. "
                "Rebuild the current run artifact before opening the dashboard."
            ) from exc
        params.setdefault("qc_threshold_mm", qc.threshold_mm)
        params.setdefault("qc_sequence_min_length", qc.sequence_min_length)
        params.setdefault("qc_sequence_lower_mm", qc.sequence_lower_mm)
        params.setdefault("qc_sequence_upper_mm", qc.sequence_upper_mm)
        params.setdefault("observed_replacement_draft", pd.DataFrame([{"station_id": x.station_id, "observed_at": x.observed_at, "value": x.value} for x in restored_artifact.observed_replacements], columns=["station_id", "observed_at", "value"]))
        params.setdefault("station_exclusion_draft", pd.DataFrame([{"station_id": x.station_id, "start_time": x.start_time, "end_time": x.end_time} for x in restored_artifact.station_exclusions], columns=["station_id", "start_time", "end_time"]))
        params.setdefault("responsible_person", restored_artifact.responsible_person or "")
        params.setdefault("update_reason", restored_artifact.reason or "")
        params.setdefault("execution_status", restored_artifact.execution.status)
        params.setdefault("execution_error", restored_artifact.execution.error_message or "")
        params.setdefault("execution_generation", restored_artifact.execution.execution_id or "")
        super().__init__(**params)
        self._warning_records: list[tuple[str | None, dict[str, Any], str]] = []
        self._message_record: tuple[str | None, dict[str, Any], str] | None = None
        self.param.watch(self._update_localized_state_text, "language")
        self._execution_pool = ThreadPoolExecutor(max_workers=1, thread_name_prefix="observed-qc-refresh")
        self._execution_lock = threading.Lock()
        self.refresh()

    def _load_required_current_artifact(self) -> CurrentRunArtifact:
        if not self._artifact_path.is_file():
            raise CurrentRunArtifactError(
                f"Current-run artifact not found: {self._artifact_path}. "
                "Build the current run artifact before opening the dashboard."
            )
        try:
            return load_current_artifact(self._artifact_path)
        except (OSError, sqlite3.Error, RuntimeError, ValueError, KeyError, TypeError) as exc:
            raise CurrentRunArtifactError(
                f"Current-run artifact is invalid: {self._artifact_path} ({exc}). "
                "Rebuild the current run artifact before opening the dashboard."
            ) from exc

    @staticmethod
    def _normalize_runtime_time(value: datetime | pd.Timestamp) -> pd.Timestamp:
        timestamp = pd.Timestamp(value)
        if timestamp.tzinfo is not None:
            timestamp = timestamp.tz_convert("America/Sao_Paulo").tz_localize(None)
        return timestamp

    def _filter_runtime_scenario_caches(self, caches):
        expected = self._normalize_runtime_time(self._runtime_reference_time)
        return [
            cache
            for cache in caches
            if cache.reference_time is not None
            and self._normalize_runtime_time(cache.reference_time) == expected
        ]

    def _resolve_dashboard_window(self, artifact_window):
        if not self.model_path.exists():
            return artifact_window
        try:
            validate_model_outputs_netcdf(self.model_path, expected_window=artifact_window)
            self._model_output_error = None
        except (FileNotFoundError, OSError, ValueError) as exc:
            self._model_output_error = str(exc)
        return artifact_window

    def _refresh_scenario_caches(self) -> None:
        try:
            caches = self._filter_runtime_scenario_caches(
                discover_latest_scenario_caches(self.context.paths.cache_dir)
            )
            self._scenario_cache_error = None
        except (FileNotFoundError, OSError, ValueError) as exc:
            caches = []
            self._scenario_cache_error = str(exc)
        self._scenario_cache_by_id = {item.scenario_id: item for item in caches}
        valid_ids = set(self._scenario_cache_by_id)
        selected = self.scenario_id if self.scenario_id in valid_ids else None
        if selected is None:
            raw = next((item for item in caches if item.kind == "raw"), None)
            zero = next((item for item in caches if item.kind == "zero"), None)
            selected = (raw or zero).scenario_id if (raw or zero) else None
        self.scenario_caches = caches
        self.scenario_id = selected
        self.comparison_scenario_ids = [
            value for value in self.comparison_scenario_ids if value in valid_ids
        ]
        self.model_path = (
            self._scenario_cache_by_id[selected].path
            if selected is not None
            else self.context.paths.processed_dir / "model_outputs.nc"
        )
        if self.model_path.is_file():
            try:
                validate_model_outputs_netcdf(self.model_path, expected_window=self.window)
                self._model_output_error = None
            except (OSError, ValueError) as exc:
                self._model_output_error = str(exc)

    def select_scenario(self, scenario_id: str) -> None:
        if scenario_id not in self._scenario_cache_by_id:
            raise ValueError(f"Unknown forecast scenario: {scenario_id}")
        self.scenario_id = scenario_id
        self.model_path = self._scenario_cache_by_id[scenario_id].path
        self.source_versions = {
            **self.source_versions,
            "model": self._published_version(self.model_path),
        }
        try:
            validate_model_outputs_netcdf(self.model_path, expected_window=self.window)
            self._model_output_error = None
        except (OSError, ValueError) as exc:
            self._model_output_error = str(exc)
            self.add_warning("", translation_key="warning.model_outputs", translation_args={"detail": self._model_output_error})
            self.model_variables = dashboard_data.list_model_variables()
            return
        self.model_variables = _model_variables(
            str(self.model_path), str(self.workspace),
            self.source_versions["model"], self.window,
        )

    def _published_version(self, path: Path) -> str:
        return f"{dashboard_map.build_file_version(path)}|generation={self.execution_generation or 'none'}"

    def _versions(self) -> DashboardSources:
        return DashboardSources(
            history=dashboard_map.build_sqlite_version(self.history_path),
            spatial=dashboard_map.build_file_version(self.gpkg_path),
            model=self._published_version(self.model_path),
        )

    def _update_localized_state_text(self, *_: Any) -> None:
        self.warnings = [self._render_translation(key, args, raw) for key, args, raw in self._warning_records]
        if self._message_record is not None:
            key, args, raw = self._message_record
            self.message = self._render_translation(key, args, raw)

    def _render_translation(self, key: str | None, args: dict[str, Any], raw: str) -> str:
        if key is None:
            return raw
        values = dict(args)
        if "operation_key" in values:
            values["operation"] = translate(self.language, values.pop("operation_key"))
        if "mode_key" in values:
            values["mode"] = translate(self.language, values.pop("mode_key"))
        return translate(self.language, key, **values)

    def clear_warnings(self) -> None:
        self._warning_records = []
        self.warnings = []

    def add_warning(
        self,
        message: str,
        *,
        translation_key: str | None = None,
        translation_args: dict[str, Any] | None = None,
    ) -> None:
        args = dict(translation_args or {})
        rendered = self._render_translation(translation_key, args, message)
        if rendered not in self.warnings:
            self._warning_records.append((translation_key, args, message))
            self.warnings = [*self.warnings, rendered]

    def warning_for_display(self, message: str) -> str:
        for current, (key, args, raw) in zip(self.warnings, self._warning_records):
            if current == message:
                return self._render_translation(key, args, raw) if key else translate(self.language, "message.external_detail", detail=raw)
        return message

    def message_for_display(self) -> str:
        if self._message_record is None:
            return self.message
        key, args, raw = self._message_record
        return self._render_translation(key, args, raw) if key else translate(self.language, "message.external_detail", detail=raw)

    def refresh(self) -> None:
        """Re-version sources and refresh only this state/session."""
        self._refresh_scenario_caches()
        versions = self._versions()
        self.source_versions = {
            "history": versions.history,
            "spatial": versions.spatial,
            "model": versions.model,
        }
        self.clear_warnings()
        if self._scenario_cache_error:
            self.add_warning("", translation_key="warning.scenario_cache", translation_args={"detail": self._scenario_cache_error})
        if self._model_output_error:
            self.add_warning("", translation_key="warning.model_outputs", translation_args={"detail": self._model_output_error})
        workspace = str(self.workspace)
        if self.history_path.exists():
            try:
                self.stations = _station_catalog(
                    str(self.history_path),
                    workspace,
                    versions.history,
                    self.window,
                )
            except (sqlite3.Error, pd.errors.DatabaseError, RuntimeError, ValueError) as exc:
                self.stations = pd.DataFrame()
                self.add_warning("", translation_key="warning.observed_database", translation_args={"detail": str(exc)})
        else:
            self.stations = pd.DataFrame()
            self.add_warning("", translation_key="warning.history_missing", translation_args={"path": self.history_path})
        self._update_station_rainfall_labels()

        segments = None
        try:
            segments = _mini_segment_paths(
                str(self.gpkg_path), workspace, versions.spatial
            )
        except (FileNotFoundError, ValueError) as exc:
            self.add_warning("", translation_key="warning.mini_layers", translation_args={"detail": str(exc)})

        try:
            self.model_variables = _model_variables(
                str(self.model_path), workspace, versions.model, self.window
            )
        except (FileNotFoundError, OSError, ValueError) as exc:
            self.model_variables = dashboard_data.list_model_variables()
            self.add_warning("", translation_key="warning.model_variables", translation_args={"detail": str(exc)})

        self.accumulation_rasters = []
        bbox = self.context.settings["spatial_grid"]["bbox"]
        if bbox is not None:
            try:
                self.accumulation_rasters = [
                    _accumulation_raster(
                        str(self._rainfall_cache_path()),
                        workspace,
                        self._published_version(self._rainfall_cache_path()),
                        self.window,
                        tuple(float(value) for value in bbox),
                        float(self.context.settings["spatial_grid"]["resolution_degrees"]),
                        self._selected_rainfall_hours(),
                        rainfall_mode=self._selected_rainfall_mode(),
                    )
                ]
            except (
                FileNotFoundError,
                sqlite3.Error,
                pd.errors.DatabaseError,
                ValueError,
            ) as exc:
                self.add_warning("", translation_key="warning.rainfall_maps", translation_args={"mode_key": f"rainfall.mode.{self._selected_rainfall_mode()}", "detail": str(exc)})
        elif bbox is None:
            self.add_warning("", translation_key="warning.configure_bbox")

        raster_names = [str(item["name"]) for item in self.accumulation_rasters]
        self.param.selected_raster.objects = [None, *raster_names]
        if self.selected_raster not in raster_names:
            self.selected_raster = raster_names[0] if raster_names else None
        self._rebuild_map(segments=segments)
        self._refresh_forecast_assets()
        self.last_refresh_at = datetime.now().strftime("%d/%m/%Y %H:%M:%S")

    def apply_rainfall_hours(self) -> None:
        """Load the requested rainfall accumulation period from legacy controls."""
        mode = str(self.rainfall_mode)
        hours = int(self.forecast_rainfall_hours if mode == "forecast" else self.rainfall_hours)
        self.rainfall_period = hours if mode == "forecast" else -hours
        before_rasters = self.accumulation_rasters
        before_selection = self.selected_raster
        before_warnings = list(self.warnings)
        before_records = list(self._warning_records)
        self.apply_map_configuration(apply_basin=False)
        if self.accumulation_rasters is before_rasters and self.selected_raster == before_selection:
            new_records = [value for value in self._warning_records if value not in before_records]
            if new_records:
                detail = new_records[-1][1].get("detail", new_records[-1][2])
                self._warning_records = before_records
                self.warnings = before_warnings
                self.add_warning("", translation_key="warning.rainfall_maps", translation_args={"mode_key": f"rainfall.mode.{mode}", "detail": detail})

    def apply_map_configuration(self, *, apply_basin: bool = True) -> None:
        """Atomically apply rainfall and optional basin-boundary map settings."""
        bbox = self.context.settings["spatial_grid"].get("bbox")
        if bbox is None:
            self.add_warning("", translation_key="warning.rainfall_unavailable")
            return
        try:
            rainfall_mode, rainfall_hours = parse_signed_rainfall_period(int(self.rainfall_period))
            cache_path = self._rainfall_cache_path(rainfall_mode)
            raster = _accumulation_raster(
                str(cache_path), str(self.workspace),
                self._published_version(cache_path), self.window,
                tuple(float(value) for value in bbox),
                float(self.context.settings["spatial_grid"]["resolution_degrees"]),
                rainfall_hours,
                rainfall_mode=rainfall_mode,
            )
            next_basin_mini = self.applied_basin_mini_id
            if apply_basin:
                next_basin_mini = self._parse_draft_basin_mini()
                if next_basin_mini is not None:
                    _basin_spatial_data(
                        next_basin_mini,
                        str(self.gpkg_path),
                        str(self.workspace),
                        self.source_versions.get("spatial", ""),
                    )
        except (FileNotFoundError, sqlite3.Error, pd.errors.DatabaseError, ValueError) as exc:
            self.add_warning("", translation_key="warning.map_configuration", translation_args={"detail": str(exc)})
            return
        raster_name = str(raster["name"])
        with param.parameterized.discard_events(self):
            self.rainfall_mode = rainfall_mode
            self.rainfall_hours = rainfall_hours if rainfall_mode == "observed" else self.rainfall_hours
            self.forecast_rainfall_hours = rainfall_hours if rainfall_mode == "forecast" else self.forecast_rainfall_hours
            self.accumulation_rasters = [raster]
            self.param.selected_raster.objects = [None, raster_name]
            self.selected_raster = raster_name
            if apply_basin:
                self.applied_basin_mini_id = next_basin_mini
        self._update_station_rainfall_labels()
        current_map = self.map_artifacts
        with param.parameterized.discard_events(self):
            self._rebuild_map()
            replacement_map = self.map_artifacts
            self.map_artifacts = current_map
        self.map_artifacts = replacement_map

    def _selected_rainfall_mode(self) -> str:
        return parse_signed_rainfall_period(int(self.rainfall_period))[0]

    def _rainfall_cache_path(self, rainfall_mode: str | None = None) -> Path:
        mode = rainfall_mode or self._selected_rainfall_mode()
        if mode == "observed":
            return self.observed_precipitation_path
        scenario = self._scenario_cache_by_id.get(self.scenario_id or "")
        if scenario is None or scenario.forecast_grid_path is None:
            label = self.scenario_label(self.scenario_id or "selected scenario")
            raise ValueError(
                f"Forecast rainfall map unavailable for {label}: this scenario has no persisted forecast grid."
            )
        return scenario.forecast_grid_path

    def _selected_rainfall_hours(self) -> int:
        return parse_signed_rainfall_period(int(self.rainfall_period))[1]

    def _update_station_rainfall_labels(self) -> None:
        """Attach selected observed-period totals for the operation-map label layer."""
        if self.stations.empty:
            return
        base = self.stations.drop(columns=["rainfall_mm"], errors="ignore").copy()
        if self._selected_rainfall_mode() != "observed" or not self.history_path.exists():
            self.stations = base
            return
        end_time = self.window.cutoff_time
        start_time = end_time - pd.Timedelta(hours=self._selected_rainfall_hours())
        try:
            accumulations = _station_rainfall_accumulations(
                str(self.history_path),
                str(self.workspace),
                self.source_versions.get("history", ""),
                start_time.to_pydatetime() if isinstance(start_time, pd.Timestamp) else start_time,
                end_time,
            )
        except (sqlite3.Error, pd.errors.DatabaseError, RuntimeError, ValueError) as exc:
            self.add_warning("", translation_key="warning.station_rainfall", translation_args={"detail": str(exc)})
            self.stations = base
            return
        self.stations = base.merge(accumulations, on="station_id", how="left")

    def _parse_draft_basin_mini(self) -> int | None:
        value = str(self.draft_basin_mini or "").strip()
        if not value:
            return None
        try:
            mini_id = int(value)
        except ValueError as exc:
            raise ValueError("Basin mini must be an integer mini_id or empty.") from exc
        if mini_id < 1:
            raise ValueError("Basin mini must be a positive integer mini_id or empty.")
        return mini_id

    @param.depends("selected_raster", watch=True)
    def _rebuild_map(
        self,
        *_: Any,
        segments: pd.DataFrame | None = None,
    ) -> None:
        if segments is None:
            try:
                segments = _mini_segment_paths(
                    str(self.gpkg_path),
                    str(self.workspace),
                    self.source_versions.get("spatial", ""),
                )
            except (FileNotFoundError, ValueError):
                segments = None
        catalog = {
            str(item["name"]): item for item in self.accumulation_rasters
        }
        basin_geojson = None
        if self.applied_basin_mini_id is not None:
            try:
                basin_geojson = self.basin_spatial_data(self.applied_basin_mini_id).geometry.__geo_interface__
            except (FileNotFoundError, TypeError, ValueError) as exc:
                self.add_warning("", translation_key="warning.basin", translation_args={"detail": str(exc)})
        self.map_artifacts = dashboard_map.build_ops_map(
            self.selected_raster,
            self.raster_opacity,
            self.stations,
            segments,
            catalog,
            basin_geojson,
        )

    @param.depends("applied_basin_mini_id", watch=True)
    def _rebuild_selected_basin(self) -> None:
        self._rebuild_map()

    def handle_map_click(self, click_state: Mapping[str, Any] | None) -> None:
        selection = dashboard_map.decode_click_state(
            click_state,
            self.map_artifacts.pick_lookups if self.map_artifacts else {},
        )
        if selection.station_id is not None:
            self.station_id = selection.station_id
        if selection.mini_id is not None:
            self.mini_id = selection.mini_id
            self.draft_basin_mini = str(selection.mini_id)
        self.raster_inspection = dashboard_map.inspect_raster_click(
            click_state,
            self.map_artifacts.raster_lookups if self.map_artifacts else {},
        )

    def _chart_window(self) -> AnalysisWindow:
        start = pd.Timestamp(self.start_time or self.window.start_time).to_pydatetime()
        if start > self.window.cutoff_time:
            raise ValueError("Dashboard start_time must not be after reference_time.")
        return AnalysisWindow(start, self.window.cutoff_time, self.window.forecast_end_exclusive)

    def observed_series(self) -> pd.DataFrame:
        if self.station_id is None or not self.history_path.exists():
            return pd.DataFrame()
        return _observed_series(
            self.station_id, str(self.history_path), str(self.workspace),
            self.source_versions.get("history", ""), self.window,
        )

    def chart_observed_series(self) -> pd.DataFrame:
        if self.station_id is None or not self.history_path.exists():
            return pd.DataFrame()
        return _observed_series(
            self.station_id, str(self.history_path), str(self.workspace),
            self.source_versions.get("history", ""), self._chart_window(),
        )
    def station_reference_levels(self) -> pd.DataFrame:
        if self.station_id is None or not self.history_path.exists():
            return pd.DataFrame(columns=["reference_type", "level_cm", "event_date"])
        return _station_reference_levels(
            self.station_id,
            str(self.history_path),
            str(self.workspace),
            self.source_versions.get("history", ""),
        )


    def _scenario_path(self, scenario_id: str | None = None) -> Path:
        selected = scenario_id or self.scenario_id
        if selected and selected in self._scenario_cache_by_id:
            return self._scenario_cache_by_id[selected].path
        return self.model_path

    def scenario_label(self, scenario_id: str) -> str:
        cache = self._scenario_cache_by_id.get(scenario_id)
        label = cache.label if cache is not None else scenario_id
        label = label.split(" - ", maxsplit=1)[0]
        if cache is None:
            return label
        if cache.kind == "zero":
            return translate(self.language, "forecast.scenario_zero")
        if cache.kind in {"raw", "corrected"} and cache.provider_code:
            return translate(self.language, f"forecast.scenario_{cache.kind}", provider=cache.provider_code.upper())
        return label

    def mgb_series(self, variable_code: str, scenario_id: str | None = None) -> pd.DataFrame:
        if self.mini_id is None:
            return pd.DataFrame()
        model_path = self._scenario_path(scenario_id)
        model_version = self._published_version(model_path)
        if variable_code == "level":
            return _prepared_mgb_level(
                self.mini_id,
                str(model_path),
                str(self.workspace),
                model_version,
                str(self.history_path),
                self._chart_window(),
            )
        return _mgb_series(
            self.mini_id,
            variable_code,
            str(model_path),
            str(self.workspace),
            model_version,
            self._chart_window(),
        )

    def basin_spatial_data(self, mini_id: int | None = None) -> BasinSpatialData:
        selected_mini = self.mini_id if mini_id is None else mini_id
        if selected_mini is None:
            raise ValueError("Select a mini before loading its basin.")
        return _basin_spatial_data(
            int(selected_mini),
            str(self.gpkg_path),
            str(self.workspace),
            self.source_versions.get("spatial", ""),
        )

    def _load_basin_precipitation(self, scenario_id: str | None, window: AnalysisWindow) -> pd.DataFrame:
        if self.mini_id is None:
            return pd.DataFrame()
        basin = self.basin_spatial_data()
        return _basin_precipitation(
            basin.mini_ids, basin.weights, str(self._scenario_path(scenario_id)),
            str(self.workspace), self._published_version(self._scenario_path(scenario_id)), window,
        )

    def basin_precipitation(self, scenario_id: str | None = None) -> pd.DataFrame:
        return self._load_basin_precipitation(scenario_id, self.window)

    def chart_basin_precipitation(self, scenario_id: str | None = None) -> pd.DataFrame:
        return self._load_basin_precipitation(scenario_id, self._chart_window())

    def comparison_model_series(self) -> dict[str, dict[str, pd.DataFrame]]:
        result: dict[str, dict[str, pd.DataFrame]] = {}
        for scenario_id in self.comparison_scenario_ids:
            try:
                result[self.scenario_label(scenario_id)] = {
                    "precipitation": self.chart_basin_precipitation(scenario_id),
                    "level": self.mgb_series("level", scenario_id),
                    "flow": self.mgb_series("flow", scenario_id),
                }
            except (FileNotFoundError, OSError, TypeError, ValueError):
                continue
        return result

    def _analysis_grid(self) -> RegularGridSpec:
        bbox = self.context.settings["spatial_grid"]["bbox"]
        if bbox is None:
            raise ValueError(
                "Set spatial_grid.bbox in <workspace>/config/custom.yaml to enable forecast maps."
            )
        return RegularGridSpec(
            bbox=tuple(float(value) for value in bbox),
            resolution=float(
                self.context.settings["spatial_grid"]["resolution_degrees"]
            ),
        )

    def _refresh_forecast_assets(self) -> None:
        self.forecast_assets = pd.DataFrame()
        self.forecast_steps = pd.DataFrame()
        if not self.history_path.exists():
            return
        frames: list[pd.DataFrame] = []
        try:
            providers = list_enabled_forecast_providers(self.history_path)
        except (FileNotFoundError, sqlite3.Error, RuntimeError, ValueError) as exc:
            self.add_warning("", translation_key="warning.forecast_providers", translation_args={"detail": str(exc)})
            return
        for provider in providers:
            try:
                frame = _forecast_assets(
                    str(self.history_path),
                    str(self.workspace),
                    self.source_versions.get("history", ""),
                    self.window,
                    provider,
                    int(self.context.settings["forecast"].get("lookback_cycles", 1)),
                )
                if not frame.empty:
                    frames.append(frame)
            except (
                FileNotFoundError,
                sqlite3.Error,
                pd.errors.DatabaseError,
                RuntimeError,
                ValueError,
            ) as exc:
                self.add_warning("", translation_key="warning.forecast_assets", translation_args={"provider": provider.upper(), "detail": str(exc)})
        self.forecast_assets = (
            pd.concat(frames, ignore_index=True)
            .sort_values(["provider_code", "asset_id"])
            .reset_index(drop=True)
            if frames
            else pd.DataFrame()
        )
        if self.forecast_assets.empty:
            return
        options = self.forecast_assets["asset_id"].astype(str).tolist()
        if self.forecast_asset_id not in options:
            self.forecast_asset_id = options[0]
        self.select_forecast_asset(self.forecast_asset_id)

    def select_forecast_asset(self, asset_id: str) -> None:
        self.forecast_asset_id = str(asset_id)
        try:
            self.forecast_steps = _forecast_steps(
                self.forecast_asset_id,
                str(self.history_path),
                str(self.workspace),
                self.source_versions.get("history", ""),
                self.window,
            )
        except (FileNotFoundError, ModuleNotFoundError, ValueError) as exc:
            self.forecast_steps = pd.DataFrame()
            self.set_message("", "warning", translation_key="message.operation_failed", translation_args={"operation_key": "message.operation_forecast_asset", "detail": str(exc)})
            return
        if not self.forecast_steps.empty:
            steps = self.forecast_steps["step_hours"].astype(int).tolist()
            self.forecast_t0_step, self.forecast_t1_step = steps[0], steps[-1]
        self.load_forecast_draft()

    def apply_preview(self) -> dashboard_forecast.ForecastPreviewRequest:
        if not self.forecast_asset_id:
            raise ValueError("Select a forecast asset first.")
        request = dashboard_forecast.ForecastPreviewRequest(
            asset_id=self.forecast_asset_id,
            t0_step=int(self.forecast_t0_step),
            t1_step=int(self.forecast_t1_step),
            shift_lat=float(self.forecast_shift_lat),
            shift_lon=float(self.forecast_shift_lon),
            rotation_deg=float(self.forecast_rotation_deg),
            multiplication_factor=float(self.forecast_multiplication_factor),
            opacity=float(self.forecast_opacity),
        )
        if request.t1_step < request.t0_step:
            raise ValueError("t1_step must be >= t0_step.")
        asset_version = ""
        if not self.forecast_assets.empty and "asset_path" in self.forecast_assets:
            selected = self.forecast_assets[
                self.forecast_assets["asset_id"].astype(str) == request.asset_id
            ]
            if not selected.empty:
                asset_version = dashboard_map.build_file_version(
                    Path(selected.iloc[0]["asset_path"])
                )
        preview = _forecast_preview(
            request.asset_id,
            request.t0_step,
            request.t1_step,
            str(self.history_path),
            str(self.workspace),
            f"{self.source_versions.get('history', '')}|{asset_version}",
            self.window,
        )
        corrected = (
            dashboard_forecast.apply_preview_corrections(
                preview, [build_forecast_instruction_from_request(request)]
            )
            if request.has_correction
            else None
        )
        self.applied_preview_request = request
        self.forecast_preview = preview
        self.forecast_map_artifacts = dashboard_forecast.build_forecast_map_artifacts(
            preview,
            corrected_preview=corrected,
            opacity=request.opacity,
            view_state=self.forecast_view_state or None,
        )
        self.forecast_view_state = dict(
            self.forecast_map_artifacts.view_state or {}
        )
        return request

    def update_forecast_view(self, view_state: Mapping[str, Any] | None) -> None:
        self.forecast_view_state = dashboard_forecast.synchronize_view_state(
            view_state, self.forecast_view_state
        )

    def load_forecast_draft(self) -> pd.DataFrame:
        if not self.forecast_asset_id or not self.context.paths.current_run_db.exists():
            self.forecast_draft = empty_forecast_edit_frame()
            return self.forecast_draft
        artifact = load_current_artifact(self.context.paths.current_run_db)
        rows = []
        for scenario in artifact.scenarios:
            if scenario.source_asset_id == self.forecast_asset_id and scenario.kind == "corrected":
                for index, correction in enumerate(scenario.corrections):
                    rows.append({"correction_id": index + 1, "asset_id": self.forecast_asset_id, "t0_step": correction.t0_step, "t1_step": correction.t1_step, "shift_lat": correction.shift_lat, "shift_lon": correction.shift_lon, "rotation_deg": correction.rotation_deg, "multiplication_factor": correction.multiplication_factor, "metadata_json": "{}", "remove": False})
        self.forecast_draft = normalize_forecast_edit_frame(pd.DataFrame(rows))
        return self.forecast_draft

    def update_forecast_draft(self, frame: pd.DataFrame) -> None:
        draft = normalize_forecast_edit_frame(frame)
        if self.forecast_asset_id: draft["asset_id"] = self.forecast_asset_id
        self.forecast_draft = draft

    def add_forecast_correction(self, **values: Any) -> None:
        if not self.forecast_asset_id: raise ValueError("Select a forecast asset first.")
        row = build_forecast_edit_row(asset_id=self.forecast_asset_id, t0_step=int(values.get("t0_step", self.forecast_t0_step)), t1_step=int(values.get("t1_step", self.forecast_t1_step)), shift_lat=float(values.get("shift_lat", self.forecast_shift_lat)), shift_lon=float(values.get("shift_lon", self.forecast_shift_lon)), rotation_deg=float(values.get("rotation_deg", self.forecast_rotation_deg)), multiplication_factor=float(values.get("multiplication_factor", self.forecast_multiplication_factor)), metadata=values.get("metadata"))
        self.update_forecast_draft(pd.concat([self.forecast_draft, pd.DataFrame([row])], ignore_index=True))
        self.set_message("", "success", translation_key="message.correction_added")

    def _qc_policy(self) -> PrecipitationQCPolicy:
        return PrecipitationQCPolicy(
            threshold_mm=float(self.qc_threshold_mm),
            sequence_min_length=int(self.qc_sequence_min_length),
            sequence_lower_mm=float(self.qc_sequence_lower_mm),
            sequence_upper_mm=float(self.qc_sequence_upper_mm),
        ).validate()

    def _replacement_instructions(self) -> tuple[ObservedReplacement, ...]:
        instructions = []
        for row in self.observed_replacement_draft.itertuples(index=False):
            value = None if pd.isna(row.value) else float(row.value)
            instructions.append(ObservedReplacement(str(row.station_id), pd.Timestamp(row.observed_at).isoformat(), value))
        return tuple(instructions)

    def _exclusion_instructions(self) -> tuple[StationExclusion, ...]:
        return tuple(StationExclusion(str(row.station_id), pd.Timestamp(row.start_time).isoformat(), pd.Timestamp(row.end_time).isoformat()) for row in self.station_exclusion_draft.itertuples(index=False))

    def scan_observed_precipitation(self) -> pd.DataFrame:
        """Scan into this session only; never write flag labels or instructions."""
        if not self.history_path.exists():
            raise FileNotFoundError(f"History database not found: {self.history_path}")
        start, end = pd.Timestamp(self.review_window_start), pd.Timestamp(self.review_window_end)
        raw = load_preferred_rainfall_observations(
            self.history_path, start_time=start.to_pydatetime(), end_time=end.to_pydatetime(),
        )
        result = detect_suspect_precipitation(raw, timestep_hours=int(self.context.settings["run"]["timestep_hours"]), policy=self._qc_policy())
        effective = load_effective_observations(
            self.history_path, start_time=start.to_pydatetime(), end_time=end.to_pydatetime(),
            timestep_hours=int(self.context.settings["run"]["timestep_hours"]),
            replacements=self._replacement_instructions(), exclusions=self._exclusion_instructions(),
            require_complete_source=False,
        )
        lookup = {(str(row.station_id), pd.Timestamp(row.observed_at)): row for row in effective.itertuples(index=False)}
        rows = []
        for match in result.matches:
            key = (match.station_id, pd.Timestamp(match.observed_at))
            current = lookup.get(key)
            rows.append({
                "station_id": match.station_id, "observed_at": pd.Timestamp(match.observed_at),
                "flag_type": match.match_type, "match_type": match.match_type,
                "sequence_id": match.sequence_id, "raw_value": match.value,
                "effective_value": getattr(current, "value", match.value),
                "replacement_state": getattr(current, "replacement_state", "raw"),
                "excluded": bool(getattr(current, "excluded", False)),
            })
        self.detected_flags = pd.DataFrame(rows, columns=[
            "station_id", "observed_at", "flag_type", "match_type", "sequence_id",
            "raw_value", "effective_value", "replacement_state", "excluded",
        ])
        self.qc_map_artifacts = dashboard_map.build_qc_station_map(
            self.stations, self.detected_flags, self.observed_replacement_draft,
            self.station_exclusion_draft, selected_station=self.qc_selected_station,
        )
        self.set_message("", "success", translation_key="message.qc_detected", translation_args={"count": len(self.detected_flags)})
        return self.detected_flags

    scan_precipitation_qc = scan_observed_precipitation

    def set_observed_replacement(self, station_id: str, observed_at: datetime | str, value: float | None) -> None:
        if value is not None and (not pd.notna(value) or not math.isfinite(float(value)) or float(value) < 0):
            raise ValueError("Replacement must be NULL or a finite non-negative value.")
        timestamp = pd.Timestamp(observed_at).isoformat()
        frame = self.observed_replacement_draft.copy()
        if frame.empty:
            frame = pd.DataFrame(columns=["station_id", "observed_at", "value"])
        mask = (frame["station_id"].astype(str) == str(station_id)) & (pd.to_datetime(frame["observed_at"]) == pd.Timestamp(timestamp)) if not frame.empty else pd.Series(dtype=bool)
        row = {"station_id": str(station_id), "observed_at": timestamp, "value": value}
        if len(mask) and mask.any():
            frame.loc[mask, ["station_id", "observed_at", "value"]] = [str(station_id), timestamp, value]
        else:
            frame = pd.concat([frame, pd.DataFrame([row])], ignore_index=True)
        self.observed_replacement_draft = frame.sort_values(["station_id", "observed_at"]).reset_index(drop=True)

    apply_observed_replacement = set_observed_replacement

    def remove_observed_replacement(self, station_id: str, observed_at: datetime | str) -> None:
        frame = self.observed_replacement_draft.copy()
        if frame.empty:
            return
        mask = (frame["station_id"].astype(str) == str(station_id)) & (pd.to_datetime(frame["observed_at"]) == pd.Timestamp(observed_at))
        self.observed_replacement_draft = frame.loc[~mask].reset_index(drop=True)

    remove_observed_instruction = remove_observed_replacement

    def replace_selected_sequence(self, sequence_id: str | None = None, value: float | None = 0.0) -> int:
        selected = sequence_id or self.qc_selected_sequence
        matches = self.detected_flags[self.detected_flags["sequence_id"].astype(str) == str(selected)] if not self.detected_flags.empty else pd.DataFrame()
        if matches.empty:
            raise ValueError("Select a detected sequence first.")
        for row in matches.itertuples(index=False):
            self.set_observed_replacement(row.station_id, row.observed_at, value)
        return len(matches)

    def exclude_station(self, station_id: str | None = None) -> None:
        selected = station_id or self.qc_selected_station
        if not selected:
            raise ValueError("Select a station first.")
        self.include_station(selected)
        row = {"station_id": str(selected), "start_time": pd.Timestamp(self.review_window_start).isoformat(), "end_time": pd.Timestamp(self.review_window_end).isoformat()}
        self.station_exclusion_draft = pd.concat([self.station_exclusion_draft, pd.DataFrame([row])], ignore_index=True)

    def include_station(self, station_id: str | None = None) -> None:
        selected = station_id or self.qc_selected_station
        if not selected or self.station_exclusion_draft.empty:
            return
        self.station_exclusion_draft = self.station_exclusion_draft[self.station_exclusion_draft["station_id"].astype(str) != str(selected)].reset_index(drop=True)

    def run_leave_one_out(self, station_id: str | None = None):
        selected = station_id or self.qc_selected_station
        if not selected:
            raise ValueError("Select a station for leave-one-out analysis.")
        self.qc_diagnostic_result = accumulated_leave_one_out_analysis(
            self.history_path, start_time=pd.Timestamp(self.review_window_start).to_pydatetime(),
            end_time=pd.Timestamp(self.review_window_end).to_pydatetime(),
            policy_draft=ObservationPolicyDraft(self._replacement_instructions(), self._exclusion_instructions()),
            selected_station=str(selected), grid=self._analysis_grid(),
            nearest_stations=int(self.context.settings["rainfall_interpolation"]["nearest_stations"]),
            power=float(self.context.settings["rainfall_interpolation"]["power"]),
            timestep_hours=int(self.context.settings["run"]["timestep_hours"]),
        )
        return self.qc_diagnostic_result

    def _scenarios_with_forecast_draft(self, artifact: CurrentRunArtifact) -> tuple[ForecastScenarioReference, ...]:
        if not self.forecast_asset_id:
            return artifact.scenarios
        rows = validate_forecast_edit_draft(self.forecast_asset_id, self.forecast_draft)
        scenarios = [item for item in artifact.scenarios if not (item.kind == "corrected" and item.source_asset_id == self.forecast_asset_id)]
        corrections = tuple(ForecastCorrectionInstruction(
            asset_id=self.forecast_asset_id, t0_step=row["t0_step"], t1_step=row["t1_step"],
            shift_lat=row["shift_lat"], shift_lon=row["shift_lon"], rotation_deg=row["rotation_deg"],
            multiplication_factor=row["multiplication_factor"],
        ) for row in rows)
        if corrections:
            source = next((item for item in scenarios if item.source_asset_id == self.forecast_asset_id), None)
            if source is None:
                raise ValueError("Selected asset is not a resolved artifact scenario.")
            scenarios.append(ForecastScenarioReference(
                f"corrected:{self.forecast_asset_id}", len(scenarios), "corrected",
                f"{source.provider_code.upper()} corrected - {self.forecast_asset_id}",
                source.provider_code, source.source_asset_id, source.source_asset_path, corrections,
            ))
        return tuple(replace(item, position=index) for index, item in enumerate(scenarios))

    def _dashboard_artifact(self) -> CurrentRunArtifact:
        return self._load_required_current_artifact()

    def apply_forecast_draft(self, *, responsible_person: str = "", reason: str = "") -> list[dict[str, Any]]:
        """Validate and stage forecast corrections; centralized Update performs the write."""
        if not self.forecast_asset_id:
            raise ValueError("Select a forecast asset first.")
        try:
            rows = validate_forecast_edit_draft(self.forecast_asset_id, self.forecast_draft)
        except ValueError as exc:
            self.set_message("", "warning", translation_key="message.operation_failed", translation_args={"operation_key": "message.operation_apply_forecast", "detail": str(exc)})
            raise
        if responsible_person:
            self.responsible_person = responsible_person.strip()
        if reason:
            self.update_reason = reason.strip()
        self.set_message("", "success", translation_key="message.forecast_applied")
        return rows

    def save_forecast_corrections(self, *, responsible_person: str = "", reason: str = "") -> list[dict[str, Any]]:
        """Compatibility API; the Forecast tab uses apply_forecast_draft instead."""
        rows = self.apply_forecast_draft(responsible_person=responsible_person, reason=reason)
        self.update_current_run()
        return rows

    def update_current_run(self) -> CurrentRunArtifact:
        base = self._dashboard_artifact()
        review = {"start": pd.Timestamp(self.review_window_start).isoformat(), "end": pd.Timestamp(self.review_window_end).isoformat()}
        artifact = replace(
            base, review_window=review, precipitation_qc_settings=self._qc_policy().as_dict(),
            observed_replacements=self._replacement_instructions(), station_exclusions=self._exclusion_instructions(),
            scenarios=self._scenarios_with_forecast_draft(base), responsible_person=self.responsible_person.strip() or None,
            reason=self.update_reason.strip() or None, saved_run_id=None, saved_run_description=None,
        )
        artifact.validate(require_reason=True)
        if self.context.paths.current_run_db.exists():
            persisted = update_current_artifact(self.context.paths.current_run_db, artifact, require_reason=True)
        else:
            persisted = create_current_artifact(self.context.paths.current_run_db, artifact)
        self.set_message("", "success", translation_key="message.run_updated")
        return persisted

    def save_run(self):
        result = save_current_artifact(
            self.context.paths.current_run_db, self.context.paths.runs_dir,
            run_id=self.save_run_id, description=self.save_run_description,
        )
        self.set_message("", "success", translation_key="message.run_saved", translation_args={"run_id": result.run_id})
        return result

    def artifact_summary(self) -> dict[str, Any]:
        artifact = load_current_artifact(self.context.paths.current_run_db)
        return {
            "reference_time": artifact.reference_time,
            "forecast_end_exclusive": artifact.forecast_end_exclusive,
            "providers": list(artifact.observed_providers), "scenarios": len(artifact.scenarios),
            "replacements": len(artifact.observed_replacements), "exclusions": len(artifact.station_exclusions),
            "responsible_person": artifact.responsible_person, "reason": artifact.reason,
        }

    def _dispatch_document(self, callback) -> None:
        try:
            import panel as pn
            document = pn.state.curdoc
            if document is not None and getattr(document, "session_context", None) is not None:
                document.add_next_tick_callback(callback)
                return
        except Exception:
            pass
        callback()

    def run_current_artifact_async(self):
        if self.execution_active:
            raise RuntimeError("This session already has an active current-run execution.")
        if not self.context.paths.current_run_db.exists():
            raise FileNotFoundError("Update Current Run before executing Refresh.")
        self.execution_active = True
        self.execution_progress = 5
        self.execution_status = "running"
        self.execution_error = ""
        future = self._execution_pool.submit(execute_current_artifact, self.context, self.context.paths.current_run_db)

        def finish(completed) -> None:
            try:
                completed.result()
            except BaseException as exc:
                def fail() -> None:
                    artifact = load_current_artifact(self.context.paths.current_run_db)
                    self.execution_active = False
                    self.execution_progress = 0
                    self.execution_status = "failed"
                    self.execution_error = artifact.execution.error_message or str(exc)
                    self.set_message(self.execution_error, "danger")
                self._dispatch_document(fail)
                return

            def succeed() -> None:
                artifact = load_current_artifact(self.context.paths.current_run_db)
                self.execution_active = False
                self.execution_progress = 100
                self.execution_status = "completed"
                self.execution_generation = artifact.execution.execution_id or ""
                self.source_versions = {**self.source_versions, "execution_generation": self.execution_generation}
                self.reload_requested = True
                try:
                    import panel as pn
                    if pn.state.location is not None:
                        pn.state.location.reload = True
                except Exception:
                    pass
            self._dispatch_document(succeed)

        future.add_done_callback(finish)
        return future

    def set_message(
        self,
        text: str,
        kind: str = "info",
        *,
        translation_key: str | None = None,
        translation_args: dict[str, Any] | None = None,
    ) -> None:
        args = dict(translation_args or {})
        self._message_record = (translation_key, args, text)
        self.message = self._render_translation(translation_key, args, text)
        self.message_kind = kind


__all__ = [
    "DashboardState",
]
