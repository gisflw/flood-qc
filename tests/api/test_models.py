from __future__ import annotations

from mgb_ops.common.models import DataState, TimeSeriesRecord


def test_dataclass_instantiation() -> None:
    record = TimeSeriesRecord(
        series_id="ana.level.123",
        station_code="123",
        variable="level",
        unit="cm",
        state=DataState.RAW,
    )
    assert record.station_code == "123"
