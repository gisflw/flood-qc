from __future__ import annotations

import json
import re
import sqlite3
from dataclasses import replace
from pathlib import Path

import geopandas as gpd
import numpy as np
import pandas as pd
import pytest
import xarray as xr
from shapely.geometry import LineString, Polygon

from apps.cli import main
from mgb_ops.assets.current_run import (
    CurrentRunArtifact,
    ForecastScenarioReference,
    create_current_artifact,
    load_current_artifact,
)
from mgb_ops.assets.databases import apply_schema
from mgb_ops.workflows.reporting import generate_report


ROOT = Path(__file__).resolve().parents[2]
HISTORY_SCHEMA = ROOT / "src/mgb_ops/assets/sql/history_schema.sql"
REFERENCE = pd.Timestamp("2026-10-11 09:00:00")
WINDOW_START = pd.Timestamp("2026-10-09 00:00:00")
WINDOW_END = pd.Timestamp("2026-10-14 00:00:00")


def _make_cache(path: Path, scenario_id: str, provider: str, *, missing_noaa_six: bool = False) -> Path:
    times = pd.date_range(WINDOW_START, WINDOW_END, inclusive="left", freq="h")
    elapsed = (times - REFERENCE).total_seconds().to_numpy() / 3600
    segments = (times > REFERENCE).astype(np.int8)
    minis = [1, 2, 3, 4]
    level = np.column_stack([80 + elapsed, 85 + elapsed, 100 + elapsed, 70 + elapsed])
    if provider == "noaa":
        level[times > REFERENCE, 2] += 4
    if missing_noaa_six:
        level[times == REFERENCE + pd.Timedelta(hours=6), 2] = np.nan
    rain = np.tile(np.array([1.0, 3.0, 5.0, 9.0]), (len(times), 1))
    dataset = xr.Dataset(
        data_vars={
            "level": (("time", "mini"), level),
            "precipitation": (("time", "mini"), rain),
            "time_segment": (("time",), segments),
        },
        coords={"time": times, "mini": np.arange(len(minis)), "mini_id": (("mini",), minis)},
        attrs={
            "window_start": WINDOW_START.isoformat(),
            "reference_time": REFERENCE.isoformat(),
            "window_end_exclusive": WINDOW_END.isoformat(),
            "scenario_id": scenario_id,
            "scenario_kind": "raw",
            "provider_code": provider,
        },
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    dataset.to_netcdf(path)
    return path


@pytest.fixture
def report_inputs(tmp_path: Path) -> dict[str, Path]:
    history = tmp_path / "data/history.sqlite"
    apply_schema(history, HISTORY_SCHEMA)
    with sqlite3.connect(history) as connection:
        stations = [
            ("ana:inside", "inside", "Estação de dentro", 3, -29.95, -52.95),
            ("ana:outside", "outside", "Estação de fora", 4, -28.0, -50.0),
            ("ana:no-mini", "no-mini", "Estação sem mini", None, -29.9, -52.9),
        ]
        connection.executemany(
            "INSERT INTO station (station_id,station_code,station_name,provider_code,mini_id,latitude,longitude) "
            "VALUES (?, ?, ?, 'ana', ?, ?, ?)",
            [(station_id, code, name, mini, lat, lon) for station_id, code, name, mini, lat, lon in stations],
        )
        connection.execute(
            "INSERT INTO station_level_reference (station_id, reference_code, level_cm) VALUES ('ana:inside','attention',90)"
        )
        connection.execute(
            "INSERT INTO station_level_reference (station_id, reference_code, level_cm) VALUES ('ana:inside','alert',105)"
        )
        connection.execute(
            "INSERT INTO station_level_reference (station_id, reference_code, level_cm) VALUES ('ana:inside','flood',123)"
        )
        connection.execute(
            "INSERT INTO observed_series (series_id,station_id,variable_code,state,created_at) VALUES ('inside.level','ana:inside','level','approved','2026-10-11')"
        )
        connection.execute(
            "INSERT INTO observed_series (series_id,station_id,variable_code,state,created_at) VALUES ('inside.rain','ana:inside','rain','raw','2026-10-11')"
        )
        connection.executemany(
            "INSERT INTO observed_value (series_id,observed_at,value) VALUES ('inside.level',?,?)",
            [
                ((REFERENCE - pd.Timedelta(hours=24)).strftime("%Y-%m-%d %H:%M:%S"), 110.0),
                ((REFERENCE - pd.Timedelta(hours=1)).strftime("%Y-%m-%d %H:%M:%S"), 120.0),
                (REFERENCE.strftime("%Y-%m-%d %H:%M:%S"), 125.0),
            ],
        )
        connection.executemany(
            "INSERT INTO observed_value (series_id,observed_at,value) VALUES ('inside.rain',?,?)",
            [
                ((REFERENCE - pd.Timedelta(hours=24) + pd.Timedelta(hours=hour)).strftime("%Y-%m-%d %H:%M:%S"), 1.0)
                for hour in range(1, 25)
            ],
        )
        connection.commit()

    spatial = tmp_path / "data/source/mini.gpkg"
    spatial.parent.mkdir(parents=True)
    bounds = [
        (1, 3, 1.0, -53.1, -30.0),
        (2, 3, 3.0, -53.0, -30.0),
        (3, 99, 5.0, -52.9, -30.0),
        (4, 99, 7.0, -50.1, -28.1),
    ]
    catchments = gpd.GeoDataFrame(
        {
            "mini_id": [row[0] for row in bounds],
            "mini_jus": [row[1] for row in bounds],
            "mini_area": [row[2] for row in bounds],
        },
        geometry=[
            Polygon([(x, y), (x + 0.1, y), (x + 0.1, y + 0.1), (x, y + 0.1)])
            for _, _, _, x, y in bounds
        ],
        crs="EPSG:4326",
    )
    catchments.to_file(spatial, layer="mini_catchments", driver="GPKG")
    segments = gpd.GeoDataFrame(
        {"mini_id": [1, 2, 3, 4]},
        geometry=[
            LineString([(-53.09, -29.99), (-53.02, -29.98)]),
            LineString([(-52.99, -29.99), (-52.92, -29.98)]),
            LineString([(-52.89, -29.99), (-52.82, -29.98)]),
            LineString([(-50.09, -28.09), (-50.02, -28.08)]),
        ],
        crs="EPSG:4326",
    )
    segments.to_file(spatial, layer="mini_segments", driver="GPKG")

    cache_dir = tmp_path / "data/cache"
    scenario_refs = (
        ForecastScenarioReference(
            "raw:ecmwf:cycle", 0, "raw", "ECMWF", provider_code="ecmwf", source_asset_id="ecmwf:cycle"
        ),
        ForecastScenarioReference(
            "raw:noaa:cycle", 1, "raw", "NOAA", provider_code="noaa", source_asset_id="noaa:cycle"
        ),
    )
    published = {
        "raw:ecmwf:cycle": "data/cache/forecast_scenarios/ecmwf.nc",
        "raw:noaa:cycle": "data/cache/forecast_scenarios/noaa.nc",
    }
    artifact = CurrentRunArtifact(
        reference_time=REFERENCE.isoformat(),
        forecast_end_exclusive=WINDOW_END.isoformat(),
        timestep_hours=1,
        mgb_settings={"observed_horizon_days": 2, "forecast_horizon_days": 2},
        spatial_settings={"bbox": [-54, -31, -49, -27]},
        interpolation_settings={},
        review_window={"start": (REFERENCE - pd.Timedelta(days=1)).isoformat(), "end": REFERENCE.isoformat()},
        observed_providers=("ana",),
        reason="test report",
        scenarios=scenario_refs,
        published_caches=published,
    )
    run_artifact = tmp_path / "data/cache/current_run.sqlite"
    create_current_artifact(run_artifact, artifact)
    _make_cache(cache_dir / "forecast_scenarios/ecmwf.nc", "raw:ecmwf:cycle", "ecmwf")
    _make_cache(cache_dir / "forecast_scenarios/noaa.nc", "raw:noaa:cycle", "noaa")
    return {
        "root": tmp_path,
        "history": history,
        "spatial": spatial,
        "cache": cache_dir,
        "artifact": run_artifact,
    }


def _generate(inputs: dict[str, Path], output: Path, **kwargs):
    kwargs.setdefault("map_station_ids", ["ana:outside", "ana:inside"])
    kwargs.setdefault("chart_station_ids", ["ana:inside"])
    spatial_path = kwargs.pop("spatial_path", inputs["spatial"])
    run_artifact_path = kwargs.pop("run_artifact_path", inputs["artifact"])
    return generate_report(
        history_path=inputs["history"],
        spatial_path=spatial_path,
        run_artifact_path=run_artifact_path,
        cache_dir=inputs["cache"],
        output_dir=output.parent,
        output_basin_name=output.stem,
        format="html",
        outlet_mini_id=3,
        title="Boletim",
        author="Equipe",
        basin_name="Bacia de teste",
        **kwargs,
    )


def test_html_report_uses_preferred_observations_and_run_scenarios(report_inputs, tmp_path):
    output = tmp_path / "report.html"
    result = _generate(report_inputs, output)
    document = result.path.read_text(encoding="utf-8")

    table = re.search(r"<tbody>(.*?)</tbody>", document, re.S).group(1)
    assert table.index("Estação de fora") < table.index("Estação de dentro")
    assert "24,0" in table
    assert "125,0" in table
    assert "5,0" in table
    assert "15,0" in table
    assert "123,0" in table
    assert "117,9–148,5" in table
    assert "ECMWF" in document
    assert "Chuva observada ponderada" in document
    assert 'id="report-map"' in document
    assert "-50.0" in document
    assert any(item.code == "station_outside_basin" for item in result.warnings)
    assert any(item.code == "threshold_missing" and item.station_id == "ana:outside" for item in result.warnings)
    assert result.path == output.parent / f"{REFERENCE:%Y%m%d_%H}_{output.stem}.html"


def test_custom_columns_exact_times_signed_changes_and_local_timezone(report_inputs, tmp_path):
    output = tmp_path / "custom.html"
    columns = [{
        "label": "Delta de nível",
        "variable": "level",
        "type": "obs",
        "time_horizon": -1,
        "operation": "change",
    }]
    with sqlite3.connect(report_inputs["history"]) as connection:
        connection.execute(
            "UPDATE observed_value SET value=115 WHERE series_id='inside.level' AND observed_at=?",
            (REFERENCE.strftime("%Y-%m-%d %H:%M:%S"),),
        )
    result = _generate(
        report_inputs,
        output,
        map_station_ids=["ana:inside"],
        chart_station_ids=[],
        table_columns=columns,
        reference_time="2026-10-11T12:00:00Z",
    )
    document = result.path.read_text(encoding="utf-8")
    assert "11/10/2026 09:00" in document
    assert "Delta de nível" in document
    assert "-5,0" in re.search(r"<tbody>(.*?)</tbody>", document, re.S).group(1)
    assert result.warnings == ()


def test_incomplete_scenario_returns_em_dash_and_warning(report_inputs, tmp_path):
    _make_cache(
        report_inputs["cache"] / "forecast_scenarios/noaa.nc",
        "raw:noaa:cycle", "noaa", missing_noaa_six=True,
    )
    output = tmp_path / "incomplete.html"
    result = _generate(report_inputs, output, map_station_ids=["ana:inside"], chart_station_ids=[])
    document = result.path.read_text(encoding="utf-8")
    table = re.search(r"<tbody>(.*?)</tbody>", document, re.S).group(1)
    assert "Nível previsto +6 h (cm)</button>" in document
    assert "—" in table
    assert any(item.code == "forecast_value_missing" and item.scenario_id == "raw:noaa:cycle" for item in result.warnings)


def test_forecast_table_adjusts_levels_and_allows_explicit_single_scenario(report_inputs, tmp_path):
    output = tmp_path / "single-provider.html"
    result = _generate(
        report_inputs, output, map_station_ids=["ana:inside"], chart_station_ids=[],
        scenarios=["raw:ecmwf:cycle"],
    )
    row = re.search(r"<tbody>(.*?)</tbody>", result.path.read_text(encoding="utf-8"), re.S).group(1)
    assert "117,9–144,1" in row


@pytest.mark.parametrize("missing_provider", ["ecmwf", "noaa"])
def test_default_missing_provider_warns_and_suppresses_partial_range(report_inputs, tmp_path, missing_provider):
    artifact = load_current_artifact(report_inputs["artifact"])
    selected = tuple(
        replace(scenario, position=index)
        for index, scenario in enumerate(
            item for item in artifact.scenarios if item.provider_code != missing_provider
        )
    )
    without_provider = replace(
        artifact,
        scenarios=selected,
        published_caches={item.scenario_id: artifact.published_caches[item.scenario_id] for item in selected},
    )
    path = tmp_path / f"without-{missing_provider}.sqlite"
    create_current_artifact(path, without_provider)
    output = tmp_path / "missing-default-provider.html"
    result = _generate(
        report_inputs, output, run_artifact_path=path,
        map_station_ids=["ana:inside"], chart_station_ids=[],
    )
    row = re.search(r"<tbody>(.*?)</tbody>", result.path.read_text(encoding="utf-8"), re.S).group(1)
    assert row.count("—") == 2
    assert any(
        item.code == "default_scenario_missing" and missing_provider.upper() in item.message
        for item in result.warnings
    )


def test_level_table_and_chart_warn_when_calibration_overlap_is_missing(report_inputs, tmp_path):
    with sqlite3.connect(report_inputs["history"]) as connection:
        connection.execute("DELETE FROM observed_value WHERE series_id='inside.level'")
    output = tmp_path / "no-level-overlap.html"
    result = _generate(report_inputs, output, map_station_ids=["ana:inside"])
    row = re.search(r"<tbody>(.*?)</tbody>", result.path.read_text(encoding="utf-8"), re.S).group(1)
    assert row.count("—") >= 2
    assert any(item.code == "calibration_unavailable" for item in result.warnings)
    assert any(item.code == "forecast_value_missing" for item in result.warnings)


def test_missing_exact_measurements_and_incomplete_accumulations_are_not_interpolated(report_inputs, tmp_path):
    missing_time = (REFERENCE - pd.Timedelta(hours=12)).strftime("%Y-%m-%d %H:%M:%S")
    with sqlite3.connect(report_inputs["history"]) as connection:
        connection.execute(
            "DELETE FROM observed_value WHERE series_id='inside.rain' AND observed_at=?",
            (missing_time,),
        )
    columns = [
        {"label": "Nível exato -2 h", "variable": "level", "type": "obs", "time_horizon": -2, "operation": "value"},
        {"label": "Chuva completa -24 h", "variable": "rain", "type": "obs", "time_horizon": -24, "operation": "accumulation"},
    ]
    result = _generate(
        report_inputs, tmp_path / "missing.html", map_station_ids=["ana:inside"],
        chart_station_ids=[], table_columns=columns,
    )
    row = re.search(r"<tbody>(.*?)</tbody>", result.path.read_text(encoding="utf-8"), re.S).group(1)
    assert row.count("—") == 2
    assert {item.column for item in result.warnings if item.code == "observation_value_missing"} == {
        "Nível exato -2 h", "Chuva completa -24 h",
    }


@pytest.mark.parametrize(
    ("map_ids", "chart_ids", "message"),
    [(["ana:unknown"], [], "Station IDs were not found"), (["ana:inside"], ["ana:no-mini"], "has no mini_id")],
)
def test_invalid_station_selection_raises(report_inputs, tmp_path, map_ids, chart_ids, message):
    with pytest.raises(ValueError, match=message):
        _generate(
            report_inputs, tmp_path / "invalid.html",
            map_station_ids=map_ids, chart_station_ids=chart_ids,
        )
    assert not (tmp_path / f"{REFERENCE:%Y%m%d_%H}_invalid.html").exists()


def test_report_requires_river_layer_for_offline_map(report_inputs, tmp_path):
    missing_rivers = tmp_path / "catchments-only.gpkg"
    gpd.read_file(report_inputs["spatial"], layer="mini_catchments").to_file(
        missing_rivers, layer="mini_catchments", driver="GPKG"
    )
    output = tmp_path / "missing-rivers.html"
    with pytest.raises(ValueError, match="Unable to read layer 'mini_segments'"):
        _generate(report_inputs, output, spatial_path=missing_rivers)
    assert not (tmp_path / f"{REFERENCE:%Y%m%d_%H}_missing-rivers.html").exists()


def test_markdown_html_is_inert_and_images_are_rejected(report_inputs, tmp_path):
    summary = tmp_path / "summary.md"
    summary.write_text("# Summary\n\n<strong>raw HTML</strong>\n", encoding="utf-8")
    safe_result = _generate(report_inputs, tmp_path / "safe.html", map_station_ids=[], chart_station_ids=[], markdown_path=summary)
    document = safe_result.path.read_text(encoding="utf-8")
    assert "&lt;strong&gt;raw HTML&lt;/strong&gt;" in document
    summary.write_text("![remote](https://example.test/image.png)", encoding="utf-8")
    with pytest.raises(ValueError, match="images are unsupported"):
        _generate(report_inputs, tmp_path / "unsafe.html", map_station_ids=[], chart_station_ids=[], markdown_path=summary)
    assert not (tmp_path / f"{REFERENCE:%Y%m%d_%H}_unsafe.html").exists()
    previous = tmp_path / "previous.html"
    previous.write_text("previous complete report", encoding="utf-8")
    with pytest.raises(ValueError, match="images are unsupported"):
        _generate(report_inputs, previous, map_station_ids=[], chart_station_ids=[], markdown_path=summary)
    assert previous.read_text(encoding="utf-8") == "previous complete report"


def test_browser_report_is_offline_interactive_and_pdf_is_written(report_inputs, tmp_path):
    from playwright.sync_api import sync_playwright

    output = tmp_path / "browser.html"
    result = _generate(report_inputs, output, chart_station_ids=["ana:inside", "ana:outside"])
    with sync_playwright() as playwright:
        browser = playwright.chromium.launch()
        try:
            page = browser.new_page()
            requests = []
            page.on("request", lambda request: requests.append(request.url) if request.url.startswith(("http:", "https:")) else None)
    page.goto(result.path.resolve().as_uri())
            page.wait_for_function("window.reportReady === true", timeout=60_000)
            assert page.locator("#report-map .main-svg").count() >= 1
            assert page.locator("#report-chart-0 .main-svg").count() >= 1
            assert page.locator("#report-chart-1 .main-svg").count() >= 1
            assert requests == []
            plot_data = page.evaluate("""() => {
              const chart = document.getElementById('report-chart-0');
              const outsideChart = document.getElementById('report-chart-1');
              const map = document.getElementById('report-map');
              const values = value => Array.isArray(value) ? value : (value?._inputArray || Array.from(value || []));
              const levels = chart.data.filter(trace => trace.name.startsWith('Nível previsto'));
              const atSixHours = levels.map(trace => ({
                name: trace.name,
                value: values(trace.y)[trace.x.indexOf('2026-10-11 15:00:00')],
              }));
              const observedRain = chart.data.find(trace => trace.name === 'Chuva observada ponderada');
              const outsideRain = outsideChart.data.find(trace => trace.name === 'Chuva observada ponderada');
              const outside = map.data.find(trace => trace.name === 'Estação fora da bacia');
              const rivers = map.data.find(trace => trace.name === 'Rios da bacia');
              const insideRow = [...document.querySelectorAll('#station-table tbody tr')]
                .find(row => row.cells[0].textContent.includes('Estação de dentro'));
              return {
                atSixHours,
                observedRain: observedRain ? values(observedRain.y) : [],
                outsideObservedRain: outsideRain ? values(outsideRain.y) : [],
                outsideIds: outside ? Array.from(outside.customdata) : [],
                riverMinis: rivers ? Array.from(rivers.customdata).filter(value => value !== null) : [],
                mapXRange: map.layout.xaxis.range,
                insideSixHourTableValue: insideRow.cells[6].textContent,
              };
            }""")
            assert {item["name"]: item["value"] for item in plot_data["atSixHours"]} == {
                "Nível previsto · ECMWF": 131,
                "Nível previsto · NOAA": 135,
            }
            assert plot_data["observedRain"]
            assert plot_data["observedRain"][-1] == pytest.approx(35 / 9)
            assert plot_data["outsideObservedRain"]
            assert plot_data["outsideObservedRain"][-1] == pytest.approx(9.0)
            assert "ana:outside" in plot_data["outsideIds"]
            assert set(plot_data["riverMinis"]) == {1, 2, 3}
            assert plot_data["mapXRange"][0] < -50
            assert plot_data["insideSixHourTableValue"] == "117,9–148,5"
            rows = page.locator("#station-table tbody tr")
            assert rows.count() == 2
            page.locator(".sort-button").first.click()
            assert page.locator("#station-table thead th").first.get_attribute("aria-sort") == "ascending"
            page.locator("#station-filter").fill("Estação de dentro")
            assert page.locator("#station-table tbody tr:visible").count() == 1
        finally:
            browser.close()

    pdf_result = generate_report(
        history_path=report_inputs["history"], spatial_path=report_inputs["spatial"],
        run_artifact_path=report_inputs["artifact"], cache_dir=report_inputs["cache"],
        output_dir=tmp_path, output_basin_name="report", format="pdf", outlet_mini_id=3,
        map_station_ids=["ana:inside"], chart_station_ids=["ana:inside"],
        title="Boletim em PDF", author="Equipe", basin_name="Bacia de teste",
    )
    pdf_bytes = pdf_result.path.read_bytes()
    assert pdf_bytes.startswith(b"%PDF-")
    assert len(pdf_bytes) > 10_000
    assert b"Boletim em PDF" in pdf_bytes
    assert pdf_result.format == "pdf"


def test_cli_and_python_api_match_with_custom_columns(report_inputs, tmp_path, capsys):
    columns = [{
        "label": "Nível na referência",
        "variable": "level",
        "type": "obs",
        "time_horizon": 0,
        "operation": "value",
    }]
    columns_path = report_inputs["root"] / "config/columns.json"
    columns_path.parent.mkdir(parents=True)
    columns_path.write_text(json.dumps(columns), encoding="utf-8")
    cli_output = report_inputs["root"] / "reports" / f"{REFERENCE:%Y%m%d_%H}_cli.html"
    status = main([
        "generate-report", "--workspace", str(report_inputs["root"]),
        "--spatial-gpkg", "data/source/mini.gpkg",
        "--output-dir", "reports", "--output-basin-name", "cli", "--format", "html", "--outlet-mini-id", "3",
        "--map-station-ids", "ana:inside", "--chart-station-ids", "ana:inside",
        "--title", "Boletim", "--author", "Equipe", "--basin-name", "Bacia",
        "--table-columns", "config/columns.json",
        "--scenario-ids", "raw:noaa:cycle", "raw:ecmwf:cycle",
    ])
    assert status == 0
    cli_result = json.loads(capsys.readouterr().out)
    python_output_dir = tmp_path / "python"
    python_result = generate_report(
        history_path=report_inputs["history"], spatial_path=report_inputs["spatial"],
        run_artifact_path=report_inputs["artifact"], cache_dir=report_inputs["cache"],
        output_dir=python_output_dir, output_basin_name="python", format="html", outlet_mini_id=3,
        map_station_ids=["ana:inside"], chart_station_ids=["ana:inside"],
        title="Boletim", author="Equipe", basin_name="Bacia",
        scenarios=["raw:noaa:cycle", "raw:ecmwf:cycle"], table_columns=columns,
    )
    assert cli_output.read_bytes() == python_result.path.read_bytes()
    assert cli_result["path"] == str(cli_output)
    assert cli_result["format"] == "html"
    assert cli_result["warnings"] == [
        {
            "code": item.code,
            "message": item.message,
            "station_id": item.station_id,
            "scenario_id": item.scenario_id,
            "column": item.column,
        }
        for item in python_result.warnings
    ]


def test_current_run_window_defaults_without_stale_cache_warning(report_inputs, tmp_path):
    output = tmp_path / "default.html"
    result = _generate(report_inputs, output, map_station_ids=["ana:inside"], chart_station_ids=[])
    document = result.path.read_text(encoding="utf-8")
    assert "09/10/2026 00:00 a 14/10/2026 00:00 (fim exclusivo)" in document
    assert all(item.code != "scenario_cache_stale" for item in result.warnings)


@pytest.mark.parametrize(
    ("attribute", "value"),
    [("reference_time", "2026-10-11T10:00:00"), ("window_start", "2026-10-08T00:00:00")],
)
def test_stale_scenario_cache_is_not_used_as_fallback(report_inputs, tmp_path, attribute, value):
    path = report_inputs["cache"] / "forecast_scenarios/noaa.nc"
    with xr.open_dataset(path) as dataset:
        stale = dataset.load()
    stale.attrs[attribute] = value
    stale.to_netcdf(path)

    output = tmp_path / "stale.html"
    result = _generate(report_inputs, output, map_station_ids=["ana:inside"], chart_station_ids=[])
    table = re.search(r"<tbody>(.*?)</tbody>", result.path.read_text(encoding="utf-8"), re.S).group(1)
    assert "—" in table
    assert any(item.code == "scenario_cache_stale" and item.scenario_id == "raw:noaa:cycle" for item in result.warnings)
