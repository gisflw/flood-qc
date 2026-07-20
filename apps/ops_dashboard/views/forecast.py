"""Forecast preview view composition and Panel callback wiring."""
from __future__ import annotations

import sqlite3
from typing import Any

import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.corrections import _correction_table


def _forecast_view(controller: DashboardState) -> pn.viewable.Viewable:
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
        name="Forecast cycle",
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

    table = _correction_table(controller)
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
        title="Forecast Corrections",
        sizing_mode="stretch_width",
    )
    render_maps()
    return pn.Column(controls, maps, corrections, sizing_mode="stretch_width")
