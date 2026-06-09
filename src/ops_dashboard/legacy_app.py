from __future__ import annotations


from datetime import datetime, timedelta
from pathlib import Path
import json
import re
from typing import Optional

import numpy as np
import pandas as pd
from affine import Affine
import folium
import branca.colormap as cm
from branca.element import MacroElement, Template
from folium.raster_layers import ImageOverlay
import rasterio
from rasterio.enums import Resampling
from rasterio.transform import array_bounds
import streamlit as st
from streamlit_folium import st_folium
from plotly.subplots import make_subplots
import plotly.graph_objects as go

REPO_ROOT = Path(__file__).resolve().parent
DATA_DIR = REPO_ROOT / "data"
TELEMETRIA_DIR = DATA_DIR / "telemetria"
INTERP_DIR = DATA_DIR / "interp"
APP_LAYERS_DIR = DATA_DIR / "app_layers"
MGB_PROCESSED_DIR = DATA_DIR / "mgb-hora" / "processed"
RIVERS_GEOJSON_PATH = APP_LAYERS_DIR / "rios_mini.geojson"
Q_META_PATH = MGB_PROCESSED_DIR / "q_meta.json"
Y_META_PATH = MGB_PROCESSED_DIR / "y_meta.json"
DAYS_WINDOW = 30
NO_DATA_COLOR = "#e64980"
DATA_ISSUE_COLOR = "#f08c00"
MGB_COLORS = {"QTUDO": "#1864ab", "YTUDO": "#2b8a3e"}
CLICK_TOKEN_PATTERN = re.compile(r"(POSTO\|[^\s]+|MINI\|\d+)")
KIND_COLORS = {"nível": "#0b7285", "chuva": "#364fc7"}

# Paleta fixa Blues (claro→escuro)
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


# ---------- Estilos ----------
st.set_page_config(page_title="Explorador de Estações RS", layout="wide")
st.markdown(
    """
    <style>
    @import url('https://fonts.googleapis.com/css2?family=Space+Grotesk:wght@400;600&display=swap');
    html, body, [class*="css"]  {
        font-family: 'Space Grotesk', 'Helvetica Neue', sans-serif;
        background: radial-gradient(circle at 10% 20%, #f2f7fb, #e8f1f7 40%, #e5ecf3 100%);
    }
    .metric-card {
        background: #0b7285;
        color: #f8fafc;
        padding: 0.75rem 1rem;
        border-radius: 10px;
    }
    </style>
    """,
    unsafe_allow_html=True,
)


# ---------- Utilidades de leitura ----------
@st.cache_data(show_spinner=False)
def load_stations() -> pd.DataFrame:
    frames: list[pd.DataFrame] = []
    for csv_name, kind in [("estacoes_nivel.csv", "nível"), ("estacoes_pluv.csv", "chuva")]:
        path = DATA_DIR / csv_name
        if not path.exists():
            continue
        df = pd.read_csv(path, sep=";", encoding="utf-8")
        keep = {"CODIGO": "station_id", "LAT": "lat", "LON": "lon", "NOME": "name"}
        df = df.rename(columns=keep)
        df = df[list(keep.values())].copy()
        df["kind"] = kind
        frames.append(df)
    if not frames:
        return pd.DataFrame(columns=["station_id", "lat", "lon", "name", "kind"])
    merged = pd.concat(frames, ignore_index=True).drop_duplicates(subset="station_id")
    for col in ["lat", "lon"]:
        merged[col] = pd.to_numeric(merged[col], errors="coerce")
    merged["station_id"] = merged["station_id"].astype(str).str.strip()
    merged["name"] = merged["name"].fillna("").astype(str).str.strip()
    merged = merged.dropna(subset=["lat", "lon"])
    return merged


