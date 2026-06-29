from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import geopandas as gpd
import pandas as pd


MINI_LAYER_NAMES = ("mini_segments", "mini_catchments")


@dataclass(frozen=True, slots=True)
class MiniSpatialLayers:
    mini_segments: gpd.GeoDataFrame
    mini_catchments: gpd.GeoDataFrame


def read_mini_layer(gpkg_path: Path, layer_name: str) -> gpd.GeoDataFrame:
    source = Path(gpkg_path)
    if layer_name not in MINI_LAYER_NAMES:
        raise ValueError(f"layer_name must be one of {MINI_LAYER_NAMES}.")
    if not source.exists():
        raise FileNotFoundError(f"Mini-layer GeoPackage not found: {source}")
    try:
        frame = gpd.read_file(source, layer=layer_name)
    except Exception as exc:
        raise ValueError(f"Unable to read layer {layer_name!r} from {source}: {exc}") from exc
    if "mini_id" not in frame.columns:
        raise ValueError(f"Layer {layer_name!r} is missing required column 'mini_id'.")
    if frame.crs is None:
        raise ValueError(f"Layer {layer_name!r} has no CRS.")
    if frame.geometry.isna().any() or frame.geometry.is_empty.any():
        raise ValueError(f"Layer {layer_name!r} contains null or empty geometries.")
    if not frame.geometry.is_valid.all():
        raise ValueError(f"Layer {layer_name!r} contains invalid geometries.")
    try:
        numeric_ids = pd.to_numeric(frame["mini_id"], errors="raise")
    except (TypeError, ValueError) as exc:
        raise ValueError(f"Layer {layer_name!r} contains non-integer mini_id values.") from exc
    if numeric_ids.isna().any() or (numeric_ids % 1 != 0).any():
        raise ValueError(f"Layer {layer_name!r} contains non-integer mini_id values.")
    mini_ids = numeric_ids.astype("int64")
    normalized = frame.to_crs(epsg=4326).copy()
    normalized["mini_id"] = mini_ids.to_numpy()
    normalized["click_id"] = normalized["mini_id"].map(lambda value: f"MINI|{int(value)}")
    return normalized


def read_mini_layers(gpkg_path: Path) -> MiniSpatialLayers:
    return MiniSpatialLayers(
        mini_segments=read_mini_layer(gpkg_path, "mini_segments"),
        mini_catchments=read_mini_layer(gpkg_path, "mini_catchments"),
    )
