from __future__ import annotations

from pathlib import Path

import pytest

pytest.importorskip("numpy")

from mgb_ops.common.models import RunKind, RunMetadata, RunStatus
from mgb_ops.ingest import forecast_grid as forecast_grid_module
from mgb_ops.ingest.forecast_grid import collect_forecast_grids


@pytest.fixture
def run_metadata() -> RunMetadata:
    return RunMetadata(
        run_id="20260310T120000",
        reference_time="2026-03-10T12:00:00",
        run_kind=RunKind.AUTOMATIC,
        status=RunStatus.DRAFT,
    )


def test_collect_forecast_grids_returns_registered_asset(run_metadata: RunMetadata, monkeypatch) -> None:
    monkeypatch.setattr(
        forecast_grid_module,
        "ingest_forecast_grids",
        lambda *args, **kwargs: forecast_grid_module.ForecastGridSummary(
            run_id="20260310T120000",
            asset_id="ecmwf.ifs.fc.20260310T000000Z.rsbuf",
            asset_path=Path("/tmp/fc_2026-03-10_00_IFS_rsbuf.grib2"),
            valid_from=forecast_grid_module.datetime(2026, 3, 10, 3, 0, 0),
            valid_to=forecast_grid_module.datetime(2026, 3, 25, 0, 0, 0),
        ),
    )

    assets = collect_forecast_grids(run_metadata)

    assert len(assets) == 1
    assert assets[0].format == "GRIB2"
    assert assets[0].relative_path == "/tmp/fc_2026-03-10_00_IFS_rsbuf.grib2"
