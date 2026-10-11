from __future__ import annotations

from copy import deepcopy
from datetime import datetime, timedelta, timezone
from pathlib import Path
from types import SimpleNamespace

import pandas as pd
import pytest

from mgb_ops.adapters import ForecastCycleUnavailableError
from mgb_ops.adapters.forecast_ecmwf import ECMWF_FORECAST_PRODUCT
from mgb_ops.adapters import forecast_ecmwf, forecast_noaa
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
    settings["forecast"]["lookback_cycles"] = 2
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
            raise ForecastCycleUnavailableError("newest cycle unavailable")
        return first_grib

    adapter = FakeNoaaAdapter(download, lambda grib, cycle: _normalized(cycle))
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    summary = forecast.download_forecast_data(context, providers="noaa", reference_time=datetime(2026, 3, 18, 15))

    assert len(adapter.download_cycles) == 2
    assert adapter.download_cycles[1] == adapter.download_cycles[0] - timedelta(hours=6)
    assert adapter.processed_gribs == [first_grib]
    assert summary.raw_grib_paths == (first_grib,)
    newest_log = context.paths.logs_dir / "forecast_noaa" / f"{adapter.download_cycles[0]:%Y%m%dT%H%M%S}.log"
    assert "forecast_acquisition_unavailable" in newest_log.read_text(encoding="utf-8")


def test_forecast_conversion_failure_does_not_try_an_older_cycle(monkeypatch, tmp_path) -> None:
    conversion_error = ValueError("NetCDF conversion failed")
    adapter = FakeNoaaAdapter(
        lambda cycle: Path("/downloads/newest.grib2"),
        lambda grib, cycle: (_ for _ in ()).throw(conversion_error),
    )
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    with pytest.raises(ValueError, match="NetCDF conversion failed") as exc_info:
        forecast.download_forecast_data(context, providers="noaa", reference_time=datetime(2026, 3, 18, 15))

    assert exc_info.value is conversion_error
    assert len(adapter.download_cycles) == 1
    log_path = context.paths.logs_dir / "forecast_noaa" / f"{adapter.download_cycles[0]:%Y%m%dT%H%M%S}.log"
    log_text = log_path.read_text(encoding="utf-8")
    assert "forecast_conversion_failed" in log_text
    assert "noaa_acquisition_failed" not in log_text


def test_cache_requires_every_native_step_through_the_last_interval() -> None:
    cycle = datetime(2026, 3, 18, tzinfo=timezone.utc)
    hours = tuple(step for step in forecast_noaa.GFS_FORECAST_PRODUCT.step_schedule if step > 0)
    bounds = tuple(
        (cycle + timedelta(hours=start), cycle + timedelta(hours=end))
        for start, end in zip((0, *hours[:-1]), hours)
    )
    adapter = SimpleNamespace(product_config=forecast_noaa.GFS_FORECAST_PRODUCT)

    assert forecast._has_complete_native_schedule(
        SimpleNamespace(time_bounds_utc=bounds), adapter=adapter, cycle_time=cycle
    )
    assert not forecast._has_complete_native_schedule(
        SimpleNamespace(time_bounds_utc=bounds[:-1]), adapter=adapter, cycle_time=cycle
    )


def test_forecast_updates_other_providers_before_raising_batch_error(monkeypatch, tmp_path) -> None:
    calls: list[str] = []

    def failing_download(cycle):
        calls.append("ecmwf")
        raise RuntimeError("ecmwf unavailable")

    def successful_download(cycle):
        calls.append("noaa")
        return Path("/downloads/noaa.grib2")

    failing = FakeNoaaAdapter(failing_download, lambda grib, cycle: _normalized(cycle))
    failing.provider_code = "ecmwf"
    successful = FakeNoaaAdapter(
        successful_download,
        lambda grib, cycle: _normalized(cycle),
    )
    adapters = {"ecmwf": failing, "noaa": successful}
    monkeypatch.setattr(forecast, "get_forecast_adapter", lambda provider: adapters[provider])
    monkeypatch.setattr(forecast, "list_forecast_assets", lambda *args, **kwargs: pd.DataFrame())
    context = _context(tmp_path)

    with pytest.raises(forecast.ForecastProviderBatchError, match="ecmwf"):
        forecast.download_forecast_data(
            context,
            providers=("ecmwf", "noaa"),
            reference_time=datetime(2026, 3, 18, 15),
        )

    assert "ecmwf" in calls
    assert "noaa" in calls


