"""Native Panel layout for the operational dashboard."""
from __future__ import annotations

from pathlib import Path
import sqlite3
from typing import Any

import numpy as np
import pandas as pd
import panel as pn
import plotly.graph_objects as go
from plotly.subplots import make_subplots

from apps.ops_dashboard.controller import DashboardController
from apps.ops_dashboard.support import data as dashboard_data


pn.extension("deckgl", "plotly", "tabulator", notifications=True)


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
    return pn.FlexBox(
        *[
            pn.Card(
                pn.pane.Markdown(f"## {value}\n{label}"),
                hide_header=True,
                min_width=150,
                sizing_mode="stretch_width",
            )
            for label, value in cards
        ],
        sizing_mode="stretch_width",
    )


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


def _station_summary(controller: DashboardController) -> pn.viewable.Viewable:
    station_id = controller.station_id
    if station_id is None:
        return pn.pane.Alert("Click a station on the map.", alert_type="info")
    selected = controller.stations[
        controller.stations["station_id"].astype(str) == station_id
    ]
    if selected.empty:
        return pn.pane.Alert("The selected station is unavailable.", alert_type="warning")
    row = selected.iloc[0]
    observed = controller.observed_series()
    if observed.empty:
        detail = "No observed data in the dashboard window."
    else:
        metrics = dashboard_data.compute_observed_metrics(
            observed, cutoff_time=controller.window.cutoff_time
        )
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


def _mini_summary(controller: DashboardController) -> pn.viewable.Viewable:
    if controller.mini_id is None:
        return pn.pane.Alert("Click a mini catchment or river.", alert_type="info")
    try:
        levels = controller.mgb_series("y")
    except (FileNotFoundError, OSError, ValueError) as exc:
        return pn.pane.Alert(str(exc), alert_type="warning")
    if levels.empty:
        return pn.pane.Alert("No level series for this mini.", alert_type="info")
    summary = dashboard_data.summarize_mini_peaks(
        levels,
        cutoff_time=controller.window.cutoff_time,
        forecast_end_exclusive=controller.window.forecast_end_exclusive,
    )
    return pn.pane.Markdown(
        f"### Mini {controller.mini_id}\n"
        f"Current level: {_format_number(summary['current_value'], 'm', 2)}  \n"
        f"Observed-window peak: {_format_number(summary['current_peak'], 'm', 2)} · "
        f"Forecast-window peak: {_format_number(summary['forecast_peak'], 'm', 2)}"
    )


