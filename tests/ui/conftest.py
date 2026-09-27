from __future__ import annotations

import pytest

from mgb_ops.assets.current_run import CurrentRunArtifact, ForecastScenarioReference, create_current_artifact


@pytest.fixture(autouse=True)
def seed_dashboard_current_run(request, tmp_path):
    if request.node.path.name not in {
        "test_ops_dashboard_state.py",
        "test_ops_dashboard_factory.py",
    }:
        return
    create_current_artifact(
        tmp_path / "data" / "cache" / "current_run.sqlite",
        CurrentRunArtifact(
            "2026-03-12T00:00:00",
            "2026-03-27T00:00:00",
            1,
            {"observed_horizon_days": 56, "forecast_horizon_days": 14},
            {},
            {},
            {},
            ("ana",),
            scenarios=(ForecastScenarioReference("zero", 0, "zero", "Zero-rain horizon"),),
        ),
    )