@st.cache_data(show_spinner=False)
def load_timeseries(station_id: str, days: int = 30) -> pd.DataFrame:
    csv_path = TELEMETRIA_DIR / f"{station_id}.csv"
    if not csv_path.exists():
        return pd.DataFrame(columns=["station_id", "datetime", "rain", "level", "flow"])
    df = pd.read_csv(csv_path)
    df["datetime"] = pd.to_datetime(df["datetime"], errors="coerce")
    df["rain"] = pd.to_numeric(df.get("rain"), errors="coerce")
    df["level"] = pd.to_numeric(df.get("level"), errors="coerce")
    df["flow"] = pd.to_numeric(df.get("flow"), errors="coerce")
    cutoff = datetime.utcnow() - timedelta(days=days)
    df = df[df["datetime"] >= cutoff].sort_values("datetime")
    return df


@st.cache_data(show_spinner=False)
def list_rasters() -> list[dict]:
    rasters = []
    for tif in sorted(INTERP_DIR.glob("*.tif")):
        try:
            with rasterio.open(tif) as src:
                tags = src.tags()
                rasters.append(
                    {
                        "path": tif,
                        "name": tif.stem,
                        "shape": src.shape,
                        "tags": tags,
                    }
                )
        except rasterio.RasterioError:
            continue
    return rasters


@st.cache_data(show_spinner=False)
def load_raster_data(path: Path, max_size: int = 600) -> tuple[np.ndarray, tuple[float, float, float, float]]:
    """
    Lê raster e devolve array float32 e bounds (west, south, east, north).
    Downsample automático para no máx. max_size pixels em cada dimensão.
    Mascara valores <= 0 como NaN (chuva zero não deve aparecer).
    """
    with rasterio.open(path) as src:
        scale = min(max_size / src.height, max_size / src.width, 1.0)
        out_h = max(1, int(src.height * scale))
        out_w = max(1, int(src.width * scale))
        data = src.read(
            1,
            out_shape=(out_h, out_w),
            resampling=Resampling.bilinear,
        )
        data = data.astype("float32")
        data[data <= 0] = np.nan  # mascara chuva zero/negativa
        data[data <= -1e20] = np.nan  # tolera nodata

        # Ajusta transform para o raster reamostrado
        scale_x = src.width / out_w
        scale_y = src.height / out_h
        new_transform = src.transform * Affine.scale(scale_x, scale_y)
        west, south, east, north = array_bounds(out_h, out_w, new_transform)
    return data, (west, south, east, north)


@st.cache_data(show_spinner=False)
def load_rivers_layer_geojson() -> Optional[dict]:
    if not RIVERS_GEOJSON_PATH.exists():
        return None
    payload = json.loads(RIVERS_GEOJSON_PATH.read_text(encoding="utf-8"))
    if payload.get("type") != "FeatureCollection":
        return None

    features = payload.get("features") or []
    for feature in features:
        props = feature.setdefault("properties", {})
        mini_raw = props.get("mini_id")
        if mini_raw is None:
            props["click_id"] = "MINI|"
            continue
        try:
            mini_id = int(mini_raw)
            props["mini_id"] = mini_id
            props["click_id"] = f"MINI|{mini_id}"
        except (TypeError, ValueError):
            props["click_id"] = "MINI|"
    return payload


