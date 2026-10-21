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
Rust capabilities. The external MGB executable remains the numerical engine.

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

Exclude ONS meteorological forecast products. Use both bulletin families as
complementary references; provide PDF, Markdown, HTML, and relevant CSV exports.

The initial executable comes from `rs_hydro_forecast/mgb_runner`. Calibration
and geometry start from the other system's `MGB/Input`. Generate time-dependent
inputs in Rust and reproduce the VB.NET hot-start lifecycle. Reference fixtures
may need to be regenerated using the existing systems.

Completed runs must contain enough information to regenerate inputs and outputs.
Sharing concerns completed operations, not simultaneous editing. Every cache is
optional; no cache-packaging design is required at this stage.

## Delivery approach

These are work plans, not fixed crate boundaries or detailed specifications.
Use them to find the responsible area and its references as development proceeds.

| File | Responsibility |
| --- | --- |
| [Core and CLI](core_and_cli.md) | Workspaces, configuration, persistence foundations, and operation commands. |
| [Acquisition](acquisition.md) | Provider access, file imports, normalization, and registration. |
| [Review and corrections](review_and_corrections.md) | QC, manual observation changes, and rainfall transformations. |
| [Model and hot starts](model_and_hotstarts.md) | Input generation, state lifecycle, scenario execution, and output decoding. |
| [Calibration editors](calibration_editors.md) | Geometry, channel/Manning, and soil-parameter editing. |
| [Analysis and reports](analysis_and_reports.md) | Post-processing, comparisons, maps, bulletins, and exports. |
| [Completed runs](completed_runs.md) | Regeneration, archival, and sharing. |
| [Desktop and automation](desktop_and_automation.md) | Windows/Linux interface, scheduling, and email. |

## Suggested increments

Establish the [initial regression tests](initial_regression_tests.md) before
replacing Python behavior. Start with the frozen regional case described there;
hot-start generation and chaining follow in a later implementation increment.

1. Preserve both references and finish their source/omission maps.
2. Establish Rust workspace/configuration/persistence support and CLI entry points.
3. Reproduce one saved case through input preparation, MGB execution, and output
   decoding, including the selected baseline and hot-start handling.
4. Connect live acquisition and review to that path; add remaining sources and
   forcing types incrementally.
5. Add calibration operations, post-processing, complementary bulletin content,
   and completed-run regeneration/sharing.
6. Expose working capabilities through the desktop interface; add scheduling and
   email over the same operations. Desktop work can follow completed core slices.
7. Validate the complete operation on Windows and Linux before the first release.

The final scope is broader than the first working slice. Keep a short record of
implemented capabilities and the next slice; avoid a detailed speculative backlog.
