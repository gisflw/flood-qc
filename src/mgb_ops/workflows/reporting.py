"""Orchestration for generating offline reports from saved run artifacts."""
from __future__ import annotations

from datetime import datetime
import math
from pathlib import Path
from typing import Literal, Mapping, Sequence

import pandas as pd

from mgb_ops.analysis.basins import basin_spatial_data
from mgb_ops.analysis.reporting import (
    clean_report_timeseries, evaluate_report_series, load_report_stations, local_report_time,
)
from mgb_ops.analysis.adjust import prepare_mgb_level_series
from mgb_ops.analysis.timeseries import (
    load_basin_precipitation, load_observed_series, load_station_reference_levels,
)
from mgb_ops.analysis.windows import build_analysis_window
from mgb_ops.assets.current_run import CurrentRunArtifact, load_current_artifact
from mgb_ops.assets.model_outputs import load_mgb_series
from mgb_ops.assets.report import (
    ReportResult, ReportWarning, format_report_number, format_report_range, map_figure,
    normalize_report_columns, station_chart, write_report,
)
from mgb_ops.assets.scenario_cache import read_scenario_cache, scenario_cache_root
from mgb_ops.assets.spatial_layers import read_mini_layer
from mgb_ops.assets.types import AnalysisWindow

def _station_ids(values: Sequence[str | int], *, name: str) -> tuple[str, ...]:
    ids = tuple(str(value).strip() for value in values)
    if any(not value for value in ids):
        raise ValueError(f"{name} cannot contain empty station IDs.")
    if len(ids) != len(set(ids)):
        raise ValueError(f"{name} cannot contain duplicate station IDs.")
    return ids

def _selected_scenarios(
    artifact: CurrentRunArtifact, requested: Sequence[str] | None, warn
) -> tuple[tuple[object, ...], bool]:
    by_id = {scenario.scenario_id: scenario for scenario in artifact.scenarios}
    if requested is not None:
        ids = tuple(str(value).strip() for value in requested)
        if not ids or any(not value for value in ids):
            raise ValueError("scenarios must contain at least one canonical scenario ID.")
        if len(ids) != len(set(ids)):
            raise ValueError("scenarios cannot contain duplicate IDs.")
        missing = [value for value in ids if value not in by_id]
        if missing:
            raise ValueError(f"Scenario IDs were not found in the selected run: {missing}.")
        return tuple(by_id[value] for value in ids), True

    selected = []
    missing_providers = []
    for provider in ("ecmwf", "noaa"):
        matches = [
            item for item in artifact.scenarios
            if item.kind == "raw" and str(item.provider_code or "").lower() == provider
        ]
        if len(matches) > 1:
            raise ValueError(f"The selected run has multiple raw {provider.upper()} scenarios.")
        if matches:
            selected.extend(matches)
        else:
            missing_providers.append(provider)
            warn(
                "default_scenario_missing",
                f"O cenário bruto {provider.upper()} não está registrado na execução selecionada.",
                scenario_id=None,
            )
    return tuple(selected), not missing_providers

def _cache_path(cache_dir: Path, reference: str) -> Path:
    value = Path(reference)
    parts = value.parts
    if "forecast_scenarios" in parts:
        suffix = parts[parts.index("forecast_scenarios") + 1 :]
    else:
        suffix = (value.name,)
    if not suffix:
        raise ValueError(f"Invalid published scenario cache path: {reference!r}.")
    root = scenario_cache_root(Path(cache_dir)).resolve()
    target = root.joinpath(*suffix).resolve()
    try:
        target.relative_to(root)
    except ValueError as exc:
        raise ValueError(f"Published cache path escapes the cache directory: {reference!r}.") from exc
    return target