def _monitoring_view(controller: DashboardController) -> pn.viewable.Viewable:
    artifacts = controller.map_artifacts
    map_pane = pn.pane.DeckGL(
        artifacts.spec,
        height=590,
        sizing_mode="stretch_width",
        name="Operational map",
    )

    def update_map(event: Any) -> None:
        map_pane.object = event.new.spec

    controller.param.watch(update_map, "map_artifacts")
    map_pane.param.watch(
        lambda event: controller.handle_map_click(event.new), "click_state"
    )

    inspection = pn.pane.Markdown("")

    def update_inspection(event: Any = None) -> None:
        value = controller.raster_inspection
        inspection.object = (
            ""
            if value is None
            else (
                f"**{value.layer_name}:** "
                f"{_format_number(value.value, 'mm')} at "
                f"{value.latitude:.4f}, {value.longitude:.4f}"
            )
        )

    controller.param.watch(update_inspection, "raster_inspection")
    station_summary = pn.bind(
        lambda *_: _station_summary(controller),
        controller.param.station_id,
        controller.param.source_versions,
    )
    mini_summary = pn.bind(
        lambda *_: _mini_summary(controller),
        controller.param.mini_id,
        controller.param.source_versions,
    )
    station_plot = pn.bind(
        lambda *_: pn.pane.Plotly(
            _station_chart(controller.observed_series(), controller.station_id),
            config={"responsive": True},
            sizing_mode="stretch_width",
        ),
        controller.param.station_id,
        controller.param.source_versions,
    )

    def model_plots(*_: Any) -> pn.viewable.Viewable:
        try:
            levels = controller.mgb_series("y")
            flows = controller.mgb_series("q")
        except (FileNotFoundError, OSError, ValueError):
            levels = flows = pd.DataFrame()
        return pn.Column(
            pn.pane.Plotly(
                _mgb_chart(levels, controller.mini_id, "y"),
                config={"responsive": True},
                sizing_mode="stretch_width",
            ),
            pn.pane.Plotly(
                _mgb_chart(flows, controller.mini_id, "q"),
                config={"responsive": True},
                sizing_mode="stretch_width",
            ),
            sizing_mode="stretch_width",
        )

    mini_plots = pn.bind(
        model_plots,
        controller.param.mini_id,
        controller.param.source_versions,
    )
    return pn.Column(
        pn.Card(
            map_pane,
            inspection,
            title="Operational Map",
            sizing_mode="stretch_width",
        ),
        pn.Row(
            pn.Card(station_summary, title="Station Summary", sizing_mode="stretch_width"),
            pn.Card(mini_summary, title="Mini Summary", sizing_mode="stretch_width"),
            sizing_mode="stretch_width",
        ),
        pn.Row(
            pn.Card(station_plot, title="Station Chart", sizing_mode="stretch_width"),
            pn.Card(mini_plots, title="Mini Charts", sizing_mode="stretch_width"),
            sizing_mode="stretch_width",
        ),
        sizing_mode="stretch_width",
    )


