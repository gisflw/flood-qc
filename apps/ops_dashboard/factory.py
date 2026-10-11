"""Dashboard factory: extensions, session state, views, and template."""

from __future__ import annotations

from pathlib import Path

from mgb_ops.config.runtime import RuntimeContext

import panel as pn

from apps.ops_dashboard.state import CurrentRunArtifactError, DashboardState
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
        return pn.template.FastListTemplate(
            title="Operational Hydrology",
            main=[pn.pane.Alert(str(exc), alert_type="danger", sizing_mode="stretch_width")],
        )
    return _build_template(state)


__all__ = ["create_dashboard"]
