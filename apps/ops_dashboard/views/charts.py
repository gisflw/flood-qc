"""Pure Plotly chart builders."""

from __future__ import annotations

from collections.abc import Mapping

import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots

from apps.ops_dashboard.services.formatting import truncate_series_one_decimal

STATION_COLOR = "#1864ab"
MINI_COLOR = "#e8590c"
VARIABLE_ROWS = {"precipitation": 1, "flow": 2, "level": 3}


def _comparison_chart(
    observed: pd.DataFrame,
    model_series: Mapping[str, pd.DataFrame],
    station_id: str | None,
    mini_id: int | None,
    reference_levels: pd.DataFrame | None = None,
) -> go.Figure:
    """Build one ordered station/mini comparison figure from prepared series."""
    fig = make_subplots(
        rows=3, cols=1, shared_xaxes=True, vertical_spacing=0.08,
        subplot_titles=("Precipitation", "Flow", "Water Level Variation"),
    )
    station_codes = {"precipitation": "rain", "level": "level", "flow": "flow"}
    station_units = {"precipitation": "mm", "level": "cm", "flow": "m³/s"}
    has_station_data = False
    for variable_code, observed_code in station_codes.items():
        if observed.empty:
            break
        data = observed[observed["variable_code"] == observed_code].dropna(subset=["value"]).copy()
        if data.empty:
            continue
        data["value"] = truncate_series_one_decimal(data["value"])
        row, name = VARIABLE_ROWS[variable_code], f"Station {station_id} · {station_units[variable_code]}"
        if variable_code == "precipitation":
            fig.add_bar(x=data["datetime"], y=data["value"], name=name, marker_color=STATION_COLOR,
                        legendgroup="station", showlegend=False, row=row, col=1)
        else:
            fig.add_scatter(x=data["datetime"], y=data["value"], name=name, mode="lines",
                            line={"color": STATION_COLOR}, legendgroup="station", showlegend=False, row=row, col=1)
        has_station_data = True

    nested = bool(model_series) and all(isinstance(value, Mapping) for value in model_series.values())
    scenario_groups = list(model_series.items()) if nested else [(None, model_series)]
    colors = (MINI_COLOR, "#2f9e44", "#7048e8", "#d9480f", "#0b7285", "#c2255c")
    scenario_legends, basin_observed_added = [], False
    for index, (label, variables) in enumerate(scenario_groups):
        color, group, has_data = colors[index % len(colors)], f"scenario:{label}" if label else "basin-forecast", False
        for variable_code, row in VARIABLE_ROWS.items():
            frame = variables.get(variable_code, pd.DataFrame())
            if frame.empty:
                continue
            for flag, dash, suffix in ((0, "solid", "observed"), (1, "dash", "forecast")):
                data = frame[frame["prev_flag"] == flag].dropna(subset=["value"]).copy()
                if data.empty:
                    continue
                if variable_code == "precipitation" and flag == 0:
                    if basin_observed_added:
                        continue
                    basin_observed_added = True
                    data["value"] = truncate_series_one_decimal(data["value"])
                    fig.add_bar(x=data["dt"], y=data["value"], name="Basin precipitation", marker_color=MINI_COLOR,
                                opacity=0.85, legendgroup="basin-observed", showlegend=False, row=row, col=1)
                    continue
                if variable_code == "precipitation" and flag == 1 and data["value"].eq(0).all():
                    continue
                data["value"] = truncate_series_one_decimal(data["value"])
                base = f"Basin precipitation - {suffix}" if variable_code == "precipitation" else f"Mini {mini_id} - {suffix}"
                name = f"{label} - {base}" if label else base
                has_data = True
                if variable_code == "precipitation":
                    fig.add_bar(x=data["dt"], y=data["value"], name=name, marker={"color": color, "pattern": {"shape": "/"}},
                                opacity=0.55, legendgroup=group, showlegend=False, row=row, col=1)
                else:
                    fig.add_scatter(x=data["dt"], y=data["value"], name=name, mode="lines",
                                    line={"color": color, "dash": dash}, legendgroup=group, showlegend=False, row=row, col=1)
        if has_data:
            scenario_legends.append((label or "Basin forecast", color, group))

    reference_styles = {
        "attention": ("Attention", "#facc15", "solid"),
        "alert": ("Alert", "#d4a017", "solid"),
        "flood": ("Flood", "#dc2626", "solid"),
        "severe": ("Severe", "#3b0764", "solid"),
        "historical_flood": ("Historical flood", "#2563eb", "dash"),
    }
    if reference_levels is not None and not reference_levels.empty:
        for _, reference in reference_levels.iterrows():
            reference_type = str(reference["reference_type"])
            style = reference_styles.get(reference_type)
            level = pd.to_numeric(reference["level_cm"], errors="coerce")
            if style is None or pd.isna(level):
                continue
            label, color, dash = style
            event_date = reference.get("event_date")
            if reference_type == "historical_flood" and pd.notna(event_date):
                label = f"{label} · {event_date}"

            fig.add_shape(
                type="line",
                x0=0,
                x1=1,
                xref="x3 domain",
                y0=float(level),
                y1=float(level),
                yref="y3",
                line={"color": color, "dash": dash, "width": 2},
            )
            fig.add_annotation(
                x=1,
                xref="x3 domain",
                y=float(level),
                yref="y3",
                text=label,
                showarrow=False,
                xanchor="right",
                yanchor="bottom",
            )
    def add_legend(name: str, color: str, group: str) -> None:
        fig.add_bar(x=[None], y=[None], name=name, marker_color=color, legendgroup=group, showlegend=True, hoverinfo="skip")

    if has_station_data:
        add_legend(f"Station {station_id}", STATION_COLOR, "station")
    if basin_observed_added:
        add_legend("Basin precipitation", MINI_COLOR, "basin-observed")
    for label, color, group in scenario_legends:
        add_legend(label, color, group)
    if not fig.data:
        text = "Click a station and/or a mini on the map." if station_id is None and mini_id is None else "No series data are available for the current selection."
        fig.add_annotation(text=text, x=0.5, y=0.5, xref="paper", yref="paper", showarrow=False)
    selections = []
    if station_id is not None:
        selections.append(f"Station {station_id}")
    if mini_id is not None:
        selections.append(f"Mini {mini_id}")
    fig.update_yaxes(title_text="mm", row=1, col=1)
    fig.update_yaxes(title_text="m³/s", row=2, col=1)
    fig.update_yaxes(title_text="cm", row=3, col=1)
    fig.update_layout(template="plotly_white", height=720, margin={"t": 55, "r": 15, "b": 25, "l": 55},
                      hovermode="x unified", barmode="group", legend={"groupclick": "togglegroup"},
                      title=" · ".join(selections) if selections else "Observed and Modeled Comparison")
    return fig
