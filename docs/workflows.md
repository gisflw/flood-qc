# Operational Workflows

The preferred workflow shape is Python-first: import the relevant `mgb_ops`
module, pass explicit workspace/database/path inputs, and compose the returned
summaries or domain objects in a notebook, script, or orchestrated data flow.

## Workflow Implemented Today

### 1. History Bootstrap

Library module: `mgb_ops.assets.databases`

1. Initialize an explicitly supplied history SQLite path with an explicitly supplied schema path.
2. Load the station inventory into `station` and `station_observed_variable`; every inventory row must provide `observed_variables`, using `none` for stations without observed data.
3. Ensure the basic `provider` and `variable` catalogs.

### 2. ANA Observation Ingestion

Fetch module: `mgb_ops.adapters.observed_ana`
Workflow helpers: `mgb_ops.workflows.observed.fetch_observed_provider`,
`mgb_ops.workflows.observed.discover_observed_provider_csvs`, and
`mgb_ops.workflows.observed.load_observed_provider_csvs`

1. Read ANA stations from the caller-supplied history database.
2. For each station, intersect ANA's provider variables with the station's `observed_variables`, then resume from the latest raw observed day for that supported set, overlapping that day; stations without supported variables are skipped.
3. Fetch hydrometeorological data by station and contiguous date window. The library workflow accepts `fetch_window_days`, defaulting to `30`; use `1` to preserve one request per day.
4. Save provider XML as ancillary evidence per fetch window under `<workspace>/data/downloads/ana/<run_id>/<station_code>/<YYYYMMDD>__<YYYYMMDD>.xml` and write one normalized observed CSV per station per run under `<workspace>/data/downloads/ana/<run_id>/<station_code>/observed.csv`.
5. Load normalized CSVs through `mgb_ops.workflows.observed.load_observed_provider_csvs()` into `observed_series` and `observed_value`, passing `run.timestep_hours` and the observed aggregation policy so SQLite stores timestep-normalized rows. Use `discover_observed_provider_csvs()` when re-loading already downloaded artifacts.
6. Register logs in `logs/observed_ana/`.

### 2b. INMET Rainfall Ingestion

Fetch module: `mgb_ops.adapters.observed_inmet`
Workflow helpers: `mgb_ops.workflows.observed.fetch_observed_provider`,
`mgb_ops.workflows.observed.discover_observed_provider_csvs`, and
`mgb_ops.workflows.observed.load_observed_provider_csvs`

1. Read INMET stations from the caller-supplied history database.
2. Resolve the local key in the thin CLI/app layer; pass `api_key` explicitly to the fetch workflow. Loading existing normalized CSVs does not require an API key.
3. For each station, use its `observed_variables` capability before planning rain requests; stations without supported variables are skipped, and stations with data resume from the latest raw observed rain day with one-day overlap.
4. Query the operational rainfall API by station and contiguous date window, using the explicit `product_code` input that defaults to `I175`. The library workflow accepts `fetch_window_days`, defaulting to `30`; use `1` to preserve one request per day.
5. Save each successful raw JSON response under `<workspace>/data/downloads/inmet/<run_id>/<station_code>/<YYYYMMDD>__<YYYYMMDD>.json`, where the two dates are the request window start and end.
6. Build one normalized observed rainfall CSV per station per run from the saved raw responses under `<workspace>/data/downloads/inmet/<run_id>/<station_code>/observed.csv`. If a later day fails after earlier days succeeded, keep the partial CSV so successful data can still be imported while the station is marked as an error.
7. Load normalized CSVs through `mgb_ops.workflows.observed.load_observed_provider_csvs()` into `observed_series` and `observed_value`, passing `run.timestep_hours` and the observed aggregation policy so SQLite stores timestep-normalized rows. Use `discover_observed_provider_csvs()` when re-loading already downloaded artifacts.
8. Register logs in `logs/observed_inmet/`.

### 3. Forecast Grid Ingestion

