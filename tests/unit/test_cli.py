from __future__ import annotations

from datetime import datetime
from pathlib import Path
import subprocess
import sys
from types import SimpleNamespace
from unittest.mock import Mock

import pytest

from apps import cli


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
    paths.mgb_runner_dir.mkdir()
    paths.mgb_executable_path.touch()
    assert cli.resolve_executable(context) == paths.mgb_executable_path
    assert cli.resolve_executable(context, "bin/runner") == paths.workspace / "bin/runner"
    assert cli.resolve_executable(context, "/tmp/runner") == Path("/tmp/runner")


def test_run_all_shares_context_and_orders_stages(context, monkeypatch):
    calls = []
    monkeypatch.setattr(cli, "_context", lambda args: context)
    for name in ("download_observed", "download_forecast", "run_current"):
        monkeypatch.setattr(
            cli, name, lambda ctx, *extra, name=name: calls.append((name, ctx, extra))
        )
    assert cli.main(["run-all", "--executable", "runner"]) == 0
    assert [call[0] for call in calls] == ["download_observed", "download_forecast", "run_current"]
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
    for name in ("download_observed", "download_forecast", "run_current"):
        monkeypatch.setattr(
            cli, name, lambda ctx, *extra: seen.append(ctx.settings["run"]["reference_time"])
        )
    assert cli.main(["run-all", "--workspace", str(tmp_path)]) == 0
    assert resolutions == ["now"]
    assert seen == ["2026-10-11T09:00:00"] * 3


@pytest.mark.parametrize("error,status", [(RuntimeError("failed"), 1), (KeyboardInterrupt(), 130)])
def test_run_all_stops_on_failure(context, monkeypatch, error, status):
    monkeypatch.setattr(cli, "_context", lambda args: context)
    monkeypatch.setattr(cli, "download_observed", Mock(side_effect=error))
    forecast = Mock()
    run = Mock()
    monkeypatch.setattr(cli, "download_forecast", forecast)
    monkeypatch.setattr(cli, "run_current", run)
    assert cli.main(["run-all"]) == status
    forecast.assert_not_called()
    run.assert_not_called()


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


def test_current_builds_before_execution_with_qc(context, monkeypatch):
    import mgb_ops.assets.history as history
    import mgb_ops.workflows as workflows
    import mgb_ops.workflows.scenarios as scenarios

    executable = context.paths.workspace / "runner"
    executable.touch()
    repository = Mock()
    repository.__enter__ = Mock(return_value=repository)
    repository.__exit__ = Mock(return_value=False)
    repository.list_enabled_providers.return_value = [{"provider_code": "ana"}]
    monkeypatch.setattr(history, "HistoryRepository", Mock(return_value=repository))
    calls = Mock()
    build = Mock()
    execute = Mock(return_value=SimpleNamespace(batch_id="batch", cache_dir="cache", results=()))
    calls.attach_mock(build, "build")
    calls.attach_mock(execute, "execute")
    monkeypatch.setattr(scenarios, "build_current_artifact", build)
    monkeypatch.setattr(workflows, "execute_current_artifact", execute)
    cli.run_current(context, "runner")
    assert [call[0] for call in calls.mock_calls] == ["build", "execute"]
    kwargs = build.call_args.kwargs
    assert kwargs["precipitation_qc_settings"] == context.settings["precipitation_qc"]
    assert kwargs["observed_providers"] == ("ana",)
    assert kwargs["reference_time"] == "2026-10-11T09:00:00"
    assert execute.call_args.kwargs["executable_path"] == executable
    build.side_effect = RuntimeError("build failed")
    execute.reset_mock()
    with pytest.raises(RuntimeError, match="build failed"):
        cli.run_current(context, "runner")
    execute.assert_not_called()


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
    assert "run-current" in result.stdout
