from __future__ import annotations

from datetime import date, datetime

from mgb_ops.common.time_utils import iter_observed_request_dates


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
