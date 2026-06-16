# Operational Workflows

The preferred workflow shape is Python-first: import the relevant `mgb_ops`
module, pass explicit workspace/database/path inputs, and compose the returned
summaries or domain objects in a notebook, script, or orchestrated data flow. CLI
commands remain available as thin wrappers for repeatable local operation.

## Workflow Implemented Today

### 1. History Bootstrap

Library module: `mgb_ops.storage.db_bootstrap`

1. Initialize an explicitly supplied history SQLite path with an explicitly supplied schema path.
2. Load the station inventory into `station`.
3. Ensure the basic `provider` and `variable` catalogs.

CLI wrapper:

```bash
mgb-ops --workspace examples/rs_hydro bootstrap history
```

### 2. ANA Observation Ingestion

Library module: `mgb_ops.ingest.fetch_observed_ana`

1. Read module-owned defaults and the optional override in `<workspace>/config/custom.yaml`.
2. Fetch hydrometeorological data by station and day.
3. Save provider XML as ancillary evidence and write normalized observed CSV files under `<workspace>/data/interim/ana/<run_id>/`.
4. Import the normalized CSV through `mgb_ops.ingest.observed_csv.import_normalized_observed_csvs()` into `observed_series` and `observed_value`.
5. Register logs in `logs/fetch_observed_ana/`.

CLI wrapper:

```bash
mgb-ops --workspace examples/rs_hydro ingest ana
```

### 2b. INMET Rainfall Ingestion

Library module: `mgb_ops.ingest.fetch_observed_inmet`

1. Read module-owned defaults and the optional override in `<workspace>/config/custom.yaml`.
2. In CLI/dashboard convenience flows, resolve the local key from explicit input, process `INMET_API_KEY`, or `<workspace>/.env`; pass `api_key` explicitly to the domain function.
3. Query the operational rainfall API by station and day, using the explicit `product_code` input that defaults to `I175`.
4. Write normalized observed rainfall CSV files under `<workspace>/data/interim/inmet/<run_id>/`.
5. Import the normalized CSV through `mgb_ops.ingest.observed_csv.import_normalized_observed_csvs()` into `observed_series` and `observed_value`.
6. Register logs in `logs/fetch_observed_inmet/`.

CLI wrapper:

```bash
mgb-ops --workspace examples/rs_hydro ingest inmet
```

### 3. Forecast Grid Ingestion

Library module: `mgb_ops.ingest.forecast_grid`

1. Resolve the cycle from `reference_time`.
2. Download the configured forecast GRIB. ECMWF is the current default product configuration.
3. Clip the grid to the caller-supplied operational bounding box plus caller-supplied buffer fraction.
4. Register the canonical asset in the explicitly supplied history database, using `provider_code` and `asset_kind` plus an explicitly supplied asset base directory for relative paths.
5. Register logs in `logs/forecast_grid/`.

Python callers pass `bbox=(west, south, east, north)` and `buffer_fraction=...`
directly to `mgb_ops.ingest.forecast_grid.ingest_forecast_grids`. CLI users can
set `forecast_grid.bbox` and `forecast_grid.buffer_fraction` in
`<workspace>/config/custom.yaml`, or override them per run with `--bbox` and
`--buffer-fraction`.

CLI wrapper:

```bash
mgb-ops --workspace examples/rs_hydro ingest forecast-grid --bbox -60 -35 -48 -26 --buffer-fraction 1
```

### 4. MGB Preparation

Library modules:

- `mgb_ops.model.prepare_mgb_meta`
- `mgb_ops.model.prepare_mgb_rainfall`

1. Rewrite `PARHIG.hig` from the current configuration.
2. Load observed rainfall from the history database.
3. Normalize rainfall to the hourly grid and interpolate it to the minis.
4. When enabled, incorporate hourly ECMWF rainfall into the forecast block.
5. Write `<workspace>/mgb_runner/Input/chuvabin.hig`.

CLI wrappers:

```bash
mgb-ops --workspace examples/rs_hydro model prepare-meta
mgb-ops --workspace examples/rs_hydro model prepare-rainfall
```

### 5. Model Execution and Consumption

Library modules:

- `mgb_ops.model.mgb_execution`
- `mgb_ops.model.run_mgb`
- `mgb_ops.model.export_mgb_outputs`

1. Prepare the runner workspace.
2. Run the MGB binary or dry-run.
3. Mirror output back to `<workspace>/mgb_runner/Output`.
4. Optionally export the operational MGB output window to SQLite.
5. Read MGB binaries or exported outputs for visualization and downstream use.

CLI wrappers:

```bash
mgb-ops --workspace examples/rs_hydro model run --dry-run
mgb-ops --workspace examples/rs_hydro model export-outputs
```

### 6. Dashboard

Interface layer: `apps/ops_dashboard`

Support modules:

- `apps.ops_dashboard.support.data`
- `apps.ops_dashboard.support.forecast`
- `apps.ops_dashboard.support.map`

1. Read `<workspace>/data/history.sqlite` for registry and observations.
2. Read MGB runner binaries for mini series.
3. Read accumulated rasters in `<workspace>/data/interim/`.
4. Allow preview and persistence of manual ECMWF forecast corrections.

CLI wrapper:

```bash
mgb-ops --workspace examples/rs_hydro dashboard
```

The dashboard support modules are app-layer helpers. They keep querying, map
assembly, and forecast preview logic out of the Streamlit app.

## Incomplete Flows

### Automatic QC

The schema and states exist, but the flow to:

- generate flags in `qc_flag`
- promote `raw -> curated -> approved`
- automatically release approved inputs

is not operational yet.

### Materialized Operational Run

The run schema exists, but the flow that copies inputs, outputs, derivatives,
flags, and lineage to `<workspace>/data/runs/<run_id>.sqlite` is not closed yet.

### Manual Review of Observations

Manual correction for ECMWF forecasts currently exists in the history database.
Manual review of observed rainfall is not implemented yet.

### Reports

Generation of `report_artifact` and publication to `run_catalog` remain pending.
There is no `mgb_ops.reporting` package yet. Future reporting should be
implemented.

## Maintained Architectural Direction

Even with implementation gaps, the canonical direction remains:

- Python library first;
- CLI and dashboard as thin wrappers;
- persistent history in SQLite;
- one SQLite file per run;
- processed spatial assets in `data/spatial/`;
- processed series in `data/timeseries/`;
- current configuration in YAML, with `.toml` still under evaluation.
