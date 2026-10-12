"""Monitoring view composition and Panel callback wiring."""
from __future__ import annotations

from typing import Any

import panel as pn

from apps.ops_dashboard.services import deckgl as dashboard_map
from apps.ops_dashboard.i18n import set_widget_text, translate
from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.charts import _comparison_chart
from apps.ops_dashboard.views.summaries import (
    _format_number,
    _selected_area_summary,
)


def _monitoring_view(
    controller: DashboardState,
) -> pn.viewable.Viewable:
    scenario_options = {
        controller.scenario_label(cache.scenario_id): cache.scenario_id
        for cache in controller.scenario_caches
        if cache.kind != "zero"
    }
    comparison_options = {
        controller.scenario_label(cache.scenario_id): cache.scenario_id
        for cache in controller.scenario_caches
    }
    scenario = pn.widgets.Select(
        name="Forecast to display",
        options=scenario_options,
        value=controller.scenario_id if controller.scenario_id in scenario_options.values() else None,
        sizing_mode="stretch_width",
    )
    comparisons = pn.widgets.MultiChoice(
        name="Compare scenarios",
        options=comparison_options,
        value=list(controller.comparison_scenario_ids),
        sizing_mode="stretch_width",
    )

    def select_scenario(event: Any) -> None:
        if event.new:
            controller.select_scenario(str(event.new))

    scenario.param.watch(select_scenario, "value")
    comparisons.param.watch(
        lambda event: setattr(
            controller, "comparison_scenario_ids", list(event.new or [])
        ),
        "value",
    )

    def refresh_scenario_widgets(_: Any = None) -> None:
        map_options = {
            controller.scenario_label(cache.scenario_id): cache.scenario_id
            for cache in controller.scenario_caches
            if cache.kind != "zero"
        }
        chart_options = {
            controller.scenario_label(cache.scenario_id): cache.scenario_id
            for cache in controller.scenario_caches
        }
        scenario.options = map_options
        comparisons.options = chart_options
        scenario.value = (
            controller.scenario_id
            if controller.scenario_id in map_options.values()
            else None
        )
        comparisons.value = list(controller.comparison_scenario_ids)

    controller.param.watch(refresh_scenario_widgets, "scenario_caches")
    controller.param.watch(refresh_scenario_widgets, "language")
    artifacts = controller.map_artifacts
    map_pane = pn.pane.DeckGL(
        artifacts.spec,
        tooltips=artifacts.tooltips,
        height=590,
        sizing_mode="stretch_width",
        name="Operational map",
    )
    map_pane.jscallback(
        args={"deck": map_pane},
        view_state=f"""
        const view = Bokeh.index.find_one_by_id(deck.id)
        if (view == null || view.deckGL == null) return
        const visible = Number(cb_obj.viewState.zoom || 0) >=
          {dashboard_map.STATION_RAINFALL_LABEL_MIN_ZOOM}
        const layers = view.deckGL.props.layers.map((layer) =>
          layer.id === "station-rainfall-labels" ? layer.clone({{visible}}) : layer
        )
        view.deckGL.setProps({{layers}})
        """,
    )

    def update_map(artifacts: Any) -> None:
        if artifacts is None:
            return
        view_state = getattr(map_pane, "view_state", None)
        spec, tooltips = dashboard_map.localize_map_display(artifacts, controller.language)
        zoom = (view_state or {}).get(
            "zoom", spec["initialViewState"].get("zoom", 0)
        )
        for layer in spec["layers"]:
            if layer.get("id") == "station-rainfall-labels":
                layer["visible"] = zoom >= dashboard_map.STATION_RAINFALL_LABEL_MIN_ZOOM
        map_pane.object = spec
        map_pane.tooltips = tooltips
        if view_state:
            map_pane.view_state = view_state

    controller.param.watch(lambda event: update_map(event.new), "map_artifacts")
    controller.param.watch(lambda event: update_map(controller.map_artifacts), "language")
    update_map(controller.map_artifacts)
    rainfall_period = pn.widgets.IntInput.from_param(
        controller.param.rainfall_period,
        name="Rainfall period",
        sizing_mode="stretch_width",
    )
    rainfall_help = pn.pane.Markdown(
        translate(controller.language, "monitor.rainfall_help"), sizing_mode="stretch_width",
    )
    basin_mini = pn.widgets.TextInput.from_param(
        controller.param.draft_basin_mini,
        name="Basin mini",
        placeholder="empty clears boundary",
        sizing_mode="stretch_width",
    )
    apply_map_configuration = pn.widgets.Button(
        name="Apply map configuration",
        button_type="primary",
        sizing_mode="stretch_width",
    )
    apply_map_configuration.on_click(lambda _: controller.apply_map_configuration())
    legacy_controls = pn.Column(
        pn.widgets.RadioButtonGroup(name="Rainfall source", visible=False),
        pn.widgets.IntInput(name="Hours before reference time", visible=False),
        pn.widgets.IntInput(name="Hours after reference time", visible=False),
        pn.widgets.Button(name="Apply rainfall period", visible=False),
        pn.widgets.Checkbox(name="Show selected basin", visible=False),
        visible=False,
    )
    opacity_slider = pn.widgets.FloatSlider.from_param(
        controller.param.raster_opacity,
        name="Raster opacity",
        sizing_mode="stretch_width",
    )
    opacity_slider.jscallback(
        args={"deck": map_pane},
        value="""
        const view = Bokeh.index.find_one_by_id(deck.id)
        if (view == null || view.deckGL == null) {
          return
        }
        const layers = view.deckGL.props.layers.map((layer) =>
          String(layer.id || "").startsWith("rainfall-raster:")
            ? layer.clone({opacity: cb_obj.value})
            : layer
        )
        view.deckGL.setProps({layers})
        """,
    )
    map_pane.param.watch(
        lambda event: controller.handle_map_click(event.new), "click_state"
    )

    inspection = pn.pane.Markdown("")

    def update_inspection(event: Any = None) -> None:
        value = controller.raster_inspection
        inspection.object = (
            ""
            if value is None
            else (
                translate(
                    controller.language, "monitor.inspection",
                    layer=value.layer_name,
                    value=_format_number(value.value, "mm", language=controller.language),
                    latitude=f"{value.latitude:.4f}", longitude=f"{value.longitude:.4f}",
                )
            )
        )

    controller.param.watch(update_inspection, "raster_inspection")
    controller.param.watch(update_inspection, "language")
    previous_hours = pn.widgets.IntInput.from_param(
        controller.param.summary_previous_hours,
        name="Previous rainfall period (hours)",
        sizing_mode="stretch_width",
    )
    forecast_hours = pn.widgets.IntInput.from_param(
        controller.param.summary_forecast_hours,
        name="Forecast rainfall period (hours)",
        sizing_mode="stretch_width",
    )
    period_controls = pn.Row(
        previous_hours,
        forecast_hours,
        sizing_mode="stretch_width",
    )
    selected_area_summary = pn.bind(
        lambda *_: _selected_area_summary(controller, period_controls, controller.language),
        controller.param.station_id,
        controller.param.mini_id,
        controller.param.summary_previous_hours,
        controller.param.summary_forecast_hours,
        controller.param.source_versions,
        controller.param.scenario_id,
        controller.param.language,
    )
    def comparison_plot(*_: Any) -> Any:
        observed = controller.chart_observed_series()
        model_series = controller.comparison_model_series()
        return _comparison_chart(
            observed,
            model_series,
            controller.station_id,
            controller.mini_id,
            controller.station_reference_levels(),
            controller._runtime_reference_time,
            controller.language,
        )

    comparison_figure = pn.bind(
        comparison_plot,
        controller.param.station_id,
        controller.param.mini_id,
        controller.param.source_versions,
        controller.param.comparison_scenario_ids,
        controller.param.start_time,
        controller.param.language,
    )
    png_resolution = pn.widgets.Select(
        name=translate(controller.language, "monitor.png_resolution"),
        options={"1×": 1, "2×": 2, "3×": 3},
        value=1,
        sizing_mode="stretch_width",
    )
    comparison = pn.pane.Plotly(
        comparison_figure,
        config={
            "responsive": True,
            "toImageButtonOptions": {
                "format": "png",
                "width": 1200,
                "height": 780,
                "scale": png_resolution.value,
            },
        },
        sizing_mode="stretch_width",
    )
    comparison.jscallback(
        config="""
        const view = Bokeh.index.find_one_by_id(cb_obj.id)
        if (view == null || view.container == null || view.container._context == null) return
        view.container._context.toImageButtonOptions = cb_obj.config.toImageButtonOptions
        """
    )

    def update_png_resolution(event: Any) -> None:
        comparison.config = {
            **comparison.config,
            "toImageButtonOptions": {
                **comparison.config["toImageButtonOptions"],
                "scale": event.new,
            },
        }

    png_resolution.param.watch(update_png_resolution, "value")
    chart_start = pn.widgets.DatetimeInput.from_param(
        controller.param.start_time,
        name="Chart start time",
        sizing_mode="stretch_width",
    )
    map_card = pn.Card(
            pn.Row(
                scenario,
                rainfall_period,
                basin_mini,
                apply_map_configuration,
                sizing_mode="stretch_width",
            ),
            rainfall_help,
            pn.Row(opacity_slider, sizing_mode="stretch_width"),
            legacy_controls,
            map_pane,
            inspection,
            title=translate(controller.language, "monitor.map"),
            sizing_mode="stretch_width",
        )
    area_card = pn.Card(
            comparisons,
            selected_area_summary,
            title=translate(controller.language, "monitor.selected_area"),
            sizing_mode="stretch_width",
        )
    comparison_card = pn.Card(
            pn.Row(chart_start, png_resolution, sizing_mode="stretch_width"),
            comparison,
            title=translate(controller.language, "monitor.comparison"),
            sizing_mode="stretch_width",
        )

    def update_language(language: str) -> None:
        for widget, key in (
            (scenario, "monitor.scenario"),
            (comparisons, "monitor.compare"),
            (rainfall_period, "monitor.rainfall_period"),
            (apply_map_configuration, "monitor.apply_map"),
            (opacity_slider, "monitor.raster_opacity"),
            (previous_hours, "monitor.previous_rain_hours"),
            (forecast_hours, "monitor.forecast_rain_hours"),
            (chart_start, "monitor.chart_start"),
            (png_resolution, "monitor.png_resolution"),
            (legacy_controls[0], "monitor.rainfall_source"),
            (legacy_controls[1], "monitor.hours_before"),
            (legacy_controls[2], "monitor.hours_after"),
            (legacy_controls[3], "monitor.apply_rainfall"),
            (legacy_controls[4], "monitor.show_basin"),
        ):
            set_widget_text(widget, language, key)
        set_widget_text(basin_mini, language, "monitor.basin_mini", placeholder="monitor.basin_placeholder")
        rainfall_help.object = translate(language, "monitor.rainfall_help")
        map_card.title = translate(language, "monitor.map")
        area_card.title = translate(language, "monitor.selected_area")
        comparison_card.title = translate(language, "monitor.comparison")

    controller.param.watch(lambda event: update_language(event.new), "language")
    update_language(controller.language)
    return pn.Column(
        map_card,
        area_card,
        comparison_card,
        sizing_mode="stretch_width",
    )
