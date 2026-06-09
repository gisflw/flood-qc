from __future__ import annotations

import json
from pathlib import Path
import sqlite3
import sys
from typing import Any, Optional

import numpy as np
import pandas as pd

REPO_ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = REPO_ROOT / "src"
if str(SRC_DIR) not in sys.path:
    sys.path.insert(0, str(SRC_DIR))

try:
    import plotly.graph_objects as go
    from plotly.subplots import make_subplots
    import streamlit as st
except ModuleNotFoundError as exc:
    raise SystemExit(
        "Dependencias de UI nao encontradas. Instale com: "
        "pip install streamlit plotly folium streamlit-folium branca rasterio"
    ) from exc

from common.paths import history_db_path
from qc.ecmwf_forecast_correction import ForecastCorrectionInstruction
from reporting import ops_dashboard_data, ops_dashboard_forecast, ops_dashboard_map
from storage.history_repository import HistoryRepository


DAYS_WINDOW = 30
MGB_COLORS = {"q": "#1864ab", "y": "#0b7285"}
NO_LAYER_OPTION = "(nenhum)"
REFRESH_TS_FORMAT = "%d/%m/%Y %H:%M:%S"
FORECAST_EDIT_COLUMNS = [
    "manual_edit_id",
    "asset_id",
    "t0_step",
    "t1_step",
    "shift_lat",
    "shift_lon",
    "rotation_deg",
    "multiplication_factor",
    "editor",
    "reason",
    "metadata_json",
    "created_at",
    "remove",
]
FORECAST_EDIT_NUMERIC_COLUMNS = ["t0_step", "t1_step", "shift_lat", "shift_lon", "rotation_deg", "multiplication_factor"]


st.set_page_config(
    page_title="Hidrologia operacional",
    page_icon=":material/water_drop:",
    layout="wide",
)


@st.cache_data(show_spinner=False, max_entries=4)
def get_station_catalog(days: int) -> pd.DataFrame:
    return ops_dashboard_data.load_station_catalog(days=days)


@st.cache_data(show_spinner=False, max_entries=128)
def get_observed_series(station_uid: int, days: int) -> pd.DataFrame:
    return ops_dashboard_data.load_observed_series(station_uid=station_uid, days=days)


@st.cache_data(show_spinner=False, max_entries=2)
def get_model_variables() -> pd.DataFrame:
    return ops_dashboard_data.list_model_variables()


@st.cache_data(show_spinner=False, max_entries=256)
def get_mgb_series(mini_id: int, variable_code: str, days_window: int) -> pd.DataFrame:
    return ops_dashboard_data.load_mgb_series(
        mini_id=mini_id,
        variable_code=variable_code,
        days_window=days_window,
    )


@st.cache_data(show_spinner=False, max_entries=4)
def get_accumulation_rasters() -> list[dict[str, object]]:
    return ops_dashboard_data.list_accumulation_rasters()


@st.cache_data(show_spinner=False, max_entries=2)
def get_rivers_geojson() -> dict | None:
    return ops_dashboard_data.load_rivers_layer_geojson()


@st.cache_data(show_spinner=False, max_entries=8)
def get_forecast_assets() -> pd.DataFrame:
    return ops_dashboard_forecast.list_forecast_assets()


@st.cache_data(show_spinner=False, max_entries=64)
def get_forecast_steps(asset_id: str) -> pd.DataFrame:
    return ops_dashboard_forecast.list_forecast_steps(asset_id)


@st.cache_data(show_spinner=False, max_entries=64)
def get_saved_forecast_edits(asset_id: str) -> pd.DataFrame:
    with HistoryRepository(history_db_path()) as repository:
        rows = repository.list_forecast_manual_edits(asset_id)
    if not rows:
        return pd.DataFrame(columns=FORECAST_EDIT_COLUMNS)
    frame = pd.DataFrame(rows)
    frame["remove"] = False
    return frame


@st.cache_data(show_spinner=False, max_entries=128)
def get_forecast_preview(asset_id: str, t0_step: int, t1_step: int) -> ops_dashboard_forecast.ForecastPreview:
    return ops_dashboard_forecast.build_forecast_preview(asset_id, t0_step=t0_step, t1_step=t1_step)


@st.cache_resource(show_spinner=False, max_entries=128)
def get_forecast_map_artifacts(
    asset_id: str,
    t0_step: int,
    t1_step: int,
    shift_lat: float,
    shift_lon: float,
    rotation_deg: float,
    multiplication_factor: float,
    opacity: float,
) -> ops_dashboard_forecast.ForecastMapComparisonArtifacts:
    request = ops_dashboard_forecast.ForecastPreviewRequest(
        asset_id=asset_id,
        t0_step=int(t0_step),
        t1_step=int(t1_step),
        shift_lat=float(shift_lat),
        shift_lon=float(shift_lon),
        rotation_deg=float(rotation_deg),
        multiplication_factor=float(multiplication_factor),
        opacity=float(opacity),
    )
    preview, corrected_preview = ops_dashboard_forecast.build_preview_pair_from_request(request)
    component_key = (
        f"forecast-map-{asset_id}-{int(t0_step)}-{int(t1_step)}-"
        f"{round(float(shift_lat), 4)}-{round(float(shift_lon), 4)}-"
        f"{round(float(rotation_deg), 4)}-{round(float(multiplication_factor), 4)}-"
        f"{round(float(opacity), 4)}"
    )
    return ops_dashboard_forecast.build_forecast_map_artifacts(
        preview,
        corrected_preview=corrected_preview,
        opacity=float(opacity),
        component_key=component_key,
    )


@st.cache_resource
def get_map_artifacts(
    map_cache_key: str,
    selected_layer_name: Optional[str],
    opacity: float,
) -> ops_dashboard_map.MapRenderArtifacts:
    del map_cache_key
    stations_df = ops_dashboard_data.load_station_catalog(days=DAYS_WINDOW)
    rivers_geojson = ops_dashboard_data.load_rivers_layer_geojson()
    raster_catalog = {str(item["name"]): item for item in ops_dashboard_data.list_accumulation_rasters()}
    base_map = ops_dashboard_map.build_ops_map(
        selected_layer_name,
        opacity,
        stations_df,
        rivers_geojson,
        raster_catalog,
    )
    return ops_dashboard_map.build_map_render_artifacts(base_map)


