# Step 2b — Saved forecasts to canonical forecast forcing

Status: implementation specification; no Rust forecast capability is implemented.
Dependencies: [reference qualification](rs-reference-alignment.md), shared values
with [observations](rs-observed-forcing.md), and the preparation join owned by
[integration](rs-model-execution.md). This step does not run MGB.

## Deliverables and source evidence

Implement standalone saved ECMWF IFS / NOAA GFS GRIB2 normalization, immutable
native NetCDF plus forecast catalog persistence, ordered correction instructions,
hourly working-grid and basin forcing preparation, and zero forecast forcing.
Inputs are explicit saved sources, provider/product/cycle, operation timezone,
requested spatial crop, target interval axis, geometry, and numerical settings.
No current-clock/provider lookup or implicit latest forecast selection is allowed.
Normalization also returns canonical values without persistence. Preparation accepts
canonical native values independently of the adapters and observation database.

Read these reference implementations before implementing their counterparts:
`adapters/_grib2.py`, `adapters/forecast_ecmwf.py`,
`adapters/forecast_noaa.py`, `edit/forcing.py`, `assets/spatial_grid.py`,
`assets/grid_transforms.py`, and `model/prepare_mgb_rainfall.py` in the preserved
Python worktree. Relevant tests are `tests/integration/test_forecast_ecmwf.py`,
`test_forecast_noaa.py`, `test_forecast_scenarios.py`, and
`tests/unit/test_forecast_grid_corrections.py`, `test_grid_transforms.py`,
`test_spatial_grid.py`. Legacy format schemas are evidence, not Rust runtime APIs.

Read-only `grib_ls` inspection of the selected July 20 UTC downloads establishes:

| Source | Actual fields and packing | Consequence |
| --- | --- | --- |
| ECMWF `ifs_fc_20260720T000000Z.grib2` | 85 messages; regular_ll, 1440 columns × 721 rows; `tp`, metres; PDT 4.8, statistical accumulation; CCSDS template 5.42; leads 0–144 every 3h, then 150–360 every 6h | Decode CCSDS, convert ×1000, retain native 3/6h intervals; source is global, unlike the cropped normalized legacy asset |
| NOAA `gfs_fc_20260720T000000Z.grib2` | 162 messages; regular_ll, 202 × 162; `tp`/APCP, kg m^-2; simple packing; both rolling and cumulative fields, including repeated 0–3 and 0–6 fields; last lead 252h | Select startStep=0 cumulative fields; compare duplicates before collapsing; never difference rolling fields as cumulative |

Preserved capture settings are bbox `[-58.95016,-34.40217,-48.85759,-26.29721]`,
working resolution 0.1°, native resolution 0.25°, IDW k=5/power=2,
ECMWF correction hours 0–126, shift latitude +8 pixels, longitude −12 pixels,
rotation +45°, multiplier 1.5. These are fixture configuration, not Rust defaults.
The original capture uses 41 ECMWF native intervals covering forecast hourly ends;
[step 1](rs-reference-alignment.md) settles the target timeline before acceptance.

## Files, owners, and callable boundaries

Proposed files are added only when their behavior is implemented. Extend the
existing facade with explicit re-exports and keep existing inspection APIs stable.

| File | Implemented responsibility and proposed API |
| --- | --- |
| `src/store.rs`, `src/store/values.rs` | Shared canonical `IntervalAxis`, `RegularGridSpec`, `GridFrame`, `GridSeries`, `BasinForcing`, `ForecastSource`, `ForecastCorrection`, `ForecastPreparation`; no duplicate observation/forecast representations |
| `src/store/netcdf.rs` | `read_grid_series`, `write_grid_series`, `read_basin_forcing`, `write_basin_forcing`, transformed-frame checkpoint persistence; private Terrio mapping, schema/version/mask validation |
| `src/store/forecast.rs` | `ForecastCatalog::open`, `register_native`, `load_native`, `save_correction_set`, `load_correction_set`; immutable retained-source/artifact lifecycle, transactions |
| `src/adapters.rs`, `src/adapters/forecast.rs` | `normalize_saved_forecast(path, &NormalizationRequest) -> Result<NormalizedForecast, NormalizationError>`; private GRIB metadata decoding plus ECMWF/GFS selection and cumulative-to-native normalization; return canonical values/provenance, no catalog writes |
| `src/analysis.rs`, `src/analysis/spatial.rs` | Shared grid generation, `bilinear_resample`, `build_grid_idw_neighbors`, `interpolate_grid_to_basins`; also used by observation preparation |
| `src/analysis/forecast.rs` | `prepare_forecast(&GridSeries, &[ForecastCorrection], &ForecastPreparationRequest) -> Result<ForecastPreparation, ForecastPreparationError>` and `prepare_zero_forecast(&IntervalAxis, &BasinGeometry) -> Result<BasinForcing, ForecastPreparationError>`; no catalog/file access |
| `src/app/forecast.rs` | `normalize_forecast(request)` composes adapter/store; selects explicit native artifact/correction set and calls analysis for preparation; config/path resolution stays in app |
| `src/main.rs` | Thin `forecast normalize --workspace PATH --request REQUEST.toml [--json]`; shared `operation prepare --workspace PATH --request REQUEST.toml --output DIR` is composed by integration |
| `tests/forecast_normalization.rs`, `tests/forecast_preparation.rs`, `tests/canonical_netcdf.rs`, `tests/forecast_catalog.rs` | Behavioral integration tests with small retained fixtures |

