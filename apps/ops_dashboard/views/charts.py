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


def _bar_width(
    data: pd.DataFrame, time_column: str, *, maximum: pd.Timedelta | None = None
) -> float:
    timestamps = pd.to_datetime(data[time_column], errors="coerce").dropna().drop_duplicates().sort_values()
    intervals = timestamps.diff().dropna()
    interval = intervals.min() if not intervals.empty else pd.Timedelta(hours=1)
    if maximum is not None:
        interval = min(interval, maximum)
    return float(interval.total_seconds() * 800)


def _comparison_chart(
    observed: pd.DataFrame,
    model_series: Mapping[str, pd.DataFrame],
    station_id: str | None,
    mini_id: int | None,
    reference_levels: pd.DataFrame | None = None,
    reference_time: pd.Timestamp | None = None,
) -> go.Figure:
    """Build one ordered station/mini comparison figure from prepared series."""
    fig = make_subplots(rows=3, cols=1, shared_xaxes=True, vertical_spacing=0.08)
    station_codes = {"precipitation": "rain", "level": "level", "flow": "flow"}
    station_units = {"precipitation": "mm", "level": "cm", "flow": "m³/s"}
    has_station_precipitation, has_station_series = False, False
    for variable_code, observed_code in station_codes.items():
        if observed.empty:
            break
        data = observed[observed["variable_code"] == observed_code].dropna(subset=["value"]).copy()
        if data.empty:
            continue
        data["value"] = truncate_series_one_decimal(data["value"])
        row = VARIABLE_ROWS[variable_code]
        if variable_code == "precipitation":
            fig.add_bar(x=data["datetime"], y=data["value"], name="observed at station", marker_color=STATION_COLOR,
                        width=_bar_width(data, "datetime", maximum=pd.Timedelta(hours=1)),
                        legendgroup="precipitation:observed", showlegend=False, row=row, col=1)
            has_station_precipitation = True
        else:
            fig.add_scatter(x=data["datetime"], y=data["value"], name="Observed", mode="lines",
                            line={"color": STATION_COLOR}, legendgroup="series:up-to-present", showlegend=False, row=row, col=1)
            has_station_series = True
        has_station_data = True

    nested = bool(model_series) and all(isinstance(value, Mapping) for value in model_series.values())
    scenario_groups = list(model_series.items()) if nested else [(None, model_series)]
    colors = (MINI_COLOR, "#2f9e44", "#7048e8", "#d9480f", "#0b7285", "#c2255c")
    scenario_legends, basin_observed_added, simulated_variables = [], False, set()
    for index, (label, variables) in enumerate(scenario_groups):
        color = colors[index % len(colors)]
        group = f"scenario:{label}" if label else "basin-forecast"
        has_precipitation_forecast = False
        has_series_forecast = False
        for variable_code, row in VARIABLE_ROWS.items():
            frame = variables.get(variable_code, pd.DataFrame())
            if frame.empty:
                continue
            for flag, suffix in ((0, "observed"), (1, "forecast")):
                data = frame[frame["prev_flag"] == flag].dropna(subset=["value"]).copy()
                if data.empty:
                    continue
                if variable_code == "precipitation" and flag == 0:
                    if basin_observed_added:
                        continue
                    basin_observed_added = True
                    data["value"] = truncate_series_one_decimal(data["value"])
                    fig.add_bar(x=data["dt"], y=data["value"], name="aggregated at basin", marker_color=MINI_COLOR,
                                width=_bar_width(data, "dt"), opacity=0.85, legendgroup="precipitation:observed", showlegend=False, row=row, col=1)
                    continue
                if variable_code == "precipitation" and flag == 1 and data["value"].eq(0).all():
                    continue
                data["value"] = truncate_series_one_decimal(data["value"])
                if variable_code == "precipitation":
                    has_precipitation_forecast = True
                    name = label or "Basin forecast"
                    fig.add_bar(x=data["dt"], y=data["value"], name=name, marker={"color": color, "pattern": {"shape": "/"}},
                                width=_bar_width(data, "dt"), opacity=0.55, legendgroup="precipitation:forecast", showlegend=False, row=row, col=1)
                else:
                    if flag == 0:
                        if variable_code in simulated_variables:
                            continue
                        simulated_variables.add(variable_code)
                        name, line, series_group = "Simulated", {"color": MINI_COLOR, "dash": "solid"}, "series:up-to-present"
                    else:
                        has_series_forecast = True
                        name = label or "Basin forecast"
                        line, series_group = {"color": color, "dash": "dash"}, "series:future-scenarios"
                    fig.add_scatter(x=data["dt"], y=data["value"], name=name, mode="lines",
                                    line=line, legendgroup=series_group, showlegend=False, row=row, col=1)
        scenario_legends.append((label or "Basin forecast", color, group, has_precipitation_forecast, has_series_forecast))

    reference_styles = {
        "attention": ("Attention", "#facc15", "solid"),
        "alert": ("Alert", "#d4a017", "solid"),
        "flood": ("Flood", "#dc2626", "solid"),
        "severe": ("Severe", "#3b0764", "solid"),
        "historical_flood": ("Historical flood", "#2563eb", "dash"),
    }
    reference_legends: list[tuple[str, str, str]] = []
    if reference_levels is not None and not reference_levels.empty:
        for _, reference in (
            reference_levels.assign(_level_sort=pd.to_numeric(reference_levels["level_cm"], errors="coerce"))
            .sort_values("_level_sort", ascending=False, na_position="last")
            .iterrows()
        ):
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
                type="line", x0=0, x1=1, xref="x3 domain", y0=float(level), y1=float(level), yref="y3",
                line={"color": color, "dash": dash, "width": 2},
            )
            reference_legends.append((f"{label} ({float(level):g} cm)", color, dash))

    def add_bar_legend(name: str, color: str, group: str, *, opacity: float = 1.0, pattern: str = "", group_title: str | None = None) -> None:
        fig.add_bar(x=[None], y=[None], name=name, marker={"color": color, "pattern": {"shape": pattern}},
                    opacity=opacity, legendgroup=group, legendgrouptitle={"text": group_title} if group_title else None,
                    legend="legend", showlegend=True, hoverinfo="skip")

    def add_line_legend(name: str, color: str, group: str, *, dash: str, group_title: str | None = None) -> None:
        fig.add_scatter(x=[None], y=[None], name=name, mode="lines", line={"color": color, "dash": dash},
                        legendgroup=group, legendgrouptitle={"text": group_title} if group_title else None,
                        legend="legend2", showlegend=True, hoverinfo="skip")
    if has_station_precipitation:
        add_bar_legend("observed at station", STATION_COLOR, "precipitation:observed", group_title="Observed")
    if has_station_series:
        add_line_legend("Observed", STATION_COLOR, "series:up-to-present", dash="solid", group_title="Up to present")
    if basin_observed_added:
        add_bar_legend("aggregated at basin", MINI_COLOR, "precipitation:observed", opacity=0.85, group_title="Observed")
    if simulated_variables:
        add_line_legend("Simulated", MINI_COLOR, "series:up-to-present", dash="solid", group_title="Up to present")
    for label, color, group, has_precipitation, has_series in scenario_legends:
        if has_precipitation:
            add_bar_legend(label, color, "precipitation:forecast", opacity=0.55, pattern="/", group_title="Forecast")
        if has_series:
            add_line_legend(label, color, "series:future-scenarios", dash="dash", group_title="Future scenarios")
    for index, (name, color, dash) in enumerate(reference_legends):
        add_line_legend(name, color, "series:reference-levels", dash=dash, group_title="Reference levels")
    if reference_time is not None:
        marker_time = pd.Timestamp(reference_time)
        marker_label = marker_time.strftime("%d/%m/%Y %H:%M")
        marker_style = {"color": "rgba(128, 128, 128, 0.2)", "dash": "dash", "width": 1}
        for row, xref, yref in ((1, "x", "y domain"), (2, "x2", "y2 domain"), (3, "x3", "y3 domain")):
            fig.add_vline(x=marker_time, row=row, col=1, line=marker_style)
            fig.add_annotation(
                x=marker_time, xref=xref, y=1, yref=yref, text=f"Reference time<br>{marker_label}",
                showarrow=False, yanchor="bottom", font={"color": "rgba(80, 80, 80, 0.8)"},
            )
    if not fig.data:
        text = "Click a station and/or a mini on the map." if station_id is None and mini_id is None else "No series data are available for the current selection."
        fig.add_annotation(text=text, x=0.5, y=0.5, xref="paper", yref="paper", showarrow=False)
    selections = []
    if station_id is not None:
        selections.append(f"Station {station_id}")
    if mini_id is not None:
        selections.append(f"Mini {mini_id}")
    fig.update_yaxes(title_text="Precipitation<br>mm", row=1, col=1)
    fig.update_yaxes(title_text="Discharge<br>m³/s", row=2, col=1)
    fig.update_yaxes(title_text="Water Level<br>cm", row=3, col=1)
    fig.update_xaxes(showticklabels=True)
    fig.update_layout(
        template="plotly_white", height=780, margin={"t": 55, "r": 160, "b": 45, "l": 55},
        hovermode="x unified", barmode="group",
        legend={"groupclick": "toggleitem"},
        legend2={"groupclick": "toggleitem", "title": {"text": "Discharge and water level"}, "x": 1.02, "y": 0.5},
        title=" · ".join(selections) if selections else "Observed and Modeled Comparison",
    )
    return fig