def initialize_session_state() -> None:
    st.session_state.setdefault("mini_id", None)
    st.session_state.setdefault("last_refresh_at", None)
    st.session_state.setdefault("forecast_applied_request", None)
    st.session_state.setdefault("forecast_edit_asset_id", None)
    st.session_state.setdefault("forecast_edit_draft", pd.DataFrame(columns=FORECAST_EDIT_COLUMNS))
    st.session_state.setdefault("forecast_edit_message", None)
    st.session_state.setdefault("forecast_edit_message_kind", "success")
    st.session_state.setdefault("forecast_add_prefill_signature", None)
    st.session_state.setdefault("forecast_last_editor", "")


def trigger_manual_refresh() -> None:
    st.cache_data.clear()
    st.cache_resource.clear()
    st.session_state["last_refresh_at"] = pd.Timestamp.now().strftime(REFRESH_TS_FORMAT)
    st.rerun()


def clear_saved_forecast_edits_cache(asset_id: str) -> None:
    try:
        get_saved_forecast_edits.clear(asset_id)
    except TypeError:
        get_saved_forecast_edits.clear()


def set_forecast_edit_message(message: str | None, *, kind: str = "success") -> None:
    st.session_state["forecast_edit_message"] = message
    st.session_state["forecast_edit_message_kind"] = kind


def empty_forecast_edit_frame() -> pd.DataFrame:
    frame = pd.DataFrame(columns=FORECAST_EDIT_COLUMNS)
    return normalize_forecast_edit_frame(frame)


def normalize_forecast_edit_frame(frame: pd.DataFrame | None) -> pd.DataFrame:
    normalized = pd.DataFrame() if frame is None else frame.copy()
    for column in FORECAST_EDIT_COLUMNS:
        if column not in normalized.columns:
            normalized[column] = pd.NA

    normalized["manual_edit_id"] = pd.to_numeric(normalized["manual_edit_id"], errors="coerce").astype("Int64")
    for column in FORECAST_EDIT_NUMERIC_COLUMNS:
        normalized[column] = pd.to_numeric(normalized[column], errors="coerce")
    normalized["asset_id"] = normalized["asset_id"].fillna("").astype(str)
    normalized["editor"] = normalized["editor"].fillna("").astype(str)
    normalized["reason"] = normalized["reason"].fillna("").astype(str)
    normalized["metadata_json"] = normalized["metadata_json"].fillna("{}").astype(str)
    normalized["created_at"] = normalized["created_at"].fillna("").astype(str)
    normalized["remove"] = normalized["remove"].fillna(False).astype(bool)
    return normalized[FORECAST_EDIT_COLUMNS]


def load_forecast_edit_draft(asset_id: str) -> pd.DataFrame:
    draft = get_saved_forecast_edits(asset_id)
    if draft.empty:
        return empty_forecast_edit_frame()
    draft = draft.copy()
    draft["asset_id"] = asset_id
    draft["remove"] = False
    return normalize_forecast_edit_frame(draft)


def build_default_forecast_instruction(
    asset_id: str,
    default_window: tuple[int, int],
    current_request: ops_dashboard_forecast.ForecastPreviewRequest | None,
) -> ForecastCorrectionInstruction:
    if current_request is not None and current_request.asset_id == asset_id:
        return build_forecast_instruction_from_request(current_request)
    return ForecastCorrectionInstruction(
        asset_id=asset_id,
        t0_step=int(default_window[0]),
        t1_step=int(default_window[1]),
        shift_lat=0.0,
        shift_lon=0.0,
        rotation_deg=0.0,
        multiplication_factor=1.0,
    )


def sync_forecast_add_form_state(
    *,
    asset_id: str,
    default_window: tuple[int, int],
    current_request: ops_dashboard_forecast.ForecastPreviewRequest | None,
) -> None:
    instruction = build_default_forecast_instruction(asset_id, default_window, current_request)
    signature = (
        asset_id,
        int(instruction.t0_step),
        int(instruction.t1_step),
        float(instruction.shift_lat),
        float(instruction.shift_lon),
        float(instruction.rotation_deg),
        float(instruction.multiplication_factor),
    )
    if st.session_state.get("forecast_add_prefill_signature") == signature:
        return

    st.session_state["forecast_add_t0_step"] = int(instruction.t0_step)
    st.session_state["forecast_add_t1_step"] = int(instruction.t1_step)
    st.session_state["forecast_add_shift_lat"] = float(instruction.shift_lat)
    st.session_state["forecast_add_shift_lon"] = float(instruction.shift_lon)
    st.session_state["forecast_add_rotation_deg"] = float(instruction.rotation_deg)
    st.session_state["forecast_add_multiplication_factor"] = float(instruction.multiplication_factor)
    st.session_state["forecast_add_editor"] = str(st.session_state.get("forecast_last_editor", "") or "")
    st.session_state["forecast_add_reason"] = ""
    st.session_state["forecast_add_prefill_signature"] = signature


def prepare_forecast_edit_workspace(asset_id: str) -> None:
    if st.session_state.get("forecast_edit_asset_id") == asset_id:
        return
    st.session_state["forecast_edit_asset_id"] = asset_id
    st.session_state["forecast_edit_draft"] = load_forecast_edit_draft(asset_id)
    set_forecast_edit_message(None)


def build_forecast_edit_row(
    *,
    asset_id: str,
    t0_step: int,
    t1_step: int,
    shift_lat: float,
    shift_lon: float,
    rotation_deg: float,
    multiplication_factor: float,
    editor: str,
    reason: str,
    metadata: dict[str, Any] | None = None,
) -> dict[str, object]:
    return {
        "manual_edit_id": pd.NA,
        "asset_id": asset_id,
        "t0_step": int(t0_step),
        "t1_step": int(t1_step),
        "shift_lat": float(shift_lat),
        "shift_lon": float(shift_lon),
        "rotation_deg": float(rotation_deg),
        "multiplication_factor": float(multiplication_factor),
        "editor": editor,
        "reason": reason,
        "metadata_json": json.dumps(metadata or {}, sort_keys=True, ensure_ascii=True),
        "created_at": "",
        "remove": False,
    }


