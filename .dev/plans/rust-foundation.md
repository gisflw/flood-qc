# Rust foundation

Status: agreed; implementation not started. Decisions agreed November 9, 2026.

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

Use one Cargo package with a shared library and thin CLI. Do not split this
application into crates. Introduce the desktop over the same library after the
first preparation/execution/decoding path works, as already agreed.

Prefer a few deep modules organized around responsibilities and information
hiding. Internal submodules may separate calculations and I/O without exposing
those implementation details to callers.

| Module | Responsibility and hidden details |
| --- | --- |
| `workspace` | Open an explicitly selected regional workspace, load configuration, resolve paths, and translate workspace conventions into explicit inputs. |
| `acquisition` | Call providers and import source files; hide protocols, source restructuring, and format conversion behind recognized normalized input data. |
| `assets` | Coordinate operational data: selection, validation, review, corrections, numerical analysis, derivation, persistence, registration, and caches. Own saved operation data and published-result references. |
| `model` | Prepare and execute an operation snapshot and produce canonical results. Hide MGB serialization, private execution directories, process handling, raw-output decoding, and hot-start mechanics. |

Allowed dependencies:

```text
CLI / desktop -> workspace, assets, model
model -> assets -> acquisition
```

Workspace resolution supplies explicit inputs; domain modules do not discover
the active workspace or read global configuration themselves. Interfaces may
compose these capabilities through small library entry points. They do not own
operational rules.

Acquisition defines its normalized output types; assets accepts those types
directly. Acquisition does not depend on assets or know its storage. Keep
provider response parsing and conversion out of assets. Assets may request data
from acquisition, avoiding a separate coordination layer for that sequence.

Assets owns selected sources, correction instructions, scenario definitions,
and published-result references. Model executes that saved snapshot and uses
asset APIs for canonical data and result persistence. Assets must not call back
into model. Do not introduce a separate operations module unless it acquires
substantial responsibilities beyond forwarding and coordination.

Review and numerical analysis are asset capabilities, not mandatory top-level
modules. Report rendering may warrant its own boundary later; its placement is
not settled here. Add actual modules and types when their behavior is implemented,
without empty placeholders for the whole migration.

### Enforcement and quality

- Use private implementation submodules and narrow intentional exports. Sibling
  modules use the owning module's interface rather than its internals.
- Add an automated check of the allowed dependency direction. Rust privacy alone
  does not enforce it inside one package. Choose the smallest adequate mechanism
  in the implementation specification.
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
- Acquisition's normalized-data boundary and assets' ownership of operational
  meaning and storage. Concrete acquisition payload schemas follow when used.
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
  data selection and numerical products informing the assets boundary.
- `src/mgb_ops/workflows/{scenarios,scenario_orchestrator,reporting}.py` and
  `src/mgb_ops/model/`: operation coordination and engine integration.
