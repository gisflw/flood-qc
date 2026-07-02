from __future__ import annotations

from pathlib import Path

import panel as pn

from apps.ops_dashboard import create_dashboard
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.forecast import _forecast_view
from apps.ops_dashboard.views.monitoring import _monitoring_view


def test_factory_returns_servable_template_with_isolated_state(tmp_path: Path) -> None:
    first = create_dashboard(tmp_path)
    second = create_dashboard(tmp_path)

    assert isinstance(first, pn.template.base.BasicTemplate)
    assert isinstance(first.state, DashboardState)
    assert first.state is not second.state
    assert first.servable() is first


def test_factory_constructs_with_missing_sources_and_reports_warnings(
    tmp_path: Path,
) -> None:
    template = create_dashboard(tmp_path)

    assert template.state.warnings
    assert any("not found" in warning.lower() for warning in template.state.warnings)


def test_monitoring_and_forecast_views_compose_from_controlled_state(
    tmp_path: Path,
) -> None:
    state = DashboardState(tmp_path)

    monitoring = _monitoring_view(state)
    assert isinstance(monitoring, pn.Column)
    assert isinstance(_forecast_view(state), pn.Column)
    widget_names = {
        widget.name for widget in monitoring.select(pn.widgets.Widget)
    }
    assert {
        "Rainfall source",
        "Hours before reference time",
        "Hours after reference time",
        "Apply rainfall period",
        "Raster opacity",
        "Show selected basin",
    }.issubset(widget_names)


def test_map_controls_are_not_in_global_sidebar(tmp_path: Path) -> None:
    template = create_dashboard(tmp_path)

    sidebar_names = set()
    for item in template.sidebar:
        sidebar_names.update(
            widget.name for widget in item.select(pn.widgets.Widget)
        )
    assert sidebar_names == {"Refresh data"}
