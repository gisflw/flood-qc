# Step 3 — Integrate canonical inputs, execute MGB, and record run results

Status: implementation specification; capabilities and acceptance remain outstanding.

Implement preparation from canonical saved sources, reusable prepared inputs, real
MGB execution, full output decoding and a per-run JSON manifest.
Dependencies are
[qualified reference/time/output mapping](rs-reference-alignment.md),
[observed preparation](rs-observed-forcing.md), and
[forecast normalization/preparation](rs-forecast-forcing.md). The independent forcing
tracks join here. No live providers, Python runtime, direct GRIB execution, run-all
command, new hot starts, AR(1), UI or reporting is required.

## Existing behavior and exact implementation locations

Current `src/app.rs` owns inspection configuration/path resolution; `src/model.rs`
owns prerequisite inspection; `src/main.rs` exposes config/workspace commands;
`src/lib.rs` re-exports six inspection names. Preserve those APIs, TOML keys,
findings, path semantics and text/JSON output. Add private children under those
owner files; moving existing inspection code is unnecessary.

Python `workflows/scenario_orchestrator.py::_execute_scenario` copies Input per
scenario and shares observed preparation. `execute_forecast_scenarios` starts a
spawned process pool sized to the selected scenarios, waits for every future and
publishes only after success. It deletes failed staging, which Rust deliberately
changes to retained diagnostics. `execute_current_artifact` locks the workspace.
`model/mgb_execution.py::execute_mgb_plan` uses a direct executable, explicit cwd,
MGB_INPUT_DIR/MGB_OUTPUT_DIR and streamed logs. Rust keeps these engine mechanics,
without copying Python's directory clearing or destructive cache replacement.

`prepare_mgb_rainfall.py::write_mini_rainfall_atomic` casts to f32 and writes time
outermost/basin innermost (Fortran flatten of its basin×time matrix).
`export_mgb_outputs.py` assumes equal-length combined inputs and reads only Atual
flow/YTUDO; captured reference stages separate streams first. Rust must use the
qualified separate-stream mapping, not port that discovery assumption. Raw level
binary is metres; Python exports centimetres. Form8 subtracts fixed level bias;
retain station-specific offsets rather than its basin-indexed overwrite policy.

| File | Implemented responsibility to add |
| --- | --- |
| `src/app.rs`, `src/app/prepare.rs`, `src/app/operation.rs` | Re-export workflow types/functions; prepare.rs owns strict preparation request parsing/resolution, selection and join; operation.rs owns execution coordination and outcomes. No SQLite SQL or binary decoding here. |
| `src/store/values.rs` | Shared types defined by forcing specs, plus RunRecipe, ScenarioInputs, ResultSeries, StationResults and input descriptors. Pure values, no required connection/path lookup. |
| `src/store/operation.rs` | Per-run JSON manifests, validated working-input copies and result paths; no workspace-wide run registry. |
| `src/store/netcdf.rs` | Extend qualified Terrio persistence for prepared forcing/results; reuse forcing schema/time codec. |
| `src/model.rs`, `src/model/input.rs` | Keep inspection; explicit input writer and parsed metadata/geometry validation. |
| `src/model/process.rs` | Child process cwd/env/logs/status mechanics; no publication or catalog queries. |
| `src/model/output.rs` | Qualified separate-stream decoder and fixed station reference conversion. |
| `src/main.rs`, `src/lib.rs` | Thin operation prepare/execute commands and intended workflow exports. |
| `tests/operation.rs`, model-local unit tests, `tests/architecture.rs` | Public workflow/lifecycle integration, private byte/decoder unit checks, architecture extensions; preserve existing inspection tests. |
| `tests/rs_saved.rs` | Explicit ignored real-case test, fails on missing prerequisites when invoked; no silent skip. |

Use existing Serde/TOML/Clap and forcing tracks' rusqlite/chrono/Terrio dependencies.
Add SHA256 hashing (`sha2`) for retained identity if not already added by store;
no process, async runtime or locking dependency is needed.

## Public workflow and model contracts