def _forecast_view(controller: DashboardController) -> pn.viewable.Viewable:
    assets = controller.forecast_assets
    asset_options = (
        {
            str(row.display_label): str(row.asset_id)
            for row in assets.itertuples()
        }
        if not assets.empty
        else {}
    )
    asset = pn.widgets.Select(
        name="ECMWF cycle",
        options=asset_options,
        value=controller.forecast_asset_id,
    )
    t0 = pn.widgets.Select(name="Start step", options=[])
    t1 = pn.widgets.Select(name="End step", options=[])

    def update_steps() -> None:
        if controller.forecast_steps.empty:
            t0.options = t1.options = []
            return
        labels = {
            str(row.label): int(row.step_hours)
            for row in controller.forecast_steps.itertuples()
        }
        t0.options = labels
        t1.options = labels
        t0.value = controller.forecast_t0_step
        t1.value = controller.forecast_t1_step

    update_steps()

    def select_asset(event: Any) -> None:
        if event.new:
            controller.select_forecast_asset(event.new)
            update_steps()
            table.value = controller.forecast_draft.copy()

    asset.param.watch(select_asset, "value")
    t0.param.watch(
        lambda event: setattr(controller, "forecast_t0_step", int(event.new)),
        "value",
    )
    t1.param.watch(
        lambda event: setattr(controller, "forecast_t1_step", int(event.new)),
        "value",
    )
    shift_lat = pn.widgets.FloatInput(name="Row shift", value=0, step=1)
    shift_lon = pn.widgets.FloatInput(name="Column shift", value=0, step=1)
    rotation = pn.widgets.FloatInput(name="Rotation (°)", value=0, step=1)
    factor = pn.widgets.FloatInput(
        name="Multiplication factor", value=1, step=0.05, start=0.01
    )
    opacity = pn.widgets.FloatSlider(
        name="Raster opacity", value=0.7, start=0, end=1, step=0.05
    )
    apply_button = pn.widgets.Button(
        name="Apply preview", button_type="primary", icon="map"
    )
    maps = pn.Column(sizing_mode="stretch_width")
    status = pn.pane.Alert("", alert_type="info", visible=False)

    def render_maps() -> None:
        artifacts = controller.forecast_map_artifacts
        maps.clear()
        if artifacts is None:
            maps.append(
                pn.pane.Alert(
                    "Choose a cycle and apply preview parameters.",
                    alert_type="info",
                )
            )
            return
        original = pn.pane.DeckGL(
            artifacts.original.spec, height=500, sizing_mode="stretch_width"
        )
        if artifacts.corrected is None:
            maps.append(
                pn.Card(
                    original,
                    pn.pane.Markdown(artifacts.original.legend_html),
                    title="Original",
                    sizing_mode="stretch_width",
                )
            )
            return
        corrected = pn.pane.DeckGL(
            artifacts.corrected.spec, height=500, sizing_mode="stretch_width"
        )
        syncing = {"active": False}

        def synchronize(event: Any, target: pn.pane.DeckGL) -> None:
            if syncing["active"] or not event.new:
                return
            syncing["active"] = True
            try:
                shared = dict(event.new)
                target.view_state = shared
                controller.update_forecast_view(shared)
            finally:
                syncing["active"] = False

        original.param.watch(lambda event: synchronize(event, corrected), "view_state")
        corrected.param.watch(lambda event: synchronize(event, original), "view_state")
        maps.append(
            pn.Row(
                pn.Card(
                    original,
                    pn.pane.Markdown(artifacts.original.legend_html),
                    title="Original",
                    sizing_mode="stretch_width",
                ),
                pn.Card(
                    corrected,
                    pn.pane.Markdown(artifacts.corrected.legend_html),
                    title="Corrected",
                    sizing_mode="stretch_width",
                ),
                sizing_mode="stretch_width",
            )
        )

    def apply_preview(_: Any) -> None:
        controller.forecast_shift_lat = shift_lat.value
        controller.forecast_shift_lon = shift_lon.value
        controller.forecast_rotation_deg = rotation.value
        controller.forecast_multiplication_factor = factor.value
        controller.forecast_opacity = opacity.value
        try:
            controller.apply_preview()
            status.visible = False
            render_maps()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            status.object = str(exc)
            status.alert_type = "warning"
            status.visible = True

    apply_button.on_click(apply_preview)

    table = pn.widgets.Tabulator(
        controller.forecast_draft.copy(),
        show_index=False,
        sizing_mode="stretch_width",
        height=260,
        hidden_columns=["asset_id", "metadata_json"],
        editors={
            "manual_edit_id": None,
            "created_at": None,
            "t0_step": {"type": "number", "min": 0, "step": 1},
            "t1_step": {"type": "number", "min": 0, "step": 1},
            "shift_lat": {"type": "number", "step": 1},
            "shift_lon": {"type": "number", "step": 1},
            "rotation_deg": {"type": "number", "step": 1},
            "multiplication_factor": {"type": "number", "min": 0.01, "step": 0.05},
            "remove": {"type": "tickCross"},
        },
    )
    table.param.watch(
        lambda event: controller.update_forecast_draft(event.new), "value"
    )
    editor = pn.widgets.TextInput(name="Editor")
    reason = pn.widgets.TextInput(name="Correction reason")
    add_button = pn.widgets.Button(name="Add correction", button_type="light")
    save_button = pn.widgets.Button(
        name="Save changes", button_type="primary", icon="device-floppy"
    )

    def show_controller_message() -> None:
        status.object = controller.message
        status.alert_type = controller.message_kind
        status.visible = bool(controller.message)

    def add_row(_: Any) -> None:
        try:
            controller.add_forecast_correction(
                t0_step=t0.value,
                t1_step=t1.value,
                shift_lat=shift_lat.value,
                shift_lon=shift_lon.value,
                rotation_deg=rotation.value,
                multiplication_factor=factor.value,
                editor=editor.value,
                reason=reason.value,
            )
            table.value = controller.forecast_draft.copy()
            show_controller_message()
        except ValueError as exc:
            status.object = str(exc)
            status.alert_type = "warning"
            status.visible = True

    def save_rows(_: Any) -> None:
        controller.update_forecast_draft(table.value)
        try:
            controller.save_forecast_corrections()
            table.value = controller.forecast_draft.copy()
        except (ValueError, sqlite3.IntegrityError) as exc:
            # The controller records the more useful validation/database message.
            if not controller.message:
                controller.set_message(str(exc), "danger")
        show_controller_message()

    add_button.on_click(add_row)
    save_button.on_click(save_rows)

    def refresh_asset_widgets(_: Any) -> None:
        frame = controller.forecast_assets
        asset.options = (
            {
                str(row.display_label): str(row.asset_id)
                for row in frame.itertuples()
            }
            if not frame.empty
            else {}
        )
        asset.value = controller.forecast_asset_id
        update_steps()
        table.value = controller.forecast_draft.copy()

    controller.param.watch(refresh_asset_widgets, "forecast_assets")
    controls = pn.Card(
        pn.Row(asset, t0, t1, sizing_mode="stretch_width"),
        pn.Row(
            shift_lat,
            shift_lon,
            rotation,
            factor,
            opacity,
            sizing_mode="stretch_width",
        ),
        apply_button,
        status,
        title="Forecast Preview",
        sizing_mode="stretch_width",
    )
    corrections = pn.Card(
        pn.pane.Markdown(
            "Edit typed cells, mark rows for removal, then save the replacement set transactionally."
        ),
        table,
        pn.Row(editor, reason, add_button, save_button, sizing_mode="stretch_width"),
        title="ECMWF Corrections",
        sizing_mode="stretch_width",
    )
    render_maps()
    return pn.Column(controls, maps, corrections, sizing_mode="stretch_width")


