# Core and CLI

## Owns

- Shared application workflows for workspace paths, explicit configuration,
  reference times, logging, input snapshots, execution status, and publication.
- Store contracts and private persistence for history, registered assets, and
  operations, growing with the capabilities that need them.
- Shared Rust functions and structured results usable by both CLI and desktop.
- CLI stages for acquisition, review/corrections, preparation, execution,
  reporting, and completed-run operations, plus a complete operation sequence.

## Starting references

Python: `apps/cli/`, `src/mgb_ops/config/`, `src/mgb_ops/assets/sql/`,
`src/mgb_ops/assets/history.py`, and `src/mgb_ops/assets/current_run.py`.

Runtime target: `rs_hydro_forecast/`, including its configuration, data layout,
and operational scripts. Port required script behavior into the Rust package.

## Implementation direction

Start with the [foundation specification](../../.dev/changes/rust-foundation.md),
which refines the agreed [plan](../../.dev/plans/rust-foundation.md):
one Cargo package, a shared library and thin CLI, explicit deep-module boundaries,
and read-only inspection of a disposable RS workspace. This roadmap covers a
broader responsibility area than that first increment; persistence grows with
the operations that need it.

The specification uses TOML at `config/workspace.toml`, explicit command-specific
CLI overrides, and a generated example printed to stdout. Rust inspection does
not silently read or convert the legacy YAML configuration. Implementation and
Linux acceptance evidence remain pending.

Keep the existing history/review/scenario flow through the shared library's app
workflows. App owns configuration loading, merging, and path resolution through
private helpers; a separate top-level workspace module is unnecessary. It also
owns selection, execution/scenario coordination, failure handling, scheduling,
export, and publication policy. CLI and desktop share these operations rather
than implementing their own lifecycle rules.

Store owns canonical contracts, metadata, queries, registries, and file lifecycle
with private database/storage implementations. Adapters return normalized data;
analysis owns QC, corrections, forcing calculations, and report rendering; model
owns engine-specific serialization, execution, and decoding. Each of these three
core modules depends only on store among internal owners. App composes them;
core modules never depend on app or interface code. Canonical data and scientific
processing must work independently of persistence and registry discovery.

In the first operational increment, app freezes selected input values or
immutable versions at execution startup and records executable identity. It
coordinates analysis to prepare canonical forcing and model to stage decoded
results. Publish the complete requested scenario set through store with consistent
execution status/references. Failures retain staged outputs/diagnostics and leave
the previous published set official. Define concrete snapshot/publication
mechanisms in that increment; foundation inspection remains read-only.

Pass resolved data, paths, settings, and times explicitly into core operations.
Keep source data immutable and corrections explicit. The desktop must not become
the only way to perform a core operation. SQLite and NetCDF are starting choices;
settle schemas and file layouts with each capability, without requiring supplied
data to be persisted before processing.

Target Windows and Linux, validating on Linux first. Defer Windows checks until
later development; they are not a foundation gate. Checking engine prerequisites
does not establish successful engine execution. Introduce the GUI once the first
shared operational path works, then extend both interfaces with subsequent
capabilities.

## Evidence of completion

The CLI can resolve a disposable copy of `rs_hydro_forecast` and execute the
implemented stages with useful results and failure reporting while preserving
source data. Check the working path against the minimal regression target as
stages become available. Record actual platform evidence and pending checks;
verify Windows support later as implementation proceeds.
