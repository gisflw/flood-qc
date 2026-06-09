# Operational Workflows

## Workflow Implemented Today

### 1. History Bootstrap

1. Initialize `<workspace>/data/history.sqlite`.
2. Load the station inventory into `station`.
3. Ensure the basic `provider` and `variable` catalogs.

### 2. ANA Observation Ingestion

1. Read module-owned defaults and the optional override in `<workspace>/config/custom.yaml`.
2. Fetch hydrometeorological data by station and day.
3. Save raw XML in `<workspace>/data/interim/ana/`.
4. Persist observations in `observed_series` and `observed_value`.
5. Register logs in `logs/fetch_observed_ana/`.

### 2b. INMET Rainfall Ingestion

1. Read module-owned defaults and the optional override in `<workspace>/config/custom.yaml`.
2. Read the local key from `INMET_API_KEY` or `.env`.
3. Query the operational rainfall API by station and day.
4. Save raw payloads in `<workspace>/data/interim/inmet/`.
5. Persist rainfall in `observed_series` and `observed_value`.
6. Register logs in `logs/fetch_observed_inmet/`.

### 3. ECMWF Forecast Ingestion

1. Resolve the cycle from `reference_time`.
2. Download the ECMWF GRIB.
3. Clip the grid to the operational bounding box.
4. Register the canonical asset in `<workspace>/data/history.sqlite`.
5. Register logs in `logs/forecast_grid/`.

### 4. MGB Preparation

1. Rewrite `PARHIG.hig` from the current configuration.
2. Load observed rainfall from the history database.
3. Normalize rainfall to the hourly grid and interpolate it to the minis.
4. When enabled, incorporate hourly ECMWF rainfall into the forecast block.
5. Write `<workspace>/mgb_runner/Input/chuvabin.hig`.

### 5. Model Execution and Consumption

1. Prepare the runner workspace.
2. Run the MGB binary or dry-run.
3. Mirror output back to `<workspace>/mgb_runner/Output`.
4. Read MGB binaries directly in the dashboard for visualization.

### 6. Dashboard

1. Read `<workspace>/data/history.sqlite` for registry and observations.
2. Read MGB runner binaries for mini series.
3. Read accumulated rasters in `<workspace>/data/interim/`.
4. Allow preview and persistence of manual ECMWF forecast corrections.

## Incomplete Flows

### Automatic QC

The schema and states exist, but the flow to:

- generate flags in `qc_flag`
- promote `raw -> curated -> approved`
- automatically release approved inputs

is not operational yet.

### Materialized Operational Run

The run schema exists, but the flow that copies inputs, outputs, derivatives, flags, and lineage to `<workspace>/data/runs/<run_id>.sqlite` is not closed yet.

### Manual Review of Observations

Manual correction for ECMWF forecasts currently exists in the history database. Manual review of observed rainfall is not implemented yet.

### Reports

Generation of `report_artifact` and publication to `run_catalog` remain pending.

## Maintained Architectural Direction

Even with implementation gaps, the canonical direction remains:

- persistent history in SQLite;
- one SQLite file per run;
- processed spatial assets in `data/spatial/`;
- processed series in `data/timeseries/`;
- current configuration in YAML, with `.toml` still under evaluation.
