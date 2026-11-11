# Core and CLI

## Scope and references

Shared library workflows cover workspace/configuration, selection, input snapshots,
execution status, publication, and commands for acquisition, review, preparation,
execution, reporting, and completed-run operations. Persistence grows with the
operations that need it; CLI and desktop share those workflows.

Python starting points: `apps/cli/`, `src/mgb_ops/config/`,
`src/mgb_ops/assets/sql/`, `src/mgb_ops/assets/history.py`, and
`src/mgb_ops/assets/current_run.py`. The regional target is `rs_hydro_forecast/`,
including its operational scripts. Port required script behavior into Rust.

## Direction

Follow the [architecture](../../docs/architecture.md) for module boundaries and execution
lifecycle. Pass resolved data, paths, settings, and times into core operations.
Preserve source data and record corrections explicitly. The desktop must not be
the only way to perform a core operation. SQLite/NetCDF schemas and layouts are
settled with their operational capabilities.

The [workspace contract](../../docs/contracts/workspace.md) describes current read-only
inspection, TOML configuration, dedicated overrides, paths, findings, and limits.
Rust does not silently read or convert legacy YAML. Current validation and review
gaps belong in the [migration summary](summary.md#current-migration-state).

The first operational path adds saved RS preparation, real execution, and decoded
results against the frozen reference, with snapshots and complete-set publication.
Introduce the desktop after that path works, then extend both interfaces alongside
the core. Target Linux and Windows; validate Linux first and qualify Windows later.

## Acceptance

Implemented commands produce useful results/failures in disposable workspaces and
preserve source data. Compare operational stages against the frozen regression case
as they arrive. Inspection alone cannot establish successful engine execution.
Record actual platform evidence and remaining gaps in the migration checklist.
