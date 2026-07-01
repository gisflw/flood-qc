from __future__ import annotations

import json

import numpy as np
import pandas as pd
import panel as pn
import pytest

from apps.ops_dashboard.services import deckgl as ops_dashboard_map
from mgb_ops.analysis.spatial import PrecipitationGrid


def _grid(values: np.ndarray | None = None) -> PrecipitationGrid:
    return PrecipitationGrid(
        values=np.array([[1.0, 2.0], [3.0, 4.0]]) if values is None else values,
        latitudes=np.array([-31.0, -30.0]),
        longitudes=np.array([-52.0, -51.0]),
        bounds=(-52.5, -31.5, -50.5, -29.5),
        start_time=pd.Timestamp("2026-01-01"),
        end_time=pd.Timestamp("2026-01-02"),
        source="test",
    )


def test_build_map_cache_key_ignores_series_selection() -> None:
    kwargs = dict(
        selected_layer_name="accum_24h",
        history_version="history-v1",
        spatial_version="spatial-v1",
        raster_version="raster-v1",
    )
    assert ops_dashboard_map.build_map_cache_key(
        **kwargs, station_id="1", mini_id=2
    ) == ops_dashboard_map.build_map_cache_key(
        **kwargs, station_id="9", mini_id=99
    )


def test_deckgl_layers_are_separate_and_json_compatible() -> None:
    stations = pd.DataFrame(
        [
            {
                "station_id": "1001",
                "station_name": "Test",
                "provider_code": "ana",
                "station_code": "A1",
                "lat": -30.0,
                "lon": -52.0,
                "kind": "rain",
                "status": "ok",
            }
        ]
    )
    rivers = {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"mini_id": 7},
                "geometry": {"type": "LineString", "coordinates": [[-52, -31], [-51, -30]]},
            }
        ],
    }
    artifacts = ops_dashboard_map.build_ops_map(
        "accum_24h",
        0.6,
        stations,
        rivers,
        {"accum_24h": {"grid": _grid(), "horizon_label": "24h"}},
    )

    layer_ids = [layer["id"] for layer in artifacts.spec["layers"]]
    assert layer_ids == [
        "rainfall-raster:accum_24h",
        "mini-segments",
        "stations",
    ]
    assert artifacts.spec["layers"][0]["image"].startswith("data:image/png;base64,")
    assert artifacts.spec["layers"][0]["opacity"] == 0.6
    segment_layer = next(
        layer for layer in artifacts.spec["layers"] if layer["id"] == "mini-segments"
    )
    station_layer = next(
        layer for layer in artifacts.spec["layers"] if layer["id"] == "stations"
    )
    assert segment_layer["lineWidthUnits"] == "pixels"
    assert station_layer["@@type"] == "GeoJsonLayer"
    assert station_layer["getFillColor"] == "@@=properties.color"
    assert station_layer["getPointRadius"] == 4500
    assert station_layer["pointRadiusMinPixels"] == 5
    assert station_layer["pointRadiusMaxPixels"] == 10
    assert station_layer["data"] == {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [-52.0, -30.0],
                },
                "properties": {
                    "station_id": "1001",
                    "station_name": "Test",
                    "provider_code": "ANA",
                    "station_code": "A1",
                    "color": ops_dashboard_map.KIND_COLORS["rain"],
                    "status": "ok",
                },
            }
        ],
    }
    assert "properties.station_name" in artifacts.tooltips["stations"]["html"]
    json.dumps(artifacts.spec)
    assert set(artifacts.raster_lookups) == {"rainfall-raster:accum_24h"}
    assert set(artifacts.tooltips) == {
        "stations",
        "mini-segments",
    }
    assert "tooltip" not in artifacts.spec


def test_build_map_adds_non_pickable_dissolved_basin_overlay() -> None:
    basin = {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"outlet_mini_id": 7, "mini_count": 3},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[-52, -31], [-51, -31], [-51, -30], [-52, -31]]],
                },
            }
        ],
    }

    artifacts = ops_dashboard_map.build_ops_map(
        None,
        0.4,
        pd.DataFrame(),
        None,
        {},
        basin,
    )

    layer = artifacts.spec["layers"][0]
    assert layer["id"] == "selected-basin"
    assert layer["pickable"] is False
    assert layer["filled"] is True
    assert "selected-basin" not in artifacts.pick_lookups


