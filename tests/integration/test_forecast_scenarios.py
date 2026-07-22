from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from copy import deepcopy
from datetime import datetime, timezone
from pathlib import Path
from threading import Barrier

import numpy as np
import pytest

from mgb_ops.assets.databases import initialize_history_db
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.assets.model_outputs import write_model_outputs_netcdf
from mgb_ops.assets.scenario_cache import (
    discover_latest_scenario_caches,
    scenario_cache_root,
)
from mgb_ops.assets.schemas import SQL_DIR
from mgb_ops.assets.spatial_grid import write_spatial_grid
from mgb_ops.config.env import RuntimeEnv
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.config.settings import DEFAULT_SETTINGS
from mgb_ops.config.workspace import RuntimePaths
from mgb_ops.workflows.forecast import list_enabled_forecast_providers
from mgb_ops.workflows.scenario_orchestrator import (
    ScenarioBatchError,
    ScenarioRunResult,
    execute_forecast_scenarios,
)
from mgb_ops.workflows.scenarios import ForecastScenario, derive_forecast_scenarios


def _history(tmp_path: Path) -> Path:
    inventory = Path(__file__).parents[1] / "fixtures" / "history_station_inventory.csv"
    return initialize_history_db(
        tmp_path / "data" / "history.sqlite",
        inventory,
        SQL_DIR / "history_schema.sql",
    )


def _forecast_asset(tmp_path: Path, provider: str = "ecmwf") -> tuple[Path, str]:
    asset_id = f"{provider}.test.20260312T000000Z"
    path = tmp_path / "data" / "assets" / f"{provider}.nc"
    bounds = [
        (
            datetime(2026, 3, 12, 0, tzinfo=timezone.utc),
            datetime(2026, 3, 12, 3, tzinfo=timezone.utc),
        ),
        (
            datetime(2026, 3, 12, 3, tzinfo=timezone.utc),
            datetime(2026, 3, 12, 6, tzinfo=timezone.utc),
        ),
    ]
    write_spatial_grid(
        path,
        variable="precipitation",
        grid_type="forecast",
        source="cropped_from_native_grid",
        providers=[provider],
        units="mm",
        bbox=(-52.0, -31.0, -51.0, -30.0),
        resolution_degrees=0.5,
        times_utc=[right for _, right in bounds],
        time_bounds_utc=bounds,
        latitudes=np.array([-30.75, -30.25]),
        longitudes=np.array([-51.75, -51.25]),
        values=np.ones((2, 2, 2)),
        timestep_hours=None,
    )
    return path, asset_id


def test_enabled_provider_registry_and_scenario_derivation(tmp_path: Path) -> None:
    database = _history(tmp_path)
    path, asset_id = _forecast_asset(tmp_path)
    with HistoryRepository(database) as repository:
        repository.connection.execute(
            "UPDATE provider SET is_active = 0 WHERE provider_code = 'noaa'"
        )
        repository.connection.commit()
        repository.upsert_asset(
            asset_id=asset_id,
            asset_kind="spatial_grid",
            format="NetCDF",
            relative_path=path.relative_to(tmp_path).as_posix(),
            provider_code="ecmwf",
            valid_from="2026-03-12T00:00:00Z",
            valid_to="2026-03-12T06:00:00Z",
            metadata={
                "type": "forecast",
                "cycle_time": "2026-03-12T00:00:00Z",
            },
        )
        repository.replace_forecast_manual_edits(
            asset_id,
            [
                {
                    "t0_step": 0,
                    "t1_step": 3,
                    "multiplication_factor": 2,
                    "reason": "wet adjustment",
                }
            ],
        )

    assert list_enabled_forecast_providers(database) == ("ecmwf",)
    scenarios = derive_forecast_scenarios(
        database,
        tmp_path,
        target_cycle=datetime(2026, 3, 12),
        required_start=datetime(2026, 3, 12),
        required_end=datetime(2026, 3, 12, 6),
        lookback_cycles=1,
    )
    assert [item.kind for item in scenarios] == ["zero", "raw", "corrected"]
    assert scenarios[1].asset_id == asset_id
    assert scenarios[2].correction is not None
    assert scenarios[2].correction.multiplication_factor == 2


