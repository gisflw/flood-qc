# Operations and Conventions

## Local Setup

1. Create a virtual environment with `Python 3.11+`.
2. Install dependencies with `pip install -e .[dev,data,geo,ui]`.
3. Adjust `config/default.yaml` when operational defaults need to change.
4. Use `<workspace>/config/custom.yaml` for optional regional overrides.

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

- `config/default.yaml` as the packaged default;
- `<workspace>/config/custom.yaml` when present;
- `config/custom.yaml` as a local compatibility fallback.

The regional workspace is provided through `mgb-ops --workspace PATH`, `MGB_OPS_WORKSPACE`, or the current directory. Each workspace must contain `data/`, `logs/`, and `mgb_runner/`. The possible migration to `.toml` remains under evaluation.

## Common Entry Points

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

`mgb-ops ingest inmet` requires `INMET_API_KEY` in the local environment or in `.env`.

## Naming Conventions

- `run_id`: preferably `YYYYMMDDTHHMMSS`
- `history.sqlite`: single history database
- `<workspace>/data/runs/<run_id>.sqlite`: one file per run
- external assets with relative paths whenever possible

## Maturity States

- `raw`: data ingested without complete review
- `curated`: data processed by automatic rules or preprocessing
- `approved`: data released for operational use

The schema and dashboard consumption already respect this convention, although the automatic promotion flow between states is still pending.

## Complete Artifact vs Run

The current operational flow directly uses complete runner artifacts:

- `<workspace>/mgb_runner/Output/QTUDO_Inercial_Atual.MGB`
- `<workspace>/mgb_runner/Output/YTUDO.MGB`
- `<workspace>/mgb_runner/Input/PARHIG.hig`
- `<workspace>/mgb_runner/Input/MINI.gtp`

The run schema is still expected to store the operational subset and the closed context of the cycle, but that step is not complete in the current pipeline.

## Raster and Vector Paths

- store relative paths in the database whenever possible
- do not store rasters as SQLite blobs
- preserve `data/spatial/` as the canonical destination for processed layers, even if part of current consumption still uses legacy artifacts

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
