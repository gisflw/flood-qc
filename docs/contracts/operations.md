# Saved preparation and execution

## Run from a workspace

Use **Processing → New run** in the desktop or run the corresponding CLI commands.

```sh
mgb-ops operation run-all --workspace /region --json
mgb-ops operation prepare --workspace /region --output /region/runs/prepared --json
mgb-ops operation execute --workspace /region --prepared /region/runs/prepared --json
```

**Run All** acquires configured inputs, freezes the run inputs, executes MGB, and
publishes the completed generation. **Save preparation** uses saved inputs and
stops before execution. A saved preparation retains everything needed to execute
after its original sources change. Relative command paths resolve from the
invocation directory. Paths in workspace settings resolve from the workspace.

Override the hour with `--reference-time` and the windows with
`--lookback-days` and `--forecast-days`. Local labels use the workspace time
zone and whole hours. History starts at midnight on the reference date minus
the lookback days and ends at the reference hour. Forecast labels start one hour
after the reference and end at the reference plus the forecast days plus one
hour. Rainfall labels mark interval ends. Ambiguous or nonexistent local labels
and time windows crossing a timezone offset change fail validation.

## Scenarios

With no overrides, the run selects zero future rain and the latest usable raw
cycle for each provider, model and product in `operation.sqlite`. Zero comes
first. Eligible cycles begin at or before the run reference and cover its
forecast window. The catalog can supply cycles from disabled acquisition
providers. Automatic raw scenario IDs use the provider, model and product
without the cycle for stable scheduled reporting. Run All resolves the
automatic set after acquisition and reports
unavailable providers in the acquisition outcome.

Repeat `--scenario` to supply an explicit, ordered selection. Use `zero`,
a provider such as `ecmwf` when it identifies one product, or a full
`provider/model/product` label. Append `@UTC-cycle` to pin a cycle. For a
corrected scenario, pass the cycle and ordered edits as one argument.

```sh
mgb-ops operation run-all --workspace /region \
  --reference-time 2026-07-20T00:00:00 \
  --scenario zero \
  --scenario ecmwf/ifs/fc@2026-07-20T00:00:00Z \
  --scenario '{"kind":"corrected","forecast":"ecmwf/ifs/fc@2026-07-20T00:00:00Z","corrections":[{"t0_hour":6,"t1_hour":18,"multiplication_factor":1.1}]}'
```

Each correction supplies `t0_hour` and `t1_hour`, measured from its forecast
cycle. Pixel shifts (`shift_lat_pixels`, `shift_lon_pixels`) and
`rotation_deg` default to zero. The positive `multiplication_factor` defaults
to one. New run can preview and reorder corrections for a pinned forecast.
The manifest retains the values in their applied order. Missing explicit
selections stop the run before execution. All scenarios use the same effective
[observation history](observations.md#selection-reviews-and-preparation).

Library callers open an `OperationalRun` for a workspace, set typed
`RunParameters`, and use `prepare_saved` or `run_all_report`. The same
`RunScenario` values are used for selection and preview.

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

The database records each attempt and its status.
The active generation keeps its manifest under `cache/run/`. Retiring it
archives the manifest under `runs/history/` and reclaims its results. Failed
attempts retain their manifest and bounded diagnostics. Export a bundle before
replacement to keep full results. Keep a [saved preparation](#prepared-dependencies-and-run-files)
for exact reruns.

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
correction values, station metadata and offsets, selected normalized forecast
grids, numerical settings, baseline Input
and executable. Source SQLite subsets are read consistently rather than copied
from an open WAL file. Canonical forcing and checkpoints retain f64 precision.
The generated `manifest.json` records resolved settings, scenario inputs and
outcomes, and the files needed to validate the snapshot. Forecast correction
values are recorded in this manifest. Derived grids carry their resulting
rainfall values.

The prepared tree contains `sources/`, `baseline/Input/`, `baseline/engine`,
`forcing/` and `checkpoints/`. Each selected scenario has a combined
historical/forecast `forcing/<id>.nc`. The baseline remains separate from
writable engine inputs. Preparation stages a new destination and removes its
staging files on failure or cancellation.

Execution takes a nonblocking OS lock on `<workspace>/.operation.lock`, held until
children and workers stop and file work finishes. Concurrent execution in that
workspace returns `Busy`. Each invocation stages a generation privately under
`cache/run/`. Finalization removes the verified preparation snapshot,
shared history inputs, and isolated worker inputs.

The active generation supplies validated `results/<id>/basin.nc` and
`station.nc`, the four original streams in `scenarios/<id>/Output/`, and
diagnostics. Small `results/<id>/engine-inputs/` files record the actual generated
metadata and rainfall digest. Supported engines provide an optional flood source
for each successful scenario. See [inundation processing](inundation.md).

`discover_trials(workspace)` returns the active root when present.
`read_trial(root)` validates canonical results and required engine streams
at an active or explicitly saved root. A saved preparation or a bundle with a
preparation supports execution after the original sources are removed.

Scenarios run concurrently in isolated directories with direct executable
invocation, an explicit working directory, and only `MGB_INPUT_DIR`,
`MGB_OUTPUT_DIR` and `OMP_NUM_THREADS` in the environment. Stdout and stderr go
directly to separate files. Execution records include command, environment,
status and elapsed time. Every launched child is waited for and every worker is
joined. Failure returns a structured operation error. `BatchFailed` carries
per-scenario status. Historical diagnostics are available in the attempt manifest.

## Controlled actions and cancellation

Use a fresh `OperationControl` for each preparation, Run All, or controlled
execution. Run All holds the workspace lock from acquisition through publication
and removes its temporary preparation on completion.

Cloned controls expose `cancel()`, `is_cancelled()` and the latest `progress()`.
Progress identifies acquisition, preparation, history, scenarios and finalization, includes the
attempt path when known and per-scenario state, and leaves unknown fractions
absent. `OperationControl::with_observer` also delivers milestones, download
updates and engine log chunks to a callback that can be called from concurrent workers.
`tail_log(path, max_bytes)` reads at most 65,536 trailing bytes without loading the
entire file.

`operation prepare`, `operation execute`, `operation run-all` and
`run regenerate` print live run milestones. The CLI displays
engine output as the executable flushes it. Each line identifies the shared
history or scenario and the engine's stdout or stderr.
Text commands use stdout. Commands returning JSON use stderr for live output,
including `run regenerate` without `--json`. JSON reports
are single documents on stdout. If the console falls behind, its output names
the affected stream and the number of skipped bytes. Engine log files retain
their original bytes under the usual run retention rules.
See the [acquisition contract](acquisition.md) for the download display.

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
