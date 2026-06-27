from __future__ import annotations

import folium
import numpy as np
import pytest

from apps.ops_dashboard.support import map as ops_dashboard_map


def test_build_map_cache_key_ignores_series_selection() -> None:
    key_a = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        spatial_version="spatial:v1",
        raster_version="raster:v1",
        station_id=1001,
        mini_id=10,
    )
    key_b = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        spatial_version="spatial:v1",
        raster_version="raster:v1",
        station_id=2002,
        mini_id=20,
    )

    assert key_a == key_b


def test_build_map_cache_key_changes_with_visual_state() -> None:
    base_key = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        spatial_version="spatial:v1",
        raster_version="raster:v1",
    )
    other_raster = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_72h",
        opacity=0.6,
        history_version="history:v1",
        spatial_version="spatial:v1",
        raster_version="raster:v2",
    )
    other_opacity = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.75,
        history_version="history:v1",
        spatial_version="spatial:v1",
        raster_version="raster:v1",
    )

    assert base_key != other_raster
    assert base_key != other_opacity


def test_parse_click_token_recognizes_station_and_mini() -> None:
    assert ops_dashboard_map.parse_click_token("POSTO|1001 - TESTE") == "POSTO|1001"
    assert ops_dashboard_map.parse_click_token("POSTO|ana:74100000 - TESTE") == "POSTO|ana:74100000"
    assert ops_dashboard_map.parse_click_token("MINI|539") == "MINI|539"
    assert ops_dashboard_map.parse_click_token("sem token") is None


def test_update_selection_from_click_token_updates_only_changed_values() -> None:
    session_state: dict[str, object] = {"station_id": "1001", "mini_id": 10}

    changed = ops_dashboard_map.update_selection_from_click_token("POSTO|1001", session_state)
    assert changed is False
    assert session_state == {"station_id": "1001", "mini_id": 10}

    changed = ops_dashboard_map.update_selection_from_click_token("MINI|20", session_state)
    assert changed is True
    assert session_state == {"station_id": "1001", "mini_id": 20}


def test_north_first_raster_values_flips_ascending_latitudes_once() -> None:
    values = np.array([[1.0, 2.0], [3.0, 4.0]])

    normalized = ops_dashboard_map.north_first_raster_values(values, np.array([-31.0, -30.0]))

    np.testing.assert_array_equal(normalized, np.array([[3.0, 4.0], [1.0, 2.0]]))


def test_north_first_raster_values_leaves_descending_latitudes_unchanged() -> None:
    values = np.array([[1.0, 2.0], [3.0, 4.0]])

    normalized = ops_dashboard_map.north_first_raster_values(values, np.array([-30.0, -31.0]))

    np.testing.assert_array_equal(normalized, values)


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


def test_add_raster_overlay_shares_north_first_values_with_click_popup(monkeypatch) -> None:
    captured: dict[str, np.ndarray] = {}

    class CapturingOverlay:
        def __init__(self, *, image, **kwargs):
            captured["overlay"] = image

        def add_to(self, parent):
            return self

    original_popup = ops_dashboard_map.RasterClickPopup

    class CapturingPopup(original_popup):
        def __init__(self, data, bounds, layer_name):
            captured["popup"] = data
            super().__init__(data, bounds, layer_name)

    monkeypatch.setattr(ops_dashboard_map, "ImageOverlay", CapturingOverlay)
    monkeypatch.setattr(ops_dashboard_map, "RasterClickPopup", CapturingPopup)
    values = np.array([[1.0, 2.0], [3.0, 4.0]])

    assert ops_dashboard_map.add_raster_overlay(
        folium.Map(),
        data=values,
        latitudes=np.array([-31.0, -30.0]),
        bounds=(-52.0, -31.0, -51.0, -30.0),
        layer_name="rain",
        opacity=0.7,
        include_legend=False,
    )
    np.testing.assert_array_equal(captured["overlay"], captured["popup"])
    np.testing.assert_array_equal(captured["overlay"], np.flipud(values))
