"""Monitoring view composition and Panel callback wiring."""
from __future__ import annotations

from typing import Any

import pandas as pd
import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.charts import _mgb_chart, _station_chart
from apps.ops_dashboard.views.summaries import (
    _format_number,
    _mini_summary,
    _station_summary,
)


def _monitoring_view(controller: DashboardState) -> pn.viewable.Viewable:
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
    station_plot = pn.bind(
        lambda *_: pn.pane.Plotly(
            _station_chart(controller.observed_series(), controller.station_id),
            config={"responsive": True},
            sizing_mode="stretch_width",
        ),
        controller.param.station_id,
        controller.param.source_versions,
    )

    def model_plots(*_: Any) -> pn.viewable.Viewable:
        try:
            levels = controller.mgb_series("y")
            flows = controller.mgb_series("q")
        except (FileNotFoundError, OSError, ValueError):
            levels = flows = pd.DataFrame()
        return pn.Column(
            pn.pane.Plotly(
                _mgb_chart(levels, controller.mini_id, "y"),
                config={"responsive": True},
                sizing_mode="stretch_width",
            ),
            pn.pane.Plotly(
                _mgb_chart(flows, controller.mini_id, "q"),
                config={"responsive": True},
                sizing_mode="stretch_width",
            ),
            sizing_mode="stretch_width",
        )

    mini_plots = pn.bind(
        model_plots,
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
        pn.Row(
            pn.Card(station_plot, title="Station Chart", sizing_mode="stretch_width"),
            pn.Card(mini_plots, title="Mini Charts", sizing_mode="stretch_width"),
            sizing_mode="stretch_width",
        ),
        sizing_mode="stretch_width",
    )
