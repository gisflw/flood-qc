"""Session-local dashboard parameters and state transitions."""
from __future__ import annotations

import sqlite3
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Mapping

import pandas as pd
import param

from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.services import forecast as dashboard_forecast
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
    _model_variables,
    parse_signed_rainfall_period,
    _observed_series,
    _station_catalog,
    BasinSpatialData,
)
from mgb_ops.analysis import timeseries as dashboard_data
from mgb_ops.analysis.windows import build_analysis_window
from mgb_ops.assets.model_outputs import validate_model_outputs_netcdf
from mgb_ops.assets.spatial_grid import RegularGridSpec
from mgb_ops.config.runtime import RuntimeContext, build_runtime_context
from mgb_ops.config.workspace import resolve_workspace_path
from mgb_ops.utils.time import resolve_reference_time
from mgb_ops.model.prepare_mgb_rainfall import (
    MGB_FORECAST_CACHE_FILENAME,
    MGB_OBSERVED_CACHE_FILENAME,
)
from mgb_ops.edit.sqlite import list_forecast_corrections, replace_forecast_corrections


@dataclass(frozen=True, slots=True)
class DashboardSources:
    history: str
    spatial: str
    model: str


class DashboardState(param.Parameterized):
    """All mutable dashboard state; one instance is created per Panel session."""

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
        run_settings = self.context.settings["run"]
        mgb_settings = self.context.settings["mgb"]
        self.history_path = self.context.paths.history_db
        self.model_path = self.context.paths.processed_dir / "model_outputs.nc"
        configured_window = build_analysis_window(
            resolve_reference_time(str(run_settings["reference_time"])),
            output_days_before=int(mgb_settings["output_days_before"]),
            forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
        )
        self.window = self._resolve_dashboard_window(configured_window)
        self.observed_precipitation_path = (
            self.context.paths.cache_dir / MGB_OBSERVED_CACHE_FILENAME
        )
        self.forecast_precipitation_path = (
            self.context.paths.cache_dir / MGB_FORECAST_CACHE_FILENAME
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
        super().__init__(**params)
        self.refresh()

    def _resolve_dashboard_window(self, configured_window):
        if not self.model_path.exists():
            return configured_window
        try:
            metadata = validate_model_outputs_netcdf(self.model_path)
        except (FileNotFoundError, OSError, ValueError):
            return configured_window
        return metadata.get("window", configured_window)

    def _versions(self) -> DashboardSources:
        return DashboardSources(
            history=dashboard_map.build_sqlite_version(self.history_path),
            spatial=dashboard_map.build_file_version(self.gpkg_path),
            model=dashboard_map.build_file_version(self.model_path),
        )

    def add_warning(self, message: str) -> None:
        if message not in self.warnings:
            self.warnings = [*self.warnings, message]

    def refresh(self) -> None:
        """Re-version sources and refresh only this state/session."""
        versions = self._versions()
        self.source_versions = {
            "history": versions.history,
            "spatial": versions.spatial,
            "model": versions.model,
        }
        self.warnings = []
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
                self.warnings = [
                    *self.warnings,
                    f"Observed database could not be read: {exc}",
                ]
        else:
            self.stations = pd.DataFrame()
            self.warnings = [
                *self.warnings,
                f"History database not found: {self.history_path}",
            ]

        segments = None
        try:
            segments = _mini_segment_paths(
                str(self.gpkg_path), workspace, versions.spatial
            )
        except (FileNotFoundError, ValueError) as exc:
            self.warnings = [
                *self.warnings,
                f"Mini spatial layers unavailable: {exc}",
            ]

        try:
            self.model_variables = _model_variables(
                str(self.model_path), workspace, versions.model, self.window
            )
        except (FileNotFoundError, OSError, ValueError) as exc:
            self.model_variables = dashboard_data.list_model_variables()
            self.warnings = [*self.warnings, str(exc)]

        self.accumulation_rasters = []
        bbox = self.context.settings["spatial_grid"]["bbox"]
        if bbox is not None:
            try:
                self.accumulation_rasters = [
                    _accumulation_raster(
                        str(self._rainfall_cache_path()),
                        workspace,
                        dashboard_map.build_file_version(
                            self._rainfall_cache_path()
                        ),
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
                self.warnings = [
                    *self.warnings,
                    f"{self._selected_rainfall_mode().title()} rainfall maps unavailable: {exc}",
                ]
        elif bbox is None:
            self.warnings = [
                *self.warnings,
                "Set spatial_grid.bbox in <workspace>/config/custom.yaml to enable rainfall maps.",
            ]

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
        self.apply_map_configuration(apply_basin=False)
        if self.accumulation_rasters is before_rasters and self.selected_raster == before_selection:
            new_warnings = [value for value in self.warnings if value not in before_warnings]
            if new_warnings:
                self.warnings = [
                    *before_warnings,
                    f"{mode.title()} rainfall map unavailable: {new_warnings[-1].split(': ', 1)[-1]}",
                ]

    def apply_map_configuration(self, *, apply_basin: bool = True) -> None:
        """Atomically apply rainfall and optional basin-boundary map settings."""
        bbox = self.context.settings["spatial_grid"].get("bbox")
        if bbox is None:
            self.warnings = [*self.warnings, "Rainfall maps are unavailable for this workspace."]
            return
        try:
            rainfall_mode, rainfall_hours = parse_signed_rainfall_period(int(self.rainfall_period))
            cache_path = self._rainfall_cache_path(rainfall_mode)
            raster = _accumulation_raster(
                str(cache_path), str(self.workspace),
                dashboard_map.build_file_version(cache_path), self.window,
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
            self.warnings = [*self.warnings, f"Map configuration was not applied: {exc}"]
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
        return self.observed_precipitation_path if mode == "observed" else self.forecast_precipitation_path

    def _selected_rainfall_hours(self) -> int:
        return parse_signed_rainfall_period(int(self.rainfall_period))[1]

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
                self.add_warning(f"Selected basin unavailable: {exc}")
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

    def observed_series(self) -> pd.DataFrame:
        if self.station_id is None or not self.history_path.exists():
            return pd.DataFrame()
        return _observed_series(
            self.station_id,
            str(self.history_path),
            str(self.workspace),
            self.source_versions.get("history", ""),
            self.window,
        )

    def mgb_series(self, variable_code: str) -> pd.DataFrame:
        if self.mini_id is None:
            return pd.DataFrame()
        return _mgb_series(
            self.mini_id,
            variable_code,
            str(self.model_path),
            str(self.workspace),
            self.source_versions.get("model", ""),
            self.window,
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

    def basin_precipitation(self) -> pd.DataFrame:
        if self.mini_id is None:
            return pd.DataFrame()
        basin = self.basin_spatial_data()
        return _basin_precipitation(
            basin.mini_ids,
            basin.weights,
            str(self.model_path),
            str(self.workspace),
            self.source_versions.get("model", ""),
            self.window,
        )

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
        try:
            self.forecast_assets = _forecast_assets(
                str(self.history_path),
                str(self.workspace),
                self.source_versions.get("history", ""),
                self.window,
                str(self.context.settings["forecast"]["provider"]),
                int(self.context.settings["forecast"].get("lookback_cycles", 1)),
            )
        except (
            FileNotFoundError,
            sqlite3.Error,
            pd.errors.DatabaseError,
            RuntimeError,
            ValueError,
        ) as exc:
            self.warnings = [
                *self.warnings,
                f"Forecast assets unavailable: {exc}",
            ]
            return
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
            self.set_message(str(exc), "warning")
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
        bbox = self.context.settings["spatial_grid"]["bbox"]
        if bbox is None:
            raise ValueError(
                "Set spatial_grid.bbox in <workspace>/config/custom.yaml to enable forecast maps."
            )
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
            tuple(float(value) for value in bbox),
            float(self.context.settings["spatial_grid"]["resolution_degrees"]),
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
        if not self.forecast_asset_id:
            self.forecast_draft = empty_forecast_edit_frame()
            return self.forecast_draft
        rows = list_forecast_corrections(
            self.history_path, self.forecast_asset_id
        )
        frame = pd.DataFrame(rows)
        if not frame.empty:
            frame["asset_id"] = self.forecast_asset_id
            frame["remove"] = False
        self.forecast_draft = normalize_forecast_edit_frame(frame)
        return self.forecast_draft

    def update_forecast_draft(self, frame: pd.DataFrame) -> None:
        draft = normalize_forecast_edit_frame(frame)
        if self.forecast_asset_id:
            draft["asset_id"] = self.forecast_asset_id
        self.forecast_draft = draft

    def add_forecast_correction(self, **values: Any) -> None:
        if not self.forecast_asset_id:
            raise ValueError("Select a forecast asset first.")
        row = build_forecast_edit_row(
            asset_id=self.forecast_asset_id,
            t0_step=int(values.get("t0_step", self.forecast_t0_step)),
            t1_step=int(values.get("t1_step", self.forecast_t1_step)),
            shift_lat=float(values.get("shift_lat", self.forecast_shift_lat)),
            shift_lon=float(values.get("shift_lon", self.forecast_shift_lon)),
            rotation_deg=float(
                values.get("rotation_deg", self.forecast_rotation_deg)
            ),
            multiplication_factor=float(
                values.get(
                    "multiplication_factor",
                    self.forecast_multiplication_factor,
                )
            ),
            editor=str(values.get("editor", "")),
            reason=str(values.get("reason", "")),
            metadata=values.get("metadata"),
        )
        next_draft = pd.concat(
            [self.forecast_draft, pd.DataFrame([row])], ignore_index=True
        )
        self.update_forecast_draft(next_draft)
        self.set_message("Correction added to draft.", "success")

    def save_forecast_corrections(self) -> list[dict[str, Any]]:
        if not self.forecast_asset_id:
            raise ValueError("Select a forecast asset first.")
        try:
            rows = validate_forecast_edit_draft(
                self.forecast_asset_id, self.forecast_draft
            )
            persisted = replace_forecast_corrections(
                self.history_path, self.forecast_asset_id, rows
            )
        except ValueError as exc:
            self.set_message(str(exc), "warning")
            raise
        except sqlite3.IntegrityError as exc:
            self.set_message(f"Database conflict: {exc}", "danger")
            raise
        self.forecast_draft = normalize_forecast_edit_frame(
            pd.DataFrame(persisted)
        )
        if not self.forecast_draft.empty:
            self.forecast_draft["asset_id"] = self.forecast_asset_id
            self.forecast_draft["remove"] = False
        self.set_message(
            "Corrections persisted to history.sqlite.", "success"
        )
        return persisted

    def set_message(self, text: str, kind: str = "info") -> None:
        self.message = text
        self.message_kind = kind


__all__ = [
    "DashboardState",
]
