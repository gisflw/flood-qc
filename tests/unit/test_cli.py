from __future__ import annotations

from datetime import datetime
from pathlib import Path
import subprocess
import sys
import sqlite3
from types import SimpleNamespace
from unittest.mock import Mock

import pytest

from apps import cli
from mgb_ops.assets.current_run import (
    CurrentRunArtifact,
    ForecastScenarioReference,
    create_current_artifact,
    load_current_artifact,
)
from mgb_ops.edit.forcing import ForecastCorrectionInstruction


@pytest.fixture
def context(tmp_path):
    args = cli.build_parser().parse_args(
        ["runtime", "--workspace", str(tmp_path), "--reference-time", "2026-10-11T09:00:00"]
    )
    return cli._context(args)


def test_runtime_options_before_and_after_command(tmp_path):
    parser = cli.build_parser()
    for argv in (
        ["--workspace", str(tmp_path), "runtime"],
        ["runtime", "--workspace", str(tmp_path)],
    ):
        assert parser.parse_args(argv).workspace == str(tmp_path)
    assert (
        parser.parse_args(["--workspace", "first", "runtime", "--workspace", "second"]).workspace
        == "second"
    )
    with pytest.raises(SystemExit):
        parser.parse_args(["panel", "--port", "65536"])
    with pytest.raises(SystemExit):
        parser.parse_args(["execute-current-run", "--reference-time", "now"])


def test_context_uses_runtime_env_and_explicit_dotenv(tmp_path, monkeypatch):
    dotenv = tmp_path / "custom.env"
    dotenv.write_text(f"MGB_OPS_WORKSPACE={tmp_path}\nINMET_API_KEY=test-key\n")
    monkeypatch.delenv("MGB_OPS_WORKSPACE", raising=False)
    monkeypatch.delenv("INMET_API_KEY", raising=False)
    args = cli.build_parser().parse_args(
        ["runtime", "--dotenv", str(dotenv), "--reference-time", "2026-10-11T09:00:00"]
    )
    context = cli._context(args)
    assert context.paths.workspace == tmp_path
    assert context.env.get("INMET_API_KEY") == "test-key"
    assert context.settings["run"]["reference_time"] == "2026-10-11T09:00:00"


def test_executable_selection(context):
    paths = context.paths
    assert cli.resolve_executable(context).name == "MGB_Inercial_Prev_RSH"
    assert cli.resolve_executable(
        context, artifact_settings={"executable_path": "saved-runner"}
    ) == (paths.workspace / "saved-runner")
    assert (
        cli.resolve_executable(context, "override-runner", {"executable_path": "saved-runner"})
        == paths.workspace / "override-runner"
    )
    paths.mgb_runner_dir.mkdir()
    paths.mgb_executable_path.touch()
    assert cli.resolve_executable(context) == paths.mgb_executable_path
    assert cli.resolve_executable(context, "bin/runner") == paths.workspace / "bin/runner"
    assert cli.resolve_executable(context, "/tmp/runner") == Path("/tmp/runner")


def test_run_all_shares_context_and_orders_stages(context, monkeypatch):
    calls = []
    monkeypatch.setattr(cli, "_context", lambda args: context)
    for name in (
        "download_observed",
        "download_forecast",
        "build_current_run",
        "execute_current_run",
    ):
        monkeypatch.setattr(
            cli, name, lambda ctx, *extra, name=name: calls.append((name, ctx, extra))
        )
    assert cli.main(["run-all", "--executable", "runner"]) == 0
    assert [call[0] for call in calls] == [
        "download_observed",
        "download_forecast",
        "build_current_run",
        "execute_current_run",
    ]
    assert all(call[1] is context for call in calls)
    assert calls[-1][2] == ("runner",)


def test_run_all_resolves_now_once(tmp_path, monkeypatch):
    original = cli.resolve_reference_time
    resolutions = []

    def resolve(value):
        resolutions.append(value)
        return datetime(2026, 10, 11, 9) if value == "now" else original(value)

    monkeypatch.setattr(cli, "resolve_reference_time", resolve)
    seen = []
    for name in (
        "download_observed",
        "download_forecast",
        "build_current_run",
        "execute_current_run",
    ):
        monkeypatch.setattr(
            cli, name, lambda ctx, *extra: seen.append(ctx.settings["run"]["reference_time"])
        )
    assert cli.main(["run-all", "--workspace", str(tmp_path)]) == 0
    assert resolutions == ["now"]
    assert seen == ["2026-10-11T09:00:00"] * 4