Expose through app (and explicitly from lib facade for CLI) these functions:

```rust
pub fn prepare(request: PreparationRequest) -> Result<PreparationReport, OperationError>;
pub fn execute_prepared(request: ExecutionRequest) -> Result<ExecutionReport, OperationError>;
// PreparationRequest { workspace: PathBuf, invocation_dir: PathBuf,
//   request_file: PathBuf, output: PathBuf }
// ExecutionRequest { workspace: PathBuf, invocation_dir: PathBuf, prepared: PathBuf }
// PreparationReport { prepared: PathBuf, manifest_sha256: String,
//   scenario_ids: Vec<String>, artifacts: Vec<PathBuf> }
// ExecutionReport { run_root: PathBuf, run_artifact: PathBuf, status: BatchStatus,
//   scenarios: Vec<ScenarioExecution> }
```

Reports/errors implement Serialize and Display as appropriate. BatchStatus is
Completed/Failed; ScenarioExecution includes ID, status, optional exit
code, signal description where available, logs, result paths and structured
failure. A manifest write/finalization failure returns an error identifying the run
root; existing results/logs remain available and the run must not report complete.
Error variants: InvalidRequest{field,message}, InvalidPrepared{path,message},
Busy{workspace}, Source{path,message}, Store{run_root,message},
Model{scenario,stage,message,log_paths}. No operation/attempt registry IDs or
publication error are exposed.
CLI prints a single report JSON with `--json`; operational failure exits 1,
Clap usage errors retain exit 2. `prepare` errors have no executable side effect.
Do not place full observations or binary data in error messages.

Model owner entry points, used by app with explicit supplied values:

```rust
pub(crate) fn read_basin_geometry(mini_gtp: &Path)
    -> Result<crate::store::BasinGeometry, ModelError>;
pub(crate) fn write_inputs(input: &ModelInputs, destination: &Path)
    -> Result<SerializedInputs, ModelError>;
pub(crate) fn run_engine(plan: &EnginePlan) -> Result<ProcessOutcome, ModelError>;
pub(crate) fn decode_outputs(input: &DecodeInputs) -> Result<DecodedEngineResults, ModelError>;
pub(crate) fn quantize_levels(raw: &DecodedEngineResults) -> Result<ResultSeries, ModelError>;
pub(crate) fn reference_levels(raw: &DecodedEngineResults, selected_station_ids: &[String],
    offsets: &[crate::store::StationOffset]) -> Result<StationResults, ModelError>;
```

ModelInputs holds combined BasinForcing, reference/time mapping, retained baseline
Input path, and qualified EngineTiming. EnginePlan holds retained executable,
private cwd/Input/Output/log paths and OMP thread count. DecodeInputs holds
explicit raw paths, basin IDs, qualified stream row mapping and rainfall values.
ModelError distinguishes invalid timing/geometry/forcing, I/O, spawn, nonzero exit,
missing output, malformed byte length and decode mask/value failure. These values
do not import app config/report types. Store APIs include read/write_prepared,
write_run_manifest, record_scenario,
freeze_inputs and finish_run. They operate on explicit per-run paths; there is
no artifact-registration or official-publication API.

## Separate preparation request and CLI

Add `operation prepare --workspace W --request FILE --output DIR [--json]` and
`operation execute --workspace W --prepared DIR [--json]`. Workspace/request/output/
prepared CLI paths resolve from invocation_dir; resource paths *inside request
TOML* resolve from the selected workspace, matching existing resource semantics.
Existing workspace.toml remains an inspection configuration; this request is a
separate strict `deny_unknown_fields` structure at every nested table.

The following RS fixture template preserves the fixed Python application
timeline from step 1:

