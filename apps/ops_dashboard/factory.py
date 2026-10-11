"""Dashboard factory: extensions, session state, views, and template."""

from __future__ import annotations

from pathlib import Path

from mgb_ops.config.runtime import RuntimeContext

import panel as pn
import param

from apps.ops_dashboard.state import CurrentRunArtifactError, DashboardState
from apps.ops_dashboard.i18n import translate


class _StartupLanguage(param.Parameterized):
    language = param.Selector(
        default="en", objects={"English": "en", "Português": "pt-BR"}
    )
from apps.ops_dashboard.views.shell import _build_template


def create_dashboard(
    workspace: str | Path | None = None,
    *,
    context: RuntimeContext | None = None,
) -> pn.template.base.BasicTemplate:
    """Create one servable dashboard with isolated session state."""
    pn.extension("deckgl", "plotly", "tabulator", notifications=True)
    try:
        state = DashboardState(workspace, context=context)
    except CurrentRunArtifactError as exc:
        startup = _StartupLanguage()
        error_detail = str(exc)
        language = pn.widgets.Select.from_param(
            startup.param.language, name=translate(startup.language, "language"),
            options={"English": "en", "Português": "pt-BR"},
        )
        alert = pn.pane.Alert(
            translate(startup.language, "dashboard.startup_error", detail=error_detail),
            alert_type="danger", sizing_mode="stretch_width",
        )
        template = pn.template.FastListTemplate(
            title=translate(startup.language, "dashboard.title"),
            sidebar=[language],
            main=[alert],
        )
        def update_startup_language(event: param.parameterized.Event) -> None:
            template.title = translate(event.new, "dashboard.title")
            alert.object = translate(
                event.new, "dashboard.startup_error", detail=error_detail
            )
            language.name = translate(event.new, "language")

        startup.param.watch(update_startup_language, "language")
        template.language_selector = language
        return template
    return _build_template(state)


__all__ = ["create_dashboard"]
