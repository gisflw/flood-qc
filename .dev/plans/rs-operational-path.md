# Saved RS operational path

Status: draft for review. This plan authorizes no implementation by itself.

## Objective and agreed scope

Deliver migration increment 3 as a general Rust library/CLI workflow from saved
observation history and downloaded precipitation forecasts to canonical model
inputs, real MGB execution, and canonical result artifacts. The four RS scenarios
are the acceptance case, not hard-coded application behavior: zero rainfall, raw
ECMWF, raw NOAA, and corrected ECMWF. Retain the complete historical simulation
and forecast timeline for every scenario.

Use Linux for acceptance; Windows remains explicitly unverified. Preserve the
external numerical engine and carry the saved calibration, geometry, and initial
state unchanged. Observed/forecast management remains outside the numerical
engine. Clarify this contract against the legacy Python and VB.NET flows rather
than assuming the capture scripts define the intended operation.

Follow [architecture ownership and execution lifecycle](../../docs/architecture.md).
Extend persistence only as these capabilities require it. Live downloading,
observation acquisition/normalization, other forecast products, GUI, reports,
hot-start generation/chaining, and hydrological calibration validation are outside
this increment.

## Evidence and preservation

- [RS reference record](../../references/regression/rs-hourly/README.md) and
  `/workspace/regression-rs-hourly-r1`: original capture/repeat, frozen history,
  normalized forecasts, runner inputs, raw outputs, decoded results, and scripts.
- Original downloaded cycle: July 20, 2026 at 00:00 UTC, under
  `/workspace/rs_hydro_forecast/data/downloads/`:
  `ecmwf/ifs_fc_20260720T000000Z.grib2` and
  `noaa/gfs_fc_20260720T000000Z.grib2`.
- [Python reference map](../../references/python_map.md), especially adapters,
  observed precipitation, QC, forecast edits, model preparation/export, and
  `workflows/scenario_orchestrator.py`.
- [VB.NET reference map](../../references/vbnet_map.md), especially Form8's
  observed/forecast preparation, execution, and output loading.

Use disposable copies without hard links. Never mutate the runtime, legacy
sources, or original frozen evidence. Copy the selected downloads into a revised
acceptance bundle and record sizes, hashes, settings, executable identity, and
source provenance. Keep large artifacts outside Git. Never fall back to live
providers, operational databases, or current-clock selection.

## Delivery structure and cycle

Each step owns its scope, implementation/check cycle, evidence, and completion
gate. Trace the relevant legacy behavior, establish concrete contracts, implement
the bounded scope, validate against checkpoints, and review acceptance before
dependent work relies on it. Distinguish implementation from qualified behavior.

Step 1 establishes the corrected reference and shared contracts. Tracks 2a and 2b
can then execute in parallel, each with independent acceptance. Their integration
closes step 2. Step 3 consumes that accepted canonical model-input boundary.

## Step 1 — Correct and qualify regression alignment

### Scope and work

1. Trace how the externally managed operation selects observations and forecasts,
   constructs forcing, invokes the engine, and interprets its outputs. Compare
   Python, VB.NET, and the actual captured runner configuration.
2. Establish an explicit timeline contract: reference time and timezone, hourly
   interval starts/ends, sample timestamps, historical/forecast boundary, coverage,
   basin ordering, and output-to-time mapping. Explain the engine flags and metadata
   used by this externally managed workflow; do not infer them from comments alone.
3. Fix the current regression's alignment in preparation, configuration, output
   assembly, and timestamps wherever the trace establishes a mismatch. Reassess
   the level-recovery interception and extra execution. Neither that workaround
   nor an executable defect is an accepted premise. Establish the correct output
   contract before deciding whether any recovery is needed.
4. Produce a revised reference in a separate location, preserving the original.
   Record each changed expectation and its reason; do not relabel changed values
   as equivalent through a larger tolerance. If engine behavior cannot be resolved
   from available evidence, record the blocker rather than freezing an assumption.
5. Reproduce all four scenarios and repeat from fresh disposable inputs. Retain
   intermediate checkpoints for normalization, reviewed observations, canonical
   forcing, engine serialization, raw streams, and decoded artifacts.

### Acceptance and gate

- Verified full historical-plus-forecast timeline, with no unexplained gaps,
  overlaps, shifts, or samples assigned to the wrong segment.
- Forecast accumulation intervals cover all required hourly targets; scenario
  differences and the common historical portion have explained checkpoints.
- All four scenarios execute successfully and fresh repetition is compared.
- Original and revised evidence remain distinguishable, with justified changes
  and reproducible commands. State compatibility uncertainty remains explicit.
- The canonical contracts needed by both tracks below are written and agreed:
  values, units, intervals, timezone rules, geometry/coordinates, basin order,
  missingness, and the observed/forecast join.

## Step 2 — Available inputs to canonical model inputs

### Track 2a — Observations to canonical observed forcing

Scope: load selected observations and station information from the saved history,
apply explicit curated review instructions, and produce canonical observed forcing
through the required temporal handling and spatial interpolation.

Implement suspect-precipitation detection, but use the six saved replacements as
curated instructions for this case. Detection produces findings; it must not
silently become authority to replace values. Preserve original values, selection
provenance, instructions, and the distinction between missing data and zero.