```toml
schema_version = 1
time_zone = "America/Sao_Paulo"
reference_time = "2026-07-20T00:00:00"
step_seconds = 3600
historical_first_end = "2026-07-10T00:00:00"
historical_last_end = "2026-07-20T00:00:00"
forecast_first_end = "2026-07-20T01:00:00"
forecast_last_end = "2026-07-25T01:00:00"

[observed]
path = "fixtures/observations.sqlite"
provider_codes = ["ana"]
station_ids = []
series_ids = []
replacement_ids = ["rs-null-1", "rs-null-2", "rs-null-3", "rs-null-4", "rs-null-5", "rs-null-6"]
# Required ordered list; [] explicitly selects no replacements.

[geometry]
mini_gtp = "mgb_runner/Input/MINI.gtp"

[grid]
bbox = [-58.95016, -34.40217, -48.85759, -26.29721]
resolution_degrees = 0.1

[interpolation]
nearest_stations = 5
power = 2.0

[qc]
high_threshold_mm = 50.0
sequence_lower_mm = 0.0
sequence_upper_mm = 1.0
sequence_min_length = 5

[levels]
station_ids = ["station-code-from-qualified-fixture"]

[engine]
executable = "mgb_runner/MGB_Inercial_Prev_RSH"
input_dir = "mgb_runner/Input"
omp_threads = 1

[forecasts]
catalog = "fixtures/forecast.sqlite"

[[scenarios]]
id = "zero"
kind = "zero"
[[scenarios]]
id = "ecmwf-raw"
kind = "raw"
asset_id = "<ecmwf-asset-id>"
[[scenarios]]
id = "noaa-raw"
kind = "raw"
asset_id = "<noaa-asset-id>"
[[scenarios]]
id = "ecmwf-corrected"
kind = "corrected"
asset_id = "<ecmwf-asset-id>"
correction_ids = ["rs-ecmwf-correction"]
```

This is a template: the one-off fixture converter writes actual asset_id
values returned by canonical normalization (or its frozen-grid fixture import),
and actual correction IDs linked to those assets. Record the ordered selected
replacement and correction values, not just their IDs.
Legacy asset IDs are provenance, not new content/normalization source IDs.
Replace the placeholder levels station list with explicit qualified selected
station IDs in the actual fixture. An explicit empty list means no station
outputs; never discover a changing default output station set during execution.
The selected forecast correction contains +8 latitude pixels, −12 longitude
pixels, 45° rotation, factor 1.5, hours 0–126, not degree translations.

Validate version, IANA zone, second precision/hour alignment, positive integral
step=3600 for this increment, increasing/contiguous intervals, reference equal to
historical_last_end, forecast_first_end=historical_last_end+step, finite bbox,
positive resolution/k/power, required ordered replacement_ids (possibly empty),
and at least one observed provider/station selector. Retain optional explicit series selection.
Reject ambiguous/nonexistent local endpoints and zone-offset transitions under
the shared unqualified-DST limit. No implicit days-to-count conversion is needed
when endpoints are supplied. Require nonempty unique scenario IDs matching
`[A-Za-z0-9][A-Za-z0-9_-]{0,63}` and at least one scenario. Zero rejects asset_id/correction fields. Raw requires
asset_id and may omit correction_ids or supply []; corrected requires asset_id
plus a nonempty ordered correction_ids list. Reject
unknown/duplicate instruction IDs and any correction.asset_id != selected asset_id.
QC scalar fields are required, finite and retained: high_threshold_mm>=0,
0<=sequence_lower_mm<sequence_upper_mm, sequence_min_length>=1; sequence spacing
is the selected hourly step. Persist all four values, not only a policy name.
Validate each selected level station exists,
offset mini references geometry, and all source/baseline paths are regular files
or directories as required. Reject nested/escaping input-reference paths and symlinked
entries in retained trees. Destination must be new; do not overwrite preparation.

## Preparation join and retained prepared contract

1. Resolve request and validate source metadata. Call store read_selection once to load eligible candidate-series metadata,
   their in-window canonical samples, replacements/stations/selected offsets;
   analysis ranks candidates in memory without a database connection. Ask model to parse
   MINI.gtp into BasinGeometry (actual Mini IDs/order/centroids); app and analysis
   do not parse engine-specific geometry and GPKG decoding is not a prerequisite. Copy the selected
   canonical rows/actual values and provenance to a new preparation staging tree.
   Use a SQLite read transaction for a consistent selected view; materialize
   canonical subset through store, never copy an open SQLite/WAL file blindly.
