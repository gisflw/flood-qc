"""Forecast map rendering; forecast reads and edits live in ``mgb_ops``."""
from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path

import folium
import folium.plugins
import numpy as np
import pandas as pd

from apps.ops_dashboard.support import map as ops_dashboard_map
from apps.ops_dashboard.support import data as ops_dashboard_data
from mgb_ops.analysis import forecast as forecast_analysis
from mgb_ops.analysis.spatial import PrecipitationGrid, RegularGridSpec
from mgb_ops.edit.forcing import apply_corrections
from mgb_ops.common.time_utils import DashboardWindow


@dataclass(frozen=True, slots=True)
class ForecastPreview:
    asset_id: str
    relative_path: str
    data: np.ndarray
    latitudes: np.ndarray
    longitudes: np.ndarray
    t0_step: int
    t1_step: int
    mode_label: str
    title: str
    start_time: pd.Timestamp | None = None
    end_time: pd.Timestamp | None = None
    source_grid: PrecipitationGrid | None = None

    @property
    def bounds(self) -> tuple[float, float, float, float]:
        if self.source_grid is not None:
            return self.source_grid.bounds
        return (
            float(np.min(self.longitudes)), float(np.min(self.latitudes)),
            float(np.max(self.longitudes)), float(np.max(self.latitudes)),
        )


@dataclass(frozen=True, slots=True)
class ForecastPreviewRequest:
    asset_id: str
    t0_step: int
    t1_step: int
    shift_lat: float = 0.0
    shift_lon: float = 0.0
    rotation_deg: float = 0.0
    multiplication_factor: float = 1.0
    opacity: float = 0.7

    @property
    def has_correction(self) -> bool:
        return any((
            abs(self.shift_lat) > 1e-9,
            abs(self.shift_lon) > 1e-9,
            abs(self.rotation_deg) > 1e-9,
            abs(self.multiplication_factor - 1.0) > 1e-9,
        ))


@dataclass(frozen=True, slots=True)
class ForecastMapPanelArtifacts:
    title: str
    legend_html: str


@dataclass(frozen=True, slots=True)
class ForecastMapComparisonArtifacts:
    map_figure: folium.MacroElement
    original: ForecastMapPanelArtifacts
    corrected: ForecastMapPanelArtifacts | None = None


def list_forecast_assets(
    database_path: Path,
    workspace_path: Path,
    *,
    window: DashboardWindow,
) -> pd.DataFrame:
    return forecast_analysis.list_expected_ecmwf_assets(
        database_path,
        workspace_path=workspace_path,
        reference_time=window.cutoff_time,
    )


def list_forecast_steps(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path,
    window: DashboardWindow,
) -> pd.DataFrame:
    frame = forecast_analysis.list_dashboard_forecast_intervals(
        asset_id,
        database_path=database_path,
        workspace_path=workspace_path,
        window=window,
    )
    if frame.empty:
        return pd.DataFrame(columns=["step_hours", "valid_time", "label"])
    first = pd.DataFrame([{
        "step_hours": int(frame.iloc[0]["start_step_hours"]),
        "valid_time": frame.iloc[0]["start_time"],
        "label": f"t={int(frame.iloc[0]['start_step_hours'])}h | {pd.Timestamp(frame.iloc[0]['start_time']):%d/%m %H:%M}",
    }])
    ends = frame.rename(columns={"end_step_hours": "step_hours", "end_time": "valid_time"}).copy()
    ends["label"] = ends.apply(
        lambda row: f"t={int(row['step_hours'])}h | {pd.Timestamp(row['valid_time']):%d/%m %H:%M}",
        axis=1,
    )
    return pd.concat([first, ends[["step_hours", "valid_time", "label"]]], ignore_index=True).drop_duplicates("step_hours")


def build_forecast_preview(
    asset_id: str,
    *,
    t0_step: int,
    t1_step: int,
    database_path: Path,
    workspace_path: Path,
    target_grid: RegularGridSpec,
) -> ForecastPreview:
    row, _ = forecast_analysis.resolve_forecast_asset(
        asset_id, database_path=database_path, workspace_path=workspace_path
    )
    grid = forecast_analysis.build_forecast_grid(
        asset_id,
        database_path=database_path,
        workspace_path=workspace_path,
        t0_step=t0_step,
        t1_step=t1_step,
        target_grid=target_grid,
    )
    return ForecastPreview(
        asset_id=asset_id,
        relative_path=str(row["relative_path"]),
        data=grid.values,
        latitudes=grid.latitudes,
        longitudes=grid.longitudes,
        t0_step=int(t0_step),
        t1_step=int(t1_step),
        mode_label="timestep_sum",
        title=f"Forecast accumulation t={t0_step}h–t={t1_step}h",
        start_time=pd.Timestamp(grid.start_time),
        end_time=pd.Timestamp(grid.end_time),
        source_grid=grid,
    )


def apply_preview_corrections(preview: ForecastPreview, instructions: list[object]) -> ForecastPreview:
    source_grid = preview.source_grid or PrecipitationGrid(
        values=preview.data,
        latitudes=preview.latitudes,
        longitudes=preview.longitudes,
        bounds=preview.bounds,
        start_time=pd.Timestamp("1970-01-01"),
        end_time=pd.Timestamp("1970-01-01 01:00"),
        source="forecast",
    )
    corrected = apply_corrections(source_grid, instructions)
    return replace(preview, data=corrected.values, source_grid=corrected, title=f"{preview.title} | corrected")


def _add_preview(fmap: folium.Map, preview: ForecastPreview, opacity: float) -> None:
    ops_dashboard_map.add_raster_overlay(
        fmap, data=preview.data, latitudes=preview.latitudes, bounds=preview.bounds, layer_name=preview.title,
        opacity=opacity, horizon_label=preview.title, show=True, include_legend=False,
    )


def build_forecast_map(
    preview: ForecastPreview,
    *,
    corrected_preview: ForecastPreview | None = None,
    opacity: float = 0.7,
) -> folium.Map | folium.plugins.DualMap:
    west, south, east, north = preview.bounds
    center = [(south + north) / 2, (west + east) / 2]
    if corrected_preview is None:
        fmap = folium.Map(location=center, zoom_start=7, tiles="CartoDB Positron")
        _add_preview(fmap, preview, opacity)
        return fmap
    fmap = folium.plugins.DualMap(location=center, zoom_start=7, tiles="CartoDB Positron")
    _add_preview(fmap.m1, preview, opacity)
    _add_preview(fmap.m2, corrected_preview, opacity)
    return fmap


def build_forecast_map_artifacts(
    preview: ForecastPreview,
    *,
    corrected_preview: ForecastPreview | None = None,
    opacity: float = 0.7,
    component_key: str = "forecast-preview-map",
) -> ForecastMapComparisonArtifacts:
    del component_key
    def panel(item: ForecastPreview) -> ForecastMapPanelArtifacts:
        spec = ops_dashboard_map.build_raster_legend_spec(item.data, caption=item.title)
        legend = ops_dashboard_map.build_raster_legend_html(spec) if spec else "No valid precipitation."
        return ForecastMapPanelArtifacts(item.title, legend)
    return ForecastMapComparisonArtifacts(
        map_figure=build_forecast_map(preview, corrected_preview=corrected_preview, opacity=opacity),
        original=panel(preview),
        corrected=panel(corrected_preview) if corrected_preview else None,
    )
