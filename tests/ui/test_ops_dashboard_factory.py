from __future__ import annotations

from pathlib import Path

import pandas as pd
import panel as pn

from apps.ops_dashboard import create_dashboard
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.forecast import _forecast_view
from apps.ops_dashboard.services import forecast as dashboard_forecast
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
        "Forecast to display",
        "Compare scenarios",
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


def test_forecast_correction_form_adds_a_draft_row(tmp_path: Path) -> None:
    state = DashboardState(tmp_path)
    state.forecast_asset_id = "asset"
    state.forecast_steps = pd.DataFrame(
        [
            {"step_hours": 0, "label": "t=0h"},
            {"step_hours": 3, "label": "t=3h"},
        ]
    )
    state.forecast_t0_step = 0
    state.forecast_t1_step = 3

    view = _forecast_view(state)
    widgets = {widget.name: widget for widget in view.select(pn.widgets.Widget)}
    assert {
        "Correction start step",
        "Correction end step",
        "Correction row shift",
        "Correction column shift",
        "Correction rotation (°)",
        "Correction multiplication factor",
        "Correction editor",
        "Correction reason",
        "Add correction",
    }.issubset(widgets)

    widgets["Correction start step"].value = 0
    widgets["Correction end step"].value = 3
    widgets["Correction row shift"].value = 2
    widgets["Correction column shift"].value = -1
    widgets["Correction rotation (°)"].value = 4
    widgets["Correction multiplication factor"].value = 1.25
    widgets["Correction editor"].value = "operator"
    widgets["Correction reason"].value = "radar alignment"
    widgets["Add correction"].clicks += 1

    assert len(state.forecast_draft) == 1
    row = state.forecast_draft.iloc[0]
    assert row.t0_step == 0
    assert row.t1_step == 3
    assert row.shift_lat == 2
    assert row.shift_lon == -1
    assert row.rotation_deg == 4
    assert row.multiplication_factor == 1.25
    assert row.editor == "operator"
    assert row.reason == "radar alignment"


def test_apply_preview_seeds_correction_form_from_displayed_map(
    tmp_path: Path, monkeypatch
) -> None:
    state = DashboardState(tmp_path)
    state.forecast_asset_id = "asset"
    state.forecast_steps = pd.DataFrame(
        [
            {"step_hours": 0, "label": "t=0h"},
            {"step_hours": 3, "label": "t=3h"},
        ]
    )
    state.forecast_t0_step = 0
    state.forecast_t1_step = 3
    monkeypatch.setattr(
        state,
        "apply_preview",
        lambda: dashboard_forecast.ForecastPreviewRequest(
            asset_id="asset",
            t0_step=0,
            t1_step=3,
            shift_lat=2,
            shift_lon=-1,
            rotation_deg=4,
            multiplication_factor=1.25,
        ),
    )

    view = _forecast_view(state)
    widgets = {widget.name: widget for widget in view.select(pn.widgets.Widget)}
    widgets["Row shift"].value = 2
    widgets["Column shift"].value = -1
    widgets["Rotation (°)"].value = 4
    widgets["Multiplication factor"].value = 1.25
    widgets["Apply preview"].clicks += 1

    assert widgets["Correction start step"].value == 0
    assert widgets["Correction end step"].value == 3
    assert widgets["Correction row shift"].value == 2
    assert widgets["Correction column shift"].value == -1
    assert widgets["Correction rotation (°)"].value == 4
    assert widgets["Correction multiplication factor"].value == 1.25
