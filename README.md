# Operational Hydrology and Forecasting Library

Local-first Python package for hydrology, rainfall, quality control, and MGB
forecasting workflows. The primary interface is the installable `mgb_ops`
library: users should be able to import modules and functions from notebooks,
scripts, and data-flow style orchestration.

## Current Status

The repository already provides a functional base for:

- bootstrapping `<workspace>/data/history.sqlite` and `<workspace>/data/runs/<run_id>.sqlite`;
- ingesting ANA observations for `rain`, `level`, and `flow`;
- ingesting ECMWF grids and registering the canonical GRIB in the history database;
- preparing metadata and hourly rainfall inputs for MGB;
- running the real MGB runner or a dry-run.

Still pending in this phase:

- operational INMET rainfall ingestion hardening;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete materialization of operational runs in `<workspace>/data/runs/`;
- generation of operational reports.

## Principles

- Python library first;
- local artifacts first;
- SQLite as the operational baseline;
- one persistent history database at `<workspace>/data/history.sqlite`;
- one SQLite file per run in `<workspace>/data/runs/`;
- rasters and vectors outside the database, with relative paths and metadata;
- QGIS as a complementary client for generated artifacts.

## Main Layout

```text
.
|-- docs/
|-- src/
|   `-- mgb_ops/
|       |-- assets/
|       |   `-- sql/
|       |-- common/
|       |-- ingest/
|       |-- model/
|       |-- qc/
|       `-- storage/
`-- tests/
```

Important: the user is responsible for providing a regional workspace containing
`data/`, `logs/`, and `mgb_runner/`.

## Layer Model

- `src/mgb_ops/common/`, `storage/`, `ingest/`, `qc/`, and `model/`
  Core library modules for notebook and data-flow use.

Future operational reporting should be added as a library capability only when
the reporting workflow is designed.

## Runtime and Configuration

- Official runtime contract: `Python >= 3.11`
- Canonical configuration in this phase:
  - module-owned in-code defaults;
  - `<workspace>/config/custom.yaml` as the only editable regional override.
- `.env` files are loaded only by `mgb_ops.common` runtime helpers. Core domain modules do not load `.env` or inspect process environment.
- Library calls should pass explicit workspace, database, schema, asset, settings, path, and time inputs into `storage`, `ingest`, `qc`, and `model` functions.
- The evaluation of migrating configuration to `.toml` remains open, with no contract change for now.

## Local Setup

Create a virtual environment and install dependencies for full local use:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .[dev,data,geo]
```

On Windows PowerShell:

```powershell
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .[dev,data,geo]
```

## Library Usage

```python
from pathlib import Path

from mgb_ops.common.runtime import build_runtime_context
from mgb_ops.common.time_utils import resolve_reference_time
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta
from mgb_ops.model.prepare_mgb_rainfall import prepare_mgb_rainfall

context = build_runtime_context(workspace=Path("scratch/rs_hydro"))
paths = context.paths
settings = context.settings
mgb_settings = settings["mgb"]
reference_time = resolve_reference_time(settings["run"]["reference_time"])

rewrite_mgb_meta(
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    reference_time=reference_time,
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    logs_dir=paths.logs_dir,
)

prepare_mgb_rainfall(
    history_db=paths.history_db,
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    mini_gtp_path=paths.mgb_input_dir / "MINI.gtp",
    output_path=paths.mgb_input_dir / "chuvabin.hig",
    reference_time=reference_time,
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    use_forecast_data=bool(mgb_settings["use_forecast_data"]),
    nearest_stations=int(settings["rainfall_interpolation"]["nearest_stations"]),
    power=float(settings["rainfall_interpolation"]["power"]),
    logs_dir=paths.logs_dir,
)
```

The exact callable surface is still maturing. When adding new operational
behavior, prefer a reusable library function first.

## Main Components

- `src/mgb_ops/`
  Installable package containing the core library.
- `<workspace>/mgb_runner/`
  Regional MGB artifacts (`Input`, `Output`, and `.exe`) provided by the user.
  Runner code lives in `src/mgb_ops/model/`.
- `src/mgb_ops/assets/sql/`
  Explicit schemas for `history.sqlite`, run databases, and model output exports.
- `docs/`
  Architecture, data model, operations, and workflows.

## History Database vs Run Database

- `<workspace>/data/history.sqlite`
  Stores station metadata, observations, flags, edits, and the run catalog.
- `<workspace>/data/runs/<run_id>.sqlite`
  Stores the closed state of a specific run.

The run schema exists and bootstrap is implemented, but complete operational run
assembly is not finished in this phase.