def validate_forecast_edit_draft(asset_id: str, frame: pd.DataFrame) -> list[dict[str, Any]]:
    normalized = normalize_forecast_edit_frame(frame)
    active = normalized.loc[~normalized["remove"]].copy().reset_index(drop=True)
    rows_to_persist: list[dict[str, Any]] = []

    for row_index, row in enumerate(active.itertuples(index=False), start=1):
        if pd.isna(row.t0_step) or pd.isna(row.t1_step):
            raise ValueError(f"Linha {row_index}: t0_step e t1_step sao obrigatorios.")
        if pd.isna(row.multiplication_factor):
            raise ValueError(f"Linha {row_index}: multiplication_factor e obrigatorio.")

        t0_step = int(row.t0_step)
        t1_step = int(row.t1_step)
        if t1_step < t0_step:
            raise ValueError(f"Linha {row_index}: t1_step deve ser >= t0_step.")

        multiplication_factor = float(row.multiplication_factor)
        if multiplication_factor <= 0:
            raise ValueError(f"Linha {row_index}: multiplication_factor deve ser > 0.")

        reason = str(row.reason or "").strip()
        if not reason:
            raise ValueError(f"Linha {row_index}: motivo da correcao obrigatorio.")

        metadata_json = str(row.metadata_json or "{}").strip() or "{}"
        try:
            metadata = json.loads(metadata_json)
        except json.JSONDecodeError as exc:
            raise ValueError(f"Linha {row_index}: metadata_json invalido.") from exc

        rows_to_persist.append(
            {
                "asset_id": asset_id,
                "t0_step": t0_step,
                "t1_step": t1_step,
                "shift_lat": float(0.0 if pd.isna(row.shift_lat) else row.shift_lat),
                "shift_lon": float(0.0 if pd.isna(row.shift_lon) else row.shift_lon),
                "rotation_deg": float(0.0 if pd.isna(row.rotation_deg) else row.rotation_deg),
                "multiplication_factor": multiplication_factor,
                "editor": str(row.editor or "").strip() or None,
                "reason": reason,
                "metadata": metadata,
            }
        )

    ordered = sorted(rows_to_persist, key=lambda item: (int(item["t0_step"]), int(item["t1_step"])))
    for previous, current in zip(ordered, ordered[1:]):
        if int(current["t0_step"]) < int(previous["t1_step"]):
            raise ValueError(
                "Sobreposicao de correcoes na grade: "
                f"[{previous['t0_step']}, {previous['t1_step']}] x [{current['t0_step']}, {current['t1_step']}]."
            )

    return ordered


def format_mm(value: float | int | None) -> str:
    if value is None or pd.isna(value):
        return "indisponivel"
    return f"{float(value):.1f} mm"


def format_value(value: float | int | None, unit: str) -> str:
    if value is None or pd.isna(value):
        return "indisponivel"
    return f"{float(value):.2f} {unit}"


def format_timestamp(value: object | None, *, include_year: bool = False) -> str:
    if value is None or pd.isna(value):
        return "indisponivel"
    timestamp = pd.Timestamp(value)
    fmt = "%d/%m/%Y %H:%M" if include_year else "%d/%m %H:%M"
    return timestamp.strftime(fmt)


def network_summary(stations: pd.DataFrame) -> dict[str, float]:
    if stations.empty:
        return {
            "total": 0.0,
            "with_data": 0.0,
            "no_data": 0.0,
            "data_issue": 0.0,
            "rain_mean_24h": np.nan,
            "rain_p90_24h": np.nan,
        }

    rain_values = stations.loc[stations["status"] == "ok", "rain_acc_24h_mm"].dropna()
    if rain_values.empty:
        rain_values = stations["rain_acc_24h_mm"].dropna()

    return {
        "total": float(len(stations)),
        "with_data": float((stations["status"] == "ok").sum()),
        "no_data": float((stations["status"] == "no_data").sum()),
        "data_issue": float((stations["status"] == "data_issue").sum()),
        "rain_mean_24h": float(rain_values.mean()) if not rain_values.empty else np.nan,
        "rain_p90_24h": float(rain_values.quantile(0.9)) if not rain_values.empty else np.nan,
    }


def render_compact_summary_item(column, label: str, value: str) -> None:
    with column:
        st.caption(label)
        st.markdown(f"<div style='font-size:1rem;font-weight:600;white-space:nowrap'>{value}</div>", unsafe_allow_html=True)


def render_header_and_summary(stations: pd.DataFrame) -> None:
    summary = network_summary(stations)

    st.title("Sistema de alerta de cheias RS")
    st.caption(
        "Explorer operacional para postos observados, raster de chuva acumulada, series do MGB e previsao ECMWF. "
        "A aba ECMWF permite inspecao por passo e pre-visualizacao imediata de correcoes parametricas."
    )

    with st.container(border=True):
        st.subheader("Resumo da rede")
        items = [
            ("Postos totais", f"{int(summary['total'])}"),
            ("Com dados", f"{int(summary['with_data'])}"),
            ("Sem dados", f"{int(summary['no_data'])}"),
            ("Falha de dados", f"{int(summary['data_issue'])}"),
            ("Media chuva 24h", format_mm(summary["rain_mean_24h"])),
            ("P90 chuva 24h", format_mm(summary["rain_p90_24h"])),
        ]
        cols = st.columns(len(items))
        for col, (label, value) in zip(cols, items):
            render_compact_summary_item(col, label, value)


