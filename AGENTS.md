# Agent guidance

## Project status

This is the documentation-only Rust successor to MGB Ops; Rust implementation
has not started. Read [README.md](README.md) for the agreed direction and
[docs/migration/summary.md](docs/migration/summary.md) for the broad roadmap.
Build shared Rust capabilities with a CLI first and a desktop introduced early;
the external MGB executable remains the numerical engine.

## Repository structure

- `README.md`: project entry point and agreed direction.
- `docs/migration/`: broad migration roadmap and responsibility areas.
- `docs/reports/`: completed work, reviews, and validation evidence.
- `references/`: legacy source maps, provenance, and compact regression evidence.
- `.dev/`: temporary implementation documentation.
- `.dev/plans/`: general plans for agreed behavior awaiting implementation.
- `.dev/changes/`: detailed specifications ready for implementation.

Keep the broad migration roadmap in `docs/migration/`. When retiring temporary
implementation documentation, retain durable results in `docs/reports/` or
`references/`. Resolve documentation links relative to their containing file.

## External references and runtime

- `../legacy_python/` on `legacy/python`: Python reference worktree.
- `../legacy_vbnet/` on `legacy/vbnet`: curated VB.NET and plugin reference worktree.
- `../rs_hydro_forecast/`: external regional runtime workspace.
- `/workspace/regression-rs-hourly-r1`: large regression artifacts outside Git;
  see [the case record](references/regression/rs-hourly/README.md).

Use the source maps in `references/` to locate legacy behavior. Preserve legacy
worktrees, operational source data, historical provenance, and frozen regression
evidence. Run experiments in disposable workspaces; keep large data, binaries,
and credentials outside Git.
