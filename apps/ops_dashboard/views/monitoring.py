"""Monitoring view composition and Panel callback wiring."""
from __future__ import annotations

from typing import Any

import pandas as pd
import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.charts import _comparison_chart
from apps.ops_dashboard.views.summaries import (
    _format_number,
    _selected_area_summary,
)


def _monitoring_view(
    controller: DashboardState,
) -> pn.viewable.Viewable:
    artifacts = controller.map_artifacts
    map_pane = pn.pane.DeckGL(
        artifacts.spec,
        tooltips=artifacts.tooltips,
        height=590,
        sizing_mode="stretch_width",
        name="Operational map",
    )

    def update_map(event: Any) -> None:
        map_pane.object = event.new.spec
        map_pane.tooltips = event.new.tooltips

    controller.param.watch(update_map, "map_artifacts")
    rainfall_mode = pn.widgets.RadioButtonGroup(
        name="Rainfall source",
        options={"Observed": "observed", "Forecast": "forecast"},
        value=controller.rainfall_mode,
        button_type="default",
    )
    rainfall_mode.param.watch(
        lambda event: setattr(controller, "rainfall_mode", event.new), "value"
    )
    observed_hours = pn.widgets.IntInput.from_param(
        controller.param.rainfall_hours,
        name="Hours before reference time",
        sizing_mode="stretch_width",
    )
    map_forecast_hours = pn.widgets.IntInput.from_param(
        controller.param.forecast_rainfall_hours,
        name="Hours after reference time",
        sizing_mode="stretch_width",
    )
    apply_rainfall = pn.widgets.Button(
        name="Apply rainfall period",
        button_type="primary",
        sizing_mode="stretch_width",
    )
    apply_rainfall.on_click(lambda _: controller.apply_rainfall_hours())
    opacity_slider = pn.widgets.FloatSlider.from_param(
        controller.param.raster_opacity,
        name="Raster opacity",
        sizing_mode="stretch_width",
    )
    show_basin = pn.widgets.Checkbox.from_param(
        controller.param.show_selected_basin,
        name="Show selected basin",
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
                f"**{value.layer_name}:** "
                f"{_format_number(value.value, 'mm')} at "
                f"{value.latitude:.4f}, {value.longitude:.4f}"
            )
        )

    controller.param.watch(update_inspection, "raster_inspection")
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
        lambda *_: _selected_area_summary(controller, period_controls),
        controller.param.station_id,
        controller.param.mini_id,
        controller.param.summary_previous_hours,
        controller.param.summary_forecast_hours,
        controller.param.source_versions,
    )
    def comparison_plot(*_: Any) -> pn.viewable.Viewable:
        observed = controller.observed_series()
        try:
            precipitation = controller.basin_precipitation()
        except (FileNotFoundError, OSError, TypeError, ValueError) as exc:
            precipitation = pd.DataFrame()
            if controller.mini_id is not None:
                controller.add_warning(f"Basin precipitation unavailable: {exc}")
        try:
            levels = controller.mgb_series("level")
            flows = controller.mgb_series("flow")
        except (FileNotFoundError, OSError, ValueError):
            levels = flows = pd.DataFrame()
        return pn.pane.Plotly(
            _comparison_chart(
                observed,
                {
                    "precipitation": precipitation,
                    "level": levels,
                    "flow": flows,
                },
                controller.station_id,
                controller.mini_id,
            ),
            config={"responsive": True},
            sizing_mode="stretch_width",
        )

    comparison = pn.bind(
        comparison_plot,
        controller.param.station_id,
        controller.param.mini_id,
        controller.param.source_versions,
    )
    return pn.Column(
        pn.Card(
            pn.Row(
                rainfall_mode,
                observed_hours,
                map_forecast_hours,
                apply_rainfall,
                sizing_mode="stretch_width",
            ),
            pn.Row(opacity_slider, show_basin, sizing_mode="stretch_width"),
            map_pane,
            inspection,
            title="Operational Map",
            sizing_mode="stretch_width",
        ),
        pn.Card(
            selected_area_summary,
            title="Selected Area",
            sizing_mode="stretch_width",
        ),
        pn.Card(
            comparison,
            title="Observed and Modeled Comparison",
            sizing_mode="stretch_width",
        ),
        sizing_mode="stretch_width",
    )