def render_station_summary_panel(row: Optional[pd.Series], observed_df: pd.DataFrame) -> None:
    with st.container(border=True):
        st.subheader("Resumo do posto")
        if row is None:
            st.caption("Clique em um posto no mapa para carregar os dados observados.")
            return

        station_name = str(row["station_name"]).strip() or "sem nome"
        provider = str(row["provider_code"]).upper()
        station_code = str(row["station_code"])
        st.markdown(f"**{station_name}**")

        if observed_df.empty:
            st.info("Sem dados observados para esta estacao na janela selecionada.")
            return

        metrics = ops_dashboard_data.compute_observed_metrics(observed_df)
        st.caption(f"{provider}:{station_code}")
        st.markdown(
            "Ultima leitura: {latest} | Chuva 12h: {rain_12h} | Chuva 24h: {rain_24h}".format(
                latest=format_timestamp(metrics["latest_time"]),
                rain_12h=format_mm(metrics["rain_12h"]),
                rain_24h=format_mm(metrics["rain_24h"]),
            )
        )
        st.markdown(
            "Chuva 72h: {rain_72h} | Nivel atual: {level} | Vazao atual: {flow}".format(
                rain_72h=format_mm(metrics["rain_72h"]),
                level=format_value(metrics["level_current"], "cm"),
                flow=format_value(metrics["flow_current"], "m3/s"),
            )
        )


def compute_mini_level_summary(df: pd.DataFrame, days: int) -> dict[str, float | pd.Timestamp | None]:
    if df.empty:
        return {
            "current_level": np.nan,
            "current_time": None,
            "recent_peak": np.nan,
            "forecast_peak": np.nan,
        }

    current_df = df[df["prev_flag"] == 0].copy().sort_values("dt")
    forecast_df = df[df["prev_flag"] == 1].copy().sort_values("dt")
    if current_df.empty:
        return {
            "current_level": np.nan,
            "current_time": None,
            "recent_peak": np.nan,
            "forecast_peak": np.nan,
        }

    latest_current = current_df.iloc[-1]
    current_time = pd.Timestamp(latest_current["dt"])
    recent_start = current_time - pd.Timedelta(days=days)
    forecast_end = current_time + pd.Timedelta(days=days)

    recent_window = current_df[current_df["dt"] >= recent_start]
    forecast_window = forecast_df[forecast_df["dt"] <= forecast_end]

    return {
        "current_level": float(latest_current["value"]) if pd.notna(latest_current["value"]) else np.nan,
        "current_time": current_time,
        "recent_peak": float(recent_window["value"].max()) if not recent_window.empty else np.nan,
        "forecast_peak": float(forecast_window["value"].max()) if not forecast_window.empty else np.nan,
    }


def render_mini_summary_panel(
    mini_id: Optional[int],
    y_series: pd.DataFrame,
    *,
    summary_days: int,
) -> int:
    with st.container(border=True):
        st.subheader("Resumo da mini")
        if mini_id is None:
            st.caption("Clique em uma geometria de rio no mapa para carregar a serie do modelo.")
            return summary_days

        st.markdown(f"**Mini {mini_id}**")
        summary_days = st.selectbox(
            "Janela do resumo (dias)",
            options=[3, 5, 7, 10, 15, 30],
            index=[3, 5, 7, 10, 15, 30].index(summary_days),
            key="mini_summary_days",
        )

        if y_series.empty:
            st.info("Sem serie de nivel para a mini selecionada.")
            return summary_days

        summary = compute_mini_level_summary(y_series, summary_days)
        st.markdown(
            "Nivel atual: {current_level} | Maior nivel ultimos {days} dias: {recent_peak}".format(
                current_level=format_value(summary["current_level"], "m"),
                days=summary_days,
                recent_peak=format_value(summary["recent_peak"], "m"),
            )
        )
        st.markdown(
            "Maior nivel proximos {days} dias: {forecast_peak} | Referencia atual: {current_time}".format(
                days=summary_days,
                forecast_peak=format_value(summary["forecast_peak"], "m"),
                current_time=format_timestamp(summary["current_time"], include_year=True),
            )
        )
        return summary_days


def lookup_variable_metadata(model_variables: pd.DataFrame, variable_code: Optional[str]) -> tuple[str, str]:
    if variable_code is None or model_variables.empty:
        return "-", "-"

    selected = model_variables[model_variables["variable_code"] == variable_code]
    if selected.empty:
        return str(variable_code), "-"

    row = selected.iloc[0]
    return str(row["display_name"]), str(row["unit"])


def time_series_chart(df: pd.DataFrame, station_uid: int, days: int) -> None:
    if df.empty:
        st.info("Sem dados para esta estacao/intervalo.")
        return

    fig = make_subplots(
        rows=2,
        cols=1,
        shared_xaxes=True,
        vertical_spacing=0.08,
        row_heights=[0.35, 0.65],
    )

    rain_df = df[df["variable_code"] == "rain"].dropna(subset=["value"])
    level_df = df[df["variable_code"] == "level"].dropna(subset=["value"])
    flow_df = df[df["variable_code"] == "flow"].dropna(subset=["value"])

    if not rain_df.empty:
        fig.add_bar(
            x=rain_df["datetime"],
            y=rain_df["value"],
            name="Chuva (mm)",
            marker_color="#4c6ef5",
            opacity=0.9,
            row=1,
            col=1,
        )
    else:
        fig.add_annotation(text="Chuva indisponivel", xref="paper", yref="paper", x=0.5, y=0.88, showarrow=False)

    if not level_df.empty:
        fig.add_scatter(
            x=level_df["datetime"],
            y=level_df["value"],
            name="Nivel (cm)",
            mode="lines+markers",
            line=dict(color="#0b7285", width=2),
            marker=dict(size=4),
            row=2,
            col=1,
        )
    if not flow_df.empty:
        fig.add_scatter(
            x=flow_df["datetime"],
            y=flow_df["value"],
            name="Vazao (m3/s)",
            mode="lines",
            line=dict(color="#f08c00", width=2),
            row=2,
            col=1,
        )
    if level_df.empty and flow_df.empty:
        fig.add_annotation(
            text="Nivel e vazao indisponiveis",
            xref="paper",
            yref="paper",
            x=0.5,
            y=0.22,
            showarrow=False,
        )

    fig.update_layout(
        template="plotly_white",
        height=520,
        margin=dict(t=30, r=20, l=10, b=30),
        legend=dict(orientation="h", yanchor="bottom", y=1.02, x=0),
        hovermode="x unified",
        xaxis=dict(title=""),
        xaxis2=dict(title="Data/hora"),
    )
    st.plotly_chart(fig, use_container_width=True, key=f"chart-{station_uid}-{days}")


