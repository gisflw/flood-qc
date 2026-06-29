# Agent Guidance

This repository is moving toward a library-first architecture. Treat `src/mgb_ops`
as the primary product: an installable Python package whose modules and functions
can be used directly from notebooks, scripts, and data-flow style orchestration.

## Architecture Direction

- Keep the Python library as the core interface. Prefer reusable functions with
  explicit path, database, settings, and time inputs over behavior hidden behind
  command-line parsing or UI state.
- Keep the GUI layer thin. `apps/ops_dashboard/` handles Panel rendering,
  callbacks, caching, and session state, then calls reusable library functions.
- Do not add Panel or UI-session dependencies to core library modules.
- Avoid coupling notebook-friendly functions to `argparse`, Panel state,
  subprocess launch behavior, or print-only results. Put interface-specific
  behavior in a CLI/app wrapper; model subprocess execution itself belongs in
  `mgb_ops.model` behind an explicit, structured API.

## Module Boundaries

| Module | Owns | Must not own |
| --- | --- | --- |
| `mgb_ops.common` | Shared domain types, settings, paths, runtime conveniences, logging, and time utilities | Provider logic, persistence workflows, or UI state |
| `mgb_ops.assets` | External-file format contracts, validation, serialization, and loading, including packaged schemas | Database registration, provider fetching, orchestration, or analytical interpretation |
| `mgb_ops.adapters` | Provider-specific network access and translation into canonical library artifacts | Operational orchestration, UI behavior, or database ownership |
| `mgb_ops.storage` | SQLite bootstrap, repositories, persistence, and asset registration/resolution | Provider fetching, numerical analysis, or UI behavior |
| `mgb_ops.analysis` | Reusable read-only queries, projections, and numerical computations | Writes, provider fetching, UI state, or file-format ownership |
| `mgb_ops.edit` | Explicit domain corrections and their persistence | UI draft state or rendering |
| `mgb_ops.qc` | Validation rules, checks, and structured QC results | UI decisions or workflow orchestration |
| `mgb_ops.model` | MGB input preparation, execution, and output production | Provider fetching or UI behavior |
| `mgb_ops.workflows` | Use-case orchestration across adapters, storage, assets, QC, and model functions | UI state or provider/file-format implementations |
| `apps/ops_dashboard` | Panel rendering, session state, UI caching, callbacks, and presentation-only transformations | Core domain rules or reusable persistence logic |

`mgb_ops.assets` describes how an external artifact is shaped and read or
written. `mgb_ops.storage` describes how artifacts and records are catalogued
and persisted. `mgb_ops.analysis` derives read-only information from explicit
inputs. For example, GeoPackage layer loading belongs in `assets`, while grid
interpolation and rainfall accumulation belong in `analysis`.

Dependencies should point toward narrower capabilities: apps call workflows or
library modules; workflows compose domain modules; adapters, storage, analysis,
edit, QC, and model may use common types and asset contracts. Keep provider
implementations out of storage and assets, and keep app modules out of the
library. Avoid circular dependencies between peer modules; when a shared
contract is needed, place it in `common` or `assets` according to whether it is
a domain type or an external-file contract.

The existing `analysis.forecast` and `analysis.timeseries` modules may combine
explicit read-only loading with computation. Treat that as the current boundary:
all writes and registry ownership still belong to `storage`.

## Library API Conventions

- Require explicit paths, database handles or paths, settings, and reference
  times at domain boundaries.
- Return structured summaries, domain objects, or data frames rather than
  relying on console output.
- Only `mgb_ops.common` runtime helpers may resolve workspace defaults, `.env`,
  or process-environment convenience values. Pass resolved values onward.
- Keep raw provider artifacts immutable and make derived assets and edits
  explicit and auditable.
