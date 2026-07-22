from __future__ import annotations

from datetime import datetime

import numpy as np

from apps.ops_dashboard.services import forecast as ops_dashboard_forecast
from db_helpers import initialize_history_db
from mgb_ops.analysis.forecast import ForecastIntegrityError
from mgb_ops.assets.history import HistoryRepository
from mgb_ops.assets.spatial_grid import RegularGridSpec, SPATIAL_GRID_ASSET_KIND, write_spatial_grid
from mgb_ops.assets.types import AnalysisWindow

from mgb_ops.edit.forcing import ForecastCorrectionInstruction

FORECAST_PRECIPITATION_GRID_ASSET_KIND = SPATIAL_GRID_ASSET_KIND


def write_forecast_precipitation_grid(
    path, *, times_utc, latitudes, longitudes, precipitation_mm, provider_code,
    source_format, source_cycle_time, timestep_hours=1, title=None
):
    from datetime import timezone
    resolution = float(abs(longitudes[1] - longitudes[0])) if len(longitudes) > 1 else 1.0
    return write_spatial_grid(
        path, variable="precipitation", grid_type="forecast",
        source="resampled_from_grid", providers=[provider_code], units="mm",
        bbox=(
            float(longitudes[0] - resolution / 2), float(latitudes[0] - resolution / 2),
            float(longitudes[-1] + resolution / 2), float(latitudes[-1] + resolution / 2),
        ),
        resolution_degrees=resolution,
        times_utc=[
            value.replace(tzinfo=timezone.utc) if value.tzinfo is None else value
            for value in times_utc
        ],
        latitudes=latitudes, longitudes=longitudes, values=precipitation_mm,
        timestep_hours=timestep_hours,
        processing_metadata={"source_format": source_format, "source_cycle_time": str(source_cycle_time)},
    )

def _preview(title: str, data: np.ndarray | None = None) -> ops_dashboard_forecast.ForecastPreview:
    return ops_dashboard_forecast.ForecastPreview(
        asset_id="asset",
        relative_path="forecast.grib2",
        data=np.array([[1.0, 2.0], [3.0, 4.0]], dtype=np.float64) if data is None else data,
        latitudes=np.array([-29.5, -30.5], dtype=np.float64),
        longitudes=np.array([-51.5, -50.5], dtype=np.float64),
        t0_step=0,
        t1_step=3,
        mode_label="acumulado_nativo",
        title=title,
    )


