# Operational Hydrology and Forecasting Library

Local-first Python package for hydrology, rainfall, quality control, and MGB
forecasting workflows. The primary interface is the installable `mgb_ops`
library: users should be able to import modules and functions from notebooks,
scripts, and data-flow style orchestration.

## Current Status

The repository already provides a functional base for:

- bootstrapping `<workspace>/data/history.sqlite` and `<workspace>/data/runs/<run_id>.sqlite`;
- ingesting ANA observations for `rain`, `level`, and `flow`;
- ingesting ECMWF source GRIB internally and registering canonical CF-style NetCDF precipitation grids in the history database;
- preparing metadata and timestep-aligned rainfall inputs for MGB;
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
|       |-- adapters/
|       |-- analysis/
|       |-- assets/
|       |   `-- sql/
|       |-- common/
|       |-- edit/
|       |-- model/
|       |-- qc/
|       |-- storage/
|       `-- workflows/
|-- apps/
|   `-- ops_dashboard/
`-- tests/
```

Important: the user is responsible for providing a regional workspace containing
`data/`, `logs/`, and `mgb_runner/`.

## Layer Model

- `common`: shared types and explicit runtime/configuration helpers.
- `assets`: external-file contracts and readers/writers, including SQL schemas,
  forecast NetCDF, and spatial GeoPackage layers.
- `adapters`: provider-specific collection and normalization.
- `storage`: SQLite persistence and asset registry operations.
- `analysis`: read-only queries, projections, interpolation, and aggregation.
- `edit` and `qc`: auditable corrections and validation rules.
- `model`: MGB preparation, execution, and output production.
- `workflows`: Python-first orchestration across those capabilities.
- `apps/ops_dashboard`: thin Panel UI, session state, and presentation services.

Future operational reporting should be added as a library capability only when
the reporting workflow is designed.

## Runtime and Configuration

- Official runtime contract: `Python >= 3.11`
- Canonical configuration in this phase:
  - module-owned in-code defaults;
  - `<workspace>/config/custom.yaml` as the only editable regional override.
- `.env` files are loaded only by `mgb_ops.common` runtime helpers. Core domain modules do not load `.env` or inspect process environment.
- Library calls should pass explicit workspace, database, schema, asset,
  settings, path, and time inputs into adapters, storage, analysis, edit, QC,
  model, and workflow functions.
- The evaluation of migrating configuration to `.toml` remains open, with no contract change for now.

## Local Setup

Install dependencies into the current root Python environment for full local use:

```bash
python -m pip install -e '.[dev,data,geo]'
```

On Windows PowerShell:

```powershell
python -m pip install -e ".[dev,data,geo]"
```

For the operational dashboard:

```bash
python -m pip install -e '.[dashboard]'
panel serve apps/ops_dashboard/serve.py --show --args --workspace scratch/rs_hydro
```

For a reverse proxy, pass its public host with
`--allow-websocket-origin dashboard.example.org` (including the port when it is
non-standard). If the app is mounted below a path, also pass Panel's
`--prefix /that-path` option and configure the proxy to forward WebSocket
upgrade headers.

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
timestep_hours = int(settings["run"]["timestep_hours"])

rewrite_mgb_meta(
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    reference_time=reference_time,
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    timestep_hours=timestep_hours,
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
    timestep_hours=timestep_hours,
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
