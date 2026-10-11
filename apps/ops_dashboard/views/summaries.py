"""Explicit-input summary panes for the dashboard."""
from __future__ import annotations

from typing import Any

import numpy as np
import pandas as pd
import panel as pn

from apps.ops_dashboard.services.formatting import truncate_one_decimal
from apps.ops_dashboard.i18n import translate
from apps.ops_dashboard.state import DashboardState


def _format_number(value: Any, unit: str, precision: int = 1, *, language: str = "en") -> str:
    if value is None or pd.isna(value):
        return translate(language, "summary.unavailable")
    return f"{truncate_one_decimal(value):.{precision}f} {unit}".strip()


def _network_summary(
    stations: pd.DataFrame,
    reference_time: pd.Timestamp,
    language: str = "en",
) -> pn.viewable.Viewable:
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
        (translate(language, "summary.reference_time"), pd.Timestamp(reference_time).strftime("%d/%m/%Y %H:%M")),
        (translate(language, "summary.total_stations"), str(values[0])),
        (translate(language, "summary.with_data"), str(values[1])),
        (translate(language, "summary.no_data"), str(values[2])),
        (translate(language, "summary.data_issue"), str(values[3])),
        (translate(language, "summary.mean_rainfall"), _format_number(values[4], "mm", language=language)),
        (translate(language, "summary.p90_rainfall"), _format_number(values[5], "mm", language=language)),
    ]
    return pn.Column(
        *[
            pn.Card(
                pn.pane.HTML(
                    f"<div style='font-size:0.78rem;line-height:1.15'>"
                    f"<strong style='font-size:1.15rem'>{value}</strong><br>{label}</div>"
                ),
                hide_header=True,
                sizing_mode="stretch_width",
                margin=3,
            )
            for label, value in cards
        ],
        sizing_mode="stretch_width",
    )


def _rainfall_total(
    values: pd.DataFrame,
    *,
    time_column: str,
    start_exclusive: pd.Timestamp,
    end_inclusive: pd.Timestamp,
    segment: int | None = None,
) -> float:
    """Sum valid rainfall values in (start, end], optionally within one segment."""
    if values.empty or time_column not in values or "value" not in values:
        return np.nan
    frame = values.copy()
    frame[time_column] = pd.to_datetime(frame[time_column], errors="coerce")
    frame["value"] = pd.to_numeric(frame["value"], errors="coerce")
    if segment is not None:
        if "prev_flag" not in frame:
            return np.nan
        frame = frame[frame["prev_flag"] == segment]
    selected = frame[
        (frame[time_column] > start_exclusive)
        & (frame[time_column] <= end_inclusive)
    ]["value"].dropna()
    return float(selected.sum()) if not selected.empty else np.nan


def _selected_area_summary(
    state: DashboardState,
    period_controls: pn.viewable.Viewable | None = None,
    language: str | None = None,
) -> pn.viewable.Viewable:
    language = language or getattr(state, "language", "en")
    cutoff = pd.Timestamp(state.window.cutoff_time)
    previous_hours = int(state.summary_previous_hours)
    forecast_hours = int(state.summary_forecast_hours)
    previous_duration = pd.Timedelta(hours=previous_hours)
    forecast_duration = pd.Timedelta(hours=forecast_hours)

    station_label = translate(language, "summary.no_station")
    station_rain = np.nan
    if state.station_id is not None:
        selected = (
            state.stations[
                state.stations["station_id"].astype(str) == state.station_id
            ]
            if "station_id" in state.stations
            else pd.DataFrame()
        )
        if selected.empty:
            station_label = translate(language, "summary.station_unavailable", station_id=state.station_id)
        else:
            row = selected.iloc[0]
            station_label = (
                f"{row['station_name']} "
                f"({str(row['provider_code']).upper()}:{row['station_code']})"
            )
            observed = state.observed_series()
            rain = (
                observed[observed["variable_code"] == "rain"]
                if not observed.empty and "variable_code" in observed
                else pd.DataFrame()
            )
            station_rain = _rainfall_total(
                rain,
                time_column="datetime",
                start_exclusive=cutoff - previous_duration,
                end_inclusive=cutoff,
            )

    basin_label = (
        translate(language, "summary.modeled_basin", mini_id=state.mini_id)
        if state.mini_id is not None
        else translate(language, "summary.no_basin")
    )
    basin_past = basin_forecast = np.nan
    if state.mini_id is not None:
        try:
            precipitation = state.basin_precipitation()
        except (FileNotFoundError, OSError, TypeError, ValueError) as exc:
            state.add_warning("", translation_key="warning.basin_precipitation", translation_args={"detail": str(exc)})
            precipitation = pd.DataFrame()
        basin_past = _rainfall_total(
            precipitation,
            time_column="dt",
            start_exclusive=cutoff - previous_duration,
            end_inclusive=cutoff,
            segment=0,
        )
        basin_forecast = _rainfall_total(
            precipitation,
            time_column="dt",
            start_exclusive=cutoff,
            end_inclusive=cutoff + forecast_duration,
            segment=1,
        )

    cards = [
        (
            translate(language, "summary.station_rain_past", hours=previous_hours),
            station_rain,
        ),
        (
            translate(language, "summary.basin_rain_past", hours=previous_hours),
            basin_past,
        ),
        (
            translate(language, "summary.basin_rain_future", hours=forecast_hours),
            basin_forecast,
        ),
    ]
    metrics = pn.Row(
        *[
            pn.Card(
                pn.pane.HTML(
                    f"<div style='font-size:0.82rem;line-height:1.25'>"
                    f"<strong style='font-size:1.2rem'>{_format_number(value, 'mm', language=language)}</strong>"
                    f"<br>{label}</div>"
                ),
                hide_header=True,
                sizing_mode="stretch_width",
                margin=3,
            )
            for label, value in cards
        ],
        sizing_mode="stretch_width",
    )
    identity = pn.pane.Markdown(
            f"**{translate(language, 'summary.station')}:** {station_label}  \n"
            f"**{translate(language, 'summary.basin')}:** {basin_label}"
    )
    contents = [identity]
    if period_controls is not None:
        contents.append(period_controls)
    contents.append(metrics)
    return pn.Column(*contents, sizing_mode="stretch_width")
