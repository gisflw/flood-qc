# Operational Workflows

The preferred workflow shape is Python-first: import the relevant `mgb_ops`
module, pass explicit workspace/database/path inputs, and compose the returned
summaries or domain objects in a notebook, script, or orchestrated data flow.

## Workflow Implemented Today

### 1. History Bootstrap

Library module: `mgb_ops.storage.db_bootstrap`

1. Initialize an explicitly supplied history SQLite path with an explicitly supplied schema path.
2. Load the station inventory into `station`.
3. Ensure the basic `provider` and `variable` catalogs.

### 2. ANA Observation Ingestion

Fetch module: `mgb_ops.adapters.observed_ana`
Fill-DB workflow: `mgb_ops.workflows.observed.fetch_and_load_observed_provider`

1. Read ANA stations from the caller-supplied history database.
2. For each station, resume from the latest raw observed day already present in SQLite, overlapping that day; stations without later data start at the configured observed request window start.
3. Fetch hydrometeorological data by station and contiguous date window. The library workflow accepts `fetch_window_days`, defaulting to `30`; use `1` to preserve one request per day.
4. Save provider XML as ancillary evidence per fetch window under `<workspace>/data/downloads/ana/<run_id>/<station_code>/<YYYYMMDD>__<YYYYMMDD>.xml` and write one normalized observed CSV per station per run under `<workspace>/data/downloads/ana/<run_id>/<station_code>/observed.csv`.
5. Load normalized CSVs through `mgb_ops.storage.observed_csv.load_normalized_observed_csvs()` into `observed_series` and `observed_value`.
6. Register logs in `logs/observed_ana/`.

### 2b. INMET Rainfall Ingestion

Fetch module: `mgb_ops.adapters.observed_inmet`
Fill-DB workflow: `mgb_ops.workflows.observed.fetch_and_load_observed_provider`

1. Read INMET stations from the caller-supplied history database.
2. Resolve the local key in the thin CLI/app layer; pass `api_key` explicitly to the library workflow.
3. For each station, resume from the latest raw observed rain day already present in SQLite, overlapping that day; stations without later data start at the configured observed request window start.
4. Query the operational rainfall API by station and contiguous date window, using the explicit `product_code` input that defaults to `I175`. The library workflow accepts `fetch_window_days`, defaulting to `30`; use `1` to preserve one request per day.
5. Save each successful raw JSON response under `<workspace>/data/downloads/inmet/<run_id>/<station_code>/<YYYYMMDD>__<YYYYMMDD>.json`, where the two dates are the request window start and end.
6. Build one normalized observed rainfall CSV per station per run from the saved raw responses under `<workspace>/data/downloads/inmet/<run_id>/<station_code>/observed.csv`. If a later day fails after earlier days succeeded, keep the partial CSV so successful data can still be imported while the station is marked as an error.
7. Load normalized CSVs through `mgb_ops.storage.observed_csv.load_normalized_observed_csvs()` into `observed_series` and `observed_value`.
8. Register logs in `logs/observed_inmet/`.

### 3. Forecast Grid Ingestion

Library module: `mgb_ops.adapters.forecast_ecmwf`

1. Resolve the cycle from `reference_time`.
2. Download the configured forecast GRIB. ECMWF is the current default product configuration.
3. Clip the grid to the caller-supplied operational bounding box plus caller-supplied buffer fraction.
4. Register the canonical asset in the explicitly supplied history database, using `provider_code` and `asset_kind` plus an explicitly supplied asset base directory for relative paths.
5. Register logs in `logs/forecast_ecmwf/`.

Python callers pass `bbox=(west, south, east, north)` and `buffer_fraction=...`
directly to `mgb_ops.adapters.forecast_ecmwf.ingest_forecast_grids`. These values
can also be set as `forecast_grid.bbox` and `forecast_grid.buffer_fraction` in
`<workspace>/config/custom.yaml`.

### 4. MGB Preparation

Library modules:

- `mgb_ops.model.prepare_mgb_meta`
- `mgb_ops.model.prepare_mgb_rainfall`

1. Rewrite `PARHIG.hig` from the current configuration.
2. Load observed rainfall from the history database.
3. Normalize rainfall to the hourly grid and interpolate it to the minis.
4. When enabled, incorporate hourly ECMWF rainfall into the forecast block.
5. Write `<workspace>/mgb_runner/Input/chuvabin.hig`.

### 5. Model Execution and Consumption

Library modules:

- `mgb_ops.model.mgb_execution`
- `mgb_ops.model.run_mgb`
- `mgb_ops.model.export_mgb_outputs`

1. Build an execution plan from the executable, input directory, output directory, and workspace root paths.
2. Clear the configured runner output directory.
3. Run the MGB binary or dry-run with `MGB_INPUT_DIR` and `MGB_OUTPUT_DIR` pointing to the configured direct paths.
4. Optionally export the operational MGB output window to SQLite.
5. Read MGB binaries or exported outputs for visualization and downstream use.

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

Manual review of observed rainfall is not implemented yet.

### Reports

Generation of `report_artifact` and publication to `run_catalog` remain pending.
Future reporting should be implemented as importable library behavior.

## Maintained Architectural Direction

Even with implementation gaps, the canonical direction remains:

- Python library first;
- persistent history in SQLite;
- one SQLite file per run;
- raw provider artifacts and normalized fetch outputs in `data/downloads/`;
- disposable model-output extracts such as `data/cache/model_outputs.sqlite` in `data/cache/`;
- reusable derived outputs in `data/processed/`;
- report artifacts in `data/reports/`;
- current configuration in YAML, with `.toml` still under evaluation.
