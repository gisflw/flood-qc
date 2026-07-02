from __future__ import annotations

from io import StringIO
from pathlib import Path

import pytest

from mgb_ops.assets.types import RunMetadata
from mgb_ops.model import mgb_execution
from mgb_ops.model import run_mgb


class FakeProcess:
    def __init__(self, lines: list[str], return_code: int, on_wait=None) -> None:
        self.stdout = StringIO("".join(lines))
        self._return_code = return_code
        self._on_wait = on_wait

    def wait(self) -> int:
        if self._on_wait is not None:
            self._on_wait()
        return self._return_code


def configure_paths(monkeypatch, tmp_path: Path) -> tuple[Path, Path, Path, Path]:
    executable_path = tmp_path / "mgb_runner" / "MGB_Inercial_PrevRS_FORTRAN.exe"
    input_dir = tmp_path / "mgb_runner" / "Input"
    output_dir = tmp_path / "mgb_runner" / "Output"
    workspace_root = tmp_path / "remote" / "mgb-hora"

    executable_path.parent.mkdir(parents=True, exist_ok=True)
    executable_path.write_text("binary", encoding="utf-8")

    input_dir.mkdir(parents=True, exist_ok=True)
    (input_dir / "PARHIG.hig").write_text("parhig", encoding="utf-8")
    (input_dir / "nested").mkdir()
    (input_dir / "nested" / "MINI.gtp").write_text("mini", encoding="utf-8")

    output_dir.mkdir(parents=True, exist_ok=True)
    (output_dir / "stale.txt").write_text("old", encoding="utf-8")

    monkeypatch.setattr(mgb_execution, "build_execution_id", lambda: "20260325T120000")
    return executable_path, input_dir, output_dir, workspace_root


def build_plan(monkeypatch, tmp_path: Path):
    run = RunMetadata(run_id="20260325T120000", reference_time="2026-03-25T12:00:00")
    executable_path, input_dir, output_dir, workspace_root = configure_paths(monkeypatch, tmp_path)
    return mgb_execution.prepare_mgb_execution(
        run,
        executable_path=str(executable_path),
        input_dir=input_dir,
        output_dir=output_dir,
        workspace_root=workspace_root,
        asset_base_dir=tmp_path,
    )


def test_prepare_mgb_execution_uses_fixed_paths(monkeypatch, tmp_path) -> None:
    executable_path, input_dir, output_dir, workspace_root = configure_paths(monkeypatch, tmp_path)
    run = RunMetadata(run_id="20260325T120000", reference_time="2026-03-25T12:00:00")

    plan = mgb_execution.prepare_mgb_execution(
        run,
        executable_path=str(executable_path),
        input_dir=input_dir,
        output_dir=output_dir,
        workspace_root=workspace_root,
        asset_base_dir=tmp_path,
    )

    assert plan.command == [str(executable_path)]
    assert plan.working_directory == str(workspace_root)
    assert plan.metadata["executable_path"] == str(executable_path)
    assert plan.metadata["input_dir"] == str(input_dir)
    assert plan.metadata["output_dir"] == str(output_dir)
    assert "local_input_dir" not in plan.metadata
    assert "local_output_dir" not in plan.metadata
    assert "remote_input_dir" not in plan.metadata
    assert "remote_output_dir" not in plan.metadata


def test_execute_mgb_plan_dry_run_has_no_side_effects(monkeypatch, tmp_path) -> None:
    plan = build_plan(monkeypatch, tmp_path)

    def fail_popen(*args, **kwargs):
        raise AssertionError("subprocess should not be called in dry-run")

    monkeypatch.setattr(mgb_execution.subprocess, "Popen", fail_popen)

    result = mgb_execution.execute_mgb_plan(plan, dry_run=True)

    assert result.output_name == "mgb_dry_run"
    assert plan.metadata["status"] == "dry_run"
    assert not (tmp_path / "logs").exists()
    assert not Path(plan.metadata["workspace_root"]).exists()


def test_prepare_output_directory_clears_only_output(monkeypatch, tmp_path) -> None:
    plan = build_plan(monkeypatch, tmp_path)
    logger = mgb_execution.configure_run_logger(tmp_path / "logs" / "mgb_execution" / "prep.log")
    output_dir = Path(plan.metadata["output_dir"])
    stale_file = output_dir / "stale.txt"
    stale_dir_file = output_dir / "old" / "legacy.dat"
    stale_dir_file.parent.mkdir(parents=True, exist_ok=True)
    stale_file.write_text("stale", encoding="utf-8")
    stale_dir_file.write_text("legacy", encoding="utf-8")

    prepared_output_dir = mgb_execution._prepare_output_directory(plan, logger)

    assert prepared_output_dir == output_dir
    assert not stale_file.exists()
    assert not stale_dir_file.exists()
    assert list(output_dir.iterdir()) == []


