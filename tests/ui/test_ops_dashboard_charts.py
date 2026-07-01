from __future__ import annotations

import pandas as pd

from apps.ops_dashboard.views.charts import (
    MINI_COLOR,
    STATION_COLOR,
    _comparison_chart,
)


def _observed() -> pd.DataFrame:
    times = pd.date_range("2026-06-01", periods=2, freq="h")
    return pd.DataFrame(
        {
            "datetime": list(times) * 3,
            "variable_code": ["rain"] * 2 + ["level"] * 2 + ["flow"] * 2,
            "value": [1.0, 2.0, 120.0, 125.0, 10.0, 11.0],
        }
    )


def _model(variable: str, values: list[float]) -> pd.DataFrame:
    return pd.DataFrame(
        {
            "dt": pd.date_range("2026-06-01", periods=2, freq="h"),
            "value": values,
            "prev_flag": [0, 1],
            "display_name": [variable.title()] * 2,
            "unit": ["m" if variable == "level" else "mm"] * 2,
        }
    )


def test_comparison_chart_is_empty_without_selections() -> None:
    figure = _comparison_chart(pd.DataFrame(), {}, None, None)

    assert not figure.data
    assert figure.layout.annotations[-1].text == (
        "Click a station and/or a mini on the map."
    )
    assert figure.layout.title.text == "Station and Mini Comparison"


def test_comparison_chart_preserves_panel_order_and_converts_mini_level() -> None:
    figure = _comparison_chart(
        _observed(),
        {
            "precipitation": _model("precipitation", [3.0, 4.0]),
            "level": _model("level", [1.5, 1.6]),
            "flow": _model("flow", [12.0, 13.0]),
        },
        "1001",
        7,
    )

    subplot_titles = [
        annotation.text for annotation in figure.layout.annotations[:3]
    ]
    assert subplot_titles == ["Precipitation", "Level", "Flow"]
    assert figure.layout.yaxis.title.text == "mm"
    assert figure.layout.yaxis2.title.text == "cm"
    assert figure.layout.yaxis3.title.text == "m³/s"

    station_level = next(
        trace for trace in figure.data if trace.name == "Station 1001 · cm"
    )
    mini_levels = [
        trace
        for trace in figure.data
        if trace.name.startswith("Mini 7") and trace.yaxis == "y2"
    ]
    assert list(station_level.y) == [120.0, 125.0]
    assert [list(trace.y) for trace in mini_levels] == [[150.0], [160.0]]
    assert station_level.line.color == STATION_COLOR
    assert {trace.line.color for trace in mini_levels} == {MINI_COLOR}
    assert {trace.line.dash for trace in mini_levels} == {"solid", "dash"}


def test_comparison_chart_supports_each_selection_independently() -> None:
    station = _comparison_chart(_observed(), {}, "1001", None)
    mini = _comparison_chart(
        pd.DataFrame(),
        {"flow": _model("flow", [12.0, 13.0])},
        None,
        7,
    )

    assert station.data
    assert station.layout.title.text == "Station 1001"
    assert mini.data
    assert mini.layout.title.text == "Mini 7"
