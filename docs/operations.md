# Operations and Conventions

Operational use can happen either through direct Python calls or through the
`mgb-ops` CLI wrapper. The preferred architecture is still library-first: CLI
commands should resolve local runtime details, call `mgb_ops` functions, and
report results.

## Local Setup

1. Create a virtual environment with `Python 3.11+`.
2. Install dependencies with `pip install -e .[dev,data,geo,ui]`.
3. Use `<workspace>/config/custom.yaml` for optional regional overrides.

Typical Linux/macOS setup:

```bash
python -m venv .venv
source .venv/bin/activate
pip install -e .[dev,data,geo,ui]
```

Typical Windows PowerShell setup:

```powershell
python -m venv .venv
.venv\Scripts\Activate.ps1
pip install -e .[dev,data,geo,ui]
```

## Operational Configuration

The runtime reads:

- module-owned in-code defaults;
- `<workspace>/config/custom.yaml` when present.

The regional workspace is provided through explicit Python arguments,
`mgb-ops --workspace PATH`, `MGB_OPS_WORKSPACE`, or the current directory. Each
workspace must contain `data/`, `logs/`, and `mgb_runner/`. The possible
migration to `.toml` remains under evaluation.

For library calls, prefer passing explicit `Path` values for the workspace,
history database, MGB input/output files, interim directories, and log
directories whenever the function supports them.

## Python-First Operation

Notebook or script workflows should import the relevant module and call the
library function directly. For example:

```python
from pathlib import Path

from mgb_ops.common.paths import runtime_paths
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta_from_config

workspace = Path("examples/rs_hydro")
paths = runtime_paths(workspace)

summary = rewrite_mgb_meta_from_config(
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    logs_dir=paths.logs_dir,
    workspace=workspace,
)
```

New operational code should preserve this shape: reusable library behavior first,
thin CLI/dashboard exposure second.

## CLI Wrapper Commands

The CLI remains convenient for manual operation and automation:

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

`mgb-ops ingest inmet` requires `INMET_API_KEY` in the local environment or in
`.env`.

## Naming Conventions

- `run_id`: preferably `YYYYMMDDTHHMMSS`
- `history.sqlite`: single history database
- `<workspace>/data/runs/<run_id>.sqlite`: one file per run
- external assets with relative paths whenever possible

## Maturity States

- `raw`: data ingested without complete review
- `curated`: data processed by automatic rules or preprocessing
- `approved`: data released for operational use

The schema and dashboard consumption already respect this convention, although
the automatic promotion flow between states is still pending.

## Complete Artifact vs Run

The current operational flow directly uses complete runner artifacts:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`
- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

The run schema is still expected to store the operational subset and the closed
context of the cycle, but that step is not complete in the current pipeline.

## Raster and Vector Paths

- store relative paths in the database whenever possible
- do not store rasters as SQLite blobs
- preserve `data/spatial/` as the canonical destination for processed layers,
  even if part of current consumption still uses legacy artifacts

## Destructive Editing and Audit

- do not overwrite source data
- register flags and edits in append-only form when applicable
- create a derived manual run instead of modifying an automatic run in place

Every relevant transformation must make explicit:

- data or asset origin
- time of change
- responsible operator or service
- reason for the change
- relationship with the impacted run, when applicable
