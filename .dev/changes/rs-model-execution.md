# Step 3 — Integrate canonical inputs, execute MGB, and publish results

Status: implementation specification; capabilities and acceptance remain outstanding.

Implement preparation from canonical saved sources, reusable prepared inputs, real
MGB execution, full output decoding and complete-set publication. Dependencies are
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
| `src/app.rs`, `src/app/prepare.rs`, `src/app/operation.rs` | Re-export workflow types/functions; prepare.rs owns strict preparation request parsing/resolution, selection and join; operation.rs owns execution coordination and statuses. No SQLite SQL or binary decoding here. |
| `src/store/values.rs` | Shared types defined by forcing specs, plus PreparedManifest, ScenarioInputs, ResultSeries, StationResults and retained artifact descriptors. Pure values, no required connection/path lookup. |
| `src/store/operation.rs` | Operation SQLite schema/queries, snapshot copies/manifests, durable artifact registration and official-set transaction. |
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
// ExecutionReport { operation_id: i64, attempt_id: i64, status: BatchStatus,
//   official: bool, scenarios: Vec<ScenarioExecution>, artifact_root: PathBuf }
```

Reports/errors implement Serialize and Display as appropriate. BatchStatus is
Completed/Failed; ScenarioExecution includes ID, status, optional exit
code, signal description where available, logs, result paths and structured
failure. A completed batch can still return publication failure with its attempt
ID/artifact root retained. Error variants: InvalidRequest{field,message},
InvalidPrepared{path,message}, Busy{workspace}, Source{path,message},
Store{message}, Model{scenario,stage,message,log_paths}, Publication{attempt_id,message}.
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
create_attempt, record_scenario, retain_snapshot, and publish_complete_set;
publication receives the exact requested scenario list and verified artifacts.

## Separate preparation request and CLI

Add `operation prepare --workspace W --request FILE --output DIR [--json]` and
`operation execute --workspace W --prepared DIR [--json]`. Workspace/request/output/
prepared CLI paths resolve from invocation_dir; resource paths *inside request
TOML* resolve from the selected workspace, matching existing resource semantics.
Existing workspace.toml remains an inspection configuration; this request is a
separate strict `deny_unknown_fields` structure at every nested table.

The following is the RS fixture template; step 1 may correct its explicitly
provisional forecast endpoint before accepting the fixture:

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
review_set_id = "rs-six-null"
# To choose none explicitly: omit review_set_id and set no_review = true.

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
native_artifact_id = "<ecmwf-native-artifact-id>"
[[scenarios]]
id = "noaa-raw"
kind = "raw"
native_artifact_id = "<noaa-native-artifact-id>"
[[scenarios]]
id = "ecmwf-corrected"
kind = "corrected"
native_artifact_id = "<ecmwf-native-artifact-id>"
correction_set_id = "rs-ecmwf-corrected"
```

This is a template: the one-off fixture converter writes actual native_artifact_id
values returned by canonical normalization (or its frozen-grid fixture import),
and actual correction-set IDs linked to those artifacts’ source identities.
Legacy asset IDs are provenance, not new content/normalization source IDs.
Replace the placeholder levels station list with explicit qualified selected
station IDs in the actual fixture. An explicit empty list means no station
outputs; never discover a changing default output station set during execution.
The forecast catalog correction set contains +8 latitude pixels, −12 longitude
pixels, 45° rotation, factor 1.5, hours 0–126, not degree translations.

