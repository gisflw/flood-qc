from __future__ import annotations

from dataclasses import dataclass
from datetime import date, datetime, time, timedelta, timezone
from typing import Iterable
from zoneinfo import ZoneInfo


TIMEZONE = ZoneInfo("America/Sao_Paulo")
DEFAULT_DT_SECONDS = 3600
DEFAULT_FORECAST_CYCLE_HOURS = (0, 6, 12, 18)


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


def resolve_forecast_cycle(
    reference_time: datetime,
    *,
    cycle_hours: tuple[int, ...] = DEFAULT_FORECAST_CYCLE_HOURS,
) -> datetime:
    """Resolve the previous UTC forecast cycle for a local reference time."""
    if not cycle_hours:
        raise ValueError("cycle_hours must not be empty.")
    normalized_hours = tuple(sorted(set(cycle_hours)))
    if any(hour < 0 or hour > 23 for hour in normalized_hours):
        raise ValueError("cycle_hours must be UTC hours in the range 0..23.")

    if reference_time.tzinfo is None:
        reference_utc = reference_time.replace(tzinfo=TIMEZONE).astimezone(timezone.utc)
    else:
        reference_utc = reference_time.astimezone(timezone.utc)

    current = reference_utc.replace(minute=0, second=0, microsecond=0)
    eligible = [hour for hour in normalized_hours if hour <= current.hour]
    if eligible:
        return current.replace(hour=max(eligible), tzinfo=None)
    return (current - timedelta(days=1)).replace(hour=normalized_hours[-1], tzinfo=None)


def iter_forecast_cycle_candidates(
    cycle_time: datetime,
    *,
    lookback_cycles: int,
    cycle_hours: tuple[int, ...] = DEFAULT_FORECAST_CYCLE_HOURS,
) -> Iterable[datetime]:
    if lookback_cycles < 1:
        raise ValueError("lookback_cycles must be >= 1.")
    current = resolve_forecast_cycle(cycle_time.replace(tzinfo=timezone.utc), cycle_hours=cycle_hours)
    for _ in range(lookback_cycles):
        yield current
        current = resolve_forecast_cycle(
            (current - timedelta(seconds=1)).replace(tzinfo=timezone.utc),
            cycle_hours=cycle_hours,
        )


def validate_timestep_hours(timestep_hours: int) -> int:
    if not isinstance(timestep_hours, int) or isinstance(timestep_hours, bool) or timestep_hours < 1:
        raise ValueError("timestep_hours must be an integer >= 1.")
    if 24 % timestep_hours != 0:
        raise ValueError("timestep_hours must divide 24.")
    return timestep_hours


def require_datetime_aligned_to_timestep(value: datetime, *, timestep_hours: int, name: str) -> None:
    validate_timestep_hours(timestep_hours)
    if value.minute != 0 or value.second != 0 or value.microsecond != 0:
        raise ValueError(f"{name} must be aligned to the hour.")
    if value.hour % timestep_hours != 0:
        raise ValueError(f"{name} must be aligned to run.timestep_hours={timestep_hours}.")


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
    timestep_hours: int = 1,
) -> HorizonWindow:
    timestep_hours = validate_timestep_hours(timestep_hours)
    if days_before < 0:
        raise ValueError("days_before must be >= 0.")
    if horizon_days < 0:
        raise ValueError("horizon_days must be >= 0.")
    require_datetime_aligned_to_timestep(
        reference_time,
        timestep_hours=timestep_hours,
        name="reference_time",
    )

    start_date = reference_time.date() - timedelta(days=days_before)
    start_time = datetime.combine(start_date, time.min)
    step_delta = timedelta(hours=timestep_hours)
    forecast_start_time = reference_time + step_delta
    forecast_nt = horizon_days * (24 // timestep_hours) + 1 if horizon_days > 0 else 0
    end_time = (
        forecast_start_time + step_delta * (forecast_nt - 1)
        if forecast_nt > 0
        else reference_time
    )
    dt_seconds = timestep_hours * DEFAULT_DT_SECONDS
    elapsed_seconds = int((end_time - start_time).total_seconds())
    if elapsed_seconds % dt_seconds != 0:
        raise ValueError(
            "Invalid timestep alignment calculated from "
            f"reference_time={reference_time}, start_time={start_time}, timestep_hours={timestep_hours}."
        )
    nt = elapsed_seconds // dt_seconds + 1
    if nt < 1:
        raise ValueError(f"Invalid NT calculated from reference_time={reference_time} and start_time={start_time}.")
    return HorizonWindow(
        reference_time=reference_time,
        start_time=start_time,
        forecast_start_time=forecast_start_time,
        forecast_nt=forecast_nt,
        nt=nt,
        dt_seconds=dt_seconds,
        days_before=days_before,
        horizon_days=horizon_days,
    )
