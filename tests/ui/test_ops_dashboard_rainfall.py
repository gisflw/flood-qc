from __future__ import annotations
from datetime import datetime
import pytest
from apps.ops_dashboard.services import loaders
from mgb_ops.common.time_utils import DashboardWindow


def test_accumulation_raster_uses_requested_hours_and_clips_to_window(tmp_path, monkeypatch) -> None:
    captured = {}
    def fake_grid(database_path, **kwargs):
        captured.update(kwargs)
        return "grid"
    monkeypatch.setattr(loaders, "observed_rainfall_grid", fake_grid)
    window = DashboardWindow(start_time=datetime(2026, 1, 1), cutoff_time=datetime(2026, 1, 3), forecast_end_exclusive=datetime(2026, 1, 4))
    raster = loaders._accumulation_raster(
        str(tmp_path / "history.sqlite"), str(tmp_path), "v1", window,
        (-52.0, -31.0, -51.0, -30.0), 0.1, 100, 4, 2.0)
    assert captured["start_time"] == window.start_time
    assert captured["end_time"] == window.cutoff_time
    assert raster["name"] == "accum_100h"
    assert raster["horizon_hours"] == 100
    assert raster["horizon_label"] == "100h"


@pytest.mark.parametrize("hours", [0, -1, 1.5, True])
def test_accumulation_raster_rejects_invalid_hours(tmp_path, hours) -> None:
    window = DashboardWindow(start_time=datetime(2026, 1, 1), cutoff_time=datetime(2026, 1, 3), forecast_end_exclusive=datetime(2026, 1, 4))
    with pytest.raises(ValueError, match="positive integer"):
        loaders._accumulation_raster(
            str(tmp_path / "history.sqlite"), str(tmp_path), "v1", window,
            (-52.0, -31.0, -51.0, -30.0), 0.1, hours, 4, 2.0)