def mgb_time_series_chart(
    df: pd.DataFrame,
    mini_id: int,
    variable_code: str,
    variable_display: str,
    unit: str,
    days_window: int,
    *,
    height: int = 420,
) -> None:
    if df.empty:
        st.info("Sem serie do modelo para a mini selecionada.")
        return

    current_df = df[df["prev_flag"] == 0]
    forecast_df = df[df["prev_flag"] == 1]

    fig = go.Figure()
    base_color = MGB_COLORS.get(variable_code, "#1864ab")
    forecast_color = "#d9480f"

    if not current_df.empty:
        fig.add_trace(
            go.Scatter(
                x=current_df["dt"],
                y=current_df["value"],
                mode="lines",
                name=f"{variable_display} atual",
                line=dict(color=base_color, width=2),
            )
        )
    if not forecast_df.empty:
        fig.add_trace(
            go.Scatter(
                x=forecast_df["dt"],
                y=forecast_df["value"],
                mode="lines",
                name=f"{variable_display} previsao",
                line=dict(color=forecast_color, width=2, dash="dash"),
            )
        )

    fig.update_layout(
        template="plotly_white",
        height=height,
        margin=dict(t=30, r=20, l=10, b=30),
        hovermode="x unified",
        legend=dict(orientation="h", yanchor="bottom", y=1.02, x=0),
        xaxis=dict(title="Data/hora"),
        yaxis=dict(title=f"{variable_display} ({unit})"),
    )
    st.plotly_chart(fig, use_container_width=True, key=f"mgb-chart-{variable_code}-{mini_id}-{days_window}")


def compute_map_cache_key(selected_layer_name: Optional[str], opacity: float) -> str:
    accumulation_rasters = get_accumulation_rasters()
    raster_catalog = {str(item["name"]): item for item in accumulation_rasters}
    raster_version = "no-raster"
    if selected_layer_name:
        meta = raster_catalog.get(selected_layer_name)
        if meta:
            raster_version = ops_dashboard_map.build_file_version(Path(str(meta["path"])))

    return ops_dashboard_map.build_map_cache_key(
        selected_layer_name=selected_layer_name,
        opacity=opacity,
        history_version=ops_dashboard_map.build_file_version(history_db_path()),
        rivers_version=ops_dashboard_map.build_file_version(ops_dashboard_data.LEGACY_RIVERS_GEOJSON_PATH),
        raster_version=raster_version,
        station_uid=st.session_state.get("station_uid"),
        mini_id=st.session_state.get("mini_id"),
    )


@st.fragment
def render_map_fragment(map_artifacts: ops_dashboard_map.MapRenderArtifacts) -> None:
    map_state = ops_dashboard_map.render_map_component(
        map_artifacts,
        height=620,
        use_container_width=True,
    )
    click_token = ops_dashboard_map.parse_click_token((map_state or {}).get("last_object_clicked_tooltip"))
    if ops_dashboard_map.update_selection_from_click_token(click_token, st.session_state):
        st.rerun()


def ensure_default_selection(stations_df: pd.DataFrame) -> None:
    if "station_uid" not in st.session_state and not stations_df.empty:
        preferred = stations_df[stations_df["status"] != "no_data"]
        default_station_uid = (
            int(preferred["station_uid"].iloc[0]) if not preferred.empty else int(stations_df["station_uid"].iloc[0])
        )
        st.session_state["station_uid"] = default_station_uid


def format_layer_option(option: str, raster_catalog: dict[str, dict[str, object]]) -> str:
    if option == NO_LAYER_OPTION:
        return NO_LAYER_OPTION
    return str(raster_catalog[option]["horizon_label"])


def render_sidebar_controls(
    accumulation_rasters: list[dict[str, object]],
    raster_catalog: dict[str, dict[str, object]],
) -> tuple[Optional[str], float]:
    layer_options = [NO_LAYER_OPTION] + [str(item["name"]) for item in accumulation_rasters]

    with st.sidebar:
        st.subheader("Controles")
        if st.button("Atualizar dados", use_container_width=True):
            trigger_manual_refresh()

        last_refresh = st.session_state.get("last_refresh_at")
        if last_refresh:
            st.caption(f"Ultima atualizacao manual: {last_refresh}")
        else:
            st.caption("Ultima atualizacao manual: nenhuma nesta sessao.")

        selected_layer_option = st.radio(
            "Raster de chuva acumulada",
            options=layer_options,
            format_func=lambda option: format_layer_option(option, raster_catalog),
            index=1 if accumulation_rasters else 0,
            key="selected_raster_layer_radio",
        )

        opacity = st.slider("Transparencia raster", min_value=0.0, max_value=1.0, value=0.6, step=0.05)
        st.caption("Camadas visiveis: postos, rios MGB e raster de chuva acumulada sobrepostos no mapa.")

    selected_layer_name = None if selected_layer_option == NO_LAYER_OPTION else selected_layer_option
    return selected_layer_name, opacity


