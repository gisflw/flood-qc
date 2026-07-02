# Conceptual Data Model

## Current Status

The canonical model remains split between:

- persistent history in `<workspace>/data/history.sqlite`;
- one file per run in `<workspace>/data/runs/<run_id>.sqlite`.

Today the history database is used operationally by ANA, ECMWF, and manual
forecast corrections. These stores should be accessed through library functions
where possible. The run schema already exists, but its operational
materialization is not complete yet.

## Main History Entities

### `provider`

Catalog of operational sources, including at least `ana`, `inmet`, and `ecmwf`.

### `variable`

Catalog of canonical variables. In this phase, the history database works with:

- `rain`
- `level`
- `flow`

### `station`

Unified operational station registry. The primary identity is `station_id`, a canonical string in the form `{provider_code}:{normalized_station_code}`. Examples include `ana:74100000` and `inmet:A801`.

`provider_code` and `station_code` remain separate searchable columns, with duplicate detection on the provider-local code pair as well as the canonical `station_id`. The initial inventory comes from `<workspace>/data/source/history_station_inventory.csv`, and bootstrap derives `station_id` from the normalized provider and station code.

### `observed_series`

Defines an observed series by combination of:

- `station_id`
- `variable_code`
- `state`

The canonical states remain:

- `raw`
- `curated`
- `approved`

In the repository's current state, the history database in active use is still mostly `raw`.

### `observed_value`

Long-format time table, with one value per `series_id + observed_at`. Observed provider fetchers write normalized CSV artifacts first, then `mgb_ops.assets.observations.load_normalized_observed_csvs()` buckets and aggregates values to `run.timestep_hours` before persisting rows into `observed_series` and `observed_value`.

The normalized observed CSV columns are:

- `station_id`
- `provider_code`
- `station_code`
- `observed_at`
- `variable_code`
- `value`
- `state`

Fetchers write one normalized CSV per station per run, for example
`data/downloads/ana/<run_id>/<station_code>/observed.csv`. Assets owns both the CSV contract and SQLite loading.

### `asset`

Generic registry of external files. Operational forecast grids are canonical CF-style NetCDF assets registered by `provider_code` and `asset_kind`, with ECMWF as the default forecast product configuration. ECMWF GRIB2 files are source-adapter inputs and are not registered as operational forecast assets.

Canonical gridded products use `asset_kind="spatial_grid"`, `format="NetCDF"`,
and metadata identifying `variable`, `type`, `source`, and `providers`.
Coordinates and time bounds are UTC and payload variables use zlib compression
level 4. Forecast grids are registered; the observed precipitation dashboard
cache at `data/cache/precipitations_observed.nc` is disposable and unregistered.

### `qc_flag`

Canonical structure for quality flags without overwriting the original data. The schema is implemented, but automatic QC does not yet populate this table operationally.

### `manual_edit`

In the current history database, this table is used for manual GRIB2 forecast corrections by asset and time window. There is no equivalent implemented contract yet for manual correction of observed rainfall.

### `run_catalog`

Index of published or available runs. The schema exists, but the current flow does not populate the catalog yet.

## Main Run Entities

The run database is still modeled to store:

- run header in `run`;
- local input copy in `run_input_series` and `run_input_value`;
- run assets in `run_asset`;
- operational derivatives in `derived_series` and `derived_value`;
- model execution in `model_execution`;
- operational subset of MGB outputs in `mgb_output_series` and `mgb_output_value`;
- local flags, edits, and report artifacts.

This contract remains valid, but the repository layer and run assembly are still incomplete in this phase.

## Separation Between History and Complete Outputs

Complete MGB output remains outside SQLite, in the canonical runner binaries:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`

Library readers use these binaries with support from:

- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

This behavior is implemented and is the current operational path for model visualization.

## Spatial Assets and Rasters

The contract remains:

- rasters and vectors live outside SQLite;
- the database stores only metadata and relative paths.

`<workspace>/data/processed/` remains the canonical destination for reusable derived outputs, including processed spatial assets, although some spatial inputs still come from legacy material in `<workspace>/data/legacy/app_layers/`.

## Configuration

The repository's operational configuration remains in:

- module-owned in-code defaults
- `<workspace>/config/custom.yaml` when present

The possible migration to `.toml` remains under evaluation and does not yet
change the data model or the runtime contract for this phase. Library functions
should prefer explicit path/config inputs where practical so notebooks, scripts,
and orchestrated data flows can share the same model.

## Canonical Schemas

The implemented canonical schemas live in:

- `src/mgb_ops/assets/sql/history_schema.sql`
- `src/mgb_ops/assets/sql/run_schema.sql`