2. App calls `analysis::prepare_observed_forcing` once with ObservationInputs,
   IntervalAxis, RegularGridSpec, BasinGeometry, IdwSettings and QC policy from
   step 2a. Detection findings never authorize new reviews. Retain effective-row
   and QC checkpoints alongside raw source/replacement subset.
3. For each forecast scenario, resolve asset_id to its native GridSeries
   and source identity, load the explicitly ordered correction_ids and validate
   correction/asset equality, then call step 2b's `prepare_forecast`;
   zero uses
   `prepare_zero_forecast(&IntervalAxis,&BasinGeometry)` returning
   `Result<BasinForcing,ForecastPreparationError>`. Retain selected native source
   artifacts/settings/corrections once, plus transformed/working checkpoints.
4. Concatenate historical and forecast BasinForcing in chronological interval
   order. Require exact timezone, step, mini IDs/order/coordinates and units;
   reject gaps/overlaps, invalid/missing required rainfall or nonfinite/negative
   totals. All scenario historical prefixes must be identical. No implicit
   regridding, timezone shifting or source selection occurs at this join.
5. Store writes canonical prepared artifacts, selected metadata/offsets, request
   bytes and manifest.json containing the resolved recipe and input references with
   relative paths/size/SHA256/kind. Generated output locations are recorded without
   a separate SQL artifact registry.
   Preserve baseline Input/executable bytes as prepared dependencies so reuse can
   work after mutable sources disappear. Preparation does not invoke MGB.
   Flush/close and sync each file; finalize by same-filesystem staging rename to
   the new destination, sync its parent. Keep failed staging with diagnostics;
   a previously completed preparation is never replaced.

Prepared directory schema v1:

```text
manifest.json             recipe/settings, ordered instruction values, input references
request.toml              exact accepted request bytes
sources/observations.sqlite   selected raw/replacement/station/offset values
sources/forecast.sqlite       selected asset/correction rows
sources/native/<source-id>.nc frozen canonical native values
baseline/Input/...        selected baseline, unchanged state/calibration
baseline/engine           frozen executable with executable mode preserved
forcing/observed.nc
forcing/<scenario-id>.nc  complete historical+forecast BasinForcing
checkpoints/...           effective rows/QC/working/transformed grids
```

Record hashes after copying, not only pre-copy source hashes. No hard links to
mutable sources. A source changing during selection/copying fails consistency
validation rather than producing mismatched metadata and bytes. Verify native
hashes against asset identities. Execution verifies the prepared input references
and freezes their values before starting children; no provider/history lookup
remains during that execution. The caller hashes prepared manifest.json separately
as manifest_sha256; it is not included in its own input references.

## JSON manifests and execution files

Generate one manifest.json per preparation/execution directory for CLI/GUI
consumption. It records a resolved recipe, input identities/references, scenario
outcomes and output locations directly as JSON. No run SQLite, operation/attempt
hierarchy, per-file output registry or latest/official pointer is required.

The version-1 manifest has schema_version=1; state
(prepared/running/failed/completed); recipe; inputs; scenarios; created_at;
finished_at and error (nullable until available). request.toml retains the exact
accepted request bytes and is included among the input references.
recipe contains the resolved operation timezone/axes, numerical settings, selected
station/series metadata and offsets, ordered full replacement/correction records,
geometry/calibration/state identities, and engine/environment identity.
inputs lists input paths, kind, size and SHA256; paths resolve relative to the
manifest's directory and cannot escape it. These are integrity references, not
a claim that hashes restore bytes. scenarios preserves request order and each
scenario's ID, state (pending/running/failed/completed), start/end, exit/signal,
structured error, log paths and validated result paths. Prepared scenarios are
pending. Timestamps follow the operation-local convention; process durations
remain monotonic elapsed times in execution.json.

