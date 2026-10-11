"""Reusable alignment of MGB level output to station observations."""
from __future__ import annotations

import pandas as pd


def prepare_mgb_level_series(
    model_levels: pd.DataFrame,
    station_observed: pd.DataFrame,
) -> pd.DataFrame:
    """Apply the latest shared observed/model level offset to the full model series."""
    if model_levels.empty or station_observed.empty or "prev_flag" not in model_levels:
        return model_levels.iloc[0:0].copy()
    station = station_observed.loc[
        station_observed["variable_code"] == "level", ["datetime", "value"]
    ].copy()
    model = model_levels.loc[model_levels["prev_flag"] == 0, ["dt", "value"]].copy()
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


__all__ = ["prepare_mgb_level_series"]
