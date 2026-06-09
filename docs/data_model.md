# Conceptual Data Model

## Current Status

The canonical model remains split between:

- persistent history in `<workspace>/data/history.sqlite`;
- one file per run in `<workspace>/data/runs/<run_id>.sqlite`.

Today the history database is used operationally by ANA, ECMWF, the dashboard, and manual forecast corrections. The run schema already exists, but its operational materialization is not complete yet.

## Main History Entities

### `provider`

Catalog of operational sources, including at least `ana`, `inmet`, and `ecmwf`.

### `variable`

Catalog of canonical variables. In this phase, the history database works with:

- `rain`
- `level`
- `flow`

### `station`

Unified operational station registry. The logical identity remains `provider_code + station_code`.

The initial inventory comes from `<workspace>/data/interim/history_station_inventory.csv`. Bootstrap computes `station_uid` by provider, including alphanumeric INMET codes.

### `observed_series`

Defines an observed series by combination of:

- `station_uid`
- `variable_code`
- `state`

The canonical states remain:

- `raw`
- `curated`
- `approved`

In the repository's current state, the history database in active use is still mostly `raw`.

### `observed_value`

Long-format time table, with one value per `series_id + observed_at`.

### `asset`

Generic registry of external files. It is already used operationally for ECMWF assets.

### `qc_flag`

Canonical structure for quality flags without overwriting the original data. The schema is implemented, but automatic QC does not yet populate this table operationally.

### `manual_edit`

In the current history database, this table is used for manual ECMWF forecast corrections by asset and time window. There is no equivalent implemented contract yet for manual correction of observed rainfall.

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
- local flags, edits, and reports.

This contract remains valid, but the repository layer and run assembly are still incomplete in this phase.

## Separation Between History and Complete Outputs

Complete MGB output remains outside SQLite, in the canonical runner binaries:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`

The dashboard reads these binaries directly with support from:

- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

This behavior is implemented and is the current operational path for model visualization.

## Spatial Assets and Rasters

The contract remains:

- rasters and vectors live outside SQLite;
- the database stores only metadata and relative paths.

`<workspace>/data/spatial/` remains the canonical destination for processed spatial assets, but the dashboard map still depends on legacy material in `<workspace>/data/legacy/app_layers/`.

## Configuration

The repository's operational configuration remains in:

- module-owned in-code defaults
- `<workspace>/config/custom.yaml` when present

The possible migration to `.toml` remains under evaluation and does not yet change the data model or the runtime contract for this phase.

## Canonical Schemas

The implemented canonical schemas live in:

- `src/mgb_ops/assets/sql/history_schema.sql`
- `src/mgb_ops/assets/sql/run_schema.sql`
