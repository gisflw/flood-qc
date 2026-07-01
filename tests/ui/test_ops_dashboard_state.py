from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from apps.ops_dashboard import state as dashboard_state
from apps.ops_dashboard.services import forecast as dashboard_forecast
from apps.ops_dashboard.services import deckgl as dashboard_map
from db_helpers import initialize_history_db
from mgb_ops.analysis.spatial import PrecipitationGrid
from mgb_ops.edit.sqlite import list_forecast_corrections
from mgb_ops.assets.history import HistoryRepository


def _write_config(workspace: Path) -> None:
    config = workspace / "config"
    config.mkdir(parents=True)
    (config / "custom.yaml").write_text(
        "forecast_grid:\n  bbox: [-52.5, -31.5, -50.5, -29.5]\n",
        encoding="utf-8",
    )


def _preview() -> dashboard_forecast.ForecastPreview:
    return dashboard_forecast.ForecastPreview(
        asset_id="asset",
        relative_path="forecast.nc",
        data=np.array([[1.0, 2.0], [3.0, 4.0]]),
        latitudes=np.array([-31.0, -30.0]),
        longitudes=np.array([-52.0, -51.0]),
        t0_step=0,
        t1_step=3,
        mode_label="test",
        title="Forecast",
    )


def test_state_selection_and_raster_inspection(tmp_path: Path) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    assert state.gpkg_path == tmp_path / "data" / "source" / "rs_hydro.gpkg"
    grid = PrecipitationGrid(
        values=np.array([[1.0, 2.0], [3.0, 4.0]]),
        latitudes=np.array([-31.0, -30.0]),
        longitudes=np.array([-52.0, -51.0]),
        bounds=(-52.5, -31.5, -50.5, -29.5),
        start_time=pd.Timestamp("2026-01-01"),
        end_time=pd.Timestamp("2026-01-02"),
        source="test",
    )
    layer, lookup, legend = dashboard_map.build_raster_layer(
        grid, layer_id="rainfall-raster:test", layer_name="Test", opacity=0.7
    )
    state.map_artifacts = dashboard_map.DeckGLArtifacts(
        spec={"layers": [layer]},
        raster_lookups={lookup.layer_id: lookup},
        pick_lookups={
            "stations": (dashboard_map.MapSelection(station_id="1001"),),
            "mini-segments": (dashboard_map.MapSelection(mini_id=7),),
        },
        tooltips={},
        legends=(legend,),
    )

    state.handle_map_click(
        {"layer": "stations", "index": 0}
    )
    state.handle_map_click(
        {"layer": "mini-segments", "index": 0}
    )
    state.handle_map_click(
        {
            "layer": "rainfall-raster:test",
            "coordinate": [-51.05, -30.05],
        }
    )

    assert state.station_id == "1001"
    assert state.mini_id == 7
    assert state.raster_inspection.value == 4.0


def test_state_refresh_recomputes_source_versions(tmp_path: Path) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    before = state.source_versions["history"]
    (tmp_path / "data").mkdir()
    (tmp_path / "data" / "history.sqlite").touch()

    state.refresh()

    assert state.source_versions["history"] != before
    assert any("could not be read" in warning for warning in state.warnings)


def test_state_applies_preview_parameters(tmp_path: Path, monkeypatch) -> None:
    _write_config(tmp_path)
    state = dashboard_state.DashboardState(tmp_path)
    state.forecast_asset_id = "asset"
    state.forecast_t0_step = 0
    state.forecast_t1_step = 3
    state.forecast_shift_lat = 1
    state.forecast_multiplication_factor = 2
    monkeypatch.setattr(
        dashboard_forecast, "build_forecast_preview", lambda *args, **kwargs: _preview()
    )

    request = state.apply_preview()

    assert request.shift_lat == 1
    assert request.multiplication_factor == 2
    assert state.applied_preview_request == request
    assert state.forecast_map_artifacts.corrected is not None


def test_state_draft_validation_failure_sets_status(tmp_path: Path) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    state.forecast_asset_id = "asset"
    state.add_forecast_correction(reason="")

    with pytest.raises(ValueError, match="reason is required"):
        state.save_forecast_corrections()

    assert state.message_kind == "warning"


def test_state_persists_transactional_replacement(tmp_path: Path) -> None:
    db_path = initialize_history_db(tmp_path / "data" / "history.sqlite")
    with HistoryRepository(db_path) as repository:
        repository.upsert_asset(
            asset_id="asset",
            asset_kind="forecast_precipitation_grid",
            format="NetCDF",
            relative_path="forecast.nc",
            provider_code="ecmwf",
        )
    state = dashboard_state.DashboardState(tmp_path)
    state.forecast_asset_id = "asset"
    state.forecast_draft = dashboard_state.empty_forecast_edit_frame()
    state.add_forecast_correction(
        t0_step=0,
        t1_step=3,
        multiplication_factor=1.2,
        editor="operator",
        reason="radar alignment",
    )

    persisted = state.save_forecast_corrections()

    assert len(persisted) == 1
    assert list_forecast_corrections(db_path, "asset")[0]["reason"] == "radar alignment"
    assert state.message_kind == "success"
