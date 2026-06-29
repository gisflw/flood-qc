"""Session-local state and transitions for the Panel dashboard."""
from __future__ import annotations

import json
import sqlite3
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import Any, Mapping

import numpy as np
import pandas as pd
import panel as pn
import param

from apps.ops_dashboard.support import data as dashboard_data
from apps.ops_dashboard.support import forecast as dashboard_forecast
from apps.ops_dashboard.support import map as dashboard_map
from mgb_ops.analysis.spatial import RegularGridSpec, observed_rainfall_grid
from mgb_ops.common.runtime import RuntimeContext, build_runtime_context
from mgb_ops.common.time_utils import (
    DashboardWindow,
    resolve_dashboard_window,
    resolve_workspace_path,
)
from mgb_ops.edit.forcing import ForecastCorrectionInstruction
from mgb_ops.edit.sqlite import (
    list_forecast_corrections,
    replace_forecast_corrections,
)


FORECAST_EDIT_COLUMNS = [
    "manual_edit_id",
    "asset_id",
    "t0_step",
    "t1_step",
    "shift_lat",
    "shift_lon",
    "rotation_deg",
    "multiplication_factor",
    "editor",
    "reason",
    "metadata_json",
    "created_at",
    "remove",
]
FORECAST_EDIT_NUMERIC_COLUMNS = [
    "t0_step",
    "t1_step",
    "shift_lat",
    "shift_lon",
    "rotation_deg",
    "multiplication_factor",
]


@dataclass(frozen=True, slots=True)
class DashboardSources:
    history: str
    spatial: str
    model: str


def normalize_forecast_edit_frame(frame: pd.DataFrame | None) -> pd.DataFrame:
    normalized = pd.DataFrame() if frame is None else frame.copy()
    for column in FORECAST_EDIT_COLUMNS:
        if column not in normalized.columns:
            normalized[column] = pd.NA
    normalized["manual_edit_id"] = pd.to_numeric(
        normalized["manual_edit_id"], errors="coerce"
    ).astype("Int64")
    for column in FORECAST_EDIT_NUMERIC_COLUMNS:
        normalized[column] = pd.to_numeric(normalized[column], errors="coerce")
    for column in ("asset_id", "editor", "reason", "created_at"):
        normalized[column] = normalized[column].fillna("").astype(str)
    normalized["metadata_json"] = (
        normalized["metadata_json"].fillna("{}").astype(str)
    )
    normalized["remove"] = normalized["remove"].fillna(False).astype(bool)
    return normalized[FORECAST_EDIT_COLUMNS]


def empty_forecast_edit_frame() -> pd.DataFrame:
    return normalize_forecast_edit_frame(None)


def build_forecast_edit_row(
    *,
    asset_id: str,
    t0_step: int,
    t1_step: int,
    shift_lat: float,
    shift_lon: float,
    rotation_deg: float,
    multiplication_factor: float,
    editor: str,
    reason: str,
    metadata: dict[str, Any] | None = None,
) -> dict[str, object]:
    return {
        "manual_edit_id": pd.NA,
        "asset_id": asset_id,
        "t0_step": int(t0_step),
        "t1_step": int(t1_step),
        "shift_lat": float(shift_lat),
        "shift_lon": float(shift_lon),
        "rotation_deg": float(rotation_deg),
        "multiplication_factor": float(multiplication_factor),
        "editor": str(editor),
        "reason": str(reason),
        "metadata_json": json.dumps(metadata or {}, sort_keys=True, ensure_ascii=True),
        "created_at": "",
        "remove": False,
    }


def validate_forecast_edit_draft(
    asset_id: str, frame: pd.DataFrame
) -> list[dict[str, Any]]:
    normalized = normalize_forecast_edit_frame(frame)
    active = normalized.loc[~normalized["remove"]].reset_index(drop=True)
    rows: list[dict[str, Any]] = []
    for row_index, row in enumerate(active.itertuples(index=False), start=1):
        if pd.isna(row.t0_step) or pd.isna(row.t1_step):
            raise ValueError(f"Row {row_index}: t0_step and t1_step are required.")
        if pd.isna(row.multiplication_factor):
            raise ValueError(
                f"Row {row_index}: multiplication_factor is required."
            )
        t0_step, t1_step = int(row.t0_step), int(row.t1_step)
        if t1_step < t0_step:
            raise ValueError(f"Row {row_index}: t1_step must be >= t0_step.")
        factor = float(row.multiplication_factor)
        if not np.isfinite(factor) or factor <= 0:
            raise ValueError(
                f"Row {row_index}: multiplication_factor must be > 0."
            )
        reason = str(row.reason or "").strip()
        if not reason:
            raise ValueError(f"Row {row_index}: correction reason is required.")
        metadata_json = str(row.metadata_json or "{}").strip() or "{}"
        try:
            metadata = json.loads(metadata_json)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Row {row_index}: invalid metadata_json.") from exc
        rows.append(
            {
                "asset_id": asset_id,
                "t0_step": t0_step,
                "t1_step": t1_step,
                "shift_lat": float(
                    0.0 if pd.isna(row.shift_lat) else row.shift_lat
                ),
                "shift_lon": float(
                    0.0 if pd.isna(row.shift_lon) else row.shift_lon
                ),
                "rotation_deg": float(
                    0.0 if pd.isna(row.rotation_deg) else row.rotation_deg
                ),
                "multiplication_factor": factor,
                "editor": str(row.editor or "").strip() or None,
                "reason": reason,
                "metadata": metadata,
            }
        )
    rows.sort(key=lambda item: (item["t0_step"], item["t1_step"]))
    for previous, current in zip(rows, rows[1:]):
        if current["t0_step"] < previous["t1_step"]:
            raise ValueError(
                "Overlapping grid corrections: "
                f"[{previous['t0_step']}, {previous['t1_step']}] x "
                f"[{current['t0_step']}, {current['t1_step']}]."
            )
    return rows


