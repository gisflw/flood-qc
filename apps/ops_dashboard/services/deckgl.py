"""JSON-only DeckGL builders for the operations dashboard."""
from __future__ import annotations

import base64
import hashlib
import json
import struct
import zlib
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping

import numpy as np
import pandas as pd

from mgb_ops.analysis.spatial import PrecipitationGrid


NO_DATA_COLOR = [230, 73, 128, 180]
DATA_ISSUE_COLOR = [240, 140, 0, 210]
KIND_COLORS = {
    "level": [11, 114, 133, 230],
    "rain": [54, 79, 199, 230],
    "mixed": [43, 138, 62, 230],
    "no_data": [134, 142, 150, 180],
}
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
)


@dataclass(frozen=True, slots=True)
class RasterLegendSpec:
    caption: str
    vmin: float
    vmax: float


@dataclass(frozen=True, slots=True)
class RasterLookup:
    """Metadata needed to map a geographic click back to a raster cell."""

    layer_id: str
    layer_name: str
    values: np.ndarray
    latitudes: np.ndarray
    longitudes: np.ndarray
    bounds: tuple[float, float, float, float]


@dataclass(frozen=True, slots=True)
class DeckGLArtifacts:
    spec: dict[str, Any]
    raster_lookups: dict[str, RasterLookup]
    pick_lookups: dict[str, tuple[MapSelection, ...]]
    tooltips: dict[str, dict[str, str]]
    legends: tuple[RasterLegendSpec, ...] = ()


@dataclass(frozen=True, slots=True)
class MapSelection:
    station_id: str | None = None
    mini_id: int | None = None
    raster_layer_id: str | None = None
    coordinate: tuple[float, float] | None = None


@dataclass(frozen=True, slots=True)
class RasterValue:
    layer_name: str
    longitude: float
    latitude: float
    row: int
    column: int
    value: float | None


def build_file_version(path: Path) -> str:
    target = Path(path)
    if not target.exists():
        return f"{target.as_posix()}:missing"
    stat = target.stat()
    return f"{target.as_posix()}:{stat.st_mtime_ns}:{stat.st_size}"


def build_sqlite_version(path: Path) -> str:
    target = Path(path)
    return "|".join(
        build_file_version(candidate) for candidate in (target, Path(f"{target}-wal"))
    )


def build_map_cache_key(
    *,
    selected_layer_name: str | None,
    history_version: str,
    spatial_version: str,
    raster_version: str,
    station_id: str | None = None,
    mini_id: int | None = None,
) -> str:
    # Selections update summaries/charts and intentionally do not rebuild the map.
    del station_id, mini_id
    payload = {
        "selected_layer_name": selected_layer_name,
        "history_version": history_version,
        "spatial_version": spatial_version,
        "raster_version": raster_version,
    }
    return hashlib.sha256(json.dumps(payload, sort_keys=True).encode()).hexdigest()


def north_first_raster_values(data: np.ndarray, latitudes: np.ndarray) -> np.ndarray:
    values = np.asarray(data, dtype=np.float64)
    latitude_values = np.asarray(latitudes, dtype=np.float64)
    if values.ndim != 2:
        raise ValueError("Raster values must be a two-dimensional array.")
    if latitude_values.ndim != 1:
        raise ValueError("Raster latitudes must be a one-dimensional array.")
    if latitude_values.size != values.shape[0]:
        raise ValueError(
            f"Raster latitude length ({latitude_values.size}) must match raster rows ({values.shape[0]})."
        )
    if not np.all(np.isfinite(latitude_values)):
        raise ValueError("Raster latitudes must contain only finite values.")
    deltas = np.diff(latitude_values)
    if not (np.all(deltas > 0) or np.all(deltas < 0)):
        raise ValueError("Raster latitudes must be strictly monotonic.")
    return np.flipud(values) if np.all(deltas > 0) else values


def build_raster_legend_spec(data: np.ndarray, *, caption: str) -> RasterLegendSpec | None:
    finite = np.asarray(data, dtype=np.float64)
    finite = finite[np.isfinite(finite)]
    if finite.size == 0:
        return None
    vmin, vmax = np.nanpercentile(finite, [5, 95])
    return RasterLegendSpec(caption=caption, vmin=float(vmin), vmax=float(vmax))


