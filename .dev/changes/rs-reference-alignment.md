# Step 1 — Qualify the saved RS reference and canonical boundaries

Status: implementation specification; revised reference and acceptance are outstanding.

This step produces an independently reproducible corrected reference, not production
Rust processing. Its deliverables unblock [observations](rs-observed-forcing.md),
[forecasts](rs-forecast-forcing.md), and [execution](rs-model-execution.md). Linux
is the acceptance platform; Windows remains unverified. Preserve the executable,
geometry, calibration and supplied initial-state bytes. Acquisition, new hot starts,
AR(1), reports and a combined run-all command remain outside this increment.

## Evidence already inspected

All Python paths below are relative to `/workspace/legacy_python/src/mgb_ops/`;
VB paths refer to Form8.vb in the [VB source map](../../references/vbnet_map.md).
The immutable baseline is [revision 1](../../references/regression/rs-hourly/README.md)
at `/workspace/regression-rs-hourly-r1`.

| Question | Concrete evidence and implementation consequence |
| --- | --- |
| Where does the 362-row timeline originate? | `utils/time.py::build_horizon_window` sets start to local midnight ten days earlier, forecast start to reference + one hour, and forecast count to five days × 24 + 1. At reference July 20 00:00 this produces 241 observed rows and 121 forecast rows, ending July 25 01:00. This is the captured Python convention, not independent proof of engine sample semantics. |
| What is the rainfall interval convention? | `assets/observed_precipitation.py::build_observed_precipitation_cache` queries local observations from the lower boundary and labels grids with interval ends (`start + step` through `end`). Capture requests UTC July 10 02:00 through July 20 03:00, giving first local interval end July 10 00:00 and lower bound July 9 23:00. `prepare_mgb_rainfall.py` builds observed and forecast matrices separately and concatenates them. |
| Is ordinary Python export equivalent to capture? | No. `model/export_mgb_outputs.py::discover_output_sources` reads only `QTUDO_Inercial_Atual.MGB` and `YTUDO.MGB`; `validate_source_lengths` requires those and CHUVABIN to have equal lengths. Capture explicitly concatenates 241 observed and 121 forecast rows into staging files before export. Rust must decode the separate streams; copying ordinary exporter file discovery would lose/fail the forecast portion. |
| What does VB do? | Form8 `SalvaHidrogramas_Binários_Inercial` and `SalvaCotagramas_Binários_Inercial` calculate observed and forecast lengths independently with inclusive `DateDiff + 1`, read separate Atual/Prev streams, and put the forecast after the observed segment. Its UI dates supply the bounds. It does not justify imposing a fixed 121-row horizon for every operation. |
| How are levels represented? | Python `write_output_netcdf` multiplies raw binary levels by 100 to export centimetres. Form8 reads Singles and subtracts `BIAS_NIVEL`; `CARREGA_BIAS_ESTAÇÕES` loads station code at split field 1, offset at 10 and basin at 11 from semicolon-separated `EstacoesNivel.txt`. Preserve each station record rather than VB's basin-indexed overwrite behavior. |
| Why is the old recovered level stream special? | `rewind-trace.txt` shows a seek to zero followed by writes on the observed-level descriptor. `recover-level.py` captures before that seek in a second zero run and checks complete observed flow plus the unaffected final 120 observed-level rows. This validates the saved adaptation for that capture; it does not establish the correct revised runtime strategy. |
| Which forecast metadata is actually involved? | Frozen `Previsao.meta` contains October 1 22:00/361 and a named `PrevisaoMINI_Estreito.mgb`. Capture replaces only its date/horizon with July 20 01:00/121. The retained `provenance/engine-open-trace.log` opens CHUVABIN at line 25 and Previsao.meta at line 152; it contains no open of that named forecast binary. Do not manufacture a second forecast input from the metadata filename. |
| What is known about state? | Frozen HOT.HOT is 1 and DATA.hot says August 28 2026 00:00. The retained diagnostic opens DATA.hot read/write/create; this is not proof that all CI files were consumed. Carry these files unchanged and retain the existing temporal/state caveat; this increment does not certify their hydrological validity. |

Sources must be pinned by actual commit and hash in the revised inventory. The
reference README's capture commit and the currently mapped Python commit differ;
use the captured revision for baseline reproduction and record any deliberate
source changes as a patch. No local Fortran source was found; do not represent a
recorded source-checkout identity as available executable source.

