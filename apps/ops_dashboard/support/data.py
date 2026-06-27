"""Thin, UI-facing adapters around canonical library APIs."""
from __future__ import annotations

from pathlib import Path

import geopandas as gpd

from mgb_ops.analysis.timeseries import (
    compute_observed_metrics,
    compute_rain_summary,
    derive_station_kind,
    list_model_variables,
    load_mgb_series,
    load_observed_series,
    load_station_catalog,
    select_preferred_series_rows,
    summarize_mini_peaks,
    summarize_station_status,
    validate_model_outputs_netcdf,
)
from mgb_ops.analysis.spatial_layers import MiniSpatialLayers, read_mini_layer, read_mini_layers


def layer_geojson(frame: gpd.GeoDataFrame) -> dict:
    return frame.__geo_interface__


__all__ = [
    "MiniSpatialLayers",
    "layer_geojson",
    "read_mini_layer",
    "read_mini_layers",
]