def test_ops_dashboard_forecast_lists_steps_and_builds_previews(tmp_path, monkeypatch) -> None:
    db_path = tmp_path / "history.sqlite"
    forecast_path = tmp_path / "forecast.nc"
    write_forecast_precipitation_grid(
        forecast_path,
        times_utc=[datetime(2026, 3, 11, 3), datetime(2026, 3, 11, 6)],
        latitudes=np.array([-30.5, -29.5]),
        longitudes=np.array([-51.5, -50.5]),
        precipitation_mm=np.stack([np.full((2, 2), 6.0), np.full((2, 2), 4.0)]),
        provider_code="ecmwf",
        source_format="GRIB2",
        source_cycle_time=datetime(2026, 3, 11),
        timestep_hours=3,
    )
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.buffered",
            asset_kind=FORECAST_PRECIPITATION_GRID_ASSET_KIND,
            format="NetCDF",
            relative_path=str(forecast_path),
            provider_code="ecmwf",
            valid_from="2026-03-11T00:00:00",
            valid_to="2026-03-12T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z", "type": "forecast"},
        )

    window = AnalysisWindow(
        start_time=datetime(2026, 3, 1),
        cutoff_time=datetime(2026, 3, 10, 23),
        forecast_end_exclusive=datetime(2026, 3, 13),
    )
    assets = ops_dashboard_forecast.list_forecast_assets(db_path, tmp_path, window=window)
    steps = ops_dashboard_forecast.list_forecast_steps(
        "ecmwf.ifs.fc.20260311T000000Z.buffered",
        database_path=db_path,
        workspace_path=tmp_path,
        window=window,
    )
    accum_preview = ops_dashboard_forecast.build_forecast_preview(
        "ecmwf.ifs.fc.20260311T000000Z.buffered",
        t0_step=0,
        t1_step=3,
        database_path=db_path,
        workspace_path=tmp_path,
        target_grid=RegularGridSpec((-51.5, -30.5, -50.5, -29.5), 0.5),
    )
    incr_preview = ops_dashboard_forecast.build_forecast_preview(
        "ecmwf.ifs.fc.20260311T000000Z.buffered",
        t0_step=3,
        t1_step=6,
        database_path=db_path,
        workspace_path=tmp_path,
        target_grid=RegularGridSpec((-51.5, -30.5, -50.5, -29.5), 0.5),
    )

    assert assets["asset_id"].tolist() == ["ecmwf.ifs.fc.20260311T000000Z.buffered"]
    assert steps["step_hours"].tolist() == [3, 6]
    assert np.allclose(accum_preview.data, 6.0)
    assert np.allclose(incr_preview.data, 4.0)


    native_preview = ops_dashboard_forecast.build_forecast_preview(
        "ecmwf.ifs.fc.20260311T000000Z.buffered",
        t0_step=0,
        t1_step=3,
        database_path=db_path,
        workspace_path=tmp_path,
    )
    assert native_preview.source_grid is not None
    assert native_preview.source_grid.bounds == (-52.0, -31.0, -50.0, -29.0)


def test_ops_dashboard_forecast_applies_preview_correction() -> None:
    preview = _preview("teste")
    instruction = ForecastCorrectionInstruction(
        asset_id="asset",
        t0_step=0,
        t1_step=3,
        shift_lat=1.0,
        shift_lon=0.0,
        rotation_deg=0.0,
        multiplication_factor=2.0,
    )

    corrected = ops_dashboard_forecast.apply_preview_corrections(preview, [instruction])

    np.testing.assert_allclose(corrected.data, [[2.0, 4.0], [6.0, 8.0]])
    np.testing.assert_allclose(corrected.latitudes, [-28.5, -29.5])
    assert corrected.source_grid is not None
    assert corrected.source_grid.bounds == (-51.5, -29.5, -50.5, -28.5)


def test_expected_ecmwf_cycle_reports_unregistered_cycle(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    window = AnalysisWindow(
        start_time=datetime(2026, 3, 1),
        cutoff_time=datetime(2026, 3, 10, 23),
        forecast_end_exclusive=datetime(2026, 3, 13),
    )

    try:
        ops_dashboard_forecast.list_forecast_assets(
            db_path, tmp_path, window=window
        )
    except ForecastIntegrityError as exc:
        assert exc.code == "unregistered_cycle"
        assert "2026-03-11T00:00:00Z" in str(exc)
    else:
        raise AssertionError("Expected an unregistered-cycle integrity error.")


def test_expected_ecmwf_cycle_reports_missing_registered_file(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.buffered",
            asset_kind=FORECAST_PRECIPITATION_GRID_ASSET_KIND,
            format="NetCDF",
            relative_path="data/downloads/ecmwf/missing.nc",
            provider_code="ecmwf",
            valid_from="2026-03-11T00:00:00",
            valid_to="2026-03-12T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z", "type": "forecast"},
        )
    window = AnalysisWindow(
        start_time=datetime(2026, 3, 1),
        cutoff_time=datetime(2026, 3, 10, 23),
        forecast_end_exclusive=datetime(2026, 3, 13),
    )

    try:
        ops_dashboard_forecast.list_forecast_assets(
            db_path, tmp_path, window=window
        )
    except ForecastIntegrityError as exc:
        assert exc.code == "missing_registered_file"
        assert "missing.nc" in str(exc)
    else:
        raise AssertionError("Expected a missing-file integrity error.")