The captured fixture selects provider `ana`, bbox
`[-58.95016,-34.40217,-48.85759,-26.29721]`, a 0.1-degree working grid
including boundary cells, five nearest neighbours and inverse-distance power 2.
The explicit source IDs are `ecmwf.ifs.fc.20260720T000000Z.precipitation_grid`
and `noaa.gfs.fc.20260720T000000Z.precipitation_grid`. The corrected scenario
uses ECMWF, hours 0–126, translation +8 latitude pixels/−12 longitude pixels
(+2° north/−3° longitude on the 0.25° native grid), rotation 45°,
and multiplier 1.5; preserve the legacy transformation order established in
step 2b. The captured forecast uses 41 three-hour source intervals spanning
UTC July 20 03:00 through July 25 06:00. The six NULL reviews must be extracted
as exact station/time records rather than recomputed and automatically approved.

## Files to create and change

Create `/workspace/regression-rs-hourly-r2` separately, without hard links. The
following are planned regression-only files, not implemented commands today:

| File | Required contents/behavior |
| --- | --- |
| `case.json` | Version, operation timezone `America/Sao_Paulo`, reference local/UTC instant, explicit start/end/count policy, dt=3600, geometry order, providers, bbox, grid/IDW settings, four scenario definitions and correction instructions. No current-clock/default-provider resolution. |
| `frozen/` | Copies of r1 saved history, runner inputs/executable, normalized assets, provenance; add the saved July 20 00Z ECMWF/NOAA GRIB downloads and selected EstacoesNivel source. Preserve their source paths, sizes and SHA256. Never fall back to operational history or downloads. |
| `qualify.py` | Adapt r1 capture using explicit `--case`, `--output`, pinned legacy checkout; create fresh databases/work directories, rebase only disposable asset references, run preparation and isolated scenarios, retain raw outputs and logs, and collect checkpoints before packing. Assert settings/array shape instead of string replacement without parsed checks. |
| `timeline.json` | Per segment: rainfall lower/upper bounds and labels, corresponding binary row ranges, raw result row labels, source of the mapping and expected stream lengths. Include PARHIG and Previsao.meta parsed values; do not infer result length solely from file size. |
| `qualification.json` | Each discrepancy, source function/line or retained evidence path, selected correction and reason, remaining caveats; explicit disposition of the old interception/extra run. Separate established facts from unresolved output mapping. |
| `checkpoints.py` | Extract all stages below from explicit files into comparison fixtures, with dimensions, masks, units and provenance. No numerical transformation hidden inside the comparator. |
| `convert-observations.py` | Read frozen history read-only; write canonical observations/stations/review/offset SQLite using step 2a's versioned schema. Preserve six explicit NULL instructions and selected source identities. |
| `convert-forecasts.py` | Convert frozen normalized native grids/catalog/corrections to step 2b canonical fixtures; preserve UTC cycle, interval bounds, coordinate orientation and missingness. This seeds downstream tests independently of GRIB normalization. |
| `normalization-checkpoints.py` | Read the preserved original downloads with ecCodes and retain decoded/cropped cumulative fields and native increments without GRIB re-encoding. Also reproduce the legacy ECMWF crop/write/read boundary and compare it with frozen normalized fields. Keep re-packing differences separate from later Terrio decoding precision. |
| `compare.py` | Full-array capture/repeat, r2/r1 and Rust/r2 comparisons; emit JSON statistics and fail on structural/mask/numerical mismatch. Support stage selection to isolate the first divergence. |
| `manifest.json`, `expectations.json`, `comparison.json` | Complete retained file inventory/hashes; explicit expected arrays/timeline/settings; comparisons with first mismatch coordinate, max absolute/relative error, differing counts, mask counts and decision for each changed expectation. |

Update `references/regression/rs-hourly/README.md` with corrected reproduction and
provenance while retaining revision 1's distinct identity. Update migration status
only after actual qualification. Do not rewrite the historical r1 scripts or
claim this documentation work has produced r2 evidence.

## Qualification procedure

1. Read r1 `capture.py`, `recover-level.py`, `snapshot.c`, `evidence.py`,
   `compare.py`, raw stream sizes, parsed inputs, logs and comparisons together.
   Reproduce the source-derived row mapping in a table before choosing corrections.
   Trace `scenario_orchestrator.py`, `prepare_mgb_meta.py`,
   `prepare_mgb_rainfall.py`, `mgb_execution.py`, and `export_mgb_outputs.py`
   alongside Form8's active preparation and loaders. Use the existing real-engine
   evidence for source-consumption conclusions; no additional proof gate is added.
2. Resolve whether the engine's PARHIG NT is the combined or historical count and
   how Previsao.meta determines forecast rows, using the retained invocation,
   preparation, output stream lengths and timestamps. Record separately the
   interval-end forcing labels and instantaneous flow/level sample labels.
   Explain the five-day-plus-one convention and any corrected end date; do not
   silently trim/pad arrays to match 362 or relabel rows to make them compare.
