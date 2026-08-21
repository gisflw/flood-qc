from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor
from copy import deepcopy
from datetime import datetime, timedelta, timezone
from pathlib import Path
from threading import Barrier

import numpy as np
import pandas as pd
import pytest

from mgb_ops.assets.current_run import CurrentRunArtifact, ForecastScenarioReference, create_current_artifact
from mgb_ops.assets.model_outputs import write_model_outputs_netcdf
from mgb_ops.assets.scenario_cache import discover_latest_scenario_caches, scenario_cache_root
from mgb_ops.config.env import RuntimeEnv
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.config.settings import DEFAULT_SETTINGS
from mgb_ops.config.workspace import RuntimePaths
from mgb_ops.edit.forcing import ForecastCorrectionInstruction
from mgb_ops.workflows.scenario_orchestrator import ScenarioBatchError, ScenarioRunResult, execute_forecast_scenarios
from mgb_ops.workflows import scenarios as scenario_workflow
from mgb_ops.workflows.scenarios import ForecastScenario, derive_forecast_scenarios


def test_scenarios_are_read_from_the_current_artifact_snapshot(tmp_path: Path) -> None:
    database = tmp_path / "data" / "current_run.sqlite"
    create_current_artifact(database, CurrentRunArtifact(
        "2026-03-12T00:00:00", "2026-03-26T00:00:00", 1,
        {}, {}, {}, {}, ("ana",), responsible_person="operator", reason="forecast review",
        scenarios=(
            ForecastScenarioReference("zero", 0, "zero", "Zero-rain horizon"),
            ForecastScenarioReference("raw:asset", 1, "raw", "ECMWF raw", "ecmwf", "asset", "data/assets/asset.nc"),
            ForecastScenarioReference("corrected:asset", 2, "corrected", "ECMWF corrected", "ecmwf", "asset", "data/assets/asset.nc", (
                ForecastCorrectionInstruction("asset", 0, 3, multiplication_factor=2),
            )),
        ),
    ))
    scenarios = derive_forecast_scenarios(database)
    assert [scenario.kind for scenario in scenarios] == ["zero", "raw", "corrected"]
    assert scenarios[-1].asset_id == "asset"
    assert scenarios[-1].corrections[0].multiplication_factor == 2
