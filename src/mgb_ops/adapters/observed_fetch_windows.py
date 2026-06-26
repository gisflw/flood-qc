from __future__ import annotations

from datetime import date, timedelta
from typing import Iterable, Iterator


DEFAULT_FETCH_WINDOW_DAYS = 30


def iter_fetch_date_windows(
    request_dates: Iterable[date],
    *,
    fetch_window_days: int = DEFAULT_FETCH_WINDOW_DAYS,
) -> Iterator[tuple[date, date]]:
    if fetch_window_days < 1:
        raise ValueError("fetch_window_days must be >= 1.")

    dates = sorted(set(request_dates))
    if not dates:
        return

    window_start = dates[0]
    window_end = dates[0]
    window_size = 1

    for request_date in dates[1:]:
        is_contiguous = request_date == window_end + timedelta(days=1)
        if is_contiguous and window_size < fetch_window_days:
            window_end = request_date
            window_size += 1
            continue

        yield window_start, window_end
        window_start = request_date
        window_end = request_date
        window_size = 1

    yield window_start, window_end