Validate JSON shapes, schema version, nonempty IDs, input hashes and paths before
execution, and output membership/paths before reporting a completed run. Write
manifest updates through a temporary sibling file, flush/sync it, atomically
rename over that run's previous manifest and sync its parent on Linux. A failed
write must not expose partial JSON or report completion. One coordinator writes
the manifest; model workers return outcomes without writing it. Interrupted runs
remain explicitly unfinished; automatic recovery remains outside scope.

```text
data/runs/<run-id>/
  manifest.json             independent copy of recipe + current outcomes/output paths
  snapshot/                 frozen prepared inputs, including original prepared recipe
  scenarios/<id>/Input/     independent writable baseline copy + serialized inputs
  scenarios/<id>/Output/    original raw streams, never concatenated in place
  scenarios/<id>/logs/stdout.log, stderr.log, execution.json
  results/<id>/basin.nc
  results/<id>/station.nc
```

Choose a unique run-id/root after workspace lock acquisition; create_dir must fail
on collision and never clear an existing root. The root manifest.json is the GUI's
recipe/outcome artifact; snapshot/manifest.json remains the unchanged prepared recipe.
Rebase its input references into snapshot/, initialize new run timestamps/status,
and update outcomes only in the root artifact. Copied source database subsets
provide provenance; execution consumes the recipe and verified frozen arrays/files
without reopening the source catalogs. Generating these files is normal execution,
not an opt-in save feature.
Operators may copy the artifact, inputs, results or cache outputs elsewhere for
long-term storage; add no retention flags, cleanup policy or archive manager.
Future regeneration requires the referenced data and compatible engine to remain
available. Copying only manifest.json records the instructions; it is not a
self-contained reproducible bundle. Existing working-input copies support reuse
while present, without imposing long-term retention or provider availability.

## Lock, child processes and failures

Open `<workspace>/.operation.lock` read/write/create without truncation; call
`File::try_lock()` and retain that File until all children finish and run outcome recording
completes/fails. Map `std::fs::TryLockError::WouldBlock` to Busy; other lock errors fail. Do not unlink the
lock file (unlink permits another inode/lock). This is a nonblocking OS lock,
not lock-file-existence/PID logic; process exit releases it. The pinned standard
library supports it ([File locking API](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)).

App performs serialization for every scenario and records any preparation failure,
then starts all viable scenarios in isolated directories using scoped std threads
(one per requested scenario). Each worker owns `Command`, Child and its private
paths; `Command::new(retained_engine)` invokes no shell, sets current_dir, clears
inherited environment and supplies absolute MGB_INPUT_DIR/MGB_OUTPUT_DIR plus
OMP_NUM_THREADS from request (positive integer). Keep environment allowlist
explicit; qualify whether runtime loader needs additional selected values, retain
those exact values in execution.json and reject uncontrolled preload variables.
The reference environment supplies only the engine variables and OMP threads.

Redirect stdout/stderr directly to separate files before spawn, avoiding pipe
buffer deadlocks and unbounded in-memory logs. Record command/cwd/env, start/end,
elapsed time, exit code/signal and output checks. A nonzero exit, missing required
stream, malformed output or decode/reference failure fails that scenario. Joining
all workers collects every failure; one failed scenario must not cancel healthy
siblings. On coordinator error after children launch, join/reap all workers
before releasing the lock. No automatic retry or user approval pause occurs.

Whole-process termination may leave unfinished records and child processes.
Automatic unfinished-run recovery, startup state rewriting and process-tree
recovery are outside scope. Controlled failures still join/reap all launched
children and preserve diagnostics before releasing the lock.

## Engine bytes, metadata and decoding

Write CHUVABIN as IEEE754 little-endian f32 with time outermost and basin innermost:
`for time { for basin { write((value as f32).to_le_bytes()) } }`. Check conversion
is finite and rainfall is valid; expected bytes = time_count×basin_count×4 with
checked integer multiplication. Do not write record markers, headers, NaN padding
or round to NetCDF packing resolution. MINI.gtp row order supplies basin order;
validate count/unique IDs and coordinates against prepared geometry.