def build_forecast_instruction_from_request(
    request: dashboard_forecast.ForecastPreviewRequest,
) -> ForecastCorrectionInstruction:
    return ForecastCorrectionInstruction(
        asset_id=request.asset_id,
        t0_step=request.t0_step,
        t1_step=request.t1_step,
        shift_lat=request.shift_lat,
        shift_lon=request.shift_lon,
        rotation_deg=request.rotation_deg,
        multiplication_factor=request.multiplication_factor,
    )


@pn.cache(max_items=8)
def _station_catalog(
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_station_catalog(
        Path(database_path),
        start_time=window.start_time,
        end_time=window.cutoff_time,
    )


@pn.cache(max_items=256)
def _observed_series(
    station_id: str,
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_observed_series(
        station_id,
        Path(database_path),
        start_time=window.start_time,
        end_time=window.cutoff_time,
    )


@pn.cache(max_items=8)
def _mini_layers(
    gpkg_path: str, workspace: str, source_version: str
) -> dashboard_data.MiniSpatialLayers:
    del workspace, source_version
    return dashboard_data.read_mini_layers(Path(gpkg_path))


@pn.cache(max_items=8)
def _model_variables(
    model_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del workspace, source_version
    dashboard_data.validate_model_outputs_netcdf(
        Path(model_path), expected_window=window
    )
    return dashboard_data.list_model_variables(Path(model_path))


@pn.cache(max_items=256)
def _mgb_series(
    mini_id: int,
    variable_code: str,
    model_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_mgb_series(
        Path(model_path),
        mini_id=mini_id,
        variable_code=variable_code,
        window=window,
    )


@pn.cache(max_items=32)
def _accumulation_rasters(
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
    bbox: tuple[float, float, float, float],
    resolution: float,
    horizons: tuple[int, ...],
    nearest_stations: int,
    power: float,
) -> tuple[dict[str, object], ...]:
    del workspace, source_version
    grid = RegularGridSpec(bbox=bbox, resolution=resolution)
    result = []
    for hours in horizons:
        end_time = window.cutoff_time
        rainfall = observed_rainfall_grid(
            Path(database_path),
            grid=grid,
            start_time=max(
                window.start_time,
                end_time - pd.Timedelta(hours=int(hours)).to_pytimedelta(),
            ),
            end_time=end_time,
            nearest_stations=nearest_stations,
            power=power,
        )
        result.append(
            {
                "name": f"accum_{hours}h",
                "horizon_hours": hours,
                "horizon_label": f"{hours}h",
                "grid": rainfall,
            }
        )
    return tuple(result)


@pn.cache(max_items=16)
def _forecast_assets(
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del source_version
    return dashboard_forecast.list_forecast_assets(
        Path(database_path), Path(workspace), window=window
    )


@pn.cache(max_items=128)
def _forecast_steps(
    asset_id: str,
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del source_version
    return dashboard_forecast.list_forecast_steps(
        asset_id,
        database_path=Path(database_path),
        workspace_path=Path(workspace),
        window=window,
    )


@pn.cache(max_items=128)
def _forecast_preview(
    asset_id: str,
    t0_step: int,
    t1_step: int,
    database_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
    bbox: tuple[float, float, float, float],
    resolution: float,
) -> dashboard_forecast.ForecastPreview:
    del source_version, window
    return dashboard_forecast.build_forecast_preview(
        asset_id,
        t0_step=t0_step,
        t1_step=t1_step,
        database_path=Path(database_path),
        workspace_path=Path(workspace),
        target_grid=RegularGridSpec(bbox=bbox, resolution=resolution),
    )


class DashboardController(param.Parameterized):
    """All mutable dashboard state; one instance is created per Panel session."""

    station_id = param.String(default=None, allow_None=True)
    mini_id = param.Integer(default=None, allow_None=True)
    selected_raster = param.Selector(default=None, objects=[None])
    raster_opacity = param.Number(default=0.6, bounds=(0, 1), step=0.05)
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
        self.window = resolve_dashboard_window(self.context.settings)
        self.history_path = self.context.paths.history_db
        self.model_path = self.context.paths.processed_dir / "model_outputs.nc"
        self.gpkg_path = resolve_workspace_path(
            self.context.settings["spatial"]["gpkg_path"], self.workspace
        )
        super().__init__(**params)
        self.refresh()

    def _versions(self) -> DashboardSources:
        return DashboardSources(
            history=dashboard_map.build_sqlite_version(self.history_path),
            spatial=dashboard_map.build_file_version(self.gpkg_path),
            model=dashboard_map.build_file_version(self.model_path),
        )

    def refresh(self) -> None:
        """Re-version sources and refresh only this controller/session."""
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

        segments = catchments = None
        try:
            minis = _mini_layers(str(self.gpkg_path), workspace, versions.spatial)
            segments = dashboard_data.layer_geojson(minis.mini_segments)
            catchments = dashboard_data.layer_geojson(minis.mini_catchments)
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
        bbox = self.context.settings["forecast_grid"]["bbox"]
        if self.history_path.exists() and bbox is not None:
            try:
                summaries = self.context.settings["summaries"]
                interpolation = self.context.settings["rainfall_interpolation"]
                self.accumulation_rasters = list(
                    _accumulation_rasters(
                        str(self.history_path),
                        workspace,
                        versions.history,
                        self.window,
                        tuple(float(value) for value in bbox),
                        float(summaries["grid_resolution_degrees"]),
                        tuple(int(value) for value in summaries["accum_hours"]),
                        int(interpolation["nearest_stations"]),
                        float(interpolation["power"]),
                    )
                )
            except (
                FileNotFoundError,
                sqlite3.Error,
                pd.errors.DatabaseError,
                ValueError,
            ) as exc:
                self.warnings = [
                    *self.warnings,
                    f"Observed rainfall maps unavailable: {exc}",
                ]
        elif bbox is None:
            self.warnings = [
                *self.warnings,
                "Set forecast_grid.bbox in <workspace>/config/custom.yaml to enable rainfall maps.",
            ]

        raster_names = [str(item["name"]) for item in self.accumulation_rasters]
        self.param.selected_raster.objects = [None, *raster_names]
        if self.selected_raster not in raster_names:
            self.selected_raster = raster_names[0] if raster_names else None
        if self.station_id is None and not self.stations.empty:
            preferred = self.stations[self.stations["status"] != "no_data"]
            row = preferred.iloc[0] if not preferred.empty else self.stations.iloc[0]
            self.station_id = str(row["station_id"])
        self._rebuild_map(segments=segments, catchments=catchments)
        self._refresh_forecast_assets()
        self.last_refresh_at = datetime.now().strftime("%d/%m/%Y %H:%M:%S")

    @param.depends("selected_raster", "raster_opacity", watch=True)
    def _rebuild_map(
        self,
        *_: Any,
        segments: dict[str, Any] | None = None,
        catchments: dict[str, Any] | None = None,
    ) -> None:
        if segments is None or catchments is None:
            try:
                minis = _mini_layers(
                    str(self.gpkg_path),
                    str(self.workspace),
                    self.source_versions.get("spatial", ""),
                )
                segments = dashboard_data.layer_geojson(minis.mini_segments)
                catchments = dashboard_data.layer_geojson(minis.mini_catchments)
            except (FileNotFoundError, ValueError):
                segments = catchments = None
        catalog = {
            str(item["name"]): item for item in self.accumulation_rasters
        }
        self.map_artifacts = dashboard_map.build_ops_map(
            self.selected_raster,
            self.raster_opacity,
            self.stations,
            segments,
            catchments,
            catalog,
        )

    def handle_map_click(self, click_state: Mapping[str, Any] | None) -> None:
        selection = dashboard_map.decode_click_state(click_state)
        if selection.station_id is not None:
            self.station_id = selection.station_id
        if selection.mini_id is not None:
            self.mini_id = selection.mini_id
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

    def _analysis_grid(self) -> RegularGridSpec:
        bbox = self.context.settings["forecast_grid"]["bbox"]
        if bbox is None:
            raise ValueError(
                "Set forecast_grid.bbox in <workspace>/config/custom.yaml to enable forecast maps."
            )
        return RegularGridSpec(
            bbox=tuple(float(value) for value in bbox),
            resolution=float(
                self.context.settings["summaries"]["grid_resolution_degrees"]
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
        bbox = self.context.settings["forecast_grid"]["bbox"]
        if bbox is None:
            raise ValueError(
                "Set forecast_grid.bbox in <workspace>/config/custom.yaml to enable forecast maps."
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
            float(self.context.settings["summaries"]["grid_resolution_degrees"]),
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
    "DashboardController",
    "FORECAST_EDIT_COLUMNS",
    "build_forecast_edit_row",
    "build_forecast_instruction_from_request",
    "empty_forecast_edit_frame",
    "normalize_forecast_edit_frame",
    "validate_forecast_edit_draft",
]
