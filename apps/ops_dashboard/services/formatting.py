"""Dashboard display-number helpers."""
from __future__ import annotations

import math
from typing import Any

import numpy as np
import pandas as pd


def truncate_one_decimal(value: Any) -> float:
    """Truncate a finite numeric display value toward zero to one decimal."""
    number = float(value)
    if not math.isfinite(number):
        return number
    return math.trunc(number * 10) / 10


def truncate_series_one_decimal(values: pd.Series) -> pd.Series:
    """Truncate numeric Series values toward zero to one decimal."""
    numeric = pd.to_numeric(values, errors="coerce")
    return pd.Series(np.trunc(numeric.to_numpy(dtype=float) * 10) / 10, index=values.index)
