"""Correction-table construction shared by the forecast view."""
from __future__ import annotations

from typing import TYPE_CHECKING

import panel as pn

from apps.ops_dashboard.i18n import translate, watch_language
if TYPE_CHECKING:
    from apps.ops_dashboard.state import DashboardState


def _correction_table(state: DashboardState) -> pn.widgets.Tabulator:
    """Construct the editable table and bind draft mutation to session state."""
    def titles(language: str) -> dict[str, str]:
        return {
            column: translate(language, key)
            for column, key in {
                "correction_id": "table.correction_id",
                "t0_step": "table.start_step",
                "t1_step": "table.end_step",
                "shift_lat": "table.row_shift",
                "shift_lon": "table.column_shift",
                "rotation_deg": "table.rotation",
                "multiplication_factor": "table.factor",
                "remove": "table.remove",
            }.items()
        }

    table = pn.widgets.Tabulator(
        state.forecast_draft.copy(),
        show_index=False,
        sizing_mode="stretch_width",
        height=260,
        hidden_columns=["asset_id", "metadata_json"],
        titles=titles(state.language),
        editors={
            "correction_id": None,

            "t0_step": {"type": "number", "min": 0, "step": 1},
            "t1_step": {"type": "number", "min": 0, "step": 1},
            "shift_lat": {"type": "number", "step": 1},
            "shift_lon": {"type": "number", "step": 1},
            "rotation_deg": {"type": "number", "step": 1},
            "multiplication_factor": {
                "type": "number",
                "min": 0.01,
                "step": 0.05,
            },
            "remove": {"type": "tickCross"},
        },
    )
    watch_language(
        state,
        lambda language: setattr(table, "titles", titles(language)),
    )
    table.param.watch(lambda event: state.update_forecast_draft(event.new), "value")
    return table
