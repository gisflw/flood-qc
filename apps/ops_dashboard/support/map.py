from __future__ import annotations

import hashlib
import json
import re
from dataclasses import dataclass
from pathlib import Path
from typing import MutableMapping, Optional

import branca
import folium
import folium.elements
import folium.plugins
import numpy as np
import pandas as pd
from folium.raster_layers import ImageOverlay

from mgb_ops.analysis.spatial import PrecipitationGrid


NO_DATA_COLOR = "#e64980"
DATA_ISSUE_COLOR = "#f08c00"
KIND_COLORS = {"level": "#0b7285", "rain": "#364fc7", "mixed": "#2b8a3e", "no_data": "#868e96"}
CLICK_TOKEN_PATTERN = re.compile(r"(POSTO\|[A-Za-z0-9_.:-]+|MINI\|\d+)")
MAP_RETURNED_OBJECTS = ("last_object_clicked_tooltip",)
BLUES = np.array(
    [
        (239, 243, 255),
        (198, 219, 239),
        (158, 202, 225),
        (107, 174, 214),
        (66, 146, 198),
        (33, 113, 181),
        (8, 81, 156),
        (8, 48, 107),
    ],
    dtype=float,
) / 255.0


@dataclass(frozen=True, slots=True)
class RasterLegendSpec:
    caption: str
    vmin: float
    vmax: float


def build_file_version(path: Path) -> str:
    target = Path(path)
    if not target.exists():
        return f"{target.as_posix()}:missing"
    stat = target.stat()
    return f"{target.as_posix()}:{stat.st_mtime_ns}:{stat.st_size}"


def build_sqlite_version(path: Path) -> str:
    target = Path(path)
    return "|".join(build_file_version(candidate) for candidate in (target, Path(f"{target}-wal")))


def build_map_cache_key(
    *,
    selected_layer_name: str | None,
    opacity: float,
    history_version: str,
    spatial_version: str,
    raster_version: str,
    station_id: str | None = None,
    mini_id: int | None = None,
) -> str:
    payload = {
        "selected_layer_name": selected_layer_name,
        "opacity": round(float(opacity), 4),
        "history_version": history_version,
        "spatial_version": spatial_version,
        "raster_version": raster_version,
    }
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode("utf-8")).hexdigest()


def parse_click_token(tooltip_value: Optional[str]) -> Optional[str]:
    if not tooltip_value:
        return None
    match = CLICK_TOKEN_PATTERN.search(str(tooltip_value))
    if not match:
        return None
    return match.group(1)


def update_selection_from_click_token(click_token: Optional[str], session_state: MutableMapping[str, object]) -> bool:
    if not click_token:
        return False

    changed = False
    if click_token.startswith("POSTO|"):
        station_id = click_token.split("|", 1)[1].strip()
        if session_state.get("station_id") != station_id:
            session_state["station_id"] = station_id
            changed = True

    if click_token.startswith("MINI|"):
        mini_id = int(click_token.split("|", 1)[1].strip())
        if session_state.get("mini_id") != mini_id:
            session_state["mini_id"] = mini_id
            changed = True

    return changed


def color_ramp_factory(vmin: float, vmax: float, alpha: float):
    stops = np.linspace(0, 1, len(BLUES))

    def cmap(val: float):
        if val is None or np.isnan(val):
            return (0, 0, 0, 0)
        span = vmax - vmin
        t = 0.5 if span <= 0 else (val - vmin) / span
        t = float(np.clip(t, 0.0, 1.0))
        r = float(np.interp(t, stops, BLUES[:, 0]))
        g = float(np.interp(t, stops, BLUES[:, 1]))
        b = float(np.interp(t, stops, BLUES[:, 2]))
        return (r, g, b, alpha)

    return cmap


def add_legend(fmap: folium.Map, vmin: float, vmax: float, *, horizon_label: Optional[str]) -> None:
    colors_hex = ["#{:02x}{:02x}{:02x}".format(int(r * 255), int(g * 255), int(b * 255)) for r, g, b in BLUES]
    colormap = branca.colormap.LinearColormap(colors=colors_hex, vmin=float(vmin), vmax=float(vmax))
    horizon_text = horizon_label or "selected period"
    colormap.caption = f"Accumulated rainfall over the last {horizon_text}"
    colormap.add_to(fmap)


