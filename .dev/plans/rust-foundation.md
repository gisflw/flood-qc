# Rust foundation

Status: agreed; implementation not started. Architecture and lifecycle decisions
revised November 10, 2026.

Detailed behavior is now specified in the
[foundation implementation specification](../changes/rust-foundation.md).
Use it for command syntax, TOML configuration, dedicated overrides, the required
inspection file list, structured findings, development checks, and acceptance.
This plan retains the agreed scope and architectural rationale; specification
completion does not complete the foundation.

## Outcome

Establish the Rust application's documentation, module boundaries, development
checks, and first usable CLI capability: inspect an explicitly selected regional
workspace, read its configuration, check required files and engine prerequisites,
and report problems without modifying the workspace.

Use a disposable copy of `rs_hydro_forecast` as the first integration target.
Model execution and comparison against the frozen regression reference follow
in the next operational increment.

This plan records agreed behavior and boundaries. Detailed command syntax,
schemas, dependencies, and the architecture-check mechanism belong in an
implementation specification under [`.dev/changes/`](../changes/), now linked above.

## Starting state

- Rust implementation has not started; the repository contains documentation
  and compact reference evidence.
- The [RS hourly reference](../../references/regression/rs-hourly/README.md)
  contains four scenarios and a fresh repeat through decoded outputs. Exact
  repeat comparisons establish Linux reference behavior, not Rust equivalence.
- Acquisition, Windows operation, initial-state generation/consumption, and
  hydrological validity are not established by that reference.
- The [migration summary](../../docs/migration/summary.md) is the current-state
  record and broad roadmap. Its responsibility files describe feature coverage,
  not required Rust modules.

## Agreed architecture

Use one Cargo package with a shared library and thin CLI/desktop interfaces.
Introduce the desktop over the same library after the first
preparation/execution/decoding path works, as already agreed.

Prefer a few deep modules organized around responsibilities and information
hiding. Internal submodules may separate calculations and I/O without exposing
those implementation details to callers.

| Module | Responsibility and hidden details | Exclusions |
| --- | --- | --- |
| `store` | Canonical data contracts, schemas, metadata, persistence, queries, artifact registries, provenance, and file lifecycle. Database operations and storage implementations remain private behind intentional APIs. | Acquisition and scientific processing. |
| `adapters` | Provider acquisition, source parsing, and normalization into store's canonical representations. Return data; the caller decides durable persistence. | Operational persistence, QC, and analysis. |
| `analysis` | QC, reproducible corrections, transformations, interpolation, diagnostics, metrics, report content, and document/chart rendering through private submodules. | Acquisition, engine execution, and interactive GUI state. |
| `model` | Engine-specific input serialization, isolated execution, state mechanics, and decoding into staged canonical outputs. Accept explicit prepared data, paths, and parameters. | General-purpose QC/forcing calculations, registry discovery, reporting, and official publication. |
| `app` | Shared application workflows: configuration/path resolution, data selection, execution snapshots, preparation/scenario coordination, failure handling, scheduling, export, and publication policy. | Scientific algorithms, persistence implementations, and interface widgets. |

Allowed dependencies:

```text
app -> store, adapters, analysis, model
adapters -> store
analysis -> store
model -> store
store -> external libraries only
```

These arrows permit dependencies; they do not require storage access. There are
no direct imports between adapters, analysis, and model. Store never calls them
or app. Its canonical representations are usable without a SQLite connection or
filesystem state. Analysis and model can consume supplied data without registry
lookups; engine execution and staged file output still use explicit paths.

CLI and desktop call intentional public library APIs. Reusable operational
sequencing and failure policy belong in app, so CLI, desktop, and scheduled
execution use the same workflows. Keep those workflows focused on concrete
operations, with private helpers as needed. Configuration loading and workspace
path resolution are private app responsibilities; no separate top-level
workspace module is needed. Core modules receive resolved inputs and do not
discover the active workspace or global configuration.