Validate version, IANA zone, second precision/hour alignment, positive integral
step=3600 for this increment, increasing/contiguous intervals, reference equal to
historical_last_end, forecast_first_end=historical_last_end+step, finite bbox,
positive resolution/k/power, explicit review/no-review exclusivity and at least
one observed provider/station selector. Retain optional explicit series selection.
Reject ambiguous/nonexistent local endpoints and zone-offset transitions under
the shared unqualified-DST limit. No implicit days-to-count conversion is needed
when endpoints are supplied. Require nonempty unique scenario IDs matching
`[A-Za-z0-9][A-Za-z0-9_-]{0,63}` and at least one scenario. Zero rejects native_artifact_id/
correction fields; raw requires native_artifact_id and rejects correction;
corrected requires native_artifact_id plus correction_set_id. Require the selected
artifact kind to be native and artifact.source_id == correction_set.source_id.
QC scalar fields are required, finite and retained: high_threshold_mm>=0,
0<=sequence_lower_mm<sequence_upper_mm, sequence_min_length>=1; sequence spacing
is the selected hourly step. Persist all four values, not only a policy name.
Validate each selected level station exists,
offset mini references geometry, and all source/baseline paths are regular files
or directories as required. Reject nested/escaping manifest paths and symlinked
entries in retained trees. Destination must be new; do not overwrite preparation.

## Preparation join and retained prepared contract

1. Resolve request and validate source metadata. Call store read_selection once to load eligible candidate-series metadata,
   their in-window canonical samples, sources/reviews/stations/selected offsets;
   analysis ranks candidates in memory without a database connection. Ask model to parse
   MINI.gtp into BasinGeometry (actual Mini IDs/order/centroids); app and analysis
   do not parse engine-specific geometry and GPKG decoding is not a prerequisite. Copy the selected
   canonical rows/actual values and provenance to a new preparation staging tree.
   Use a SQLite read transaction for a consistent selected view; materialize
   canonical subset through store, never copy an open SQLite/WAL file blindly.
2. App calls `analysis::prepare_observed_forcing` once with ObservationInputs,
   IntervalAxis, RegularGridSpec, BasinGeometry, IdwSettings and QC policy from
   step 2a. Detection findings never authorize new reviews. Retain effective-row
   and QC checkpoints alongside raw source/review subset.
3. For each forecast scenario, resolve native_artifact_id to its native GridSeries
   and source identity, validate correction-set/source equality and load correction
   records from the selected catalog and call step 2b's `prepare_forecast`;
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
   bytes and a complete manifest inventory with relative paths/size/SHA256/kind.
   Preserve baseline Input/executable bytes as prepared dependencies so reuse can
   work after mutable sources disappear. Preparation does not invoke MGB.
   Flush/close and sync each file; finalize by same-filesystem staging rename to
   the new destination, sync its parent. Keep failed staging with diagnostics;
   a previously completed preparation is never replaced.

Prepared directory schema v1:

```text
manifest.json             schema/version, source identities, inventory, scenario order
request.toml              exact accepted request bytes
resolved.json             local axes, settings, selected IDs, engine identity
sources/observations.sqlite   selected raw/review/station/offset values
sources/forecast.sqlite       selected catalog/correction rows
sources/native/<source-id>.nc retained canonical native values
baseline/Input/...        complete selected baseline, unchanged state/calibration
baseline/engine           retained executable with executable mode preserved
forcing/observed.nc
forcing/<scenario-id>.nc  complete historical+forecast BasinForcing
checkpoints/...           effective rows/QC/working/transformed grids
```

Inventory all files; manifest itself is hashed by its caller, avoiding recursive
self-hashing. No hard links to mutable sources. Copies become authoritative input
values; record their hashes after copying, not only pre-copy source hashes.
A source changing while selected/copying must fail consistency validation rather
than produce mismatched metadata and retained bytes. Reread/verify selected
native artifact hashes against catalog identities. Execution verifies the entire
prepared inventory and copies it at startup; no provider/history lookup remains.

## Execution filesystem and operation SQLite

Use `<workspace>/data/operations.sqlite` for operation metadata, with foreign_keys
ON, journal_mode=DELETE, synchronous=FULL; schema version via user_version=1.
All SQL binds parameters. One connection on coordinator thread, short
transactions only; model worker threads never write the catalog.

