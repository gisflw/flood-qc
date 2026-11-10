# Architecture

Use one Rust Cargo package with a shared library and thin CLI/desktop interfaces.
The external MGB executable remains the numerical engine. Add owners only when
behavior exists; the foundation currently implements app and model.

| Owner | Responsibility | Allowed internal dependencies |
| --- | --- | --- |
| `store` | Canonical data, schemas, metadata, persistence, queries, artifact registration, provenance, file lifecycle | None |
| `adapters` | Provider acquisition, source parsing, normalization; return data without deciding persistence | `store` |
| `analysis` | QC, corrections, transformations, forcing, diagnostics, metrics, report content/rendering | `store` |
| `model` | Engine prerequisites, input serialization, execution, state mechanics, staged decoding | `store` |
| `app` | Workflows, private configuration/path resolution, selection, preparation/scenario coordination, failures, publication policy | All four core owners |
| CLI/desktop | Arguments, invocation-directory capture, presentation, interactive state | Intended public library APIs |

These boundaries keep scientific processing independent of acquisition, registry
lookups, and operational publication. Canonical values work without database
connections or filesystem state. Adapters, analysis, and model do not depend on
each other. App supplies selected data to analysis for canonical forcing, then to
model for serialization, execution, and staged decoding. App also coordinates
analysis for downstream diagnostics and rendering.

Keep storage implementations private in store, rendering private in analysis,
and interactive state in interfaces. Configuration/path helpers stay private in
app; core modules receive explicit resolved inputs. Model never imports app's
configuration/report types. App assembles inspection findings from model's explicit
prerequisite checks. The root facade declares modules and re-exports intended APIs;
composition lives in app. Prefer deep modules with narrow APIs and private internals.

SQLite and NetCDF are starting choices. Define concrete schemas, time conventions,
units, interval bounds, and missing-value semantics when first used. Do not equate
missing values with zero or silently discard timezones.

Rust privacy protects internals. Cross-owner references name `crate::<owner>`;
interfaces use public `mgb_ops` paths. Root aliases and re-exports must not bypass
ownership. The architecture test enforces the graph; checker mechanics and supported
syntax belong beside that test, not in the architecture contract. Owned Cargo
targets forbid unsafe code; dependencies may use unsafe internally.

## Execution lifecycle

These requirements govern the next operational increment, not read-only inspection:

- At startup, app freezes actual selected observations, forecasts, corrections,
  settings, calibration, geometry, and initial state, and records executable identity.
  Store retains values or immutable versions, not merely selectors, mutable paths,
  or hashes without retained data.
- Later acquisition/editing cannot alter an execution. Refreshing inputs creates a
  new snapshot; reruns cannot require live providers, mutable history, or caches.
- Model stages canonical outputs. App owns execution status, overlap/failure policy,
  and scenario-set coordination; store owns persistence and file lifecycle.
- App asks store to publish only a complete, durable requested set with consistent
  status/references. Failure retains staged outputs/diagnostics and leaves the
  previous published set official. Model never publishes individual scenarios.

Specify concrete snapshot/publication mechanisms with saved RS preparation,
execution, and decoding. Carry the saved state unchanged; hot-start generation
and chaining follow later.
