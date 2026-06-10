from __future__ import annotations

from pathlib import Path

import pytest

from mgb_ops.common.models import DataState, RunKind, RunMetadata, RunStatus, TimeSeriesRecord
from mgb_ops.ingest import forecast_grid as forecast_grid_module
from mgb_ops.ingest.forecast_grid import collect_forecast_grids
from mgb_ops.ingest.observed import collect_observed_timeseries
from mgb_ops.model.run_builder import assemble_model_inputs
from mgb_ops.qc.automatic import apply_automatic_qc
from mgb_ops.qc.review import register_manual_review


@pytest.fixture
def run_metadata() -> RunMetadata:
    return RunMetadata(
        run_id="20260310T120000",
        reference_time="2026-03-10T12:00:00",
        run_kind=RunKind.AUTOMATIC,
        status=RunStatus.DRAFT,
    )


def test_dataclass_instantiation() -> None:
    record = TimeSeriesRecord(
        series_id="ana.level.123",
        station_code="123",
        variable="level",
        unit="cm",
        state=DataState.RAW,
    )
    assert record.station_code == "123"


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


def test_stubs_raise_not_implemented(run_metadata: RunMetadata) -> None:
    with pytest.raises(NotImplementedError):
        collect_observed_timeseries(run_metadata)
    with pytest.raises(NotImplementedError):
        apply_automatic_qc(run_metadata)
    with pytest.raises(NotImplementedError):
        register_manual_review(run_metadata, [])
    with pytest.raises(NotImplementedError):
        assemble_model_inputs(run_metadata)
