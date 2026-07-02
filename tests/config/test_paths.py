from __future__ import annotations

from mgb_ops.config.workspace import (
    MGB_OPS_WORKSPACE_ENV,
    RuntimePaths,
    resolve_workspace,
    resolve_workspace_path,
)


def test_standard_paths_are_explicitly_derived_from_workspace(tmp_path) -> None:
    workspace = tmp_path / "region"
    paths = RuntimePaths(workspace)

    assert paths.workspace == workspace.resolve()
    assert paths.history_db == workspace.resolve() / "data" / "history.sqlite"
    assert paths.source_dir == workspace.resolve() / "data" / "source"
    assert paths.downloads_dir == workspace.resolve() / "data" / "downloads"
    assert paths.assets_dir == workspace.resolve() / "data" / "assets"
    assert paths.cache_dir == workspace.resolve() / "data" / "cache"
    assert paths.processed_dir == workspace.resolve() / "data" / "processed"
    assert paths.reports_dir == workspace.resolve() / "data" / "reports"
    assert paths.runs_dir == workspace.resolve() / "data" / "runs"
    assert paths.run_db_path("20260310T120000") == (
        workspace.resolve() / "data" / "runs" / "20260310T120000.sqlite"
    )
    assert paths.station_inventory_csv_path == (
        workspace.resolve() / "data" / "source" / "history_station_inventory.csv"
    )
    assert paths.logs_dir == workspace.resolve() / "logs"
    assert paths.mgb_input_dir == workspace.resolve() / "mgb_runner" / "Input"
    assert paths.mgb_output_dir == workspace.resolve() / "mgb_runner" / "Output"


def test_workspace_argument_wins_over_env(monkeypatch, tmp_path) -> None:
    monkeypatch.setenv(MGB_OPS_WORKSPACE_ENV, str(tmp_path / "env"))
    explicit_workspace = tmp_path / "explicit"
    assert resolve_workspace(explicit_workspace) == explicit_workspace.resolve()


def test_workspace_env_wins_over_cwd(monkeypatch, tmp_path) -> None:
    env_workspace = tmp_path / "env"
    cwd_workspace = tmp_path / "cwd"
    cwd_workspace.mkdir()
    monkeypatch.setenv(MGB_OPS_WORKSPACE_ENV, str(env_workspace))
    monkeypatch.chdir(cwd_workspace)
    assert resolve_workspace() == env_workspace.resolve()


def test_resolve_workspace_path_handles_relative_and_absolute_paths(tmp_path) -> None:
    absolute = tmp_path / "outside.gpkg"
    assert resolve_workspace_path(tmp_path / "region", "data/source/a.gpkg") == (
        tmp_path / "region" / "data" / "source" / "a.gpkg"
    ).resolve()
    assert resolve_workspace_path(tmp_path / "region", absolute) == absolute.resolve()


def test_ensure_standard_dirs_creates_only_canonical_directories(tmp_path) -> None:
    paths = RuntimePaths(tmp_path / "region")
    legacy_file = paths.data_dir / "interim" / "legacy.txt"
    legacy_file.parent.mkdir(parents=True)
    legacy_file.write_text("legacy", encoding="utf-8")

    paths.ensure_standard_dirs()

    for path in (
        paths.source_dir,
        paths.downloads_dir,
        paths.assets_dir,
        paths.cache_dir,
        paths.processed_dir,
        paths.reports_dir,
        paths.runs_dir,
        paths.logs_dir,
    ):
        assert path.is_dir()
    assert legacy_file.read_text(encoding="utf-8") == "legacy"
