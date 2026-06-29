from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

Severity = Literal["info", "warning", "error"]


@dataclass(frozen=True, slots=True)
class QCResult:
    rule_code: str
    passed: bool
    severity: Severity
    message: str

    @property
    def is_valid(self) -> bool:
        return self.passed


STATION_AVAILABLE = "station_available"
NETCDF_CONTRACT = "netcdf_contract"
PRECIPITATION_VALID = "precipitation_valid"
CORRECTION_WINDOW = "correction_window"
CORRECTION_OVERLAP = "correction_overlap"
