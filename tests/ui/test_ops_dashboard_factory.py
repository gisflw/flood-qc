from __future__ import annotations

from datetime import datetime
from pathlib import Path
from types import SimpleNamespace

import pandas as pd
import panel as pn

from apps.ops_dashboard import create_dashboard
from mgb_ops.assets.scenario_cache import ScenarioCache
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.i18n import validate_catalogs
from apps.ops_dashboard.services.corrections import build_forecast_edit_row
from apps.ops_dashboard.services.deckgl import RasterLegendSpec
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


def test_factory_displays_actionable_error_without_current_run_artifact(tmp_path: Path) -> None:
    (tmp_path / "data" / "cache" / "current_run.sqlite").unlink()

    template = create_dashboard(tmp_path)

    assert "Build the current run artifact" in str(template.main[0].object)
    assert not hasattr(template, "state")

    template.language_selector.value = "pt-BR"
    assert "Não foi possível iniciar o painel" in template.main[0].object


def test_language_catalogs_have_matching_keys_and_placeholders() -> None:
    validate_catalogs()


def test_language_switch_is_session_local_and_preserves_live_view_state(
    tmp_path: Path, monkeypatch
) -> None:
    template = create_dashboard(tmp_path)
    other_session = create_dashboard(tmp_path)
    tabs = next(item for item in template.main if isinstance(item, pn.Tabs))
    other_tabs = next(item for item in other_session.main if isinstance(item, pn.Tabs))
    views = list(tabs.objects)
    tabs.active = 1
    root = tabs.get_root()

    monitoring = tabs[0]
    qc = tabs[1]
    forecast = tabs[2]
    monitor_widgets = {widget.name: widget for widget in monitoring.select(pn.widgets.Widget)}
    qc_widgets = {widget.name: widget for widget in qc.select(pn.widgets.Widget)}
    basin = monitor_widgets["Basin mini"]
    threshold = qc_widgets["Threshold (mm)"]
    forecast_widgets = {widget.name: widget for widget in forecast.select(pn.widgets.Widget)}
    correction_shift = forecast_widgets["Correction row shift"]
    basin.value = "42"
    threshold.value = 17.5
    correction_shift.value = 5
    template.state.station_id = "ana:1001"
    template.state.mini_id = 7

    table = forecast.select(pn.widgets.Tabulator)[0]
    table.value = pd.DataFrame([build_forecast_edit_row(
        asset_id="asset", t0_step=0, t1_step=3, shift_lat=2, shift_lon=-1,
        rotation_deg=4, multiplication_factor=1.25,
    )])
    table.selection = [0]
    map_pane = monitoring.select(pn.pane.DeckGL)[0]
    map_pane.view_state = {"latitude": -30.0, "longitude": -52.0, "zoom": 7}
    artifact_before = template.state._artifact_path.read_bytes()
    execution_before = (
        template.state.execution_status, template.state.execution_generation,
    )

    def unexpected(*args, **kwargs):
        raise AssertionError("Language changes must not refresh, persist, or execute.")

    monkeypatch.setattr(template.state, "refresh", unexpected)
    monkeypatch.setattr(template.state, "_rebuild_map", unexpected)
    monkeypatch.setattr(template.state, "run_current_artifact_async", unexpected)
    monkeypatch.setattr(template.state, "apply_forecast_draft", unexpected)

    template.language_selector.value = "pt-BR"

    assert template.state.language == "pt-BR"
    assert other_session.state.language == "en"
    assert template.sidebar[3].object == "## Controles"
    assert tabs._names == ["Monitoramento", "CQ de observações", "CQ de previsão"]
    assert [panel.title for panel in root.tabs] == tabs._names
    assert other_tabs._names == ["Monitoring", "Observed QC", "Forecast QC"]
    assert tabs.active == 1
    assert all(current is original for current, original in zip(tabs.objects, views))
    assert basin.name == "Mini da bacia" and basin.value == "42"
    assert threshold.name == "Limite (mm)" and threshold.value == 17.5
    assert correction_shift.name == "Deslocamento da linha da correção"
    assert correction_shift.value == 5
    assert table.selection == [0]
    assert table.titles["shift_lat"] == "Deslocamento de linha"
    assert template.state.forecast_draft.iloc[0].shift_lat == 2
    assert template.state.station_id == "ana:1001" and template.state.mini_id == 7
    assert map_pane.view_state == {"latitude": -30.0, "longitude": -52.0, "zoom": 7}
    assert template.state._artifact_path.read_bytes() == artifact_before
    assert (
        template.state.execution_status, template.state.execution_generation,
    ) == execution_before


