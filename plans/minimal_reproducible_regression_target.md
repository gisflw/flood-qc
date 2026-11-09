# Minimal reproducible regression target implementation plan

Status: agreed scope; reference capture and verification remain to be implemented.

This plan specializes [the migration outline](../migration/initial_regression_tests.md)
using the session decisions. Its narrower start and end points govern this initial
target: existing normalized inputs through decoded model results.

## Agreed target

- Use one saved RS operation with both ECMWF and NOAA forecasts available locally.
- Use the full existing RS model domain, calibration, geometry, and compatible
  required initial state. Use only the available external executable for now.
- Use hourly steps, a 10-day observation setting, and a five-day forecast setting.
  Fix all times explicitly; never select data using `now`.
- Start from frozen normalized forecast NetCDFs and observation history, including
  the station metadata and registered asset records needed to consume them.
- Apply observation sequence corrections, prepare four scenarios, run the real
  engine in isolation, and decode its results.
- Stop at decoded results. Acquisition, normalization/registration regression,
  analysis, bulletins, other exports, GUI, Windows validation, new hot-start
  generation/chaining, and completed-run sharing are deferred.
- Establish reference evidence with the existing Python implementation first.
  Do not implement the Rust application as part of this increment. Later Rust
  comparisons must run without a Python dependency.

## Case and correction definitions

Prefer the forecast cycle `2026-07-20T00:00:00Z`. Both normalized assets exist,
and the saved current-run artifact selects them alongside a zero-rain scenario.
The saved artifact reports a completed execution, but uses a 30-day observation
setting and is not evidence that the new 10-day case works.

Use July 20 local midnight (`America/Sao_Paulo`, UTC−03:00) as the candidate
operation reference, distinct from the forecasts' 00 UTC cycle. The observation
start is July 10 local midnight. Qualify coverage and actual endpoint semantics
before freezing the case. Legacy `build_horizon_window` includes endpoints and
produces 121 forecast timestamps for a five-day hourly setting; its export window
is calculated separately. Record exact local/UTC timestamps, counts, interval
semantics, and any discrepancies rather than assuming 240/120 samples or silently
changing the legacy time helpers.

If this case cannot supply valid inputs, compatible state, or representative
sequence corrections, qualify another saved date with both providers. Record the
reason and selected identities; do not silently invent data or fetch replacements.

Observation correction:

- On the effective preferred observations, identify station sequences containing
  at least five consecutive hourly rainfall values strictly between 0 and 1 mm.
  Preserve this legacy rule; repeated 0.2 mm values are the representative clogged
  indication, not an exact-value-only filter.
- Replace all matching sequence values in the observation review window with
  nodata, preserving original normalized observations and explicit replacement
  instructions. Gaps break sequences. Nodata must remain distinguishable from zero.
- Include at least one actual repeated-0.2 sequence in the frozen evidence and
  record affected stations/timestamps, counts, and unchanged surrounding values.
- Apply the same corrected observations to all four scenarios. Do not add other
  observation correction policies to this target implicitly through legacy defaults.

| Scenario | Forecast forcing |
| --- | --- |
| Zero rain | Zero forecast rainfall; retain corrected observed rainfall. |
| Raw ECMWF | Selected normalized ECMWF asset without forecast edits. |
| Raw NOAA | Selected normalized NOAA asset without forecast edits. |
| Corrected ECMWF | Same ECMWF asset, multiplied by 1.5, translated +2° latitude / −3° longitude, rotated +45°, across the full selected forecast window. |

Match legacy geographic transformation semantics: translation precedes rotation;
positive rotation is counterclockwise about the original grid center. Preserve
nearest-neighbor sampling, expanded footprint, and nodata handling. Legacy shift
fields contain cell counts: on the actual 0.25° source grid encode +8 latitude
cells and −12 longitude cells, verifying resolution before conversion. Freeze
both geographic intent and encoded parameters, including the time-step range.

## Implementation sequence

1. **Qualify the case in a disposable workspace.** Record actual reference commits,
   local modifications affecting behavior/settings, executable hash and runtime
   requirements. Inspect the current RS `mgb_runner/Input`, classify consumed
   files as fixed calibration/geometry, forcing, or state, and verify that state
   is usable at the shorter window's start. The historical baseline manifest
   does not establish current input compatibility. Do not regenerate state or
   substitute the VB.NET input directory silently. Verify complete forecast and
   observation coverage and representative low-value sequences; document missing
   data and their legacy handling.
