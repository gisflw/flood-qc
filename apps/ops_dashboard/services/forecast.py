"""Forecast reads and JSON-only DeckGL preview builders."""
from __future__ import annotations

from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any, Mapping

import numpy as np
import pandas as pd

from apps.ops_dashboard.services import deckgl as dashboard_map
from mgb_ops.adapters import get_forecast_adapter
from mgb_ops.analysis import forecast as forecast_analysis
from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec
from mgb_ops.assets.types import AnalysisWindow
from mgb_ops.edit.forcing import apply_corrections
from mgb_ops.utils.time import iter_forecast_cycle_candidates, resolve_forecast_cycle


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
            float(np.min(self.longitudes)),
            float(np.min(self.latitudes)),
            float(np.max(self.longitudes)),
            float(np.max(self.latitudes)),
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
        return any(
            (
                abs(self.shift_lat) > 1e-9,
                abs(self.shift_lon) > 1e-9,
                abs(self.rotation_deg) > 1e-9,
                abs(self.multiplication_factor - 1.0) > 1e-9,
            )
        )


@dataclass(frozen=True, slots=True)
class ForecastMapPanelArtifacts:
    title: str
    spec: dict[str, Any]
    lookup: dashboard_map.RasterLookup
    legend: dashboard_map.RasterLegendSpec | None

    @property
    def legend_html(self) -> str:
        return (
            dashboard_map.build_raster_legend_html(self.legend)
            if self.legend is not None
            else "No valid precipitation."
        )


@dataclass(frozen=True, slots=True)
class ForecastMapComparisonArtifacts:
    original: ForecastMapPanelArtifacts
    corrected: ForecastMapPanelArtifacts | None = None
    view_state: dict[str, float] | None = None


def list_forecast_assets(
    database_path: Path,
    workspace_path: Path,
    *,
    window: AnalysisWindow,
    provider_code: str = "ecmwf",
    lookback_cycles: int = 1,
) -> pd.DataFrame:
    adapter = get_forecast_adapter(provider_code)
    target_cycle = resolve_forecast_cycle(window.cutoff_time)
    last_error: forecast_analysis.ForecastIntegrityError | None = None
    for cycle_time in iter_forecast_cycle_candidates(
        target_cycle, lookback_cycles=int(lookback_cycles)
    ):
        try:
            return forecast_analysis.list_expected_forecast_assets(
                database_path,
                workspace_path=workspace_path,
                provider_code=adapter.provider_code,
                cycle_time=cycle_time,
            )
        except forecast_analysis.ForecastIntegrityError as exc:
            last_error = exc
            if exc.code != "unregistered_cycle":
                raise
    if last_error is not None:
        raise forecast_analysis.ForecastIntegrityError(
            "unregistered_cycle",
            f"Forecast integrity error for {adapter.provider_code}: no registered canonical NetCDF was found from expected cycle "
            f"{target_cycle.isoformat(timespec='seconds')}Z within {int(lookback_cycles)} cycle(s).",
        ) from last_error
    return pd.DataFrame()


def list_forecast_steps(
    asset_id: str,
    *,
    database_path: Path,
    workspace_path: Path,
    window: AnalysisWindow,
) -> pd.DataFrame:
    frame = forecast_analysis.list_dashboard_forecast_intervals(
        asset_id,
        database_path=database_path,
        workspace_path=workspace_path,
        window=window,
    )
    if frame.empty:
        return pd.DataFrame(columns=["step_hours", "valid_time", "label"])
    first = pd.DataFrame(
        [
            {
                "step_hours": int(frame.iloc[0]["start_step_hours"]),
                "valid_time": frame.iloc[0]["start_time"],
                "label": (
                    f"t={int(frame.iloc[0]['start_step_hours'])}h | "
                    f"{pd.Timestamp(frame.iloc[0]['start_time']):%d/%m %H:%M}"
                ),
            }
        ]
    )
    ends = frame.rename(
        columns={"end_step_hours": "step_hours", "end_time": "valid_time"}
    ).copy()
    ends["label"] = ends.apply(
        lambda row: (
            f"t={int(row['step_hours'])}h | "
            f"{pd.Timestamp(row['valid_time']):%d/%m %H:%M}"
        ),
        axis=1,
    )
    return pd.concat(
        [first, ends[["step_hours", "valid_time", "label"]]], ignore_index=True
    ).drop_duplicates("step_hours")


