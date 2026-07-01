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
                row=row,
                col=1,
            )

    for variable_code, row in VARIABLE_ROWS.items():
        frame = model_series.get(variable_code, pd.DataFrame())
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
            (0, "solid", "current"),
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
            name = (
                f"Basin precipitation · {suffix}"
                if variable_code == "precipitation"
                else f"Mini {mini_id} · {suffix}"
            )
            if variable_code == "precipitation":
                fig.add_bar(
                    x=data["dt"],
                    y=values,
                    name=name,
                    marker={
                        "color": MINI_COLOR,
                        "pattern": {"shape": "/" if flag else ""},
                    },
                    opacity=0.85 if flag == 0 else 0.55,
                    legendgroup=f"mini-{suffix}",
                    row=row,
                    col=1,
                )
            else:
                fig.add_scatter(
                    x=data["dt"],
                    y=values,
                    name=name,
                    mode="lines",
                    line={"color": MINI_COLOR, "dash": dash},
                    legendgroup=f"mini-{suffix}",
                    row=row,
                    col=1,
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
        title=" · ".join(selections) if selections else "Station and Mini Comparison",
    )
    return fig
