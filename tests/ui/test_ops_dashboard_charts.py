from __future__ import annotations

import inspect

import pandas as pd
import pytest

from apps.ops_dashboard.services.loaders import (
    _prepared_mgb_level,
    prepare_mgb_level_series,
)
from apps.ops_dashboard.views.charts import MINI_COLOR, STATION_COLOR, _comparison_chart


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
            "dt": pd.date_range("2026-06-01", periods=len(values), freq="h"),
            "value": values,
            "prev_flag": [0] + [1] * (len(values) - 1),
            "display_name": [variable.title()] * len(values),
            "unit": ["m" if variable == "level" else "mm"] * len(values),
        }
    )


def test_comparison_chart_is_empty_without_selections() -> None:
    figure = _comparison_chart(pd.DataFrame(), {}, None, None)

    assert not figure.data
    assert figure.layout.annotations[-1].text == "Click a station and/or a mini on the map."


def test_comparison_chart_keeps_station_and_prepared_mini_levels_independent() -> None:
    prepared_level = _model("level", [120.0, 120.1])
    figure = _comparison_chart(
        _observed(),
        {
            "precipitation": _model("precipitation", [3.0, 4.0]),
            "level": prepared_level,
            "flow": _model("flow", [12.0, 13.0]),
        },
        "1001",
        7,
    )

    station_level = next(trace for trace in figure.data if trace.name == "Observed" and trace.yaxis == "y3")
    mini_levels = [trace for trace in figure.data if trace.name in {"Simulated", "Basin forecast"} and trace.yaxis == "y3"]
    assert list(station_level.y) == [120.0, 125.0]
    assert [list(trace.y) for trace in mini_levels] == [[120.0], [120.1]]
    assert station_level.line.color == STATION_COLOR
    assert {trace.line.color for trace in mini_levels} == {MINI_COLOR}


def test_prepared_level_uses_latest_shared_observation_for_all_segments() -> None:
    station = pd.DataFrame(
        {
            "datetime": pd.to_datetime(["1999-01-01", "2099-01-01"]),
            "variable_code": ["level", "level"],
            "value": [120.0, 125.0],
        }
    )
    model = pd.DataFrame(
        {
            "dt": pd.to_datetime(["1999-01-01", "2099-01-01", "2199-01-01"]),
            "value": [10.0, 14.0, 18.0],
            "prev_flag": [0, 0, 1],
        }
    )

    prepared = prepare_mgb_level_series(model, station)

    assert prepared["value"].tolist() == [121.0, 125.0, 129.0]
    assert prepared["prev_flag"].tolist() == [0, 0, 1]


def test_prepared_level_is_empty_without_overlap_or_station_level() -> None:
    model = _model("level", [1.0, 2.0])
    unrelated = pd.DataFrame(
        {"datetime": [pd.Timestamp("2026-06-03")], "variable_code": ["level"], "value": [100.0]}
    )

    assert prepare_mgb_level_series(model, unrelated).empty
    assert prepare_mgb_level_series(model, pd.DataFrame()).empty


def test_unassociated_mini_keeps_selected_station_data_and_mini_flow() -> None:
    figure = _comparison_chart(
        _observed(),
        {"flow": _model("flow", [12.0, 13.0]), "level": pd.DataFrame()},
        "1001",
        7,
    )

    assert next(trace for trace in figure.data if trace.name == "Observed" and trace.yaxis == "y3")
    assert [trace for trace in figure.data if trace.name in {"Simulated", "Basin forecast"} and trace.yaxis == "y2"]
    assert not [trace for trace in figure.data if trace.name in {"Simulated", "Basin forecast"} and trace.yaxis == "y3"]


def test_unrelated_station_remains_visible_with_prepared_mini_level() -> None:
    figure = _comparison_chart(
        _observed(),
        {"level": _model("level", [50.0, 51.0])},
        "unrelated",
        7,
    )

    station_level = next(trace for trace in figure.data if trace.name == "Observed" and trace.yaxis == "y3")
    mini_levels = [trace for trace in figure.data if trace.name in {"Simulated", "Basin forecast"} and trace.showlegend is False]
    assert list(station_level.y) == [120.0, 125.0]
    assert [list(trace.y) for trace in mini_levels] == [[50.0], [51.0]]


def test_prepared_level_cache_has_no_history_version_key() -> None:
    assert "history_version" not in inspect.signature(_prepared_mgb_level).parameters