@st.cache_data(show_spinner=False)
def load_mgb_meta() -> dict[str, dict]:
    metas: dict[str, dict] = {}
    for meta_path in (Q_META_PATH, Y_META_PATH):
        if not meta_path.exists():
            continue
        try:
            payload = json.loads(meta_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue

        variable = str(payload.get("variable", "")).upper().strip()
        if variable not in {"QTUDO", "YTUDO"}:
            continue

        output_raw = payload.get("output_file")
        if not isinstance(output_raw, str) or not output_raw.strip():
            continue
        parquet_path = REPO_ROOT / Path(output_raw.replace("\\", "/"))
        metas[variable] = {
            "meta_path": meta_path,
            "parquet_path": parquet_path,
            "nc": payload.get("nc"),
            "nt": payload.get("nt"),
            "dt_seconds": payload.get("dt_seconds"),
        }
    return metas


@st.cache_data(show_spinner=False)
def load_mgb_series(mini_id: int, variable: str, days_window: int) -> pd.DataFrame:
    metas = load_mgb_meta()
    variable_key = variable.upper().strip()
    meta = metas.get(variable_key)
    if meta is None:
        return pd.DataFrame(columns=["dt", "prev", "value"])

    parquet_path = Path(meta["parquet_path"])
    if not parquet_path.exists():
        return pd.DataFrame(columns=["dt", "prev", "value"])

    mini_col = str(int(mini_id))
    try:
        df = pd.read_parquet(parquet_path, columns=["dt", "prev", mini_col])
    except ImportError as exc:
        raise RuntimeError("Parquet read requires pyarrow. Install with: pip install pyarrow") from exc
    except (ValueError, KeyError):
        return pd.DataFrame(columns=["dt", "prev", "value"])

    if df.empty:
        return pd.DataFrame(columns=["dt", "prev", "value"])

    df = df.rename(columns={mini_col: "value"}).copy()
    df["dt"] = pd.to_datetime(df["dt"], errors="coerce")
    df["prev"] = pd.to_numeric(df["prev"], errors="coerce").fillna(0).astype("int8")
    df["value"] = pd.to_numeric(df["value"], errors="coerce")
    df = df.dropna(subset=["dt"]).sort_values("dt")

    current_df = df[df["prev"] == 0].copy()
    forecast_df = df[df["prev"] == 1].copy()
    if not current_df.empty and days_window > 0:
        cutoff = current_df["dt"].max() - timedelta(days=days_window)
        current_df = current_df[current_df["dt"] >= cutoff]

    out = pd.concat([current_df, forecast_df], ignore_index=True)
    if out.empty:
        return pd.DataFrame(columns=["dt", "prev", "value"])
    out["segment"] = np.where(out["prev"] == 1, "previsao", "atual")
    return out.sort_values("dt")


def parse_click_token(tooltip_value: Optional[str]) -> Optional[str]:
    if not tooltip_value:
        return None
    text = str(tooltip_value)
    match = CLICK_TOKEN_PATTERN.search(text)
    if not match:
        return None
    return match.group(1)


@st.cache_data(show_spinner=False)
def load_station_health(station_ids: tuple[str, ...], days: int = 30) -> pd.DataFrame:
    cutoff = datetime.utcnow() - timedelta(days=days)
    rows: list[dict[str, object]] = []

    for station_id in station_ids:
        csv_path = TELEMETRIA_DIR / f"{station_id}.csv"
        if not csv_path.exists():
            rows.append(
                {
                    "station_id": station_id,
                    "status": "no_data",
                    "status_reason": "arquivo ausente",
                    "rows_recent": 0,
                    "rain_mean_mm_h": np.nan,
                    "rain_acc_24h_mm": np.nan,
                    "rain_p90_mm_h": np.nan,
                }
            )
            continue

        try:
            df = pd.read_csv(csv_path, usecols=["datetime", "rain", "level", "flow"])
        except Exception:
            rows.append(
                {
                    "station_id": station_id,
                    "status": "data_issue",
                    "status_reason": "erro de leitura",
                    "rows_recent": 0,
                    "rain_mean_mm_h": np.nan,
                    "rain_acc_24h_mm": np.nan,
                    "rain_p90_mm_h": np.nan,
                }
            )
            continue

        if df.empty:
            rows.append(
                {
                    "station_id": station_id,
                    "status": "no_data",
                    "status_reason": "arquivo vazio",
                    "rows_recent": 0,
                    "rain_mean_mm_h": np.nan,
                    "rain_acc_24h_mm": np.nan,
                    "rain_p90_mm_h": np.nan,
                }
            )
            continue

        total_rows = len(df)
        df["datetime"] = pd.to_datetime(df["datetime"], errors="coerce")
        invalid_dt = int(df["datetime"].isna().sum())
        df = df.dropna(subset=["datetime"]).sort_values("datetime")
        recent = df[df["datetime"] >= cutoff].copy()

        if recent.empty:
            rows.append(
                {
                    "station_id": station_id,
                    "status": "no_data",
                    "status_reason": f"sem registros nos últimos {days} dias",
                    "rows_recent": 0,
                    "rain_mean_mm_h": np.nan,
                    "rain_acc_24h_mm": np.nan,
                    "rain_p90_mm_h": np.nan,
                }
            )
            continue

        recent["rain"] = pd.to_numeric(recent["rain"], errors="coerce")
        recent["level"] = pd.to_numeric(recent["level"], errors="coerce")
        recent["flow"] = pd.to_numeric(recent["flow"], errors="coerce")

        rain_valid = recent["rain"].dropna()
        level_valid = recent["level"].dropna()
        flow_valid = recent["flow"].dropna()
        duplicate_time_mask = recent.duplicated(subset=["datetime"], keep=False)
        duplicate_nonzero_rain_mask = duplicate_time_mask & recent["rain"].fillna(0).abs().gt(0)
        duplicate_ratio = float(duplicate_nonzero_rain_mask.sum() / max(len(recent), 1))

        issues = []
        if invalid_dt > 0:
            issues.append("datetime inválido")
        if (invalid_dt / max(total_rows, 1)) > 0.2:
            issues.append("muitos datetime inválidos")
        if duplicate_ratio > 0.2:
            issues.append("muitos horários repetidos com chuva > 0")
        if rain_valid.empty and level_valid.empty and flow_valid.empty:
            issues.append("sem variáveis válidas")

        latest_time = recent["datetime"].max()
        rain_24h = recent.loc[
            recent["datetime"] >= latest_time - timedelta(hours=24), "rain"
        ].sum(min_count=1)

        rows.append(
            {
                "station_id": station_id,
                "status": "data_issue" if issues else "ok",
                "status_reason": "; ".join(issues) if issues else "",
                "rows_recent": int(len(recent)),
                "rain_mean_mm_h": float(rain_valid.mean()) if not rain_valid.empty else np.nan,
                "rain_acc_24h_mm": float(rain_24h) if pd.notna(rain_24h) else np.nan,
                "rain_p90_mm_h": float(rain_valid.quantile(0.9)) if not rain_valid.empty else np.nan,
            }
        )

    return pd.DataFrame(rows)


def merge_station_context(stations: pd.DataFrame, days: int = 30) -> pd.DataFrame:
    if stations.empty:
        out = stations.copy()
        out["status"] = pd.Series(dtype="object")
        out["status_reason"] = pd.Series(dtype="object")
        out["rain_mean_mm_h"] = pd.Series(dtype="float64")
        out["rain_acc_24h_mm"] = pd.Series(dtype="float64")
        out["rain_p90_mm_h"] = pd.Series(dtype="float64")
        return out
    health = load_station_health(tuple(stations["station_id"].tolist()), days=days)
    merged = stations.merge(health, on="station_id", how="left")
    merged["status"] = merged["status"].fillna("no_data")
    merged["status_reason"] = merged["status_reason"].fillna("")
    return merged


def format_mm(value: float | int | None) -> str:
    if value is None or pd.isna(value):
        return "—"
    return f"{float(value):.1f} mm"


def network_summary(stations: pd.DataFrame) -> dict[str, float]:
    if stations.empty or "status" not in stations:
        return {
            "total": 0.0,
            "with_data": 0.0,
            "no_data": 0.0,
            "data_issue": 0.0,
            "rain_mean_24h": np.nan,
            "rain_p90_24h": np.nan,
        }

    total = float(len(stations))
    no_data = float((stations["status"] == "no_data").sum())
    data_issue = float((stations["status"] == "data_issue").sum())
    with_data = float((stations["status"] == "ok").sum())

    rain_values = stations.loc[stations["status"] == "ok", "rain_acc_24h_mm"].dropna()
    if rain_values.empty:
        rain_values = stations["rain_acc_24h_mm"].dropna()

    return {
        "total": total,
        "with_data": with_data,
        "no_data": no_data,
        "data_issue": data_issue,
        "rain_mean_24h": float(rain_values.mean()) if not rain_values.empty else np.nan,
        "rain_p90_24h": float(rain_values.quantile(0.9)) if not rain_values.empty else np.nan,
    }


def render_network_summary(stations: pd.DataFrame) -> None:
    summary = network_summary(stations)
    cols = st.columns(7)
    cols[0].metric("Postos totais", f"{int(summary['total'])}")
    cols[1].metric("Com dados", f"{int(summary['with_data'])}")
    cols[2].metric("Sem dados", f"{int(summary['no_data'])}")
    cols[3].metric("Falha de dados", f"{int(summary['data_issue'])}")
    cols[4].metric("Média chuva 24h", format_mm(summary["rain_mean_24h"]))
    cols[5].metric("P90 chuva 24h", format_mm(summary["rain_p90_24h"]))


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


# ---------- Componentes de UI ----------
def add_legend(fmap: folium.Map, vmin: float, vmax: float, *, horizon_label: Optional[str]) -> None:
    colors_hex = ["#{:02x}{:02x}{:02x}".format(int(r*255), int(g*255), int(b*255)) for r, g, b in BLUES]
    colormap = cm.LinearColormap(colors=colors_hex, vmin=float(vmin), vmax=float(vmax))
    horizon_text = horizon_label or "período selecionado"
    colormap.caption = f"Chuva acumulada das últimas {horizon_text}"
    colormap.add_to(fmap)


def extract_horizon_label(layer_name: Optional[str]) -> Optional[str]:
    if not layer_name:
        return None
    match = re.search(r"_(\d+h)$", layer_name)
    if match:
        return match.group(1)
    return None


def render_selected_station_context(row: Optional[pd.Series], station_id: str) -> None:
    if row is None:
        st.markdown(f"### Posto selecionado: {station_id}")
        return

    status_labels = {
        "ok": "ok",
        "data_issue": "falha de dados",
        "no_data": "sem dados",
    }
    station_name = str(row.get("name", "")).strip() or "sem nome"
    status = status_labels.get(str(row.get("status", "no_data")), "sem dados")
    kind = str(row.get("kind", "—"))
    reason = str(row.get("status_reason", "")).strip()

    st.markdown(f"### Posto selecionado: {station_name}")
    st.caption(f"Código: {station_id} | Tipo: {kind} | Status: {status}")
    if reason:
        st.caption(f"Obs: {reason}")


class RasterClickPopup(MacroElement):
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
        self._template = Template(
            """
            {% macro script(this, kwargs) %}
            var rasterData = {{this.data}};
            var rasterBounds = {south: {{this.south}}, west: {{this.west}}, north: {{this.north}}, east: {{this.east}}};
            (function attachRasterClick() {
                var mapRef = window.map;
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


def build_map(
    selected_layer: Optional[str],
    opacity: float,
    stations: pd.DataFrame,
    rivers_geojson: Optional[dict],
) -> folium.Map:
    center = (
        [stations["lat"].mean(), stations["lon"].mean()]
        if not stations.empty
        else [-29.7, -53.3]
    )
    fmap = folium.Map(location=center, zoom_start=7, tiles="CartoDB Positron", control_scale=True)

    catalog = {layer["name"]: layer for layer in list_rasters()}
    if selected_layer:
        meta = catalog.get(selected_layer)
        if meta:
            data, (west, south, east, north) = load_raster_data(meta["path"])
            finite_values = data[np.isfinite(data)]
            if finite_values.size > 0:
                vmin, vmax = np.nanpercentile(data, [5, 95])
                overlay = ImageOverlay(
                    name=f"Raster {selected_layer}",
                    image=data,
                    bounds=[[south, west], [north, east]],
                    opacity=opacity,
                    interactive=False,
                    cross_origin=False,
                    mercator_project=False,
                    colormap=color_ramp_factory(vmin, vmax, opacity),
                )
                raster_group = folium.FeatureGroup(name="Chuva interpolada", show=True)
                overlay.add_to(raster_group)
                raster_group.add_to(fmap)
                add_legend(
                    fmap,
                    vmin,
                    vmax,
                    horizon_label=extract_horizon_label(selected_layer),
                )
                RasterClickPopup(data, (west, south, east, north), selected_layer).add_to(fmap)

    if rivers_geojson and rivers_geojson.get("features"):
        rios_layer = folium.FeatureGroup(name="Rios MGB", show=True)
        folium.GeoJson(
            rivers_geojson,
            style_function=lambda _: {"color": "#1971c2", "weight": 1.2, "opacity": 0.45},
            highlight_function=lambda _: {"color": "#0b7285", "weight": 2.2, "opacity": 0.9},
            tooltip=folium.GeoJsonTooltip(
                fields=["click_id"],
                aliases=[""],
                labels=False,
                sticky=False,
            ),
            name="Rios MGB",
        ).add_to(rios_layer)
        rios_layer.add_to(fmap)

    station_layer = folium.FeatureGroup(name="Postos com dados", show=True)
    no_data_layer = folium.FeatureGroup(name="Postos sem dados", show=True)
    for row in stations.itertuples():
        station_name = row.name if row.name else "sem nome"
        tooltip = f"POSTO|{row.station_id} — {station_name}"
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

        marker_color = KIND_COLORS.get(getattr(row, "kind", ""), "#364fc7")
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


def time_series_chart(df: pd.DataFrame, station_id: str, days: int):
    if df.empty:
        st.info("Sem dados para esta estação/intervalo.")
        return

    fig = make_subplots(
        rows=2,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.08,
        row_heights=[0.35, 0.65],
    )

    fig.add_bar(
        x=df["datetime"],
        y=df["rain"],
        name="Chuva (mm)",
        marker_color="#4dabf7",
        opacity=0.9,
        row=1,
        col=1,
    )
    fig.add_scatter(
        x=df["datetime"],
        y=df["level"],
        name="Nível (cm)",
        mode="lines+markers",
        line=dict(color="#0b7285", width=2),
        marker=dict(size=4),
        row=2,
        col=1,
    )

    fig.update_layout(
        template="plotly_white",
        height=520,
        margin=dict(t=30, r=20, l=10, b=30),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, x=0),
        hovermode="x unified",
        xaxis=dict(title=""),
        xaxis2=dict(title="Data/hora (UTC)"),
    )
    st.plotly_chart(fig, use_container_width=True, key=f"chart-{station_id}-{days}")


def render_selected_mini_context(mini_id: Optional[int], variable: str, days_window: int) -> None:
    if mini_id is None:
        st.markdown("### Mini selecionada: â€”")
        st.caption("Clique em uma geometria de rio no mapa para carregar a série do modelo.")
        return
    st.markdown(f"### Mini selecionada: {mini_id}")
    st.caption(f"Variavel: {variable} | Janela: ultimos {days_window} dias + previsao")
    st.caption("Fonte: MGB-Hora")


def mgb_metric_cards(df: pd.DataFrame, variable: str) -> None:
    if df.empty:
        st.info("Sem dados do modelo para a mini selecionada.")
        return

    current_df = df[df["prev"] == 0].copy()
    forecast_df = df[df["prev"] == 1].copy()

    col1, col2 = st.columns(2)
    col1.metric("Pontos atuais", f"{len(current_df)}")
    col2.metric("Pontos previsão", f"{len(forecast_df)}")

    if not current_df.empty:
        last_current = current_df.sort_values("dt").tail(1).iloc[0]
        st.caption(
            f"Último atual: {last_current['dt']:%d/%m/%Y %H:%M} | {variable}={last_current['value']:.3f}"
        )
    if not forecast_df.empty:
        first_forecast = forecast_df.sort_values("dt").head(1).iloc[0]
        st.caption(
            f"Início previsão: {first_forecast['dt']:%d/%m/%Y %H:%M} | {variable}={first_forecast['value']:.3f}"
        )


def mgb_time_series_chart(df: pd.DataFrame, mini_id: int, variable: str, days_window: int) -> None:
    if df.empty:
        st.info("Sem série do modelo para a mini selecionada.")
        return

    current_df = df[df["prev"] == 0]
    forecast_df = df[df["prev"] == 1]

    fig = go.Figure()
    base_color = MGB_COLORS.get(variable, "#1864ab")
    forecast_color = "#e67700"

    if not current_df.empty:
        fig.add_trace(
            go.Scatter(
                x=current_df["dt"],
                y=current_df["value"],
                mode="lines",
                name=f"{variable} atual",
                line=dict(color=base_color, width=2),
            )
        )
    if not forecast_df.empty:
        fig.add_trace(
            go.Scatter(
                x=forecast_df["dt"],
                y=forecast_df["value"],
                mode="lines",
                name=f"{variable} previsão",
                line=dict(color=forecast_color, width=2, dash="dash"),
            )
        )

    fig.update_layout(
        template="plotly_white",
        height=420,
        margin=dict(t=30, r=20, l=10, b=30),
        hovermode="x unified",
        legend=dict(orientation="h", yanchor="bottom", y=1.02, x=0),
        xaxis=dict(title="Data/hora"),
        yaxis=dict(title=variable),
    )
    st.plotly_chart(fig, use_container_width=True, key=f"mgb-chart-{variable}-{mini_id}-{days_window}")


def metric_cards(df: pd.DataFrame):
    if df.empty:
        st.write("Nenhum dado recente.")
        return
    recent = df.dropna(subset=["datetime"]).sort_values("datetime")
    if recent.empty:
        st.write("Nenhum dado recente.")
        return

    last = recent.tail(1).iloc[0]
    latest_time = recent["datetime"].max()
    rain_12h = recent[recent["datetime"] >= latest_time - timedelta(hours=12)]["rain"].sum(min_count=1)
    rain_24h = recent[recent["datetime"] >= latest_time - timedelta(hours=24)]["rain"].sum(min_count=1)
    rain_72h = recent[recent["datetime"] >= latest_time - timedelta(hours=72)]["rain"].sum(min_count=1)
    st.markdown(f"**Última leitura:** {last['datetime']:%d/%m %H:%M}")

    st.markdown("**Chuvas acumuladas**")
    rains_table = pd.DataFrame(
        {
            "Valor": [
                f"{rain_12h:.1f} mm" if pd.notna(rain_12h) else "—",
                f"{rain_24h:.1f} mm" if pd.notna(rain_24h) else "—",
                f"{rain_72h:.1f} mm" if pd.notna(rain_72h) else "—",
            ]
        },
        index=["12h", "24h", "72h"],
    )
    st.table(rains_table)

    level_txt = f"{last['level']:.1f} cm" if pd.notna(last.get("level")) else "—"
    st.markdown("**Estado do nível**")
    level_table = pd.DataFrame(
        {"Valor": [level_txt, ""]},
        index=["atual", "alerta"],
    )
    st.table(level_table)


stations_df = load_stations()
stations_context_df = merge_station_context(stations_df, days=DAYS_WINDOW)
rivers_geojson = load_rivers_layer_geojson()
mgb_meta = load_mgb_meta()

st.title("Sistema de Alerta de Cheias RS - Explorer")
render_network_summary(stations_context_df)
st.caption(
    "Clique em um posto para dados observados (ANA) ou em uma geometria de rio para series do modelo MGB."
)

with st.sidebar:
    st.subheader("Controles")
    available_rasters = [r["name"] for r in list_rasters()]
    selected_layer = st.selectbox(
        "Raster interpolado",
        options=["(nenhum)"] + available_rasters,
        index=1 if available_rasters else 0,
    )
    selected_layer = None if selected_layer == "(nenhum)" else selected_layer
    opacity = st.slider("Transparencia raster", min_value=0.0, max_value=1.0, value=0.6, step=0.05)
    st.markdown(
        "**Camadas:** postos e rios MGB estao sobrepostos no mapa; o clique define qual serie carregar."
    )

base_map = build_map(selected_layer, opacity, stations_context_df, rivers_geojson=rivers_geojson)
map_state = st_folium(
    base_map,
    height=620,
    use_container_width=True,
    key="map",
    returned_objects=["last_object_clicked_tooltip"],
)

clicked_tooltip = (map_state or {}).get("last_object_clicked_tooltip")
click_token = parse_click_token(clicked_tooltip)

default_station: Optional[str] = None
if not stations_context_df.empty:
    preferred = stations_context_df[stations_context_df["status"] != "no_data"]
    default_station = (
        str(preferred["station_id"].iloc[0])
        if not preferred.empty
        else str(stations_context_df["station_id"].iloc[0])
    )

clicked_station_id: Optional[str] = None
clicked_mini_id: Optional[int] = None
if click_token and click_token.startswith("POSTO|"):
    clicked_station_id = click_token.split("|", 1)[1].strip()
if click_token and click_token.startswith("MINI|"):
    raw_mini = click_token.split("|", 1)[1].strip()
    try:
        clicked_mini_id = int(raw_mini)
    except ValueError:
        clicked_mini_id = None

station_id = clicked_station_id or st.session_state.get("station_id", default_station)
if station_id:
    st.session_state["station_id"] = station_id

mini_id = clicked_mini_id if clicked_mini_id is not None else st.session_state.get("mini_id")
if mini_id is not None:
    st.session_state["mini_id"] = int(mini_id)
    mini_id = int(mini_id)

station_series = load_timeseries(station_id, days=DAYS_WINDOW) if station_id else pd.DataFrame()
selected_station_row: Optional[pd.Series] = None
if station_id and not stations_context_df.empty:
    selected = stations_context_df[stations_context_df["station_id"] == station_id]
    if not selected.empty:
        selected_station_row = selected.iloc[0]

st.subheader("Dados observados (ANA)")
left, right = st.columns([0.45, 0.55])
with left:
    if station_id:
        render_selected_station_context(selected_station_row, station_id)
    else:
        st.markdown("### Estacao selecionada: -")
    if station_id:
        metric_cards(station_series)
    else:
        st.info("Selecione um posto no mapa.")

with right:
    if station_id:
        time_series_chart(station_series, station_id, DAYS_WINDOW)
    else:
        st.info("Sem posto selecionado.")

st.markdown("---")
st.subheader("Outputs MGB")

if rivers_geojson is None:
    st.warning(
        "Camada de rios nao encontrada em data/app_layers/rios_mini.geojson. "
        "Rode: python src/build_app_layers_mgb.py"
    )

available_mgb_vars = [var for var in ("QTUDO", "YTUDO") if var in mgb_meta]
if not available_mgb_vars:
    st.warning(
        "Metadados MGB nao encontrados em data/mgb-hora/processed/q_meta.json e y_meta.json."
    )

selected_mgb_variable = (
    st.selectbox("Variavel MGB", options=available_mgb_vars, index=0, key="mgb_variable")
    if available_mgb_vars
    else "QTUDO"
)

model_series = pd.DataFrame(columns=["dt", "prev", "value"])
model_error: Optional[str] = None
if mini_id is not None and available_mgb_vars:
    try:
        model_series = load_mgb_series(mini_id=mini_id, variable=selected_mgb_variable, days_window=DAYS_WINDOW)
    except RuntimeError as exc:
        model_error = str(exc)

mgb_left, mgb_right = st.columns([0.35, 0.65])
with mgb_left:
    render_selected_mini_context(mini_id, selected_mgb_variable, DAYS_WINDOW)
    if model_error:
        st.error(model_error)
    elif mini_id is None:
        st.info("Clique em uma geometria de rio para carregar a serie do modelo.")
    else:
        mgb_metric_cards(model_series, selected_mgb_variable)

with mgb_right:
    if model_error:
        st.info("Nao foi possivel renderizar o grafico do modelo.")
    elif mini_id is None:
        st.info("Sem mini selecionada.")
    else:
        mgb_time_series_chart(model_series, mini_id, selected_mgb_variable, DAYS_WINDOW)
