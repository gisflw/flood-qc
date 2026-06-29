"""Pure Plotly chart builders."""
from __future__ import annotations

import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots


def _station_chart(frame: pd.DataFrame, station_id: str | None) -> go.Figure:
    fig = make_subplots(
        rows=2,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.08,
        row_heights=[0.35, 0.65],
    )
    if frame.empty:
        fig.add_annotation(
            text="Select a station with data on the map.",
            x=0.5,
            y=0.5,
            xref="paper",
            yref="paper",
            showarrow=False,
        )
    else:
        styles = {
            "rain": ("bar", "#4c6ef5", "Rainfall (mm)", 1),
            "level": ("scatter", "#0b7285", "Level (cm)", 2),
            "flow": ("scatter", "#1864ab", "Flow (m³/s)", 2),
        }
        for code, (kind, color, label, row) in styles.items():
            data = frame[frame["variable_code"] == code].dropna(subset=["value"])
            if data.empty:
                continue
            if kind == "bar":
                fig.add_bar(
                    x=data["datetime"],
                    y=data["value"],
                    name=label,
                    marker_color=color,
                    row=row,
                    col=1,
                )
            else:
                fig.add_scatter(
                    x=data["datetime"],
                    y=data["value"],
                    name=label,
                    mode="lines",
                    line={"color": color},
                    row=row,
                    col=1,
                )
    fig.update_layout(
        template="plotly_white",
        height=420,
        margin={"t": 30, "r": 15, "b": 25, "l": 35},
        hovermode="x unified",
        title=f"Station {station_id}" if station_id else "Station series",
    )
    return fig


def _mgb_chart(frame: pd.DataFrame, mini_id: int | None, code: str) -> go.Figure:
    fig = go.Figure()
    if not frame.empty:
        display = str(frame["display_name"].iloc[0])
        unit = str(frame["unit"].iloc[0])
        for flag, dash, suffix in ((0, "solid", "current"), (1, "dash", "forecast")):
            subset = frame[frame["prev_flag"] == flag]
            if not subset.empty:
                fig.add_scatter(
                    x=subset["dt"],
                    y=subset["value"],
                    mode="lines",
                    name=f"{display} {suffix}",
                    line={"color": "#0b7285" if code == "y" else "#1864ab", "dash": dash},
                )
        fig.update_yaxes(title=f"{display} ({unit})")
    else:
        fig.add_annotation(
            text="Select a mini with model output.",
            x=0.5,
            y=0.5,
            xref="paper",
            yref="paper",
            showarrow=False,
        )
    fig.update_layout(
        template="plotly_white",
        height=280,
        margin={"t": 30, "r": 15, "b": 25, "l": 35},
        hovermode="x unified",
        title=f"Mini {mini_id} · {code.upper()}" if mini_id else code.upper(),
    )
    return fig