def test_comparison_chart_overlays_scenarios_without_repeating_observations() -> None:
    observed = pd.DataFrame(
        {"variable_code": ["flow"], "datetime": pd.to_datetime(["2026-03-12T00:00:00"]), "value": [5.0]}
    )
    frame_a = pd.DataFrame(
        {"dt": pd.to_datetime(["2026-03-12T00:00:00", "2026-03-12T01:00:00"]), "prev_flag": [0, 1], "value": [1.0, 2.0]}
    )
    frame_b = frame_a.assign(value=[1.5, 3.0])

    figure = _comparison_chart(observed, {"Zero": {"flow": frame_a}, "ECMWF raw": {"flow": frame_b}}, "ana:1", 7)

    station_traces = [trace for trace in figure.data if trace.name == "Observed" and trace.showlegend is False]
    scenario_traces = [trace for trace in figure.data if trace.name in {"Simulated", "Zero", "ECMWF raw"} and trace.showlegend is False]
    assert len(station_traces) == 1
    assert len(scenario_traces) == 3
    simulated = next(trace for trace in scenario_traces if trace.name == "Simulated")
    forecasts = [trace for trace in scenario_traces if trace.line.dash == "dash"]
    assert simulated.line.color == MINI_COLOR
    assert len(forecasts) == 2
    assert len({trace.line.color for trace in forecasts}) == 2

    flow_legend = [trace for trace in figure.data if trace.showlegend and trace.legend == "legend2"]
    assert [trace.name for trace in flow_legend] == ["Observed", "Simulated", "Zero", "ECMWF raw"]
    assert all(trace.type == "scatter" for trace in flow_legend)
    assert [trace.legendgrouptitle.text for trace in flow_legend] == ["Up to present", "Up to present", "Future scenarios", "Future scenarios"]
    assert [trace.line.dash for trace in flow_legend] == ["solid", "solid", "dash", "dash"]


def test_precipitation_legend_matches_bar_styles() -> None:
    figure = _comparison_chart(
        _observed(),
        {"precipitation": _model("precipitation", [3.0, 4.0])},
        "1001",
        7,
    )

    legend = [trace for trace in figure.data if trace.showlegend and trace.legend == "legend"]
    assert [trace.name for trace in legend] == ["observed at station", "aggregated at basin", "Basin forecast"]
    assert [trace.legendgrouptitle.text for trace in legend] == ["Observed", "Observed", "Forecast"]
    plotted = [trace for trace in figure.data if not trace.showlegend and trace.type == "bar"]
    assert {trace.name for trace in plotted} == {trace.name for trace in legend}
    assert all(trace.type == "bar" for trace in legend)
    assert legend[1].opacity == 0.85
    assert legend[2].opacity == 0.55
    assert legend[2].marker.pattern.shape == "/"



def test_station_bar_width_is_capped_when_observations_are_sparse() -> None:
    observed = pd.DataFrame({
        "datetime": pd.to_datetime(["2026-01-01", "2026-01-10"]),
        "variable_code": ["rain", "rain"],
        "value": [1.0, 2.0],
    })

    figure = _comparison_chart(observed, {}, "1001", None)

    station_bars = next(trace for trace in figure.data if trace.name == "observed at station" and not trace.showlegend)
    assert station_bars.width == 2_880_000.0


def test_reference_levels_appear_in_the_legend_without_annotations() -> None:
    references = pd.DataFrame({
        "reference_type": ["alert", "historical_flood"],
        "level_cm": [650.0, 700.0],
        "event_date": [None, "2024-05-03"],
    })

    figure = _comparison_chart(pd.DataFrame(), {}, "1001", None, references)

    legend = [trace for trace in figure.data if trace.showlegend and trace.legend == "legend2"]
    assert [trace.name for trace in legend] == ["Historical flood · 2024-05-03 (700 cm)", "Alert (650 cm)"]
    assert [trace.legendgrouptitle.text for trace in legend] == ["Reference levels", "Reference levels"]
    assert not any(annotation.text.startswith("Alert") for annotation in figure.layout.annotations)



def test_reference_time_marker_spans_all_subplots_without_a_legend_entry() -> None:
    reference_time = pd.Timestamp("2026-03-12 06:00")
    figure = _comparison_chart(
        _observed(), {"flow": _model("flow", [1.0, 2.0])}, "1001", 7, reference_time=reference_time
    )

    assert len(figure.layout.shapes) == 3
    assert all(shape.line.color == "rgba(128, 128, 128, 0.2)" for shape in figure.layout.shapes)
    assert all(shape.line.dash == "dash" for shape in figure.layout.shapes)
    assert sum(annotation.text == "Reference time<br>12/03/2026 06:00" for annotation in figure.layout.annotations) == 3
    assert [figure.layout.yaxis.title.text, figure.layout.yaxis2.title.text, figure.layout.yaxis3.title.text] == [
        "Precipitation<br>mm", "Discharge<br>m³/s", "Water Level<br>cm"
    ]



@pytest.mark.parametrize("value, expected", [(1.29, 1.2), (-1.29, -1.2)])
def test_comparison_chart_truncates_trace_values_toward_zero(value: float, expected: float) -> None:
    observed = _observed().assign(value=[value] * 6)
    figure = _comparison_chart(observed, {}, "1001", None)

    assert all(list(trace.y) == [expected, expected] for trace in figure.data if not trace.showlegend)
