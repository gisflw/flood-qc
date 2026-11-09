# Minimal reproducible regression target

## Purpose and scope

Establish this target as the first migration step, before replacing Python
behavior. Use the actual `rs_hydro_forecast` runtime workspace and its existing
compatible executable, calibration, geometry, and required initial state.
Capture reference evidence first; implement Rust checks against it as the shared
core becomes available.

The current [capture specification](../docs/changes/regression-reference-capture.md)
narrows the first milestone to saved normalized inputs through decoded results.
The November 9, 2026 [established case](../references/regression/rs-hourly/README.md)
includes all four scenarios and an exact fresh repeat, with a
[review report](../docs/reports/regression-reference-capture.md). Broader stages
in this migration outline remain incremental work.

Cover the main operational steps with a small representative case:

```text
Saved source observations and forecast
    -> normalization and history/asset registration
    -> review and corrections
    -> model input preparation and scenario execution
    -> decoded results
    -> available analysis, bulletin, and data exports
```

Use saved source artifacts so the target is reproducible without live providers.
Retain enough history and the model domain needed for a valid operation. Select
only the representative scenarios and corrections needed to exercise the main
path; an exhaustive scenario matrix or edge-case suite is not required now.

Extend the target as capabilities are implemented, including completed-run
regeneration/sharing when available. New features establish their expected
behavior during their own implementation step. Calibration/geometry editors
have functional acceptance and no regression requirement. Their port does not
require replacing the calibration or geometry used by the operational target.

## Establish the reference

1. Qualify one coherent saved RS operation using a disposable workspace copy.
   Identify the data window, settings, source assets, review instructions,
   scenarios, executable, and required model inputs. Confirm the existing path
   actually runs before accepting its results as expectations.
2. Capture representative checkpoints across the main steps, including prepared
   model inputs, real engine outputs, decoded results, and available report/data
   exports. Record what the legacy code generates and what it carries unchanged.
3. Record the reference commits, executable/input identities, capture commands,
   settings, and comparison expectations needed to rebuild the target. Use a
   small versioned manifest and instructions; keep large datasets and executable
   binaries outside Git. Check reproducibility from a fresh disposable copy.
4. Record any discovered legacy defects and agree on intended behavior before
   freezing the affected expectations. Preserve the reference inputs and make
   expectation updates an explicit capture/review operation.

Use the frozen Python and VB.NET references and the runtime scripts to locate
behavior. Leave tools that cannot be mapped to source behavior aside; they do
not block establishing this target. Historical executable metadata is available
in [model_baseline.json](../references/model_baseline.json); verify the actual
case rather than assuming that record proves compatibility.

## Use and improve the target

- Implement the comparisons in Rust using the shared core. Python may capture
  reference results, but routine Rust checks must not depend on Python.
- Use explicit saved inputs and an isolated scratch workspace. Never fall back
  to mutable operational data, live providers, or wall-clock data selection.
- Check stage identity, time/cell alignment, units, and representative results.
  Define numerical tolerances and detailed assertions during implementation;
  unexpected missing or non-finite values must not silently pass.
- Exercise real MGB execution explicitly, retaining useful logs and failed
  artifacts. Missing prerequisites or failed execution must report failure,
  rather than silently skipping a requested check.
- Keep lightweight checks separate from full-engine execution as they are added.
  Document the actual commands, prerequisites, and measured runtime when known.
- Keep the established operational path working as capabilities are added.
  Extend relevant checks and review their evidence in each increment; choose
  additional cases when implementation reveals a need for them.
- Check Windows/Linux feasibility early. Identify compatible engines separately;
  success with the available Linux runner does not establish Windows support.

The initial target is established when the saved operation can be reproduced,
the main-step checkpoints and known gaps are recorded, and another developer can
follow the capture instructions. Rust equivalence is demonstrated incrementally
as implementation reaches those checkpoints. This is operational regression
evidence, not a test of hydrological calibration skill.

## Existing evidence to reuse

Within `legacy_python/`, start with `src/mgb_ops/workflows/`,
`src/mgb_ops/model/`, the reporting code, and their existing tests. Include
`rs_hydro_forecast` scripts in the behavior inventory. The tests include
synthetic data and mocked execution; their presence alone does not prove that
the saved runtime operation works.
