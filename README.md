# Operational Hydrology and Forecasting System

Local-first operational base for hydrology, rainfall, and forecasting, organized around regional workspaces, SQLite, and an installable CLI.

## Current Status

The repository already provides a functional base for:

- bootstrapping `<workspace>/data/history.sqlite` and `<workspace>/data/runs/<run_id>.sqlite`;
- ingesting ANA observations for `rain`, `level`, and `flow`;
- ingesting ECMWF grids and registering the canonical GRIB in the history database;
- preparing metadata and hourly rainfall inputs for MGB;
- running the real MGB runner or a dry-run;
- a Streamlit dashboard for monitoring, MGB series, and ECMWF forecast preview/manual correction.

Still pending in this phase:

- operational INMET rainfall ingestion;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete materialization of operational runs in `<workspace>/data/runs/`;
- generation of operational reports.

## Principles

- local artifacts first;
- SQLite as the operational baseline;
- one persistent history database at `<workspace>/data/history.sqlite`;
- one SQLite file per run in `<workspace>/data/runs/`;
- rasters and vectors outside the database, with relative paths and metadata;
- Streamlit as the main interface;
- QGIS as a complementary client for generated artifacts.

## Main Layout

```text
.
|-- config/
|-- docs/
|-- examples/
|   `-- rs_hydro/
|       |-- data/
|       |-- logs/
|       `-- mgb_runner/
|-- sql/
|-- src/
|   |-- mgb_ops/
|   |-- ops_dashboard/
|   |-- common/
|   |-- ingest/
|   |-- model/
|   |-- qc/
|   |-- reporting/
|   `-- storage/
`-- tests/
```

Important: the user is responsible for providing a regional workspace containing `data/`, `logs/`, and `mgb_runner/`. The repository includes `examples/rs_hydro/` as a test workspace with RS artifacts.

## Runtime and Configuration

- Official runtime contract: `Python >= 3.11`
- Canonical configuration in this phase:
  - `config/default.yaml` as the packaged default;
  - `<workspace>/config/custom.yaml` as an optional regional override;
  - `config/custom.yaml` remains supported for local compatibility.
- If `--workspace` is not provided, the CLI uses `MGB_OPS_WORKSPACE` and then the current directory.
- The evaluation of migrating configuration to `.toml` remains open, with no contract change for now.

The initial station inventory lives at `<workspace>/data/interim/history_station_inventory.csv`. During history bootstrap, the system computes `station_uid` as `1000000000 + code` for ANA and `2000000000 + code` for INMET, converting letters in the code to numbers (`A=1`, `B=2`, etc.).

## Local Setup

Create a virtual environment and install dependencies for full local use:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .[dev,data,geo,ui]
```

On Windows PowerShell:

```powershell
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .[dev,data,geo,ui]
```

## Canonical Entry Points

```bash
mgb-ops --workspace examples/rs_hydro bootstrap history
mgb-ops --workspace examples/rs_hydro ingest ana
mgb-ops --workspace examples/rs_hydro ingest inmet
mgb-ops --workspace examples/rs_hydro ingest forecast-grid
mgb-ops --workspace examples/rs_hydro model prepare-meta
mgb-ops --workspace examples/rs_hydro model prepare-rainfall
mgb-ops --workspace examples/rs_hydro model run --dry-run
mgb-ops --workspace examples/rs_hydro model export-outputs
mgb-ops --workspace examples/rs_hydro dashboard
```

To run INMET ingestion, set `INMET_API_KEY` in the environment or fill `.env` from `.env.example`.

## Main Components

- `src/ops_dashboard/`
  Operational Streamlit dashboard for monitoring, observed series, MGB series, and ECMWF forecast preview/correction.
- `<workspace>/mgb_runner/`
  Regional MGB artifacts (`Input`, `Output`, and `.exe`) provided by the user. Runner code lives in `src/model/`.
- `src/mgb_ops/`
  `mgb-ops` CLI that runs headless commands and starts/prints the dashboard.
- `src/`
  Domain modules split across ingestion, QC, model, storage, reporting, and common utilities.
- `sql/`
  Explicit schemas for `history.sqlite` and `run.sqlite`.
- `docs/`
  Architecture, data model, operations, and workflows.

## History Database vs Run Database

- `<workspace>/data/history.sqlite`
  Stores station metadata, observations, flags, edits, and the run catalog.
- `<workspace>/data/runs/<run_id>.sqlite`
  Stores the closed state of a specific run.

The run schema exists and bootstrap is implemented, but complete operational run assembly is not finished in this phase.
