"""Explicit-input summary panes for the dashboard."""
from __future__ import annotations

from typing import Any

import numpy as np
import pandas as pd
import panel as pn

from apps.ops_dashboard.state import DashboardState
from mgb_ops.analysis.timeseries import compute_observed_metrics, summarize_mini_peaks


def _format_number(value: Any, unit: str, precision: int = 1) -> str:
    if value is None or pd.isna(value):
        return "unavailable"
    return f"{float(value):.{precision}f} {unit}".strip()


def _network_summary(stations: pd.DataFrame) -> pn.viewable.Viewable:
    if stations.empty:
        values = (0, 0, 0, 0, np.nan, np.nan)
    else:
        rain = stations.loc[
            stations.get("status", pd.Series(index=stations.index)) == "ok",
            "rain_acc_24h_mm",
        ].dropna()
        if rain.empty and "rain_acc_24h_mm" in stations:
            rain = stations["rain_acc_24h_mm"].dropna()
        values = (
            len(stations),
            int((stations["status"] == "ok").sum()),
            int((stations["status"] == "no_data").sum()),
            int((stations["status"] == "data_issue").sum()),
            rain.mean() if not rain.empty else np.nan,
            rain.quantile(0.9) if not rain.empty else np.nan,
        )
    cards = [
        ("Total stations", str(values[0])),
        ("With data", str(values[1])),
        ("No data", str(values[2])),
        ("Data issue", str(values[3])),
        ("Mean rainfall 24h", _format_number(values[4], "mm")),
        ("P90 rainfall 24h", _format_number(values[5], "mm")),
    ]
    return pn.Row(
        *[
            pn.Card(
                pn.pane.HTML(
                    f"<div style='font-size:0.78rem;line-height:1.15'>"
                    f"<strong style='font-size:1.15rem'>{value}</strong><br>{label}</div>"
                ),
                hide_header=True,
                width=145,
                margin=3,
            )
            for label, value in cards
        ],
        sizing_mode="stretch_width",
        styles={"overflow-x": "auto", "flex-wrap": "nowrap"},
    )


def _station_summary(state: DashboardState) -> pn.viewable.Viewable:
    station_id = state.station_id
    if station_id is None:
        return pn.pane.Alert("Click a station on the map.", alert_type="info")
    selected = state.stations[state.stations["station_id"].astype(str) == station_id]
    if selected.empty:
        return pn.pane.Alert("The selected station is unavailable.", alert_type="warning")
    row = selected.iloc[0]
    observed = state.observed_series()
    if observed.empty:
        detail = "No observed data in the dashboard window."
    else:
        metrics = compute_observed_metrics(observed, cutoff_time=state.window.cutoff_time)
        detail = (
            f"Latest: {pd.Timestamp(metrics['latest_time']):%d/%m %H:%M}  \n"
            f"Rainfall 12h: {_format_number(metrics['rain_12h'], 'mm')} · "
            f"24h: {_format_number(metrics['rain_24h'], 'mm')} · "
            f"72h: {_format_number(metrics['rain_72h'], 'mm')}  \n"
            f"Level: {_format_number(metrics['level_current'], 'cm', 2)} · "
            f"Flow: {_format_number(metrics['flow_current'], 'm³/s', 2)}"
        )
    return pn.pane.Markdown(
        f"### {row['station_name']}\n"
        f"{str(row['provider_code']).upper()}:{row['station_code']}  \n{detail}"
    )


def _mini_summary(state: DashboardState) -> pn.viewable.Viewable:
    if state.mini_id is None:
        return pn.pane.Alert("Click a mini catchment or river.", alert_type="info")
    try:
        levels = state.mgb_series("level")
    except (FileNotFoundError, OSError, ValueError) as exc:
        return pn.pane.Alert(str(exc), alert_type="warning")
    if levels.empty:
        return pn.pane.Alert("No level series for this mini.", alert_type="info")
    summary = summarize_mini_peaks(
        levels,
        cutoff_time=state.window.cutoff_time,
        forecast_end_exclusive=state.window.forecast_end_exclusive,
    )
    return pn.pane.Markdown(
        f"### Mini {state.mini_id}\n"
        f"Current level: {_format_number(summary['current_value'], 'cm', 2)}  \n"
        f"Observed-window peak: {_format_number(summary['current_peak'], 'cm', 2)} · "
        f"Forecast-window peak: {_format_number(summary['forecast_peak'], 'cm', 2)}"
    )
