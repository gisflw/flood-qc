# Migration summary

## Objective

Build a Rust successor to `mgb_ops` using its existing data flow and the additional
operational capabilities found in the parallel VB.NET/Python system. Preserve
both implementations as references in the same Git repository.
The Python reference is checked out at `/workspace/legacy_python/` on
`legacy/python`; the curated VB.NET reference is at `/workspace/legacy_vbnet/`
on `legacy/vbnet`. Rust development uses the independent `main` worktree at
`/workspace/mgb_ops`.

The application is CLI-first. A Windows/Linux desktop application uses the same
Rust capabilities and is introduced early, then extended with the core. All
intended functionality belongs in the Rust package. The external MGB executable
remains the numerical engine; `rs_hydro_forecast` is the target runtime workspace.

## Current migration state

Updated November 16, 2026 for shared operational capabilities.
The Linux saved-data operational path is complete:
workspace inspection, canonical observations/QC/replacements, saved forecast
normalization/corrections, reusable prepared inputs, isolated real MGB execution,
and raw/station integer-centimetre results through shared library/CLI workflows.
Full saved scientific arrays and engine streams match exactly, including a repeat
from retained inputs after the disposable original sources were removed. The
[qualification record](../../references/regression/rs-hourly/rust-qualification.md)
retains source/driver identities, comparison evidence and numerical caveats.

Shared library and CLI capabilities include typed preparation, drafts and
previews, selective registration, controlled trials, geometry and reference
imports, rainfall summaries, series queries and CSV exports. Library callers can also read
and atomically save workspace configuration, initialize drafts from selected
configuration, export representable operation TOML, discover retained rainfall
grids, and resolve signed rainfall hours through shared interval validation.
The last explicit frozen-case release RS acceptance
passed in 82.76 seconds after coordinate-independent review was separated from
interpolation requirements. Failure checks cover OS
lock contention, nonzero/spawn/malformed output, surviving siblings, unavailable
original sources, altered prepared inputs and failed final manifest persistence.
Linux is qualified for this saved case; Windows and hosted CI remain unverified.
The saved initial state's hydrological and temporal compatibility is unqualified.

Current usage lives in the [workspace](../../docs/contracts/workspace.md),
[observation](../../docs/contracts/observations.md),
[forecast](../../docs/contracts/forecasts.md) and
[operation](../../docs/contracts/operations.md) contracts. The accepted operational
specifications are retired. Live acquisition, broader review/analysis and new hot
starts follow their migration rows below.

Use `not started`, `planned`, `in progress`, `blocked`, and `complete` for the
scope stated in each row. `Planned` means an agreed delivery plan exists, not
that code works. Completion requires acceptance evidence for that stated scope;
record unverified platforms and remaining gaps explicitly.

