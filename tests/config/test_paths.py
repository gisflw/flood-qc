from __future__ import annotations

from mgb_ops.common.paths import (
    MGB_OPS_WORKSPACE_ENV,
    build_run_db_path,
    cache_dir,
    clear_workspace,
    downloads_dir,
    ensure_standard_dirs,
    history_db_path,
    logs_dir,
    processed_dir,
    reports_dir,
    resolve_workspace,
    runtime_paths,
    runs_dir,
    set_workspace,
    source_dir,
    station_inventory_csv_path,
)


def test_standard_paths_are_under_data() -> None:
    assert history_db_path().as_posix().endswith("data/history.sqlite")
    assert source_dir().as_posix().endswith("data/source")
    assert downloads_dir().as_posix().endswith("data/downloads")
    assert cache_dir().as_posix().endswith("data/cache")
    assert processed_dir().as_posix().endswith("data/processed")
    assert reports_dir().as_posix().endswith("data/reports")
    assert runs_dir().as_posix().endswith("data/runs")
    assert station_inventory_csv_path().as_posix().endswith("data/source/history_station_inventory.csv")


def test_run_path_uses_single_sqlite_file() -> None:
    assert build_run_db_path("20260310T120000").as_posix().endswith("data/runs/20260310T120000.sqlite")


def test_workspace_argument_wins_over_env(monkeypatch, tmp_path) -> None:
    env_workspace = tmp_path / "env"
    explicit_workspace = tmp_path / "explicit"
    monkeypatch.setenv(MGB_OPS_WORKSPACE_ENV, str(env_workspace))

    assert resolve_workspace(explicit_workspace) == explicit_workspace.resolve()


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
    assert paths.source_dir == workspace.resolve() / "data" / "source"
    assert paths.downloads_dir == workspace.resolve() / "data" / "downloads"
    assert paths.cache_dir == workspace.resolve() / "data" / "cache"
    assert paths.processed_dir == workspace.resolve() / "data" / "processed"
    assert paths.reports_dir == workspace.resolve() / "data" / "reports"
    assert paths.station_inventory_csv_path == workspace.resolve() / "data" / "source" / "history_station_inventory.csv"
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


def test_ensure_standard_dirs_creates_only_canonical_data_dirs(tmp_path) -> None:
    workspace = tmp_path / "region"
    ensure_standard_dirs(workspace)

    assert (workspace / "data" / "source").is_dir()
    assert (workspace / "data" / "downloads").is_dir()
    assert (workspace / "data" / "cache").is_dir()
    assert (workspace / "data" / "processed").is_dir()
    assert (workspace / "data" / "reports").is_dir()
    assert (workspace / "data" / "runs").is_dir()
    assert (workspace / "logs").is_dir()
    assert not (workspace / "data" / "interim").exists()
    assert not (workspace / "data" / "timeseries").exists()
    assert not (workspace / "data" / "spatial").exists()


def test_ensure_standard_dirs_leaves_existing_legacy_dirs_alone(tmp_path) -> None:
    workspace = tmp_path / "region"
    legacy_file = workspace / "data" / "interim" / "legacy.txt"
    legacy_file.parent.mkdir(parents=True)
    legacy_file.write_text("legacy", encoding="utf-8")

    ensure_standard_dirs(workspace)

    assert legacy_file.read_text(encoding="utf-8") == "legacy"
