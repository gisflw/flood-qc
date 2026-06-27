"""Thin, UI-facing adapters around canonical library APIs."""
from __future__ import annotations

import json
from pathlib import Path

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


def load_rivers_layer_geojson(path: Path) -> dict | None:
    if not Path(path).exists():
        return None
    payload = json.loads(Path(path).read_text(encoding="utf-8"))
    if payload.get("type") != "FeatureCollection":
        return None
    for feature in payload.get("features", []):
        properties = feature.setdefault("properties", {})
        try:
            mini_id = int(properties.get("mini_id"))
        except (TypeError, ValueError):
            properties["click_id"] = "MINI|"
        else:
            properties["mini_id"] = mini_id
            properties["click_id"] = f"MINI|{mini_id}"
    return payload
