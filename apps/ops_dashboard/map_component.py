from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable

import branca
import folium
import folium.elements
import folium.plugins
from streamlit_folium import _component_func
from streamlit_folium import _get_header, _get_html, _get_map_string, generate_js_hash, get_full_id

from mgb_ops.reporting.ops_dashboard_map import MAP_RETURNED_OBJECTS


@dataclass(frozen=True, slots=True)
class MapRenderArtifacts:
    script: str
    header: str
    html: str
    element_id: str
    key: str
    defaults: dict[str, object]
    css_links: tuple[str, ...]
    js_links: tuple[str, ...]


def _bounds_to_dict(bounds_list: list[list[float]]) -> dict[str, dict[str, float | None]]:
    southwest, northeast = bounds_list
    return {
        "_southWest": {"lat": southwest[0], "lng": southwest[1]},
        "_northEast": {"lat": northeast[0], "lng": northeast[1]},
    }


def _walk(fig):
    if isinstance(fig, branca.colormap.ColorMap):
        yield fig
    if isinstance(fig, folium.plugins.DualMap):
        yield from _walk(fig.m1)
        yield from _walk(fig.m2)
    if isinstance(fig, folium.elements.JSCSSMixin):
        yield fig
    if hasattr(fig, "_children"):
        for child in fig._children.values():
            yield from _walk(child)


def build_map_render_artifacts(
    fig: folium.MacroElement,
    *,
    component_key: str = "ops-dashboard-map",
    returned_objects: Iterable[str] = MAP_RETURNED_OBJECTS,
) -> MapRenderArtifacts:
    folium_map: folium.Map = fig  # type: ignore[assignment]
    if isinstance(fig, folium.plugins.DualMap):
        fig.render()
    else:
        fig.get_root().render()

    if not isinstance(fig, (folium.Map, folium.plugins.DualMap)):
        folium_map = next(iter(fig._children.values()))

    html = _get_html(folium_map)
    header = _get_header(folium_map)
    leaflet = _get_map_string(folium_map)
    map_id = get_full_id(folium_map)

    try:
        bounds = folium_map.get_bounds()
    except AttributeError:
        bounds = [[None, None], [None, None]]

    defaults_all = {
        "last_clicked": None,
        "last_object_clicked": None,
        "last_object_clicked_tooltip": None,
        "last_object_clicked_popup": None,
        "all_drawings": None,
        "last_active_drawing": None,
        "bounds": _bounds_to_dict(bounds),
        "zoom": folium_map.options.get("zoom") if hasattr(folium_map, "options") else {},
        "last_circle_radius": None,
        "last_circle_polygon": None,
        "selected_layers": None,
    }
    returned = tuple(returned_objects)
    defaults = {key: value for key, value in defaults_all.items() if key in returned}

    css_links: list[str] = []
    js_links: list[str] = []
    for elem in _walk(folium_map):
        if isinstance(elem, branca.colormap.ColorMap):
            js_links.insert(0, "https://cdnjs.cloudflare.com/ajax/libs/d3/3.5.5/d3.min.js")
            js_links.insert(0, "https://d3js.org/d3.v4.min.js")
        css_links.extend([href for _, href in getattr(elem, "default_css", [])])
        js_links.extend([src for _, src in getattr(elem, "default_js", [])])

    hash_key = generate_js_hash(leaflet, component_key, return_on_hover=False)
    return MapRenderArtifacts(
        script=leaflet,
        header=header,
        html=html,
        element_id=map_id,
        key=hash_key,
        defaults=defaults,
        css_links=tuple(css_links),
        js_links=tuple(js_links),
    )


def render_map_component(
    artifacts: MapRenderArtifacts,
    *,
    height: int = 700,
    width: int | None = 500,
    use_container_width: bool = False,
) -> dict[str, object]:
    if use_container_width:
        width = None

    return _component_func(
        script=artifacts.script,
        header=artifacts.header,
        html=artifacts.html,
        id=artifacts.element_id,
        key=artifacts.key,
        height=height,
        width=width,
        returned_objects=MAP_RETURNED_OBJECTS,
        default=artifacts.defaults,
        zoom=None,
        center=None,
        feature_group=None,
        return_on_hover=False,
        layer_control=None,
        pixelated=False,
        css_links=list(artifacts.css_links),
        js_links=list(artifacts.js_links),
        on_change=None,
    )
