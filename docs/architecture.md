# Architecture

Use one Rust Cargo package with a shared library and thin CLI/desktop interfaces.
The external MGB executable remains the numerical engine. Add owners only when
behavior exists; the foundation currently implements app and model.

## Product direction

The intended application targets Linux and Windows. Core operations remain
available through the CLI; desktop and scheduling use the same library workflows.
Operational capabilities belong in Rust, including station-inventory updates and
bulletin generation currently supplied by regional Python scripts. Regional data
and settings remain in independently selected workspaces.

The intended scope includes acquisition, QC/corrections, forcing preparation,
scenario execution and hot starts, geometry/calibration editors, analysis,
bulletins, completed-run sharing, scheduling, and email. ONS meteorological
forecast products are excluded; ONS hydraulic observations and reservoir schedules
are included. Both existing bulletin families inform report content, with PDF,
Markdown, HTML, and relevant CSV exports. Sharing concerns completed runs between
independent workspaces; live multi-operator editing is outside scope. These are
product commitments; the [workspace contract](contracts/workspace.md) describes
the currently implemented inspection capability.

## Ownership

| Owner | Responsibility | Allowed internal dependencies |
| --- | --- | --- |
| `store` | Canonical data, schemas, metadata, persistence, queries, provenance, file lifecycle | None |
| `adapters` | Provider acquisition, source parsing, normalization; return data without deciding persistence | `store` |
| `analysis` | QC, corrections, transformations, forcing, diagnostics, metrics, report content/rendering | `store` |
| `model` | Engine prerequisites, input serialization, execution, state mechanics, staged decoding, fixed station-reference conversion | `store` |
| `app` | Workflows, private configuration/path resolution, selection, preparation/scenario coordination, failures, run outcomes | All four core owners |
| CLI/desktop | Arguments, invocation-directory capture, presentation, interactive state | Intended public library APIs |

These boundaries keep scientific processing independent of acquisition, registry
lookups, and execution coordination. Canonical values work without database
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

Define concrete schemas, units, interval bounds, and missing-value semantics when
first used. Do not equate missing values with zero or silently discard timezones.

## Operational data decisions

The next operational increment uses Rust-owned canonical formats. SQLite holds
observations, station metadata, and observation review instructions, without a
source/provenance table or source foreign keys. A separate
SQLite holds a two-table forecast asset/correction catalog. Each run generates
a JSON manifest containing its resolved recipe, ordered instruction values,
input references, scenario outcomes and output paths for CLI/desktop consumption.
There is no workspace-wide operations database or official-run pointer. NetCDF
holds native forecast grids, working grids, basin forcing/results, and station-level results.
Store owns these schemas and persistence; adapters return normalized canonical
data without choosing persistence. Concrete layouts grow with the capabilities.

Observation replacements preserve raw measurements and support a point or a
[start,end) period, with an absent end meaning indefinitely. NA replacements
express exclusions; ordered selected instructions apply with last match winning.
There is no separate review-set or station-exclusion entity. Forecast scenarios
select an asset and an explicit ordered list of correction IDs. Run artifacts
record the actual selected instruction values, not only mutable IDs.

Canonical timestamps use the operation's timezone, recorded in artifact metadata.
Forecast normalization converts source UTC timestamps into that convention while
preserving source cycle and interval provenance. This keeps canonical observation,
forcing, and result times aligned with the operation.

### Level reference and analysis corrections

Store persists fixed station-offset records alongside station metadata. Records
are keyed by station and identify the supplying model mini-basin, signed offset
in metres, and provenance. Different stations may share a mini-basin and have
different offsets. Missing offsets remain unknown; explicit zero is valid.

Model applies supplied fixed offsets as station level = engine level minus
offset. Canonical level observations and results use signed integer centimetres;
offsets retain their supplied precision in metres. Subtract from the unquantized
engine level before converting once to centimetres, using nearest rounding with
ties to even and checked integer range. Preserve original engine streams, raw
canonical levels by basin, and referenced levels by station. App supplies and
snapshots the offset records; model does not query catalogs or estimate offsets
from recent observations.

Observation-based AR(1) correction of flow/level belongs to analysis for dataviz.
Its implementation is deferred to that later capability. Fixed reference
conversion belongs to the initial model pipeline; the two operations remain
distinct, with raw and referenced model results available to later analysis.

## Enforcement

Rust privacy protects owner internals. Cargo's `unsafe_code = "forbid"` lint
applies to owned targets; dependencies may use unsafe internally.
The [architecture test](../tests/architecture.rs) enforces these source boundaries:

- Every Rust source under `src/` belongs to a declared owner, the root facade, or
  an interface (`main.rs` or `bin/`). Conditional code is checked too.
- Cross-owner references use explicit `crate::<owner>` paths and obey the table
  above. Relative paths stay within their owner; cross-owner glob imports are
  forbidden. Interfaces use intended public `mgb_ops` APIs.
- The root facade contains only owner module declarations and explicit public
  re-exports. It contains no workflow code or wildcard re-exports.
- Inside an owner, public re-exports expose items from its own private submodules
  through `self::<private_module>::<item>`. Imported foreign items cannot be
  republished to disguise a forbidden dependency.
- Paths in supported macros and attributes are checked. Unsupported constructs
  and generated source are rejected rather than silently left unchecked.

Run `cargo test --locked --test architecture` independently or as part of
`cargo test --locked`.
GitHub Actions runs the formatting, lint, and test gates on Linux for pushes and
pull requests. The toolchain is pinned and qualified deliberately.
The gate checks bounded source syntax, not behavioral ownership or execution
lifecycle correctness; review and behavior tests must cover those responsibilities.
Checker allowlists, parser mechanics, supported syntax, and known limitations stay
beside the test. Extend its checks and fixtures when adding needed syntax, without
weakening the ownership rules to accommodate a bypass.

## Execution lifecycle

These requirements govern the next operational increment, not read-only inspection:

- At startup, app freezes actual selected observations, forecasts, corrections,
  settings, calibration, geometry, initial state, and station offsets, and records
  executable identity.
  Store retains values or immutable versions, not merely selectors, mutable paths,
  or hashes without retained data.
- Later acquisition/editing cannot alter an execution. Refreshing inputs creates a
  new snapshot. Regeneration from the recorded recipe requires referenced data
  and the compatible engine to remain available; hashes cannot recover missing bytes.
- Model stages canonical outputs. App owns execution status, overlap/failure policy,
  and scenario-set coordination; store owns persistence and file lifecycle.
- Each run has independent results and a generated JSON manifest
  for CLI/desktop use. App reports success only after every requested scenario and
  its required outputs validate. Failures preserve diagnostics and successful
  siblings' outputs without modifying other runs. No official-set publication or
  latest-run pointer is maintained.
- Operators may copy the recipe, dependencies, results or cache outputs elsewhere
  for long-term storage. Generation is normal application behavior; no retention
  flags, cleanup policy or archive-management workflow is added.

Specify concrete snapshot/run-artifact mechanisms with saved RS preparation,
execution, and decoding. Carry the saved state unchanged; hot-start generation
and chaining follow later.