@pytest.mark.parametrize("error,status", [(RuntimeError("failed"), 1), (KeyboardInterrupt(), 130)])
def test_run_all_stops_on_failure(context, monkeypatch, error, status):
    monkeypatch.setattr(cli, "_context", lambda args: context)
    monkeypatch.setattr(cli, "download_observed", Mock(side_effect=error))
    forecast = Mock()
    build = Mock()
    run = Mock()
    monkeypatch.setattr(cli, "download_forecast", forecast)
    monkeypatch.setattr(cli, "build_current_run", build)
    monkeypatch.setattr(cli, "execute_current_run", run)
    assert cli.main(["run-all"]) == status
    forecast.assert_not_called()
    build.assert_not_called()
    run.assert_not_called()


def test_run_all_stops_before_execution_when_build_fails(context, monkeypatch):
    monkeypatch.setattr(cli, "_context", lambda args: context)
    monkeypatch.setattr(cli, "download_observed", Mock())
    monkeypatch.setattr(cli, "download_forecast", Mock())
    monkeypatch.setattr(cli, "build_current_run", Mock(side_effect=RuntimeError("build failed")))
    execute = Mock()
    monkeypatch.setattr(cli, "execute_current_run", execute)

    assert cli.main(["run-all"]) == 1
    execute.assert_not_called()


def test_observed_initializes_downloads_and_ingests(context, monkeypatch):
    import mgb_ops.assets.databases as databases
    import mgb_ops.workflows as workflows

    initialize = Mock()
    path = context.paths.downloads_dir / "observed.csv"
    download = Mock(return_value=SimpleNamespace(normalized_csv_paths=(path,)))
    ingest = Mock(return_value=SimpleNamespace(rows_total=2, rows_imported=2))
    monkeypatch.setattr(databases, "initialize_history_db", initialize)
    monkeypatch.setattr(workflows, "download_observed_data", download)
    monkeypatch.setattr(workflows, "ingest_from_csv", ingest)
    cli.download_observed(context)
    assert initialize.call_args.args[0] == context.paths.history_db
    assert download.call_args.args == (context, ("ana", "inmet"))
    assert download.call_args.kwargs["reference_time"] == datetime(2026, 10, 11, 9)
    ingest.assert_called_once_with(context, path, state="raw")


def test_forecast_delegates_to_workflow(context, monkeypatch):
    import mgb_ops.workflows as workflows

    update = Mock(return_value=SimpleNamespace(new_asset_paths=(), reused_asset_paths=()))
    monkeypatch.setattr(workflows, "update_enabled_forecast_providers", update)
    cli.download_forecast(context)
    update.assert_called_once_with(context, reference_time=datetime(2026, 10, 11, 9))


def test_build_current_run_uses_config_and_does_not_execute(context, monkeypatch):
    import mgb_ops.assets.history as history
    import mgb_ops.workflows as workflows
    import mgb_ops.workflows.scenarios as scenarios

    repository = Mock()
    repository.__enter__ = Mock(return_value=repository)
    repository.__exit__ = Mock(return_value=False)
    repository.list_enabled_providers.return_value = [{"provider_code": "ana"}]
    monkeypatch.setattr(history, "HistoryRepository", Mock(return_value=repository))
    build = Mock()
    execute = Mock()
    monkeypatch.setattr(scenarios, "build_current_artifact", build)
    monkeypatch.setattr(workflows, "execute_current_artifact", execute)
    cli.build_current_run(context)
    build.assert_called_once()
    execute.assert_not_called()
    kwargs = build.call_args.kwargs
    assert kwargs["precipitation_qc_settings"] == context.settings["precipitation_qc"]
    assert kwargs["observed_providers"] == ("ana",)
    assert kwargs["reference_time"] == "2026-10-11T09:00:00"


