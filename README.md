# Operational Hydrology and Forecasting Library

Local-first Python package for hydrology, rainfall, quality control, and MGB
forecasting workflows. The primary interface is the installable `mgb_ops`
library: users should be able to import modules and functions from notebooks,
scripts, and data-flow style orchestration. The CLI and Streamlit dashboard are
thin operational layers over those library functions.

## Current Status

The repository already provides a functional base for:

- bootstrapping `<workspace>/data/history.sqlite` and `<workspace>/data/runs/<run_id>.sqlite`;
- ingesting ANA observations for `rain`, `level`, and `flow`;
- ingesting ECMWF grids and registering the canonical GRIB in the history database;
- preparing metadata and hourly rainfall inputs for MGB;
- running the real MGB runner or a dry-run;
- using a Streamlit dashboard for monitoring, MGB series, and ECMWF forecast preview/manual correction.

Still pending in this phase:

- operational INMET rainfall ingestion hardening;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete materialization of operational runs in `<workspace>/data/runs/`;
- generation of operational reports.

## Principles

- Python library first;
- CLI and GUI as thin wrappers over library modules;
- local artifacts first;
- SQLite as the operational baseline;
- one persistent history database at `<workspace>/data/history.sqlite`;
- one SQLite file per run in `<workspace>/data/runs/`;
- rasters and vectors outside the database, with relative paths and metadata;
- Streamlit as an operational dashboard, not the core architecture;
- QGIS as a complementary client for generated artifacts.

## Main Layout

```text
.
|-- apps/
|   `-- ops_dashboard/
|-- docs/
|-- src/
|   `-- mgb_ops/
|       |-- assets/
|       |   `-- sql/
|       |-- cli/
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
- `src/mgb_ops/cli/`
  Thin command-line wrapper that resolves arguments/settings and calls library
  functions.
- `apps/ops_dashboard/`
  Streamlit dashboard layer for operational monitoring and manual forecast
  correction. Dashboard-owned support helpers live under
  `apps/ops_dashboard/support/`.

There is no `mgb_ops.reporting` package yet. Future operational reporting should
be added as a library capability only when the reporting workflow is designed.

## Runtime and Configuration

- Official runtime contract: `Python >= 3.11`
- Canonical configuration in this phase:
  - module-owned in-code defaults;
  - `<workspace>/config/custom.yaml` as the only editable regional override.
- Library calls should prefer explicit workspace, database, path, and time inputs when available.
- The evaluation of migrating configuration to `.toml` remains open, with no contract change for now.

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

## Library Usage

```python
from pathlib import Path

from mgb_ops.ingest.fetch_observed_ana import ingest_observed_ana
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta_from_config
from mgb_ops.model.prepare_mgb_rainfall import prepare_mgb_rainfall
from mgb_ops.common.paths import runtime_paths

workspace = Path("examples/rs_hydro")
paths = runtime_paths(workspace)

rewrite_mgb_meta_from_config(
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    logs_dir=paths.logs_dir,
    workspace=workspace,
)

prepare_mgb_rainfall(
    history_db=paths.history_db,
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    mini_gtp_path=paths.mgb_input_dir / "MINI.gtp",
    output_path=paths.mgb_input_dir / "chuvabin.hig",
    logs_dir=paths.logs_dir,
    workspace=workspace,
)
```

The exact callable surface is still maturing. When adding new operational
behavior, prefer a reusable library function first, then expose it through the
CLI or dashboard only as needed.

## CLI Wrapper

The CLI remains useful for repeatable local operations, but it is a wrapper over
the Python modules:

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

To run INMET ingestion, set `INMET_API_KEY` in the environment or fill `.env`
from `.env.example`.

## Main Components

- `src/mgb_ops/`
  Installable package containing the core library and CLI wrapper.
- `apps/ops_dashboard/`
  Operational Streamlit dashboard for monitoring, observed series, MGB series,
  and ECMWF forecast preview/correction.
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
