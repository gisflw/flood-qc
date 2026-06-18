# Operations and Conventions

Operational use should happen through direct Python calls. Import the relevant
`mgb_ops` module, pass explicit workspace/database/path inputs, and compose the
returned summaries or domain objects in a notebook, script, or orchestrated data
flow.

## Local Setup

1. Use the current root Python environment with `Python 3.11+`.
2. Install dependencies with `python -m pip install -e '.[dev,data,geo]'`.
3. Use `<workspace>/config/custom.yaml` for optional regional settings overrides.
4. Use `<workspace>/.env` only for runtime convenience values consumed by `mgb_ops.common`.

Typical Linux/macOS setup:

```bash
python -m pip install -e '.[dev,data,geo]'
```

Typical Windows PowerShell setup:

```powershell
python -m pip install -e ".[dev,data,geo]"
```

## Operational Configuration

Common runtime helpers read:

- module-owned in-code defaults;
- `<workspace>/config/custom.yaml` when present;
- `<workspace>/.env` for runtime convenience values such as `INMET_API_KEY` and `MGB_OPS_REMOTE_WORKSPACE`.

Precedence is explicit Python arguments first, process environment second,
`.env` third, and defaults last. `.env` loading is intentionally limited to
`mgb_ops.common`. Domain modules under `storage`, `ingest`, `qc`, and `model`
require explicit inputs and must not inspect process environment or workspace
state.

The regional workspace is provided through explicit Python arguments,
`MGB_OPS_WORKSPACE`, workspace `.env`, or the current directory. Each workspace
must contain `data/`, `logs/`, and `mgb_runner/`. The possible migration to
`.toml` remains under evaluation.

For core library calls, pass explicit `Path` values for database paths, schema
paths, station inventory CSVs, MGB input/output files, asset base directories,
download directories, and log directories.

## Python-First Operation

Notebook or script workflows should import the relevant module and call the
library function directly. For example:

```python
from pathlib import Path

from mgb_ops.common.runtime import build_runtime_context
from mgb_ops.common.time_utils import resolve_reference_time
from mgb_ops.model.prepare_mgb_meta import rewrite_mgb_meta

context = build_runtime_context(workspace=Path("scratch/rs_hydro"))
paths = context.paths
settings = context.settings
mgb_settings = settings["mgb"]

summary = rewrite_mgb_meta(
    parhig_path=paths.mgb_input_dir / "PARHIG.hig",
    reference_time=resolve_reference_time(settings["run"]["reference_time"]),
    input_days_before=int(mgb_settings["input_days_before"]),
    forecast_horizon_days=int(mgb_settings["forecast_horizon_days"]),
    logs_dir=paths.logs_dir,
)
```

New operational code should preserve this shape: reusable library behavior
first, with explicit inputs and structured return values.

## Naming Conventions

- `run_id`: preferably `YYYYMMDDTHHMMSS`
- `history.sqlite`: single history database
- `<workspace>/data/runs/<run_id>.sqlite`: one file per run
- `station_id`: canonical `{provider_code}:{normalized_station_code}`, for example `ana:74100000` or `inmet:A801`
- normalized observed CSVs, one file per station per run: `station_id,provider_code,station_code,observed_at,variable_code,value,state`
- external assets with relative paths whenever possible

## Maturity States

- `raw`: data ingested without complete review
- `curated`: data processed by automatic rules or preprocessing
- `approved`: data released for operational use

The schema already respects this convention, although the automatic promotion
flow between states is still pending.

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
- preserve `data/processed/` as the canonical destination for reusable derived
  layers, even if part of current consumption still uses legacy artifacts

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