```sql
CREATE TABLE operation (
 operation_id INTEGER PRIMARY KEY,
 prepared_sha256 TEXT NOT NULL, operation_timezone TEXT NOT NULL,
 reference_time TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE attempt (
 attempt_id INTEGER PRIMARY KEY,
 operation_id INTEGER NOT NULL REFERENCES operation,
 state TEXT NOT NULL CHECK(state IN ('snapshotting','running','failed','completed')),
 root_path TEXT UNIQUE NOT NULL, started_at TEXT NOT NULL, finished_at TEXT,
 error_json TEXT
);
CREATE TABLE scenario (
 attempt_id INTEGER NOT NULL REFERENCES attempt,
 scenario_id TEXT NOT NULL, position INTEGER NOT NULL CHECK(position>=0),
 state TEXT NOT NULL CHECK(state IN ('pending','running','failed','completed')),
 exit_code INTEGER, started_at TEXT, finished_at TEXT, error_json TEXT,
 PRIMARY KEY(attempt_id,scenario_id), UNIQUE(attempt_id,position)
);
CREATE TABLE artifact (
 artifact_id INTEGER PRIMARY KEY,
 attempt_id INTEGER NOT NULL REFERENCES attempt,
 scenario_id TEXT, kind TEXT NOT NULL, relative_path TEXT NOT NULL,
 sha256 TEXT NOT NULL, byte_length INTEGER NOT NULL CHECK(byte_length>=0),
 durable INTEGER NOT NULL CHECK(durable IN (0,1)),
 UNIQUE(attempt_id,relative_path),
 FOREIGN KEY(attempt_id,scenario_id) REFERENCES scenario(attempt_id,scenario_id)
);
CREATE TABLE official_set (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1),
 attempt_id INTEGER UNIQUE NOT NULL REFERENCES attempt,
 published_at TEXT NOT NULL
);
```

Operation timestamps use operation-local ISO labels with operation timezone on
operation; process durations use monotonic elapsed time stored in execution.json.
Paths are relative to workspace/attempt as appropriate and cannot escape roots.
Schema/version mismatch fails before writing. Re-execution may create a new
operation row; no content-deduplication subsystem is required.

```text
data/runs/<attempt-id>/
  snapshot/                 independent retained prepared tree
  scenarios/<id>/Input/     independent writable baseline copy + serialized inputs
  scenarios/<id>/Output/    original raw streams, never concatenated in place
  scenarios/<id>/logs/stdout.log, stderr.log, execution.json
  results/<id>/basin.nc
  results/<id>/station.nc
  manifest.json             complete staged/final artifact inventory
```

Attempt ID allocation and root reservation occur after workspace lock acquisition.
Use create_dir with collision failure; never clear a pre-existing root. Retain
snapshot before starting children, record selected engine and environment
identity. Complete-set publication is the SQLite official_set pointer; readers
query it then immutable artifact paths. No second authoritative symlink/current
JSON pointer is introduced.

## Lock, child processes and failures

Open `<workspace>/.operation.lock` read/write/create without truncation; call
`File::try_lock()` and retain that File until all children finish and publication
commits/fails. Map `std::fs::TryLockError::WouldBlock` to Busy; other lock errors fail. Do not unlink the
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
Automatic stale-attempt recovery, startup state rewriting and process-tree
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
Step 1 must supply its resolved complete-level strategy before implementing that
runtime path; neither interception nor an extra engine run is prescribed here.
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
processing_json plus snapshot SQLite. With raw basin level L metres and offset O,
station level=round_ties_even(100*(L−O)) centimetres. A missing raw level or
absent offset gives unknown mini mapping and offset: mini_id=0/mini_valid=0,
offset_m=0/offset_valid=0 and level=0/level_valid=0 at every time. Do not invent
a supplying basin. Present offsets require a validated mini_id>0 with mini_valid=1,
and finite offset_m with offset_valid=1; explicit zero and negative offsets
are valid. A
negative finite referenced level is valid. Multiple station IDs may select the
same mini and produce different results. Raw basin levels remain unchanged;
apply fixed offset over both segments and every scenario. AR(1) is excluded.