def test_execute_mgb_plan_runs_process_logs_and_collects_direct_output(monkeypatch, tmp_path, capsys) -> None:
    plan = build_plan(monkeypatch, tmp_path)
    output_dir = Path(plan.metadata["output_dir"])

    def fake_popen(command, cwd, env, stdout, stderr, text, bufsize):
        assert command == plan.command
        assert cwd == plan.working_directory
        assert env["EXISTING_ENV"] == "kept"
        assert env["MGB_INPUT_DIR"] == plan.metadata["input_dir"]
        assert env["MGB_OUTPUT_DIR"] == plan.metadata["output_dir"]
        assert stdout is mgb_execution.subprocess.PIPE
        assert stderr is mgb_execution.subprocess.STDOUT
        assert text is True
        assert bufsize == 1

        def on_wait() -> None:
            (output_dir / "QTUDO01.MGB").write_text("binary-output", encoding="utf-8")

        return FakeProcess(["linha 1\n", "linha 2\n"], 0, on_wait=on_wait)

    monkeypatch.setattr(mgb_execution.subprocess, "Popen", fake_popen)

    result = mgb_execution.execute_mgb_plan(plan, logs_dir=tmp_path / "logs", env={"EXISTING_ENV": "kept"})
    captured = capsys.readouterr()
    log_path = tmp_path / "logs" / "mgb_execution" / "20260325T120000.log"

    assert result.output_name == "mgb_output"
    assert "mgb_runner/Output/QTUDO01.MGB" in result.asset_refs[0]
    assert plan.metadata["status"] == "success"
    assert plan.metadata["return_code"] == 0
    assert "linha 1" in captured.out
    assert "linha 2" in captured.out
    assert log_path.exists()
    assert "mgb_execution_finished" in log_path.read_text(encoding="utf-8")
    assert (tmp_path / "mgb_runner" / "Output" / "QTUDO01.MGB").read_text(encoding="utf-8") == "binary-output"
    assert not (tmp_path / "mgb_runner" / "Output" / "stale.txt").exists()


def test_execute_mgb_plan_fails_on_nonzero_exit(monkeypatch, tmp_path) -> None:
    plan = build_plan(monkeypatch, tmp_path)

    monkeypatch.setattr(
        mgb_execution.subprocess,
        "Popen",
        lambda *args, **kwargs: FakeProcess(["erro\n"], 7),
    )

    with pytest.raises(RuntimeError, match="exit code 7"):
        mgb_execution.execute_mgb_plan(plan, logs_dir=tmp_path / "logs")


def test_execute_mgb_plan_fails_when_output_is_empty(monkeypatch, tmp_path) -> None:
    plan = build_plan(monkeypatch, tmp_path)

    monkeypatch.setattr(
        mgb_execution.subprocess,
        "Popen",
        lambda *args, **kwargs: FakeProcess(["sem output\n"], 0),
    )

    with pytest.raises(RuntimeError, match="sem arquivos em Output"):
        mgb_execution.execute_mgb_plan(plan, logs_dir=tmp_path / "logs")


def test_model_run_summary_reports_execution_plan(tmp_path) -> None:
    plan = mgb_execution.CommandPlan(
        command=["fake.exe"],
        working_directory="C:/mgb-hora",
        metadata={
            "workspace_root": "C:/mgb-hora",
            "input_dir": "mgb_runner/Input",
            "output_dir": "mgb_runner/Output",
            "asset_base_dir": str(tmp_path),
            "status": "success",
            "log_path": "logs/mgb_execution/20260325T120000.log",
        },
    )

    result = mgb_execution.ModelOutput(
        output_name="mgb_output",
        description="Execucao real do MGB concluida com sucesso.",
        asset_refs=["mgb_runner/Output/QTUDO01.MGB"],
    )

    summary = run_mgb.build_summary(plan, result, dry_run=False)

    assert summary["status"] == "success"
    assert summary["command"] == ["fake.exe"]
    assert summary["input_dir"] == "mgb_runner/Input"
    assert summary["output_dir"] == "mgb_runner/Output"
    assert summary["asset_refs"] == ["mgb_runner/Output/QTUDO01.MGB"]
