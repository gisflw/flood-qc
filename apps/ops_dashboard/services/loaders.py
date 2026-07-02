"""Panel-cached loaders with immutable inputs and no session-state access."""
from __future__ import annotations

from dataclasses import dataclass
import math
from pathlib import Path

import numpy as np
import pandas as pd
import panel as pn
import geopandas as gpd

from apps.ops_dashboard.services import forecast as dashboard_forecast
from mgb_ops.analysis import timeseries as dashboard_data
from mgb_ops.analysis.spatial import PrecipitationGrid, RegularGridSpec
from mgb_ops.assets.spatial_grid import read_spatial_grid
from mgb_ops.assets.spatial_layers import read_mini_layer
from mgb_ops.common import dissolve_geometries, find_upstream_ids
from mgb_ops.common.time_utils import DashboardWindow


@dataclass(frozen=True, slots=True)
class BasinSpatialData:
    mini_ids: tuple[int, ...]
    weights: tuple[float, ...]
    geometry: gpd.GeoDataFrame


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
def _mini_catchments(
    gpkg_path: str, workspace: str, source_version: str
) -> gpd.GeoDataFrame:
    del workspace, source_version
    return read_mini_layer(Path(gpkg_path), "mini_catchments")


@pn.cache(max_items=128)
def _basin_spatial_data(
    mini_id: int,
    gpkg_path: str,
    workspace: str,
    source_version: str,
) -> BasinSpatialData:
    catchments = _mini_catchments(gpkg_path, workspace, source_version)
    required = {"mini_jus", "area_km2"}
    missing = required.difference(catchments.columns)
    if missing:
        raise ValueError(
            f"Layer 'mini_catchments' is missing required columns: {sorted(missing)}."
        )
    basin_ids = find_upstream_ids(
        catchments,
        mini_id,
        id_col="mini_id",
        id_down_col="mini_jus",
    )
    selected = catchments[catchments["mini_id"].isin(basin_ids)].copy()
    area_counts = selected.groupby("mini_id")["area_km2"].nunique(dropna=False)
    inconsistent = sorted(int(value) for value in area_counts[area_counts != 1].index)
    if inconsistent:
        raise ValueError(
            f"Catchments contain inconsistent area_km2 values for minis: {inconsistent}."
        )
    areas = (
        selected[["mini_id", "area_km2"]]
        .drop_duplicates("mini_id")
        .set_index("mini_id")["area_km2"]
    )
    try:
        weights = tuple(float(areas.loc[value]) for value in basin_ids)
    except (TypeError, ValueError) as exc:
        raise ValueError("Catchment area_km2 values must be numeric.") from exc
    if any(not math.isfinite(value) or value <= 0 for value in weights):
        raise ValueError("Catchment area_km2 values must be finite and positive.")
    dissolved = dissolve_geometries(
        selected,
        attributes={"outlet_mini_id": int(mini_id), "mini_count": len(basin_ids)},
    )
    return BasinSpatialData(tuple(basin_ids), weights, dissolved)


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


@pn.cache(max_items=256)
def _basin_precipitation(
    mini_ids: tuple[int, ...],
    weights: tuple[float, ...],
    model_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_basin_precipitation(
        Path(model_path),
        mini_ids=mini_ids,
        weights=weights,
        window=window,
    )


@pn.cache(max_items=32)
def _accumulation_raster(
    cache_path: str,
    workspace: str,
    source_version: str,
    window: DashboardWindow,
    bbox: tuple[float, float, float, float],
    resolution: float,
    hours: int,
) -> dict[str, object]:
    del workspace, source_version
    if not isinstance(hours, int) or isinstance(hours, bool) or hours < 1:
        raise ValueError("Rainfall accumulation hours must be a positive integer.")
    expected_grid = RegularGridSpec(bbox=bbox, resolution=resolution)
    cached = read_spatial_grid(Path(cache_path))
    if cached.variable != "precipitation" or cached.grid_type != "observed":
        raise ValueError("Observed rainfall cache must contain observed precipitation.")
    if cached.source != "interpolated_from_stations":
        raise ValueError("Observed rainfall cache must be interpolated from stations.")
    if not (
        np.allclose(cached.latitudes, expected_grid.latitudes)
        and np.allclose(cached.longitudes, expected_grid.longitudes)
    ):
        raise ValueError("Observed rainfall cache does not match spatial_grid.")
    end_utc = pd.Timestamp(window.cutoff_time, tz="America/Sao_Paulo").tz_convert("UTC")
    start_local = max(
        window.start_time,
        window.cutoff_time - pd.Timedelta(hours=hours).to_pytimedelta(),
    )
    start_utc = pd.Timestamp(start_local, tz="America/Sao_Paulo").tz_convert("UTC")
    indices = [
        index
        for index, (left, right) in enumerate(cached.time_bounds_utc)
        if pd.Timestamp(left) >= start_utc and pd.Timestamp(right) <= end_utc
    ]
    if not indices:
        raise ValueError("Observed rainfall cache does not cover the requested accumulation window.")
    if pd.Timestamp(cached.time_bounds_utc[indices[0]][0]) != start_utc or pd.Timestamp(
        cached.time_bounds_utc[indices[-1]][1]
    ) != end_utc:
        raise ValueError("Observed rainfall cache incompletely covers the requested accumulation window.")
    selected = cached.values[indices]
    accumulated = np.nansum(selected, axis=0)
    accumulated[np.all(~np.isfinite(selected), axis=0)] = np.nan
    rainfall = PrecipitationGrid(
        values=accumulated,
        latitudes=cached.latitudes,
        longitudes=cached.longitudes,
        bounds=expected_grid.bbox,
        start_time=start_utc,
        end_time=end_utc,
        units=cached.units,
        source=str(cache_path),
    )
    return {
        "name": f"accum_{hours}h",
        "horizon_hours": hours,
        "horizon_label": f"{hours}h",
        "grid": rainfall,
    }


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
    "_accumulation_raster",
    "_basin_precipitation",
    "_basin_spatial_data",
    "_forecast_assets",
    "_forecast_preview",
    "_forecast_steps",
    "_mgb_series",
    "_mini_catchments",
    "_mini_segments",
    "_model_variables",
    "_observed_series",
    "_station_catalog",
]