def _write_scenario_output(
    path: Path, scenario: ForecastScenario, *, forecast_grid_relative_path: str | None = None
) -> None:
    attrs: dict[str, str | int] = {
        "window_start": "2026-03-11T00:00:00",
        "reference_time": "2026-03-12T00:00:00",
        "window_end_exclusive": "2026-03-13T00:00:00",
        "scenario_id": scenario.scenario_id,
        "scenario_label": scenario.label,
        "scenario_kind": scenario.kind,
    }
    if scenario.provider_code:
        attrs["provider_code"] = scenario.provider_code
    if scenario.asset_id:
        attrs["source_forecast_asset_id"] = scenario.asset_id
    if scenario.correction_id is not None:
        attrs["correction_id"] = scenario.correction_id
    if forecast_grid_relative_path is not None:
        attrs["forecast_grid_relative_path"] = forecast_grid_relative_path
    write_model_outputs_netcdf(
        path=path,
        variables={"flow": np.array([[1.0], [2.0]])},
        variable_attrs={"flow": {"units": "m3/s"}},
        time_values=np.array(
            ["2026-03-12T00:00:00", "2026-03-12T01:00:00"],
            dtype="datetime64[ns]",
        ),
        time_segment=np.array([0, 1], dtype=np.int8),
        mini_ids=[1],
        global_attrs=attrs,
    )


def test_scenario_cache_discovery_reads_direct_current_files(tmp_path: Path) -> None:
    root = scenario_cache_root(tmp_path)
    root.mkdir(parents=True)
    scenario = ForecastScenario(
        "raw:ecmwf.asset", "ECMWF raw", "raw", "ecmwf", "ecmwf.asset"
    )
    output = root / "raw.nc"
    forecast_grid = root / "grids" / "raw.nc"
    forecast_grid.parent.mkdir()
    forecast_grid.touch()
    _write_scenario_output(
        output, scenario, forecast_grid_relative_path="grids/raw.nc"
    )

    caches = discover_latest_scenario_caches(tmp_path)

    assert len(caches) == 1
    assert caches[0].scenario_id == scenario.scenario_id
    assert caches[0].path == output
    assert caches[0].forecast_grid_path == forecast_grid


def _context(tmp_path: Path) -> RuntimeContext:
    settings = deepcopy(DEFAULT_SETTINGS)
    settings["run"]["reference_time"] = "2026-03-12T00:00:00"
    paths = RuntimePaths(tmp_path)
    paths.ensure_standard_dirs()
    executable = paths.mgb_executable_path
    executable.parent.mkdir(parents=True, exist_ok=True)
    executable.write_text("runner", encoding="utf-8")
    return RuntimeContext(paths, settings, RuntimeEnv({}))


def _thread_executor(*, max_workers: int) -> ThreadPoolExecutor:
    return ThreadPoolExecutor(max_workers=max_workers)


def test_orchestrator_uses_spawned_process_workers() -> None:
    from mgb_ops.workflows.scenario_orchestrator import _scenario_executor

    executor = _scenario_executor(max_workers=1)
    try:
        assert executor._mp_context.get_start_method() == "spawn"
    finally:
        executor.shutdown()


def test_orchestrator_runs_concurrently_and_publishes_complete_batch(
    tmp_path: Path, monkeypatch
) -> None:
    context = _context(tmp_path)
    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._scenario_executor",
        _thread_executor,
    )
    scenarios = (
        ForecastScenario("zero", "Zero", "zero"),
        ForecastScenario("raw:asset", "Raw", "raw", "ecmwf", "asset", tmp_path / "asset.nc"),
    )
    barrier = Barrier(2)

    def fake_execute(context, scenario, *, batch_id, staging_dir, **kwargs):
        barrier.wait(timeout=2)
        path = staging_dir / f"{scenario.kind}.nc"
        path.write_bytes(scenario.scenario_id.encode())
        return ScenarioRunResult(scenario, path, f"{batch_id}-{scenario.kind}")

    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._execute_scenario",
        fake_execute,
    )
    result = execute_forecast_scenarios(
        context,
        scenarios,
        observed_provider_codes=("ana",),
        reference_time=datetime(2026, 3, 12),
    )

    assert [item.scenario.scenario_id for item in result.results] == [
        "zero",
        "raw:asset",
    ]
    assert all(item.cache_path.is_file() for item in result.results)
    assert result.cache_dir == scenario_cache_root(context.paths.cache_dir)
    assert sorted(path.name for path in result.cache_dir.iterdir()) == ["raw.nc", "zero.nc"]


