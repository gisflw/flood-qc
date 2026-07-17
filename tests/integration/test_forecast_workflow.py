from __future__ import annotations

from copy import deepcopy
from datetime import datetime, timedelta
from pathlib import Path
from types import SimpleNamespace

import pandas as pd
import pytest

from mgb_ops.config.env import RuntimeEnv
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.config.settings import DEFAULT_SETTINGS
from mgb_ops.config.workspace import RuntimePaths
from mgb_ops.workflows import forecast


class FakeNoaaAdapter:
    provider_code = "noaa"
    product_config = SimpleNamespace(model="gfs", product_type="fc")

    def __init__(self, download, process) -> None:
        self.download = download
        self.process = process
        self.download_cycles: list[datetime] = []
        self.processed_gribs: list[Path] = []

    def asset_id(self, cycle_time: datetime) -> str:
        return f"noaa.{cycle_time:%Y%m%d%H}"

    def download_grib(self, **kwargs) -> Path:
        cycle = kwargs["cycle_time"]
        self.download_cycles.append(cycle)
        return self.download(cycle)

    def process_grib(self, grib_path: Path, **kwargs):
        self.processed_gribs.append(grib_path)
        return self.process(grib_path, kwargs["cycle_time"])


def _context(tmp_path: Path) -> RuntimeContext:
    settings = deepcopy(DEFAULT_SETTINGS)
    settings["forecast"].update(provider="noaa", lookback_cycles=2)
    settings["spatial_grid"].update(bbox=[-52.0, -31.0, -50.0, -30.0], resolution_degrees=0.25)
    settings["mgb"]["forecast_horizon_days"] = 1
    return RuntimeContext(paths=RuntimePaths(tmp_path), settings=settings, env=RuntimeEnv({}))


def _install_adapter(monkeypatch, adapter: FakeNoaaAdapter) -> None:
    monkeypatch.setattr(forecast, "get_forecast_adapter", lambda provider: adapter)
    monkeypatch.setattr(forecast, "list_forecast_assets", lambda *args, **kwargs: pd.DataFrame())


def _normalized(cycle: datetime, *, valid_to: datetime | None = None) -> SimpleNamespace:
    return SimpleNamespace(
        asset_path=Path(f"/assets/{cycle:%Y%m%d%H}.nc"),
        valid_from=cycle,
        valid_to=valid_to or cycle + timedelta(days=3),
    )


def test_forecast_retries_only_acquisition_failures(monkeypatch, tmp_path) -> None:
    first_grib = Path("/downloads/older.grib2")

    def download(cycle: datetime) -> Path:
        if len(adapter.download_cycles) == 1:
            raise RuntimeError("newest cycle unavailable")
        return first_grib

    adapter = FakeNoaaAdapter(download, lambda grib, cycle: _normalized(cycle))
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    summary = forecast.download_forecast_data(context, reference_time=datetime(2026, 3, 18, 15))

    assert len(adapter.download_cycles) == 2
    assert adapter.download_cycles[1] == adapter.download_cycles[0] - timedelta(hours=6)
    assert adapter.processed_gribs == [first_grib]
    assert summary.raw_grib_paths == (first_grib,)
    newest_log = context.paths.logs_dir / "forecast_noaa" / f"{adapter.download_cycles[0]:%Y%m%dT%H%M%S}.log"
    assert "noaa_acquisition_failed" in newest_log.read_text(encoding="utf-8")


def test_forecast_conversion_failure_does_not_try_an_older_cycle(monkeypatch, tmp_path) -> None:
    conversion_error = ValueError("NetCDF conversion failed")
    adapter = FakeNoaaAdapter(
        lambda cycle: Path("/downloads/newest.grib2"),
        lambda grib, cycle: (_ for _ in ()).throw(conversion_error),
    )
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    with pytest.raises(ValueError, match="NetCDF conversion failed") as exc_info:
        forecast.download_forecast_data(context, reference_time=datetime(2026, 3, 18, 15))

    assert exc_info.value is conversion_error
    assert len(adapter.download_cycles) == 1
    log_path = context.paths.logs_dir / "forecast_noaa" / f"{adapter.download_cycles[0]:%Y%m%dT%H%M%S}.log"
    log_text = log_path.read_text(encoding="utf-8")
    assert "noaa_conversion_failed" in log_text
    assert "noaa_acquisition_failed" not in log_text


def test_forecast_coverage_failure_does_not_try_an_older_cycle(monkeypatch, tmp_path) -> None:
    adapter = FakeNoaaAdapter(
        lambda cycle: Path("/downloads/newest.grib2"),
        lambda grib, cycle: _normalized(cycle, valid_to=cycle),
    )
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    with pytest.raises(ValueError, match="does not cover the required forecast window"):
        forecast.download_forecast_data(context, reference_time=datetime(2026, 3, 18, 15))

    assert len(adapter.download_cycles) == 1
    log_path = context.paths.logs_dir / "forecast_noaa" / f"{adapter.download_cycles[0]:%Y%m%dT%H%M%S}.log"
    log_text = log_path.read_text(encoding="utf-8")
    assert "noaa_validation_failed" in log_text
    assert "noaa_acquisition_failed" not in log_text
