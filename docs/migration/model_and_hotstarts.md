# Model inputs, hot starts, and execution

## Owns

- Engine-specific serialization and time layout from prepared canonical forcing
  and explicitly selected baseline, settings, and run metadata.
- Rainfall, observed-flow, reservoir-release, and wind engine-input generation.
- Hot-start loading and the initial-state lifecycle found in VB.NET.
- Isolated scenario execution and decoding model outputs for downstream use.

## Starting references

Executable: `rs_hydro_forecast/mgb_runner`.
Initial calibration, geometry, and state: the compatible input set in
`rs_hydro_forecast/mgb_runner/Input`. The VB.NET input contracts are references
for added functionality, not a requirement to replace the RS baseline.

Python: `src/mgb_ops/model/` and
`src/mgb_ops/workflows/scenario_orchestrator.py`.
VB.NET: `Form3` (QOBS), `Form5` (QSUBST and reservoirs), `FormWind` (wind), and
`Form8` (preparation, initial conditions, execution, state persistence).

## Implementation direction

First trace which inputs the chosen executable consumes. Classify them as
calibration/geometry, time-dependent forcing, or model state. Incorporate the
needed behavior; do not copy every file in the other system indiscriminately.

First reproduce the saved RS preparation/execution/decoding path against the
minimal regression target, carrying its required initial state unchanged. Add
remaining forcing generation and hot-start lifecycle behavior incrementally.

Model accepts explicit prepared datasets, paths, and parameters using store's
canonical contracts. Shared app workflows select inputs and freeze actual values
or immutable versions at execution startup, including calibration, geometry,
initial state, and executable identification. App supplies data/target geometry
to analysis for general QC, corrections, resampling, and interpolation, then
passes prepared canonical forcing to model. Model must not import analysis or
adapters, discover registry data, or load global configuration.

Model owns engine serialization, private execution directories, process handling,
decoding into staged canonical results, and fixed station-reference level
conversion. Preserve raw basin levels and produce referenced station levels using
the [agreed offset contract](../architecture.md#level-reference-and-analysis-corrections).
Numerical data and output contracts belong to store. Model may use explicit staged
files for large outputs; canonical
data does not have to be registered in a database before processing. App
coordinates the scenario set and asks store to record per-run outcomes and output
paths; model stages outputs without maintaining a run registry.

Reverse engineer the existing hot-start procedure, including `CI.meta`,
`CI_*.HOT`, `HOT.HOT` where used, and state copied from output to input. Establish
the exact behavior against the selected executable during implementation.

This is a later increment, outside the initial regression target. Reproduce
VB.NET `Telemetria/Form8.vb` behavior: `ESCREVE_HOTHOT` writes the use-hot-start
flag; `ESCREVE_DATA_HOT` writes the requested state-save date/hour; and
`SalvaCiInput`, when enabled, promotes output `CI.meta`, `CI_W.HOT`,
`CI_VOL.HOT`, `CI_HFL.HOT`, `CI_VOL1_FL.HOT`, and `CI_Q2FL.HOT` into Input.
Verify engine-side state generation separately and recompile the engine if
needed to enable it. None of these writes, promotion steps, or recompilation
is required to establish the initial regression target.

Prepare scenarios in isolated directories. App reports whole-run success only
when every requested scenario and required output validates. A failed run retains
diagnostics and successful siblings' outputs for inspection; other runs remain
unchanged. Generate a per-run JSON manifest for application use.
App owns execution status, overlap policy and failure handling; store owns
persistence/file lifecycle. There is no official/latest-run pointer.
Keep input preparation and process execution usable from the CLI without the
desktop. Investigate routing-selection behavior against
the selected engine before promising multiple routing modes.

## Evidence of completion

Regenerate a representative reference case if necessary. Compare generated
formats and numerical outputs against that case using appropriate tolerances.
Verify a subsequent operation uses the expected initial state. Record executable
identity and required input compatibility; do not assume existing folders match.
Check that later history/review changes do not alter an execution snapshot, and
that a failed scenario preserves diagnostics and successful sibling outputs
without changing another run. Verify the recorded recipe contains selected
instruction values and generated output paths.
