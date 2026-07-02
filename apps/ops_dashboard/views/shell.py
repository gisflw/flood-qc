"""Responsive dashboard shell and top-level view placement."""
from __future__ import annotations

import panel as pn

from apps.ops_dashboard.state import DashboardState
from apps.ops_dashboard.views.forecast import _forecast_view
from apps.ops_dashboard.views.monitoring import _monitoring_view
from apps.ops_dashboard.views.summaries import _network_summary


def _build_template(state: DashboardState) -> pn.template.base.BasicTemplate:
    refresh = pn.widgets.Button(
        name="Refresh data",
        button_type="primary",
        icon="refresh",
        sizing_mode="stretch_width",
    )
    refresh.on_click(lambda _: state.refresh())
    refreshed = pn.bind(
        lambda value: pn.pane.Markdown(
            f"Last session refresh:  \n{value}" if value else "Not refreshed yet."
        ),
        state.param.last_refresh_at,
    )
    warnings = pn.bind(
        lambda values: pn.Column(
            *[pn.pane.Alert(value, alert_type="warning") for value in values],
            sizing_mode="stretch_width",
        ),
        state.param.warnings,
    )
    tabs = pn.Tabs(
        ("Monitoring", _monitoring_view(state)),
        ("Forecast", _forecast_view(state)),
        dynamic=True,
        sizing_mode="stretch_width",
    )
    template = pn.template.FastListTemplate(
        title="Operational Hydrology",
        sidebar=[
            pn.pane.Markdown("## Controls"),
            refresh,
            refreshed,
            pn.layout.Divider(),
            warnings,
        ],
        main=[
            pn.pane.Markdown(
                "# Operational MGB System\n"
                "Observed and forecasted hydrological data for the operation of MGB results."
            ),
            pn.bind(
                lambda stations: _network_summary(
                    stations,
                    state.window.cutoff_time,
                ),
                state.param.stations,
            ),
            tabs,
        ],
        sidebar_width=320,
        accent_base_color="#1864ab",
        header_background="#1864ab",
    )
    template.state = state
    return template