App's normalization request contains provider (`ecmwf`/`noaa`), model (`ifs`/`gfs`),
product `fc`, expected RFC3339 UTC cycle, source path, timezone, crop bbox, and
explicit destination/catalog selection. Unknown keys/products, mismatched cycles,
non-finite settings, antimeridian crops and unsupported geometry fail explicitly.
Response contains canonical native artifact ID/location, source identity, interval
coverage, effective crop and source dimensions. The adapter's response contains
values and normalization evidence without inventing paths. `ForecastPreparation`
returns transformed frames, working GridSeries and BasinForcing; app chooses which
checkpoints store retains. Execution only consumes prepared canonical inputs.

Use `chrono` + `chrono-tz` for adapter UTC-to-operation conversion and local timestamps, `rusqlite` for SQLite,
and `sha2` for retained artifact/source identities. Pin exact tested versions in
Cargo.lock when implementing; no new numerical/GIS dependency is needed for the
specified arithmetic. Time/geometry value definitions are shared with step 2a.

## Canonical numerical contracts

Store owns values and validates shapes before any allocation/product indexing.
`IntervalAxis` carries timezone and explicit local start/end timestamps; interval rainfall
is the total over `(start,end]`, timestamped at end. Positive bounds are ordered,
non-overlapping, and contiguous where complete coverage is requested. Source cycle
remains explicitly UTC; canonical timestamps retain operation timezone/offset.
Ambiguous/nonexistent local request times must carry offsets or fail.

`GridFrame` contains one interval, increasing latitude/longitude cell centres,
edge bounds `(west,south,east,north)`, finite f64 totals in mm plus separate validity
mask, source identity and transform provenance. `GridSeries` uses common axes with
flat time-major `[time,latitude,longitude]` arrays (longitude fastest), one explicit
mask per value. Native resolution and working resolution are separate metadata.
Transformed frames can have different footprints, so they are `Vec<GridFrame>`,
not a padded series pretending all intervals share coordinates. `BasinForcing`
uses `[time,basin]`, explicit unique mini IDs in selected geometry order, centroid
coordinates, mm interval totals and masks. No implicit float-to-integer ID mapping.
Unknown/missing is never zero. Finite negative rainfall is invalid after normalized
increment clipping; unexpected infinities fail rather than become missing.

