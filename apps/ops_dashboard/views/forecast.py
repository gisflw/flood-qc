"""Forecast preview view composition and Panel callback wiring."""
from __future__ import annotations

import sqlite3
from typing import Any

import panel as pn

from apps.ops_dashboard.i18n import set_widget_text, translate
from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.services.deckgl_sync import link_deckgl_panes
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
    correction_t0 = pn.widgets.Select(name="Correction start step", options=[])
    correction_t1 = pn.widgets.Select(name="Correction end step", options=[])

    def update_steps() -> None:
        if controller.forecast_steps.empty:
            t0.options = t1.options = []
            correction_t0.options = correction_t1.options = []
            return
        labels = {
            str(row.label): int(row.step_hours)
            for row in controller.forecast_steps.itertuples()
        }
        t0.options = labels
        t1.options = labels
        correction_t0.options = labels
        correction_t1.options = labels
        t0.value = controller.forecast_t0_step
        t1.value = controller.forecast_t1_step
        correction_t0.value = controller.forecast_t0_step
        correction_t1.value = controller.forecast_t1_step

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
    status_error: tuple[str, str] | None = None
    map_cards: list[tuple[pn.Card, str]] = []
    map_legends: list[tuple[pn.pane.Markdown, bool]] = []

    def preview_caption(language: str, corrected: bool = False) -> str:
        request = controller.applied_preview_request
        title = translate(
            language, "forecast.accumulation",
            start=request.t0_step if request is not None else controller.forecast_t0_step,
            end=request.t1_step if request is not None else controller.forecast_t1_step,
        )
        return translate(language, "forecast.corrected_suffix", title=title) if corrected else title

    def localized_legend(legend, language: str, corrected: bool) -> str:
        if legend is None:
            return ""
        return dashboard_map.build_raster_legend_html(
            legend, caption=preview_caption(language, corrected)
        )

    def update_language(language: str) -> None:
        for widget, key in (
            (asset, "forecast.cycle"), (t0, "forecast.start_step"),
            (t1, "forecast.end_step"), (correction_t0, "forecast.correction_start_step"),
            (correction_t1, "forecast.correction_end_step"), (shift_lat, "forecast.row_shift"),
            (shift_lon, "forecast.column_shift"), (rotation, "forecast.rotation"),
            (factor, "forecast.factor"), (opacity, "forecast.opacity"),
            (apply_button, "forecast.apply_preview"),
            (correction_shift_lat, "forecast.correction_row_shift"),
            (correction_shift_lon, "forecast.correction_column_shift"),
            (correction_rotation, "forecast.correction_rotation"),
            (correction_factor, "forecast.correction_factor"),
            (responsible_person, "forecast.responsible"), (run_reason, "forecast.run_reason"),
            (add_button, "forecast.add_correction"), (save_button, "forecast.apply_draft"),
        ):
            set_widget_text(widget, language, key)
        for card, key in map_cards:
            card.title = translate(language, key)
        controls.title = translate(language, "forecast.preview")
        corrections.title = translate(language, "forecast.corrections")
        correction_help.object = translate(language, "forecast.correction_help")
        artifacts = controller.forecast_map_artifacts
        if artifacts is not None:
            for pane, corrected in map_legends:
                panel = artifacts.corrected if corrected else artifacts.original
                pane.object = localized_legend(panel.legend, language, corrected)
        if status_error is not None:
            operation, detail = status_error
            status.object = translate(
                language, "message.operation_failed",
                operation=translate(language, operation), detail=detail,
            )
        elif controller.message:
            show_controller_message()
        elif maps.objects and isinstance(maps[0], pn.pane.Alert):
            maps[0].object = translate(language, "forecast.choose_cycle")

    def render_maps() -> None:
        artifacts = controller.forecast_map_artifacts
        maps.clear()
        map_cards.clear()
        map_legends.clear()
        if artifacts is None:
            maps.append(
                pn.pane.Alert(
                    translate(controller.language, "forecast.choose_cycle"),
                    alert_type="info",
                )
            )
            return
        original = pn.pane.DeckGL(
            artifacts.original.spec, height=500, sizing_mode="stretch_width"
        )
        if artifacts.corrected is None:
            legend = pn.pane.Markdown(localized_legend(artifacts.original.legend, controller.language, False))
            card = pn.Card(original, legend, title=translate(controller.language, "forecast.original"), sizing_mode="stretch_width")
            map_cards.append((card, "forecast.original"))
            map_legends.append((legend, False))
            maps.append(
                card
            )
            return
        corrected = pn.pane.DeckGL(
            artifacts.corrected.spec, height=500, sizing_mode="stretch_width"
        )
        original.param.watch(
            lambda event: controller.update_forecast_view(event.new), "view_state"
        )
        corrected.param.watch(
            lambda event: controller.update_forecast_view(event.new), "view_state"
        )
        original_legend = pn.pane.Markdown(localized_legend(artifacts.original.legend, controller.language, False))
        corrected_legend = pn.pane.Markdown(localized_legend(artifacts.corrected.legend, controller.language, True))
        original_card = pn.Card(original, original_legend, title=translate(controller.language, "forecast.original"), sizing_mode="stretch_width")
        corrected_card = pn.Card(corrected, corrected_legend, title=translate(controller.language, "forecast.corrected"), sizing_mode="stretch_width")
        map_cards.extend(((original_card, "forecast.original"), (corrected_card, "forecast.corrected")))
        map_legends.extend(((original_legend, False), (corrected_legend, True)))
        comparison = pn.Row(original_card, corrected_card, sizing_mode="stretch_width")
        maps.append(comparison)
        link_deckgl_panes(original, corrected)

    def apply_preview(_: Any) -> None:
        nonlocal status_error
        controller.forecast_shift_lat = shift_lat.value
        controller.forecast_shift_lon = shift_lon.value
        controller.forecast_rotation_deg = rotation.value
        controller.forecast_multiplication_factor = factor.value
        controller.forecast_opacity = opacity.value
        try:
            controller.apply_preview()
            # Seed the correction form with the exact interval and transforms
            # used for the map currently being displayed.
            correction_t0.value = t0.value
            correction_t1.value = t1.value
            correction_shift_lat.value = shift_lat.value
            correction_shift_lon.value = shift_lon.value
            correction_rotation.value = rotation.value
            correction_factor.value = factor.value
            status.visible = False
            status_error = None
            render_maps()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            status_error = ("forecast.apply_preview_operation", str(exc))
            status.object = translate(
                controller.language, "message.operation_failed",
                operation=translate(controller.language, "forecast.apply_preview_operation"),
                detail=status_error[1],
            )
            status.alert_type = "warning"
            status.visible = True

    apply_button.on_click(apply_preview)

    table = _correction_table(controller)
    correction_shift_lat = pn.widgets.FloatInput(
        name="Correction row shift", value=0, step=1
    )
    correction_shift_lon = pn.widgets.FloatInput(
        name="Correction column shift", value=0, step=1
    )
    correction_rotation = pn.widgets.FloatInput(
        name="Correction rotation (°)", value=0, step=1
    )
    correction_factor = pn.widgets.FloatInput(
        name="Correction multiplication factor", value=1, step=0.05, start=0.01
    )
    responsible_person = pn.widgets.TextInput(name="Responsible person")
    run_reason = pn.widgets.TextInput(name="Run reason")
    add_button = pn.widgets.Button(name="Add correction", button_type="light")
    save_button = pn.widgets.Button(
        name="Apply to Current Run Draft", button_type="primary", icon="device-floppy"
    )

    def show_controller_message() -> None:
        nonlocal status_error
        status_error = None
        status.object = controller.message_for_display()
        status.alert_type = controller.message_kind
        status.visible = bool(controller.message)

    def add_row(_: Any) -> None:
        nonlocal status_error
        try:
            controller.add_forecast_correction(
                t0_step=correction_t0.value,
                t1_step=correction_t1.value,
                shift_lat=correction_shift_lat.value,
                shift_lon=correction_shift_lon.value,
                rotation_deg=correction_rotation.value,
                multiplication_factor=correction_factor.value,


            )
            table.value = controller.forecast_draft.copy()
            show_controller_message()
        except ValueError as exc:
            status_error = ("message.operation_add_forecast", str(exc))
            status.object = translate(controller.language, "message.operation_failed", operation=translate(controller.language, status_error[0]), detail=status_error[1])
            status.alert_type = "warning"
            status.visible = True

    def save_rows(_: Any) -> None:
        nonlocal status_error
        controller.update_forecast_draft(table.value)
        try:
            controller.apply_forecast_draft(responsible_person=responsible_person.value, reason=run_reason.value)
            table.value = controller.forecast_draft.copy()
        except (ValueError, sqlite3.IntegrityError) as exc:
            # The controller records the more useful validation/database message.
            if not controller.message:
                controller.set_message(str(exc), "danger")
            status_error = ("message.operation_apply_forecast", str(exc))
            status.object = translate(controller.language, "message.operation_failed", operation=translate(controller.language, status_error[0]), detail=status_error[1])
            status.alert_type = "warning"
            status.visible = True
            return
        show_controller_message()

    add_button.on_click(add_row)
    save_button.on_click(save_rows)
    conflicting = [asset, apply_button, add_button, save_button]
    def disable_conflicting(event: Any) -> None:
        for widget in conflicting:
            widget.disabled = bool(event.new)
    controller.param.watch(disable_conflicting, "execution_active")
    for widget in conflicting:
        widget.disabled = bool(controller.execution_active)

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
        title=translate(controller.language, "forecast.preview"),
        sizing_mode="stretch_width",
    )
    correction_help = pn.pane.Markdown(translate(controller.language, "forecast.correction_help"))
    corrections = pn.Card(
        correction_help,
        pn.Row(correction_t0, correction_t1, sizing_mode="stretch_width"),
        pn.Row(
            correction_shift_lat,
            correction_shift_lon,
            correction_rotation,
            correction_factor,
            sizing_mode="stretch_width",
        ),
        pn.Row(add_button, sizing_mode="stretch_width"),
        pn.Row(responsible_person, run_reason, sizing_mode="stretch_width"),
        table,
        pn.Row(save_button, sizing_mode="stretch_width"),
        title=translate(controller.language, "forecast.corrections"),
        sizing_mode="stretch_width",
    )
    controller.param.watch(lambda event: update_language(event.new), "language")
    controller.param.watch(lambda event: show_controller_message(), "message")
    controller.param.watch(lambda event: show_controller_message(), "message_kind")
    update_language(controller.language)
    render_maps()
    return pn.Column(controls, maps, corrections, sizing_mode="stretch_width")
