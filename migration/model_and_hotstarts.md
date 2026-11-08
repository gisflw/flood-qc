# Model inputs, hot starts, and execution

## Owns

- Selected model baseline, input serialization, run metadata, and time alignment.
- Rainfall, observed-flow, reservoir-release, and wind input generation.
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

Reverse engineer the existing hot-start procedure, including `CI.meta`,
`CI_*.HOT`, `HOT.HOT` where used, and state copied from output to input. Establish
the exact behavior against the selected executable during implementation.

Prepare scenarios in isolated directories. Preserve prior successful results
when a new execution fails. Keep input preparation and process execution usable
from the CLI without the desktop. Investigate routing-selection behavior against
the selected engine before promising multiple routing modes.

## Evidence of completion

Regenerate a representative reference case if necessary. Compare generated
formats and numerical outputs against that case using appropriate tolerances.
Verify a subsequent operation uses the expected initial state. Record executable
identity and required input compatibility; do not assume existing folders match.
