from __future__ import annotations

from datetime import date, datetime, timedelta
from typing import Iterable
from zoneinfo import ZoneInfo


TIMEZONE = ZoneInfo("America/Sao_Paulo")


def resolve_reference_time(raw_value: str | None) -> datetime:
    now = datetime.now(TIMEZONE)
    if raw_value in (None, "", "now"):
        return now.replace(minute=0, second=0, microsecond=0, tzinfo=None)
    if raw_value == "yesterday":
        yesterday = now.date() - timedelta(days=1)
        return datetime.combine(yesterday, datetime.min.time()) + timedelta(hours=23)

    text = str(raw_value).strip()
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    if text and "T" not in text and " " not in text:
        return datetime.fromisoformat(text) + timedelta(hours=23)
    reference_time = datetime.fromisoformat(text)
    if reference_time.tzinfo is not None:
        reference_time = reference_time.astimezone(TIMEZONE).replace(tzinfo=None)
    return reference_time.replace(second=0, microsecond=0)


def iter_observed_request_dates(
    window_start: datetime,
    window_end: datetime,
    latest_observed_at: datetime | None = None,
) -> Iterable[date]:
    if window_end < window_start:
        return

    start_date = window_start.date()
    if latest_observed_at is not None:
        start_date = max(start_date, latest_observed_at.date())
    end_date = window_end.date()

    current_date = start_date
    while current_date <= end_date:
        yield current_date
        current_date += timedelta(days=1)