Library modules:

- `mgb_ops.adapters.forecast_ecmwf`
- `mgb_ops.assets.spatial_grid`
- `mgb_ops.assets.grid_transforms`
- `mgb_ops.workflows.forecast`

Install the optional forecast dependencies in the operational environment:

```bash
python -m pip install -e ".[forecast]"
```

1. Resolve the cycle from `reference_time`.
2. Download the configured ECMWF GRIB source inside the adapter.
3. Expand the model bbox by 50% of its width and height on every side, then
   retain every native source cell whose footprint touches that buffered bbox.
4. Crop the native grid by retaining every source cell whose footprint touches
   the configured bbox.
5. Convert cumulative precipitation to native 3/6-hour interval totals and
   write a CF-style NetCDF with authoritative coordinate and time bounds. No
   spatial or MGB-timestep resampling occurs in the registered asset.
6. Register the canonical NetCDF with `asset_kind="spatial_grid"` and
   `type="forecast"`.
7. Register logs in `logs/forecast_ecmwf/`.

Python callers pass the unbuffered model `bbox=(west, south, east, north)` to forecast ingestion.
The configured working resolution and MGB timestep are accepted by legacy
entrypoints but are applied only later by model preparation, never to the
registered forecast asset.

### 3b. Observed Precipitation Grid

Library module: `mgb_ops.assets.observed_precipitation`

The asset layer selects preferred observed series, interpolates timestep values
with `mgb_ops.assets.grid_transforms`, and atomically writes the disposable
dashboard cache using the canonical `assets.spatial_grid` contract. This is an
asset-construction operation rather than a workflow or analysis API.

### 4. MGB Preparation

Library modules:

- `mgb_ops.model.prepare_mgb_meta`
- `mgb_ops.model.prepare_mgb_rainfall`

1. Rewrite `PARHIG.hig` from the current configuration, including `DT = run.timestep_hours * 3600`.
2. Build deterministic observed and forecast working-grid caches at the
   configured spatial resolution for their complete portions of the MGB window,
   including every cell whose closed footprint touches the configured bbox.
3. Uniformly split native forecast interval totals into MGB timesteps and
   bilinearly resample them to the working grid.
4. IDW-interpolate each cache timestep from grid-cell centroids to the
   `MINI.gtp` centroids, preserving mini order.
5. Write `<workspace>/mgb_runner/Input/chuvabin.hig`.

### 5. Model Execution and Consumption

Library modules:

- `mgb_ops.model.mgb_execution`
- `mgb_ops.model.run_mgb`
- `mgb_ops.model.export_mgb_outputs`

1. Build an execution plan from the executable, input directory, output directory, and workspace root paths.
2. Clear the configured runner output directory.
3. Run the MGB binary or dry-run with `MGB_INPUT_DIR` and `MGB_OUTPUT_DIR` pointing to the configured direct paths.
4. Optionally export the operational MGB output window to NetCDF.
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
- disposable model-output extracts in `data/cache/`;
- reusable derived outputs such as `data/processed/model_outputs.nc` in `data/processed/`;
- report artifacts in `data/reports/`;
- current configuration in YAML, with `.toml` still under evaluation.


## Multi-scenario forecast execution

Operational forecast runs are derived at runtime from history.sqlite. The provider
registry selects active forecast providers; each eligible registered asset produces
a raw scenario, and each linked manual_edit row produces one independent corrected
scenario. A zero-rain scenario is always included. YAML does not select
forecast providers; enablement belongs exclusively to the provider registry.

mgb_ops.workflows.derive_forecast_scenarios() builds immutable, transient scenario
descriptions. mgb_ops.workflows.execute_forecast_scenarios() runs them in isolated
runner directories concurrently and publishes a complete batch only when every run
succeeds. Dashboard artifacts live under data/cache/forecast_scenarios/<batch>/,
with latest.json pointing to the current complete batch. These caches and scenario
descriptions are not registered as persistent inputs.