def test_forecast_update_registers_reused_and_new_assets(monkeypatch, tmp_path) -> None:
    context = _context(tmp_path)
    reused, produced = Path("/assets/reused.nc"), Path("/assets/produced.nc")
    registered: list[Path] = []
    monkeypatch.setattr(forecast, "list_enabled_forecast_providers", lambda _: ("noaa",))
    monkeypatch.setattr(
        forecast,
        "download_forecast_data",
        lambda *args, **kwargs: forecast.ForecastDownloadSummary((reused,), (produced,), ()),
    )
    monkeypatch.setattr(forecast, "ingest_forecast_asset", lambda _, path: registered.append(path))

    summary = forecast.update_enabled_forecast_providers(context)

    assert registered == [reused, produced]
    assert summary.reused_asset_paths == (reused,)
    assert summary.new_asset_paths == (produced,)



def test_missing_preflight_dependency_stops_before_any_download(monkeypatch, tmp_path) -> None:
    dependency_error = ModuleNotFoundError("No module named eccodes")
    adapter = FakeNoaaAdapter(lambda cycle: Path("/downloads/should-not-run.grib2"), lambda grib, cycle: _normalized(cycle))
    adapter.preflight_dependencies = lambda: (_ for _ in ()).throw(dependency_error)
    _install_adapter(monkeypatch, adapter)

    with pytest.raises(ModuleNotFoundError, match="No module named eccodes") as exc_info:
        forecast.download_forecast_data(
            _context(tmp_path), providers="noaa", reference_time=datetime(2026, 3, 18, 15)
        )

    assert exc_info.value is dependency_error
    assert adapter.download_cycles == []


def test_ecmwf_cycle_logs_include_context_and_full_unavailable_traceback(monkeypatch, tmp_path) -> None:
    first_grib = Path("/downloads/ecmwf.grib2")

    def download(cycle: datetime) -> Path:
        if len(adapter.download_cycles) == 1:
            raise ForecastCycleUnavailableError("remote cycle missing")
        return first_grib

    adapter = FakeNoaaAdapter(download, lambda grib, cycle: _normalized(cycle))
    adapter.provider_code = "ecmwf"
    adapter.product_config = ECMWF_FORECAST_PRODUCT
    adapter.forecast_cycle_hours = (0, 12)
    _install_adapter(monkeypatch, adapter)
    context = _context(tmp_path)

    summary = forecast.download_forecast_data(
        context, providers="ecmwf", reference_time=datetime(2026, 3, 18, 15)
    )

    assert summary.raw_grib_paths == (first_grib,)
    assert [cycle.hour for cycle in adapter.download_cycles] == [12, 0]
    first_cycle = adapter.download_cycles[0]
    log_text = (context.paths.logs_dir / "forecast_ecmwf" / f"{first_cycle:%Y%m%dT%H%M%S}.log").read_text(encoding="utf-8")
    assert "provider=ecmwf" in log_text
    assert "attempt=1" in log_text
    assert "required_end=" not in log_text
    assert "ForecastCycleUnavailableError: remote cycle missing" in log_text



def test_builtin_forecast_preflights_check_the_expected_dependencies(monkeypatch) -> None:
    missing_eccodes = ModuleNotFoundError("No module named eccodes")
    monkeypatch.setattr(forecast_noaa, "_require_requests", lambda: object())
    monkeypatch.setattr("mgb_ops.adapters._grib2.require_eccodes", lambda: (_ for _ in ()).throw(missing_eccodes))
    with pytest.raises(ModuleNotFoundError, match="No module named eccodes"):
        forecast_noaa.preflight_forecast_dependencies()

    missing_opendata = ModuleNotFoundError("No module named ecmwf")
    monkeypatch.setattr(forecast_ecmwf, "_require_opendata_client", lambda: (_ for _ in ()).throw(missing_opendata))
    with pytest.raises(ModuleNotFoundError, match="No module named ecmwf"):
        forecast_ecmwf.preflight_forecast_dependencies()
