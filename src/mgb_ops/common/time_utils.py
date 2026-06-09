from __future__ import annotations

from datetime import datetime, timedelta
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
