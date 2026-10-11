"""Observed Precipitation QC tab composed around session-local DashboardState."""
from __future__ import annotations

from typing import Any

import pandas as pd
import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.i18n import set_widget_text, table_titles, translate


def _observed_qc_view(state: DashboardState) -> pn.viewable.Viewable:
    review_start = pn.widgets.DatetimeInput(name="Review window start", value=state.review_window_start)
    review_end = pn.widgets.DatetimeInput(name="Review window end", value=state.review_window_end)
    threshold = pn.widgets.FloatInput(name="Threshold (mm)", value=state.qc_threshold_mm, start=0)
    sequence_length = pn.widgets.IntInput(name="Minimum sequence length", value=state.qc_sequence_min_length, start=1)
    sequence_lower = pn.widgets.FloatInput(name="Sequence lower bound (mm)", value=state.qc_sequence_lower_mm, start=0)
    sequence_upper = pn.widgets.FloatInput(name="Sequence upper bound (mm)", value=state.qc_sequence_upper_mm, start=0)
    scan = pn.widgets.Button(name="Scan observations", button_type="primary", icon="search")
    status = pn.pane.Alert("", alert_type="info", visible=False)
    table = pn.widgets.Tabulator(
        state.detected_flags.copy(), name="Detected precipitation flags",
        pagination="local", page_size=20, selectable=1, header_filters=True,
        sizing_mode="stretch_width", height=420,
        titles=table_titles(state.language, (
            "station_id", "observed_at", "flag_type", "match_type", "sequence_id",
            "raw_value", "effective_value", "replacement_state", "excluded",
        )),
    )
    status_error: str | None = None

    def show_message(kind: str = "warning", text: str | None = None) -> None:
        nonlocal status_error
        if text is not None:
            status_error = text
            status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, "message.operation_qc"), detail=text,
            )
        else:
            status_error = None
            status.object = state.message_for_display()
        status.alert_type = kind if text is not None else state.message_kind
        status.visible = bool(status.object)

    def update_language(language: str) -> None:
        for widget, key in (
            (review_start, "qc.review_start"), (review_end, "qc.review_end"),
            (threshold, "qc.threshold"), (sequence_length, "qc.sequence_length"),
            (sequence_lower, "qc.sequence_lower"), (sequence_upper, "qc.sequence_upper"),
            (scan, "qc.scan"), (custom_value, "qc.custom_value"),
            (null_button, "qc.apply_null"), (value_button, "qc.apply_custom"),
            (sequence_zero, "qc.sequence_zero"), (sequence_custom, "qc.sequence_custom"),
            (remove_button, "qc.remove_instruction"), (exclude_button, "qc.exclude_station"),
            (include_button, "qc.include_station"), (loo_station, "qc.leave_one_out_station"),
            (run_loo, "qc.run_leave_one_out"), (responsible, "forecast.responsible"),
            (reason, "forecast.run_reason"), (run_id, "qc.saved_run_id"),
            (description, "qc.saved_description"), (update, "qc.update_current"),
            (save, "qc.save_run"),
        ):
            set_widget_text(widget, language, key)
        table.label = translate(language, "qc.flags")
        table.titles = table_titles(language, (
            "station_id", "observed_at", "flag_type", "match_type", "sequence_id",
            "raw_value", "effective_value", "replacement_state", "excluded",
        ))
        policy.title = translate(language, "qc.policy_card")
        edits.title = translate(language, "qc.draft_card")
        analysis.title = translate(language, "qc.analysis_card")
        qc_map_card.title = translate(language, "qc.map_card")
        persistence.title = translate(language, "qc.persistence_card")
        if status_error is not None:
            show_message(status.alert_type, status_error)
        else:
            show_message()

    def scan_flags(_: Any) -> None:
        state.review_window_start, state.review_window_end = review_start.value, review_end.value
        state.qc_threshold_mm = threshold.value
        state.qc_sequence_min_length = sequence_length.value
        state.qc_sequence_lower_mm = sequence_lower.value
        state.qc_sequence_upper_mm = sequence_upper.value
        try:
            table.value = state.scan_observed_precipitation().copy()
            show_message()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            show_message("warning", str(exc))

    scan.on_click(scan_flags)

    def selected_row() -> pd.Series:
        if not table.selection:
            raise ValueError("Select a flagged observation first.")
        return table.value.iloc[int(table.selection[0])]

    def select_flag(event: Any) -> None:
        if event.new:
            row = table.value.iloc[int(event.new[0])]
            state.qc_selected_station = str(row.station_id)
            state.qc_selected_sequence = None if pd.isna(row.sequence_id) else str(row.sequence_id)

    table.param.watch(select_flag, "selection")
    custom_value = pn.widgets.FloatInput(name="Custom replacement (mm)", value=0.0, start=0)
    null_button = pn.widgets.Button(name="Apply NULL", button_type="warning")
    value_button = pn.widgets.Button(name="Apply custom value", button_type="light")
    sequence_zero = pn.widgets.Button(name="Replace sequence with 0.0", button_type="light")
    sequence_custom = pn.widgets.Button(name="Replace sequence with custom value", button_type="light")
    remove_button = pn.widgets.Button(name="Remove instruction", button_type="light")
    exclude_button = pn.widgets.Button(name="Exclude station for review window", button_type="warning")
    include_button = pn.widgets.Button(name="Include station", button_type="light")

    def edit(action) -> None:
        try:
            row = selected_row()
            action(row)
            table.value = state.scan_observed_precipitation().copy()
            show_message()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            show_message("warning", str(exc))

    null_button.on_click(lambda _: edit(lambda row: state.set_observed_replacement(row.station_id, row.observed_at, None)))
    value_button.on_click(lambda _: edit(lambda row: state.set_observed_replacement(row.station_id, row.observed_at, custom_value.value)))
    sequence_zero.on_click(lambda _: edit(lambda row: state.replace_selected_sequence(None if pd.isna(row.sequence_id) else str(row.sequence_id), 0.0)))
    sequence_custom.on_click(lambda _: edit(lambda row: state.replace_selected_sequence(None if pd.isna(row.sequence_id) else str(row.sequence_id), custom_value.value)))
    remove_button.on_click(lambda _: edit(lambda row: state.remove_observed_replacement(row.station_id, row.observed_at)))
    exclude_button.on_click(lambda _: edit(lambda row: state.exclude_station(str(row.station_id))))
    include_button.on_click(lambda _: edit(lambda row: state.include_station(str(row.station_id))))

    qc_map = pn.Column(sizing_mode="stretch_width")
    qc_map_pane: pn.pane.DeckGL | None = None

    def update_qc_map(artifacts=None) -> None:
        nonlocal qc_map_pane
        if artifacts is None:
            qc_map.objects = [pn.pane.Alert(translate(state.language, "qc.scan_map"), alert_type="info")]
            qc_map_pane = None
            return
        spec, tooltips = dashboard_map.localize_map_display(artifacts, state.language)
        if qc_map_pane is None:
            qc_map_pane = pn.pane.DeckGL(spec, tooltips=tooltips, height=480, sizing_mode="stretch_width")
            qc_map.objects = [qc_map_pane]
        else:
            view_state = qc_map_pane.view_state
            qc_map_pane.object = spec
            qc_map_pane.tooltips = tooltips
            if view_state:
                qc_map_pane.view_state = view_state

    state.param.watch(lambda event: update_qc_map(event.new), "qc_map_artifacts")
    state.param.watch(lambda event: update_qc_map(state.qc_map_artifacts), "language")
    update_qc_map(state.qc_map_artifacts)
    loo_station = pn.widgets.Select(name="Leave-one-out station", options={})
    run_loo = pn.widgets.Button(name="Run leave-one-out", button_type="primary")
    diagnostic_map_pane: pn.pane.DeckGL | None = None

    def station_options(_: Any = None) -> None:
        if state.stations.empty:
            loo_station.options = {}
            return
        loo_station.options = {f"{getattr(row, 'station_name', row.station_id)} ({row.station_id})": str(row.station_id) for row in state.stations.itertuples()}

    station_options()

    def diagnostic(result, language):
        nonlocal diagnostic_map_pane
        if result is None:
            diagnostic_map_pane = None
            return pn.pane.Alert(translate(language, "qc.choose_station"), alert_type="info")
        if not result.sufficient_source:
            return pn.pane.Alert(translate(language, "qc.analysis_unavailable", detail=result.message), alert_type="warning")
        previous_view_state = diagnostic_map_pane.view_state if diagnostic_map_pane is not None else None
        residual = translate(language, "qc.suppressed") if result.residual is None else f"{result.residual:.2f} mm"
        error = translate(language, "qc.suppressed") if result.absolute_error is None else f"{result.absolute_error:.2f} mm"
        layer, _, legend = dashboard_map.build_raster_layer(
            result.grid, layer_id="qc-leave-one-out-raster", layer_name=translate(language, "qc.residual_layer"), opacity=0.75,
        )
        layers = [layer] if layer is not None else []
        selected = state.stations[state.stations["station_id"].astype(str) == str(result.station_id)] if not state.stations.empty else pd.DataFrame()
        if not selected.empty:
            row = selected.iloc[0]
            layers.append({
                "@@type": "GeoJsonLayer", "id": "qc-omitted-station", "pickable": True,
                "data": {"type": "FeatureCollection", "features": [{
                    "type": "Feature", "geometry": {"type": "Point", "coordinates": [float(row.lon), float(row.lat)]},
                    "properties": {"station_id": result.station_id, "label": translate(language, "qc.omitted_station")},
                }]},
                "pointType": "circle", "filled": True, "getFillColor": [220, 38, 38, 255],
                "getLineColor": [255, 255, 255, 255], "getLineWidth": 3,
                "getPointRadius": 9500,
            })
        raster = pn.pane.DeckGL({
            "initialViewState": dashboard_map.default_view_state(bounds=result.grid.bounds),
            "controller": True, "layers": layers,
        }, height=480, sizing_mode="stretch_width")
        diagnostic_map_pane = raster
        if previous_view_state:
            raster.view_state = previous_view_state
        metrics = pn.pane.Markdown(
            f"{translate(language, 'qc.observed_total')}: **{result.observed_total if result.observed_total is not None else translate(language, 'qc.missing')} mm**  \n"
            f"{translate(language, 'qc.predicted_total')}: **{result.predicted_total:.2f} mm**  \n"
            f"{translate(language, 'qc.residual')}: **{residual}**  \n"
            f"{translate(language, 'qc.absolute_error')}: **{error}**  \n"
            f"{translate(language, 'qc.coverage')}: **{result.expected_timesteps} / {result.target_timesteps} / {result.predicted_timesteps}**"
        )
        legend_pane = pn.pane.Markdown(dashboard_map.build_raster_legend_html(legend, caption=translate(language, "qc.residual_layer"))) if legend is not None else pn.Spacer()
        return pn.Column(metrics, raster, legend_pane)

    diagnostic_view = pn.bind(diagnostic, state.param.qc_diagnostic_result, state.param.language)

    def run_diagnostic(_: Any) -> None:
        try:
            state.qc_selected_station = loo_station.value
            state.run_leave_one_out()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            show_message("warning", str(exc))

    run_loo.on_click(run_diagnostic)
    responsible = pn.widgets.TextInput(name="Responsible person", value=state.responsible_person)
    reason = pn.widgets.TextInput(name="Run reason", value=state.update_reason)
    run_id = pn.widgets.TextInput(name="Saved run ID", value=state.save_run_id)
    description = pn.widgets.TextAreaInput(name="Saved run description", value=state.save_run_description)
    update = pn.widgets.Button(name="Update Current Run", button_type="primary", icon="device-floppy")
    save = pn.widgets.Button(name="Save Run", button_type="success", icon="archive")
    conflicting = [scan, null_button, value_button, sequence_zero, sequence_custom, remove_button, exclude_button, include_button, run_loo, update, save]
    def disable_conflicting(event: Any) -> None:
        for widget in conflicting:
            widget.disabled = bool(event.new)
    state.param.watch(disable_conflicting, "execution_active")
    for widget in conflicting:
        widget.disabled = bool(state.execution_active)

    def update_current(_: Any) -> None:
        state.responsible_person, state.update_reason = responsible.value, reason.value
        try:
            state.update_current_run()
            show_message()
        except (RuntimeError, ValueError) as exc:
            show_message("warning", str(exc))

    def save_copy(_: Any) -> None:
        state.save_run_id, state.save_run_description = run_id.value, description.value
        try:
            state.save_run()
            show_message()
        except (FileExistsError, FileNotFoundError, RuntimeError, ValueError) as exc:
            show_message("warning", str(exc))

    update.on_click(update_current)
    save.on_click(save_copy)
    policy = pn.Card(
        pn.Row(review_start, review_end), pn.Row(threshold, sequence_length),
        pn.Row(sequence_lower, sequence_upper), scan, title=translate(state.language, "qc.policy_card"),
    )
    edits = pn.Card(
        custom_value, pn.Row(null_button, value_button), pn.Row(sequence_zero, sequence_custom),
        pn.Row(remove_button, exclude_button, include_button), title=translate(state.language, "qc.draft_card"),
    )
    analysis = pn.Card(pn.Row(loo_station, run_loo), diagnostic_view, title=translate(state.language, "qc.analysis_card"))
    qc_map_card = pn.Card(qc_map, title=translate(state.language, "qc.map_card"))
    persistence = pn.Card(pn.Row(responsible, reason), pn.Row(run_id, description), pn.Row(update, save), title=translate(state.language, "qc.persistence_card"))
    state.param.watch(lambda event: update_language(event.new), "language")
    state.param.watch(lambda event: show_message(), "message")
    state.param.watch(lambda event: show_message(), "message_kind")
    update_language(state.language)
    return pn.Column(policy, status, table, edits, qc_map_card, analysis, persistence, sizing_mode="stretch_width")


__all__ = ["_observed_qc_view"]
