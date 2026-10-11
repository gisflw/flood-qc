# Offline operational reports

`mgb_ops.workflows.reporting.generate_report` creates a standalone HTML report or a PDF
from the history database, mini-catchment GeoPackage, selected current-run
artifact, and that run's published scenario caches. It reads existing data only;
it does not fetch providers or run MGB. HTML is self-contained and has no tile
or CDN dependencies. PDF rendering uses local Chromium. The map uses river
segments from the same GeoPackage, and each chart uses rainfall from its own
station mini basin. Water-level forecasts in the table and charts use the same
station calibration.

Install the report dependencies and, for PDF output, Chromium:

```bash
uv sync --extra report
uv run playwright install chromium
```

On Linux, install any missing browser system libraries with
`uv run playwright install-deps chromium`.

The CLI resolves relative paths under the selected workspace. Its default paths
are `data/history.sqlite`, `data/source/rs_hydro.gpkg`,
`data/cache/current_run.sqlite`, and `data/cache`. Station IDs are ordered as
given; the map station list also defines the table rows. `--scenario-ids`
accepts canonical IDs from the saved artifact; without it, the report selects
both raw ECMWF and NOAA scenarios. If either is absent, the report warns and
leaves forecast table ranges blank; an explicit single scenario is allowed.
The default table has station name, observed
rainfall and level metrics, flood threshold, and +6/+12 hour level ranges.

```bash
uv run mgb-ops generate-report \
  --workspace scratch/rs_hydro \
  --output-dir data/reports \
  --output-basin-name alto-uruguai \
  --format html \
  --outlet-mini-id 539 \
  --map-station-ids ana:123 ana:456 \
  --chart-station-ids ana:123 \
  --title "Boletim hidrológico" \
  --author "Equipe de operação" \
  --basin-name "Alto Uruguai"
```

For a PDF, pass `--format pdf`. Optional `--reference-time`, `--window-start`,
and `--window-end` values are interpreted in `America/Sao_Paulo` when they have
no timezone; `--window-end` is exclusive. `--table-columns` points to a JSON
array, for example. The generated filename is
`{reference_time:YYYYMMDD_HH}_{output_basin_name}.{format}`; a report at
2026-10-11 09:00 named `alto-uruguai` becomes
`20261011_09_alto-uruguai.html`.

```json
[
  {"label":"Nível atual (cm)","variable":"level","type":"obs","time_horizon":0,"operation":"value"},
  {"label":"Nível previsto +6 h (cm)","variable":"level","type":"forecast","time_horizon":6,"operation":"value"}
]
```

Custom table columns replace the default numeric columns; station name remains
the first column. Numeric specs use `obs` or `forecast`, an integer hour offset,
and one of `value`, `accumulation`, or `change`. Markdown summaries are read as
UTF-8 and rendered after the charts. Raw HTML is escaped. Markdown images are
rejected so a standalone report cannot load embedded external resources.

Python callers can pass explicit paths and ordered IDs directly:

```python
from pathlib import Path
from mgb_ops.workflows.reporting import generate_report

result = generate_report(
    history_path=Path("data/history.sqlite"),
    spatial_path=Path("data/source/rs_hydro.gpkg"),
    run_artifact_path=Path("data/cache/current_run.sqlite"),
    cache_dir=Path("data/cache"),
    output_dir=Path("data/reports"),
    output_basin_name="alto-uruguai",
    format="pdf",
    outlet_mini_id=539,
    map_station_ids=["ana:123", "ana:456"],
    chart_station_ids=["ana:123"],
    title="Boletim hidrológico",
    author="Equipe de operação",
    basin_name="Alto Uruguai",
)
print(result.path, result.warnings)
```
