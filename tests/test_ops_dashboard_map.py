from __future__ import annotations

from mgb_ops.reporting import ops_dashboard_map


def test_build_map_cache_key_ignores_series_selection() -> None:
    key_a = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        rivers_version="rivers:v1",
        raster_version="raster:v1",
        station_uid=1001,
        mini_id=10,
    )
    key_b = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        rivers_version="rivers:v1",
        raster_version="raster:v1",
        station_uid=2002,
        mini_id=20,
    )

    assert key_a == key_b


def test_build_map_cache_key_changes_with_visual_state() -> None:
    base_key = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.6,
        history_version="history:v1",
        rivers_version="rivers:v1",
        raster_version="raster:v1",
    )
    other_raster = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_72h",
        opacity=0.6,
        history_version="history:v1",
        rivers_version="rivers:v1",
        raster_version="raster:v2",
    )
    other_opacity = ops_dashboard_map.build_map_cache_key(
        selected_layer_name="accum_24h",
        opacity=0.75,
        history_version="history:v1",
        rivers_version="rivers:v1",
        raster_version="raster:v1",
    )

    assert base_key != other_raster
    assert base_key != other_opacity


def test_parse_click_token_recognizes_station_and_mini() -> None:
    assert ops_dashboard_map.parse_click_token("POSTO|1001 - TESTE") == "POSTO|1001"
    assert ops_dashboard_map.parse_click_token("MINI|539") == "MINI|539"
    assert ops_dashboard_map.parse_click_token("sem token") is None


def test_update_selection_from_click_token_updates_only_changed_values() -> None:
    session_state: dict[str, object] = {"station_uid": 1001, "mini_id": 10}

    changed = ops_dashboard_map.update_selection_from_click_token("POSTO|1001", session_state)
    assert changed is False
    assert session_state == {"station_uid": 1001, "mini_id": 10}

    changed = ops_dashboard_map.update_selection_from_click_token("MINI|20", session_state)
    assert changed is True
    assert session_state == {"station_uid": 1001, "mini_id": 20}
