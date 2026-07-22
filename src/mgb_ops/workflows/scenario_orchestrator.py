from __future__ import annotations

from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime
import hashlib
import json
from multiprocessing import get_context
from pathlib import Path
from typing import Mapping
import shutil
from uuid import uuid4

from mgb_ops.assets.scenario_cache import (
    LATEST_SCENARIO_CACHE_INDEX,
    scenario_cache_root,
)
from mgb_ops.assets.types import RunMetadata
from mgb_ops.config.runtime import RuntimeContext
from mgb_ops.model.export_mgb_outputs import export_mgb_outputs
from mgb_ops.model.mgb_execution import execute_mgb_plan, prepare_mgb_execution
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta
from mgb_ops.model.prepare_mgb_rainfall import (
    MGB_OBSERVED_CACHE_FILENAME,
    prepare_mgb_rainfall,
)
from mgb_ops.utils.logging import configure_run_logger
from mgb_ops.workflows.scenarios import ForecastScenario


@dataclass(frozen=True, slots=True)
class ScenarioRunResult:
    scenario: ForecastScenario
    cache_path: Path
    run_id: str


@dataclass(frozen=True, slots=True)
class ScenarioBatchResult:
    batch_id: str
    results: tuple[ScenarioRunResult, ...]
    index_path: Path


class ScenarioBatchError(RuntimeError):
    def __init__(self, failures: dict[str, BaseException]) -> None:
        self.failures = dict(failures)
        details = "; ".join(
            f"{scenario_id}: {type(error).__name__}: {error}"
            for scenario_id, error in sorted(failures.items())
        )
        super().__init__(f"Scenario batch failed; no caches were published. {details}")


def _scenario_executor(max_workers: int) -> ProcessPoolExecutor:
    return ProcessPoolExecutor(
        max_workers=max_workers,
        mp_context=get_context("spawn"),
    )


def _cleanup_orphaned_staging(root: Path) -> None:
    for candidate in root.iterdir():
        if candidate.is_dir() and candidate.name.startswith(".staging-"):
            shutil.rmtree(candidate)


def _safe_name(scenario: ForecastScenario) -> str:
    digest = hashlib.sha256(scenario.scenario_id.encode("utf-8")).hexdigest()[:12]
    return f"{scenario.kind}-{digest}"


def _scenario_metadata(scenario: ForecastScenario) -> dict[str, str | int]:
    metadata: dict[str, str | int] = {
        "scenario_id": scenario.scenario_id,
        "scenario_label": scenario.label,
        "scenario_kind": scenario.kind,
    }
    if scenario.provider_code:
        metadata["provider_code"] = scenario.provider_code
    if scenario.asset_id:
        metadata["source_forecast_asset_id"] = scenario.asset_id
    if scenario.correction_id is not None:
        metadata["correction_id"] = scenario.correction_id
    return metadata