def build_forecast_preview(
    asset_id: str,
    *,
    t0_step: int,
    t1_step: int,
    database_path: Path,
    workspace_path: Path,
    target_grid: RegularGridSpec | None = None,
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


def apply_preview_corrections(
    preview: ForecastPreview, instructions: list[object]
) -> ForecastPreview:
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
    return replace(
        preview,
        data=corrected.values,
        latitudes=corrected.latitudes,
        longitudes=corrected.longitudes,
        source_grid=corrected,
        title=f"{preview.title} | corrected",
    )


def synchronize_view_state(
    view_state: Mapping[str, Any] | None,
    fallback: Mapping[str, Any] | None = None,
) -> dict[str, float]:
    """Return the portable DeckGL view fields shared by both forecast panes."""
    source = view_state or fallback or {}
    defaults = {"longitude": -53.3, "latitude": -29.7, "zoom": 6.5, "pitch": 0, "bearing": 0}
    return {
        key: float(source.get(key, default))
        for key, default in defaults.items()
        if source.get(key, default) is not None
    }


def _panel(
    preview: ForecastPreview,
    *,
    layer_id: str,
    opacity: float,
    view_state: Mapping[str, Any],
) -> ForecastMapPanelArtifacts:
    grid = preview.source_grid or PrecipitationGrid(
        values=preview.data,
        latitudes=preview.latitudes,
        longitudes=preview.longitudes,
        bounds=preview.bounds,
        start_time=preview.start_time or pd.Timestamp("1970-01-01"),
        end_time=preview.end_time or pd.Timestamp("1970-01-01 01:00"),
        source="forecast",
    )
    layer, lookup, legend = dashboard_map.build_raster_layer(
        grid, layer_id=layer_id, layer_name=preview.title, opacity=opacity
    )
    spec = {
        "initialViewState": dict(view_state),
        "controller": True,
        "mapStyle": "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json",
        "layers": [layer] if layer is not None else [],
    }
    return ForecastMapPanelArtifacts(preview.title, spec, lookup, legend)


def build_forecast_map(
    preview: ForecastPreview,
    *,
    corrected_preview: ForecastPreview | None = None,
    opacity: float = 0.7,
    view_state: Mapping[str, Any] | None = None,
) -> tuple[dict[str, Any], ...]:
    """Build one or two synchronized, JSON-compatible DeckGL specifications."""
    artifacts = build_forecast_map_artifacts(
        preview,
        corrected_preview=corrected_preview,
        opacity=opacity,
        view_state=view_state,
    )
    specs = [artifacts.original.spec]
    if artifacts.corrected is not None:
        specs.append(artifacts.corrected.spec)
    return tuple(specs)


def build_forecast_map_artifacts(
    preview: ForecastPreview,
    *,
    corrected_preview: ForecastPreview | None = None,
    opacity: float = 0.7,
    component_key: str = "forecast-preview-map",
    view_state: Mapping[str, Any] | None = None,
) -> ForecastMapComparisonArtifacts:
    del component_key
    fallback = dashboard_map.default_view_state(bounds=preview.bounds)
    shared_view = synchronize_view_state(view_state, fallback)
    return ForecastMapComparisonArtifacts(
        original=_panel(
            preview, layer_id="forecast-original", opacity=opacity, view_state=shared_view
        ),
        corrected=(
            _panel(
                corrected_preview,
                layer_id="forecast-corrected",
                opacity=opacity,
                view_state=shared_view,
            )
            if corrected_preview is not None
            else None
        ),
        view_state=shared_view,
    )
