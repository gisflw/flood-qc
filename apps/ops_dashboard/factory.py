"""Dashboard factory: extensions, session state, views, and template."""
from __future__ import annotations

from pathlib import Path

import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.shell import _build_template


def create_dashboard(
    workspace: str | Path | None = None,
) -> pn.template.base.BasicTemplate:
    """Create one servable dashboard with isolated session state."""
    pn.extension("deckgl", "plotly", "tabulator", notifications=True)
    state = DashboardState(workspace)
    return _build_template(state)


__all__ = ["create_dashboard"]