def test_orchestrator_replaces_current_cache_after_complete_second_run(
    tmp_path: Path, monkeypatch
) -> None:
    context = _context(tmp_path)
    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._scenario_executor",
        _thread_executor,
    )
    scenario = ForecastScenario("zero", "Zero", "zero")
    writes = iter((b"first", b"second"))

    def fake_execute(context, scenario, *, batch_id, staging_dir, **kwargs):
        path = staging_dir / "zero.nc"
        path.write_bytes(next(writes))
        return ScenarioRunResult(scenario, path, f"{batch_id}-zero")

    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._execute_scenario",
        fake_execute,
    )
    first = execute_forecast_scenarios(
        context,
        (scenario,),
        observed_provider_codes=("ana",),
        reference_time=datetime(2026, 3, 12),
    )
    assert (first.cache_dir / "zero.nc").read_bytes() == b"first"

    second = execute_forecast_scenarios(
        context,
        (scenario,),
        observed_provider_codes=("ana",),
        reference_time=datetime(2026, 3, 13),
    )
    assert second.cache_dir == first.cache_dir
    assert (second.cache_dir / "zero.nc").read_bytes() == b"second"
    assert [path.name for path in second.cache_dir.iterdir()] == ["zero.nc"]


def test_orchestrator_removes_orphaned_staging_directories(tmp_path: Path, monkeypatch) -> None:
    context = _context(tmp_path)
    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._scenario_executor",
        _thread_executor,
    )
    root = scenario_cache_root(context.paths.cache_dir)
    orphan = root.parent / ".forecast_scenarios.staging-crashed"
    orphan.mkdir(parents=True)
    (orphan / "partial.nc").write_bytes(b"partial")
    published = root
    published.mkdir()
    (published / "current.nc").write_bytes(b"current")

    scenario = ForecastScenario("zero", "Zero", "zero")

    def fake_execute(context, scenario, *, batch_id, staging_dir, **kwargs):
        path = staging_dir / "zero.nc"
        path.write_bytes(b"complete")
        return ScenarioRunResult(scenario, path, f"{batch_id}-zero")

    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._execute_scenario",
        fake_execute,
    )
    execute_forecast_scenarios(
        context,
        (scenario,),
        observed_provider_codes=("ana",),
        reference_time=datetime(2026, 3, 12),
    )

    assert not orphan.exists()
    assert published.is_dir()


def test_orchestrator_failure_keeps_previous_batch(
    tmp_path: Path, monkeypatch
) -> None:
    context = _context(tmp_path)
    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._scenario_executor",
        _thread_executor,
    )
    root = scenario_cache_root(context.paths.cache_dir)
    root.mkdir(parents=True)
    old = root / "old.nc"
    old.write_bytes(b"published")
    scenarios = (
        ForecastScenario("zero", "Zero", "zero"),
        ForecastScenario("raw:asset", "Raw", "raw"),
    )

    def fake_execute(context, scenario, *, batch_id, staging_dir, **kwargs):
        if scenario.kind == "raw":
            raise RuntimeError("model failed")
        path = staging_dir / "zero.nc"
        path.write_bytes(b"partial")
        return ScenarioRunResult(scenario, path, f"{batch_id}-zero")

    monkeypatch.setattr(
        "mgb_ops.workflows.scenario_orchestrator._execute_scenario",
        fake_execute,
    )
    with pytest.raises(ScenarioBatchError, match="no caches were published"):
        execute_forecast_scenarios(
            context,
            scenarios,
            observed_provider_codes=("ana",),
            reference_time=datetime(2026, 3, 12),
        )

    assert root.is_dir()
    assert old.read_bytes() == b"published"
