"""Responsive dashboard shell and top-level view placement."""
from __future__ import annotations

import panel as pn

from apps.ops_dashboard.i18n import set_widget_text, translate, watch_language
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.forecast import _forecast_view
from apps.ops_dashboard.views.monitoring import _monitoring_view
from apps.ops_dashboard.views.observed_qc import _observed_qc_view
from apps.ops_dashboard.views.summaries import _network_summary


def _build_template(state: DashboardState) -> pn.template.base.BasicTemplate:
    language = pn.widgets.Select.from_param(
        state.param.language,
        name=translate(state.language, "language"),
        options={"English": "en", "Português": "pt-BR"},
        sizing_mode="stretch_width",
    )
    refresh = pn.widgets.Button(
        name=translate(state.language, "dashboard.refresh"), button_type="primary",
        icon="refresh", sizing_mode="stretch_width",
    )
    refreshed = pn.bind(
        lambda lang, value: pn.pane.Markdown(
            translate(lang, "dashboard.refreshed", value=value)
            if value else translate(lang, "dashboard.not_refreshed")
        ),
        state.param.language, state.param.last_refresh_at,
    )
    warnings = pn.bind(
        lambda values, lang: pn.Column(
            *[pn.pane.Alert(state.warning_for_display(value), alert_type="warning") for value in values],
            sizing_mode="stretch_width",
        ),
        state.param.warnings,
        state.param.language,
    )
    execution = pn.bind(
        lambda lang, active, progress, status, error: pn.Column(
            pn.indicators.Progress(
                name=translate(lang, "dashboard.execution"), value=progress, max=100,
                visible=active or status in {"failed", "completed"},
            ),
            pn.pane.Alert(
                translate(lang, "message.operation_failed", operation=translate(lang, "message.operation_refresh"), detail=error)
                if error else translate(lang, f"execution.status.{status}"),
                alert_type="danger" if error else "info",
                visible=bool(error) or active,
            ),
        ),
        state.param.language, state.param.execution_active, state.param.execution_progress,
        state.param.execution_status, state.param.execution_error,
    )
    views = [
        _monitoring_view(state),
        _observed_qc_view(state),
        _forecast_view(state),
    ]
    tabs = pn.Tabs(
        (translate(state.language, "tab.monitoring"), views[0]),
        (translate(state.language, "tab.observed_qc"), views[1]),
        (translate(state.language, "tab.forecast_qc"), views[2]),
        dynamic=True, sizing_mode="stretch_width",
    )
    heading = pn.pane.Markdown(
        f"# {translate(state.language, 'dashboard.heading')}\n"
        f"{translate(state.language, 'dashboard.description')}"
    )
    controls_heading = pn.pane.Markdown(
        f"## {translate(state.language, 'dashboard.controls')}"
    )
    network_summary = pn.bind(
        _network_summary, state.param.stations, state.window.cutoff_time,
        state.param.language,
    )
    template = pn.template.FastListTemplate(
        title=translate(state.language, "dashboard.title"),
        sidebar=[
            heading,
            network_summary,
            pn.layout.Divider(),
            controls_heading,
            language,
            refresh,
            refreshed,
            pn.layout.Divider(),
            warnings,
        ],
        main=[tabs],
        sidebar_width=320, accent_base_color="#1864ab", header_background="#1864ab",
    )
    summary = pn.pane.JSON({}, depth=2, sizing_mode="stretch_width")
    summary_card = pn.Card(
        summary, title=translate(state.language, "dashboard.validated_artifact"),
        collapsed=False, sizing_mode="stretch_width",
    )
    modal_status = pn.pane.Alert("", alert_type="warning", visible=False)
    run = pn.widgets.Button(name=translate(state.language, "common.run"), button_type="primary", icon="player-play")
    cancel = pn.widgets.Button(name=translate(state.language, "common.cancel"), button_type="light")
    confirmation = pn.pane.Markdown(
        f"## {translate(state.language, 'dashboard.confirm_refresh')}\n"
        f"{translate(state.language, 'dashboard.review_artifact')}"
    )
    template.modal.append(pn.Column(
        confirmation, summary_card, execution, modal_status, pn.Row(cancel, run),
        sizing_mode="stretch_width",
    ))
    modal_error: tuple[str, str] | None = None

    def update_language(lang: str) -> None:
        template.title = translate(lang, "dashboard.title")
        heading.object = f"# {translate(lang, 'dashboard.heading')}\n{translate(lang, 'dashboard.description')}"
        controls_heading.object = f"## {translate(lang, 'dashboard.controls')}"
        set_widget_text(language, lang, "language")
        set_widget_text(refresh, lang, "dashboard.refresh")
        set_widget_text(run, lang, "common.run")
        set_widget_text(cancel, lang, "common.cancel")
        summary_card.title = translate(lang, "dashboard.validated_artifact")
        confirmation.object = f"## {translate(lang, 'dashboard.confirm_refresh')}\n{translate(lang, 'dashboard.review_artifact')}"
        active = tabs.active
        tabs[:] = [
            (translate(lang, key), view)
            for key, view in zip(("tab.monitoring", "tab.observed_qc", "tab.forecast_qc"), views)
        ]
        # shortcut: Panel 1.9 has no public in-place tab-title update; replace when it adds one.
        for panels in tabs._panels.values():
            for view, name in zip(views, tabs._names):
                if (tab := panels.get(id(view))) is not None:
                    tab.title = name
        tabs.active = active
        if modal_error is not None:
            operation, detail = modal_error
            modal_status.object = translate(
                lang, "message.operation_failed",
                operation=translate(lang, operation), detail=detail,
            )

    watch_language(state, update_language)

    def open_confirmation(_: object) -> None:
        nonlocal modal_error
        modal_error = None
        try:
            summary.object = state.artifact_summary()
            modal_status.visible = False
            template.open_modal()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            modal_error = ("message.operation_refresh", str(exc))
            modal_status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, modal_error[0]), detail=str(exc),
            )
            modal_status.visible = True
            template.open_modal()

    def start_execution(_: object) -> None:
        nonlocal modal_error
        modal_error = None
        try:
            state.run_current_artifact_async()
            template.close_modal()
        except (FileNotFoundError, RuntimeError, ValueError) as exc:
            modal_error = ("message.operation_refresh", str(exc))
            modal_status.object = translate(
                state.language, "message.operation_failed",
                operation=translate(state.language, modal_error[0]), detail=str(exc),
            )
            modal_status.visible = True

    refresh.on_click(open_confirmation)
    run.on_click(start_execution)
    cancel.on_click(lambda _: template.close_modal())
    state.param.watch(lambda event: setattr(refresh, "disabled", bool(event.new)), "execution_active")
    state.param.watch(lambda event: setattr(run, "disabled", bool(event.new)), "execution_active")
    template.state = state
    template.language_selector = language
    template.refresh_confirmation = {"summary": summary, "run": run, "cancel": cancel, "status": modal_status}
    return template


__all__ = ["_build_template"]
