from __future__ import annotations

import json

import numpy as np
import pandas as pd
import pytest

from apps.ops_dashboard.support import map as ops_dashboard_map
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
        opacity=0.6,
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
    catchments = {
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "properties": {"mini_id": np.int64(7)},
                "geometry": {
                    "type": "Polygon",
                    "coordinates": [[[-52, -31], [-51, -31], [-51, -30], [-52, -31]]],
                },
            }
        ],
    }
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
        catchments,
        {"accum_24h": {"grid": _grid(), "horizon_label": "24h"}},
    )

    layer_ids = [layer["id"] for layer in artifacts.spec["layers"]]
    assert layer_ids == [
        "rainfall-raster:accum_24h",
        "mini-catchments",
        "mini-rivers",
        "stations",
    ]
    assert artifacts.spec["layers"][0]["image"].startswith("data:image/png;base64,")
    json.dumps(artifacts.spec)
    assert set(artifacts.raster_lookups) == {"rainfall-raster:accum_24h"}


def test_decode_station_mini_and_raster_clicks() -> None:
    station = ops_dashboard_map.decode_click_state(
        {"layer": {"id": "stations"}, "object": {"station_id": 1001}}
    )
    mini = ops_dashboard_map.decode_click_state(
        {
            "layer": "mini-catchments",
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
        None, 0.5, pd.DataFrame(), None, None, {}
    )
    assert artifacts.spec["layers"] == []
    assert artifacts.raster_lookups == {}


def test_raster_layer_keeps_missing_values_transparent() -> None:
    layer, _, _ = ops_dashboard_map.build_raster_layer(
        _grid(np.array([[1.0, np.nan], [np.inf, 4.0]])),
        layer_id="rain",
        layer_name="Rain",
        opacity=0.5,
    )
    assert layer is not None
    assert layer["image"].startswith("data:image/png;base64,")


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
