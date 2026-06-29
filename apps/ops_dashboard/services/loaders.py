"""Panel-cached loaders with immutable inputs and no session-state access."""
from __future__ import annotations

from pathlib import Path

import pandas as pd
import panel as pn
import geopandas as gpd

from apps.ops_dashboard.services import forecast as dashboard_forecast
from mgb_ops.analysis import timeseries as dashboard_data
from mgb_ops.analysis.spatial import RegularGridSpec, observed_rainfall_grid
from mgb_ops.analysis.spatial_layers import read_mini_layer
from mgb_ops.common.time_utils import DashboardWindow


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
def _mini_segments(
    gpkg_path: str, workspace: str, source_version: str
) -> gpd.GeoDataFrame:
    del workspace, source_version
    return read_mini_layer(Path(gpkg_path), "mini_segments")


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


__all__ = [
    "_accumulation_rasters",
    "_forecast_assets",
    "_forecast_preview",
    "_forecast_steps",
    "_mgb_series",
    "_mini_segments",
    "_model_variables",
    "_observed_series",
    "_station_catalog",
]