def build_raster_legend_html(spec: RasterLegendSpec) -> str:
    colors = ["#{:02x}{:02x}{:02x}".format(*row.astype(int)) for row in BLUES]
    gradient = ", ".join(
        f"{color} {round(index * 100 / (len(colors) - 1))}%"
        for index, color in enumerate(colors)
    )
    return (
        f"**{spec.caption}**  \n"
        f"<div style='height:12px;border-radius:8px;background:linear-gradient(90deg,{gradient})'></div>"
        f"<small>{spec.vmin:.1f} mm — {spec.vmax:.1f} mm</small>"
    )


def _png_chunk(kind: bytes, payload: bytes) -> bytes:
    return (
        struct.pack(">I", len(payload))
        + kind
        + payload
        + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
    )


def _rgba_png_data_uri(rgba: np.ndarray) -> str:
    image = np.asarray(rgba, dtype=np.uint8)
    height, width, channels = image.shape
    if channels != 4:
        raise ValueError("RGBA image must have four channels.")
    scanlines = b"".join(b"\x00" + image[row].tobytes() for row in range(height))
    png = (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + _png_chunk(b"IDAT", zlib.compress(scanlines, 9))
        + _png_chunk(b"IEND", b"")
    )
    return "data:image/png;base64," + base64.b64encode(png).decode("ascii")


def _raster_image(values: np.ndarray, legend: RasterLegendSpec) -> str:
    finite = np.isfinite(values)
    span = legend.vmax - legend.vmin
    scaled = np.full(values.shape, 0.5) if span <= 0 else (values - legend.vmin) / span
    scaled = np.clip(scaled, 0, 1)
    scaled = np.where(finite, scaled, 0)
    positions = scaled * (len(BLUES) - 1)
    low = np.floor(positions).astype(int)
    high = np.ceil(positions).astype(int)
    weight = (positions - low)[..., None]
    rgb = BLUES[low] * (1 - weight) + BLUES[high] * weight
    alpha = np.where(finite, 255, 0)
    return _rgba_png_data_uri(np.dstack((rgb, alpha)))


def build_raster_layer(
    grid: PrecipitationGrid,
    *,
    layer_id: str,
    layer_name: str,
    opacity: float,
) -> tuple[dict[str, Any] | None, RasterLookup, RasterLegendSpec | None]:
    values = np.asarray(grid.values, dtype=float)
    display_values = north_first_raster_values(values, grid.latitudes)
    legend = build_raster_legend_spec(display_values, caption=layer_name)
    lookup = RasterLookup(
        layer_id=layer_id,
        layer_name=layer_name,
        values=values.copy(),
        latitudes=np.asarray(grid.latitudes, dtype=float).copy(),
        longitudes=np.asarray(grid.longitudes, dtype=float).copy(),
        bounds=tuple(float(value) for value in grid.bounds),
    )
    if legend is None:
        return None, lookup, None
    west, south, east, north = lookup.bounds
    layer = {
        "@@type": "BitmapLayer",
        "id": layer_id,
        "image": _raster_image(display_values, legend),
        "bounds": [west, south, east, north],
        "opacity": float(np.clip(opacity, 0, 1)),
        "pickable": True,
    }
    return layer, lookup, legend


