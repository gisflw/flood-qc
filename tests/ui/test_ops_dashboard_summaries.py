from __future__ import annotations

from types import SimpleNamespace

import numpy as np
import pandas as pd

from apps.ops_dashboard.views.summaries import (
    _network_summary,
    _rainfall_total,
    _selected_area_summary,
)


def test_rainfall_total_uses_open_start_and_closed_end_without_crossing_segments() -> None:
    values = pd.DataFrame(
        {
            "dt": pd.to_datetime(
                [
                    "2026-06-01 00:00",
                    "2026-06-01 01:00",
                    "2026-06-01 02:00",
                    "2026-06-01 03:00",
                ]
            ),
            "value": [100.0, 1.0, 2.0, 4.0],
            "prev_flag": [0, 0, 0, 1],
        }
    )

    past = _rainfall_total(
        values,
        time_column="dt",
        start_exclusive=pd.Timestamp("2026-06-01 00:00"),
        end_inclusive=pd.Timestamp("2026-06-01 02:00"),
        segment=0,
    )
    forecast = _rainfall_total(
        values,
        time_column="dt",
        start_exclusive=pd.Timestamp("2026-06-01 02:00"),
        end_inclusive=pd.Timestamp("2026-06-01 04:00"),
        segment=1,
    )

    assert past == 3.0
    assert forecast == 4.0


def test_rainfall_total_returns_unavailable_without_values_in_window() -> None:
    values = pd.DataFrame(
        {"datetime": [pd.Timestamp("2026-06-01")], "value": [1.0]}
    )

    result = _rainfall_total(
        values,
        time_column="datetime",
        start_exclusive=pd.Timestamp("2026-06-02"),
        end_inclusive=pd.Timestamp("2026-06-03"),
    )

    assert np.isnan(result)


def test_network_summary_includes_reference_time_with_station_totals() -> None:
    summary = _network_summary(
        pd.DataFrame(),
        pd.Timestamp("2026-06-01 02:00"),
    )
    cards = [card[0].object for card in summary]

    assert "01/06/2026 02:00" in cards[0]
    assert "Reference time" in cards[0]
    assert "Total stations" in cards[1]


class SummaryState:
    station_id = "station-1"
    mini_id = 7
    summary_previous_hours = 2
    summary_forecast_hours = 2
    window = SimpleNamespace(cutoff_time=pd.Timestamp("2026-06-01 02:00"))
    stations = pd.DataFrame(
        [
            {
                "station_id": "station-1",
                "station_name": "Test Station",
                "provider_code": "ana",
                "station_code": "123",
            }
        ]
    )

    def observed_series(self) -> pd.DataFrame:
        return pd.DataFrame(
            {
                "datetime": pd.date_range("2026-06-01", periods=3, freq="h"),
                "variable_code": ["rain", "rain", "rain"],
                "value": [100.0, 1.0, 2.0],
            }
        )

    def basin_precipitation(self) -> pd.DataFrame:
        return pd.DataFrame(
            {
                "dt": pd.date_range("2026-06-01", periods=5, freq="h"),
                "value": [100.0, 3.0, 4.0, 5.0, 6.0],
                "prev_flag": [0, 0, 0, 1, 1],
            }
        )

    def add_warning(self, message: str) -> None:
        raise AssertionError(message)


def test_selected_area_summary_shows_identity_reference_and_window_totals() -> None:
    summary = _selected_area_summary(SummaryState())
    identity = summary[0].object
    metric_html = [card[0].object for card in summary[1]]

    assert "Test Station (ANA:123)" in identity
    assert "Modeled basin 7" in identity
    assert "Reference time" not in identity
    assert "3.0 mm" in metric_html[0]
    assert "Station rainfall in the last 2 hours" in metric_html[0]
    assert "7.0 mm" in metric_html[1]
    assert "Basin rainfall in the last 2 hours" in metric_html[1]
    assert "11.0 mm" in metric_html[2]
    assert "Basin rainfall in the next 2 hours" in metric_html[2]


def test_selected_area_summary_keeps_partial_empty_selection() -> None:
    state = SummaryState()
    state.station_id = None
    state.mini_id = None

    summary = _selected_area_summary(state)
    identity = summary[0].object
    metric_html = [card[0].object for card in summary[1]]

    assert "No station selected" in identity
    assert "No modeled basin selected" in identity
    assert all("unavailable" in html for html in metric_html)


def test_selected_area_summary_supports_each_selection_independently() -> None:
    station_only = SummaryState()
    station_only.mini_id = None
    basin_only = SummaryState()
    basin_only.station_id = None

    station_summary = _selected_area_summary(station_only)
    basin_summary = _selected_area_summary(basin_only)

    assert "3.0 mm" in station_summary[1][0][0].object
    assert "unavailable" in station_summary[1][1][0].object
    assert "unavailable" in basin_summary[1][0][0].object
    assert "7.0 mm" in basin_summary[1][1][0].object


def test_selected_area_summary_sums_available_short_forecast_without_extrapolation() -> None:
    state = SummaryState()
    state.summary_forecast_hours = 4

    summary = _selected_area_summary(state)

    assert "11.0 mm" in summary[1][2][0].object
    assert "Basin rainfall in the next 4 hours" in summary[1][2][0].object