Parse PARHIG by header plus next noncomment/nonblank data line as legacy
`prepare_mgb_meta.py` does. Replace only simulation start and NT/DT fields using
qualified EngineTiming; keep all other baseline bytes, including encoding,
comments and calibration flags. Parse Previsao.meta's start/count and basin
metadata; replace corresponding fields using qualified forecast segment, retaining
unrelated content. Implement byte-oriented ASCII numeric substitution rather than
re-encoding Latin-1 comments as UTF-8. Fail duplicate/missing headers or mismatched
basin metadata. No blind October string replacement. Timing/count/ICALIB changes
beyond these fields require step 1 evidence; do not assume ICALIB=2 from its comment.
Carry every required baseline input/state unchanged into each private Input and
never copy engine-written DATA.hot back into snapshot/baseline/runtime sources.

Decode original `QTUDO_Inercial_Atual.MGB`, `QTUDO_Inercial_Prev.MGB`, `YTUDO.MGB`,
`YTUDO_prev.MGB` as little-endian f32, exact qualified segment lengths × basin
count × 4. Reject truncated/extra/nonmultiple bytes. Map rows to qualified sample
labels independently of rainfall interval bounds. Preserve original streams and
avoid fabricating missing observed levels from overwritten YTUDO prefixes.
Use step 1's shared history-only execution (NT=241, forecast horizon zero)
for complete historical levels, then the four independent scenario executions.
Validate common historical flow and unaffected level suffixes as specified there;
apply Python application labels without inferring them from engine text outputs.
Promote f32 values to f64 without quantization. Unexpected nonfinite values fail;
apply only qualified missing sentinels/masks, never blanket negative-value tests.

Canonical raw and station-referenced levels are signed **i32 centimetres**, as
chosen by the user. Preserve engine f32 metre files byte-for-byte. Model-private
`DecodedEngineResults` holds unquantized f64 metre levels promoted from those
f32 samples, with explicit masks; it is distinct from store's persisted result
types. `ResultSeries` holds rainfall/flow `Vec<f64>` plus `raw_level_cm: Vec<i32>`
and separate variable masks; `StationResults` holds `level_cm: Vec<i32>`, mask,
station IDs in explicit requested order, `mini_ids: Vec<Option<i64>>`, optional
full-precision metre offsets and supplied provenance. App validates selected
station identities against canonical station records. Model derives mini mapping
only from an optional StationOffset record; missing offset does not imply a
mapping. `quantize_levels` creates canonical raw-basin ResultSeries independently
of `reference_levels`, which creates station results from the same unquantized
DecodedEngineResults. Retain results for every selected geometry basin.

Quantize once: raw level is `round_ties_even(100 * raw_level_m)`; referenced level
is `round_ties_even(100 * (raw_level_m - offset_m))`. Apply offset in f64 metres
to the unquantized engine sample, never subtract from already quantized raw cm.
For finite valid inputs, require multiplication/subtraction/rounded result finite
and rounded value in inclusive i32 range before casting. Reject overflow; never
truncate, clip or wrap. Known engine missing samples or absent offsets produce
payload 0 with valid=0; valid centimetre zero has valid=1. No reserved sentinel
or NaN integer payload is used, so the complete i32 range remains available.
Persist invalid integer payloads as exactly zero and reject mask/payload
inconsistency on read; offset_m remains f64 with its own validity and provenance.

A concrete regression fixture uses the f32 engine sample nearest 1.0041 m and
offset 0.609 m: raw canonical level is 100 cm, referenced level is 40 cm. Deriving
the referenced level from the already rounded 100 cm would produce 39 cm and is
incorrect. Use the actual f32 sample bits in the fixture, not a decimal assumption
about the original engine value.