def render_monitoring_tab(
    stations_df: pd.DataFrame,
    rivers_geojson: dict | None,
    model_variables: pd.DataFrame,
    selected_layer_name: Optional[str],
    opacity: float,
) -> None:
    map_cache_key = compute_map_cache_key(selected_layer_name, opacity)
    map_warning: Optional[str] = None
    try:
        map_artifacts = get_map_artifacts(map_cache_key, selected_layer_name, opacity)
    except RuntimeError as exc:
        map_warning = str(exc)
        fallback_key = compute_map_cache_key(None, opacity)
        map_artifacts = get_map_artifacts(fallback_key, None, opacity)

    station_uid = st.session_state.get("station_uid")
    station_uid = int(station_uid) if station_uid is not None else None
    mini_id = st.session_state.get("mini_id")
    mini_id = int(mini_id) if mini_id is not None else None

    observed_series = get_observed_series(station_uid, DAYS_WINDOW) if station_uid is not None else pd.DataFrame()
    selected_station_row: Optional[pd.Series] = None
    if station_uid is not None and not stations_df.empty:
        selected = stations_df[stations_df["station_uid"] == station_uid]
        if not selected.empty:
            selected_station_row = selected.iloc[0]

    with st.container(border=True):
        st.subheader("Mapa operacional")
        st.caption("Clique em um posto para dados observados ou em um trecho de rio para series do MGB.")
        if map_warning:
            st.warning(map_warning)
        if rivers_geojson is None:
            st.warning("Camada de rios nao encontrada em data/legacy/app_layers/rios_mini.geojson.")
        render_map_fragment(map_artifacts)

    lower_left, lower_right = st.columns(2)
    y_display, y_unit = lookup_variable_metadata(model_variables, "y")
    q_display, q_unit = lookup_variable_metadata(model_variables, "q")
    y_series = (
        get_mgb_series(mini_id=mini_id, variable_code="y", days_window=DAYS_WINDOW)
        if mini_id is not None
        else pd.DataFrame(columns=["dt", "prev_flag", "value", "variable_code", "display_name", "unit"])
    )
    q_series = (
        get_mgb_series(mini_id=mini_id, variable_code="q", days_window=DAYS_WINDOW)
        if mini_id is not None
        else pd.DataFrame(columns=["dt", "prev_flag", "value", "variable_code", "display_name", "unit"])
    )

    with lower_left:
        st.subheader("Dados de postos")
        render_station_summary_panel(selected_station_row, observed_series)

    with lower_right:
        st.subheader("Dados das minis")
        render_mini_summary_panel(
            mini_id,
            y_series,
            summary_days=st.session_state.get("mini_summary_days", 7),
        )

    chart_left, chart_right = st.columns(2)
    with chart_left:
        with st.container(border=True):
            st.subheader("Grafico do posto")
            if station_uid is None:
                st.info("Selecione um posto no mapa.")
            else:
                time_series_chart(observed_series, station_uid, DAYS_WINDOW)

    with chart_right:
        with st.container(border=True):
            st.subheader("Graficos da mini")
            if mini_id is None:
                st.info("Selecione uma mini no mapa.")
            else:
                mgb_time_series_chart(
                    y_series,
                    mini_id=mini_id,
                    variable_code="y",
                    variable_display=y_display,
                    unit=y_unit,
                    days_window=DAYS_WINDOW,
                    height=320,
                )
                mgb_time_series_chart(
                    q_series,
                    mini_id=mini_id,
                    variable_code="q",
                    variable_display=q_display,
                    unit=q_unit,
                    days_window=DAYS_WINDOW,
                    height=320,
                )


@st.fragment
def render_forecast_map_fragment(map_artifacts: ops_dashboard_forecast.ForecastMapComparisonArtifacts) -> None:
    ops_dashboard_map.render_map_component(
        map_artifacts.map_artifacts,
        height=520,
        use_container_width=True,
    )

    if map_artifacts.corrected is None:
        st.markdown(map_artifacts.original.legend_html, unsafe_allow_html=True)
        return

    original_col, corrected_col = st.columns(2)
    with original_col:
        st.caption("Original")
        st.markdown(map_artifacts.original.legend_html, unsafe_allow_html=True)
    with corrected_col:
        st.caption("Corrigido")
        st.markdown(map_artifacts.corrected.legend_html, unsafe_allow_html=True)


