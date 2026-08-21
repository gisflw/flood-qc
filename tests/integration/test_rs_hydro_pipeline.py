from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[2]
RS_HYDRO = REPO_ROOT.parent / "rs_hydro"
STAGES = (
    "01_download_observed.py",
    "02_download_forecast.py",
    "03_build_current_run.py",
    "04_execute_current_run.py",
    "05_analysis.py",
)


def _import_module(path: Path):
    spec = importlib.util.spec_from_file_location(f"test_{path.stem}", path)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_renumbered_pipeline_modules_import_without_running_main() -> None:
    if not RS_HYDRO.exists():
        pytest.skip("RS Hydro workspace is not available")
    assert not (RS_HYDRO / ("01_download_" + "providers.py")).exists()
    for name in STAGES:
        module = _import_module(RS_HYDRO / name)
        assert callable(module.main)


def test_pipeline_summary_and_analysis_execution_hint_use_new_stage_names() -> None:
    if not RS_HYDRO.exists():
        pytest.skip("RS Hydro workspace is not available")
    summary = (RS_HYDRO / "00_runtime.py").read_text(encoding="utf-8")
    analysis = (RS_HYDRO / "05_analysis.py").read_text(encoding="utf-8")

    assert "01_download_observed.py" in summary
    assert "02_download_forecast.py" in summary
    assert "03_build_current_run.py" in summary
    assert "04_execute_current_run.py" in summary
    assert "05_analysis.py" in summary
    assert "Run 04_execute_current_run.py first." in analysis
