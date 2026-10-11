"""Report file contracts and offline HTML/PDF serialization."""
from __future__ import annotations

from dataclasses import dataclass
from html.parser import HTMLParser
import html
import json
import math
import os
from pathlib import Path
import re
import tempfile
from typing import Iterable, Literal, Mapping, Sequence

import geopandas as gpd
import pandas as pd
import plotly.graph_objects as go
from plotly.subplots import make_subplots
import plotly.io as pio
from plotly.offline import get_plotlyjs

from mgb_ops.assets.model_outputs import MGB_VARIABLE_METADATA

_DEFAULT_COLUMNS: tuple[dict[str, object], ...] = (
    {"label": "Chuva observada (24 h, mm)", "type": "obs", "variable": "rain", "time_horizon": -24, "operation": "accumulation"},
    {"label": "Nível atual (cm)", "type": "obs", "variable": "level", "time_horizon": 0, "operation": "value"},
    {"label": "Variação do nível (1 h, cm)", "type": "obs", "variable": "level", "time_horizon": -1, "operation": "change"},
    {"label": "Variação do nível (24 h, cm)", "type": "obs", "variable": "level", "time_horizon": -24, "operation": "change"},
    {"label": "Cota de inundação (cm)", "type": "threshold"},
    {"label": "Nível previsto +6 h (cm)", "type": "forecast", "variable": "level", "time_horizon": 6, "operation": "value"},
    {"label": "Nível previsto +12 h (cm)", "type": "forecast", "variable": "level", "time_horizon": 12, "operation": "value"},
)

@dataclass(frozen=True, slots=True)
class ReportWarning:
    code: str
    message: str
    station_id: str | None = None
    scenario_id: str | None = None
    column: str | None = None

@dataclass(frozen=True, slots=True)
class ReportResult:
    path: Path
    format: Literal["html", "pdf"]
    warnings: tuple[ReportWarning, ...]

