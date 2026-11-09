# Agent guidance

## Project status

This is the documentation-only Rust successor to MGB Ops; Rust implementation
has not started. Read [README.md](README.md) for the agreed direction and
[docs/migration/summary.md](docs/migration/summary.md) for the broad roadmap.
The agreed first increment is [the Rust foundation](.dev/plans/rust-foundation.md),
with a [detailed implementation specification](.dev/changes/rust-foundation.md).
Specification is complete; implementation is pending and requires a subsequent
implementation task.
Build shared Rust capabilities with a CLI first and a desktop introduced early;
the external MGB executable remains the numerical engine.

## Repository structure

- `README.md`: project entry point and agreed direction.
- `docs/architecture.md` and `docs/contracts/`: durable architecture and contracts
  to establish with the foundation.
- `docs/migration/`: broad migration roadmap and responsibility areas.
- `docs/reports/`: completed work, reviews, and validation evidence.
- `references/`: legacy source maps, provenance, and compact regression evidence.
- `.dev/`: disposable working documentation, not long-lived project records.
  Use descriptive filenames without numbering; preserve durable results in
  `docs/` or `references/` when work is complete.
- `.dev/plans/`: general plans for agreed behavior awaiting implementation.
- `.dev/changes/`: detailed specifications ready for implementation.

Keep the broad migration roadmap in `docs/migration/`. When retiring temporary
implementation documentation, retain durable results in `docs/reports/` or
`references/`. Resolve documentation links relative to their containing file.

## Migration state

Keep the capability checklist directly in
[the migration summary](docs/migration/summary.md). Update it when implementation,
verification, blockers, or the next increment changes. Each entry links reference
behavior, its delivery step, current status, and acceptance evidence or remaining
gaps. Split broad entries as features are implemented, including runtime scripts
and bundled plugins; avoid a speculative detailed backlog.

Record acceptance checks, evidence, remaining gaps, and intentional departures
from legacy behavior for each increment. A plan or reference capture is not an
implemented Rust capability. Keep platform evidence explicit: Linux is checked
first, Windows checks come later. Never imply that prerequisite inspection proves
engine execution, numerical equivalence, or hydrological validity. Keep the
project-status paragraph and README consistent when implementation starts.

## Rust architecture and checks

Use one Cargo package with a shared library and thin interfaces; do not split
this application into crates. Prefer a few deep modules with private internals
and narrow interfaces, organized around ownership and information hiding:

- `workspace`: configuration loading, workspace conventions, and path resolution.
- `acquisition`: provider calls, source restructuring, and format conversion into
  normalized input types defined at this boundary.
- `assets`: data coordination, review, numerical analysis, persistence, saved
  operation data, and published-result references. It accepts acquisition outputs
  directly and may request data from acquisition.
- `model`: engine input preparation, isolated execution, decoding, and state
  mechanics, using operation snapshots and data from assets.

Dependency direction is `model -> assets -> acquisition`; CLI/desktop use the
library and workspace resolution. Pass resolved inputs explicitly rather than
discovering global workspace/configuration inside domain modules. Acquisition
must not depend on assets, and assets must not depend on model. Keep provider
protocols and source conversion inside acquisition. Do not reproduce the Python
package's procedural divisions as mandatory top-level Rust modules or create
empty modules for future capabilities.

Enforce internal access through Rust privacy and check the allowed dependency
direction automatically. The foundation specification chooses the mechanism.
Forbid unsafe code in owned library/binary targets; dependencies may use unsafe.
Require formatting, Clippy with warnings treated as errors for owned code, the
architecture check, and relevant tests. No blanket pedantic lint set is required.
Windows checks are deferred; preserve Windows as a build target without making
Windows validation infrastructure a foundation gate.

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
