# Run input sets

The [workspace](workspace.md#operating-layout) keeps reusable observations and
normalized forecasts, and operators can select imported wind and reservoir
tables. See [acquisition](acquisition.md) for coverage, refresh and reuse.
After normalization, runs use the canonical inputs independently of raw downloads.

A run freezes its selected input values and effective configuration into one
self-contained set under `cache/inputs/`. The set includes observations and review
instructions, station metadata/offsets, normalized forecast bases, selected wind
and reservoir data, model inputs/calibration/initial state, and an executable
identity. `input-set.json` indexes every retained file by path, length and SHA-256
and records the resolved recipe. Each run has its own copies. Derived scenarios,
interpolated forcing and results can be regenerated and are excluded from a saved
input set. The executable must be installed separately and match the recorded
identity. Exact numerical replay also depends on the engine’s platform and
runtime, which are not packaged in the input set. See the [regional execution
evidence](../../references/regression/rs-hourly/rust-qualification.md#current-execution-environment).

TOML supplies defaults to the shared application session. Session experiments
are revision checked and captured atomically with job admission. Saving workspace
settings is an explicit action. A past run never silently supplies defaults.
The run manifest records resolved values and its input-set identity, including
configuration that may later change.

```sh
mgb-ops inputs capture --workspace /region --json
mgb-ops inputs save --workspace /region --output /archive/inputs-july20
mgb-ops inputs open --inputs /archive/inputs-july20
mgb-ops inputs execute --workspace /region --inputs /archive/inputs-july20 \
  --engine /region/model/engine --discard-inputs --json
mgb-ops inputs discard --workspace /region
```

Capturing inputs acquires configured sources and stops before MGB execution.
Run All composes acquisition, capture, materialization, execution and publication.
Before replacing unsaved current inputs, callers must choose save or discard.
Use `--save-inputs-to PATH` or `--discard-inputs` with Run All, `inputs
capture`, `inputs execute`, `operation execute` or `run regenerate`. Omitting
both returns `InputDecisionRequired` when the workspace has unsaved inputs. Automation steps use `inputs = { action = "discard" }` or
`inputs = { action = "save", destination = "..." }`. The desktop offers
save/discard/cancel before a new run and when closing or switching workspaces.

Current inputs are an atomically published cache: they survive ordinary process
failures, but may be lost or incomplete after a power loss. Saving makes an
independent verified copy and syncs it before marking the inputs saved. Explicit
Discard also resets an incomplete input cache after a power loss. Discarding removes current inputs;
run manifests remain, but regeneration requires a saved input set. A failed engine
run retains its newly frozen inputs and leaves the previous successful results
active. Inputs and successful results therefore have independent lifecycles.
Users choose how long to retain independently saved inputs and exported results.

Tabular canonical data use Parquet writer version 2 with Zstandard compression;
observation levels are signed INT32 and rainfall/flow are DOUBLE. Null, absent samples and
valid zero remain distinct. Rainfall grids retain unpacked f64 values and
explicit validity in NetCDF. Atomic manifests select complete generations;
readers never depend on half-written tables. No operational SQLite dependency,
migration path or fallback is supported. Existing spatial GeoPackage files remain
readable through the pure Rust spatial reader.