Canonical result NetCDF extends the forcing schema: dimensions time, basin,
bounds=2; explicit mini_id and basin coordinates; i64 local-origin time and
rainfall time_bounds; operation_timezone, schema_version, geometry hash,
reference_time, engine hash, snapshot hash and processing provenance. Variables
precipitation(time,basin) mm, flow(time,basin) `m3 s-1`,
raw_level(time,basin) i32 `cm`; rainfall/flow remain f64 and every variable has
a separate `<variable>_valid` u8; time_segment
u8 with 0 historical/1 forecast. The time coordinate uses qualified result labels;
retain rainfall bounds explicitly and distinguish instantaneous flow/level from
rainfall totals in variable metadata. No UTC canonical conversion or numeric
packing; level units follow the user decision above.

Station NetCDF dimensions time,station, fixed-width station string dimension if
required by qualified Terrio; variables station_id, mini_id(station), mini_valid(station),
offset_m(station) f64 `m`, offset_valid(station), level(time,station) i32 `cm`,
level_valid and
time_segment. Retain exact selected station list/order and offset provenance in
processing_json plus the snapshot observations SQLite. With raw basin level L metres and offset O,
station level=round_ties_even(100*(L−O)) centimetres. A missing raw level or
absent offset gives unknown mini mapping and offset: mini_id=0/mini_valid=0,
offset_m=0/offset_valid=0 and level=0/level_valid=0 at every time. Do not invent
a supplying basin. Present offsets require a validated mini_id>0 with mini_valid=1,
and finite offset_m with offset_valid=1; explicit zero and negative offsets
are valid. A
negative finite referenced level is valid. Multiple station IDs may select the
same mini and produce different results. Raw basin levels remain unchanged;
apply fixed offset over both segments and every scenario. AR(1) is excluded.

## Run outcomes and failure handling

1. Under the workspace lock, reserve a new root, verify/freeze prepared inputs and
   generate manifest.json. Start workers only from complete working inputs.
2. Record each scenario's status, diagnostics and validated output locations.
   A failure does not cancel healthy siblings; join every launched child.
3. On all successes, validate exact requested scenario membership, required raw
   streams/results, paths and decoded values. Close/sync generated files and
   affected directories on Linux, then atomically update the run manifest with
   completed and finished_at. A failed artifact write cannot report completion.
4. With any failure, record failed and structured errors where writable; retain
   diagnostics and successful siblings' outputs for inspection. Do not present
   incomplete outputs as successful. There is no official-set pointer or promotion
   transaction, and no other run's files/status are modified.
5. A process death may leave running/unfinished records. Do not infer completion
   from file presence. Automatic recovery remains outside scope. Linux durability
   evidence is required; Windows behavior remains unverified.

The CLI report and GUI read this run's manifest/results. JSON is generated for
consumption; copying files for long-term storage is the operator's choice, with
no application retention/archive lifecycle added here.

## Implementation sequence and verification

1. Shared store values/forcing persistence and JSON manifest format; prepared recipe/input
   validation and frozen working values. Unit tests reject malformed version,
   escaping paths, hash mismatch, absent artifacts and inconsistent masks.
2. Strict request and pure preparation join. Exercise full library and CLI on
   synthetic canonical inputs without MGB/Python. Tests cover unknown keys,
   selectors/replacements, correction-asset mismatch, local timezone metadata,
   noncontiguous interval join, mismatched basin order and missing rainfall.
3. Input serialization and output/reference conversion unit tests live in
   `src/model/input.rs` and `src/model/output.rs` under `#[cfg(test)] mod tests`,
   where private/pub(crate) helpers are callable. External integration tests call
   public prepare/execute_prepared and inspect retained files, rather than importing
   model-private APIs. Use small fixtures:
   two times×two basins [1,2;3,4] must produce the four little-endian f32 words
   in that order; malformed length fails. Verify text metadata edits retain
   non-ASCII/comment bytes and unrelated fields. Test both segments, raw missing,
   valid negative station level, unknown/zero/signed offsets, and two stations
   sharing one mini with distinct offsets. Quantization tests cover offsets
   0.609 m and 0.832 m against the same unquantized level (e.g. 1.234 m gives
   referenced 62 cm and 40 cm with ties-to-even), exact positive/negative half-cm
   ties (0.125 m→12 cm, 0.375 m→38 cm, −0.125 m→−12 cm, −0.375 m→−38 cm),
   just-below/above-half boundaries, the actual f32→f64 precision boundary, i32
   minimum/maximum rounded values and overflow on either side. Assert i32
   array/mask round trips, missing payload zero versus valid zero, invalid payload
   rejection, unknown offsets and nonfinite input errors. Include a selected station
   with no offset: its identity/order persists, mini/offset/level payloads are zero
   with all respective masks invalid; raw basin results remain complete. Reject
   unknown selected station identities and invalid present mini mappings. Verify referenced values
   come from unquantized metres rather than rounded raw cm and original raw
   engine files remain byte-identical. No AR(1).
