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

These are work plans, not fixed crate boundaries or detailed specifications.
Use them to find the responsible area and its references as development proceeds.

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
| [Acquisition](acquisition.md) | Provider access, file imports, normalization, and registration. |
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
2. Establish Rust workspace/configuration/persistence support and basic CLI entry
   points. Check Windows/Linux feasibility, including the external engine, early.
   Verify operation on a disposable RS workspace without damaging source data.
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