@st.fragment
def render_forecast_corrections_fragment(
    selected_asset_id: str,
    default_window: tuple[int, int],
    current_request: ops_dashboard_forecast.ForecastPreviewRequest | None,
    preview_metadata_json: str,
) -> None:
    prepare_forecast_edit_workspace(selected_asset_id)
    sync_forecast_add_form_state(
        asset_id=selected_asset_id,
        default_window=default_window,
        current_request=current_request,
    )

    message = st.session_state.get("forecast_edit_message")
    message_kind = str(st.session_state.get("forecast_edit_message_kind", "success"))
    if message:
        getattr(st, message_kind if message_kind in {"success", "warning", "error", "info"} else "info")(message)

    st.caption("Edite as linhas abaixo, marque `remove` para excluir e use `Salvar alteracoes` para persistir o conjunto final.")
    draft = normalize_forecast_edit_frame(st.session_state.get("forecast_edit_draft"))
    edited_draft = st.data_editor(
        draft,
        key=f"forecast_edit_table__{selected_asset_id}",
        hide_index=True,
        use_container_width=True,
        num_rows="fixed",
        column_order=[
            "manual_edit_id",
            "t0_step",
            "t1_step",
            "shift_lat",
            "shift_lon",
            "rotation_deg",
            "multiplication_factor",
            "editor",
            "reason",
            "created_at",
            "remove",
        ],
        disabled=["manual_edit_id", "created_at"],
        column_config={
            "asset_id": None,
            "metadata_json": None,
            "manual_edit_id": st.column_config.NumberColumn("ID", format="%d", width="small"),
            "t0_step": st.column_config.NumberColumn("t0_step", min_value=0, step=1, format="%d"),
            "t1_step": st.column_config.NumberColumn("t1_step", min_value=0, step=1, format="%d"),
            "shift_lat": st.column_config.NumberColumn("shift_lat", step=1.0, format="%.2f"),
            "shift_lon": st.column_config.NumberColumn("shift_lon", step=1.0, format="%.2f"),
            "rotation_deg": st.column_config.NumberColumn("rotation_deg", step=1.0, format="%.2f"),
            "multiplication_factor": st.column_config.NumberColumn(
                "multiplication_factor",
                min_value=0.01,
                step=0.05,
                format="%.2f",
            ),
            "editor": st.column_config.TextColumn("Editor"),
            "reason": st.column_config.TextColumn("Motivo da correcao", width="large"),
            "created_at": st.column_config.TextColumn("created_at", width="medium"),
            "remove": st.column_config.CheckboxColumn("remove"),
        },
    )
    edited_draft = normalize_forecast_edit_frame(edited_draft)
    edited_draft["asset_id"] = selected_asset_id
    st.session_state["forecast_edit_draft"] = edited_draft

    st.markdown("**Nova correcao**")
    with st.form(f"forecast_add_form__{selected_asset_id}"):
        add_left, add_right = st.columns([1.1, 0.9])
        with add_left:
            add_t0_step = st.number_input("t0_step", min_value=0, step=1, key="forecast_add_t0_step")
            add_t1_step = st.number_input("t1_step", min_value=0, step=1, key="forecast_add_t1_step")
            add_shift_lat = st.number_input("shift_lat", step=1.0, key="forecast_add_shift_lat")
            add_shift_lon = st.number_input("shift_lon", step=1.0, key="forecast_add_shift_lon")
        with add_right:
            add_rotation_deg = st.number_input("rotation_deg", step=1.0, key="forecast_add_rotation_deg")
            add_multiplication_factor = st.number_input(
                "multiplication_factor",
                min_value=0.01,
                step=0.05,
                key="forecast_add_multiplication_factor",
            )
            add_editor = st.text_input("Editor", key="forecast_add_editor")
            add_reason = st.text_input("Motivo da correcao", key="forecast_add_reason")
        add_clicked = st.form_submit_button("Adicionar correcao", use_container_width=True)

    if add_clicked:
        try:
            if int(add_t1_step) < int(add_t0_step):
                raise ValueError("Nova correcao: t1_step deve ser >= t0_step.")
            if float(add_multiplication_factor) <= 0:
                raise ValueError("Nova correcao: multiplication_factor deve ser > 0.")
            preview_metadata = json.loads(preview_metadata_json or "{}")
            new_row = build_forecast_edit_row(
                asset_id=selected_asset_id,
                t0_step=int(add_t0_step),
                t1_step=int(add_t1_step),
                shift_lat=float(add_shift_lat),
                shift_lon=float(add_shift_lon),
                rotation_deg=float(add_rotation_deg),
                multiplication_factor=float(add_multiplication_factor),
                editor=str(add_editor or "").strip(),
                reason=str(add_reason or "").strip(),
                metadata=preview_metadata if isinstance(preview_metadata, dict) else {},
            )
            next_draft = pd.concat([edited_draft, pd.DataFrame([new_row])], ignore_index=True)
            st.session_state["forecast_edit_draft"] = normalize_forecast_edit_frame(next_draft)
            st.session_state["forecast_last_editor"] = str(add_editor or "").strip()
            set_forecast_edit_message("Correcao adicionada ao draft.", kind="success")
            st.rerun(scope="fragment")
        except ValueError as exc:
            set_forecast_edit_message(str(exc), kind="warning")
            st.rerun(scope="fragment")

    save_col, info_col = st.columns([0.35, 0.65], vertical_alignment="center")
    with save_col:
        save_clicked = st.button("Salvar alteracoes", use_container_width=True, type="primary")
    with info_col:
        st.caption("O save substitui todas as correcoes do asset atual em uma unica transacao curta.")

    if save_clicked:
        try:
            rows_to_persist = validate_forecast_edit_draft(selected_asset_id, edited_draft)
            with HistoryRepository(history_db_path()) as repository:
                persisted_rows = repository.replace_forecast_manual_edits(selected_asset_id, rows_to_persist)
            clear_saved_forecast_edits_cache(selected_asset_id)
            st.session_state["forecast_edit_draft"] = normalize_forecast_edit_frame(pd.DataFrame(persisted_rows))
            st.session_state["forecast_edit_draft"]["asset_id"] = selected_asset_id
            if rows_to_persist:
                last_editor = rows_to_persist[-1].get("editor")
                if last_editor:
                    st.session_state["forecast_last_editor"] = str(last_editor)
            set_forecast_edit_message("Correcoes persistidas no history.sqlite.", kind="success")
            st.rerun(scope="fragment")
        except ValueError as exc:
            set_forecast_edit_message(str(exc), kind="warning")
            st.rerun(scope="fragment")
        except sqlite3.IntegrityError as exc:
            set_forecast_edit_message(f"Conflito no banco: {exc}", kind="error")
            st.rerun(scope="fragment")


def build_forecast_instruction_from_request(
    request: ops_dashboard_forecast.ForecastPreviewRequest,
) -> ForecastCorrectionInstruction:
    return ForecastCorrectionInstruction(
        asset_id=request.asset_id,
        t0_step=int(request.t0_step),
        t1_step=int(request.t1_step),
        shift_lat=float(request.shift_lat),
        shift_lon=float(request.shift_lon),
        rotation_deg=float(request.rotation_deg),
        multiplication_factor=float(request.multiplication_factor),
    )


def resolve_default_forecast_window(
    step_options: list[int],
    applied_request: ops_dashboard_forecast.ForecastPreviewRequest | None,
    asset_id: str,
) -> tuple[int, int]:
    if applied_request is not None and applied_request.asset_id == asset_id:
        candidate = (int(applied_request.t0_step), int(applied_request.t1_step))
        if candidate[0] in step_options and candidate[1] in step_options and candidate[0] <= candidate[1]:
            return candidate
    return int(step_options[0]), int(step_options[-1])


