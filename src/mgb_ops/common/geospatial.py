from __future__ import annotations

from typing import Any

import geopandas as gpd


def dissolve_geometries(
    frame: gpd.GeoDataFrame,
    *,
    attributes: dict[str, Any] | None = None,
) -> gpd.GeoDataFrame:
    """Dissolve every feature into one geometry while preserving the source CRS."""
    if not isinstance(frame, gpd.GeoDataFrame):
        raise TypeError("frame must be a GeoDataFrame.")
    if frame.empty:
        raise ValueError("Cannot dissolve an empty GeoDataFrame.")
    if frame.crs is None:
        raise ValueError("Cannot dissolve geometries without a CRS.")
    if frame.geometry.isna().any() or frame.geometry.is_empty.any():
        raise ValueError("Cannot dissolve null or empty geometries.")
    if not frame.geometry.is_valid.all():
        raise ValueError("Cannot dissolve invalid geometries.")
    geometry = frame.geometry.union_all()
    if geometry is None or geometry.is_empty or not geometry.is_valid:
        raise ValueError("Dissolved geometry is empty or invalid.")
    return gpd.GeoDataFrame(
        [attributes or {}],
        geometry=[geometry],
        crs=frame.crs,
    )


__all__ = ["dissolve_geometries"]
