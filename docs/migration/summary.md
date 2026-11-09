# Migration summary

## Objective

Build a Rust successor to `mgb_ops` using its existing data flow and the additional
operational capabilities found in the parallel VB.NET/Python system. Preserve
both implementations as references in the same Git repository.
The Python reference is checked out at `/workspace/legacy_python/` on
`legacy/python`; the curated VB.NET reference is at `/workspace/legacy_vbnet/`
on `legacy/vbnet`. Rust development starts on the independent, clean `main`
worktree at `/workspace/mgb_ops`.

The application is CLI-first. A Windows/Linux desktop application uses the same
Rust capabilities and is introduced early, then extended with the core. All
intended functionality belongs in the Rust package. The external MGB executable
remains the numerical engine; `rs_hydro_forecast` is the target runtime workspace.

## Current migration state

Updated November 9, 2026. The RS hourly reference is established, and the
[first Rust foundation plan](../../.dev/plans/0001-rust-foundation.md) is agreed.
No Rust capabilities have been implemented. Next: write the foundation
implementation specification under `.dev/changes/`, then implement its read-only
workspace inspection CLI and development checks.

Use `not started`, `planned`, `in progress`, `blocked`, and `complete` for the
scope stated in each row. `Planned` means an agreed delivery plan exists, not
that code works. Completion requires acceptance evidence for that stated scope;
record unverified platforms and remaining gaps explicitly.

| Capability and reference behavior | Delivery step | State, evidence, and gaps |
| --- | --- | --- |
| [Frozen RS hourly reference](../../references/regression/rs-hourly/README.md) | 1: reference capture | Complete for saved normalized inputs through decoded outputs: four scenarios and exact fresh repeat. [Evidence](../reports/regression-reference-capture.md). Linux only; acquisition, Windows, and state qualification remain outside this case. |
| [Rust workspace inspection, shared library, and CLI foundation](core_and_cli.md) | 2: [foundation plan](../../.dev/plans/0001-rust-foundation.md) | Planned. Architecture, quality rules, and first CLI behavior agreed; specification and implementation pending. |
| [History, assets, and persisted operation data](core_and_cli.md) | 2/5: grow assets with each operational capability | Not started in Rust. Legacy stores are references; storage/import decisions follow the capabilities that need them. |
| [Saved RS preparation, real execution, and decoded results](model_and_hotstarts.md) | 3: first operational Rust path | Not started in Rust. Compare with the frozen reference; carry saved initial state unchanged. |
| [Acquisition and imports, including inventory updates](acquisition.md) | 5: add sources incrementally | Not started in Rust. Cover Python, VB.NET/plugins, and runtime scripts; exclude ONS meteorological forecasts. |
| [QC, review, corrections, and availability](review_and_corrections.md) | 5: extend assets | Not started in Rust. Preserve sources and make effective-data changes explicit. |
| [Additional forcing inputs and hot-start lifecycle](model_and_hotstarts.md) | 5: extend model support | Not started in Rust. State generation, consumption, and chaining still require qualification. |
| [Geometry and calibration editors](calibration_editors.md) | 6: functional acceptance | Not started in Rust. No editor regression or baseline replacement required. |
| [Numerical analysis and post-processing](analysis_and_reports.md) | Grow assets with operational increments; complete in 7 | Not started in Rust. Includes comparisons, accumulations, peaks, and reference post-processing. |
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
    -> normalize and register history/assets
    -> inspect, validate, and correct data
    -> prepare rainfall, observed flows, reservoir releases, wind, and model state
    -> execute forecast scenarios
    -> inspect and post-process results
    -> produce bulletins and data exports
    -> archive and share the completed run
```

Calibration editors support geometry, channel/Manning parameters, and soil
parameters. Scheduling and email automate the same operational capabilities.

## Scope

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

The responsibility files below are broad feature roadmaps, not required Rust
modules or detailed specifications. The [foundation plan](../../.dev/plans/0001-rust-foundation.md)
establishes one Cargo package with deep `workspace`, `acquisition`, `assets`, and
`model` modules. Dependency direction is `model -> assets -> acquisition`;
review and numerical analysis belong in assets. Use the roadmaps to find feature
coverage and references as development proceeds.

Maintain a short functionality checklist linking each intended capability to its
reference behavior, responsible implementation step, status, and completion
evidence. Include runtime scripts and bundled plugins. Leave legacy tools that
cannot be mapped to source behavior aside; they do not block implementation.

Each increment establishes its design target, specification, implementation,
checks, and review, then records completion evidence and remaining gaps. Settle
detailed behavior and technology choices within that increment. Extend the
minimal regression target where relevant and keep the established operational
path working; exhaustive edge-case coverage is not an initial planning gate.
Calibration/geometry editors use functional acceptance, with no regression
requirement.

| File | Responsibility |
| --- | --- |
| [Core and CLI](core_and_cli.md) | Workspaces, configuration, persistence foundations, and operation commands. |
| [Acquisition](acquisition.md) | Provider access, file imports, and normalization for assets to accept and register. |
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
2. Establish the [Rust foundation](../../.dev/plans/0001-rust-foundation.md): a
   shared library, read-only workspace inspection CLI, documented contracts, and
   enforced architecture/quality rules. Validate on a disposable RS workspace on
   Linux; defer Windows checks. Add persistence as operational capabilities need
   it rather than creating the entire storage design upfront.
3. Reproduce the saved preparation, real MGB execution, and decoded-results path
   in Rust. Compare with the regression target, carrying the required initial
   state unchanged; hot-start generation and chaining follow later.
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