4. Process/coordinator and per-run outcome recording. Add the std-only standalone source
   `tests/support/engine_helper.rs`; an integration-test helper compiles it once
   per integration target with the pinned `rustc` into a retained temporary
   directory. Select this binary through the existing engine executable request.
   It reads an explicit per-test control file from its private Input and writes
   predetermined fixture streams for success/nonzero/malformed-output modes.
   It is not a hydrological simulation, ships no src/bin target and requires no
   Python/shell or product flags solely for tests; test four concurrent isolated
   scenarios, no shared Input mutations, siblings finishing after failure,
   empty logs/large logs, spawn failure, lock contention from another process,
   independent failed/completed runs, and injected manifest write/finalization
   failures, including interruption before rename and malformed/truncated JSON. Verify the generated JSON manifest is readable by GUI consumers, contains
   exact ordered instruction values after catalog edits, and references generated
   outputs without a workspace registry. Exercise missing regeneration dependencies
   as an explicit failure, not provider fallback. No save/retention flag is required.
   Pure decoder tests use exact fixture bytes, not an imitation full hydrological
   engine. Architecture tests reject model→app/adapters/analysis. Model→store is allowed,
   so the import gate cannot prohibit a particular store lifecycle function;
   code/behavior review verifies that app coordinates outcomes and model only
   stages outputs. Import direction alone does not establish ownership.
5. Update `tests/architecture.rs` facade allowlist for prepare/execute_prepared,
   request/report/error names (plus normalization names from step 2b), root
   declarations and explicit same-owner exports. Add positive interface fixtures
   and retain forbidden-edge/foreign-re-export/unknown-attribute negative cases.
   Existing checker already recognizes store/analysis/adapters owners; do not
   weaken unsupported generated-code/macro checks for new modules. Use ordinary
   supported derives and explicit crate::<owner> references.
6. Add and run the explicitly requested saved-case acceptance below. Compare
   canonical joined forcing before engine work, then engine bytes/raw streams,
   then full decoded arrays and station references with step 1. Repeat execution
   from the same prepared input after deleting/changing the original mutable
   source catalogs/reviews/offsets/baseline; require retained values and results
   stable under the qualified engine conditions. These destructive source edits
   apply only to disposable acceptance copies.

```sh
cargo run -- operation prepare --workspace /path/to/disposable-rs --request /path/to/rs-request.toml --output /path/to/prepared-rs --json
cargo run -- operation execute --workspace /path/to/disposable-rs --prepared /path/to/prepared-rs --json
MGB_RS_REFERENCE=/workspace/regression-rs-hourly cargo test --locked --test rs_saved -- --ignored --nocapture
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test architecture
```

The real-case test consumes only explicit consolidated reference paths and retained
engine artifacts; missing prerequisites fail visibly. Routine tests remain
regional-data/provider/engine independent. Use step 1 full-array structural/mask
checks and exact numerical comparison policy; no tolerance hides a
shifted timestamp, packed export discrepancy or lost forecast segment.

Implementation updates `docs/contracts/operations.md` for supported request,
prepared/per-run storage/API/output behavior, README usage/navigation,
architecture's durable decisions and [migration status](../migration/summary.md).
Keep Linux evidence and Windows limits explicit. Retain this specification until
all requested preparation/execution/run-artifact acceptance is complete.