| Capability and reference behavior | Delivery step | State, evidence, and gaps |
| --- | --- | --- |
| [Frozen RS hourly reference](../../references/regression/rs-hourly/README.md) | 1: consolidated reference capture | Complete: canonical fixtures and staged expectations for four scenarios, exact fresh repeat, direct source normalization equality and shared history-only level recovery. Linux only; acquisition, Windows, state qualification and Rust equivalence remain outside this evidence. |
| [Rust workspace inspection, shared library, and CLI foundation](core_and_cli.md) | 2: read-only foundation | Complete for Linux inspection: strict TOML, path semantics, text/JSON findings, source preservation and architecture checks pass. Hosted CI and Windows remain unverified. Numerical execution acceptance is separate below. |
| [History, assets, and persisted operation data](core_and_cli.md) | 3 onward: grow store with each operational capability | Complete for the saved path: canonical observation and forecast/correction SQLite, forcing/result NetCDF, frozen prepared dependencies and per-run JSON. Legacy conversion remains regression-only; broader imports follow acquisition. |
| [Canonical observations, QC, and observed forcing](../../docs/contracts/observations.md) | 3, step 2a: observed forcing | Complete: canonical SQLite, explicit ordered replacements, ranking/QC and pure interpolation; full saved arrays, selected rows and findings exact. Timestamp fixture conversion and numerical provenance retained in the qualification record. |
| [Saved forecast normalization and forecast forcing](../../docs/contracts/forecasts.md) | 3, step 2b: forecast forcing | Complete: qualified driver and metadata/grid validation, immutable catalog, ordered corrections and library/CLI workflow. Full native, working, basin and transformed-frame comparisons exact; masks, coordinates and decoded local times/bounds verified. |
| [Saved RS integration, execution, and decoded results](../../docs/contracts/operations.md) | 3, integration and step 3: first operational Rust path | Complete on Linux: reusable frozen preparation, OS lock, isolated scenarios, history-only level recovery, raw/station integer centimetres and per-trial manifests. Two saved executions match full arrays and raw streams exactly after original sources are removed; lifecycle/failure and integrity checks pass. State generation, Windows and hydrological validity remain unqualified. |
| [Acquisition and imports, including inventory updates](acquisition.md) | 5: add sources incrementally | Not started in Rust. Cover Python, VB.NET/plugins, and runtime scripts; exclude ONS meteorological forecasts. |
| [Broader QC, review, corrections, and availability](review_and_corrections.md) | 5: extend analysis beyond the saved path | Not started beyond the saved-case capabilities implemented above. Analysis operates on canonical data; store persists explicit review instructions and preserved sources. |
| [Additional forcing inputs and hot-start lifecycle](model_and_hotstarts.md) | 5: extend model support | Not started in Rust. State generation, consumption, and chaining still require qualification. |
| [Geometry and calibration editors](calibration_editors.md) | 6: functional acceptance | Not started in Rust. No editor regression or baseline replacement required. |
| [Numerical analysis and post-processing](analysis_and_reports.md) | Grow analysis with operational increments; complete in 7 | Saved-path forcing is implemented above; broader analysis remains not started. Observation-based AR(1) belongs to analysis for dataviz and is deferred. No model-internal dependencies. |
| [Bulletins and document/data exports](analysis_and_reports.md) | Exercise useful exports early; complete in 7 | Not started in Rust. Both bulletin families and runtime report scripts remain references. |
| [Completed-run regeneration and sharing](completed_runs.md) | 7: completed-run sharing | Not started in Rust. Operators may copy generated files; regeneration requires referenced dependencies and a compatible engine. No retention/archive manager in the saved-run increment. |
| [Desktop interface](desktop_and_automation.md) | 4, then alongside subsequent capabilities | Planned. The [GUI specification](../plans/desktop_gui.md) defines RustGIS-based monitoring, project configuration, preparation/execution and assistant behavior. QC editors and forecast comparison require separate planning; shared headless prerequisites are available. |
| [Scheduling and email](desktop_and_automation.md) | 7: automate established operations | Not started in Rust. Follow the capabilities they automate. |

Split these broad rows as individual capabilities are implemented, preserving
their reference links, delivery responsibility, actual state, evidence, and next
work or gaps. Include runtime scripts and bundled plugins. Update the checklist
when implementation, verification, blockers, or the next increment changes, as
required by [AGENTS.md](../../AGENTS.md).

Linux is the first validation platform. Windows remains a build target, with
checks deferred until later development; no Windows validation infrastructure is
a foundation gate. Engine prerequisite inspection is not proof of successful
execution. Reference repeatability is not Rust equivalence or hydrological
validation.

## Operational flow

```text
Acquire observations and forecasts
    -> normalize canonical data and register selected artifacts through store
    -> inspect, validate, and correct data
    -> freeze selected data, corrections, settings, calibration, geometry, and state
    -> analysis prepares canonical forcing; model serializes engine inputs
    -> execute forecast scenarios and stage decoded canonical outputs
    -> record run/scenario outcomes and output locations in per-run JSON manifests
    -> inspect and post-process results
    -> produce bulletins and data exports
    -> optionally copy generated files for long-term storage; share completed runs later
```

App owns the operation and its status/failure policy; the
[architecture](../../docs/architecture.md) defines frozen working inputs and independent run outcomes.
Calibration editors support geometry, channel/Manning parameters, and soil parameters.
Scheduling and email automate the same operational capabilities.

## Scope

