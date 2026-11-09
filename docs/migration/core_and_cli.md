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

Keep the existing history/assets/review/scenario flow. Separate provider access,
artifact formats, persistence, calculations, model execution, and interface code.
Pass workspace, settings, and times explicitly into core operations. Keep raw
source data immutable and corrections explicit. The desktop must not become the
only way to perform a core operation.

Use the existing SQLite and external-file contracts as starting references;
settle necessary schema changes as their features are implemented.

Check Windows/Linux feasibility, including external-engine availability, during
this foundation step. Introduce the GUI once the first shared operational path
works, then extend CLI and GUI coverage with subsequent capabilities.

## Evidence of completion

The CLI can resolve a disposable copy of `rs_hydro_forecast` and execute the
implemented stages with useful results and failure reporting while preserving
source data. Check the working path against the minimal regression target as
stages become available. Record platform feasibility findings early and verify
supported capabilities on Windows and Linux as implementation proceeds.
