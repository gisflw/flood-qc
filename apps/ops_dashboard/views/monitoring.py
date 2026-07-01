"""Monitoring view composition and Panel callback wiring."""
from __future__ import annotations

from typing import Any

import pandas as pd
import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.charts import _comparison_chart
from apps.ops_dashboard.views.summaries import (
    _format_number,
    _mini_summary,
    _station_summary,
)


def _monitoring_view(
    controller: DashboardState,
    opacity_slider: pn.widgets.FloatSlider | None = None,
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
    if opacity_slider is not None:
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
    station_summary = pn.bind(
        lambda *_: _station_summary(controller),
        controller.param.station_id,
        controller.param.source_versions,
    )
    mini_summary = pn.bind(
        lambda *_: _mini_summary(controller),
        controller.param.mini_id,
        controller.param.source_versions,
    )
    def comparison_plot(*_: Any) -> pn.viewable.Viewable:
        observed = controller.observed_series()
        try:
            precipitation = controller.mgb_series("precipitation")
            levels = controller.mgb_series("level")
            flows = controller.mgb_series("flow")
        except (FileNotFoundError, OSError, ValueError):
            precipitation = levels = flows = pd.DataFrame()
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
            map_pane,
            inspection,
            title="Operational Map",
            sizing_mode="stretch_width",
        ),
        pn.Row(
            pn.Card(station_summary, title="Station Summary", sizing_mode="stretch_width"),
            pn.Card(mini_summary, title="Mini Summary", sizing_mode="stretch_width"),
            sizing_mode="stretch_width",
        ),
        pn.Card(
            comparison,
            title="Station and Mini Comparison",
            sizing_mode="stretch_width",
        ),
        sizing_mode="stretch_width",
    )