def _write_scenario_output(
    path: Path,
    scenario: ForecastScenario,
    *,
    forecast_grid_relative_path: str | None = None,
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
    _write_scenario_output(output, scenario, forecast_grid_relative_path="grids/raw.nc")

    caches = discover_latest_scenario_caches(tmp_path)

    assert len(caches) == 1
    assert caches[0].scenario_id == scenario.scenario_id
    assert caches[0].path == output
    assert caches[0].forecast_grid_path == forecast_grid
    assert caches[0].reference_time == datetime(2026, 3, 12)


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


def test_artifact_observed_cache_uses_full_model_horizon(tmp_path: Path, monkeypatch) -> None:
    context = _context(tmp_path)
    context.settings["mgb"]["observed_horizon_days"] = 30
    monkeypatch.setattr("mgb_ops.workflows.scenario_orchestrator._scenario_executor", _thread_executor)
    captured = {}

    def fake_cache(database_path, cache_dir, **kwargs):
        captured.update(kwargs)
        cache_dir.mkdir(parents=True, exist_ok=True)
        path = cache_dir / kwargs["filename"]
        path.touch()
        return path

    def fake_execute(context, scenario, *, batch_id, staging_dir, **kwargs):
        path = staging_dir / "zero.nc"
        path.write_bytes(b"complete")
        return ScenarioRunResult(scenario, path, f"{batch_id}-zero")

    monkeypatch.setattr("mgb_ops.workflows.scenario_orchestrator.build_observed_precipitation_cache", fake_cache)
    monkeypatch.setattr("mgb_ops.workflows.scenario_orchestrator._execute_scenario", fake_execute)
    artifact = CurrentRunArtifact(
        "2026-03-12T00:00:00", "2026-03-18T00:00:00", 1,
        {"observed_horizon_days": 30, "forecast_horizon_days": 5, "use_forecast_data": True},
        {"bbox": [-53, -31, -50, -29], "resolution_degrees": 0.1},
        {"nearest_stations": 5, "power": 2.0},
        {"start": "2026-03-01T00:00:00", "end": "2026-03-12T00:00:00"},
        ("ana",),
    )
    execute_forecast_scenarios(
        context, (ForecastScenario("zero", "Zero", "zero"),),
        observed_provider_codes=("ana",), reference_time=datetime(2026, 3, 12), artifact=artifact,
    )

    assert (captured["end_time_utc"] - captured["start_time_utc"]).total_seconds() == 721 * 3600



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
        ForecastScenario(
            "raw:asset", "Raw", "raw", "ecmwf", "asset", tmp_path / "asset.nc"
        ),
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
    assert sorted(path.name for path in result.cache_dir.iterdir()) == [
        "raw.nc",
        "zero.nc",
    ]


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


def test_orchestrator_removes_orphaned_staging_directories(
    tmp_path: Path, monkeypatch
) -> None:
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


def test_orchestrator_failure_keeps_previous_batch(tmp_path: Path, monkeypatch) -> None:
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


def test_build_current_artifact_resolves_the_mgb_coverage_window(
    tmp_path: Path, monkeypatch
) -> None:
    captured: dict[str, datetime] = {}

    def fake_resolve(database_path, workspace_path, *, required_start, required_end):
        captured["start"] = required_start
        captured["end"] = required_end
        return (ForecastScenarioReference("zero", 0, "zero", "Zero-rain horizon"),)

    monkeypatch.setattr(scenario_workflow, "resolve_forecast_scenario_references", fake_resolve)
    monkeypatch.setattr(scenario_workflow, "create_current_artifact", lambda path, artifact: artifact)
    existing_database = tmp_path / "current_run.sqlite"
    existing_database.touch()

    artifact = scenario_workflow.build_current_artifact(
        existing_database,
        tmp_path / "history.sqlite",
        tmp_path,
        reference_time="2026-03-18T15:00:00",
        forecast_end_exclusive="2026-03-24T00:00:00",
        timestep_hours=1,
        mgb_settings={"forecast_horizon_days": 5},
        spatial_settings={},
        interpolation_settings={},
        review_window={},
        observed_providers=("ana",),
    )

    assert captured == {
        "start": datetime(2026, 3, 18, 19, tzinfo=timezone.utc),
        "end": datetime(2026, 3, 23, 19, tzinfo=timezone.utc),
    }
    assert artifact.forecast_end_exclusive == "2026-03-24T00:00:00"


@pytest.mark.parametrize(
    ("valid_from", "valid_to"),
    (
        ("2026-03-18T19:00:01+00:00", "2026-03-23T19:00:00+00:00"),
        ("2026-03-18T19:00:00+00:00", "2026-03-23T18:59:59+00:00"),
    ),
)
def test_forecast_scenario_resolution_rejects_incomplete_boundaries(
    tmp_path: Path, monkeypatch, valid_from: str, valid_to: str
) -> None:
    asset_path = tmp_path / "forecast.nc"
    asset_path.touch()
    assets = pd.DataFrame(
        [{
            "asset_id": "ecmwf.asset",
            "provider_code": "ecmwf",
            "cycle_time": "2026-03-18T12:00:00Z",
            "valid_from": valid_from,
            "valid_to": valid_to,
            "asset_path": asset_path,
        }]
    )
    monkeypatch.setattr(scenario_workflow, "list_enabled_forecast_providers", lambda path: ("ecmwf",))
    monkeypatch.setattr(scenario_workflow, "get_forecast_adapter", lambda provider: object())
    monkeypatch.setattr(scenario_workflow, "list_forecast_assets", lambda *args, **kwargs: assets)

    with pytest.raises(RuntimeError, match="ecmwf"):
        scenario_workflow.resolve_forecast_scenario_references(
            tmp_path / "history.sqlite",
            tmp_path,
            required_start=datetime(2026, 3, 18, 19, tzinfo=timezone.utc),
            required_end=datetime(2026, 3, 23, 19, tzinfo=timezone.utc),
        )


def test_forecast_scenario_resolution_accepts_exact_coverage(
    tmp_path: Path, monkeypatch
) -> None:
    asset_path = tmp_path / "forecast.nc"
    asset_path.touch()
    assets = pd.DataFrame(
        [{
            "asset_id": "ecmwf.asset",
            "provider_code": "ecmwf",
            "cycle_time": "2026-03-18T12:00:00Z",
            "valid_from": "2026-03-18T19:00:00+00:00",
            "valid_to": "2026-03-23T19:00:00+00:00",
            "asset_path": asset_path,
        }]
    )
    monkeypatch.setattr(scenario_workflow, "list_enabled_forecast_providers", lambda path: ("ecmwf",))
    monkeypatch.setattr(scenario_workflow, "get_forecast_adapter", lambda provider: object())
    monkeypatch.setattr(scenario_workflow, "list_forecast_assets", lambda *args, **kwargs: assets)

    references = scenario_workflow.resolve_forecast_scenario_references(
        tmp_path / "history.sqlite",
        tmp_path,
        required_start=datetime(2026, 3, 18, 19, tzinfo=timezone.utc),
        required_end=datetime(2026, 3, 23, 19, tzinfo=timezone.utc),
    )

    assert [reference.scenario_id for reference in references] == [
        "zero",
        "raw:ecmwf.asset",
    ]