NetCDF schema v1: root attrs `schema_version=1`, `artifact_kind`, `units=mm`,
`operation_timezone`, `source_id` when applicable, `geometry_sha256` for basin data,
`processing_json` with complete settings/source/parent/correction identities.
Grid dimensions `time,latitude,longitude,bounds=2`; arrays `time(time)`,
`time_bounds(time,bounds)`, `latitude(latitude)`, `longitude(longitude)`,
`precipitation(time,latitude,longitude)` f64, `valid(...)` u8 (0/1).
Basin files replace grid axes with `basin`, `mini_id(basin)` integer,
`basin_latitude`, `basin_longitude`, and `precipitation(time,basin)`.
Encode time/time_bounds as i64 seconds relative to a retained operation-local
origin with units `seconds since <local_origin>` (no UTC suffix), calendar
`proleptic_gregorian`, and required operation_timezone. Endpoints in memory are
`chrono::NaiveDateTime` in that zone; SQLite uses operation-local ISO8601 labels.
Adapters convert source UTC once before returning canonical values. Reject
ambiguous/nonexistent local endpoints and axes crossing a timezone-offset change
until a corresponding local-time/DST contract is explicitly qualified; the RS
fixture has no such transition. Preserve source cycle UTC separately.
All grid coordinate axes use geographic EPSG:4326, longitude units `degrees_east`
and latitude units `degrees_north`; record `crs=EPSG:4326` and axis orientation.
Set precipitation `_FillValue` to NaN through the writer-supported string `NaN`
attribute representation; the explicit valid mask remains authoritative and
reader verifies its consistency. `Array::from_bytes` receives little-endian
row-major bytes (including i64 times and mini IDs, u8 validity and f64 rainfall).
Use unpacked values, no 0.001 quantization, compression optional only after exact
round-trip qualification. Invalid cells use NaN payload plus valid=0; valid=1
requires finite values; read rejects disagreement. Set CF coordinate units and
`time:bounds=time_bounds`; retain explicit requested/effective bbox and resolution.
Transformed checkpoints are one frame NetCDF per native interval with indexed
manifest recording original interval/source and correction IDs; no fake uniform
footprint or missing-as-zero padding.

## Forecast catalog and corrections

Use `data/forecasts.sqlite`, separate from `data/observations.sqlite` and operation
metadata. `PRAGMA user_version=1`, foreign_keys=ON; use explicit schema migrations.
Canonical timestamps below are operation-local ISO8601 labels (without a UTC
suffix); operation_timezone is required. Explicitly UTC cycle fields retain Z.

| Table | Fields and constraints |
| --- | --- |
| `forecast_source` | `source_id TEXT PRIMARY KEY`, provider/model/product TEXT NOT NULL, `cycle_utc TEXT NOT NULL`, `retained_path TEXT NOT NULL`, `sha256 TEXT NOT NULL`, `byte_length INTEGER NOT NULL CHECK(byte_length>=0)`, `normalization_json TEXT NOT NULL`, `operation_timezone TEXT NOT NULL`; identity includes content and normalization settings, not merely cycle |
| `forecast_artifact` | `artifact_id TEXT PRIMARY KEY`, `source_id TEXT NOT NULL REFERENCES forecast_source`, kind (`native`,`working`,`basin`,`transformed`), `relative_path TEXT UNIQUE NOT NULL`, sha256, byte_length, schema_version, `valid_from`,`valid_to`, processing_json; source/correction lineage explicit |
| `forecast_correction_set` | `set_id TEXT PRIMARY KEY`, source_id FK NOT NULL, editor, reason, created_at; immutable revisions, no in-place mutation of selected instructions |
| `forecast_correction` | set_id FK, `position INTEGER CHECK(position>=0)`, `t0_hour INTEGER CHECK(t0_hour>=0)`, `t1_hour INTEGER CHECK(t1_hour>=t0_hour)`, shift_lat_pixels REAL, shift_lon_pixels REAL, rotation_deg REAL, multiplication_factor REAL CHECK(>0), `PRIMARY KEY(set_id,position)` |

Validate all numeric values finite in Rust (SQLite alone does not establish this).
Preserve empty/no-op windows for faithful explicit instruction replay, but report
which native intervals each instruction affects. Reject applying a set to another
source. Source paths are workspace-relative retained immutable copies, not pointers
to mutable downloads. Write/flush/rename artifact before committing its catalog
reference; on failure retain unregistered staging diagnostics, never commit a
reference to a partial file. Reopening validates checksum/schema/source identity.
Raw and corrected scenarios share native source; an empty set means raw. Zero
forcing has no forecast source or catalog record; operation artifact registration
belongs to integration. No production Python SQLite schema support is added.

## Exact normalization algorithm

1. Open the saved source with native Terrio GRIB driver; enumerate band metadata
   before selecting fields. Require expected provider/product/cycle, surface
   accumulated precipitation, time units, regular latitude/longitude grid and
   interpretable PDT. Record message index, statistical method, start/end lead,
   units and raw template metadata. Do not guess absent start bounds as zero.
2. Convert longitude >180° to signed longitude, sort columns and reorder values;
   make latitude increasing and reorder rows. Preserve masks from bitmap/nodata.
   Select only cells whose closed footprint intersects the requested buffered bbox,
   retaining all native cells needed by correction/interpolation. Crop settings
   are explicit; legacy buffer policy expands bbox by 2×width/height on each side.
   Reject antimeridian crossing; compute effective bounds from centres ±half-cell.
