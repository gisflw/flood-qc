# Initial Rust regression tests

## Agreed scope

This plan records the interview decisions of 2026-10-21. Establish regression
evidence before replacing Python behavior, then use it throughout the Rust
migration.

The tested boundary is:

```text
Frozen observations, forecast, configuration, corrections, and model inputs
    -> input preparation
    -> external MGB execution
    -> decoded precipitation, discharge, and level outputs
```

- Python is the behavioral reference. The inspected reference commit is
  `7ecc86a21416b7601384715062ec4c41557d9313`; pin the actual capture commit.
- Use a temporal subset of `rs_hydro_forecast`, retaining the full model domain,
  geometry, routing, and calibration. Retain the history needed for a valid run.
- Include zero forecast rainfall, one saved raw forecast, and a corrected version
  of that forecast. All five concerns matter: time alignment, rainfall
  preparation, scenarios/corrections, initial-state isolation, and decoding.
- Tests and their harness are pure Rust. Python is used only when preparing the
  frozen reference results; routine tests must not invoke or import Python.
- Provide fast checks without launching MGB and a separate full-engine group.
  Tests can run locally and offline. No runtime ceiling has been agreed.
- Defer hot-start generation and chaining to their implementation. Carry the
  initial-state files required by this case without designing that lifecycle now.
- Record discovered Python defects and agree on intended behavior before freezing
  the affected expectations. Do not silently preserve or correct a known bug.

Live acquisition, provider response parsing, UI, reports, archiving, scheduling,
email, and the broader VB.NET additions are outside this initial boundary.
This suite establishes migration equivalence, not hydrological calibration skill.

## 1. Qualify one coherent reference case

Do this before choosing dates or implementing Rust preparation logic.

1. Inspect saved observations, forecast coverage, configuration, and runner inputs
   in `rs_hydro_forecast`. Choose one provider and the shortest useful window
   containing nonzero, spatially varying rainfall and an observation/forecast
   boundary. Record the exact reference time, timezone rules, timestep, observed
   window, forecast cycle, and end-boundary conventions.
2. Check that the required preceding observations and initial-state dates support
   that window. Shortening a run must not silently misalign state or other
   time-dependent inputs. If a valid short case is unavailable, keep the necessary
   history rather than inventing a new initialization method.
3. Establish which files the selected executable actually consumes. Preserve a
   compatible set of calibration, geometry, state, and other required forcing
   files. Identify which inputs Python regenerates and which it merely carries
   forward; parity for carried files does not prove Rust can generate them.
4. Snapshot the selected observations and their station/provider metadata,
   forecast asset and registration metadata, effective observation edits,
   scenario definitions, and explicit resolved settings. Preserve the information
   needed to test station selection and corrections, not just final rainfall.
5. Run Python against a disposable copy of this snapshot, using only saved assets.
   Prove that the case reaches input preparation, real MGB execution, and decoding
   without network access or access to mutable operational inputs.

The existing runner is
`rs_hydro_forecast/mgb_runner/MGB_Inercial_Prev_RSH`. Its historical identity is
in [model_baseline.json](../references/model_baseline.json); calculate the actual
fixture identity rather than assuming that record still matches. For this first
Python parity case, use a coherent regional input set. Do not mix it with the
separate VB.NET input folder just to follow the eventual calibration direction.
A later calibration change needs an explicit new reference.

**Exit condition:** one reproducible Python case with compatible inputs, complete
saved data, and recorded provenance. If qualification finds an incompatibility,
resolve it before accepting any outputs as expected results.

## 2. Capture and freeze the reference

Keep the extensive real-data fixture and executable outside Git. Version its
manifest, capture instructions, comparison policy, and small fixtures in Git.
Use one explicit local fixture-root setting; never fall back to the live
`rs_hydro_forecast` workspace during tests.

The fixture manifest must identify:

- Python commit, relevant dependency versions, exact capture commands, resolved
  settings, and any explicit environment settings affecting results.
- Executable hash, platform/build identity, and required runtime libraries.
- Source paths and extraction recipe, time window and forecast identity, retained
  station/cell ordering, scenario definitions, and correction instructions.
- Paths, sizes, and SHA-256 hashes of fixture inputs and expected artifacts,
  including carried initial state and calibration files.
- Expected dimensions, units, missing-value conventions, binary precision/order,
  and the tolerance for each numerical comparison, with its justification.
- Any reviewed departure from Python, with the original result, intended result,
  reason, and decision recorded together.