class _ImageDetector(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.found = False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.found |= tag.lower() == "img"

    handle_startendtag = handle_starttag

def normalize_report_columns(columns: Sequence[Mapping[str, object]] | None) -> tuple[dict[str, object], ...]:
    if columns is None:
        return _DEFAULT_COLUMNS
    output: list[dict[str, object]] = []
    for position, raw in enumerate(columns, start=1):
        if not isinstance(raw, Mapping):
            raise ValueError(f"Table column {position} must be an object.")
        required = {"label", "variable", "type", "time_horizon", "operation"}
        missing = required.difference(raw)
        if missing:
            raise ValueError(f"Table column {position} is missing fields: {sorted(missing)}.")
        label = str(raw["label"] or "").strip()
        kind = str(raw["type"] or "").strip().lower()
        variable = str(raw["variable"] or "").strip().lower()
        operation = str(raw["operation"] or "").strip().lower()
        horizon = raw["time_horizon"]
        if not label:
            raise ValueError(f"Table column {position} label cannot be empty.")
        if kind not in {"obs", "forecast"}:
            raise ValueError(f"Table column {position} type must be 'obs' or 'forecast'.")
        allowed = {"rain", "level", "flow"} if kind == "obs" else set(MGB_VARIABLE_METADATA)
        if variable not in allowed:
            raise ValueError(f"Table column {position} has unsupported {kind} variable {variable!r}.")
        if operation not in {"value", "accumulation", "change"}:
            raise ValueError(f"Table column {position} operation must be value, accumulation, or change.")
        if not isinstance(horizon, int) or isinstance(horizon, bool):
            raise ValueError(f"Table column {position} time_horizon must be an integer number of hours.")
        output.append({
            "label": label, "variable": variable, "type": kind,
            "time_horizon": horizon, "operation": operation,
        })
    return tuple(output)

def format_report_number(value: float | None) -> str:
    if value is None or not math.isfinite(value):
        return "—"
    return f"{value:.1f}".replace(".", ",")

def format_report_range(values: Sequence[float]) -> str:
    lower, upper = min(values) * 0.9, max(values) * 1.1
    return f"{format_report_number(lower)}–{format_report_number(upper)}"

def _geometry_polygons(geometry) -> Iterable[object]:
    if geometry.geom_type == "Polygon":
        yield geometry
    elif geometry.geom_type in {"MultiPolygon", "GeometryCollection"}:
        for child in geometry.geoms:
            yield from _geometry_polygons(child)
    else:
        raise ValueError("Basin catchment geometry must contain polygons.")

def map_figure(basin, stations: Sequence[dict[str, object]], basin_ids: set[int], mini_segments: gpd.GeoDataFrame) -> go.Figure:
    figure = go.Figure()
    geometry = basin.geometry.geometry.iloc[0]
    all_x: list[float] = []
    all_y: list[float] = []
    for polygon in _geometry_polygons(geometry):
        points = list(polygon.exterior.coords)
        x, y = [float(point[0]) for point in points], [float(point[1]) for point in points]
        all_x.extend(x)
        all_y.extend(y)
        figure.add_trace(go.Scatter(
            x=x, y=y, mode="lines", fill="toself", fillcolor="rgba(25, 118, 120, 0.16)",
            line={"color": "#197678", "width": 1.5}, name="Bacia contribuinte",
            hoverinfo="skip", showlegend=len(figure.data) == 0,
        ))
    selected_segments = mini_segments[mini_segments["mini_id"].isin(basin_ids)]
    river_x: list[float | None] = []
    river_y: list[float | None] = []
    river_minis: list[int | None] = []

    def lines(geometry):
        if geometry.geom_type == "LineString":
            yield geometry
        elif geometry.geom_type in {"MultiLineString", "GeometryCollection"}:
            for child in geometry.geoms:
                yield from lines(child)
        else:
            raise ValueError("Layer 'mini_segments' must contain river line geometries.")

    for _, segment in selected_segments.iterrows():
        for line in lines(segment.geometry):
            points = list(line.coords)
            river_x.extend(float(point[0]) for point in points)
            river_y.extend(float(point[1]) for point in points)
            river_minis.extend([int(segment["mini_id"])] * len(points))
            river_x.append(None)
            river_y.append(None)
            river_minis.append(None)
    if not river_minis:
        raise ValueError("Layer 'mini_segments' has no river geometry for the selected basin.")
    all_x.extend(value for value in river_x if value is not None)
    all_y.extend(value for value in river_y if value is not None)
    figure.add_trace(go.Scatter(
        x=river_x, y=river_y, mode="lines", name="Rios da bacia",
        customdata=river_minis, line={"color": "#4a91b8", "width": 1.4},
        hoverinfo="skip",
    ))
    located = [row for row in stations if row.get("lat") is not None and row.get("lon") is not None]
    for inside, color, label in ((True, "#246b45", "Estação na bacia"), (False, "#b23b3b", "Estação fora da bacia")):
        selected = [row for row in located if (row.get("mini_id") in basin_ids) is inside]
        if not selected:
            continue
        x = [float(row["lon"]) for row in selected]
        y = [float(row["lat"]) for row in selected]
        all_x.extend(x)
        all_y.extend(y)
        figure.add_trace(go.Scatter(
            x=x, y=y, mode="markers", name=label,
            text=[str(row["station_name"]) for row in selected],
            customdata=[str(row["station_id"]) for row in selected],
            marker={"color": color, "size": 10, "line": {"color": "white", "width": 1}},
            hovertemplate="<b>%{text}</b><br>ID %{customdata}<br>%{y:.4f}, %{x:.4f}<extra></extra>",
        ))
    if not all_x or not all_y:
        raise ValueError("The basin map has no coordinates.")
    def limits(values: list[float]) -> list[float]:
        low, high = min(values), max(values)
        pad = max((high - low) * 0.08, 0.02)
        return [low - pad, high + pad]
    figure.update_layout(
        title="Bacia e estações selecionadas", height=560, margin={"l": 30, "r": 30, "t": 55, "b": 35},
        dragmode="pan", hovermode="closest", legend={"orientation": "h", "y": -0.16},
        xaxis={"title": "Longitude", "range": limits(all_x), "showgrid": True, "zeroline": False},
        yaxis={"title": "Latitude", "range": limits(all_y), "scaleanchor": "x", "scaleratio": 1, "showgrid": True, "zeroline": False},
    )
    return figure

def station_chart(
    station: dict[str, object],
    observed: pd.DataFrame,
    scenario_data: Mapping[str, Mapping[str, object]],
    scenario_labels: Mapping[str, str],
    thresholds: pd.DataFrame,
    *,
    reference: pd.Timestamp,
    window_start: pd.Timestamp,
    window_end: pd.Timestamp,
) -> go.Figure:
    station_id = str(station["station_id"])
    title = str(station["station_name"])
    figure = make_subplots(
        rows=2, cols=1, shared_xaxes=True, vertical_spacing=0.12,
        subplot_titles=(f"Nível do rio em {title}", "Chuva média ponderada da bacia"),
    )
    levels = observed[
        (observed["variable_code"] == "level")
        & (observed["datetime"] >= window_start)
        & (observed["datetime"] <= reference)
        & observed["value"].notna()
    ]
    figure.add_trace(go.Scatter(
        x=levels["datetime"].astype(str), y=levels["value"], mode="lines+markers",
        name="Nível observado", line={"color": "#263f73", "width": 2}, marker={"size": 4},
    ), row=1, col=1)
    for scenario_id, data in scenario_data.items():
        prepared = data.get("level")
        if isinstance(prepared, pd.DataFrame):
            forecasts = prepared[
                (prepared["prev_flag"] == 1)
                & (prepared["dt"] > reference)
                & (prepared["dt"] >= window_start)
                & (prepared["dt"] < window_end)
                & prepared["value"].notna()
            ]
            if not forecasts.empty:
                figure.add_trace(go.Scatter(
                    x=forecasts["dt"].astype(str), y=forecasts["value"], mode="lines",
                    name=f"Nível previsto · {scenario_labels[scenario_id]}",
                    line={"width": 2, "dash": "dash"},
                ), row=1, col=1)
        rain = data.get("basin_rain")
        if isinstance(rain, pd.DataFrame):
            selected = rain[(rain["dt"] >= window_start) & (rain["dt"] < window_end)]
            current = selected[(selected["prev_flag"] == 0) & (selected["dt"] <= reference)]
            forecast = selected[(selected["prev_flag"] == 1) & (selected["dt"] > reference)]
            if current.attrs.get("show_observed", False) and not current.empty:
                figure.add_trace(go.Bar(
                    x=current["dt"].astype(str), y=current["value"], name="Chuva observada ponderada",
                    marker_color="rgba(48, 122, 173, 0.65)",
                ), row=2, col=1)
            if not forecast.empty:
                figure.add_trace(go.Scatter(
                    x=forecast["dt"].astype(str), y=forecast["value"], mode="lines",
                    name=f"Chuva prevista · {scenario_labels[scenario_id]}",
                    line={"width": 1.7, "dash": "dash"},
                ), row=2, col=1)
    colors = {"attention": "#d7a900", "alert": "#e57c16", "flood": "#bd3333"}
    for reference_type in ("attention", "alert", "flood"):
        if thresholds.empty:
            selected = thresholds
        else:
            numeric_levels = pd.to_numeric(thresholds["level_cm"], errors="coerce")
            selected = thresholds.loc[
                (thresholds["reference_type"] == reference_type)
                & numeric_levels.map(lambda value: pd.notna(value) and math.isfinite(float(value)))
            ]
        if selected.empty:
            continue
        level = float(selected.iloc[0]["level_cm"])
        figure.add_hline(
            y=level, line_dash="dot", line_color=colors[reference_type],
            annotation_text={"attention": "Atenção", "alert": "Alerta", "flood": "Inundação"}[reference_type],
            row=1, col=1,
        )
    figure.update_layout(
        title=f"{title} · {station_id}", height=760, margin={"l": 55, "r": 50, "t": 80, "b": 45},
        hovermode="x unified", legend={"orientation": "h", "y": -0.14},
        barmode="overlay", dragmode="zoom",
    )
    figure.update_yaxes(title_text="Nível (cm)", row=1, col=1)
    figure.update_yaxes(title_text="Chuva (mm)", row=2, col=1)
    figure.update_xaxes(title_text="Horário local (America/Sao_Paulo)", row=2, col=1)
    return figure

def _markdown_html(markdown_path: Path | None) -> str:
    if markdown_path is None:
        return ""
    source = Path(markdown_path).read_text(encoding="utf-8")
    if re.search(r"<\s*(?:img|iframe|audio|video|source|object|embed|link|script)\b", source, re.IGNORECASE):
        raise ValueError("Markdown cannot embed external HTML resources.")
    # Escaping before Markdown keeps raw HTML inert while preserving Markdown syntax.
    import markdown

    rendered = markdown.markdown(
        html.escape(source, quote=False), extensions=["tables", "fenced_code"]
    )
    detector = _ImageDetector()
    detector.feed(rendered)
    if detector.found:
        raise ValueError("Markdown images are unsupported in standalone reports.")
    return rendered

def _html_document(
    *,
    title: str,
    author: str,
    basin_name: str,
    reference: pd.Timestamp,
    window_start: pd.Timestamp,
    window_end: pd.Timestamp,
    headers: Sequence[str],
    rows: Sequence[Sequence[str]],
    figures: Sequence[tuple[str, go.Figure]],
    markdown_body: str,
    warnings: Sequence[ReportWarning],
) -> str:
    header = "".join(
        f'<th scope="col" aria-sort="none"><button class="sort-button" type="button" data-column="{index}" '
        f'aria-label="Ordenar por {html.escape(label, quote=True)}">{html.escape(label)}</button></th>'
        for index, label in enumerate(headers)
    )
    body = "".join(
        "<tr>" + "".join(f"<td>{html.escape(str(value))}</td>" for value in row) + "</tr>"
        for row in rows
    )
    warning_items = "".join(
        f'<li data-code="{html.escape(item.code, quote=True)}">{html.escape(item.message)}</li>'
        for item in warnings
    ) or "<li>Nenhum aviso.</li>"
    figure_sections = "".join(
        f'<section class="plot-section"><div class="plot" id="{html.escape(element_id, quote=True)}" '
        f'aria-label="Gráfico interativo"></div></section>'
        for element_id, _ in figures
    )
    plot_specs = []
    for element_id, figure in figures:
        figure_json = pio.to_json(figure, validate=True).replace("</", "<\\/")
        plot_specs.append(
            "{id:" + json.dumps(element_id, ensure_ascii=False) + ", figure:" + figure_json + "}"
        )
    plots = ",".join(plot_specs)
    return f'''<!doctype html>
<html lang="pt-BR"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{html.escape(title)}</title><style>
:root {{ color-scheme: light; font-family: Arial, sans-serif; color: #17222d; }}
body {{ max-width: 1180px; margin: 0 auto; padding: 24px; background: #fff; }}
h1,h2 {{ color: #153d45; }} header {{ border-bottom: 2px solid #197678; margin-bottom: 22px; }}
.meta {{ color: #52626d; line-height: 1.6; }} section {{ margin: 26px 0; }}
.table-controls {{ margin: 10px 0; }} table {{ width: 100%; border-collapse: collapse; font-size: 14px; }}
th,td {{ border: 1px solid #cbd5dc; padding: 8px; text-align: left; }}
th {{ background: #eaf1f2; }} tbody tr:nth-child(even) {{ background: #f6f8f8; }}
.sort-button {{ width: 100%; border: 0; background: transparent; color: inherit; text-align: left; font: inherit; font-weight: bold; cursor: pointer; }}
.plot {{ width: 100%; min-height: 560px; }} .plot-section {{ break-inside: avoid; page-break-inside: avoid; }}
.warnings {{ border-left: 4px solid #d7a900; padding-left: 14px; }}
.markdown-summary {{ overflow-wrap: anywhere; }} .markdown-summary pre {{ white-space: pre-wrap; }}
@media print {{ body {{ max-width: none; padding: 0; }} .screen-controls {{ display: none !important; }}
  section.plot-section {{ break-inside: avoid; page-break-inside: avoid; }} .plot {{ height: 690px; }}
  table {{ font-size: 9pt; }} tr {{ break-inside: avoid; page-break-inside: avoid; }}
}}
</style></head><body>
<header><h1>{html.escape(title)}</h1><p class="meta"><strong>Bacia:</strong> {html.escape(basin_name)}<br>
<strong>Referência:</strong> {reference:%d/%m/%Y %H:%M} (America/Sao_Paulo)<br>
<strong>Janela:</strong> {window_start:%d/%m/%Y %H:%M} a {window_end:%d/%m/%Y %H:%M} (fim exclusivo)<br>
<strong>Autoria:</strong> {html.escape(author)}</p></header>
<section><h2>Estações e níveis</h2><div class="table-controls screen-controls"><label for="station-filter">Filtrar estações: </label>
<input id="station-filter" type="search" autocomplete="off"></div><table id="station-table">
<thead><tr>{header}</tr></thead><tbody>{body}</tbody></table></section>
<section><h2>Mapa</h2><div class="plot" id="report-map" aria-label="Mapa interativo"></div></section>
<section><h2>Séries temporais</h2>{figure_sections or '<p>Nenhuma estação de gráfico selecionada.</p>'}</section>
<section class="markdown-summary"><h2>Resumo</h2>{markdown_body or '<p>Sem resumo adicional.</p>'}</section>
<section class="warnings"><h2>Avisos</h2><ul>{warning_items}</ul></section>
<script>{get_plotlyjs()}</script><script>
window.reportReady = false;
const reportPlots = [{plots}];
const plotConfig = {{responsive:true, scrollZoom:true, displaylogo:false}};
Promise.all(reportPlots.map(item => Plotly.newPlot(item.id, item.figure.data, item.figure.layout, plotConfig)))
  .then(() => {{ window.reportReady = true; }});
const table = document.getElementById('station-table');
document.querySelectorAll('.sort-button').forEach(button => {{
  button.addEventListener('click', () => {{
    const index = Number(button.dataset.column), body = table.tBodies[0];
    const direction = button.dataset.direction === 'asc' ? 'desc' : 'asc';
    button.dataset.direction = direction;
    [...body.rows].sort((left, right) => {{
      const a = left.cells[index].textContent.trim(), b = right.cells[index].textContent.trim();
      return a.localeCompare(b, 'pt-BR', {{numeric:true, sensitivity:'base'}}) * (direction === 'asc' ? 1 : -1);
    }}).forEach(row => body.appendChild(row));
    button.closest('th').setAttribute('aria-sort', direction === 'asc' ? 'ascending' : 'descending');
  }});
}});
document.getElementById('station-filter').addEventListener('input', event => {{
  const term = event.target.value.trim().toLocaleLowerCase('pt-BR');
  [...table.tBodies[0].rows].forEach(row => {{ row.hidden = !row.textContent.toLocaleLowerCase('pt-BR').includes(term); }});
}});
</script></body></html>'''

def _atomic_html(path: Path, value: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.name}.", suffix=".html", dir=path.parent)
    try:
        with os.fdopen(fd, "w", encoding="utf-8", newline="") as handle:
            handle.write(value)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)

