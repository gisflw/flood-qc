from __future__ import annotations

import json
from pathlib import Path

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