3. Determine the applicable flag/configuration interpretation from active code
   and evidence rather than PARHIG's ICALIB comment alone (the saved value is 0
   despite separate forecast streams). Reconcile the normal exporter, capture
   staging and VB loader. Correct only the disposable preparation/assembly that
   the evidence identifies as wrong. Retain raw streams independently of decoding.
4. Where retained evidence cannot distinguish a timeline interpretation, run
   bounded disposable cases through the same `qualify.py` preparation functions:
   `--diagnostic horizon` compares forecast counts 120 and 121 while preserving
   the same historical forcing prefix; `--diagnostic metadata` changes only
   parsed Previsao.meta horizon/start consistently with the selected target axis.
   Retain per-case parsed values, stream sizes and first/last text-output dates.
   Choose the mapping whose sample labels, rainfall row consumption and separate
   output lengths agree; fail qualification if those disagree. These are
   discrepancy-resolution cases, not an additional source-consumption gate.
   Do not change calibration/state or select a winner merely because it resembles
   revision 1. Explicitly distinguish measured outcomes from proposed corrections.
5. Reassess the rewind using the complete trace and actual configuration. Prefer
   direct complete historical/forecast output when supported. Do not promise an
   interception, extra execution or executable patch as the revised solution.
   If correct configuration still cannot supply a complete required level stream,
   report the exact unresolved evidence and options for user decision before
   downstream execution depends on a recovery mechanism. The old recovered stream
   remains available for comparison without declaring it the production contract.
6. Freeze the resolved timeline/settings in case.json/timeline.json; run all four
   scenarios in a fresh capture and again in an independently copied repeat.
   Preserve process invocation, cwd, environment (including OMP_NUM_THREADS=1),
   executable identity, exit code, stdout/stderr and per-scenario input hashes.
   Compare each stage and explain every changed expectation from r1. A repeated
   result proves reproducibility, not correctness of the interpretation by itself.

## Shared canonical contracts

Store owns canonical `TimeInterval`, `IntervalAxis`, `GridSeries`, and `BasinForcing` values in
`src/store/values.rs`; schemas/persistence are private to store. Exact observation
DDL belongs to step 2a; forecast catalog/correction DDL belongs to step 2b;
operation/snapshot DDL belongs to execution. Avoid a competing schema in this step.

Canonical endpoints are operation-local `chrono::NaiveDateTime` values, with
`IntervalAxis.time_zone` naming the IANA zone. SQLite stores local ISO8601 labels;
NetCDF uses integer seconds relative to an operation-local origin and records
`operation_timezone`, without a UTC epoch shadow representation. Adapters convert
source UTC timestamps once; retain original UTC cycle/interval provenance.
Reject ambiguous/nonexistent local inputs and periods crossing unqualified
timezone-offset transitions rather than silently selecting an offset or merging
samples. The saved RS period has no such transition. Rainfall is interval total
in mm, never an unlabelled intensity; every field has explicit lower/upper bounds
and end labels. Flow is m³/s. Canonical level observations and raw/referenced
results are signed i32 centimetres, with separate validity masks in numerical
artifacts and NULL for missing SQLite samples. Offsets retain their supplied f64
metre precision. Model subtracts offsets from unquantized engine metre samples
before converting once to integer centimetres using nearest rounding with ties
to even and checked i32 range. Preserve original engine f32 metre streams and
convert legacy packed centimetres explicitly when comparing. Preserve absence
separately from zero and reject unexpected infinity/NaN outside declared masks.

Grid arrays are `[time,y,x]`, with longitude/latitude vectors, CRS, orientation,
footprint and source/working resolution recorded. Basin arrays are `[time,mini]`
in the selected MINI.gtp row order with explicit `mini_id`; station results are
`[time,station]` with stable station IDs and supplying mini IDs. Never equate
basin row index to mini ID without validation. Rainfall and flow checkpoints use
unpacked f64. Canonical level checkpoints use i32 centimetres and their masks;
retain unquantized engine levels separately to explain quantization boundaries
and distinguish them from legacy NetCDF packing. Persist original raw f32 streams.

SQLite fixtures retain source observations, stations, explicit review instructions
and station offsets; a separate SQLite retains forecast catalog/corrections.
Offsets are signed finite metres, one current record per station, with station FK,
mini ID and provenance. Zero is valid; absent is unknown. Multiple stations can
share a mini with different offsets. Model computes raw basin level minus offset
and retains both arrays. Observation-residual AR(1) is deferred.

## Checkpoints and downstream handoff