def render_forecast_tab() -> None:
    assets_df = get_forecast_assets()
    st.subheader("Chuva prevista ECMWF")
    st.caption(
        "Selecione um ciclo ECMWF canonico, ajuste a janela temporal e os parametros de correcao, "
        "e clique em Carregar mapas para atualizar o preview sincronizado."
    )
    if assets_df.empty:
        st.info("Nenhum asset ECMWF canonicamente registrado foi encontrado em data/history.sqlite.")
        return

    asset_options = assets_df["asset_id"].tolist()
    asset_lookup = dict(zip(assets_df["asset_id"], assets_df["display_label"]))
    selected_asset_id = st.selectbox(
        "Ciclo ECMWF",
        options=asset_options,
        format_func=lambda asset_id: asset_lookup.get(asset_id, asset_id),
        key="forecast_asset_id",
    )

    steps_df = get_forecast_steps(selected_asset_id)
    if steps_df.empty:
        st.warning("O asset selecionado nao possui mensagens de forecast legiveis.")
        return

    step_options = steps_df["step_hours"].astype(int).tolist()
    step_labels = {int(row.step_hours): str(row.label) for row in steps_df.itertuples()}
    applied_request = st.session_state.get("forecast_applied_request")
    if not isinstance(applied_request, ops_dashboard_forecast.ForecastPreviewRequest):
        applied_request = None
    default_window = resolve_default_forecast_window(step_options, applied_request, selected_asset_id)

    with st.form("forecast_preview_form"):
        controls_left, controls_right = st.columns([1.15, 0.85])
        with controls_left:
            st.markdown("**Parametros do preview**")
            selected_window = st.select_slider(
                "Janela temporal",
                options=step_options,
                value=default_window,
                format_func=lambda step: step_labels.get(int(step), str(step)),
                key=f"forecast_draft_step_window__{selected_asset_id}",
            )
            shift_lat = st.number_input("shift_lat (pixels)", value=0.0, step=1.0, key="forecast_draft_shift_lat")
            shift_lon = st.number_input("shift_lon (pixels)", value=0.0, step=1.0, key="forecast_draft_shift_lon")
            rotation_deg = st.number_input("rotation_deg", value=0.0, step=1.0, key="forecast_draft_rotation_deg")
            multiplication_factor = st.number_input(
                "multiplication_factor",
                min_value=0.01,
                value=1.0,
                step=0.05,
                key="forecast_draft_multiplication_factor",
            )
            opacity = st.slider(
                "Transparencia do mapa",
                min_value=0.1,
                max_value=1.0,
                value=0.75,
                step=0.05,
                key="forecast_draft_opacity",
            )
        with controls_right:
            st.markdown("**Passos disponiveis**")
            st.dataframe(
                steps_df[["step_hours", "valid_time"]].rename(
                    columns={"step_hours": "t", "valid_time": "valid_time_local"}
                ),
                hide_index=True,
                use_container_width=True,
            )
        load_preview = st.form_submit_button("Carregar mapas", use_container_width=True)

    if load_preview:
        st.session_state["forecast_applied_request"] = ops_dashboard_forecast.ForecastPreviewRequest(
            asset_id=selected_asset_id,
            t0_step=int(selected_window[0]),
            t1_step=int(selected_window[1]),
            shift_lat=float(shift_lat),
            shift_lon=float(shift_lon),
            rotation_deg=float(rotation_deg),
            multiplication_factor=float(multiplication_factor),
            opacity=float(opacity),
        )
        applied_request = st.session_state["forecast_applied_request"]

    current_request = applied_request if applied_request is not None and applied_request.asset_id == selected_asset_id else None
    preview: ops_dashboard_forecast.ForecastPreview | None = None

    if current_request is None:
        st.info("Ajuste os parametros e clique em `Carregar mapas` para montar o preview deste ciclo ECMWF.")
    else:
        preview = get_forecast_preview(current_request.asset_id, int(current_request.t0_step), int(current_request.t1_step))
        map_artifacts = get_forecast_map_artifacts(
            current_request.asset_id,
            int(current_request.t0_step),
            int(current_request.t1_step),
            float(current_request.shift_lat),
            float(current_request.shift_lon),
            float(current_request.rotation_deg),
            float(current_request.multiplication_factor),
            float(current_request.opacity),
        )

        selected_steps = [int(current_request.t0_step), int(current_request.t1_step)]
        selected_rows = steps_df.loc[steps_df["step_hours"].isin(selected_steps), ["step_hours", "valid_time"]].rename(
            columns={"step_hours": "t", "valid_time": "valid_time_local"}
        )
        with st.container(border=True):
            st.markdown("**Janela carregada**")
            st.caption(preview.title)
            st.dataframe(selected_rows, hide_index=True, use_container_width=True)

        with st.container(border=True):
            if current_request.has_correction:
                st.caption("Mapa original e corrigido sincronizados. Clique no raster para inspecionar os valores.")
            else:
                st.caption("Mapa do forecast. Clique no raster para inspecionar os valores.")
            render_forecast_map_fragment(map_artifacts)

        instruction = build_forecast_instruction_from_request(current_request)
        st.markdown("**Instrucao carregada**")
        st.dataframe(
            pd.DataFrame(
                [
                    {
                        "asset_id": instruction.asset_id,
                        "t0_step": instruction.t0_step,
                        "t1_step": instruction.t1_step,
                        "shift_lat": instruction.shift_lat,
                        "shift_lon": instruction.shift_lon,
                        "rotation_deg": instruction.rotation_deg,
                        "multiplication_factor": instruction.multiplication_factor,
                    }
                ]
            ),
            hide_index=True,
            use_container_width=True,
        )

    st.markdown("**Correcoes ECMWF**")
    preview_metadata_json = json.dumps(
        {"mode_label": preview.mode_label, "relative_path": preview.relative_path} if preview is not None else {},
        sort_keys=True,
        ensure_ascii=True,
    )
    render_forecast_corrections_fragment(
        selected_asset_id,
        default_window,
        current_request,
        preview_metadata_json,
    )


def main() -> None:
    initialize_session_state()

    stations_df = get_station_catalog(DAYS_WINDOW)
    rivers_geojson = get_rivers_geojson()
    model_variables = get_model_variables()
    accumulation_rasters = get_accumulation_rasters()
    raster_catalog = {str(item["name"]): item for item in accumulation_rasters}

    ensure_default_selection(stations_df)
    render_header_and_summary(stations_df)
    selected_layer_name, opacity = render_sidebar_controls(accumulation_rasters, raster_catalog)

    monitoring_tab, forecast_tab = st.tabs(["Monitoramento", "Chuva Prevista ECMWF"])
    with monitoring_tab:
        render_monitoring_tab(
            stations_df,
            rivers_geojson,
            model_variables,
            selected_layer_name,
            opacity,
        )
    with forecast_tab:
        render_forecast_tab()


if __name__ == "__main__":
    main()