def build_raster_legend_spec(data: np.ndarray, *, caption: str) -> RasterLegendSpec | None:
    finite_values = np.asarray(data, dtype=np.float64)
    finite_values = finite_values[np.isfinite(finite_values)]
    if finite_values.size == 0:
        return None

    vmin, vmax = np.nanpercentile(finite_values, [5, 95])
    return RasterLegendSpec(caption=caption, vmin=float(vmin), vmax=float(vmax))


def build_raster_legend_html(spec: RasterLegendSpec) -> str:
    colors_hex = ["#{:02x}{:02x}{:02x}".format(int(r * 255), int(g * 255), int(b * 255)) for r, g, b in BLUES]
    gradient = ", ".join(
        f"{color} {int(round(index * 100 / max(1, len(colors_hex) - 1)))}%"
        for index, color in enumerate(colors_hex)
    )
    return (
        "<div style=\"padding:0.35rem 0 0.15rem 0;\">"
        f"<div style=\"font-size:0.9rem;font-weight:600;margin-bottom:0.35rem;\">{spec.caption}</div>"
        f"<div style=\"height:12px;border-radius:999px;background:linear-gradient(90deg, {gradient});\"></div>"
        "<div style=\"display:flex;justify-content:space-between;font-size:0.8rem;color:#495057;margin-top:0.25rem;\">"
        f"<span>{spec.vmin:.1f} mm</span>"
        f"<span>{spec.vmax:.1f} mm</span>"
        "</div>"
        "</div>"
    )


class RasterClickPopup(branca.element.MacroElement):
    def __init__(self, data: np.ndarray, bounds: tuple[float, float, float, float], layer_name: str) -> None:
        super().__init__()
        west, south, east, north = bounds
        payload = np.where(np.isnan(data), None, data).tolist()
        self._name = "RasterClickPopup"
        self.data = json.dumps(payload)
        self.south = south
        self.west = west
        self.north = north
        self.east = east
        self.layer_name = json.dumps(layer_name)
        self._template = branca.element.Template(
            """
            {% macro script(this, kwargs) %}
            var rasterData = {{this.data}};
            var rasterBounds = {south: {{this.south}}, west: {{this.west}}, north: {{this.north}}, east: {{this.east}}};
            (function attachRasterClick() {
                var mapRef = {{this._parent.get_name()}};
                if (!mapRef) {
                    setTimeout(attachRasterClick, 50);
                    return;
                }
                if (mapRef._rasterClickHandler) {
                    mapRef.off('click', mapRef._rasterClickHandler);
                }
                mapRef._rasterClickHandler = function(e) {
                    var lat = e.latlng.lat;
                    var lng = e.latlng.lng;
                    if (lat < rasterBounds.south || lat > rasterBounds.north || lng < rasterBounds.west || lng > rasterBounds.east) {
                        return;
                    }
                    var rows = rasterData.length;
                    var cols = rasterData[0].length;
                    var row = Math.floor((rasterBounds.north - lat) / (rasterBounds.north - rasterBounds.south) * (rows - 1));
                    var col = Math.floor((lng - rasterBounds.west) / (rasterBounds.east - rasterBounds.west) * (cols - 1));
                    var val = rasterData[row][col];
                    if (val === null || isNaN(val) || val <= 0) {
                        return;
                    }
                    var layerName = {{this.layer_name}};
                    var html = `<b>${layerName}</b><br>Lat: ${lat.toFixed(4)}<br>Lon: ${lng.toFixed(4)}<br>Valor: ${val.toFixed(1)} mm`;
                    L.popup().setLatLng(e.latlng).setContent(html).openOn(mapRef);
                };
                mapRef.on('click', mapRef._rasterClickHandler);
            })();
            {% endmacro %}
            """
        )


def add_raster_overlay(
    fmap: folium.Map,
    *,
    data: np.ndarray,
    bounds: tuple[float, float, float, float],
    layer_name: str,
    opacity: float,
    horizon_label: Optional[str] = None,
    feature_group_name: Optional[str] = None,
    show: bool = True,
    include_legend: bool = True,
) -> bool:
    legend_spec = build_raster_legend_spec(np.asarray(data, dtype=np.float64), caption=horizon_label or layer_name)
    if legend_spec is None:
        return False

    west, south, east, north = bounds
    vmin, vmax = legend_spec.vmin, legend_spec.vmax
    overlay = ImageOverlay(
        name=layer_name,
        image=np.asarray(data, dtype=np.float64),
        bounds=[[south, west], [north, east]],
        opacity=float(opacity),
        interactive=False,
        cross_origin=False,
        mercator_project=False,
        colormap=color_ramp_factory(float(vmin), float(vmax), float(opacity)),
    )
    raster_group = folium.FeatureGroup(name=feature_group_name or layer_name, show=show)
    overlay.add_to(raster_group)
    raster_group.add_to(fmap)
    if include_legend:
        add_legend(fmap, float(vmin), float(vmax), horizon_label=horizon_label or layer_name)
    RasterClickPopup(np.asarray(data, dtype=np.float64), bounds, layer_name).add_to(fmap)
    return True