def test_language_switch_preserves_displayed_forecast_preview(tmp_path: Path) -> None:
    state = DashboardState(tmp_path)
    state.applied_preview_request = SimpleNamespace(t0_step=0, t1_step=3)
    state.forecast_map_artifacts = SimpleNamespace(
        original=SimpleNamespace(
            spec={"layers": []},
            legend=RasterLegendSpec("raw caption", 1.0, 2.0),
        ),
        corrected=None,
    )

    view = _forecast_view(state)
    card = view[1][0]
    map_pane = view.select(pn.pane.DeckGL)[0]
    map_pane.view_state = {"latitude": -30.0, "longitude": -52.0, "zoom": 8}
    original_spec = map_pane.object

    state.language = "pt-BR"

    assert view[1][0] is card
    assert card[0] is map_pane
    assert map_pane.object is original_spec
    assert map_pane.view_state == {"latitude": -30.0, "longitude": -52.0, "zoom": 8}
    assert "Acúmulo da previsão t=0h–t=3h" in card[1].object


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


def test_zero_scenario_is_hidden_from_map_selector_but_available_for_charts(
    tmp_path: Path,
) -> None:
    state = DashboardState(tmp_path)
    state.scenario_caches = [
        ScenarioCache(
            "zero", "Zero", "zero", tmp_path / "zero.nc", None, None, None,
            reference_time=datetime(2026, 3, 12),
        ),
        ScenarioCache(
            "raw", "Raw", "raw", tmp_path / "raw.nc", "ecmwf", "asset", None,
            reference_time=datetime(2026, 3, 12),
        ),
    ]
    state.scenario_id = "raw"
    state.comparison_scenario_ids = ["zero", "raw"]

    widgets = {
        widget.name: widget
        for widget in _monitoring_view(state).select(pn.widgets.Widget)
    }

    assert "zero" not in widgets["Forecast to display"].options.values()
    assert "zero" in widgets["Compare scenarios"].options.values()


def test_map_controls_are_not_in_global_sidebar(tmp_path: Path) -> None:
    template = create_dashboard(tmp_path)

    sidebar_names = set()
    for item in template.sidebar:
        sidebar_names.update(
            widget.name for widget in item.select(pn.widgets.Widget)
        )
    assert sidebar_names == {"Language", "Refresh data"}


def test_tab_labels_and_order_match_quality_control_workflow(tmp_path: Path) -> None:
    template = create_dashboard(tmp_path)

    tabs = next(item for item in template.main if isinstance(item, pn.Tabs))

    assert list(tabs._names) == ["Monitoring", "Observed QC", "Forecast QC"]


def test_chart_start_time_is_scoped_to_comparison_card(tmp_path: Path) -> None:
    state = DashboardState(tmp_path)
    monitoring = _monitoring_view(state)

    widgets = {widget.name for widget in monitoring.select(pn.widgets.Widget)}

    assert "Chart start time" in widgets


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
        "Responsible person",
        "Run reason",
        "Add correction",
    }.issubset(widgets)

    widgets["Correction start step"].value = 0
    widgets["Correction end step"].value = 3
    widgets["Correction row shift"].value = 2
    widgets["Correction column shift"].value = -1
    widgets["Correction rotation (°)"].value = 4
    widgets["Correction multiplication factor"].value = 1.25
    widgets["Add correction"].clicks += 1

    assert len(state.forecast_draft) == 1
    row = state.forecast_draft.iloc[0]
    assert row.t0_step == 0
    assert row.t1_step == 3
    assert row.shift_lat == 2
    assert row.shift_lon == -1
    assert row.rotation_deg == 4
    assert row.multiplication_factor == 1.25


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