def test_geojson_overlays_do_not_create_panel_data_sources() -> None:
    stations = pd.DataFrame(
        [
            {
                "station_id": "1001",
                "station_name": "Test",
                "provider_code": "ana",
                "station_code": "A1",
                "lat": -30.0,
                "lon": -52.0,
                "kind": "rain",
                "status": "ok",
            }
        ]
    )
    artifacts = ops_dashboard_map.build_ops_map(None, 0.4, stations, None, {})
    pane = pn.pane.DeckGL(artifacts.spec)
    model = pane.get_root()

    assert model.data_sources == []
    assert model.layers[0]["data"]["features"][0]["properties"]["station_id"] == "1001"

    replacement = ops_dashboard_map.build_ops_map(
        "accum_48h",
        0.4,
        stations,
        None,
        {"accum_48h": {"grid": _grid(), "horizon_label": "48h"}},
    )
    pane.object = replacement.spec

    assert model.data_sources == []
    station_layer = next(layer for layer in model.layers if layer["id"] == "stations")
    assert station_layer["data"]["features"][0]["properties"]["station_id"] == "1001"


def test_decode_station_mini_and_raster_clicks() -> None:
    station = ops_dashboard_map.decode_click_state(
        {
            "layer": {"id": "stations"},
            "object": {"properties": {"station_id": 1001}},
        }
    )
    mini = ops_dashboard_map.decode_click_state(
        {
            "layer": "mini-segments",
            "object": {"properties": {"mini_id": "7"}},
        }
    )
    raster = ops_dashboard_map.decode_click_state(
        {
            "layer": {"id": "rainfall-raster:accum_24h"},
            "coordinate": [-51.1, -30.1],
        }
    )

    assert station.station_id == "1001"
    assert mini.mini_id == 7
    assert raster.raster_layer_id == "rainfall-raster:accum_24h"
    assert raster.coordinate == (-51.1, -30.1)


def test_decode_panel_index_only_clicks() -> None:
    lookups = {
        "stations": (ops_dashboard_map.MapSelection(station_id="1001"),),
        "mini-segments": (ops_dashboard_map.MapSelection(mini_id=7),),
    }

    station = ops_dashboard_map.decode_click_state(
        {"layer": "stations", "index": 0}, lookups
    )
    mini = ops_dashboard_map.decode_click_state(
        {"layer": "mini-segments", "index": 0}, lookups
    )

    assert station.station_id == "1001"
    assert mini.mini_id == 7


def test_raster_coordinate_lookup_returns_original_array_value() -> None:
    _, lookup, _ = ops_dashboard_map.build_raster_layer(
        _grid(),
        layer_id="rain",
        layer_name="24h",
        opacity=0.7,
    )

    result = ops_dashboard_map.lookup_raster_value(lookup, -51.05, -30.05)

    assert result is not None
    assert (result.row, result.column, result.value) == (1, 1, 4.0)
    assert ops_dashboard_map.lookup_raster_value(lookup, 0, 0) is None


def test_empty_sources_build_an_empty_but_usable_map() -> None:
    artifacts = ops_dashboard_map.build_ops_map(
        None, 0.5, pd.DataFrame(), None, {}
    )
    assert artifacts.spec["layers"] == []
    assert artifacts.raster_lookups == {}
    assert artifacts.pick_lookups == {}


def test_raster_layer_keeps_missing_values_transparent() -> None:
    layer, _, _ = ops_dashboard_map.build_raster_layer(
        _grid(np.array([[1.0, np.nan], [np.inf, 4.0]])),
        layer_id="rain",
        layer_name="Rain",
        opacity=0.5,
    )
    assert layer is not None
    assert layer["image"].startswith("data:image/png;base64,")
    assert layer["opacity"] == 0.5


def test_updating_raster_opacity_preserves_generated_image_and_artifacts() -> None:
    artifacts = ops_dashboard_map.build_ops_map(
        "accum_24h",
        0.6,
        pd.DataFrame(),
        None,
        {"accum_24h": {"grid": _grid(), "horizon_label": "24h"}},
    )
    original_layer = artifacts.spec["layers"][0]

    updated = ops_dashboard_map.update_raster_opacity(artifacts, 0.25)

    assert updated is not None
    assert updated.spec["layers"][0]["opacity"] == 0.25
    assert updated.spec["layers"][0]["image"] is original_layer["image"]
    assert updated.raster_lookups is artifacts.raster_lookups
    assert updated.pick_lookups is artifacts.pick_lookups


def test_north_first_raster_values_flips_ascending_latitudes_once() -> None:
    values = np.array([[1.0, 2.0], [3.0, 4.0]])
    normalized = ops_dashboard_map.north_first_raster_values(
        values, np.array([-31.0, -30.0])
    )
    np.testing.assert_array_equal(normalized, np.flipud(values))


@pytest.mark.parametrize(
    ("latitudes", "message"),
    [
        (np.array([[-30.0, -31.0]]), "one-dimensional"),
        (np.array([-30.0]), "must match raster rows"),
        (np.array([-30.0, -30.0]), "strictly monotonic"),
        (np.array([-30.0, np.nan]), "finite"),
    ],
)
def test_north_first_raster_values_rejects_invalid_latitudes(
    latitudes: np.ndarray, message: str
) -> None:
    with pytest.raises(ValueError, match=message):
        ops_dashboard_map.north_first_raster_values(np.ones((2, 2)), latitudes)
