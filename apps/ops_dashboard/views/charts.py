"""Pure Plotly chart builders."""
from __future__ import annotations

from collections.abc import Mapping

import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots


STATION_COLOR = "#1864ab"
MINI_COLOR = "#e8590c"
VARIABLE_ROWS = {
    "precipitation": 1,
    "flow": 2,
    "level": 3,
}


def _comparison_chart(
    observed: pd.DataFrame,
    model_series: Mapping[str, pd.DataFrame],
    station_id: str | None,
    mini_id: int | None,
) -> go.Figure:
    """Build one ordered station/mini comparison figure."""
    fig = make_subplots(
        rows=3,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.08,
        subplot_titles=("Precipitation", "Flow", "Water Level Variation"),
    )

    station_codes = {
        "precipitation": "rain",
        "level": "level",
        "flow": "flow",
    }
    station_units = {
        "precipitation": "mm",
        "level": "cm",
        "flow": "m³/s",
    }
    has_station_data = False
    for variable_code, observed_code in station_codes.items():
        if observed.empty:
            break
        data = observed[
            observed["variable_code"] == observed_code
        ].dropna(subset=["value"])
        if data.empty:
            continue
        if variable_code == "level":
            data = data.assign(value=data["value"] - data["value"].mean())
        row = VARIABLE_ROWS[variable_code]
        name = f"Station {station_id} · {station_units[variable_code]}"
        if variable_code == "precipitation":
            fig.add_bar(
                x=data["datetime"],
                y=data["value"],
                name=name,
                marker_color=STATION_COLOR,
                legendgroup="station",
                showlegend=False,
                row=row,
                col=1,
            )
        else:
            fig.add_scatter(
                x=data["datetime"],
                y=data["value"],
                name=name,
                mode="lines",
                line={"color": STATION_COLOR},
                legendgroup="station",
                showlegend=False,
                row=row,
                col=1,
            )
        has_station_data = True

    nested = bool(model_series) and all(
        isinstance(value, Mapping) for value in model_series.values()
    )
    scenario_groups = (
        list(model_series.items())
        if nested
        else [(None, model_series)]
    )
    scenario_colors = (
        MINI_COLOR,
        "#2f9e44",
        "#7048e8",
        "#d9480f",
        "#0b7285",
        "#c2255c",
    )
    visible_scenarios: list[tuple[str, str]] = []
    for scenario_index, (scenario_label, variables) in enumerate(scenario_groups):
        color = scenario_colors[scenario_index % len(scenario_colors)]
        for variable_code, row in VARIABLE_ROWS.items():
            frame = variables.get(variable_code, pd.DataFrame())
            if frame.empty:
                continue
            level_mean = None
            if variable_code == "level":
                current_levels = frame[frame["prev_flag"] == 0].dropna(
                    subset=["value"]
                )
                if current_levels.empty:
                    continue
                level_mean = current_levels["value"].mean()
            for flag, dash, suffix in (
                (0, "solid", "observed"),
                (1, "dash", "forecast"),
            ):
                data = frame[frame["prev_flag"] == flag].dropna(subset=["value"])
                if data.empty:
                    continue
                values = (
                    data["value"] - level_mean
                    if variable_code == "level"
                    else data["value"]
                )
                base_name = (
                    f"Basin precipitation - {suffix}"
                    if variable_code == "precipitation"
                    else f"Mini {mini_id} - {suffix}"
                )
                name = (
                    f"{scenario_label} - {base_name}"
                    if scenario_label is not None
                    else base_name
                )
                legend_group = (
                    f"{scenario_label}-{suffix}"
                    if scenario_label is not None
                    else f"mini-{suffix}"
                )
                if scenario_label is not None and (scenario_label, color) not in visible_scenarios:
                    visible_scenarios.append((scenario_label, color))
                if variable_code == "precipitation":
                    fig.add_bar(
                        x=data["dt"],
                        y=values,
                        name=name,
                        marker={
                            "color": color,
                            "pattern": {"shape": "/" if flag else ""},
                        },
                        opacity=0.85 if flag == 0 else 0.55,
                        legendgroup=legend_group,
                        showlegend=False,
                        row=row,
                        col=1,
                    )
                else:
                    fig.add_scatter(
                        x=data["dt"],
                        y=values,
                        name=name,
                        mode="lines",
                        line={"color": color, "dash": dash},
                        legendgroup=legend_group,
                        showlegend=False,
                        row=row,
                        col=1,
                    )

    has_series_data = bool(fig.data)
    if has_station_data:
        fig.add_scatter(
            x=[None], y=[None], name=f"Station {station_id}", mode="lines",
            line={"color": STATION_COLOR}, showlegend=True, visible="legendonly"
        )
    for scenario_label, color in visible_scenarios:
        fig.add_scatter(
            x=[None], y=[None], name=scenario_label, mode="lines",
            line={"color": color}, showlegend=True, visible="legendonly"
        )
    if has_series_data:
        fig.add_scatter(
            x=[None], y=[None], name="Observed", mode="lines",
            line={"color": "#adb5bd"}, showlegend=True, visible="legendonly"
        )
        fig.add_scatter(
            x=[None], y=[None], name="Forecast", mode="lines",
            line={"color": "#adb5bd", "dash": "dash"}, showlegend=True, visible="legendonly"
        )

    if not fig.data:
        text = (
            "Click a station and/or a mini on the map."
            if station_id is None and mini_id is None
            else "No series data are available for the current selection."
        )
        fig.add_annotation(
            text=text,
            x=0.5,
            y=0.5,
            xref="paper",
            yref="paper",
            showarrow=False,
        )

    selections = []
    if station_id is not None:
        selections.append(f"Station {station_id}")
    if mini_id is not None:
        selections.append(f"Mini {mini_id}")
    fig.update_yaxes(title_text="mm", row=1, col=1)
    fig.update_yaxes(title_text="m³/s", row=2, col=1)
    fig.update_yaxes(title_text="cm", row=3, col=1)
    fig.update_layout(
        template="plotly_white",
        height=720,
        margin={"t": 55, "r": 15, "b": 25, "l": 55},
        hovermode="x unified",
        barmode="group",
        title=" · ".join(selections) if selections else "Observed and Modeled Comparison",
    )
    return fig