def _execute_scenario(
    context: RuntimeContext,
    scenario: ForecastScenario,
    *,
    batch_id: str,
    staging_dir: Path,
    executable_path: Path,
    observed_provider_codes: tuple[str, ...],
    execution_env: Mapping[str, str],
) -> ScenarioRunResult:
    settings = context.settings
    paths = context.paths
    safe_name = _safe_name(scenario)
    run_id = f"{batch_id}-{safe_name}"
    work_dir = staging_dir / "work" / safe_name
    input_dir = work_dir / "Input"
    output_dir = work_dir / "Output"
    shutil.copytree(paths.mgb_input_dir, input_dir)
    output_dir.mkdir(parents=True)
    parhig_path = input_dir / "PARHIG.hig"
    mini_gtp_path = input_dir / "MINI.gtp"
    chuvabin_path = input_dir / "CHUVABIN.hig"
    scenario_logs = paths.logs_dir / "forecast_scenarios" / batch_id / safe_name
    logger = configure_run_logger(
        f"forecast_scenario.{run_id}",
        scenario_logs / "scenario.log",
        console=False,
    )

    mgb_settings = settings["mgb"]
    timestep_hours = int(settings["run"]["timestep_hours"])
    reference_time = datetime.fromisoformat(str(settings["run"]["reference_time"]))
    rewrite_mgb_meta(
        parhig_path=parhig_path,
        reference_time=reference_time,
        input_days_before=int(mgb_settings["input_days_before"]),
        forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
        timestep_hours=timestep_hours,
        logger=logger,
    )
    rainfall_settings = settings["rainfall_interpolation"]
    spatial_settings = settings["spatial_grid"]
    bbox = spatial_settings["bbox"]
    if bbox is None:
        raise ValueError("spatial_grid.bbox is required for scenario execution.")
    prepare_mgb_rainfall(
        history_db=paths.history_db,
        parhig_path=parhig_path,
        mini_gtp_path=mini_gtp_path,
        output_path=chuvabin_path,
        reference_time=reference_time,
        input_days_before=int(mgb_settings["input_days_before"]),
        forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
        use_forecast_data=scenario.kind != "zero",
        forecast_asset_path=scenario.asset_path,
        forecast_correction=scenario.correction,
        cache_dir=work_dir / "cache",
        spatial_bbox=tuple(float(value) for value in bbox),
        spatial_resolution_degrees=float(spatial_settings["resolution_degrees"]),
        observed_providers=observed_provider_codes,
        nearest_stations=int(rainfall_settings["nearest_stations"]),
        power=float(rainfall_settings["power"]),
        timestep_hours=timestep_hours,
        logger=logger,
    )
    run = RunMetadata(run_id=run_id, reference_time=reference_time.isoformat(timespec="seconds"))
    plan = prepare_mgb_execution(
        run,
        executable_path=str(executable_path),
        input_dir=input_dir,
        output_dir=output_dir,
        workspace_root=work_dir,
        asset_base_dir=paths.workspace,
    )
    execute_mgb_plan(
        plan,
        logs_dir=scenario_logs,
        env=execution_env,
    )
    cache_path = staging_dir / f"{safe_name}.nc"
    export_mgb_outputs(
        reference_time=reference_time,
        output_days_before=int(mgb_settings["output_days_before"]),
        forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
        parhig_path=parhig_path,
        mini_gtp_path=mini_gtp_path,
        chuvabin_path=chuvabin_path,
        output_dir=output_dir,
        output_nc_path=cache_path,
        logger=logger,
        scenario_metadata=_scenario_metadata(scenario),
    )
    logger.info("scenario_complete scenario_id=%s cache=%s", scenario.scenario_id, cache_path)
    return ScenarioRunResult(scenario=scenario, cache_path=cache_path, run_id=run_id)


def _publish_batch(
    root: Path,
    staging_dir: Path,
    batch_id: str,
    results: tuple[ScenarioRunResult, ...],
) -> Path:
    work_dir = staging_dir / "work"
    if results:
        observed_source = (
            work_dir
            / _safe_name(results[0].scenario)
            / "cache"
            / MGB_OBSERVED_CACHE_FILENAME
        )
        if observed_source.is_file():
            observed_target = root.parent / MGB_OBSERVED_CACHE_FILENAME
            observed_temp = observed_target.with_name(
                f".{observed_target.name}.{uuid4().hex}.tmp"
            )
            shutil.copy2(observed_source, observed_temp)
            observed_temp.replace(observed_target)
    if work_dir.exists():
        shutil.rmtree(work_dir)
    batch_dir = root / batch_id
    staging_dir.replace(batch_dir)
    files = [result.cache_path.name for result in results]
    payload = {"batch": batch_id, "files": files}
    index_path = root / LATEST_SCENARIO_CACHE_INDEX
    previous_batch: str | None = None
    if index_path.is_file():
        try:
            previous_batch = str(json.loads(index_path.read_text(encoding="utf-8"))["batch"])
        except (KeyError, TypeError, json.JSONDecodeError):
            previous_batch = None
    temp_index = root / f".{LATEST_SCENARIO_CACHE_INDEX}.{uuid4().hex}.tmp"
    temp_index.write_text(json.dumps(payload, indent=2, sort_keys=True), encoding="utf-8")
    temp_index.replace(index_path)
    if previous_batch and previous_batch != batch_id:
        old_dir = root / previous_batch
        if old_dir.parent == root and old_dir.is_dir():
            shutil.rmtree(old_dir)
    return index_path


