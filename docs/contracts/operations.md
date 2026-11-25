# Saved preparation and execution

## Choose an execution path

| Action | Start with | Publishes after success |
| --- | --- | --- |
| `operation prepare` | Workspace configuration and saved inputs | A reusable preparation at the requested path |
| `operation execute` | Saved preparation | A replacement active run |
| `draft run` or **Run experiment** | Draft with accepted inputs | A replacement active run |
| **Processing → New run → Run All** | Workspace configuration and a selected local reference | A replacement active run |
| `run replay` | Workspace attempt manifest | A replacement active run |

Use [operation drafts](drafts.md) to edit and preview a recipe, or the
[desktop run form](desktop.md#controlled-jobs) to run a saved recipe for a selected
reference. The shared library exposes `prepare(PreparationRequest)` and
`execute_prepared(ExecutionRequest)`, also available through the CLI.

```sh
mgb-ops operation prepare --workspace /regional/workspace --output prepared --json
mgb-ops operation execute --workspace /regional/workspace --prepared prepared --json
```

Use `--reference-time` to prepare a saved historical window. Set a repeatable
`--scenario` when choosing a scenario explicitly. Use `zero`, `ecmwf`, or `noaa`,
pin a cycle with `ecmwf@2026-07-20T00:00:00Z`, or use
`corrected:ecmwf@2026-07-20T00:00:00Z` for a cycle with stored corrections.

Workspace, request, output and prepared arguments resolve from the invocation
directory. Paths inside the request resolve from the selected workspace.
Preparation creates a new directory and never invokes MGB. Execution uses retained
prepared dependencies, independently of subsequent changes to their source files.
A new preparation is needed to select different data or settings.

Library callers can also use `prepare_typed(TypedPreparationRequest)` with a
`PreparationDocument` and the same workspace, invocation-directory and output
path rules. `replacements: None` resolves the document's replacement IDs;
`Some(values)` supplies the ordered replacement values and replaces those IDs.
`corrections` contains one entry per scenario in scenario order: `None` resolves
its correction IDs, while `Some(values)` supplies ordered correction values and
replaces those IDs. Use `None` for a zero scenario. Supplied records receive the
same validation as registered records and are not registered in source catalogs.
The prepared snapshot retains their actual values and a generated request, so it
can execute after the original sources are removed.

[Editable drafts](drafts.md) provide incomplete JSON recipes, explicit source
acceptance and unregistered previews over this same typed preparation workflow.

The desktop's [standalone run form](desktop.md#controlled-jobs) loads the same
workspace settings and supports explicit reference and scenario choices.
Acquisition-enabled runs resolve their forecast assets before freezing values.
The [acquisition contract](acquisition.md#run-all) covers Run All.

## Active run and rainfall cache

Each workspace has one active run after its first successful execution. The
active generation recorded in `operation.sqlite` selects the generation that supplies
model results, rainfall, and any processed inundation masks. Successful
execution publishes the whole generation together and reclaims the previous
generation. An unsuccessful or cancelled execution preserves the active run.
Concurrent executions use the same workspace lock. A later writable execution
retries cleanup of abandoned staging and superseded generations.

The active generation retains its completed manifest, canonical basin and station
results, selected engine evidence and logs, and optional flood sources and masks.
It also retains the observed and selected scenario working rainfall grids,
prepared basin forcing, and frozen MINI.gtp. Zero scenarios use the working grid
geometry and verified zero forecast intervals. Rainfall and basin summaries read
the selected generation. A missing cache returns no rainfall. Missing or corrupt
members produce findings.

The workspace retains only a small attempt manifest for past executions under
`runs/history/`. This includes the recipe, recorded input identities,
outcome, and diagnostic excerpts. To keep full results, export a bundle or copy
the active run to a user-owned location before another successful run.
Explicitly saved preparations remain reusable. See
[sharing and replay](saved-data.md#sharing-and-automation).

### Upgrade an existing workspace

Preview the migration with
`mgb-ops operation migrate-cache --workspace /region --keep-run <run-id>`.
The preview lists affected paths and estimates reclaimable space. Move any
manually kept runs or bundles out of `data/runs/` before applying. Repeat the
command with `--apply` to activate the selected completed run, preserve the
legacy manifests, and remove the legacy payloads. Legacy rainfall is adopted
when its forcing agrees with the selected run. A migrated run without matching
rainfall has no rainfall overlay until a successful execution supplies it.

Use `--inundation-donor <run-id>` to transfer existing products from a legacy
copy with matching model outputs. The migration validates provenance against
the workspace TIFFs at `data/source/HAND_Flood.tif` and
`data/source/catchments.tif`. Applying the migration also retires that donor's
legacy entry.

## Request

The separate UTF-8 TOML request is strict at every table: unknown keys fail.
Use an explicit request for a standalone preparation outside the normal workspace workflow. This small example selects
saved observations and a zero-rain forecast; replace paths and times for the
selected regional baseline:

```toml
schema_version = 2
time_zone = "America/Sao_Paulo"
reference_time = "2026-07-20T00:00:00"
observed_horizon_days = 10
forecast_horizon_days = 5

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
Version 2 authoring has one datetime: `reference_time`. `observed_horizon_days`
is a nonnegative integer; `forecast_horizon_days` is a positive integer. Hourly
steps are fixed. The first historical label is midnight on the reference date
minus observed days; the last is the reference. Forecast labels run from reference
plus one hour through reference plus forecast days plus one hour, inclusively.
Thus the example has 241 historical and 121 forecast labels; a non-midnight
reference adds historical labels since midnight. These are interval-end labels,
so covered rainfall begins one hour before the first label.
Historical and forecast axes are contiguous. In the example, the first history
interval starts at July 9, 23:00 and ends at the first history label, July 10,
00:00. July 20, 00:00 is both the final historical label and the reference.
The forecast labels run from July 20, 01:00 through July 25, 01:00.

```mermaid
flowchart TD
    accTitle: Interval endpoints in the example operation
    accDescr: Rainfall coverage starts at July 9 23:00. The first history label is July 10 00:00, the reference and final history label is July 20 00:00, and forecast labels run from July 20 01:00 through July 25 01:00.
    start["July 9, 23:00 · coverage starts"] --> history["July 10, 00:00 · first history label"]
    history --> reference["July 20, 00:00 · reference and last history label"]
    reference --> forecast["July 20, 01:00 · first forecast label"]
    forecast --> endLabel["July 25, 01:00 · last forecast label"]
```

Explicit boundary datetimes and `step_seconds` cannot be combined with version 2
day horizons.

Version 1 requests with `step_seconds = 3600` and explicit
`historical_first_end`, `historical_last_end`, `forecast_first_end`, and
`forecast_last_end` remain readable with their original exact coverage. The
reference must equal the final historical endpoint, followed by the first
forecast endpoint one hour later. The typed `PreparationDocument` API and retained
recipes keep this resolved version 1 representation; retained preparations,
requests and manifests are not rewritten. Draft import/export converts to version
2 only when expansion reproduces every original boundary and step exactly;
otherwise it retains version 1. No rounding to whole days occurs.
Ambiguous/nonexistent local labels and offset transitions
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
metadata, calibration and initial-state dependencies. See the [additional inputs
and state contract](saved-data.md#forcing-state-and-calibration) for optional
forcing, calibration selection and state capture.

## Execution lifecycle

Under the workspace lock, execution verifies a private preparation and runs a
history-only worker. A run that prepares its own inputs uses that preparation
directly. Execution of a saved preparation takes a verified private copy.
After history completes, each isolated forecast worker stages its own inputs
and starts MGB as soon as those inputs are ready. Execution joins the workers,
validates all requested scenarios, stages rainfall from the same prepared
inputs, and removes temporary inputs. It syncs the generation before committing
the results and rainfall together.

A cancellation before the commit stops and reaps owned engines. Failed
execution, rainfall preparation, or publication leaves the previous active
generation available. See [control and cancellation](#controlled-actions-and-cancellation).

## Prepared dependencies and run files

Preparation freezes selected canonical observation rows, ordered replacement and
correction values, station metadata/offsets, selected forecast source/native files,
request bytes (or a generated typed request), numerical settings, baseline Input
and executable. Source SQLite subsets are read consistently rather than copied
from an open WAL file. Canonical
forcing/checkpoints retain f64 precision. A version-1 `manifest.json` records the
recipe and input paths, kinds, sizes and SHA256 identities.

Manifest fields are `schema_version`, `state`, `recipe`, `inputs`, `scenarios`,
`created_at`, `finished_at`, `error` and optional `preparation_sha256`. The latter
identifies the preparation used by a new trial; older manifests omit it. State is `prepared`, `running`, `failed`
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
destination on Linux. Failed or cancelled staging is removed; an existing complete
preparation is preserved.

Execution takes a nonblocking OS lock on `<workspace>/.operation.lock`, held until
children and workers stop and file work finishes. Concurrent execution in that
workspace returns `Busy`. Each invocation stages a generation privately under
`data/cache/run/`. Finalization removes the verified preparation snapshot,
shared history inputs, and isolated worker inputs.

The active generation supplies validated `results/<id>/basin.nc` and
`station.nc`, the four original streams in `scenarios/<id>/Output/`, and
diagnostics. Small `results/<id>/engine-inputs/` files record the actual generated
metadata and rainfall digest. Supported engines provide an optional flood source
for each successful scenario. See [inundation processing](inundation.md).

`discover_trials(workspace)` returns the active root when present.
`read_trial(root)` validates canonical results and required engine streams
at an active or explicitly saved root. A manifest can replay a recipe using its
referenced inputs. An independently saved preparation or a bundle with a
preparation can reproduce an execution after the original sources are removed.
Execution reports expose `run_root` for current results and `attempt_manifest`
for the durable recipe. Automation job status keeps the durable reference.

Scenarios run concurrently in isolated directories with direct executable
invocation, an explicit working directory, and only `MGB_INPUT_DIR`,
`MGB_OUTPUT_DIR` and `OMP_NUM_THREADS` in the environment. Stdout and stderr go
directly to separate files. Execution records include command, environment,
status and elapsed time. Every launched child is waited for and every worker is
joined. Failure returns a structured operation error. `BatchFailed` carries
per-scenario status. Historical diagnostics are available in the attempt manifest.

## Controlled actions and cancellation

Use a fresh `OperationControl` per action with `prepare_controlled`,
`Draft::prepare_controlled`, `execute_prepared_controlled` or `run_draft`. The
ordinary preparation/execution functions use the same workflow with a fresh
control. `run_draft` prepares current accepted draft values and executes while
holding the workspace lock throughout. It removes its own temporary preparation;
Prepare only saves a reusable preparation and stops before launching MGB.

Cloned controls expose `cancel()`, `is_cancelled()` and the latest `progress()`.
Progress identifies preparation, history, scenarios and finalization, includes the
attempt path when known and per-scenario state, and leaves unknown fractions
absent. `tail_log(path, max_bytes)` reads at most 65,536 trailing bytes without
loading the entire file. JSON stdout contains the result, not progress text.

CLI SIGINT requests cancellation. Controlled cancellation checks preparation
boundaries, stops and reaps owned engines, joins concurrent workers, and finishes
cleanup under the lock. Cancellation before the final commit records the
cancellation reason in the attempt manifest and keeps the previous active run.
Cancellation after the commit preserves success. After hard process termination,
the next writable execution cleans up abandoned cache stages. Engine-created
descendant processes are outside the normal cancellation guarantee.

## Engine and canonical results

CHUVABIN is little-endian f32, time outermost and basin innermost, without headers
or record markers. MINI.gtp supplies actual mini IDs, order and centroids.
Metadata edits preserve unrelated baseline content and calibration. Every private
Input carries the retained baseline state. An explicit qualified state request
retains state under the successful scenario.

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

Linux is the qualification platform. Windows, live providers, warm-state
consumption and hydrological validity remain unqualified. Scientific evidence
and the saved case's state caveat live with the
[regression reference](../../references/regression/rs-hourly/README.md).