## Durable complete-set publication

1. Under the workspace lock, insert operation/attempt/scenario records; retain
   snapshot and inventory; transition to running only once snapshot is durable.
2. Record each scenario's status and retained diagnostics. With any failure,
   set attempt failed, finished_at/error_json and leave official_set untouched.
3. On all successes, close/sync result/raw/log/manifest files and sync affected
   directories on Linux. Store validates exact requested scenario membership,
   completed states, all required artifact kinds, inventory paths/sizes/hashes,
   and durable=1. Ancillary raw outputs/diagnostics remain retained as inventory.
4. In one BEGIN IMMEDIATE transaction, recheck complete membership, register final
   artifact references, set attempt completed and finished_at, and UPSERT the
   singleton official_set pointer to this attempt. Commit with FULL sync. Do not
   delete previous official artifacts; no filesystem rename is needed after commit.
5. If transaction fails, rollback leaves previous official_set intact; retain new
   files and record publication failure if the database remains writable. If a
   process dies before commit, retained files are unreferenced/unfinished; after
   commit, the new official set resolves entirely to already durable files.

Readers accept only the complete set referenced by official_set; staged individual
scenario success is never official. Test filesystem/database failure injection
before durability, before pointer update, and before commit. Preserve all attempt
evidence; cleanup/retention policy is deferred. Linux directory-sync acceptance is
required; Windows durability remains unverified, not implied by successful build.

## Implementation sequence and verification

1. Shared store values/forcing persistence and operation schema; prepared manifest
   validation and selected-value retention. Unit tests reject malformed version,
   escaping paths, hash mismatch, absent artifacts and inconsistent masks.
2. Strict request and pure preparation join. Exercise full library and CLI on
   synthetic canonical inputs without MGB/Python. Tests cover unknown keys,
   selectors/reviews, source-correction mismatch, local timezone metadata,
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
4. Process/coordinator and store publication. Add the std-only standalone source
   `tests/support/engine_helper.rs`; an integration-test helper compiles it once
   per integration target with the pinned `rustc` into a retained temporary
   directory. Select this binary through the existing engine executable request.
   It reads an explicit per-test control file from its private Input and writes
   predetermined fixture streams for success/nonzero/malformed-output modes.
   It is not a hydrological simulation, ships no src/bin target and requires no
   Python/shell or product flags solely for tests; test four concurrent isolated
   scenarios, no shared Input mutations, siblings finishing after failure,
   empty logs/large logs, spawn failure, lock contention from another process,
   failed set retaining prior official pointer, and injected publication failures.
   Pure decoder tests use exact fixture bytes, not an imitation full hydrological
   engine. Architecture tests reject model→app/adapters/analysis. Model→store is allowed,
   so the import gate cannot prohibit a particular store publication function;
   code/behavior review verifies that app alone requests publication and model
   only stages outputs. Import direction alone does not establish ownership.
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
MGB_RS_REFERENCE=/workspace/regression-rs-hourly-r2 cargo test --locked --test rs_saved -- --ignored --nocapture
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test architecture
```

The real-case test consumes only explicit revised reference paths and retained
engine artifacts; missing prerequisites fail visibly. Routine tests remain
regional-data/provider/engine independent. Use step 1 full-array structural/mask
checks and measured stage-specific tolerance decisions; no tolerance hides a
shifted timestamp, packed export discrepancy or lost forecast segment.

Implementation updates `docs/contracts/operations.md` for supported request,
prepared/operation storage/API/output behavior, README usage/navigation,
architecture's durable decisions and [migration status](../migration/summary.md).
Keep Linux evidence and Windows limits explicit. Retain this specification until
all requested preparation/execution/publication acceptance is complete.