def _load_scenario_caches(
    artifact: CurrentRunArtifact,
    cache_dir: Path,
    scenarios: Sequence[object],
    expected_window: AnalysisWindow,
    warn,
) -> dict[str, Path | None]:
    output: dict[str, Path | None] = {}
    for scenario in scenarios:
        scenario_id = str(scenario.scenario_id)
        reference = artifact.published_caches.get(scenario_id)
        if not reference:
            warn(
                "scenario_cache_unpublished",
                f"A execução não publicou dados para o cenário {scenario.label}.",
                scenario_id=scenario_id,
            )
            output[scenario_id] = None
            continue
        path = _cache_path(cache_dir, reference)
        if not path.is_file():
            warn(
                "scenario_cache_missing",
                f"O arquivo publicado do cenário {scenario.label} não existe: {path.name}.",
                scenario_id=scenario_id,
            )
            output[scenario_id] = None
            continue
        metadata = read_scenario_cache(path)
        cache_reference = local_report_time(metadata.reference_time) if metadata.reference_time else None
        cache_window = metadata.window
        if (
            metadata.scenario_id != scenario_id
            or cache_reference is None
            or cache_reference != local_report_time(expected_window.cutoff_time)
            or cache_window is None
            or local_report_time(cache_window.start_time) != local_report_time(expected_window.start_time)
            or local_report_time(cache_window.cutoff_time) != local_report_time(expected_window.cutoff_time)
            or local_report_time(cache_window.forecast_end_exclusive)
            != local_report_time(expected_window.forecast_end_exclusive)
        ):
            warn(
                "scenario_cache_stale",
                f"O arquivo do cenário {scenario.label} não pertence à referência da execução selecionada.",
                scenario_id=scenario_id,
            )
            output[scenario_id] = None
            continue
        output[scenario_id] = path
    return output