def create_dashboard(
    workspace: str | Path | None = None,
) -> pn.template.base.BasicTemplate:
    """Create one servable dashboard with isolated session state."""
    controller = DashboardController(workspace)
    refresh = pn.widgets.Button(
        name="Refresh data", button_type="primary", icon="refresh", sizing_mode="stretch_width"
    )
    refresh.on_click(lambda _: controller.refresh())
    raster = pn.widgets.Select.from_param(
        controller.param.selected_raster,
        name="Accumulated rainfall",
        sizing_mode="stretch_width",
    )
    opacity = pn.widgets.FloatSlider.from_param(
        controller.param.raster_opacity,
        name="Raster opacity",
        sizing_mode="stretch_width",
    )
    refreshed = pn.bind(
        lambda value: pn.pane.Markdown(
            f"Last session refresh:  \n{value}" if value else "Not refreshed yet."
        ),
        controller.param.last_refresh_at,
    )
    warnings = pn.bind(
        lambda values: pn.Column(
            *[
                pn.pane.Alert(value, alert_type="warning")
                for value in values
            ],
            sizing_mode="stretch_width",
        ),
        controller.param.warnings,
    )
    summary = pn.bind(_network_summary, controller.param.stations)
    monitoring = _monitoring_view(controller)
    forecast = _forecast_view(controller)
    tabs = pn.Tabs(
        ("Monitoring", monitoring),
        ("Forecast", forecast),
        dynamic=True,
        sizing_mode="stretch_width",
    )
    template = pn.template.FastListTemplate(
        title="Operational Hydrology",
        sidebar=[
            pn.pane.Markdown("## Controls"),
            refresh,
            refreshed,
            raster,
            opacity,
            pn.layout.Divider(),
            warnings,
        ],
        main=[
            pn.pane.Markdown(
                "# RS Flood Alert System\n"
                "Observed stations, MGB output, rainfall rasters, and ECMWF correction workflows."
            ),
            summary,
            tabs,
        ],
        sidebar_width=320,
        accent_base_color="#1864ab",
        header_background="#1864ab",
    )
    # Kept reachable for unit tests and advanced embedding without global state.
    template.controller = controller
    return template


__all__ = ["create_dashboard"]
