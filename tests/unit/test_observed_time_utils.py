from __future__ import annotations

from datetime import date, datetime

from mgb_ops.common.time_utils import build_horizon_window, iter_observed_request_dates


def test_build_horizon_window_includes_forecast_horizon() -> None:
    window = build_horizon_window(
        datetime(2026, 3, 11, 23, 0, 0),
        days_before=2,
        horizon_days=2,
    )

    assert window.start_time == datetime(2026, 3, 9, 0, 0, 0)
    assert window.forecast_start_time == datetime(2026, 3, 12, 0, 0, 0)
    assert window.forecast_nt == 49
    assert window.nt == 121


def test_build_horizon_window_fetch_only_ends_at_reference_hour() -> None:
    window = build_horizon_window(
        datetime(2026, 3, 11, 23, 0, 0),
        days_before=89,
    )

    assert window.start_time == datetime(2025, 12, 12, 0, 0, 0)
    assert window.forecast_start_time == datetime(2026, 3, 12, 0, 0, 0)
    assert window.forecast_nt == 0
    assert window.nt == 2160


def test_build_horizon_window_uses_configured_timestep() -> None:
    window = build_horizon_window(
        datetime(2026, 3, 11, 21, 0, 0),
        days_before=2,
        horizon_days=2,
        timestep_hours=3,
    )

    assert window.start_time == datetime(2026, 3, 9, 0, 0, 0)
    assert window.forecast_start_time == datetime(2026, 3, 12, 0, 0, 0)
    assert window.forecast_nt == 17
    assert window.nt == 41
    assert window.dt_seconds == 10800


def test_build_horizon_window_rejects_reference_time_off_timestep() -> None:
    import pytest

    with pytest.raises(ValueError, match="run.timestep_hours=3"):
        build_horizon_window(
            datetime(2026, 3, 11, 23, 0, 0),
            days_before=2,
            horizon_days=2,
            timestep_hours=3,
        )


def test_iter_observed_request_dates_empty_db_starts_at_window_start() -> None:
    assert list(
        iter_observed_request_dates(
            datetime(2026, 3, 10, 0),
            datetime(2026, 3, 12, 23),
        )
    ) == [date(2026, 3, 10), date(2026, 3, 11), date(2026, 3, 12)]


def test_iter_observed_request_dates_latest_before_window_uses_window_start() -> None:
    assert list(
        iter_observed_request_dates(
            datetime(2026, 3, 10, 0),
            datetime(2026, 3, 11, 23),
            latest_observed_at=datetime(2026, 3, 8, 12),
        )
    ) == [date(2026, 3, 10), date(2026, 3, 11)]


def test_iter_observed_request_dates_latest_inside_window_overlaps_latest_day() -> None:
    assert list(
        iter_observed_request_dates(
            datetime(2026, 3, 10, 0),
            datetime(2026, 3, 12, 23),
            latest_observed_at=datetime(2026, 3, 11, 15),
        )
    ) == [date(2026, 3, 11), date(2026, 3, 12)]


def test_iter_observed_request_dates_latest_after_window_yields_no_dates() -> None:
    assert list(
        iter_observed_request_dates(
            datetime(2026, 3, 10, 0),
            datetime(2026, 3, 11, 23),
            latest_observed_at=datetime(2026, 3, 12, 0),
        )
    ) == []