def test_build_forecast_map_returns_single_deckgl_spec_with_raster() -> None:
    specs = ops_dashboard_forecast.build_forecast_map(
        _preview("Mapa original"), opacity=0.65
    )

    assert len(specs) == 1
    assert specs[0]["layers"][0]["@@type"] == "BitmapLayer"
    assert specs[0]["layers"][0]["id"] == "forecast-original"


def test_build_forecast_map_returns_two_specs_with_synced_view() -> None:
    original = _preview("Mapa original")
    corrected = _preview("Mapa corrigido", data=np.array([[2.0, 3.0], [4.0, 5.0]], dtype=np.float64))

    specs = ops_dashboard_forecast.build_forecast_map(
        original, corrected_preview=corrected, opacity=0.8
    )

    assert len(specs) == 2
    assert specs[0]["initialViewState"] == specs[1]["initialViewState"]
    assert specs[0]["layers"][0]["id"] == "forecast-original"
    assert specs[1]["layers"][0]["id"] == "forecast-corrected"


def test_build_forecast_map_artifacts_returns_external_legends() -> None:
    original = _preview("Mapa original")
    corrected = _preview("Mapa corrigido", data=np.array([[2.0, 3.0], [4.0, 5.0]], dtype=np.float64))

    artifacts = ops_dashboard_forecast.build_forecast_map_artifacts(
        original,
        corrected_preview=corrected,
        opacity=0.8,
        component_key="forecast-test",
    )

    assert artifacts.corrected is not None
    assert "Mapa original" in artifacts.original.legend_html
    assert "Mapa corrigido" in artifacts.corrected.legend_html
    assert artifacts.original.spec["initialViewState"] == artifacts.corrected.spec["initialViewState"]


def test_synchronize_view_state_copies_only_portable_values() -> None:
    state = ops_dashboard_forecast.synchronize_view_state(
        {
            "longitude": -51,
            "latitude": -30,
            "zoom": 8,
            "pitch": 20,
            "bearing": 4,
            "transitionDuration": 100,
        }
    )

    assert state == {
        "longitude": -51.0,
        "latitude": -30.0,
        "zoom": 8.0,
        "pitch": 20.0,
        "bearing": 4.0,
    }



def test_ops_dashboard_forecast_uses_provider_lookback_cycle(tmp_path) -> None:
    db_path = initialize_history_db(tmp_path / "history.sqlite")
    forecast_path = tmp_path / "noaa_06.nc"
    write_forecast_precipitation_grid(
        forecast_path,
        times_utc=[datetime(2026, 7, 20, 13), datetime(2026, 7, 20, 14)],
        latitudes=np.array([-30.5, -29.5]),
        longitudes=np.array([-51.5, -50.5]),
        precipitation_mm=np.ones((2, 2, 2)),
        provider_code="noaa",
        source_format="GRIB2",
        source_cycle_time=datetime(2026, 7, 20, 6),
    )
    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="noaa.gfs.fc.20260720T060000Z.precipitation_grid",
            asset_kind=FORECAST_PRECIPITATION_GRID_ASSET_KIND,
            format="NetCDF",
            relative_path=str(forecast_path),
            provider_code="noaa",
            valid_from="2026-07-20T06:00:00",
            valid_to="2026-07-30T18:00:00",
            metadata={"cycle_time": "2026-07-20T06:00:00Z", "type": "forecast"},
        )
    window = AnalysisWindow(
        start_time=datetime(2026, 7, 5),
        cutoff_time=datetime(2026, 7, 20, 9),
        forecast_end_exclusive=datetime(2026, 7, 31),
    )

    assets = ops_dashboard_forecast.list_forecast_assets(
        db_path, tmp_path, window=window, provider_code="noaa", lookback_cycles=2
    )

    assert assets["asset_id"].tolist() == ["noaa.gfs.fc.20260720T060000Z.precipitation_grid"]
