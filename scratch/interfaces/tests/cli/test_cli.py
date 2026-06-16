from __future__ import annotations

import json
from pathlib import Path
from types import SimpleNamespace

import pytest

from mgb_ops.cli import main as cli_main


def build_workspace(tmp_path: Path) -> Path:
    workspace = tmp_path / "region"
    input_dir = workspace / "mgb_runner" / "Input"
    output_dir = workspace / "mgb_runner" / "Output"
    input_dir.mkdir(parents=True)
    output_dir.mkdir(parents=True)
    (workspace / "mgb_runner" / "MGB_Inercial_PrevRS_FORTRAN.exe").write_text("binary", encoding="utf-8")
    (input_dir / "PARHIG.hig").write_text("parhig", encoding="utf-8")
    return workspace


def test_cli_model_run_dry_run_uses_workspace_paths(tmp_path, capsys) -> None:
    workspace = build_workspace(tmp_path)

    result = cli_main.main(["--workspace", str(workspace), "model", "run", "--dry-run"])
    captured = capsys.readouterr()
    payload = json.loads(captured.out)

    assert result == 0
    assert payload["status"] == "dry_run"
    assert payload["command"] == [str(workspace.resolve() / "mgb_runner" / "MGB_Inercial_PrevRS_FORTRAN.exe")]
    assert payload["local_input_dir"] == str(workspace.resolve() / "mgb_runner" / "Input")
    assert payload["local_output_dir"] == str(workspace.resolve() / "mgb_runner" / "Output")


def test_cli_dashboard_prints_streamlit_command(tmp_path, capsys) -> None:
    workspace = build_workspace(tmp_path)

    result = cli_main.main(["--workspace", str(workspace), "dashboard"])
    captured = capsys.readouterr()

    assert result == 0
    assert "streamlit run" in captured.out
    assert "apps/ops_dashboard/app.py" in captured.out
    assert f"--workspace {workspace.resolve()}" in captured.out


def test_cli_forecast_grid_passes_spatial_flags_to_library(tmp_path, monkeypatch, capsys) -> None:
    workspace = build_workspace(tmp_path)
    (workspace / "data").mkdir()
    history_db = workspace / "data" / "history.sqlite"
    history_db.write_text("", encoding="utf-8")

    captured_call = {}

    def fake_ingest_forecast_grids(database_path, **kwargs):
        captured_call["database_path"] = database_path
        captured_call.update(kwargs)
        return SimpleNamespace(asset_id="asset", asset_path="path")

    monkeypatch.setattr("mgb_ops.ingest.forecast_grid.ingest_forecast_grids", fake_ingest_forecast_grids)

    result = cli_main.main(
        [
            "--workspace",
            str(workspace),
            "ingest",
            "forecast-grid",
            "--bbox",
            "-60",
            "-35",
            "-48",
            "-26",
            "--buffer-fraction",
            "1",
        ]
    )

    assert result == 0
    assert captured_call["database_path"] == history_db.resolve()
    assert captured_call["bbox"] == (-60.0, -35.0, -48.0, -26.0)
    assert captured_call["buffer_fraction"] == 1.0
    assert json.loads(capsys.readouterr().out)["asset_id"] == "asset"


def test_cli_forecast_grid_uses_workspace_config_for_spatial_inputs(tmp_path, monkeypatch) -> None:
    workspace = build_workspace(tmp_path)
    config_dir = workspace / "config"
    config_dir.mkdir()
    (config_dir / "custom.yaml").write_text(
        """\
forecast_grid:
  bbox: [-60.0, -35.0, -48.0, -26.0]
  buffer_fraction: 0.5
""",
        encoding="utf-8",
    )
    captured_call = {}

    def fake_ingest_forecast_grids(database_path, **kwargs):
        captured_call.update(kwargs)
        return SimpleNamespace()

    monkeypatch.setattr("mgb_ops.ingest.forecast_grid.ingest_forecast_grids", fake_ingest_forecast_grids)

    result = cli_main.main(["--workspace", str(workspace), "ingest", "forecast-grid"])

    assert result == 0
    assert captured_call["bbox"] == (-60.0, -35.0, -48.0, -26.0)
    assert captured_call["buffer_fraction"] == 0.5


def test_cli_forecast_grid_requires_spatial_inputs(tmp_path) -> None:
    workspace = build_workspace(tmp_path)

    with pytest.raises(ValueError, match="bbox is required"):
        cli_main.main(["--workspace", str(workspace), "ingest", "forecast-grid"])
