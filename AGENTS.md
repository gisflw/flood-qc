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

- `store`: canonical data contracts, schemas, metadata, persistence, queries,
  artifact registration, provenance, and file lifecycle. Hide database/storage
  implementations behind intentional APIs; no acquisition or scientific processing.
- `adapters`: provider calls, source parsing, and normalization into store's
  canonical representations. Return data; callers decide durable persistence.
- `analysis`: QC, corrections, transformations, forcing calculations, diagnostics,
  metrics, report content, and rendering through private submodules. GUI widgets
  and interactive state stay in interfaces.
- `model`: engine-specific input serialization, isolated execution, state
  mechanics, and decoding into staged canonical outputs from explicit prepared
  data, paths, and parameters. No general-purpose QC, registry discovery, or
  official publication.
- `app`: shared library workflows, including configuration/path resolution,
  selection, snapshots, preparation/scenario coordination, failures, scheduling,
  export, and publication policy. Keep scientific algorithms and persistence
  implementations in their owning core modules.

Adapters, analysis, and model may depend on store, never on each other. Store
depends only on external libraries. App may depend on all four core modules;
core modules must not depend on app or interface code. CLI/desktop use intended
public library APIs and share app's operational workflows. Configuration and
path helpers stay private inside app; no top-level workspace module is needed.
Pass resolved inputs explicitly rather than discovering global configuration.

Store's canonical representations must be usable without database connections
or filesystem state. Analysis and model can process supplied data without
registry access. App calls analysis to prepare canonical forcing, then model to
serialize, execute, and decode it. SQLite and NetCDF are starting choices; settle
exact schemas and file layouts with their operational increments. Create only
modules with implemented behavior; foundation needs app and model.

For the next operational increment, freeze actual selected input values or
immutable versions at execution startup and record executable identity. Later
acquisitions/edits cannot alter that execution; refreshing inputs creates a new
snapshot. App owns execution status, overlap/failure policy, and scenario-set
coordination. Model stages outputs; app asks store to commit only a complete
requested result set and consistent status/references. A failed attempt retains
staged outputs/diagnostics and leaves the previous published set official.
Snapshots must not depend on mutable history, live providers, or disposable
caches. These are future operational requirements, not foundation implementation.

Enforce internal access through Rust privacy and check the allowed dependency
direction automatically in the foundation. Follow the specification's bounded
source checker, covering app, the root facade, and interfaces as well as core
owners. Include bypass and path-bearing derive-attribute fixtures. Keep root
helpers/re-exports from bypassing ownership; root composition belongs in app.
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