For preparation, app supplies selected data and target geometry to analysis for
QC, corrections, temporal resampling, and spatial interpolation. It then passes
prepared canonical forcing to model for engine-specific serialization and
execution. Model decodes outputs into store's canonical representations. The
workflow calls analysis for downstream diagnostics and rendering, without
requiring analysis to know model internals. Interactive controls stay in the GUI;
app coordinates saving and publication through store.

SQLite and NetCDF are starting choices. Settle concrete schemas, encodings, and
file layouts with the operational capabilities that use them. A canonical value
does not have to be persisted before processing. Add actual modules and types
when implemented, without empty placeholders for the migration.

### Execution snapshots and publication

These requirements govern the next operational increment; the foundation
documents them without implementing execution or persistence:

- At execution startup, app freezes the selected observations, forecasts,
  corrections, settings, calibration, geometry, and initial state, and records
  executable identity. Store preserves actual values or immutable versions;
  saved selectors, mutable paths, or a hash without retained data are insufficient.
- Later acquisitions and edits cannot change that execution's inputs. Refreshing
  inputs creates a new execution snapshot. Reruns using a snapshot must not depend
  on live providers, mutable history, or disposable caches.
- Model produces staged canonical outputs. App owns execution status, scenario-set
  coordination, overlap policy, and failure handling. Store owns the persistence
  and file-lifecycle mechanisms used by those workflows.
- Publication requires the complete requested scenario set. App asks store to
  commit that set and its execution status/references consistently, exposing only
  complete, durable results. A failed attempt leaves the previous published set
  official and retains staged successful outputs and failure diagnostics for
  inspection. Model never publishes individual scenarios as official results.

Specify the concrete snapshot and publication mechanisms with the first
operational path. Completed-run archive formats and hot-start generation remain
later work.

### Enforcement and quality

- Use private implementation submodules and narrow intentional exports. Sibling
  modules use the owning module's interface rather than its internals.
- Enforce the allowed dependency direction automatically in the foundation.
  Check app and library-root bypasses as well as core modules; Rust privacy alone
  does not enforce the graph inside one package. The specification defines a
  bounded source checker and required positive/negative fixtures.
- Forbid unsafe code in owned library and binary targets; retain the rule for
  new owned targets. Dependencies may use unsafe internally.
- Require formatting, Clippy with warnings treated as errors for owned code,
  and relevant tests. Do not enable a blanket pedantic lint set.
- Expected checks are `cargo fmt --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`, and the architecture
  check. Record the actual commands once implemented.

## Documentation and contracts to establish

Keep durable architecture in `docs/architecture.md` and durable contracts under
`docs/contracts/`. Keep agreed delivery plans under `.dev/plans/`, implementation
specifications under `.dev/changes/`, and completed validation evidence under
`docs/reports/` or `references/`.

Document these boundaries to the depth the foundation requires, without
specifying the complete future application:

- Workspace selection, configuration ownership, path resolution, and required
  inputs, starting from the existing RS inspection target.
- Source ownership and read-only inspection, including database access that
  does not create databases or mutate sidecars.
- Store's canonical contracts and private storage implementations; adapters'
  normalization boundary; independent analysis/model APIs; and app's ownership of
  configuration, snapshots, and complete-set publication. Concrete data schemas
  and operational mechanisms follow when used.
- Explicit time conventions, units, interval bounds, and missing-value semantics
  wherever introduced. Do not silently strip timezones or equate missing data
  with zero. Regression conventions remain documented in its manifest.
- Structured library results and failures, useful CLI diagnostics, and an
  unsuccessful exit status when requested validation fails. Exact output shape
  and syntax follow in the specification.
- The distinction between checking prerequisites and demonstrating real engine
  execution or numerical equivalence.