def generate_report(
    *,
    history_path: Path,
    spatial_path: Path,
    run_artifact_path: Path,
    cache_dir: Path,
    output_dir: Path,
    output_basin_name: str,
    format: Literal["html", "pdf"],
    outlet_mini_id: int,
    map_station_ids: Sequence[str | int],
    chart_station_ids: Sequence[str | int],
    markdown_path: Path | None = None,
    title: str,
    author: str,
    basin_name: str,
    reference_time: datetime | str | None = None,
    window_start: datetime | str | None = None,
    window_end: datetime | str | None = None,
    scenarios: Sequence[str] | None = None,
    table_columns: Sequence[Mapping[str, object]] | None = None,
) -> ReportResult:
    """Generate a self-contained report from canonical history, spatial, and run artifacts."""
    if format not in {"html", "pdf"}:
        raise ValueError("format must be 'html' or 'pdf'.")
    if not str(title).strip() or not str(author).strip() or not str(basin_name).strip():
        raise ValueError("title, author, and basin_name must be non-empty.")
    output_basin_name = str(output_basin_name).strip()
    if (
        not output_basin_name
        or output_basin_name in {".", ".."}
        or any(char in output_basin_name for char in '/\\<>:"|?*\x00')
        or any(ord(char) < 32 for char in output_basin_name)
    ):
        raise ValueError("output_basin_name must be a non-empty filename component.")
    history_path, spatial_path = Path(history_path), Path(spatial_path)
    run_artifact_path, cache_dir, output_dir = Path(run_artifact_path), Path(cache_dir), Path(output_dir)
    protected_paths = [history_path, spatial_path, run_artifact_path]
    if markdown_path is not None:
        protected_paths.append(Path(markdown_path))
    if output_dir.resolve().is_relative_to(cache_dir.resolve()):
        raise ValueError("output_dir must be outside the scenario cache directory.")
    for path, label in (
        (history_path, "History database"), (spatial_path, "Spatial GeoPackage"),
        (run_artifact_path, "Current-run artifact"),
    ):
        if not path.is_file():
            raise FileNotFoundError(f"{label} not found: {path}")

    artifact = load_current_artifact(run_artifact_path)
    run_reference = local_report_time(artifact.reference_time)
    reference = local_report_time(reference_time) if reference_time is not None else run_reference
    if reference.minute or reference.second or reference.microsecond:
        raise ValueError("reference_time must be aligned to an exact hour.")
    default_window = build_analysis_window(
        run_reference.to_pydatetime(),
        observed_horizon_days=int(artifact.mgb_settings["observed_horizon_days"]),
        forecast_horizon_days=int(artifact.mgb_settings["forecast_horizon_days"]),
    )
    if local_report_time(artifact.forecast_end_exclusive) != local_report_time(default_window.forecast_end_exclusive):
        raise ValueError("Current-run artifact horizon settings do not match forecast_end_exclusive.")
    start = local_report_time(window_start) if window_start is not None else local_report_time(default_window.start_time)
    end = local_report_time(window_end) if window_end is not None else local_report_time(default_window.forecast_end_exclusive)
    if not start <= reference < end:
        raise ValueError("Report window must contain reference_time and satisfy start < end.")
    output_path = output_dir / f"{reference:%Y%m%d_%H}_{output_basin_name}.{format}"
    if any(output_path.resolve() == path.resolve() for path in protected_paths):
        raise ValueError("Generated output path must not overwrite a report input.")

    map_ids = _station_ids(map_station_ids, name="map_station_ids")
    chart_ids = _station_ids(chart_station_ids, name="chart_station_ids")
    columns = normalize_report_columns(table_columns)
    stations = load_report_stations(history_path, tuple(dict.fromkeys((*map_ids, *chart_ids))))
    catchments = read_mini_layer(spatial_path, "mini_catchments")
    basin = basin_spatial_data(catchments, int(outlet_mini_id))
    mini_segments = read_mini_layer(spatial_path, "mini_segments")
    basin_ids = set(basin.mini_ids)

    warning_items: list[ReportWarning] = []
    warning_keys: set[tuple[object, ...]] = set()

    def warn(
        code: str,
        message: str,
        *,
        station_id: str | None = None,
        scenario_id: str | None = None,
        column: str | None = None,
    ) -> None:
        warning = ReportWarning(code, message, station_id, scenario_id, column)
        key = (code, message, station_id, scenario_id, column)
        if key not in warning_keys:
            warning_keys.add(key)
            warning_items.append(warning)

    map_stations = [stations[value] for value in map_ids]
    for station in map_stations:
        if station["mini_id"] not in basin_ids:
            warn(
                "station_outside_basin",
                f"A estação {station['station_name']} está fora da bacia contribuinte.",
                station_id=str(station["station_id"]),
            )
        if station["lat"] is None or station["lon"] is None:
            warn(
                "station_without_coordinates",
                f"A estação {station['station_name']} não tem coordenadas e não aparece no mapa.",
                station_id=str(station["station_id"]),
            )
    for station_id in chart_ids:
        if stations[station_id]["mini_id"] is None:
            raise ValueError(f"Chart station {station_id!r} has no mini_id association.")

    selected_scenarios, default_scenarios_complete = _selected_scenarios(artifact, scenarios, warn)
    scenario_labels = {str(item.scenario_id): str(item.label) for item in selected_scenarios}
    cache_paths = _load_scenario_caches(
        artifact, cache_dir, selected_scenarios, default_window, warn
    )

    horizons = [int(column["time_horizon"]) for column in columns if column.get("type") in {"obs", "forecast"}]
    query_start = min([start, reference - pd.Timedelta(hours=24), *(reference + pd.Timedelta(hours=h) for h in horizons if h < 0)])
    query_end = max([reference, *(reference + pd.Timedelta(hours=h) for h in horizons if h > 0)])
    observed: dict[str, pd.DataFrame] = {}
    for station_id in stations:
        observed[station_id] = clean_report_timeseries(
            load_observed_series(
                station_id, history_path,
                start_time=query_start.to_pydatetime(), end_time=query_end.to_pydatetime(),
            ),
            time_col="datetime",
        )
    references = {
        station_id: load_station_reference_levels(station_id, history_path)
        for station_id in stations
    }

    model_series: dict[tuple[str, int, str], pd.DataFrame | None] = {}
    adjusted_levels: dict[tuple[str, str], pd.DataFrame | None] = {}
    basin_rain_series: dict[tuple[int, str], pd.DataFrame | None] = {}

    def get_model_series(scenario_id: str, mini_id: int | None, variable: str, station_id: str) -> pd.DataFrame | None:
        if mini_id is None:
            return None
        key = (scenario_id, mini_id, variable)
        if key in model_series:
            return model_series[key]
        path = cache_paths.get(scenario_id)
        if path is None:
            model_series[key] = None
            return None
        try:
            frame = clean_report_timeseries(
                load_mgb_series(path, mini_id=mini_id, variable_code=variable), time_col="dt"
            )
        except ValueError as exc:
            warn(
                "forecast_data_missing",
                f"Não há série {variable} completa para a estação {station_id} no cenário {scenario_labels[scenario_id]}: {exc}.",
                station_id=station_id, scenario_id=scenario_id,
            )
            model_series[key] = None
            return None
        model_series[key] = frame
        return frame

    def get_adjusted_level(scenario_id: str, station: Mapping[str, object]) -> pd.DataFrame | None:
        station_id = str(station["station_id"])
        key = (scenario_id, station_id)
        if key in adjusted_levels:
            return adjusted_levels[key]
        raw = get_model_series(scenario_id, station["mini_id"], "level", station_id)
        if raw is None:
            adjusted_levels[key] = None
            return None
        calibration_observations = observed[station_id][observed[station_id]["datetime"] <= reference]
        prepared = prepare_mgb_level_series(raw, calibration_observations)
        if prepared.empty:
            warn(
                "calibration_unavailable",
                f"Não foi possível calibrar os níveis previstos de {station['station_name']} no cenário {scenario_labels[scenario_id]}: faltam observações coincidentes.",
                station_id=station_id, scenario_id=scenario_id,
            )
        adjusted_levels[key] = prepared
        return prepared

    station_basins = {
        int(stations[station_id]["mini_id"]): basin_spatial_data(
            catchments, int(stations[station_id]["mini_id"])
        )
        for station_id in chart_ids
    }

    def get_basin_rain(scenario_id: str, mini_id: int, station_id: str) -> pd.DataFrame | None:
        key = (mini_id, scenario_id)
        if key in basin_rain_series:
            return basin_rain_series[key]
        path = cache_paths.get(scenario_id)
        if path is None:
            basin_rain_series[key] = None
            return None
        station_basin = station_basins[mini_id]
        try:
            frame = clean_report_timeseries(
                load_basin_precipitation(
                    path, mini_ids=station_basin.mini_ids, weights=station_basin.weights,
                ),
                time_col="dt",
            )
        except ValueError as exc:
            warn(
                "basin_rainfall_missing",
                f"Não foi possível calcular a chuva média ponderada no cenário {scenario_labels[scenario_id]}: {exc}",
                station_id=station_id, scenario_id=scenario_id,
            )
            frame = None
        basin_rain_series[key] = frame
        return frame

    def in_window(value: pd.Timestamp, *, span_start: pd.Timestamp | None = None) -> bool:
        return start <= value < end and (span_start is None or span_start >= start)

    def metric_value(station: dict[str, object], column: Mapping[str, object]) -> float | None:
        station_id = str(station["station_id"])
        kind = str(column.get("type"))
        if kind == "threshold":
            frame = references[station_id]
            selected = frame[frame["reference_type"] == "flood"] if not frame.empty else frame
            levels = pd.to_numeric(selected["level_cm"], errors="coerce") if not selected.empty else pd.Series(dtype=float)
            levels = levels[levels.map(lambda value: pd.notna(value) and math.isfinite(float(value)))]
            if levels.empty:
                warn(
                    "threshold_missing", f"A estação {station['station_name']} não tem cota de inundação cadastrada.",
                    station_id=station_id, column=str(column["label"]),
                )
                return None
            return float(levels.iloc[0])
        variable = str(column["variable"])
        horizon = int(column["time_horizon"])
        operation = str(column["operation"])
        span_start = min(reference, reference + pd.Timedelta(hours=horizon))
        target = reference + pd.Timedelta(hours=horizon)
        if not in_window(target, span_start=span_start if operation != "value" else None):
            value = None
        elif kind == "obs":
            value = evaluate_report_series(
                observed[station_id], variable=variable, reference=reference,
                horizon=horizon, operation=operation, time_col="datetime",
                timestep_hours=artifact.timestep_hours,
            )
        else:
            raise AssertionError("Forecast table values are evaluated as scenario ranges.")
        if value is None:
            warn(
                "observation_value_missing" if kind == "obs" else "forecast_value_missing",
                f"Não há medição exata para {column['label']} da estação {station['station_name']}.",
                station_id=station_id, column=str(column["label"]),
            )
        return value

    headers = ["Estação", *(str(column["label"]) for column in columns)]
    rows: list[list[str]] = []
    for station_id in map_ids:
        station = stations[station_id]
        row = [str(station["station_name"])]
        for column in columns:
            if column.get("type") == "forecast":
                # Forecast columns are shown as a conservative range across all selected scenarios.
                station_mini = station["mini_id"]
                values: list[float] = []
                complete = bool(
                    selected_scenarios and station_mini is not None and default_scenarios_complete
                )
                horizon = int(column["time_horizon"])
                operation = str(column["operation"])
                target = reference + pd.Timedelta(hours=horizon)
                span_start = min(reference, target)
                in_report_window = in_window(
                    target, span_start=span_start if operation != "value" else None
                )
                for scenario in selected_scenarios:
                    scenario_id = str(scenario.scenario_id)
                    frame = (
                        get_adjusted_level(scenario_id, station)
                        if column["variable"] == "level"
                        else get_model_series(scenario_id, station_mini, str(column["variable"]), station_id)
                    )
                    item = (
                        evaluate_report_series(
                            frame if frame is not None else pd.DataFrame(),
                            variable=str(column["variable"]), reference=reference,
                            horizon=horizon, operation=operation,
                            time_col="dt", timestep_hours=artifact.timestep_hours,
                        )
                        if in_report_window else None
                    )
                    if item is None:
                        complete = False
                        warn(
                            "forecast_value_missing",
                            f"Não há valor exato para {column['label']} da estação {station['station_name']} no cenário {scenario.label}.",
                            station_id=station_id, scenario_id=scenario_id, column=str(column["label"]),
                        )
                    else:
                        values.append(item)
                if not selected_scenarios:
                    warn(
                        "forecast_scenarios_missing", f"Não há cenários previstos para {column['label']}.",
                        station_id=station_id, column=str(column["label"]),
                    )
                elif station_mini is None:
                    warn(
                        "station_mini_missing", f"A estação {station['station_name']} não está associada a uma mini bacia.",
                        station_id=station_id, column=str(column["label"]),
                    )
                cell = format_report_range(values) if complete and values else "—"
                row.append(cell)
            else:
                value = metric_value(station, column)
                row.append(format_report_number(value))
        rows.append(row)

    # Load chart series from the canonical run caches, calibrating each level series to its station.
    chart_station_data: dict[str, dict[str, object]] = {}
    for station_id in chart_ids:
        station = stations[station_id]
        observation = observed[station_id]
        if not observation[
            (observation["variable_code"] == "level")
            & (observation["datetime"] >= start)
            & (observation["datetime"] <= reference)
            & observation["value"].notna()
        ].shape[0]:
            warn(
                "chart_observation_missing", f"A estação {station['station_name']} não tem níveis observados na janela.",
                station_id=station_id,
            )
        station_thresholds = references[station_id]
        if station_thresholds.empty:
            valid_thresholds = station_thresholds
        else:
            numeric_levels = pd.to_numeric(station_thresholds["level_cm"], errors="coerce")
            valid_thresholds = station_thresholds.loc[
                station_thresholds["reference_type"].isin(["attention", "alert", "flood"])
                & numeric_levels.map(lambda value: pd.notna(value) and math.isfinite(float(value)))
            ]
        if valid_thresholds.empty:
            warn(
                "chart_thresholds_missing", f"A estação {station['station_name']} não tem limites de atenção, alerta ou inundação.",
                station_id=station_id,
            )
        scenario_data: dict[str, dict[str, object]] = {}
        for scenario in selected_scenarios:
            scenario_id = str(scenario.scenario_id)
            mini_id = int(station["mini_id"])
            calibrated = get_adjusted_level(scenario_id, station)
            if calibrated is None:
                calibrated = pd.DataFrame()
            if not calibrated.empty:
                plotted_forecasts = calibrated[
                    (calibrated["prev_flag"] == 1)
                    & (calibrated["dt"] > reference)
                    & (calibrated["dt"] >= start)
                    & (calibrated["dt"] < end)
                    & calibrated["value"].notna()
                ]
                if plotted_forecasts.empty:
                    warn(
                        "chart_forecast_missing",
                        f"O cenário {scenario.label} não tem níveis previstos na janela da estação {station['station_name']}.",
                        station_id=station_id, scenario_id=scenario_id,
                    )
            rain = get_basin_rain(scenario_id, mini_id, station_id)
            if rain is not None:
                selected_rain = rain[(rain["dt"] >= start) & (rain["dt"] < end)]
                current_rain = selected_rain[
                    (selected_rain["prev_flag"] == 0)
                    & (selected_rain["dt"] <= reference)
                    & selected_rain["value"].notna()
                ]
                forecast_rain = selected_rain[
                    (selected_rain["prev_flag"] == 1)
                    & (selected_rain["dt"] > reference)
                    & selected_rain["value"].notna()
                ]
                if current_rain.empty:
                    warn(
                        "basin_rainfall_observed_missing",
                        f"Não há chuva observada ponderada na bacia da estação {station['station_name']}.",
                        station_id=station_id, scenario_id=scenario_id,
                    )
                if forecast_rain.empty:
                    warn(
                        "basin_rainfall_forecast_missing",
                        f"Não há chuva prevista ponderada na bacia da estação {station['station_name']} no cenário {scenario.label}.",
                        station_id=station_id, scenario_id=scenario_id,
                    )
            scenario_data[scenario_id] = {"level": calibrated, "basin_rain": rain}
        first_id = next((
            key for key, item in scenario_data.items()
            if isinstance(item.get("basin_rain"), pd.DataFrame)
        ), None)
        if first_id is not None:
            for scenario_id, item in scenario_data.items():
                if isinstance(item.get("basin_rain"), pd.DataFrame):
                    rain_frame = item["basin_rain"].copy()
                    rain_frame.attrs["show_observed"] = scenario_id == first_id
                    item["basin_rain"] = rain_frame
        chart_station_data[station_id] = scenario_data

    figures = [("report-map", map_figure(basin, map_stations, basin_ids, mini_segments))]
    for index, station_id in enumerate(chart_ids):
        station = stations[station_id]
        figures.append((
            f"report-chart-{index}",
            station_chart(
                station, observed[station_id], chart_station_data[station_id],
                scenario_labels, references[station_id],
                reference=reference, window_start=start, window_end=end,
            ),
        ))

    write_report(
        output_path=output_path, format=format, title=title, author=author, basin_name=basin_name,
        reference=reference, window_start=start, window_end=end, headers=headers, rows=rows,
        figures=figures, markdown_path=Path(markdown_path) if markdown_path is not None else None,
        warnings=warning_items,
    )
    return ReportResult(output_path, format, tuple(warning_items))


__all__ = ["generate_report"]