Capture checkpoints, not only the final export:

| Checkpoint | Evidence to retain |
| --- | --- |
| Selection and alignment | Selected stations/providers, forecast cycle, effective edits, time axes, and observation/forecast split. |
| Rainfall preparation | Small diagnostic station/grid arrays, uncorrected and corrected forecast values, and complete prepared model rainfall. |
| Model launch | Prepared timing/configuration, required input inventory and hashes, scenario identity, and initial-state identity. |
| Engine result | Original binary outputs for discharge and level, plus execution status/logs. |
| Decoding | Complete expected arrays for precipitation, discharge, and level, with cell IDs, timestamps, units, and segment labels. |

Run the Python reference at least twice from fresh directories. Check that input
hashes remain unchanged and measure repeatability, elapsed time, and fixture/scratch
size. Investigate unexplained variability rather than absorbing it into a large
tolerance. Retain the complete capture recipe so the local fixture can be rebuilt.

Expected results are read-only during tests. Updating them is an explicit capture
and review operation, never a consequence of a failed assertion. Validate the
manifest and artifact hashes before using the large case.

## 3. Implement the two Rust test groups

Use ordinary Cargo tests and the shared Rust core as it is implemented. Do not
choose a new crate architecture or build a general test framework for this suite.
Reuse preparation and decoding functions; add a thin CLI smoke check once that
interface exists. The test driver, assertions, fixture loading, and comparison
logic must run without Python.

### Fast group

Run by the normal test command. Use tiny, versioned fixtures with independently
understandable expected answers, supplemented by small Python-captured examples.
No network, real engine, large local fixture, or legacy checkout is required.

| Concern | Deliberate cases and assertions |
| --- | --- |
| Time alignment | Test local/UTC conversion and midnight rollover, first/last included timestamps, observation/forecast transition, and supported non-hourly timesteps. Off-grid timestamps and insufficient forecast coverage must produce the agreed error. |
| Rainfall and station selection | Use distinguishable station values and unsorted IDs. Test provider precedence, effective edits/exclusions, missing versus zero rainfall, exact station/cell coincidence, and interpolation with a hand-computable answer. Preserve the reviewed Python policy for gaps and duplicates. |
| Accumulation and spatial mapping | Test splitting native forecast intervals without changing totals, inclusive spatial edges, and mapping into model cell order. Assert totals only for transformations that should conserve them; a multiplication correction deliberately changes totals. |
| Corrections | Exercise multiplication, translation, rotation, their defined order, half-open correction windows, and missing values at transformed edges. Use an asymmetric grid so reversed axes or rotation direction cannot pass unnoticed. |
| Scenarios | Assert zero/raw/corrected identities, latest eligible forecast selection, rejection of ambiguous or incomplete selection, unchanged observed periods, and corrections applied only where specified. |
| Initial inputs and isolation | Check that required carried inputs reach each scenario unchanged, missing required files fail clearly, and one scenario cannot overwrite another's input or reuse stale outputs. No hot-start generation/chaining test in this phase. |
| Binary serialization and decoding | Use distinct values for each cell and timestep and nonconsecutive cell IDs. Assert axis order, byte order/precision, units, observation/forecast labels, and output time range. Independently known binary records must expose a matching encoder/decoder mistake. |
| Invalid artifacts and failed runs | Reject truncated binaries, inconsistent lengths, duplicate cell IDs, and packed-value overflow where packing is retained. Test nonzero engine exit and absent outputs through a Rust test helper or process seam. Preserve previously published successful results when a new scenario batch fails. |

Port relevant behavioral examples, not every legacy test or implementation
detail. Where Python's policy is surprising, characterize it explicitly and
resolve any proposed correction through the agreed defect policy.

### Full-engine group

Run explicitly against the qualified local fixture. Start with the available
Linux runner; Windows engine parity requires a separately identified compatible
binary and is not established by Linux success.

For zero, raw, and corrected scenarios:

1. Verify fixture prerequisites and hashes. An explicitly requested run with a
   missing fixture, executable, or runtime dependency fails with setup guidance;
   it must not report success by silently skipping the test.
2. Build each scenario in a fresh, separate scratch directory using frozen inputs.
   Pass explicit settings and times. Do not read operational caches or use the
   wall clock to select data. Keep the source workspace and fixture untouched.
3. Compare Rust selection/alignment and prepared inputs with their checkpoints
   before launching the engine. This distinguishes preparation failures from
   execution failures.