def _json_compatible(value: Any) -> Any:
    if isinstance(value, Mapping):
        return {str(key): _json_compatible(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [_json_compatible(item) for item in value]
    if isinstance(value, np.generic):
        return value.item()
    if pd.isna(value):
        return None
    return value


def _station_records(stations: pd.DataFrame) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for row in stations.itertuples():
        status = str(getattr(row, "status", "no_data"))
        color = (
            NO_DATA_COLOR
            if status == "no_data"
            else DATA_ISSUE_COLOR
            if status == "data_issue"
            else KIND_COLORS.get(str(getattr(row, "kind", "no_data")), KIND_COLORS["no_data"])
        )
        records.append(
            {
                "station_id": str(row.station_id),
                "station_name": str(getattr(row, "station_name", "")),
                "provider_code": str(getattr(row, "provider_code", "")).upper(),
                "station_code": str(getattr(row, "station_code", "")),
                "position": [float(row.lon), float(row.lat)],
                "color": color,
                "status": status,
            }
        )
    return records


def _geojson_layer(
    layer_id: str,
    data: dict[str, Any] | None,
    *,
    line_color: list[int],
    fill_color: list[int],
    line_width: float,
) -> dict[str, Any] | None:
    if not data or not data.get("features"):
        return None
    return {
        "@@type": "GeoJsonLayer",
        "id": layer_id,
        "data": _json_compatible(data),
        "pickable": True,
        "stroked": True,
        "filled": True,
        "getLineColor": line_color,
        "getFillColor": fill_color,
        "getLineWidth": line_width,
        "lineWidthUnits": "pixels",
        "lineWidthMinPixels": 2,
        "autoHighlight": True,
    }


def default_view_state(
    *,
    stations: pd.DataFrame | None = None,
    bounds: tuple[float, float, float, float] | None = None,
) -> dict[str, float]:
    if bounds is not None:
        west, south, east, north = bounds
        longitude, latitude = (west + east) / 2, (south + north) / 2
    elif stations is not None and not stations.empty:
        longitude = float(stations["lon"].mean())
        latitude = float(stations["lat"].mean())
    else:
        longitude, latitude = -53.3, -29.7
    return {"longitude": longitude, "latitude": latitude, "zoom": 6.5, "pitch": 0, "bearing": 0}


def build_ops_map(
    selected_layer_name: str | None,
    opacity: float,
    stations: pd.DataFrame,
    segments_geojson: dict[str, Any] | None,
    raster_catalog: dict[str, dict[str, object]],
) -> DeckGLArtifacts:
    layers: list[dict[str, Any]] = []
    lookups: dict[str, RasterLookup] = {}
    pick_lookups: dict[str, tuple[MapSelection, ...]] = {}
    legends: list[RasterLegendSpec] = []
    raster_bounds = None
    if selected_layer_name and selected_layer_name in raster_catalog:
        meta = raster_catalog[selected_layer_name]
        grid = meta.get("grid")
        if not isinstance(grid, PrecipitationGrid):
            raise TypeError("Rainfall map catalog entries must contain a PrecipitationGrid.")
        raster_bounds = grid.bounds
        layer, lookup, legend = build_raster_layer(
            grid,
            layer_id=f"rainfall-raster:{selected_layer_name}",
            layer_name=str(meta.get("horizon_label", selected_layer_name)),
            opacity=opacity,
        )
        lookups[lookup.layer_id] = lookup
        if layer is not None:
            layers.append(layer)
        if legend is not None:
            legends.append(legend)

    for layer in (
        _geojson_layer(
            "mini-segments",
            segments_geojson,
            line_color=[25, 113, 194, 190],
            fill_color=[0, 0, 0, 0],
            line_width=2,
        ),
    ):
        if layer is not None:
            layers.append(layer)
            features = layer["data"].get("features", [])
            pick_lookups[layer["id"]] = tuple(
                MapSelection(mini_id=int(feature["properties"]["mini_id"]))
                for feature in features
            )

    station_data = _station_records(stations)
    if station_data:
        layers.append(
            {
                "@@type": "ScatterplotLayer",
                "id": "stations",
                "data": station_data,
                "pickable": True,
                "getPosition": "@@=position",
                "getFillColor": "@@=color",
                "getLineColor": [255, 255, 255, 220],
                "lineWidthMinPixels": 1,
                "stroked": True,
                "getRadius": 4500,
                "radiusMinPixels": 5,
                "radiusMaxPixels": 10,
                "autoHighlight": True,
            }
        )
        pick_lookups["stations"] = tuple(
            MapSelection(station_id=str(record["station_id"]))
            for record in station_data
        )

    spec = {
        "initialViewState": default_view_state(stations=stations, bounds=raster_bounds),
        "controller": True,
        "mapStyle": "https://basemaps.cartocdn.com/gl/positron-gl-style/style.json",
        "layers": layers,
    }
    tooltips = {
        "stations": {
            "html": "<b>{station_name}</b><br/>{provider_code}:{station_code}<br/>{status}"
        },
        "mini-segments": {"html": "<b>Mini {properties.mini_id}</b>"},
    }
    return DeckGLArtifacts(
        spec=spec,
        raster_lookups=lookups,
        pick_lookups=pick_lookups,
        tooltips=tooltips,
        legends=tuple(legends),
    )


def update_raster_opacity(
    artifacts: DeckGLArtifacts | None, opacity: float
) -> DeckGLArtifacts | None:
    """Return artifacts with only BitmapLayer opacity changed."""
    if artifacts is None:
        return None
    opacity = float(np.clip(opacity, 0, 1))
    layers = artifacts.spec.get("layers", [])
    updated_layers = [
        {**layer, "opacity": opacity}
        if str(layer.get("id", "")).startswith("rainfall-raster:")
        else layer
        for layer in layers
    ]
    if all(new is old for new, old in zip(updated_layers, layers, strict=True)):
        return artifacts
    return DeckGLArtifacts(
        spec={**artifacts.spec, "layers": updated_layers},
        raster_lookups=artifacts.raster_lookups,
        pick_lookups=artifacts.pick_lookups,
        tooltips=artifacts.tooltips,
        legends=artifacts.legends,
    )


def decode_click_state(
    click_state: Mapping[str, Any] | None,
    pick_lookups: Mapping[str, tuple[MapSelection, ...]] | None = None,
) -> MapSelection:
    if not click_state:
        return MapSelection()
    layer = click_state.get("layer") or click_state.get("layer_id") or {}
    layer_id = layer.get("id") if isinstance(layer, Mapping) else layer
    layer_id = str(layer_id or "")
    obj = click_state.get("object") or {}
    properties = obj.get("properties", obj) if isinstance(obj, Mapping) else {}
    coordinate = click_state.get("coordinate")
    parsed_coordinate = None
    if isinstance(coordinate, (list, tuple)) and len(coordinate) >= 2:
        parsed_coordinate = (float(coordinate[0]), float(coordinate[1]))

    station_id = properties.get("station_id") if isinstance(properties, Mapping) else None
    mini_id = properties.get("mini_id") if isinstance(properties, Mapping) else None
    index = click_state.get("index")
    indexed_selection = None
    if (
        station_id is None
        and mini_id is None
        and pick_lookups
        and layer_id in pick_lookups
        and isinstance(index, int)
        and not isinstance(index, bool)
        and 0 <= index < len(pick_lookups[layer_id])
    ):
        indexed_selection = pick_lookups[layer_id][index]
        station_id = indexed_selection.station_id
        mini_id = indexed_selection.mini_id
    if station_id is not None or layer_id == "stations":
        station_id = str(station_id) if station_id is not None else None
    if mini_id is not None:
        mini_id = int(mini_id)
    raster_id = layer_id if layer_id.startswith("rainfall-raster:") else None
    return MapSelection(station_id, mini_id, raster_id, parsed_coordinate)


def lookup_raster_value(
    lookup: RasterLookup,
    longitude: float,
    latitude: float,
) -> RasterValue | None:
    lon, lat = float(longitude), float(latitude)
    west, south, east, north = lookup.bounds
    if not (west <= lon <= east and south <= lat <= north):
        return None
    column = int(np.argmin(np.abs(lookup.longitudes - lon)))
    row = int(np.argmin(np.abs(lookup.latitudes - lat)))
    raw = float(lookup.values[row, column])
    return RasterValue(
        layer_name=lookup.layer_name,
        longitude=lon,
        latitude=lat,
        row=row,
        column=column,
        value=raw if np.isfinite(raw) else None,
    )


def inspect_raster_click(
    click_state: Mapping[str, Any] | None,
    lookups: Mapping[str, RasterLookup],
) -> RasterValue | None:
    selection = decode_click_state(click_state)
    if selection.raster_layer_id is None or selection.coordinate is None:
        return None
    lookup = lookups.get(selection.raster_layer_id)
    if lookup is None:
        return None
    return lookup_raster_value(lookup, *selection.coordinate)