def build_ops_map(
    selected_layer_name: Optional[str],
    opacity: float,
    stations: pd.DataFrame,
    segments_geojson: Optional[dict],
    catchments_geojson: Optional[dict],
    raster_catalog: dict[str, dict[str, object]],
) -> folium.Map:
    center = [stations["lat"].mean(), stations["lon"].mean()] if not stations.empty else [-29.7, -53.3]
    fmap = folium.Map(location=center, zoom_start=7, tiles="CartoDB Positron", control_scale=True)

    if selected_layer_name:
        meta = raster_catalog.get(selected_layer_name)
        if meta:
            grid = meta.get("grid")
            if not isinstance(grid, PrecipitationGrid):
                raise TypeError("Rainfall map catalog entries must contain a PrecipitationGrid.")
            add_raster_overlay(
                fmap,
                data=grid.values,
                bounds=grid.bounds,
                layer_name=f"Raster {meta['horizon_label']}",
                opacity=opacity,
                horizon_label=str(meta["horizon_label"]),
                feature_group_name="Accumulated rainfall",
                show=True,
            )

    if catchments_geojson and catchments_geojson.get("features"):
        catchments_layer = folium.FeatureGroup(name="MGB mini catchments", show=True)
        folium.GeoJson(
            catchments_geojson,
            style_function=lambda _: {
                "color": "#74c0fc", "weight": 0.7, "opacity": 0.65,
                "fillColor": "#d0ebff", "fillOpacity": 0.08,
            },
            highlight_function=lambda _: {"color": "#0b7285", "weight": 1.8, "fillOpacity": 0.2},
            tooltip=folium.GeoJsonTooltip(fields=["click_id"], aliases=[""], labels=False, sticky=False),
            name="MGB mini catchments",
        ).add_to(catchments_layer)
        catchments_layer.add_to(fmap)

    if segments_geojson and segments_geojson.get("features"):
        rivers_layer = folium.FeatureGroup(name="MGB rivers", show=True)
        folium.GeoJson(
            segments_geojson,
            style_function=lambda _: {"color": "#1971c2", "weight": 1.2, "opacity": 0.45},
            highlight_function=lambda _: {"color": "#0b7285", "weight": 2.2, "opacity": 0.9},
            tooltip=folium.GeoJsonTooltip(fields=["click_id"], aliases=[""], labels=False, sticky=False),
            name="MGB rivers",
        ).add_to(rivers_layer)
        rivers_layer.add_to(fmap)

    station_layer = folium.FeatureGroup(name="Stations with data", show=True)
    no_data_layer = folium.FeatureGroup(name="Stations without data", show=True)

    for row in stations.itertuples():
        tooltip = f"POSTO|{row.station_id} - {row.station_name} ({str(row.provider_code).upper()} {row.station_code})"
        status = getattr(row, "status", "no_data")

        if status == "no_data":
            folium.CircleMarker(
                location=[row.lat, row.lon],
                radius=6,
                color=NO_DATA_COLOR,
                fill=True,
                fill_color=NO_DATA_COLOR,
                weight=1,
                fill_opacity=0.55,
                tooltip=tooltip,
                bubbling_mouse_events=False,
            ).add_to(no_data_layer)
            continue

        marker_color = KIND_COLORS.get(getattr(row, "kind", "no_data"), "#364fc7")
        if status == "data_issue":
            marker_color = DATA_ISSUE_COLOR

        folium.CircleMarker(
            location=[row.lat, row.lon],
            radius=6,
            color=marker_color,
            fill=True,
            fill_color=marker_color,
            weight=1,
            fill_opacity=0.9 if status == "ok" else 0.75,
            tooltip=tooltip,
            bubbling_mouse_events=False,
        ).add_to(station_layer)

    station_layer.add_to(fmap)
    no_data_layer.add_to(fmap)
    folium.LayerControl(collapsed=False).add_to(fmap)
    return fmap
