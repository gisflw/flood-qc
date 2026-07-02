from __future__ import annotations

import pandas as pd
import pytest

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
    assert figure.layout.title.text == "Observed and Modeled Comparison"


def test_comparison_chart_preserves_panel_order_and_centers_levels() -> None:
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
    assert subplot_titles == [
        "Precipitation",
        "Flow",
        "Water Level Variation",
    ]
    assert figure.layout.yaxis.title.text == "mm"
    assert figure.layout.yaxis2.title.text == "m³/s"
    assert figure.layout.yaxis3.title.text == "cm"

    station_level = next(
        trace for trace in figure.data if trace.name == "Station 1001 · cm"
    )
    mini_levels = [
        trace
        for trace in figure.data
        if trace.name.startswith("Mini 7") and trace.yaxis == "y3"
    ]
    assert list(station_level.y) == [-2.5, 2.5]
    assert list(mini_levels[0].y) == pytest.approx([0.0])
    assert list(mini_levels[1].y) == pytest.approx([0.1])
    assert station_level.line.color == STATION_COLOR
    assert {trace.line.color for trace in mini_levels} == {MINI_COLOR}
    assert {trace.line.dash for trace in mini_levels} == {"solid", "dash"}


def test_level_normalization_uses_all_loaded_values_without_time_filtering() -> None:
    observed = _observed()
    observed.loc[observed["variable_code"] == "level", "datetime"] = [
        pd.Timestamp("1999-01-01"),
        pd.Timestamp("2099-01-01"),
    ]
    levels = pd.DataFrame(
        {
            "dt": pd.to_datetime(
                ["1999-01-01", "2099-01-01", "2199-01-01"]
            ),
            "value": [10.0, 14.0, 18.0],
            "prev_flag": [0, 0, 1],
        }
    )

    figure = _comparison_chart(
        observed, {"level": levels}, "1001", 7
    )

    station_level = next(
        trace for trace in figure.data if trace.name == "Station 1001 · cm"
    )
    mini_levels = [
        trace for trace in figure.data if trace.name.startswith("Mini 7")
    ]
    assert list(station_level.y) == [-2.5, 2.5]
    assert [list(trace.y) for trace in mini_levels] == [
        [-2.0, 2.0],
        [6.0],
    ]


def test_level_series_are_handled_independently_when_data_are_missing() -> None:
    station_only = _comparison_chart(_observed(), {}, "1001", None)
    forecast_only_level = pd.DataFrame(
        {
            "dt": [pd.Timestamp("2026-06-02")],
            "value": [2.0],
            "prev_flag": [1],
        }
    )
    mini_without_current = _comparison_chart(
        pd.DataFrame(), {"level": forecast_only_level}, None, 7
    )

    station_level = next(
        trace
        for trace in station_only.data
        if trace.name == "Station 1001 · cm"
    )
    assert list(station_level.y) == [-2.5, 2.5]
    assert not mini_without_current.data


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
