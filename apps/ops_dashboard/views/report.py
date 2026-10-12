"""Report basin selection, configuration, preview, and download controls."""
from __future__ import annotations

import html
from pathlib import Path
from typing import Any

import pandas as pd
import panel as pn

from apps.ops_dashboard.i18n import set_widget_text, translate, watch_language
from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.state import DashboardState
from mgb_ops.assets.report import normalize_report_columns


def _report_view(state: DashboardState) -> pn.viewable.Viewable:
    mini = pn.widgets.IntInput.from_param(
        state.param.report_mini_id, name=translate(state.language, "report.mini_id"), start=1,
    )
    buffer = pn.widgets.FloatInput.from_param(
        state.param.report_buffer_km, name=translate(state.language, "report.buffer"), start=0, step=1,
    )
    load_basin = pn.widgets.Button(
        name=translate(state.language, "report.load_basin"), button_type="primary", icon="map-search",
    )
    map_status = pn.pane.Alert("", alert_type="danger", visible=False)
    basin_map = None
    map_container = pn.Column(
        pn.pane.Alert(translate(state.language, "report.load_map_prompt"), alert_type="info"),
        sizing_mode="stretch_width",
    )

    def update_map(artifacts: Any = None) -> None:
        nonlocal basin_map
        artifacts = artifacts or state.report_map_artifacts
        if artifacts is None:
            return
        spec, tooltips = dashboard_map.localize_map_display(artifacts, state.language)
        if basin_map is None:
            basin_map = pn.pane.DeckGL(
                spec, tooltips=tooltips, height=540, sizing_mode="stretch_width",
                name=translate(state.language, "report.map"),
            )
            map_container.objects = [basin_map]
        else:
            basin_map.object, basin_map.tooltips = spec, tooltips

    def load(_: Any = None) -> None:
        map_status.visible = False
        try:
            state.load_report_basin(mini.value, buffer.value)
        except (FileNotFoundError, OSError, RuntimeError, TypeError, ValueError) as exc:
            map_status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, "report.load_basin"), detail=str(exc),
            )
            map_status.visible = True

    load_basin.on_click(load)
    state.param.watch(lambda event: update_map(event.new), "report_map_artifacts")
    state.param.watch(lambda event: update_map(), "language")

    for name in (
        "report_mini_id", "report_buffer_km", "report_map_station_ids",
        "report_chart_station_ids", "report_title", "report_author",
        "report_basin_name", "report_output_name", "report_reference_time",
        "report_window_start", "report_window_end", "report_scenarios",
        "report_table_columns", "execution_active", "source_versions",
    ):
        state.param.watch(state.invalidate_report, name)

    map_choices = pn.widgets.MultiChoice(
        name=translate(state.language, "report.search_stations"), options={},
        value=list(state.report_map_station_ids), solid=False, sizing_mode="stretch_width",
    )
    chart_choices = pn.widgets.MultiChoice(
        name=translate(state.language, "report.search_stations"), options={},
        value=list(state.report_chart_station_ids), solid=False, sizing_mode="stretch_width",
    )
    map_order = pn.widgets.Select(name=translate(state.language, "report.reorder_station"), options={})
    chart_order = pn.widgets.Select(name=translate(state.language, "report.reorder_station"), options={})
    map_remove = pn.widgets.Button(name=translate(state.language, "report.remove_station"))
    chart_remove = pn.widgets.Button(name=translate(state.language, "report.remove_station"))
    map_up = pn.widgets.Button(name=translate(state.language, "report.move_up"))
    map_down = pn.widgets.Button(name=translate(state.language, "report.move_down"))
    chart_up = pn.widgets.Button(name=translate(state.language, "report.move_up"))
    chart_down = pn.widgets.Button(name=translate(state.language, "report.move_down"))
    def remove_station(target: str) -> None:
        selector = map_order if target == "map" else chart_order
        param_name = "report_map_station_ids" if target == "map" else "report_chart_station_ids"
        ids = list(getattr(state, param_name))
        if selector.value in ids:
            ids.remove(selector.value)
            setattr(state, param_name, ids)

    def move_station(target: str, offset: int) -> None:
        selector = map_order if target == "map" else chart_order
        param_name = "report_map_station_ids" if target == "map" else "report_chart_station_ids"
        ids = list(getattr(state, param_name))
        if selector.value not in ids:
            return
        index = ids.index(selector.value)
        destination = index + offset
        if 0 <= index < len(ids) and 0 <= destination < len(ids):
            ids[index], ids[destination] = ids[destination], ids[index]
            setattr(state, param_name, ids)

    map_remove.on_click(lambda _: remove_station("map"))
    chart_remove.on_click(lambda _: remove_station("chart"))
    map_up.on_click(lambda _: move_station("map", -1))
    map_down.on_click(lambda _: move_station("map", 1))
    chart_up.on_click(lambda _: move_station("chart", -1))
    chart_down.on_click(lambda _: move_station("chart", 1))

    def render_station_choices(*_: Any) -> None:
        all_options = {label: station_id for station_id, label in state.report_station_options.items()}
        chart_options = {
            label: station_id for station_id, label in state.report_station_options.items()
            if state.report_station_catalog.get(station_id, {}).get("mini_id") is not None
        }
        for widget, choices, ids in (
            (map_choices, all_options, state.report_map_station_ids),
            (chart_choices, chart_options, state.report_chart_station_ids),
        ):
            widget.options = choices
            widget.value = list(ids)
        for selector, ids in ((map_order, state.report_map_station_ids), (chart_order, state.report_chart_station_ids)):
            selector.options = {state.report_station_options.get(station_id, station_id): station_id for station_id in ids}
            if selector.value not in ids:
                selector.value = ids[0] if ids else None

    state.param.watch(render_station_choices, "report_station_options")
    state.param.watch(render_station_choices, "report_station_catalog")
    state.param.watch(render_station_choices, "report_map_station_ids")
    state.param.watch(render_station_choices, "report_chart_station_ids")
    map_choices.param.watch(lambda event: setattr(state, "report_map_station_ids", list(event.new or [])), "value")
    chart_choices.param.watch(lambda event: setattr(state, "report_chart_station_ids", list(event.new or [])), "value")
    state.param.watch(lambda event: state.rebuild_report_map(), "report_map_station_ids")

    map_selection = pn.Card(
        map_choices, map_order, pn.Row(map_remove, map_up, map_down),
        title=translate(state.language, "report.map_table_stations"), sizing_mode="stretch_width",
    )
    chart_selection = pn.Card(
        chart_choices, chart_order, pn.Row(chart_remove, chart_up, chart_down),
        title=translate(state.language, "report.chart_stations"), sizing_mode="stretch_width",
    )

    if state.report_reference_time is None:
        state.report_reference_time = pd.Timestamp(state.window.cutoff_time).to_pydatetime()
        state.report_window_start = pd.Timestamp(state.window.start_time).to_pydatetime()
        state.report_window_end = pd.Timestamp(state.window.forecast_end_exclusive).to_pydatetime()
    artifact = state._load_required_current_artifact()
    scenario_options = {item.label: item.scenario_id for item in artifact.scenarios}
    default_scenarios = [
        item.scenario_id for item in artifact.scenarios
        if item.kind == "raw" and str(item.provider_code or "").lower() in {"ecmwf", "noaa"}
    ]
    if not state.report_scenarios:
        state.report_scenarios = default_scenarios

    title = pn.widgets.TextInput.from_param(state.param.report_title, name=translate(state.language, "report.title"))
    author = pn.widgets.TextInput.from_param(state.param.report_author, name=translate(state.language, "report.author"))
    basin_name = pn.widgets.TextInput.from_param(state.param.report_basin_name, name=translate(state.language, "report.basin_name"))
    output_name = pn.widgets.TextInput.from_param(state.param.report_output_name, name=translate(state.language, "report.output_name"))
    reference = pn.widgets.DatetimePicker.from_param(state.param.report_reference_time, name=translate(state.language, "report.reference"))
    window_start = pn.widgets.DatetimePicker.from_param(state.param.report_window_start, name=translate(state.language, "report.window_start"))
    window_end = pn.widgets.DatetimePicker.from_param(state.param.report_window_end, name=translate(state.language, "report.window_end"))
    scenarios = pn.widgets.MultiChoice.from_param(
        state.param.report_scenarios, name=translate(state.language, "report.scenarios"),
        options=scenario_options, solid=False, sizing_mode="stretch_width",
    )

    def refresh_scenario_options(*_: Any) -> None:
        current = state._load_required_current_artifact().scenarios
        options = {item.label: item.scenario_id for item in current}
        valid = set(options.values())
        selected = [value for value in state.report_scenarios if value in valid]
        if not selected:
            selected = [
                item.scenario_id for item in current
                if item.kind == "raw" and str(item.provider_code or "").lower() in {"ecmwf", "noaa"}
            ]
        scenarios.options = options
        scenarios.value = selected

    state.param.watch(refresh_scenario_options, "report_current_run_revision")
    markdown = pn.widgets.TextAreaInput(name=translate(state.language, "report.summary"), height=140, sizing_mode="stretch_width")
    markdown.param.watch(lambda event: state.invalidate_report(), "value")
    column_rows = []
    for column in normalize_report_columns(None):
        column_rows.append({
            "label": column["label"], "type": column["type"],
            "variable": column.get("variable", ""), "time_horizon": column.get("time_horizon", 0),
            "operation": column.get("operation", "value"),
        })
    if state.report_table_columns.empty:
        state.report_table_columns = pd.DataFrame(column_rows)
    table_columns = pn.widgets.Tabulator(
        state.report_table_columns.copy(), selectable=1, height=280,
        editors={
            "label": "input",
            "type": {"type": "list", "values": ["obs", "forecast", "threshold"]},
            "variable": {"type": "list", "values": ["rain", "level", "flow", "precipitation", "discharge"]},
            "time_horizon": {"type": "number", "step": 1},
            "operation": {"type": "list", "values": ["value", "accumulation", "change"]},
        }, sizing_mode="stretch_width", name=translate(state.language, "report.columns"),
    )
    add_column = pn.widgets.Button(name=translate(state.language, "report.add_column"))
    add_threshold = pn.widgets.Button(name=translate(state.language, "report.add_threshold"))
    remove_column = pn.widgets.Button(name=translate(state.language, "report.remove_column"))
    move_column_up = pn.widgets.Button(name=translate(state.language, "report.move_up"))
    move_column_down = pn.widgets.Button(name=translate(state.language, "report.move_down"))

    def sync_columns(event: Any = None) -> None:
        frame = table_columns.value.copy()
        if not frame.equals(state.report_table_columns):
            state.report_table_columns = frame

    table_columns.param.watch(sync_columns, "value")
    state.param.watch(lambda event: setattr(table_columns, "value", event.new.copy()), "report_table_columns")

    def append_column(threshold: bool = False) -> None:
        row = {"label": "Flood threshold" if threshold else "New column", "type": "threshold" if threshold else "obs",
               "variable": "" if threshold else "level", "time_horizon": 0, "operation": "value"}
        table_columns.value = pd.concat([table_columns.value, pd.DataFrame([row])], ignore_index=True)

    def edit_column(action: str) -> None:
        frame = table_columns.value.copy()
        if not table_columns.selection:
            return
        index = table_columns.selection[0]
        if action == "remove" and 0 <= index < len(frame):
            frame = frame.drop(index).reset_index(drop=True)
        elif action in {"up", "down"}:
            destination = index + (-1 if action == "up" else 1)
            if 0 <= index < len(frame) and 0 <= destination < len(frame):
                frame.iloc[[index, destination]] = frame.iloc[[destination, index]].to_numpy()
        table_columns.value = frame

    add_column.on_click(lambda _: append_column())
    add_threshold.on_click(lambda _: append_column(True))
    remove_column.on_click(lambda _: edit_column("remove"))
    move_column_up.on_click(lambda _: edit_column("up"))
    move_column_down.on_click(lambda _: edit_column("down"))

    help_text = pn.pane.Markdown(translate(state.language, "report.column_help"))
    build_preview = pn.widgets.Button(name=translate(state.language, "report.build_preview"), button_type="primary", icon="file-text")
    build_pdf = pn.widgets.Button(name=translate(state.language, "report.build_pdf"), button_type="primary", icon="file-type-pdf")
    html_download = pn.widgets.FileDownload(name=translate(state.language, "report.download_html"), file=None, disabled=True)
    pdf_download = pn.widgets.FileDownload(name=translate(state.language, "report.download_pdf"), file=None, disabled=True)
    report_status = pn.pane.Alert("", alert_type="info", visible=False)
    report_warnings = pn.Column(sizing_mode="stretch_width")
    preview = pn.pane.HTML("", height=750, sizing_mode="stretch_width")

    def current_config() -> dict[str, Any]:
        columns = []
        for row in table_columns.value.to_dict("records"):
            if str(row.get("type")) == "threshold":
                columns.append({"label": row.get("label", ""), "type": "threshold"})
            else:
                columns.append({
                    "label": row.get("label", ""), "type": row.get("type", ""),
                    "variable": row.get("variable", ""),
                    "time_horizon": int(row.get("time_horizon")),
                    "operation": row.get("operation", ""),
                })
        return {
            "mini_id": mini.value, "title": title.value, "author": author.value,
            "basin_name": basin_name.value, "output_basin_name": output_name.value,
            "reference_time": reference.value, "window_start": window_start.value,
            "window_end": window_end.value, "scenarios": list(scenarios.value or []) or None,
            "table_columns": columns, "markdown": markdown.value,
        }

    def build(output_format: str) -> None:
        report_status.visible = False
        try:
            state.build_report_async(output_format, current_config())
        except (FileNotFoundError, OSError, RuntimeError, TypeError, ValueError) as exc:
            state.report_error = str(exc)
            report_status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, "report.build_preview" if output_format == "html" else "report.build_pdf"),
                detail=str(exc),
            )
            report_status.alert_type = "danger"
            report_status.visible = True

    build_preview.on_click(lambda _: build("html"))
    build_pdf.on_click(lambda _: build("pdf"))

    def update_report_state(*_: Any) -> None:
        active = state.report_build_active or state.execution_active
        basin_pending = state.report_basin_loaded and (
            state.report_mini_id != state.report_loaded_mini_id
            or float(state.report_buffer_km) != float(state.report_loaded_buffer_km)
        )
        for widget in (build_preview, build_pdf):
            widget.disabled = active
        load_basin.disabled = active
        if state.report_error:
            report_status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, "report.build_preview"), detail=state.report_error,
            )
            report_status.alert_type = "danger"
            report_status.visible = True
        elif state.report_status:
            report_status.object = state.report_status
            report_status.alert_type = "info"
            report_status.visible = True
        elif not state.report_build_active and not basin_pending:
            report_status.visible = False
        report_warnings.objects = [pn.pane.Alert(message, alert_type="warning") for message in state.report_warnings]
        if state.report_preview_document:
            preview.object = (
                '<iframe title="Report preview" sandbox="allow-scripts" '
                'style="width:100%;height:730px;border:0" srcdoc="'
                + html.escape(state.report_preview_document, quote=True) + '"></iframe>'
            )
        else:
            preview.object = ""
        html_download.file = state.report_html_path or None
        html_download.filename = Path(state.report_html_path).name if state.report_html_path else "report.html"
        html_download.disabled = not bool(state.report_html_path) or active
        pdf_download.file = state.report_pdf_path or None
        pdf_download.filename = Path(state.report_pdf_path).name if state.report_pdf_path else "report.pdf"
        pdf_download.disabled = not bool(state.report_pdf_path) or active
        report_controls.visible = state.report_basin_loaded
        build_preview.disabled = active or not state.report_basin_loaded or basin_pending
        build_pdf.disabled = active or not state.report_html_path or basin_pending

    for name in (
        "report_build_active", "execution_active", "report_status", "report_error",
        "report_warnings", "report_preview_document", "report_html_path", "report_pdf_path",
        "report_basin_loaded", "report_station_options", "report_mini_id", "report_buffer_km",
    ):
        state.param.watch(update_report_state, name)

    metadata = pn.Card(
        pn.Row(title, author), pn.Row(basin_name, output_name),
        pn.Row(reference, window_start, window_end), scenarios, markdown,
        title=translate(state.language, "report.metadata"), sizing_mode="stretch_width",
    )
    column_editor = pn.Card(
        help_text, table_columns, pn.Row(add_column, add_threshold, remove_column, move_column_up, move_column_down),
        title=translate(state.language, "report.columns"), sizing_mode="stretch_width",
    )
    station_controls = pn.Column(
        pn.Row(map_selection, chart_selection), sizing_mode="stretch_width",
    )
    report_controls = pn.Column(
        pn.pane.Markdown(translate(state.language, "report.ready")),
        station_controls, metadata, column_editor,
        pn.Row(build_preview, html_download, build_pdf, pdf_download), report_status,
        report_warnings, preview, visible=state.report_basin_loaded, sizing_mode="stretch_width",
    )
    update_report_state()

    def update_language(language: str) -> None:
        for widget, key in (
            (mini, "report.mini_id"), (buffer, "report.buffer"),
            (load_basin, "report.load_basin"), (map_choices, "report.search_stations"),
            (chart_choices, "report.search_stations"), (map_order, "report.reorder_station"),
            (chart_order, "report.reorder_station"),
            (map_remove, "report.remove_station"), (chart_remove, "report.remove_station"),
            (map_up, "report.move_up"), (map_down, "report.move_down"),
            (chart_up, "report.move_up"), (chart_down, "report.move_down"),
            (title, "report.title"), (author, "report.author"), (basin_name, "report.basin_name"),
            (output_name, "report.output_name"), (reference, "report.reference"),
            (window_start, "report.window_start"), (window_end, "report.window_end"),
            (scenarios, "report.scenarios"), (markdown, "report.summary"),
            (add_column, "report.add_column"), (add_threshold, "report.add_threshold"),
            (remove_column, "report.remove_column"), (move_column_up, "report.move_up"),
            (move_column_down, "report.move_down"), (build_preview, "report.build_preview"),
            (build_pdf, "report.build_pdf"), (html_download, "report.download_html"),
            (pdf_download, "report.download_pdf"),
        ):
            set_widget_text(widget, language, key)
        map_selection.title = translate(language, "report.map_table_stations")
        chart_selection.title = translate(language, "report.chart_stations")
        metadata.title = translate(language, "report.metadata")
        column_editor.title = translate(language, "report.columns")
        help_text.object = translate(language, "report.column_help")
        if basin_map is None:
            map_container[0].object = translate(language, "report.load_map_prompt")
        if state.report_error or state.report_status:
            update_report_state()
        update_map()

    watch_language(state, update_language)
    return pn.Column(
        pn.pane.Markdown(translate(state.language, "report.intro")),
        pn.Row(mini, buffer, load_basin), map_status, map_container,
        report_controls, sizing_mode="stretch_width",
    )


__all__ = ["_report_view"]