def _atomic_pdf(path: Path, value: str) -> None:
    from playwright.sync_api import sync_playwright

    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=f".{path.stem}.", suffix=".pdf", dir=path.parent)
    os.close(fd)
    try:
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch()
            try:
                page = browser.new_page()
                page.set_content(value, wait_until="load")
                page.wait_for_function("window.reportReady === true", timeout=60_000)
                page.pdf(
                    path=temporary, format="A4", print_background=True,
                    prefer_css_page_size=True, display_header_footer=False,
                )
            finally:
                browser.close()
        if not Path(temporary).is_file() or Path(temporary).stat().st_size == 0:
            raise RuntimeError("Chromium did not produce a PDF report.")
        os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def write_report(
    *, output_path: Path, format: Literal["html", "pdf"], title: str, author: str, basin_name: str,
    reference: pd.Timestamp, window_start: pd.Timestamp, window_end: pd.Timestamp,
    headers: Sequence[str], rows: Sequence[Sequence[str]], figures: Sequence[tuple[str, go.Figure]],
    markdown_path: Path | None, warnings: Sequence[ReportWarning],
) -> None:
    document = _html_document(
        title=title, author=author, basin_name=basin_name, reference=reference,
        window_start=window_start, window_end=window_end, headers=headers, rows=rows,
        figures=figures, markdown_body=_markdown_html(markdown_path), warnings=warnings,
    )
    if format == "html":
        _atomic_html(output_path, document)
    else:
        _atomic_pdf(output_path, document)


__all__ = [
    "ReportResult", "ReportWarning", "format_report_number", "format_report_range",
    "map_figure", "normalize_report_columns", "station_chart", "write_report",
]
