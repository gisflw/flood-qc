"""Panel-cached loaders with immutable inputs and no session-state access."""
from __future__ import annotations

from dataclasses import dataclass
import math
from pathlib import Path

import numpy as np
import pandas as pd
import panel as pn
import geopandas as gpd
from shapely.geometry import LineString, MultiLineString

from apps.ops_dashboard.services import forecast as dashboard_forecast
from mgb_ops.analysis import timeseries as dashboard_data
from mgb_ops.assets.spatial_grid import PrecipitationGrid, RegularGridSpec, read_spatial_grid_window
from mgb_ops.assets.spatial_layers import read_mini_layer
from mgb_ops.utils.geospatial import dissolve_geometries
from mgb_ops.assets.types import AnalysisWindow
from mgb_ops.utils.topology import find_upstream_ids


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
    window: AnalysisWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_station_catalog(
        Path(database_path),
        start_time=window.start_time,
        end_time=window.cutoff_time,
    )


@pn.cache(max_items=64)
def _station_rainfall_accumulations(
    database_path: str,
    workspace: str,
    source_version: str,
    start_time,
    end_time,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_station_rainfall_accumulations(
        Path(database_path), start_time=start_time, end_time=end_time
    )


@pn.cache(max_items=256)
def _observed_series(
    station_id: str,
    database_path: str,
    workspace: str,
    source_version: str,
    window: AnalysisWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_observed_series(
        station_id,
        Path(database_path),
        start_time=window.start_time,
        end_time=window.cutoff_time,
    )



@pn.cache(max_items=256)
def _station_reference_levels(
    station_id: str,
    database_path: str,
    workspace: str,
    source_version: str,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_station_reference_levels(station_id, Path(database_path))

@pn.cache(max_items=8)
def _mini_segments(
    gpkg_path: str, workspace: str, source_version: str
) -> gpd.GeoDataFrame:
    del workspace, source_version
    return read_mini_layer(Path(gpkg_path), "mini_segments")


def _quantized_path(geometry: LineString, *, precision: int = 5) -> list[list[float]]:
    return [
        [round(float(x), precision), round(float(y), precision)]
        for x, y, *_ in geometry.coords
    ]


@pn.cache(max_items=8)
def _mini_segment_paths(
    gpkg_path: str, workspace: str, source_version: str
) -> pd.DataFrame:
    """Return a compact, display-only PathLayer table for mini river segments."""
    segments = _mini_segments(gpkg_path, workspace, source_version)
    try:
        projected_crs = segments.estimate_utm_crs()
    except RuntimeError:
        projected_crs = None
    projected = segments.to_crs(projected_crs) if projected_crs is not None else segments
    simplified = projected.geometry.simplify(50.0, preserve_topology=True)
    if projected_crs is not None:
        simplified = gpd.GeoSeries(simplified, crs=projected_crs).to_crs(epsg=4326)
    rows: list[dict[str, object]] = []
    for mini_id, geometry in zip(segments["mini_id"], simplified, strict=True):
        if isinstance(geometry, LineString):
            lines = [geometry]
        elif isinstance(geometry, MultiLineString):
            lines = list(geometry.geoms)
        else:
            raise ValueError("Mini segment display geometries must be LineStrings.")
        path: list[list[float]] = []
        for line in lines:
            if line.is_empty or len(line.coords) < 2:
                continue
            coords = _quantized_path(line)
            if path and coords:
                path.extend(coords[1:] if path[-1] == coords[0] else coords)
            else:
                path.extend(coords)
        if len(path) < 2:
            raise ValueError(f"Mini segment {int(mini_id)} simplified to an empty path.")
        rows.append({"mini_id": int(mini_id), "path": path})
    return pd.DataFrame(rows, columns=["mini_id", "path"])


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
    required = {"mini_jus", "mini_area"}
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
    area_counts = selected.groupby("mini_id")["mini_area"].nunique(dropna=False)
    inconsistent = sorted(int(value) for value in area_counts[area_counts != 1].index)
    if inconsistent:
        raise ValueError(
            f"Catchments contain inconsistent mini_area values for minis: {inconsistent}."
        )
    areas = (
        selected[["mini_id", "mini_area"]]
        .drop_duplicates("mini_id")
        .set_index("mini_id")["mini_area"]
    )
    try:
        weights = tuple(float(areas.loc[value]) for value in basin_ids)
    except (TypeError, ValueError) as exc:
        raise ValueError("Catchment mini_area values must be numeric.") from exc
    if any(not math.isfinite(value) or value <= 0 for value in weights):
        raise ValueError("Catchment mini_area values must be finite and positive.")
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
    window: AnalysisWindow,
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
    window: AnalysisWindow,
) -> pd.DataFrame:
    del workspace, source_version
    return dashboard_data.load_mgb_series(
        Path(model_path),
        mini_id=mini_id,
        variable_code=variable_code,
        window=window,
    )


def prepare_mgb_level_series(
    model_levels: pd.DataFrame,
    station_observed: pd.DataFrame,
) -> pd.DataFrame:
    """Apply a latest-overlap station offset to one complete MGB level series."""
    if model_levels.empty or station_observed.empty or "prev_flag" not in model_levels:
        return model_levels.iloc[0:0].copy()
    station = station_observed.loc[
        station_observed["variable_code"] == "level", ["datetime", "value"]
    ].copy()
    model = model_levels.loc[
        model_levels["prev_flag"] == 0, ["dt", "value"]
    ].copy()
    station["datetime"] = pd.to_datetime(station["datetime"], errors="coerce")
    model["dt"] = pd.to_datetime(model["dt"], errors="coerce")
    station["value"] = pd.to_numeric(station["value"], errors="coerce")
    model["value"] = pd.to_numeric(model["value"], errors="coerce")
    overlap = station.dropna().merge(
        model.dropna(), left_on="datetime", right_on="dt", suffixes=("_station", "_mgb")
    )
    if overlap.empty:
        return model_levels.iloc[0:0].copy()
    latest = overlap.sort_values("datetime").iloc[-1]
    prepared = model_levels.copy()
    prepared["value"] = pd.to_numeric(prepared["value"], errors="coerce") + (
        float(latest["value_station"]) - float(latest["value_mgb"])
    )
    return prepared


@pn.cache(max_items=256)
def _prepared_mgb_level(
    mini_id: int,
    model_path: str,
    workspace: str,
    model_version: str,
    database_path: str,
    window: AnalysisWindow,
) -> pd.DataFrame:
    """Build a cached, dashboard-only aligned level series for one mini."""
    del workspace, model_version
    station_id = dashboard_data.load_mini_station_id(mini_id, Path(database_path))
    model_levels = dashboard_data.load_mgb_series(
        Path(model_path), mini_id=mini_id, variable_code="level", window=window
    )
    if station_id is None:
        return model_levels.iloc[0:0].copy()
    station_observed = dashboard_data.load_observed_series(
        station_id, Path(database_path), start_time=window.start_time, end_time=window.cutoff_time
    )
    return prepare_mgb_level_series(model_levels, station_observed)


@pn.cache(max_items=256)
def _basin_precipitation(
    mini_ids: tuple[int, ...],
    weights: tuple[float, ...],
    model_path: str,
    workspace: str,
    source_version: str,
    window: AnalysisWindow,
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
    window: AnalysisWindow,
    bbox: tuple[float, float, float, float],
    resolution: float,
    hours: int,
    *,
    rainfall_mode: str = "observed",
) -> dict[str, object]:
    del workspace, source_version
    if not isinstance(hours, int) or isinstance(hours, bool) or hours < 1:
        raise ValueError("Rainfall accumulation hours must be a positive integer.")
    if rainfall_mode not in {"observed", "forecast"}:
        raise ValueError("Rainfall mode must be 'observed' or 'forecast'.")
    expected_grid = RegularGridSpec(
        bbox=bbox,
        resolution=resolution,
        include_boundary_cells=True,
    )
    reference_utc = pd.Timestamp(
        window.cutoff_time, tz="America/Sao_Paulo"
    ).tz_convert("UTC")
    if rainfall_mode == "observed":
        start_utc = reference_utc - pd.Timedelta(hours=hours)
        end_utc = reference_utc
    else:
        start_utc = reference_utc
        end_utc = reference_utc + pd.Timedelta(hours=hours)
    cached = read_spatial_grid_window(Path(cache_path), start_time=start_utc, end_time=end_utc)
    if cached.variable != "precipitation" or cached.grid_type != rainfall_mode:
        raise ValueError(
            f"{rainfall_mode.title()} rainfall cache must contain "
            f"{rainfall_mode} precipitation."
        )
    expected_source = (
        "interpolated_from_stations"
        if rainfall_mode == "observed"
        else "resampled_from_grid"
    )
    if cached.source != expected_source:
        raise ValueError(
            f"{rainfall_mode.title()} rainfall cache must have source "
            f"'{expected_source}'."
        )
    if not (
        np.allclose(cached.latitudes, expected_grid.latitudes)
        and np.allclose(cached.longitudes, expected_grid.longitudes)
    ):
        raise ValueError(
            f"{rainfall_mode.title()} rainfall cache does not match spatial_grid."
        )
    indices = [
        index
        for index, (left, right) in enumerate(cached.time_bounds_utc)
        if pd.Timestamp(left) >= start_utc and pd.Timestamp(right) <= end_utc
    ]
    if not indices:
        raise ValueError(
            f"{rainfall_mode.title()} rainfall cache does not cover the "
            "requested accumulation window."
        )
    selected_bounds = [
        tuple(pd.Timestamp(value) for value in cached.time_bounds_utc[index])
        for index in indices
    ]
    complete = (
        selected_bounds[0][0] == start_utc
        and selected_bounds[-1][1] == end_utc
        and all(
            left[1] == right[0]
            for left, right in zip(selected_bounds, selected_bounds[1:])
        )
    )
    if not complete:
        raise ValueError(
            f"{rainfall_mode.title()} rainfall cache incompletely covers the "
            "requested accumulation window."
        )
    selected = cached.values[indices]
    accumulated = np.nansum(selected, axis=0)
    accumulated[np.all(~np.isfinite(selected), axis=0)] = np.nan
    rainfall = PrecipitationGrid(
        values=accumulated,
        latitudes=cached.latitudes,
        longitudes=cached.longitudes,
        bounds=expected_grid.effective_bbox,
        start_time=start_utc,
        end_time=end_utc,
        units=cached.units,
        source=str(cache_path),
    )
    return {
        "name": f"{rainfall_mode}_accum_{hours}h",
        "rainfall_mode": rainfall_mode,
        "horizon_hours": hours,
        "horizon_label": f"{hours}h",
        "grid": rainfall,
    }


def parse_signed_rainfall_period(period: int) -> tuple[str, int]:
    if not isinstance(period, int) or isinstance(period, bool):
        raise ValueError("Rainfall period must be an integer from -999..-1 or 1..999.")
    if period == 0 or abs(period) > 999:
        raise ValueError("Rainfall period must be -999..-1 or 1..999; zero is not valid.")
    return ("observed", abs(period)) if period < 0 else ("forecast", period)


@pn.cache(max_items=16)
def _forecast_assets(
    database_path: str,
    workspace: str,
    source_version: str,
    window: AnalysisWindow,
    provider_code: str,
    lookback_cycles: int,
) -> pd.DataFrame:
    del source_version
    return dashboard_forecast.list_forecast_assets(
        Path(database_path),
        Path(workspace),
        window=window,
        provider_code=provider_code,
        lookback_cycles=lookback_cycles,
    )


@pn.cache(max_items=128)
def _forecast_steps(
    asset_id: str,
    database_path: str,
    workspace: str,
    source_version: str,
    window: AnalysisWindow,
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
    window: AnalysisWindow,
) -> dashboard_forecast.ForecastPreview:
    del source_version, window
    return dashboard_forecast.build_forecast_preview(
        asset_id,
        t0_step=t0_step,
        t1_step=t1_step,
        database_path=Path(database_path),
        workspace_path=Path(workspace),
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
    "_mini_segment_paths",
    "_mini_segments",
    "_model_variables",
    "_prepared_mgb_level",
    "prepare_mgb_level_series",
    "_observed_series",
    "_station_catalog",
    "_station_rainfall_accumulations",
    "parse_signed_rainfall_period",
]