| Checkpoint | Exact retained data | Consumer |
| --- | --- | --- |
| Selected/effective observations | `fixtures/observations.sqlite`; selected source/series identities, raw and effective values, interval labels, coordinates, NULL reviews; `checkpoints/qc.json` with ordered threshold/sequence findings | Step 2a validates selection, detection and review independently |
| Native forecasts | `fixtures/forecast.sqlite` plus `fixtures/native/<source>.nc`; source cycle/product/message identity, original/canonical bounds, native coordinates and masks | Step 2b normalization acceptance |
| Corrected native forecast | `checkpoints/ecmwf-corrected-native.nc`; all transformed coordinates, values, bounds, masks, ordered instructions, before resampling | Step 2b correction acceptance; replace opaque NPZ-only expectations |
| Working grids | `checkpoints/observed-grid.nc` and per-scenario forecast grids; complete f64 values, coordinates, bounds, masks, interpolation/resampling settings | Both forcing tracks |
| Basin forcing | `checkpoints/observed-basin.nc`, per-scenario forecast/combined basin files; rainfall mm, bounds, segments, MINI order | Integration and serializer |
| Engine boundary | Per scenario PARHIG, Previsao.meta, CHUVABIN bytes, unchanged ancillary/state files, execution metadata and four original flow/level streams | Step 3 serialization/execution |
| Canonical results | Per-scenario unpacked raw basin flow/level and referenced station level NetCDF; explicit time/segment mapping and selected offset snapshot | Step 3 decoder and fixed-reference conversion |

Converters materialize each canonical fixture when its implementing step supplies
the store schema; do not let fixture conversion become a production legacy reader.
Step 2a and 2b may proceed independently after the timeline and common values are
qualified. Step 3 requires their complete canonical forcing plus qualified raw
output mapping, not merely successful prerequisite inspection.

## Reproduction and comparison interface

The implementation must make the following sequence work with explicit paths.
These are commands to build, not claims of runnable tooling today:

```sh
python /workspace/regression-rs-hourly-r2/qualify.py --case /workspace/regression-rs-hourly-r2/case.json --output /path/to/fresh/capture
python /workspace/regression-rs-hourly-r2/qualify.py --case /workspace/regression-rs-hourly-r2/case.json --output /path/to/fresh/repeat
python /workspace/regression-rs-hourly-r2/checkpoints.py --run /path/to/fresh/capture
python /workspace/regression-rs-hourly-r2/checkpoints.py --run /path/to/fresh/repeat
python /workspace/regression-rs-hourly-r2/compare.py --actual /path/to/fresh/repeat --expected /path/to/fresh/capture
python /workspace/regression-rs-hourly-r2/compare.py --actual /path/to/fresh/capture --expected /workspace/regression-rs-hourly-r1 --revision-diff
```

Run using the pinned legacy environment, recording interpreter/packages and source
patches. Fresh output paths must not exist; scripts must fail rather than overwrite
a capture. `--revision-diff` reports deliberate timeline/format differences without
waiving exact structure checks within either revision. Rust acceptance later uses
`compare.py --actual <rust-checkpoints> --expected <r2-checkpoints> --stage <name>`.

## Verification and acceptance

Require exact identifiers, shapes, coordinates, timestamps/bounds, segment tags,
settings and masks; compare complete arrays. Capture/repeat must match exactly
for raw streams and deterministic numerical checkpoints. Exclude only recorded
creation/history metadata from equality, never scientific metadata. For r2/r1,
report structural changes separately from value errors; no broad tolerance may
hide a changed timeline or transformation.

Verify the integer centimetre conversion exactly on known engine samples and
offsets, including negative values, half-centimetre ties, i32 limits, missingness,
and applying fractional-centimetre offsets before rounding. Regenerate canonical
level expectations from unquantized streams, not the old packed exports.

Start Rust-to-reference comparisons at exact equality and report measured error
by stage. Propose any nonzero absolute/relative tolerance with numerical rationale
and full error measurements for review; do not invent final tolerances before
measurements. Compare raw f32 and unpacked values separately from legacy packed
NetCDF; its 0.001 resolution does not imply a universal error bound (the retained
case reports float32 flow packing error around 0.00224 m³/s).

Acceptance requires the four scenarios and fresh repeat, explained time/output
mapping, common historical segment and scenario differences, known station-offset
conversion, retained sources and reproducible commands, plus documented reasons
for every revised expectation. Any remaining blocker names the missing fact and
affected downstream capability. Saved-state compatibility remains an explicit
scientific caveat, not a new scope gate. Routine tests require neither Python nor
MGB; documentation-only changes receive content/link checks under
[AGENTS.md](../../AGENTS.md). Retain this specification until acceptance.
