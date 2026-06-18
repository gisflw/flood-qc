from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime, time, timedelta
from typing import Iterable
from zoneinfo import ZoneInfo


TIMEZONE = ZoneInfo("America/Sao_Paulo")
DEFAULT_DT_SECONDS = 3600


@dataclass(frozen=True, slots=True)
class HorizonWindow:
    reference_time: datetime
    start_time: datetime
    forecast_start_time: datetime
    forecast_nt: int
    nt: int
    dt_seconds: int
    days_before: int
    horizon_days: int


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


def build_horizon_window(
    reference_time: datetime,
    *,
    days_before: int,
    horizon_days: int = 0,
) -> HorizonWindow:
    if days_before < 0:
        raise ValueError("days_before must be >= 0.")
    if horizon_days < 0:
        raise ValueError("horizon_days must be >= 0.")
    if reference_time.minute != 0 or reference_time.second != 0 or reference_time.microsecond != 0:
        raise ValueError("reference_time must be aligned to the hour.")

    start_date = reference_time.date() - timedelta(days=days_before)
    start_time = datetime.combine(start_date, time.min)
    forecast_start_time = reference_time + timedelta(hours=1)
    forecast_nt = horizon_days * 24 + 1 if horizon_days > 0 else 0
    end_time = (
        forecast_start_time + timedelta(hours=forecast_nt - 1)
        if forecast_nt > 0
        else reference_time
    )
    nt = int((end_time - start_time).total_seconds() // DEFAULT_DT_SECONDS) + 1
    if nt < 1:
        raise ValueError(f"Invalid NT calculated from reference_time={reference_time} and start_time={start_time}.")
    return HorizonWindow(
        reference_time=reference_time,
        start_time=start_time,
        forecast_start_time=forecast_start_time,
        forecast_nt=forecast_nt,
        nt=nt,
        dt_seconds=DEFAULT_DT_SECONDS,
        days_before=days_before,
        horizon_days=horizon_days,
    )