2. **Freeze the normalized inputs and operation.** Snapshot SQLite consistently,
   then retain the required observation window, preferred-series metadata,
   provider/asset records, spatial inputs, settings, executable identity, and
   consumed model inputs/state. Keep supporting records required by legacy
   loading. Pin both asset IDs explicitly so automatic latest-cycle resolution
   cannot choose a newer forecast. Rebase saved absolute paths into the scratch
   copy. Keep large datasets and binaries outside Git; document their exact
   location, retrieval/copy instructions, and checksums.
3. **Capture corrections and stage expectations.** Reuse legacy APIs to construct
   a fresh operation with the 10/5 settings and four scenarios; do not reuse stale
   published caches or mutate the operational current-run artifact. Persist the
   exact sequence replacements and forecast instruction. Capture the checkpoints
   below, separating regenerated files from unchanged carried inputs. Any legacy
   defect affecting the case needs an explicit behavior decision before freezing
   its expectation.
4. **Execute and decode.** Run each scenario with private Input/Output directories
   and its own copy of the same initial state. Preserve commands, logs, exit status,
   raw outputs, decoded artifacts, and measured runtime. A failed engine run,
   missing prerequisite, or invalid output is a failed capture, never a skip or
   successful reuse of older output.
5. **Reproduce from a fresh copy.** Repeat from frozen inputs without live providers,
   the original mutable history, current-run artifact, or preexisting caches.
   Compare checkpoints using recorded rules. Finish a small versioned manifest,
   expectation summaries, and exact runnable capture/replay instructions. Update
   the migration outline to link the established case and its explicit exclusions.

## Checkpoints and comparisons

| Checkpoint | Required evidence |
| --- | --- |
| Frozen input identity | Checksums, asset/cycle IDs, observation provenance, settings, spatial domain, executable and state identities. |
| Observation review | Sequence membership, persisted nodata replacements, corrected effective series, unchanged nonmatched values. |
| Forecast correction | Source resolution, +8/−12 encoded shifts, ×1.5 and +45° instruction, time range, transformed coordinates, footprint and missing-value mask. |
| Prepared engine inputs | Scenario identity, time metadata, rainfall binary shape/order/units and values, unchanged required fixed inputs and state. |
| Real execution | Successful process, fresh expected output files, dimensions/lengths, logs and runtime per scenario. |
| Decoded results | All variables actually decoded by the legacy path, units, timestamps, cell IDs/order, segment labels, missing-value masks and numerical values. |

Compare structure, identities, coordinates, and missing-value masks exactly.
Compare deterministic prepared files exactly where appropriate; ignore incidental
run IDs, paths, and creation timestamps through an explicit field list. Establish
and record numerical tolerances from repeated capture and actual storage precision
before accepting floating-point comparisons; do not invent permissive tolerances.
Check full captured arrays for alignment, expected missingness, and unexpected
non-finite values. Retain compact representative values in the versioned summary
and full numerical artifacts outside Git. Demonstrate that the corrected scenario
actually changes forcing and that the observation replacements survive preparation
with the intended missing-data treatment.

## Existing code to reuse and affected surfaces

Paths below are relative to `/workspace/legacy_python`:

- `src/mgb_ops/workflows/scenarios.py` and `assets/current_run.py`: saved operation,
  explicit scenarios and correction persistence; inspect automatic QC seeding.
- `src/mgb_ops/qc/precipitation.py` and `assets/effective_observations.py`: legacy
  bounded sequence rule, preferred observations, and replacement application.
- `src/mgb_ops/edit/forcing.py`: grid correction and cell-based displacement.
- `src/mgb_ops/workflows/scenario_orchestrator.py`: isolated preparation/execution.
- `src/mgb_ops/model/prepare_mgb_meta.py`, `prepare_mgb_rainfall.py`,
  `mgb_execution.py`, `export_mgb_outputs.py`, and `utils/time.py`: serialization,
  real runner, decoding, and time boundaries.
- Existing `tests/unit/test_forecast_grid_corrections.py` and relevant QC/model
  tests: reuse behavioral checks rather than creating a parallel framework.

Add only the capture/replay glue and focused checks needed for this case. Review
callers before any necessary legacy fix, preserve its existing validation, and
record the fix's provenance. No new runtime dependencies, Rust crate structure,
public exports, or operational configuration changes are required by this plan.

## Completion criteria

Another developer can obtain the identified frozen inputs and available engine,
follow the documented commands in a fresh disposable workspace, and reproduce all
four scenarios through decoded results. Evidence establishes the sequence/nodata
correction and geographic forecast edit, exact time boundaries, prepared inputs,
real execution, comparison rules, and repeatability. Source workspaces remain
unchanged. Known limitations are recorded; Rust equivalence and deferred stages
are not claimed by this reference-capture milestone.
