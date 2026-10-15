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

## Implementation direction

Keep the existing history/assets/review/scenario flow. Separate provider access,
artifact formats, persistence, calculations, model execution, and interface code.
Pass workspace, settings, and times explicitly into core operations. Keep raw
source data immutable and corrections explicit. The desktop must not become the
only way to perform a core operation.

Use the existing SQLite and external-file contracts as starting references;
settle necessary schema changes as their features are implemented.

## Evidence of completion

The CLI can resolve a regional workspace and execute the implemented stages with
useful results and failure reporting. A focused fixture verifies the first full
path through the shared core. Both Windows and Linux are supported.