4. Launch the pinned executable with explicit input/output locations and a
   documented timeout chosen after baseline measurement. Require successful exit
   and complete expected outputs; retain logs on failure.
5. Compare complete engine output arrays and Rust-decoded outputs with the
   Python reference. Also decode frozen Python-run binaries separately so decoding
   can be checked without relying on Rust preparation being correct.
6. Verify isolation by rerunning scenarios in a different order and checking that
   results agree and protected source/fixture inputs remain unchanged. Run this
   repeatability check during qualification and after changes affecting isolation.

Choose a correction that changes at least one forecast input value in the real
case, so the corrected scenario cannot pass as an accidental no-op. Do not require
every discharge or level value to change in response.

## 4. Compare precisely and report useful failures

- Compare timestamps, cell IDs/order, shapes, units, missing-value masks, scenario
  identity, and selection metadata exactly.
- Require exact prepared bytes where the model contract requires them. Compare
  structured container contents semantically when compression, metadata ordering,
  or explicitly excluded creation timestamps may differ. Document exclusions;
  never normalize meaningful differences away.
- Compare every numerical element in the agreed arrays, not just a sample,
  checksum, basin mean, or peak. Chunk large comparisons if needed.
- For finite floating-point values, use
  `abs(actual - expected) <= atol + rtol * abs(expected)` with explicit tolerances
  by variable and stage. Exact comparisons remain the default where justified.
  Missing/non-finite values require explicit classification checks before this
  formula; unexpected NaNs or infinities must not pass.
- Establish tolerances from capture repeatability, arithmetic, and stored
  precision. Repeatability alone does not justify a tolerance for different Rust
  arithmetic. Assess packed export rounding separately from pre-packing values;
  do not allow quantization to hide preparation or decoding errors.
- Report the failed stage/scenario/variable, timestamp and cell ID, expected and
  actual values, allowed error, mismatch count, and maximum error. Retain failed
  actual artifacts and logs under a clearly reported scratch path.

Prove that these checks detect mistakes: shift an expected time by one timestep,
swap cell order, perturb one value beyond tolerance, change a missing-value mask,
and substitute the wrong scenario input. Small Rust checks of the comparator
should fail these altered cases and accept values within the declared tolerance.

## 5. Implementation order and completion

1. Qualify the regional subset and settle any baseline defects or incompatibility.
2. Capture checkpoints for the three scenarios; publish the manifest and local
   setup/capture instructions. Measure repeatability and runtime.
3. Add the minimal Rust fixture reader/comparisons and prove their failure
   detection. Add the fast cases alongside each migrated capability.
4. Reproduce preparation and decoding against frozen checkpoints, then connect
   real MGB execution. Keep the first complete path working before expanding it.
5. Document the actual fast/full commands and local prerequisites when they exist.
   Measure their runtime and agree on a budget rather than assuming one now.

The initial suite is complete when:

- Fast Rust tests pass without Python, network, MGB, or unversioned data.
- All three full-engine scenarios pass from the immutable local case with the
  five agreed concerns covered and stage-specific comparisons working.
- Repeated fresh runs give the expected results and preserve inputs; missing
  prerequisites and deliberately corrupted cases fail visibly.
- Provenance, hashes, tolerances, any accepted behavioral changes, reproduction
  instructions, commands, and measured runtime/storage requirements are recorded.

The exact dates, retained history, forecast provider, correction parameters,
required file set, tolerances, and runtime remain to be established during
qualification. This document does not claim that the baseline already runs.

## Existing evidence to reuse

Paths below are relative to `legacy_python/`:

- `src/mgb_ops/workflows/scenarios.py` and `scenario_orchestrator.py`: scenario
  selection, isolated preparation, and batch publication.
- `src/mgb_ops/model/prepare_mgb_meta.py`, `prepare_mgb_rainfall.py`,
  `mgb_execution.py`, and `export_mgb_outputs.py`: timing, rainfall, process
  execution, and output contracts.
- `tests/engine/`: preparation, timestep/UTC boundaries, rainfall conservation,
  cell ordering, binary errors, and decoding/packing examples.
- `tests/integration/test_forecast_scenarios.py` and
  `test_observed_spatial_grid.py`: artifact snapshots, provider combination,
  coverage boundaries, isolation, and preservation of successful batches.
- `tests/unit/test_forecast_grid_corrections.py`: asymmetric transformation and
  native-grid-before-resampling behavior.

These are starting evidence, not proof of a qualified end-to-end baseline. The
inspected tests include synthetic data and mocked process execution.