def execute_forecast_scenarios(
    context: RuntimeContext,
    scenarios: tuple[ForecastScenario, ...] | list[ForecastScenario],
    *,
    executable_path: Path | None = None,
    observed_provider_codes: tuple[str, ...] = (),
    reference_time: datetime | None = None,
    execution_env: Mapping[str, str] | None = None,
) -> ScenarioBatchResult:
    """Execute pre-derived scenarios concurrently and atomically publish their caches."""
    ordered = tuple(scenarios)
    if not ordered:
        raise ValueError("At least one forecast scenario is required.")
    if len({scenario.scenario_id for scenario in ordered}) != len(ordered):
        raise ValueError("Forecast scenario IDs must be unique.")
    if not observed_provider_codes:
        raise ValueError("observed_provider_codes must contain at least one provider.")

    resolved_reference = reference_time
    if resolved_reference is None:
        raw_reference = str(context.settings["run"]["reference_time"])
        try:
            resolved_reference = datetime.fromisoformat(raw_reference)
        except ValueError as exc:
            raise ValueError(
                "execute_forecast_scenarios requires reference_time when run.reference_time is symbolic."
            ) from exc
    run_settings = dict(context.settings["run"])
    run_settings["reference_time"] = resolved_reference.isoformat(timespec="seconds")
    settings = dict(context.settings)
    settings["run"] = run_settings
    scenario_context = RuntimeContext(context.paths, settings, context.env)

    executable = Path(executable_path or context.paths.mgb_executable_path)
    if not executable.is_file():
        raise FileNotFoundError(f"MGB executable not found: {executable}")
    root = scenario_cache_root(context.paths.cache_dir)
    root.mkdir(parents=True, exist_ok=True)
    _cleanup_orphaned_staging(root)
    batch_id = resolved_reference.strftime("%Y%m%dT%H%M%S") + "-" + uuid4().hex[:8]
    staging_dir = root / f".staging-{batch_id}"
    staging_dir.mkdir()

    results_by_id: dict[str, ScenarioRunResult] = {}
    failures: dict[str, BaseException] = {}
    with _scenario_executor(max_workers=len(ordered)) as executor:
        futures = {
            executor.submit(
                _execute_scenario,
                scenario_context,
                scenario,
                batch_id=batch_id,
                staging_dir=staging_dir,
                executable_path=executable,
                observed_provider_codes=tuple(observed_provider_codes),
                execution_env=dict(execution_env or {}),
            ): scenario
            for scenario in ordered
        }
        for future in as_completed(futures):
            scenario = futures[future]
            try:
                results_by_id[scenario.scenario_id] = future.result()
            except BaseException as exc:
                failures[scenario.scenario_id] = exc

    if failures:
        if staging_dir.exists():
            shutil.rmtree(staging_dir)
        raise ScenarioBatchError(failures)

    results = tuple(results_by_id[scenario.scenario_id] for scenario in ordered)
    index_path = _publish_batch(root, staging_dir, batch_id, results)
    published = tuple(
        ScenarioRunResult(
            scenario=result.scenario,
            cache_path=root / batch_id / result.cache_path.name,
            run_id=result.run_id,
        )
        for result in results
    )
    return ScenarioBatchResult(batch_id=batch_id, results=published, index_path=index_path)