Lasting product commitments live in the
[architecture](../../docs/architecture.md#product-direction). The coverage below
tracks their migration from the reference systems.

Retain the implemented `mgb_ops` acquisition, QC, corrections, analysis, scenarios,
reporting, and regional-workspace behavior. Add the other system's acquisition
and operational features, including satellite rainfall, wind, reservoir inputs,
hot starts, calibration editors, richer post-processing, archive comparisons,
availability views, and relevant imports/exports.

Include behavior currently implemented in `rs_hydro_forecast` scripts, such as
station-inventory updates and bulletin generation, in the functionality to port.
The runtime workspace must not retain Python scripts as required implementations
of migrated capabilities.

Exclude ONS meteorological forecast products. Use both bulletin families as
complementary references; provide PDF, Markdown, HTML, and relevant CSV exports.

Use the existing executable and compatible inputs in `rs_hydro_forecast` for the
initial regression target, retaining its calibration, geometry, and required
initial state. Porting calibration/geometry editors does not require replacing
that baseline or establishing regression evidence for the editors. Generate
time-dependent inputs in Rust and reproduce the VB.NET hot-start lifecycle as
those capabilities are implemented.

Run artifacts record the recipe and references for regenerating inputs and outputs
when dependencies remain available; operators choose which generated files to copy.
Sharing concerns completed operations, not simultaneous editing. Every cache is
optional; no cache-packaging design is required at this stage.

## Delivery approach

The responsibility files below describe feature coverage, not Rust modules or
implementation details. Follow the [architecture](../../docs/architecture.md) and extend
the frozen regression case as capabilities arrive. Keep the working operational
path intact. Calibration/geometry editors use functional acceptance rather than
requiring replacement of the regression baseline.

Plan and specify active work under `.dev/`; implementation results may live under
`.dev/reports/`. Retire accepted plans/specifications, keep current evidence/gaps in
this checklist, and preserve unique scientific provenance with its reference.

| File | Responsibility |
| --- | --- |
| [Core and CLI](core_and_cli.md) | Workspaces, configuration, persistence foundations, and operation commands. |
| [Acquisition](acquisition.md) | Adapters acquire/normalize canonical data; app decides persistence through store. |
| [Review and corrections](review_and_corrections.md) | QC, manual observation changes, and rainfall transformations. |
| [Model and hot starts](model_and_hotstarts.md) | Input generation, state lifecycle, scenario execution, and output decoding. |
| [Calibration editors](calibration_editors.md) | Geometry, channel/Manning, and soil-parameter editing. |
| [Analysis and reports](analysis_and_reports.md) | Post-processing, comparisons, maps, bulletins, and exports. |
| [Completed runs](completed_runs.md) | Regeneration, archival, and sharing. |
| [Desktop and automation](desktop_and_automation.md) | Early Windows/Linux interface, incremental GUI coverage, scheduling, and email. |

## Suggested increments

1. Establish the [minimal reproducible regression target](initial_regression_tests.md)
   from `rs_hydro_forecast`, covering the main operational steps, and the
   functionality checklist. This captures reference evidence before Rust
   replacement; it does not require implementing the Rust application first.
2. Establish the [Rust foundation](../../docs/contracts/workspace.md): a
   shared library, read-only workspace inspection CLI, documented contracts, and
   enforced architecture/quality rules. Validate on a disposable RS workspace on
   Linux; defer Windows checks. Add persistence as operational capabilities need
   it rather than creating the entire storage design upfront.
3. Reproduce the saved preparation, real MGB execution, and decoded-results path
   in Rust. Compare with the regression target, carrying the required initial
   state unchanged. Generate execution-start input snapshots and per-run
   JSON manifests through shared app workflows/store; keep failure
   diagnostics and successful sibling outputs without changing other runs. Hot-start
   generation and chaining follow later.
4. Introduce a working GUI over that shared core. Perform the available operation
   through CLI and GUI with consistent results. Extend the GUI alongside every
   subsequent capability rather than waiting for the complete core.
5. Add acquisition, QC, corrections, remaining forcing inputs, and hot-start
   handling incrementally. Validate each addition, extend the regression target
   where relevant, and keep the established operational path working.
6. Port calibration/geometry editors. Define functional acceptance during their
   implementation; no regression target or baseline replacement is required.
7. Complete analysis, complementary bulletins, exports, completed-run sharing,
   scheduling, and email, updating the GUI throughout. Demonstrate the completed
   workflows on the supported platforms and account for the functionality
   checklist.

Keep a short record of implemented capabilities and the next increment; avoid a
detailed speculative backlog. Basic analysis or exports may be connected earlier
to exercise the working operation; the last increment completes their coverage.
