"""Shared forecast-acquisition error types."""

from __future__ import annotations

from datetime import datetime


class ForecastCycleUnavailableError(RuntimeError):
    """A remote forecast cycle is unavailable and another cycle may be tried.

    This is deliberately narrow: callers must not use it for dependency,
    filesystem, GRIB decoding, conversion, or validation failures.
    """

    def __init__(self, message: str, *, provider: str | None = None, cycle_time: datetime | None = None) -> None:
        self.provider = provider
        self.cycle_time = cycle_time
        super().__init__(message)