Acceptance: compare selection, detection findings, the six effective replacements,
working-grid checkpoints, and final observed forcing against step 1. Verify time
coverage, units, geometry, missingness, and numerical tolerance independently of
the forecast track. Use small fixtures for detection and interpolation edge cases.

### Track 2b — Forecast downloads to canonical forecast forcing

Scope: normalize saved ECMWF IFS and NOAA GFS precipitation GRIB files, apply the
reference's explicit forecast correction instructions, and produce canonical
forecast forcing with the required spatial and temporal interpolation. Include
the zero-rainfall scenario without inventing a provider artifact for it.

Use Terrio as the sole direct dependency for spatial-format I/O, using native Rust
drivers. Keep provider normalization semantics and scientific processing in our
domain code. First qualify its GRIB metadata/values and NetCDF capabilities against
the actual files; report unsupported requirements before proposing another library.
Pin the qualified version and enable only needed features. Document the lasting
dependency boundary in architecture and reference it from AGENTS.md.

Acceptance: compare normalized fields before corrections, transformed fields, and
canonical forcing against step 1, including accumulation resets/differencing,
units, source bounds, orientation, coordinates, masks, target-hour coverage, and
numerical tolerance. Native source resolution must remain distinguishable from
the working grid. Normalization works independently through library and CLI and
automatically when a normal operation receives GRIB inputs.

Terrio starting sources: [repository](https://github.com/opengeos/terrio),
[format support](https://terrio.dev/formats/), and
[Rust API guide](https://terrio.dev/rust/). Documentation claims are not acceptance
evidence for our files.

### Integration, snapshots, and step 2 gate

App composes both accepted tracks into canonical model inputs under store-owned
contracts. Model-specific binary/text serialization belongs to step 3. Freeze actual
selected observations, forecast sources, corrections, settings, calibration,
geometry, initial state, and executable identity at execution startup; hashes and
mutable paths alone are insufficient. Retain values or immutable versions.

Acceptance: all four scenarios have complete canonical inputs on the agreed
timeline, the historical/forecast join passes explicitly, and later source or
review changes cannot change the snapshot. Reuse prepared snapshots without live
providers, mutable history, or caches. Define the canonical schema/layout and
publication metadata only to the extent needed by these operations.

## Step 3 — Model execution to canonical result artifacts

### Scope and work

- Serialize accepted canonical inputs into engine-specific files; verify generated
  metadata, rainfall binaries, ordering, and carried baseline/state identities.
- Follow legacy Python's batch behavior: prepare shared observations once, run
  scenarios concurrently in isolated input/output directories, let every scenario
  finish, and collect failures. Record the engine thread environment so resource
  use and numerical repeatability are explicit.
- Hold a workspace execution/publication lock through the batch and reject a
  second overlapping batch. Do not confuse independent scenario parallelism with
  concurrent operations publishing in one workspace.
- Execute the real MGB process with useful logs, exit status, and structured
  failures. Decode according to step 1's qualified output/time contract into
  canonical precipitation, flow, and level artifacts for the full timeline.
- Model stages results; app coordinates status and asks store to publish only a
  complete durable requested set with consistent references. A failed scenario or
  publication leaves the previous published set official. Retain failed-attempt
  diagnostics and successful staged outputs, intentionally differing from Python's
  deletion of failed-batch staging. No automatic retry subsystem is required.
- Expose workflows through the shared library and thin CLI. Provide one normal
  operation command composing normalization, preparation, execution, decoding, and
  publication without an inspection/approval pause. Also provide preparation and
  execution from prepared inputs, plus standalone forecast normalization. Final
  command names and TOML contracts are settled with implementation. Accept explicit
  dates, saved sources, settings, and scenario definitions; do not bake in RS dates
  or four fixed scenarios.

### Acceptance and gate

- All four scenarios run concurrently in isolated directories and produce canonical
  full-timeline results matching the corrected reference within accepted tolerances.
- Normal execution and execution from prepared inputs agree. Requested real-engine
  checks fail visibly when prerequisites are absent; they never silently skip.
- Verify snapshot stability, overlapping-batch rejection, all-scenario completion
  after an individual failure, retained evidence, and complete-set publication,
  including failure leaving previous results intact.
- CLI and library invoke the same workflows; users receive useful artifact
  locations, status, and failures. Preserve existing inspection behavior.

## Numerical and implementation validation

Establish proposed absolute/relative tolerances from measured differences at each
stage for rainfall, flow, and level, and bring them for review. Require exact
identifiers, dimensions, time alignment, segment classification, and missing-value
masks; reject unexpected non-finite values. Compare complete arrays, not only
selected checkpoints. Distinguish raw numerical differences from NetCDF packing;
the original 0.001 packing resolution alone does not justify every tolerance.

Use small behavioral tests for scientific edge cases, source validation, lifecycle,
and failures, separate from explicitly requested full-engine acceptance. Rust
routine checks must not depend on Python; legacy Python may establish reference
evidence. Run the required formatting, Clippy, full tests, and independent
architecture test described in [AGENTS.md](../../AGENTS.md).

Implementation updates the workspace/operation contracts and usage documentation,
architecture for durable decisions, regression provenance for corrected scientific
evidence, and the migration checklist for actual status and gaps. Retire this plan
after acceptance once unique lasting information has its authoritative home.