3. ECMWF: require cumulative start=0 series; step0 supplies initial cumulative
   state but creates no zero-duration rainfall interval. NOAA: require start=0,
   ignore rolling messages, collapse exactly equal coordinate/value/mask duplicates;
   conflicting duplicates fail. Sort positive leads, require a single source cycle
   and stable coordinates. Duplicate ambiguity never silently uses last message.
4. Convert ECMWF m→mm ×1000; NOAA kg/m²→mm ×1. For each cumulative message subtract
   previous cumulative field (zero at cycle if no lead0), create `(previous_end,end]`
   native total, and clip finite negative differences to zero as legacy does.
   Missing either operand yields missing. Record clipped count/magnitude; never
   interpret a rolling-window reset as a fresh accumulation. Mixed cycles or
   unknown accumulation/reset semantics fail with metadata evidence. Negative
   differences within a valid cumulative series follow the explicit clamp policy.
5. Convert source interval/cycle timestamps to canonical operation convention,
   retaining original UTC cycle/lead bounds. Return native intervals before any
   hourly splitting/correction; store may persist/catalog the same returned values.

### Direct-source decoding versus legacy GRIB re-packing

ECMWF `process_forecast_grib` first invokes `crop_grib_to_bbox` into a temporary
GRIB, which uses `set_regular_ll_grid` / ecCodes `codes_write`, then reads that
re-encoded file for normalization. NOAA directly normalizes its already cropped
saved source. Rust crops decoded ECMWF values in memory and does not introduce a
GRIB write/read round trip. This is format processing, not a scientific grid rule.

Step 1 must preserve three distinct checkpoints: direct saved-source ecCodes
values cropped by cell intersection; values decoded after legacy crop/re-packing;
and the frozen normalized native interval fields. Compare first versus second to
measure GRIB re-packing effects, then second versus frozen native fields to check
unit conversion/differencing. Compare Terrio decoded source versus direct-source
ecCodes before normalization to isolate Terrio f32 decoding precision. Finally
compare Rust native intervals versus the qualified native reference, reporting
these explained contributions separately. Do not hide re-packing differences in
interpolation tolerances, require a new GRIB write, or change frozen arrays without
step-1 evidence and reviewed expectations. Record complete masks/coordinates and
per-stage maximum errors, not only final-field summaries.

## Corrections and forcing algorithm

For every native interval used by a requested target, apply instructions in
ascending `position` when the entire native interval lies inside
`[cycle+t0_hour,cycle+t1_hour]`. Partially overlapping native intervals remain
uncorrected, matching Python; report affected IDs so a partial selection is visible.
No overlapping instruction merging. Require square regular geographic cells.
Translation rounds pixel counts with ties-to-even (Python round), multiplies by
native resolution, and moves coordinate axes and bounds. Multiplication scales
finite cells and clamps at zero while preserving missingness. For rotation, apply
translation first and rotate counterclockwise in longitude/latitude around the
pre-translation footprint centre. Rotate all four edge corners, round bounds
outward to resolution-aligned edges, construct centres, inverse-rotate and undo
translation to sample the original grid with nearest ties-to-even row/column.
Outside original footprint stays missing; do not clip the transformed canvas.
Subsequent instructions act on the current transformed footprint and centre.

For each requested hourly `(start,end]`, require exactly one containing native
interval and native duration divisible by one hour. Divide that corrected interval
total uniformly by native duration in hours. Corrections precede division and
working-grid resampling. Use separable linear interpolation along longitude then
latitude, matching `numpy.interp`; do not extrapolate beyond source centres or
replace a missing contributor with zero. Require every working-grid target/time
finite. Working centres follow shared closed-footprint intersection policy:
`lower-resolution/2 + i*resolution`, count `floor((upper-lower)/resolution)+2`;
retain requested and effective bounds. Do not use driver warp defaults.

Flatten working cells latitude-major/longitude-fastest. For each basin use the
shared grid-to-basin IDW: Euclidean distance in degrees, nearest min(k,N) static
cells, zero distance replaced by 1e-12, weights=distance^-power, renormalize over
finite selected values, fail if no positive valid weight. Equal-distance selection
must be checked against the frozen Python ordering; use stable source-flat-index
ordering only after tie fixture/reference comparison establishes equivalence.
Zero forcing constructs explicit zeros on the supplied forecast axis and basin
order, valid everywhere, without a provider artifact. Historical integration remains
outside this function.

