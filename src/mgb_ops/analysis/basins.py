"""Basin membership, area weights, and dissolved geometry."""
from __future__ import annotations

from dataclasses import dataclass
import math
from pathlib import Path

import geopandas as gpd

from mgb_ops.assets.spatial_layers import read_mini_layer
from mgb_ops.utils.geospatial import dissolve_geometries
from mgb_ops.utils.topology import find_upstream_ids


@dataclass(frozen=True, slots=True)
class BasinSpatialData:
    mini_ids: tuple[int, ...]
    weights: tuple[float, ...]
    geometry: gpd.GeoDataFrame


def basin_spatial_data(catchments: gpd.GeoDataFrame, mini_id: int) -> BasinSpatialData:
    """Return upstream mini IDs, their validated areas, and dissolved geometry."""
    required = {"mini_jus", "mini_area"}
    missing = required.difference(catchments.columns)
    if missing:
        raise ValueError(
            f"Layer 'mini_catchments' is missing required columns: {sorted(missing)}."
        )
    basin_ids = find_upstream_ids(
        catchments, mini_id, id_col="mini_id", id_down_col="mini_jus"
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


def load_basin_spatial_data(gpkg_path: Path, mini_id: int) -> BasinSpatialData:
    """Load and summarize the mini-catchment layer for one outlet."""
    return basin_spatial_data(read_mini_layer(Path(gpkg_path), "mini_catchments"), mini_id)


__all__ = ["BasinSpatialData", "basin_spatial_data", "load_basin_spatial_data"]
