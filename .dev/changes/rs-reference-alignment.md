# Step 1 — Consolidate regression-rs-hourly

Status: agreed implementation plan; the consolidated target has not been built.

Build one offline regression target at `/workspace/regression-rs-hourly` for the
following Rust [observation](rs-observed-forcing.md),
[forecast](rs-forecast-forcing.md), and [execution](rs-model-execution.md)
implementations. It replaces the existing r1 bundle. Copy the required inputs and
provenance into the replacement, validate the replacement, then delete r1. Do not
keep revision comparison tooling, a second acceptance target, or a dependency on
r1 after consolidation.

Linux is the acceptance platform. Carry the supplied executable, geometry,
calibration, ancillary inputs and state files unchanged. Hot-start implementation
follows the VB behavior in its own Rust increment; neither state investigation nor
hydrological qualification is a prerequisite for this regression. Acquisition,
Windows, AR(1), reports and production Rust implementation are outside this step.

## Fixed case

Preserve Python's application-level date handling. Do not redesign the timeline
from VB preparation or engine-internal dates. The following are fixture values,
not universal product defaults:

| Setting | Required value |
| --- | --- |
| Operation timezone | `America/Sao_Paulo` |
| Reference | July 20, 2026 00:00 local / 03:00 UTC |
| Historical interval ends | July 10 00:00 through July 20 00:00 local, inclusive; 241 rows |
| Historical rainfall lower bound | July 9 23:00 local |
| Forecast interval ends | July 20 01:00 through July 25 01:00 local, inclusive; 121 rows |
| Combined axis | 362 rows, historical followed by forecast; dt=3600 seconds |
| Result labels | The same Python application labels and segment split; retain engine-internal timing separately as provenance, without relabelling results |
| Scenario PARHIG | Start July 10 00:00, NT=362, DT=3600; preserve other fields |
| Scenario Previsao.meta | July 20 01:00, horizon 121; preserve unrelated fields |
| Observed provider | `ana` |
| Requested bbox | `[-58.95016,-34.40217,-48.85759,-26.29721]` |
| Working grid | 0.1 degrees, including boundary-touching cells; 83 latitude × 102 longitude cells |
| IDW | Five nearest neighbours, power 2, legacy degree-distance arithmetic |
| Basin order | The 8,649 rows of the supplied MINI.gtp, with explicit unique mini IDs |
| Forecast sources | `ecmwf.ifs.fc.20260720T000000Z.precipitation_grid` and `noaa.gfs.fc.20260720T000000Z.precipitation_grid` |
| Scenarios, in order | `zero`, `ecmwf-raw`, `noaa-raw`, `ecmwf-corrected` |
| ECMWF correction | ID `rs-ecmwf-correction`; hours 0–126; latitude +8 pixels, longitude −12 pixels, rotation +45 degrees, multiplier 1.5 |

The correction operates on the 0.25-degree native grid: translation, rotation
around the original footprint centre with expanded footprint and nearest-cell
sampling, then multiplication/clipping. Uniform interval splitting and bilinear
working-grid resampling follow, then grid-to-basin IDW. Preserve source masks,
coordinates and interval bounds. The saved forcing uses 41 native three-hour
intervals from July 20 03:00 through July 25 06:00 UTC.

Use the six persisted point NULL replacements below. Detection does not authorize
new edits. Assign these fixed IDs in chronological order:

| Replacement ID | Station | Local time, July 15, 2026 | Original mm | Replacement |
| --- | --- | --- | --- | --- |
| `rs-null-1` | `ana:71200000` | 05:00 | 0.2 | NULL |
| `rs-null-2` | `ana:71200000` | 06:00 | 0.2 | NULL |
| `rs-null-3` | `ana:71200000` | 07:00 | 0.2 | NULL |
| `rs-null-4` | `ana:71200000` | 08:00 | 0.2 | NULL |
| `rs-null-5` | `ana:71200000` | 09:00 | 0.4 | NULL |
| `rs-null-6` | `ana:71200000` | 10:00 | 0.4 | NULL |

