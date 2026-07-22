from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from apps.ops_dashboard import state as dashboard_state
from apps.ops_dashboard.services import forecast as dashboard_forecast
from apps.ops_dashboard.services import deckgl as dashboard_map
from db_helpers import initialize_history_db
from mgb_ops.assets.spatial_grid import PrecipitationGrid
from mgb_ops.edit.sqlite import list_forecast_corrections
from mgb_ops.assets.history import HistoryRepository


def _write_config(workspace: Path) -> None:
    config = workspace / "config"
    config.mkdir(parents=True)
    (config / "custom.yaml").write_text(
        "spatial_grid:\n  bbox: [-52.5, -31.5, -50.5, -29.5]\n",
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
    assert state.station_id is None
    assert state.mini_id is None
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


def test_state_does_not_preselect_an_available_station(
    tmp_path: Path, monkeypatch
) -> None:
    data = tmp_path / "data"
    data.mkdir()
    (data / "history.sqlite").touch()
    monkeypatch.setattr(
        dashboard_state,
        "_station_catalog",
        lambda *args: pd.DataFrame(
            [
                {
                    "station_id": "1001",
                    "status": "ok",
                    "kind": "mixed",
                    "station_name": "Station",
                    "provider_code": "ana",
                    "station_code": "1001",
                    "lon": -52.0,
                    "lat": -30.0,
                }
            ]
        ),
    )

    state = dashboard_state.DashboardState(tmp_path)

    assert state.stations["station_id"].tolist() == ["1001"]
    assert state.station_id is None
    assert state.mini_id is None


def test_opacity_change_does_not_rebuild_map_or_reload_spatial_data(
    tmp_path: Path, monkeypatch
) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    layer = {
        "@@type": "BitmapLayer",
        "id": "rainfall-raster:test",
        "image": "data:image/png;base64,test",
        "opacity": 0.6,
    }
    state.map_artifacts = dashboard_map.DeckGLArtifacts(
        spec={"layers": [layer]},
        raster_lookups={},
        pick_lookups={},
        tooltips={},
    )
    monkeypatch.setattr(
        dashboard_map,
        "build_ops_map",
        lambda *args, **kwargs: pytest.fail("full map rebuild"),
    )
    monkeypatch.setattr(
        dashboard_state,
        "_mini_segment_paths",
        lambda *args, **kwargs: pytest.fail("spatial reload"),
    )

    state.raster_opacity = 0.25

    assert state.map_artifacts.spec["layers"][0] is layer
    assert state.map_artifacts.spec["layers"][0]["opacity"] == 0.6


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


def test_custom_rainfall_hours_apply_and_refresh(tmp_path: Path, monkeypatch) -> None:
    _write_config(tmp_path)
    initialize_history_db(tmp_path / "data" / "history.sqlite")
    calls: list[int] = []

    def fake_raster(*args, **kwargs):
        hours = args[6]
        calls.append(hours)
        return {"name": f"accum_{hours}h", "horizon_hours": hours,
                "horizon_label": f"{hours}h", "grid": None}

    monkeypatch.setattr(dashboard_state, "_accumulation_raster", fake_raster)
    monkeypatch.setattr(dashboard_state.DashboardState, "_rebuild_map", lambda self, **kwargs: None)
    state = dashboard_state.DashboardState(tmp_path)
    assert state.observed_precipitation_path.name == "precipitations_mgb_observed.nc"
    assert state.rainfall_hours == 24
    assert calls == [24]
    assert state.selected_raster == "accum_24h"

    state.rainfall_hours = 100
    state.apply_rainfall_hours()
    assert calls[-1] == 100
    assert state.selected_raster == "accum_100h"
    assert len(state.accumulation_rasters) == 1

    state.refresh()
    assert state.rainfall_hours == 100
    assert calls[-1] == 100


def test_forecast_rainfall_uses_separate_hours_and_cache(
    tmp_path: Path, monkeypatch
) -> None:
    _write_config(tmp_path)
    initialize_history_db(tmp_path / "data" / "history.sqlite")
    calls: list[tuple[str, str, int]] = []

    def fake_raster(*args, **kwargs):
        mode = kwargs["rainfall_mode"]
        hours = args[6]
        calls.append((mode, Path(args[0]).name, hours))
        return {
            "name": f"{mode}_accum_{hours}h",
            "horizon_hours": hours,
            "horizon_label": f"{hours}h",
            "grid": None,
        }

    monkeypatch.setattr(dashboard_state, "_accumulation_raster", fake_raster)
    monkeypatch.setattr(
        dashboard_state.DashboardState,
        "_rebuild_map",
        lambda self, **kwargs: None,
    )
    forecast_grid = tmp_path / "raw_forecast_grid.nc"
    forecast_grid.touch()
    from mgb_ops.assets.scenario_cache import ScenarioCache
    monkeypatch.setattr(
        dashboard_state,
        "discover_latest_scenario_caches",
        lambda cache_dir: (
            ScenarioCache(
                "raw:asset", "Raw", "raw", tmp_path / "raw.nc",
                "ecmwf", "asset", None, forecast_grid
            ),
        ),
    )
    state = dashboard_state.DashboardState(tmp_path)
    state.rainfall_hours = 12
    state.forecast_rainfall_hours = 48
    state.rainfall_mode = "forecast"

    state.apply_rainfall_hours()

    assert calls[-1] == (
        "forecast",
        "raw_forecast_grid.nc",
        48,
    )
    assert state.rainfall_hours == 12
    assert state.selected_raster == "forecast_accum_48h"


def test_failed_rainfall_apply_retains_last_valid_map(
    tmp_path: Path, monkeypatch
) -> None:
    _write_config(tmp_path)
    initialize_history_db(tmp_path / "data" / "history.sqlite")

    def fake_raster(*args, **kwargs):
        if kwargs["rainfall_mode"] == "forecast":
            raise ValueError("incomplete cache")
        return {
            "name": "observed_accum_24h",
            "horizon_hours": 24,
            "horizon_label": "24h",
            "grid": None,
        }

    monkeypatch.setattr(dashboard_state, "_accumulation_raster", fake_raster)
    monkeypatch.setattr(
        dashboard_state.DashboardState,
        "_rebuild_map",
        lambda self, **kwargs: None,
    )
    state = dashboard_state.DashboardState(tmp_path)
    previous_rasters = state.accumulation_rasters
    previous_selection = state.selected_raster
    state.rainfall_mode = "forecast"

    state.apply_rainfall_hours()

    assert state.accumulation_rasters is previous_rasters
    assert state.selected_raster == previous_selection
    assert any("Forecast rainfall map unavailable" in value for value in state.warnings)


def test_changing_rainfall_hours_preserves_station_map_layer(
    tmp_path: Path, monkeypatch
) -> None:
    _write_config(tmp_path)
    initialize_history_db(tmp_path / "data" / "history.sqlite")
    stations = pd.DataFrame(
        [
            {
                "station_id": "ana:1001",
                "status": "ok",
                "kind": "rain",
                "station_name": "Station",
                "provider_code": "ana",
                "station_code": "1001",
                "lon": -51.5,
                "lat": -30.5,
            }
        ]
    )

    def fake_raster(*args, **kwargs):
        hours = args[6]
        return {
            "name": f"accum_{hours}h",
            "horizon_hours": hours,
            "horizon_label": f"{hours}h",
            "grid": PrecipitationGrid(
                values=np.ones((2, 2)),
                latitudes=np.array([-31.0, -30.0]),
                longitudes=np.array([-52.0, -51.0]),
                bounds=(-52.5, -31.5, -50.5, -29.5),
                start_time=pd.Timestamp("2026-01-01"),
                end_time=pd.Timestamp("2026-01-02"),
                source="test",
            ),
        }

    monkeypatch.setattr(dashboard_state, "_station_catalog", lambda *args: stations)
    monkeypatch.setattr(dashboard_state, "_accumulation_raster", fake_raster)
    monkeypatch.setattr(
        dashboard_state,
        "_mini_segment_paths",
        lambda *args: pd.DataFrame(
            [{"mini_id": 7, "path": [[-52.0, -31.0], [-51.0, -30.0]]}]
        ),
    )
    state = dashboard_state.DashboardState(tmp_path)

    for hours in (48, 72):
        state.rainfall_hours = hours
        state.apply_rainfall_hours()
        layer_ids = [layer["id"] for layer in state.map_artifacts.spec["layers"]]
        assert f"rainfall-raster:accum_{hours}h" in layer_ids
        assert "stations" in layer_ids
        assert state.map_artifacts.pick_lookups["stations"] == (
            dashboard_map.MapSelection(station_id="ana:1001"),
        )
        assert next(
            layer for layer in state.map_artifacts.spec["layers"]
            if layer["id"] == "stations"
        )["@@type"] == "GeoJsonLayer"
        assert "mini-segments" in layer_ids
        assert state.stations["station_id"].tolist() == ["ana:1001"]

    state.refresh()
    assert "stations" in [
        layer["id"] for layer in state.map_artifacts.spec["layers"]
    ]


def test_rainfall_hours_parameter_rejects_non_positive_values(tmp_path: Path) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    with pytest.raises(ValueError, match="at least 1"):
        state.rainfall_hours = 0



def test_river_click_populates_draft_basin_without_replacing_applied_basin(tmp_path: Path) -> None:
    state = dashboard_state.DashboardState(tmp_path)
    state.applied_basin_mini_id = 5
    state.map_artifacts = dashboard_map.DeckGLArtifacts(
        spec={"layers": []},
        raster_lookups={},
        pick_lookups={"mini-segments": (dashboard_map.MapSelection(mini_id=7),)},
        tooltips={},
    )

    state.handle_map_click({"layer": "mini-segments", "index": 0})

    assert state.mini_id == 7
    assert state.draft_basin_mini == "7"
    assert state.applied_basin_mini_id == 5



def test_state_uses_model_output_window_when_reference_time_is_now(tmp_path: Path, monkeypatch) -> None:
    from mgb_ops.assets.types import AnalysisWindow

    _write_config(tmp_path)
    model_path = tmp_path / "data" / "processed" / "model_outputs.nc"
    model_path.parent.mkdir(parents=True)
    model_path.touch()
    model_window = AnalysisWindow(
        start_time=pd.Timestamp("2026-07-05T00:00:00").to_pydatetime(),
        cutoff_time=pd.Timestamp("2026-07-20T09:00:00").to_pydatetime(),
        forecast_end_exclusive=pd.Timestamp("2026-07-31T00:00:00").to_pydatetime(),
    )
    monkeypatch.setattr(
        dashboard_state,
        "validate_model_outputs_netcdf",
        lambda *args, **kwargs: {"window": model_window, "variables": ("flow",)},
    )
    monkeypatch.setattr(dashboard_state.DashboardState, "refresh", lambda self: None)

    state = dashboard_state.DashboardState(tmp_path)

    assert state.window == model_window


def test_state_defaults_to_raw_scenario_and_zero_comparison(
    tmp_path: Path, monkeypatch
) -> None:
    from mgb_ops.assets.scenario_cache import ScenarioCache

    zero_path = tmp_path / "zero.nc"
    raw_path = tmp_path / "raw.nc"
    caches = (
        ScenarioCache("zero", "Zero", "zero", zero_path, None, None, None),
        ScenarioCache(
            "raw:asset", "Raw asset", "raw", raw_path, "ecmwf", "asset", None
        ),
    )
    monkeypatch.setattr(
        dashboard_state,
        "discover_latest_scenario_caches",
        lambda cache_dir: caches,
    )
    monkeypatch.setattr(dashboard_state.DashboardState, "refresh", lambda self: None)

    state = dashboard_state.DashboardState(tmp_path)

    assert state.scenario_id == "raw:asset"
    assert state.model_path == raw_path
    assert state.comparison_scenario_ids == ["raw:asset", "zero"]
