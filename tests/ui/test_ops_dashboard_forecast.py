from __future__ import annotations

from datetime import datetime

import folium
import folium.plugins
import numpy as np

from mgb_ops.ingest.forecast_grid import TpGribMessage
from mgb_ops.qc import ecmwf_forecast_correction
from apps.ops_dashboard.support import forecast as ops_dashboard_forecast
from db_helpers import initialize_history_db
from mgb_ops.storage.history_repository import HistoryRepository


def _message(step_hours: int, value: float) -> TpGribMessage:
    return TpGribMessage(
        valid_time=datetime(2026, 3, 11, step_hours, 0, 0),
        step_hours=step_hours,
        latitudes=np.array([-29.5, -30.5], dtype=np.float64),
        longitudes=np.array([-51.5, -50.5], dtype=np.float64),
        values_mm=np.full((2, 2), value, dtype=np.float64),
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
    forecast_path = tmp_path / "forecast.grib2"
    forecast_path.write_bytes(b"fake grib")
    initialize_history_db(db_path)

    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="ecmwf.ifs.fc.20260311T000000Z.rsbuf",
            asset_kind="forecast_grib_rs_buffered",
            format="GRIB2",
            relative_path=str(forecast_path),
            provider_code="ecmwf",
            valid_from="2026-03-11T00:00:00",
            valid_to="2026-03-12T00:00:00",
            metadata={"cycle_time": "2026-03-11T00:00:00Z"},
        )

    messages = [_message(0, 0.0), _message(3, 6.0), _message(6, 10.0)]
    monkeypatch.setattr(ops_dashboard_forecast, "read_tp_grib_messages", lambda _: messages)

    assets = ops_dashboard_forecast.list_forecast_assets(db_path)
    steps = ops_dashboard_forecast.list_forecast_steps("ecmwf.ifs.fc.20260311T000000Z.rsbuf", db_path)
    accum_preview = ops_dashboard_forecast.build_forecast_preview(
        "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
        t0_step=0,
        t1_step=3,
        database_path=db_path,
    )
    incr_preview = ops_dashboard_forecast.build_forecast_preview(
        "ecmwf.ifs.fc.20260311T000000Z.rsbuf",
        t0_step=3,
        t1_step=6,
        database_path=db_path,
    )

    assert assets["asset_id"].tolist() == ["ecmwf.ifs.fc.20260311T000000Z.rsbuf"]
    assert steps["step_hours"].tolist() == [0, 3, 6]
    assert np.allclose(accum_preview.data, 6.0)
    assert np.allclose(incr_preview.data, 4.0)


def test_ops_dashboard_forecast_applies_preview_correction() -> None:
    preview = _preview("teste")
    instruction = ecmwf_forecast_correction.ForecastCorrectionInstruction(
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