## Terrio source qualification and implementation gate

Investigated native source revision
[`29fd6e78ba290e37e58aac16df61883e8ed4aed3`](https://github.com/opengeos/terrio/tree/29fd6e78ba290e37e58aac16df61883e8ed4aed3),
workspace version 0.8.0. Website/API and repository caches can disagree; pin this
revision during qualification, then pin a released version only if its code is
identical for the qualified behavior. Candidate Cargo dependency is `terrio` at
that git revision, `default-features=false`, features `grib,netcdf-hdf5`; no GDAL.
The `grib` feature enables pure-Rust `rust-aec` for CCSDS 5.42. No direct dependency
on GRIB/NetCDF/HDF/GDAL driver crates is introduced.

Actual entry points, verified from source:

- `terrio::open_raster_lazy`, `LazyRaster::band_metadata`,
  `read_window_f64(Window, &FloatReadOptions)` read GRIB bands/windows. Metadata
  includes GRIB_REF_TIME, VALID_TIME, FORECAST_SECONDS, UNIT, ELEMENT, PDS_PDTN,
  PDS_TEMPLATE_NUMBERS and optional ASSEMBLED_VALUES. Adapter interprets PDT 4.8
  bytes/time-range descriptors to obtain accumulation start bounds; Terrio's
  `multidim::open` supports NetCDF/HDF/Zarr, not GRIB.
- `terrio::multidim::{Group::new,Dimension,ArrayInfo,Array::from_bytes,Dataset}`
  build typed numeric arrays/attributes; `Dataset::write` writes numeric NetCDF-4.
  `multidim::open`, `Dataset::array`, `Array::read` read it. Use these multidimensional
  APIs for time/grid/basin contracts rather than 2-D raster export.
- Driver `drivers/grib.rs` decodes simple and CCSDS packing and returns internal
  f32-decoded values before f64 exposure. This is a precision qualification item,
  not proof of unsupported I/O. Metadata bytes, bitmap/nodata, global longitude
  wrapping, north-up→ascending reordering and the source crops require real probes.

Implement a disposable probe under revised reference `qualification/terrio/`, not
production fallback code. Read both preserved sources band-by-band, compare each
selected metadata record and complete cropped field/mask against ecCodes-derived
reference; report maximum absolute/relative and cumulative-difference error.
Write/read all canonical dimensions, integer basin IDs, f64 values, u8 masks,
string/numeric attributes, time bounds and transformed frames; compare exactly.
Measure peak memory with global ECMWF (do not eagerly retain 85 global planes).
Pin resulting Cargo.lock, driver revision/features, commands and measured precision
in `qualification.json`. Official sources: [features](https://github.com/opengeos/terrio/blob/29fd6e78ba290e37e58aac16df61883e8ed4aed3/crates/terrio/Cargo.toml),
[GRIB driver](https://github.com/opengeos/terrio/blob/29fd6e78ba290e37e58aac16df61883e8ed4aed3/crates/terrio/src/drivers/grib.rs),
[multidimensional API](https://github.com/opengeos/terrio/blob/29fd6e78ba290e37e58aac16df61883e8ed4aed3/crates/terrio/src/multidim/mod.rs).
No required capability has yet been proven unsupported. If a probe fails, report
exact requirement/source/API/error before proposing another library or upstream fix.
Scientific interpolation/correction policy is independent of format adequacy.

## Implementation sequence and acceptance checks

1. Settle shared timestamp encoding/types and corrected timeline with step 1;
   qualify Terrio above. Add store values/NetCDF and synthetic round trips first.
2. Add adapter selection/differencing; create tiny checked-in GRIB fixtures under
   `tests/fixtures/forecast/` for both packing templates, lead0, rolling/cumulative
   duplicates, changed grid, conflicting duplicates, missing and invalid metadata.
   Fixture provenance gives source hashes and generation command; routine tests
   require neither Python nor ecCodes. Saved large files remain outside Git.
3. Add catalog/correction schema and persistence tests, then standalone library/CLI
   normalization tests. Check rejection leaves no registered partial artifact and
   changing external source cannot change retained native data.
4. Add correction + hourly + bilinear + shared IDW preparation. Tiny fixtures cover
   instruction ordering, whole/partial windows, ties-to-even shifts/nearest pixels,
   90° rotation and expanded canvas, fractional shifts, masks, duration splitting
   conservation, overlapping/gapped intervals, source coverage, exact coordinates,
   IDW neighbours and zero/no-provider behavior.
5. Revised reference folder `/workspace/regression-rs-hourly-r2/` owns one-off
   `convert-forecasts.py` / checkpoint conversion code; emit canonical expected
   normalized-native, transformed-frame, working-grid and basin artifacts plus
   hashes/settings. It may use legacy Python/ecCodes to build reference fixtures;
   production Rust never reads Python schemas or runs those scripts.
6. Run explicit acceptance normalization against both saved GRIBs and compare full
   native fields/masks/bounds, raw versus corrected source reuse, transformed
   footprint/values, hourly working grid and final basin forcing against corrected
   reference. Compare structural fields exactly; proposed numerical tolerances are
   measured stage-by-stage and reviewed per step 1, never inflated to hide mismatch.
   Library and `forecast normalize` CLI must give identical canonical values;
   integration's `operation prepare` command consumes native artifacts without GRIB access.

Document exact acceptance invocations in revised `case.json`/reproduction script:
`cargo run --locked -- forecast normalize --workspace <disposable> --request <ecmwf.toml> --json`
and NOAA equivalent; `cargo run --locked -- operation prepare --workspace <disposable> --request <prepare.toml> --output <prepared-dir> --json`.
Routine gates: `cargo fmt --check`,
`cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`,
`cargo test --locked --test architecture`. Extend architecture tests for new owners
without weakening ownership. This planning change only requires content/link checks.

Acceptance adds concrete formats and command/request contracts under `docs/contracts/`,
shared dependency decisions to architecture, usage to README, and verified progress
to [migration summary](../migration/summary.md). Linux first; Windows remains
unverified. Live downloads, run-all/direct-GRIB execution, GUI, general migration,
engine serialization/execution, and additional forecast products are deferred.

## Concrete shared values and request example

The shared fields agreed with step 2a are:

```rust
struct TimeInterval { start: chrono::NaiveDateTime, end: chrono::NaiveDateTime }
struct IntervalAxis { time_zone: String, bounds: Vec<TimeInterval> }
struct RegularGridSpec {
    requested_bbox: [f64; 4], resolution_degrees: f64,
    latitudes: Vec<f64>, longitudes: Vec<f64>, effective_bbox: [f64; 4],
}
struct BasinGeometry {
    mini_ids: Vec<i64>, longitudes: Vec<f64>, latitudes: Vec<f64>, source: Provenance,
}
struct GridSeries {
    timeline: IntervalAxis, grid: RegularGridSpec,
    values: Vec<f64>, valid: Vec<bool>, provenance: Provenance,
}
struct BasinForcing {
    timeline: IntervalAxis, basins: BasinGeometry,
    values: Vec<f64>, valid: Vec<bool>, provenance: Provenance,
}
```

`Provenance` stores immutable source IDs/hashes and processing settings JSON;
`ForecastCorrection` mirrors the correction table with source/set identity and
position. `NormalizationRequest` has the following TOML contract (all keys required,
no flatten/unknown-key fallback). Source/catalog/artifact paths resolve relative to the selected workspace using
the existing workspace resource rules; request-file location does not change
resource resolution. Explicit absolute paths follow the same resource rules.

```toml
schema_version = 1
provider = "ecmwf"
model = "ifs"
product = "fc"
cycle_utc = "2026-07-20T00:00:00Z"
operation_timezone = "America/Sao_Paulo"
source = "sources/ifs_fc_20260720T000000Z.grib2"
crop_bbox = [-79.13530, -50.61209, -28.67245, -10.08729]
catalog = "data/forecasts.sqlite"
artifact_root = "data/forecasts"
```

The example crop is the capture bbox expanded by twice its width/height; source
cell intersection determines effective bounds. Normalization never resamples to
0.1°. The in-memory request omits catalog/artifact_root; app adds persistence
selection in its workflow request. Corrected preparation selects an explicit
`artifact_id` and `correction_set_id`; raw uses no set; zero uses neither.

## Executable schema and test examples

Implement this forecast SQLite DDL (add NOT NULL to required scalar fields as
shown; bind parameters for all writes):

```sql
CREATE TABLE forecast_source (
 source_id TEXT PRIMARY KEY, provider TEXT NOT NULL, model TEXT NOT NULL,
 product TEXT NOT NULL, cycle_utc TEXT NOT NULL, retained_path TEXT NOT NULL,
 sha256 TEXT NOT NULL, byte_length INTEGER NOT NULL CHECK(byte_length>=0),
 normalization_json TEXT NOT NULL, operation_timezone TEXT NOT NULL
);
CREATE TABLE forecast_correction_set (
 set_id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES forecast_source,
 editor TEXT, reason TEXT NOT NULL, created_at TEXT NOT NULL
);
CREATE TABLE forecast_correction (
 set_id TEXT NOT NULL REFERENCES forecast_correction_set,
 position INTEGER NOT NULL CHECK(position>=0),
 t0_hour INTEGER NOT NULL CHECK(t0_hour>=0),
 t1_hour INTEGER NOT NULL CHECK(t1_hour>=t0_hour),
 shift_lat_pixels REAL NOT NULL, shift_lon_pixels REAL NOT NULL,
 rotation_deg REAL NOT NULL,
 multiplication_factor REAL NOT NULL CHECK(multiplication_factor>0),
 PRIMARY KEY(set_id,position)
);
CREATE TABLE forecast_artifact (
 artifact_id TEXT PRIMARY KEY, source_id TEXT NOT NULL REFERENCES forecast_source,
 correction_set_id TEXT REFERENCES forecast_correction_set,
 kind TEXT NOT NULL CHECK(kind IN ('native','working','basin','transformed')),
 relative_path TEXT UNIQUE NOT NULL, sha256 TEXT NOT NULL,
 byte_length INTEGER NOT NULL CHECK(byte_length>=0), schema_version INTEGER NOT NULL,
 valid_from TEXT NOT NULL, valid_to TEXT NOT NULL, processing_json TEXT NOT NULL
);
```

Require correction-set/source consistency before insert/load; native has NULL
correction_set_id. Reject absolute/escaping retained paths. Store retains the exact
TOML/settings and source bytes; source identity includes normalization settings.

Minimum numerical assertions, expressed on toy fields before regional acceptance:

- Cumulative mm fields at leads 0,3,6 are `[0,0]`, `[3,6]`, `[9,3]`:
  native totals `[3,6]`, `[6,0]` (one clipped negative −3); first interval hourly
  rows `[1,2]` three times, second `[2,0]` three times. NOAA rolling 3–6 field
  `[99,99]` must not alter these selected cumulative results.
- A 3h interval total `[6,12]` split into three hours yields `[2,4]` each, summing
  exactly to the native total; a requested hour covered by two overlapping source
  intervals fails. A valid zero remains valid and a missing operand stays missing.
- Bilinear field `[[0,2],[2,4]]` on latitude/longitude `[0,1]` produces 2 at
  `(0.5,0.5)`; target longitude −0.1 is missing and fails complete-grid validation.
- Translate a 0.25° field by +8 latitude/−12 longitude: axes move +2°/−3°;
  values/masks unchanged before multiplication. Shift 2.5 pixels rounds to 2;
  3.5 rounds to 4. A 0–3 interval inside correction 0–3 is corrected, interval
  0–3 against window 1–3 is not. Ordered multiplication 2 then 3 yields factor 6.
- Two cells at x=0 and x=2 with values 2 and 6, basin x=1 and equal latitude,
  k=2/power=2 yield 4; missing second value yields 2 by renormalization.
- NetCDF round trip preserves `[0,1.234567890123,NaN]`, mask `[1,1,0]`, integer
  mini IDs and both bounds exactly; contradictory valid mask/value fails.
- Zero forcing has all-zero `[requested_time_count,basin_count]`, all valid and
  no forecast_source insertion; standalone normalization never opens observations.

For `tests/architecture.rs`, add facade allowlist entries for app
`normalize_forecast`, `NormalizeForecastRequest`, `NormalizeForecastReport` and
integration's preparation facade explicitly. Add checker fixtures accepting
same-owner `self::forecast` re-exports and interface calls to these facade APIs;
reject adapters→analysis, analysis→adapters, model→adapters, and foreign-item
re-export through store. Existing owners already include store/adapters/analysis.
Use existing supported Serde/Clap derives; no new helper attribute support is
required unless implementation actually introduces one. Keep negative fixtures
for unknown attributes and string-encoded foreign paths intact.

## Typed library requests, results, and failures

These are canonical library contracts; app workflow requests add workspace resource
selection and persistence without changing adapter semantics:

```rust
enum ForecastProvider { Ecmwf, Noaa }
struct NormalizationRequest {
    provider: ForecastProvider, model: String, product: String,
    cycle_utc: chrono::DateTime<chrono::Utc>, operation_timezone: chrono_tz::Tz,
    crop_bbox: [f64; 4],
}
struct NormalizedForecast {
    source: ForecastSource, native: GridSeries,
    messages: Vec<ForecastMessageEvidence>, clipped_negative_count: usize,
    clipped_negative_total_mm: f64,
}
struct ForecastSource {
    source_id: String, provider: ForecastProvider, model: String, product: String,
    cycle_utc: chrono::DateTime<chrono::Utc>, sha256: String, byte_length: u64,
    normalization_json: serde_json::Value,
}
struct ForecastMessageEvidence {
    band: usize, start_lead_seconds: i64, end_lead_seconds: i64,
    product_template: u16, units: String, selected: bool,
    metadata: Vec<(String, String)>,
}
struct ForecastPreparationRequest {
    timeline: IntervalAxis, working_grid: RegularGridSpec, basins: BasinGeometry,
    interpolation: IdwSettings,
}
struct ForecastPreparation {
    transformed: Vec<GridFrame>, working: GridSeries, basin: BasinForcing,
    correction_coverage: Vec<CorrectionCoverage>,
}
struct CorrectionCoverage { position: usize, native_interval_indices: Vec<usize> }
struct NormalizeForecastRequest {
    invocation_dir: std::path::PathBuf,
    workspace: std::path::PathBuf, request_file: std::path::PathBuf,
}
struct NormalizeForecastReport {
    source_id: String, native_artifact_id: String,
    retained_source: std::path::PathBuf, native_artifact: std::path::PathBuf,
    timeline: IntervalAxis, native_shape: [usize; 3], effective_bbox: [f64; 4],
}
fn normalize_saved_forecast(
    path: &std::path::Path, request: &NormalizationRequest,
) -> Result<NormalizedForecast, NormalizationError>;
fn prepare_forecast(
    native: &GridSeries, corrections: &[ForecastCorrection],
    request: &ForecastPreparationRequest,
) -> Result<ForecastPreparation, ForecastPreparationError>;
fn prepare_zero_forecast(
    timeline: &IntervalAxis, basins: &BasinGeometry,
) -> Result<BasinForcing, ForecastPreparationError>;
fn normalize_forecast(
    request: NormalizeForecastRequest,
) -> Result<NormalizeForecastReport, NormalizeForecastError>;
```

`NormalizationError` variants distinguish `Io`, `Format`, `UnsupportedProduct`,
`Metadata`, `CycleMismatch`, `GridMismatch`, `ConflictingDuplicate`, `Coverage`,
and `InvalidRequest`; preserve source/band/interval context and underlying errors.
`ForecastPreparationError` distinguishes `InvalidInput`, `SourceMismatch`,
`InvalidCorrection`, `AmbiguousInterval`, `MissingTimeCoverage`,
`MissingSpatialCoverage`, and `InvalidGeometry`, identifying failing interval or
cell/basin. `NormalizeForecastError` wraps app `Configuration`, adapter
`Normalization`, and store `Persistence` errors. Store APIs return
`Result<T, StoreError>` with `Io`, `Sqlite`, `Format`, `Schema`, `Integrity`,
`InvalidPath`, and `InvalidValue` variants. No public API returns string-only
errors or silently converts failure to an empty artifact. Reports identify actual
persisted artifacts; in-memory adapter results contain no persistence paths.

The interface captures `invocation_dir` and passes it explicitly. App resolves
relative workspace/request arguments against it using the existing library path
contract; core functions never discover process cwd with `std::env`. Resource
paths inside the request resolve against the selected workspace. The integration
commands are `operation prepare` and `operation execute`; normalization remains
`forecast normalize`.

Preparation selects `native_artifact_id` returned by normalization, then loads its
source identity and validates any correction-set/source relation. A legacy asset
label such as `ecmwf.ifs.fc.20260720T000000Z.precipitation_grid` is provenance only:
it is neither the content/settings-derived `source_id` nor a canonical artifact
ID. Raw and corrected scenarios select the same native artifact ID and different
explicit correction-set choices. Keep these identifiers distinct in request
examples, fixture converters, catalog queries and retained snapshots.