def test_execute_current_run_uses_saved_artifact_and_preserves_corrections(context, monkeypatch):
    import mgb_ops.workflows.scenario_orchestrator as orchestrator

    artifact_runner = context.paths.workspace / "artifact-runner"
    artifact_runner.touch()
    artifact = CurrentRunArtifact(
        reference_time="2026-03-12T00:00:00",
        forecast_end_exclusive="2026-03-16T00:00:00",
        timestep_hours=3,
        mgb_settings={
            "observed_horizon_days": 99,
            "forecast_horizon_days": 4,
            "executable_path": "artifact-runner",
        },
        spatial_settings={"bbox": [-50, -30, -45, -25], "resolution_degrees": 0.2},
        interpolation_settings={"nearest_stations": 8, "power": 3.0},
        review_window={"start": "2026-03-11T00:00:00", "end": "2026-03-12T00:00:00"},
        observed_providers=("ana",),
        scenarios=(
            ForecastScenarioReference(
                "corrected:asset",
                0,
                "corrected",
                "Saved correction",
                "ecmwf",
                "asset",
                "data/assets/asset.nc",
                (ForecastCorrectionInstruction("asset", 0, 3, multiplication_factor=2),),
            ),
        ),
    )
    create_current_artifact(context.paths.current_run_db, artifact)
    seen = {}

    def execute_saved(context_arg, scenarios, **kwargs):
        seen["settings"] = context_arg.settings
        seen["scenarios"] = scenarios
        seen.update(kwargs)
        return SimpleNamespace(batch_id="batch", cache_dir="cache", results=())

    monkeypatch.setattr(orchestrator, "execute_forecast_scenarios", execute_saved)
    monkeypatch.setattr(
        cli, "build_current_run", Mock(side_effect=AssertionError("must not rebuild"))
    )

    cli.execute_current_run(context)

    assert seen["settings"]["run"] == {
        "reference_time": "2026-03-12T00:00:00",
        "timestep_hours": 3,
    }
    assert seen["settings"]["mgb"] == artifact.mgb_settings
    assert seen["settings"]["spatial_grid"] == artifact.spatial_settings
    assert seen["settings"]["rainfall_interpolation"] == artifact.interpolation_settings
    assert seen["observed_provider_codes"] == ("ana",)
    assert seen["executable_path"] == artifact_runner
    assert seen["scenarios"][0].corrections[0].multiplication_factor == 2
    persisted = load_current_artifact(context.paths.current_run_db)
    assert persisted.scenarios[0].corrections[0].multiplication_factor == 2


def test_execute_context_does_not_resolve_or_align_yaml_time(tmp_path):
    custom = tmp_path / "config" / "custom.yaml"
    custom.parent.mkdir()
    custom.write_text(
        "run:\n  reference_time: '2026-10-11T09:00:00'\n  timestep_hours: 8\n",
        encoding="utf-8",
    )
    args = cli.build_parser().parse_args(["execute-current-run", "--workspace", str(tmp_path)])
    context = cli._context(args)
    assert context.settings["run"]["timestep_hours"] == 8
    assert not hasattr(args, "reference_time")


def test_execute_missing_artifact_fails_without_building(context, monkeypatch):
    build = Mock(side_effect=AssertionError("must not rebuild"))
    monkeypatch.setattr(cli, "build_current_run", build)

    with pytest.raises(FileNotFoundError, match="Current artifact does not exist"):
        cli.execute_current_run(context)

    build.assert_not_called()


def test_execute_invalid_artifact_fails_without_building(context, monkeypatch):
    context.paths.current_run_db.parent.mkdir(parents=True)
    context.paths.current_run_db.write_text("not a sqlite artifact", encoding="utf-8")
    build = Mock(side_effect=AssertionError("must not rebuild"))
    monkeypatch.setattr(cli, "build_current_run", build)

    with pytest.raises(sqlite3.DatabaseError):
        cli.execute_current_run(context)

    build.assert_not_called()


def test_panel_uses_factory_for_each_session(context, monkeypatch):
    import panel
    import apps.ops_dashboard as dashboard

    serve = Mock()
    create = Mock(side_effect=[object(), object()])
    monkeypatch.setattr(panel, "serve", serve)
    monkeypatch.setattr(dashboard, "create_dashboard", create)
    args = cli.build_parser().parse_args(["panel", "--no-show", "--port", "5010"])
    cli.serve_panel(context, args)
    factory = serve.call_args.args[0]
    assert factory() is not factory()
    create.assert_called_with(context.paths.workspace, context=context)
    assert serve.call_args.kwargs["show"] is False
    assert serve.call_args.kwargs["port"] == 5010


def test_missing_dependency_message(context, monkeypatch, capsys):
    monkeypatch.setattr(cli, "_context", lambda args: context)
    monkeypatch.setattr(cli, "serve_panel", Mock(side_effect=ModuleNotFoundError(name="panel")))
    assert cli.main(["panel"]) == 1
    assert "mgb-ops[dashboard]" in capsys.readouterr().err


def test_help_does_not_import_optional_dependencies():
    source = Path(cli.__file__).resolve().parents[2]
    code = """
import sys
sys.path.insert(0, sys.argv[1])
sys.path.insert(0, sys.argv[1] + '/src')
class BlockOptional:
    def find_spec(self, fullname, *args):
        if fullname in {'panel', 'pandas', 'numpy', 'xarray', 'geopandas', 'apps.ops_dashboard'}:
            raise AssertionError('optional import: ' + fullname)
sys.meta_path.insert(0, BlockOptional())
from apps.cli import main
main(['--help'])
"""
    result = subprocess.run(
        [sys.executable, "-c", code, str(source)], capture_output=True, text=True
    )
    assert result.returncode == 0, result.stderr
    assert "build-current-run" in result.stdout
    assert "execute-current-run" in result.stdout
