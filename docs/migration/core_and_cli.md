# Core and CLI

## Owns

- Regional workspace paths, explicit configuration, reference times, logging,
  and the persistence needed for history, registered assets, and operations.
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

Keep the existing history/assets/review/scenario flow. Separate provider access,
artifact formats, persistence, calculations, model execution, and interface code.
Use the owning modules' private internals for these distinctions rather than
creating a top-level module for each procedure. Assets owns review, numerical
analysis, persistence, and saved operation data; acquisition owns source access
and conversion, and model owns engine mechanics.
Pass workspace, settings, and times explicitly into core operations. Keep raw
source data immutable and corrections explicit. The desktop must not become the
only way to perform a core operation.

Use the existing SQLite and external-file contracts as starting references;
settle necessary schema changes as their features are implemented.

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
