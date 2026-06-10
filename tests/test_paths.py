from __future__ import annotations

from mgb_ops.common.paths import (
    MGB_OPS_WORKSPACE_ENV,
    build_run_db_path,
    clear_workspace,
    history_db_path,
    interim_dir,
    resolve_workspace,
    runtime_paths,
    runs_dir,
    set_workspace,
    spatial_dir,
    timeseries_dir,
)


def test_standard_paths_are_under_data() -> None:
    assert history_db_path().as_posix().endswith("data/history.sqlite")
    assert runs_dir().as_posix().endswith("data/runs")
    assert interim_dir().as_posix().endswith("data/interim")
    assert timeseries_dir().as_posix().endswith("data/timeseries")
    assert spatial_dir().as_posix().endswith("data/spatial")


def test_run_path_uses_single_sqlite_file() -> None:
    assert build_run_db_path("20260310T120000").as_posix().endswith("data/runs/20260310T120000.sqlite")


def test_workspace_argument_wins_over_env(monkeypatch, tmp_path) -> None:
    env_workspace = tmp_path / "env"
    cli_workspace = tmp_path / "cli"
    monkeypatch.setenv(MGB_OPS_WORKSPACE_ENV, str(env_workspace))

    assert resolve_workspace(cli_workspace) == cli_workspace.resolve()


def test_workspace_env_wins_over_cwd(monkeypatch, tmp_path) -> None:
    env_workspace = tmp_path / "env"
    cwd_workspace = tmp_path / "cwd"
    cwd_workspace.mkdir()
    monkeypatch.setenv(MGB_OPS_WORKSPACE_ENV, str(env_workspace))
    monkeypatch.chdir(cwd_workspace)

    assert resolve_workspace() == env_workspace.resolve()


def test_runtime_paths_resolve_under_workspace(tmp_path) -> None:
    workspace = tmp_path / "region"
    paths = runtime_paths(workspace)

    assert paths.history_db == workspace.resolve() / "data" / "history.sqlite"
    assert paths.logs_dir == workspace.resolve() / "logs"
    assert paths.mgb_runner_dir == workspace.resolve() / "mgb_runner"
    assert paths.mgb_input_dir == workspace.resolve() / "mgb_runner" / "Input"
    assert paths.mgb_output_dir == workspace.resolve() / "mgb_runner" / "Output"


def test_set_workspace_controls_default_paths(tmp_path) -> None:
    workspace = tmp_path / "region"
    try:
        set_workspace(workspace)
        assert history_db_path() == workspace.resolve() / "data" / "history.sqlite"
    finally:
        clear_workspace()
