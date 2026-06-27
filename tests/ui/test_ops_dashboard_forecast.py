from __future__ import annotations

from datetime import datetime, timedelta

import folium
import folium.plugins
import numpy as np

from mgb_ops.qc import grib2_forecast_correction
from mgb_ops.model.forecast_grid import (
    FORECAST_PRECIPITATION_GRID_ASSET_KIND,
    write_forecast_precipitation_grid,
)
from mgb_ops.analysis.spatial import RegularGridSpec
from apps.ops_dashboard.support import forecast as ops_dashboard_forecast
from db_helpers import initialize_history_db
from mgb_ops.storage.history_repository import HistoryRepository


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
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )

    assets = ops_dashboard_forecast.list_forecast_assets(db_path, tmp_path)
    steps = ops_dashboard_forecast.list_forecast_steps(
        "ecmwf.ifs.fc.20260311T000000Z.buffered", database_path=db_path, workspace_path=tmp_path
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
    assert steps["step_hours"].tolist() == [0, 3, 6]
    assert np.allclose(accum_preview.data, 6.0)
    assert np.allclose(incr_preview.data, 4.0)


def test_ops_dashboard_forecast_applies_preview_correction() -> None:
    preview = _preview("teste")
    instruction = grib2_forecast_correction.ForecastCorrectionInstruction(
        asset_id="asset",
        t0_step=0,
        t1_step=3,
        shift_lat=1.0,
        shift_lon=0.0,
        rotation_deg=0.0,
        multiplication_factor=2.0,
    )

    corrected = ops_dashboard_forecast.apply_preview_corrections(preview, [instruction])

    assert corrected.data[0, 0] == 0.0
    assert corrected.data[1, 0] == 2.0


def test_build_forecast_map_returns_single_map_with_raster_inspector(monkeypatch) -> None:
    monkeypatch.setattr(ops_dashboard_forecast.ops_dashboard_data, "load_rivers_layer_geojson", lambda *args, **kwargs: None)

    fmap = ops_dashboard_forecast.build_forecast_map(_preview("Mapa original"), opacity=0.65)

    child_names = {child._name for child in fmap._children.values()}
    assert isinstance(fmap, folium.Map)
    assert "RasterClickPopup" in child_names
    assert "LayerControl" not in child_names


def test_build_forecast_map_returns_dual_map_with_synced_layers(monkeypatch) -> None:
    monkeypatch.setattr(ops_dashboard_forecast.ops_dashboard_data, "load_rivers_layer_geojson", lambda *args, **kwargs: None)
    original = _preview("Mapa original")
    corrected = _preview("Mapa corrigido", data=np.array([[2.0, 3.0], [4.0, 5.0]], dtype=np.float64))

    fmap = ops_dashboard_forecast.build_forecast_map(original, corrected_preview=corrected, opacity=0.8)

    assert isinstance(fmap, folium.plugins.DualMap)
    assert "RasterClickPopup" in {child._name for child in fmap.m1._children.values()}
    assert "RasterClickPopup" in {child._name for child in fmap.m2._children.values()}
    assert "LayerControl" not in {child._name for child in fmap.m1._children.values()}
    assert "LayerControl" not in {child._name for child in fmap.m2._children.values()}


def test_build_forecast_map_artifacts_returns_external_legends(monkeypatch) -> None:
    monkeypatch.setattr(ops_dashboard_forecast.ops_dashboard_data, "load_rivers_layer_geojson", lambda *args, **kwargs: None)
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