Preserve useful legacy data and behavior while allowing deliberate format and
schema changes. Direct compatibility with every Python configuration/database
format is not a permanent requirement. When conversion is needed, provide an
explicit import/migration path rather than silently rewriting source data.
No migration command is required in this increment.

## First CLI capability

The shared library, exposed through the CLI, must:

1. Open the workspace supplied explicitly by the caller, independently of the
   process working directory or a hard-coded RS location.
2. Read and validate the configuration needed for inspection and resolve its
   required paths. Use the existing `config/custom.yaml` and RS layout as
   starting references; settle supported inspection inputs in the specification.
3. Check required files and engine prerequisites, with the exact bounded set of
   checks stated in the contract. Do not imply that this validates every stored
   dataset or proves that the engine can run.
4. Return structured findings and actionable errors through shared library
   behavior. The CLI handles arguments and terminal presentation.
5. Leave the workspace unchanged, including on failure. Do not create output or
   cache directories, bootstrap databases, acquire data, apply corrections,
   resolve live forecast sources, or launch the engine during inspection.

## Delivery and acceptance

1. Write the implementation specification using these agreed decisions.
   Establish durable architecture/contracts and development setup alongside the
   behavior they govern.
2. Implement the single-package library/CLI foundation and its enforced rules.
   Materialize only modules with actual foundation responsibilities.
3. Verify valid inspection and useful failures for invalid configuration,
   missing/unreadable required inputs, and unavailable engine prerequisites.
   Demonstrate that inspection does not alter the disposable workspace.
4. Exercise the CLI from outside the selected workspace using a disposable RS
   copy. Keep small fixture checks separate from this integration check; normal
   Rust checks must not require Python or live providers.
5. Run development checks on Linux, review the result, and record commands,
   evidence, known limits, and remaining work. Update the migration state.

Build for Linux and Windows as the product targets. Validate on Linux in this
increment; defer Windows checks until later development. Do not add Windows CI,
cross-compilation infrastructure, or engine qualification as foundation gates.
Record Windows as unverified until checks actually occur.

Foundation completion means a usable read-only CLI, shared library behavior,
documented boundaries, enforced architecture/quality rules, and Linux acceptance
evidence. Planning or documentation alone does not complete the foundation.

## Migration tracking

Keep the capability checklist directly in the migration summary. Each area
retains reference behavior, responsible delivery step, status, evidence, and next
work or known gaps. Split broad rows as capabilities are implemented, including
runtime scripts and bundled plugins; avoid a speculative detailed backlog.

Update the summary whenever implementation, verification, a blocker, or the next
increment changes. Record intentional departures from legacy behavior. Whether
a decision needs discussion depends on its consequences and the session's scope;
there is no blanket approval rule for all departures.

After this foundation, specify and implement the saved RS preparation, real MGB
execution, and decoded-results path against the frozen reference. Carry the
saved initial state unchanged; generating and chaining hot starts comes later.

## Outside this increment

No acquisition implementation, operational data migration, corrections, full
dataset validation, model execution/decoding, GUI, reports, scheduling, email,
or completed-run sharing. Their coverage remains in the broad roadmap.
Do not change frozen expectations or claim Rust equivalence here.

## Reference code inspected

Paths are relative to `../legacy_python/`; use the
[source map](../../references/python_map.md) to navigate the retained reference.
Older status prose is not evidence of current behavior.

- `src/mgb_ops/config/`: explicit runtime context and workspace configuration.
- `src/mgb_ops/assets/`: canonical grids, history, effective observations,
  operation snapshots, and results.
- `src/mgb_ops/adapters/` and `src/mgb_ops/workflows/{observed,forecast}.py`:
  provider acquisition and normalization.
- `src/mgb_ops/analysis/{observations,forecast,timeseries,precipitation}.py`:
  data selection and numerical products informing the analysis boundary.
- `src/mgb_ops/workflows/{scenarios,scenario_orchestrator,reporting}.py` and
  `src/mgb_ops/model/`: operation coordination and engine integration.
