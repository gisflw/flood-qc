# Saved preparation and execution

The shared library exposes `prepare(PreparationRequest)` and
`execute_prepared(ExecutionRequest)`, also available through the CLI:

```sh
mgb-ops operation prepare --workspace /regional/workspace --request request.toml --output prepared --json
mgb-ops operation execute --workspace /regional/workspace --prepared prepared --json
```

Workspace, request, output and prepared arguments resolve from the invocation
directory. Paths inside the request resolve from the selected workspace.
Preparation creates a new directory and never invokes MGB. Execution uses retained
prepared dependencies, independently of subsequent changes to their source files.
A new preparation is needed to select different data or settings.

## Request

The separate UTF-8 TOML request is strict at every table: unknown keys fail.
Inspection's `config/workspace.toml` remains separate. This small example selects
saved observations and a zero-rain forecast; replace paths and times for the
selected regional baseline:

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
path = "data/observations.sqlite"
provider_codes = ["ana"]
station_ids = []
series_ids = []
replacement_ids = []

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
station_ids = []

[engine]
executable = "mgb_runner/MGB_Inercial_Prev_RSH"
input_dir = "mgb_runner/Input"
omp_threads = 1

[forecasts]
catalog = "data/forecasts.sqlite"

[[scenarios]]
id = "zero"
kind = "zero"
```

All shown scalar settings and tables are required. `series_ids` may be omitted;
replacement IDs must be explicit, including `[]` to select no reviews. At least
one provider or station selector is required. See the
[observation contract](observations.md) for ranking and ordered replacements.
QC findings do not automatically create reviews. Output level station IDs are
explicit and ordered; `[]` requests no station columns. Unknown stations fail.

Times are local `YYYY-MM-DDTHH:MM:SS`, hour aligned, in the supplied IANA timezone.
Only 3600-second steps are supported. Historical and forecast axes are contiguous;
reference equals the final historical endpoint, followed by the first forecast
endpoint one hour later. Ambiguous/nonexistent local labels and offset transitions
fail. Grid bounds and resolution, IDW settings, QC thresholds and thread count
are validated. Engine-bound rainfall must be present, finite and nonnegative.

Scenario IDs are unique and match `[A-Za-z0-9][A-Za-z0-9_-]{0,63}`. At least one
scenario is required. `kind` selects:

| Kind | Asset | Corrections |
| --- | --- | --- |
| `zero` | Omitted | Omitted |
| `raw` | Required `asset_id` | Omitted or `correction_ids = []` |
| `corrected` | Required `asset_id` | Required nonempty ordered `correction_ids` |

IDs resolve through the explicitly selected forecast catalog. Duplicate/unknown
instructions and cross-asset corrections fail. Zero scenarios need no forecast
asset or catalog file. See the [forecast contract](forecasts.md) for normalization
and correction semantics. Baseline Input must contain compatible geometry,
metadata, calibration and initial-state dependencies; preparation does not infer
state compatibility or generate a new hot start.

## Prepared dependencies and run files

Preparation freezes selected canonical observation rows, ordered replacement and
correction values, station metadata/offsets, selected forecast source/native files,
request bytes, numerical settings, baseline Input and executable. Source SQLite
subsets are read consistently rather than copied from an open WAL file. Canonical
forcing/checkpoints retain f64 precision. A version-1 `manifest.json` records the
recipe and input paths, kinds, sizes and SHA256 identities.

Manifest fields are `schema_version`, `state`, `recipe`, `inputs`, `scenarios`,
`created_at`, `finished_at` and `error`. State is `prepared`, `running`, `failed`
or `completed`. The recipe contains the resolved request document, full selected
station/series and instruction records, historical/forecast counts, selected
output stations, offsets and thread count. Each input reference has `path`,
`kind`, `size` and `sha256`; paths are relative to the artifact directory.
Scenario records preserve request order and contain `id`, `state`, timestamps,
`exit_code`, `signal`, structured `error`, `stdout`, `stderr`, `basin_result`
and `station_result`. Pending outcome fields are null. Errors contain `stage`,
`message` and `log_paths`. Manifest timestamps use operation-local labels;
execution logs additionally record monotonic `elapsed_seconds`.

The prepared tree contains `request.toml`, `sources/`, `baseline/Input/`,
`baseline/engine`, `forcing/` and `checkpoints/`. Each selected scenario has a
combined historical/forecast `forcing/<id>.nc`. The original baseline and source
files remain separate from writable engine inputs. Symlinks and escaping retained
paths fail. Files are closed and synced before atomic finalization to a new
destination on Linux. Failed staging is retained with diagnostics.

Execution takes a nonblocking OS lock on `<workspace>/.operation.lock`, held until
children finish and outcome recording ends. Concurrent execution in that workspace
returns `Busy`; the lock file remains present and its existence alone means
nothing. Each invocation reserves a new `data/runs/<run-id>/` containing:

```text
manifest.json
snapshot/manifest.json
snapshot/...                 verified frozen preparation
scenarios/.history/...       shared history-only execution
scenarios/<id>/Input/         private writable baseline + generated inputs
scenarios/<id>/Output/        original engine streams
scenarios/<id>/logs/          stdout.log, stderr.log, execution.json
results/<id>/basin.nc
results/<id>/station.nc
```

The root manifest describes current outcomes; the snapshot manifest preserves the
prepared recipe. Input references are rebased into `snapshot/`. No workspace run
database, official-run pointer, publication or retention policy is added.
Copying only a manifest records instructions; it cannot restore missing bytes.

Scenarios run in isolated directories with direct executable invocation, an
explicit working directory, and an environment containing only `MGB_INPUT_DIR`,
`MGB_OUTPUT_DIR` and `OMP_NUM_THREADS`. Stdout and stderr go directly to separate
files. Execution records include command, environment, status and elapsed time.
Every launched child is joined; a failed scenario preserves healthy siblings'
results and diagnostics. A run reports completed only after all selected scenarios
and required results validate and outcome persistence succeeds. Otherwise it
reports failed or returns a structured operation error identifying retained paths.

Interrupted runs can remain unfinished and children can survive whole-process
termination. Automatic recovery and process-tree cleanup are outside this contract.
Existing runs and mutable regional sources are never cleared or overwritten.

## Engine and canonical results

CHUVABIN is little-endian f32, time outermost and basin innermost, without headers
or record markers. MINI.gtp supplies actual mini IDs, order and centroids.
Metadata edits preserve unrelated baseline content and calibration. Every private
Input carries the retained baseline state; engine-written state is not copied back.

The qualified decoder keeps four original streams separate. A history-only run
recovers levels overwritten by the engine's forecast prefix; historical flow and
the unaffected level suffix must agree across scenarios. Labels follow the
supplied application axis, with rainfall bounds retained independently. Exact
stream lengths and finite values are validated.

Basin NetCDF stores precipitation in f64 mm, flow in f64 `m3 s-1`, raw level in
signed i32 cm and separate u8 validity masks. `time_segment` is 0 for history and
1 for forecast. Coordinates, ordered mini IDs, local times/bounds, timezone,
reference time, engine identity, snapshot identity and processing provenance are
explicit. There is no numeric packing.

Station results preserve requested station order and full-precision metre offsets.
Raw level is `round_ties_even(100 * engine_metres)`; station level is
`round_ties_even(100 * (engine_metres - offset_metres))`. The offset is applied
before rounding, independently of raw centimetres. Valid negative/zero levels and
the full i32 range are supported; overflow fails. Missing integer payloads are
exactly zero with mask zero. Unknown offsets retain station identity but have
unknown mini/offset/levels; explicit zero and signed offsets are valid. Multiple
stations can reference the same mini with different offsets. No AR(1) is applied.

JSON reports contain resolved artifact paths and scenario outcomes. CLI success
exits 0, operational failure exits 1 and argument errors exit 2. Library errors
identify invalid requests/preparations, busy workspaces, source/storage failures
or model failures without embedding binary or full observation contents.

Linux is the qualification platform. Windows, live providers, automatic retries,
new hot starts and hydrological validity remain unqualified. Scientific evidence
and the saved case's state caveat live with the
[regression reference](../../references/regression/rs-hourly/README.md).
