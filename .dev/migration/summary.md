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

Updated November 11, 2026. The RS hourly reference is established. The Rust
read-only inspection library/CLI is implemented with strict TOML, dedicated path
overrides, bounded text/JSON findings, and source-preservation checks. Disposable
RS acceptance passed previously on Linux. Current local qualification with Rust
1.99.1 passed formatting, Clippy, seven inspection tests, and two architecture
tests. The architecture checker now accepts same-owner re-exports and ordinary
text containing `!`, with regression fixtures. Hosted CI and Windows remain
unverified. Foundation acceptance remains in progress.

Next: complete foundation acceptance, then qualify the corrected RS reference and
implement the four operational specifications linked below. Carry the saved
state unchanged and implement the [execution lifecycle](../../docs/architecture.md#execution-lifecycle).
Supported behavior lives in the [workspace contract](../../docs/contracts/workspace.md).
The operational specifications replace the saved RS operational plan; their
capabilities remain unimplemented. Canonical storage, operation timezones, and
level-reference ownership are agreed [design decisions](../../docs/architecture.md#operational-data-decisions).

Use `not started`, `planned`, `in progress`, `blocked`, and `complete` for the
scope stated in each row. `Planned` means an agreed delivery plan exists, not
that code works. Completion requires acceptance evidence for that stated scope;
record unverified platforms and remaining gaps explicitly.

| Capability and reference behavior | Delivery step | State, evidence, and gaps |
| --- | --- | --- |
| [Frozen RS hourly reference](../../references/regression/rs-hourly/README.md) | 1: reference capture | Complete for saved normalized inputs through decoded outputs: four scenarios and exact fresh repeat. [Case and evidence](../../references/regression/rs-hourly/README.md). Linux only; acquisition, Windows, and state qualification remain outside this case. |
| [Rust workspace inspection, shared library, and CLI foundation](core_and_cli.md) | 2: read-only foundation | In progress: bounded inspection and Linux checks pass; architecture checker regressions pass locally; hosted CI remains unverified. Rust uses TOML without YAML fallback/conversion. Windows, runtime compatibility, execution, and content/numerical validity unverified. |
| [History, assets, and persisted operation data](core_and_cli.md) | 3 onward: grow store with each operational capability | Planned for the saved path; not implemented. Rust-owned SQLite/NetCDF contracts, a separate forecast/correction SQLite, and station-offset records follow the operational specifications. Regression-only conversion scripts may supply fixtures. |
| [Corrected RS reference and canonical acceptance inputs](../changes/rs-reference-alignment.md) | 3, step 1: qualify reference/contracts | Planned; no revised reference accepted yet. Resolve alignment/output interpretation using existing legacy evidence, preserve revision 1, and qualify all four scenarios with fresh repetition. |
| [Canonical observations, QC, and observed forcing](../changes/rs-observed-forcing.md) | 3, step 2a: observed forcing | Planned; not implemented. Canonical saved history, six curated NULL replacements, independent detection/interpolation acceptance, and station-offset persistence. |
| [Saved forecast normalization and forecast forcing](../changes/rs-forecast-forcing.md) | 3, step 2b: forecast forcing | Planned; not implemented. Adapters normalize saved ECMWF/NOAA files; store persists native data/catalog/corrections; analysis prepares forcing. Qualify native Terrio I/O independently of scientific processing. |
| [Saved RS integration, execution, and decoded results](../changes/rs-model-execution.md) | 3, integration and step 3: first operational Rust path | Planned; not implemented. App freezes inputs, coordinates isolated scenarios, and publishes complete sets through store. Model retains raw basin levels and produces station-referenced levels. Carry saved state unchanged; combined run-all commands deferred. |
| [Acquisition and imports, including inventory updates](acquisition.md) | 5: add sources incrementally | Not started in Rust. Cover Python, VB.NET/plugins, and runtime scripts; exclude ONS meteorological forecasts. |
| [Broader QC, review, corrections, and availability](review_and_corrections.md) | 5: extend analysis beyond the saved path | Not started beyond the saved-case capabilities planned above. Analysis operates on canonical data; store persists explicit review instructions and preserved sources. |
| [Additional forcing inputs and hot-start lifecycle](model_and_hotstarts.md) | 5: extend model support | Not started in Rust. State generation, consumption, and chaining still require qualification. |
| [Geometry and calibration editors](calibration_editors.md) | 6: functional acceptance | Not started in Rust. No editor regression or baseline replacement required. |
| [Numerical analysis and post-processing](analysis_and_reports.md) | Grow analysis with operational increments; complete in 7 | Saved-path forcing is planned above; broader analysis remains not started. Observation-based AR(1) belongs to analysis for dataviz and is deferred. No model-internal dependencies. |
| [Bulletins and document/data exports](analysis_and_reports.md) | Exercise useful exports early; complete in 7 | Not started in Rust. Both bulletin families and runtime report scripts remain references. |
| [Completed-run regeneration and sharing](completed_runs.md) | 7: complete archival workflow | Not started in Rust. Regeneration must work without provider access or caches. |
| [Desktop interface](desktop_and_automation.md) | 4, then alongside subsequent capabilities | Not started. Uses shared Rust capabilities; framework remains undecided. |
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
    -> publish the complete scenario set through store
    -> inspect and post-process results
    -> produce bulletins and data exports
    -> archive and share the completed run
```

App owns the operation and its status/failure policy; the
[architecture](../../docs/architecture.md) defines snapshots and complete-set publication.
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

Completed runs must contain enough information to regenerate inputs and outputs.
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
   state unchanged. Implement execution-start input snapshots and complete-set
   publication through shared app workflows/store; keep failed-attempt evidence
   and previous published results. Hot-start generation and chaining follow later.
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