Retain the supplied runner/Input/EstacoesNivel.txt as the offset fixture source,
with its 101 station records and original bytes. Do not substitute the different
185-station operational file. Preserve station identity and shared-mini records.
Raw engine levels remain the primary engine expectation. Derived station-offset
checks follow the existing [level conversion contract](../../docs/architecture.md#level-reference-and-analysis-corrections)
and execution specification; they do not change engine inputs or require a new run.

## Capture implementation

Use the Python capture revision `15981642a7476fb583f63aeb89cdb1472cfd6b30`
and record actual interpreter, packages, native decoder and executable identities.
Adapt the existing capture code rather than building another framework. The
regression code stays outside production Rust; Rust never reads the legacy
SQLite schema or invokes Python.

Provide only two command entry points: `capture.py` and `compare.py`. Capture
owns input conversion, preparation, execution and checkpoint extraction; private
helpers may share code. Do not add separate qualification, diagnostic, conversion,
normalization or checkpoint commands.

1. Retain an independent copy of the original history backup, normalized native
   forecast assets, original July 20 00Z ECMWF/NOAA GRIB downloads, supplied runner
   inputs/executable and necessary provenance under `frozen/`. Use no hard links.
   Pin sizes and SHA256; runtime sources are used only to assemble this frozen set.
2. Record the fixed settings, selected observation/forecast identities, ordered
   instructions and complete axes in `case.json`. No clock-based selection,
   latest-provider resolution or operational-data fallback is allowed.
3. In each fresh capture directory, convert frozen inputs into the canonical
   SQLite/NetCDF fixtures specified by steps 2a, 2b and 3. Preserve source units,
   missingness, station/series identities and source provenance. Create those
   fixtures during this step, without waiting for Rust implementations. Schemas
   remain authoritative in the linked specifications; do not invent another schema.
4. Prepare historical forcing once and each forecast scenario independently using
   the pinned Python scientific functions and explicit case settings. Parse and
   validate metadata edits; do not rely on an unchecked literal string replacement.
   Preserve complete values before engine f32 conversion and NetCDF packing.
5. Obtain complete historical levels with one shared history-only execution:
   PARHIG start July 10 00:00, NT=241, DT=3600; Previsao.meta July 20 01:00,
   horizon zero; the same 241 historical rainfall rows and unchanged ancillary
   inputs. Use only its historical streams. This tested method replaces the old
   interception; no LD_PRELOAD library, executable patch or recovery command ships.
6. Execute all four scenarios with the fixed 362-row input. Retain the four raw
   flow/level streams. Check every scenario's historical flow against the shared
   history-only execution and its unaffected historical-level suffix (rows
   121–240, zero-based) against the complete shared levels. Assemble complete
   historical levels from the shared execution and forecast levels from each
   scenario's forecast stream. Never decode the overwritten YTUDO prefix as history.
7. Decode and label results using Python's application convention above. Capture
   raw f32 metre levels and f32 flow, unpacked results and derived canonical level
   expectations. Engine text timestamps are not the application time authority.
8. Preserve invocation, environment including OMP_NUM_THREADS=1, input identities,
   exit status, stdout/stderr and useful engine logs. Nonzero exit, malformed or
   missing required output, unexpected nonfinite values or comparison failure
   fails capture. Preserve failed work for diagnosis.

## Checkpoints for the following implementations

Capture writes `fixtures/` for canonical saved inputs and `checkpoints/` for
expected stage results, using the layouts in the linked implementation plans.

| Following implementation | Retained input and full-array expectations |
| --- | --- |
| Observations, step 2a | Canonical observations SQLite; exact source/series selection, raw/effective observations, six ordered replacements, QC findings, coordinates, historical working grid and basin rainfall |
| Forecast normalization, step 2b | Original GRIB downloads, source message selection/provenance, cropped decoded cumulative fields, normalized native interval grids, units, coordinates, bounds and masks |
| Forecast preparation, step 2b | Forecast catalog/correction fixture, transformed native ECMWF frames, each hourly working grid, zero forcing and each forecast basin rainfall array |
| Integration/serialization, step 3 | Complete pre-serialization f64 historical/forecast/combined basin rainfall, MINI order, PARHIG/Previsao.meta and exact CHUVABIN f32 bytes per scenario |
| Execution/decoding, step 3 | Shared historical outputs, four original scenario streams, complete decoded flow/raw levels, Python application time labels/segments and independently derived station-offset expectations |

Direct ecCodes decoding/cropping without GRIB re-encoding already matches both
saved native arrays exactly. Preserve this equality check in capture; no special
re-packing tolerance or extra crop/write/read checkpoint is required. Preserve
native data before corrections so normalization and preparation can be tested
independently. Missing data is distinct from zero at every stage.

## Commands and comparison policy

These are interfaces to implement, not commands already available:

```sh
python /workspace/regression-rs-hourly/capture.py --case /workspace/regression-rs-hourly/case.json --output /path/to/fresh/capture
python /workspace/regression-rs-hourly/capture.py --case /workspace/regression-rs-hourly/case.json --output /path/to/fresh/repeat
python /workspace/regression-rs-hourly/compare.py --actual /path/to/fresh/repeat --expected /path/to/fresh/capture
python /workspace/regression-rs-hourly/compare.py --actual /path/to/rust/results --expected /path/to/fresh/capture --stage observed
```

Capture output paths must not exist; fail rather than overwrite. The bundle's
accepted expectations live under `capture/`; `--stage` accepts `observed`,
`normalization`, `forecast`, `engine`, or `results`, and omission compares all.
The comparator reads values without performing scientific transformations.
It emits JSON with pass/fail, shape/mask differences, first mismatch coordinate,
maximum absolute/relative error and differing-value counts; mismatch exits nonzero.

Require exact IDs, shapes, coordinates, interval bounds, application timestamps,
segments, scientific settings, masks, integer levels, raw bytes and numerical
arrays. Exclude only recorded creation/history metadata. No tolerance flags or
implicit rounding/packing are allowed to make a mismatch pass. Compare f64
checkpoints before packing, CHUVABIN after f32 serialization and original engine
streams separately from legacy packed exports. A Rust arithmetic mismatch is a
reported regression failure, not authorization to change the frozen expectation.

## Acceptance and handoff

The target is complete when both fresh captures cover all four scenarios, every
checkpoint and raw byte comparison passes, historical outputs agree across
scenarios, and nonzero scenarios produce differing forecast rainfall/results.
A small comparator self-check must reject a changed value, shifted timestamp,
missingness change and truncated stream. Input sources must remain unchanged.

Retain `manifest.json` with case/source/tool identities and the full retained-file
inventory, plus `comparison.json` with measured acceptance results. Update
[the case README](../../references/regression/rs-hourly/README.md) and its tracked
manifest/expectations to describe only the consolidated target and its reproducible
commands. Update the frozen-artifact path in [AGENTS.md](../../AGENTS.md), retire
r1 and repair remaining active links/paths. Historical reports may remain as
historical records; they do not override the fixed case above.

Update [migration status](../migration/summary.md) only after these checks pass.
Routine Rust tests remain independent of Python, providers and MGB. Real saved-case
Rust acceptance is explicit and stage-based as each capability is implemented.
No additional timeline, state or engine-configuration investigation gate remains.
