# Step 2a — Canonical observations to observed forcing

Status: implementation specification; no operational implementation or acceptance
is claimed. Depends on [step 1](rs-reference-alignment.md); can be implemented
independently of [step 2b](rs-forecast-forcing.md). [Step 3](rs-model-execution.md)
owns the observed/forecast join, execution snapshots, and model serialization.

## Deliverable and evidence

Implement a Rust-owned observations SQLite, an in-memory effective-observation
pipeline, rainfall QC, station-to-working-grid IDW, working-grid-to-basin IDW,
and persisted canonical historical forcing. A caller supplies selections,
intervals, geometry and settings. Neither scientific analysis nor model queries
a database. Production Python-history support, acquisition, review UI, direct
station-to-basin preparation, AR(1), and a combined run-all command are excluded.

The implementation reference is `src/mgb_ops/` under the sibling
`legacy_python` worktree: selection in
`assets/history_queries.py`; review in `assets/effective_observations.py`; QC in
`qc/precipitation.py`; station interpolation in `assets/observed_precipitation.py`;
grid coordinates in `assets/spatial_grid.py`; the two IDW kernels in
`assets/grid_transforms.py`; and the cache-grid branch and `_grid_to_mini_matrix`
in `model/prepare_mgb_rainfall.py`. The named module paths are relative to `src/mgb_ops/` there; the
worktree location is described in [AGENTS.md](../../AGENTS.md).

The frozen `capture.py` selects ANA, k=5 and power=2, requested bbox
`[-58.95016,-34.40217,-48.85759,-26.29721]`, resolution 0.1 degrees, and boundary
touching cells. It stores six explicit NULL replacements derived during capture
from QC sequence findings. These are fixture values, not product constants.
The original fixture has 18 raw rainfall series, 7 raw level series and 6 raw
flow series. Its variables declare rainfall mm, level cm, and flow m3/s. The
captured observed endpoints run July 10 00:00 through July 20 00:00 in
America/Sao_Paulo; step 1 qualifies their interpretation before acceptance.

The older direct station-to-basin branch uses missing-to-zero filling. The saved
cache-grid path does not. Implement only the latter, retaining missingness and
failing uncovered timesteps. Do not reproduce an unused branch as a fallback.

## Files, dependencies, and boundaries

New production files, created with their behavior rather than as empty layers:

| File | Implementation |
| --- | --- |
| `src/store.rs` | Owner facade, explicit exports of canonical values and persistence API |
| `src/store/values.rs` | Shared timeline, grid, basin and provenance values agreed with step 2b |
| `src/store/observations.rs` | Station/series/sample/review/offset values, validated SQLite reads and transactions |
| `src/store/sql/observations_v1.sql` | Version-1 DDL below, embedded with `include_str!` |
| `src/store/netcdf.rs` | Shared canonical grid/forcing persistence, through qualified Terrio only |
| `src/analysis.rs` | Owner facade; exports pure preparation/QC functions |
| `src/analysis/observations.rs` | Preferred selection, review application, QC and preparation orchestration on values |
| `src/analysis/spatial.rs` | Shared grid construction and two explicitly named IDW algorithms |
| `src/app/prepare.rs` | Resolve selection/settings, read store, invoke analysis, save outputs |
| `src/app.rs`, `src/lib.rs`, `src/main.rs` | Declare app submodule, explicit public APIs, thin preparation subcommand |
| `tests/observations.rs`, `tests/observed_forcing.rs` | Persistence/review and numerical behavioral fixtures |
| `tests/architecture.rs` | New public facade names and allowed source/attribute syntax fixtures |

Preferred-series selection is pure analysis over supplied series metadata;
store queries return eligible candidates and samples, never decide QC or reviews.
App filters explicit provider/station selection before calling it and requests
samples for the resulting series IDs. No adapters module is needed here.
Canonical spatial types and NetCDF implementation are shared with step 2b,
not duplicated under each forcing track. Model reads `BasinForcing` in step 3.

Add `rusqlite` with `bundled` for the one SQLite implementation, and `chrono`
plus `chrono-tz` for parsing, arithmetic and IANA operation zones. Pin qualified
versions in Cargo.lock; do not invent a version before build qualification.
IDW uses Rust f64 arithmetic and standard sorting/hypot; no GIS or interpolation
dependency. NetCDF uses step 2b's qualified Terrio dependency and feature set.
Use the shared hashing dependency selected for step 3 provenance.

The architecture gate already recognizes store/analysis owners but its interface
root allowlist currently admits only inspection APIs. Add precisely the new
public API names and positive/negative fixtures; retain all dependency rules.
The current gate does not support `include_str!`. Extend its macro visitor
only for `include_str!("sql/observations_v1.sql")` inside
`src/store/observations.rs`: require one static string literal, resolve it
relative to the invoking source, require that exact owned schema file, and
reject interpolated/generated paths, traversal, cross-owner paths and Rust
source inclusion. Add one accepted own-schema fixture and rejected nonliteral,
parent traversal, foreign-owner and `.rs` fixtures. Keep `include!` and generated
Rust rejected. No root workflow implementation or hidden cross-owner
re-exports. Preserve existing inspection's read-only behavior and tests.

## Canonical observations SQLite v1

Use an explicitly selected observations path, conventionally
`data/observations.sqlite`; do not reinterpret the existing optional
`data/history.sqlite` inspection path as this format. `PRAGMA user_version=1`,
foreign keys ON on every connection. Create/migrate explicitly through store,
never by preparation's read-only query. Reject unrecognized versions before
reading. All reads needed for one selection run in one read transaction.
All writes validate then commit in one transaction; failed batches change none
of the existing rows. No cascading deletion of retained raw observations.

The following table definitions are the required logical DDL. All IDs are
nonempty TEXT. Canonical time labels are operation-local ISO
`YYYY-MM-DDTHH:MM:SS`, parsed as `chrono::NaiveDateTime`; one validated IANA
`time_zone` labels the database. Compare fixed-width local labels directly and
preserve observations/results without UTC conversion. No epoch shadow columns
or fake UTC instants are stored. Forecast adapters convert UTC once into this
same local convention. This increment rejects requested periods crossing an
ambiguous/nonexistent local hour rather than silently merging distinct samples;
step 1 verifies the saved operation has no such transition.

| Table | Columns, keys and constraints |
| --- | --- |
| `metadata` | `key TEXT PRIMARY KEY`, `value TEXT NOT NULL`; required `format=mgb-ops-observations`, `time_zone`; schema version lives only in PRAGMA user_version |
| `source` | `source_id TEXT PRIMARY KEY`, `kind TEXT NOT NULL`, `identity TEXT NOT NULL`, `sha256 TEXT`, `details_json TEXT NOT NULL`; hash required when identity denotes a retained file; JSON object preserves original IDs/units and converter identity |
| `station` | `station_id TEXT PRIMARY KEY`, `provider_code TEXT NOT NULL`, `station_code TEXT NOT NULL`, `name TEXT NOT NULL`, `latitude REAL`, `longitude REAL`, `source_id TEXT NOT NULL REFERENCES source`; `UNIQUE(provider_code,station_code)`; coordinates both NULL or both present, within geographic ranges |
| `observation_series` | `series_id TEXT PRIMARY KEY`, `station_id TEXT NOT NULL REFERENCES station`, `variable TEXT NOT NULL CHECK IN ('rain','level','flow')`, `state TEXT NOT NULL CHECK IN ('approved','curated','raw')`, `unit TEXT NOT NULL`, `interval_seconds INTEGER`, `created_at TEXT NOT NULL`, `source_id TEXT NOT NULL REFERENCES source`; rain requires mm and interval_seconds=3600 in this increment; level integer cm and flow m3/s are instantaneous and interval_seconds NULL |
| `observation` | `series_id TEXT NOT NULL REFERENCES observation_series`, `observed_at TEXT NOT NULL`, `value NUMERIC`, `PRIMARY KEY(series_id,observed_at)`; NULL means recorded missing, absent row means no recorded sample |
| `review_set` | `review_set_id TEXT PRIMARY KEY`, `created_at TEXT NOT NULL`, `source_id TEXT NOT NULL REFERENCES source`, `description TEXT NOT NULL`; retained sets immutable once used |
| `observation_replacement` | `review_set_id TEXT NOT NULL REFERENCES review_set`, `station_id TEXT NOT NULL REFERENCES station`, `variable TEXT NOT NULL CHECK(variable='rain')`, `observed_at TEXT NOT NULL`, `value REAL`, `reason TEXT NOT NULL`, `PRIMARY KEY(review_set_id,station_id,variable,observed_at)`; row with NULL is explicit removal; numeric zero is valid |
| `station_exclusion` | `review_set_id TEXT NOT NULL REFERENCES review_set`, `exclusion_id TEXT NOT NULL`, `station_id TEXT NOT NULL REFERENCES station`, `start_at TEXT NOT NULL`, `end_at TEXT NOT NULL`, `reason TEXT NOT NULL`, `PRIMARY KEY(review_set_id,exclusion_id)`, CHECK(start_at<end_at) |
| `station_offset` | `station_id TEXT PRIMARY KEY REFERENCES station`, `mini_id INTEGER NOT NULL CHECK(mini_id>0)`, `offset_m REAL NOT NULL`, `source_id TEXT NOT NULL REFERENCES source`; one current record per station |

Implement the following DDL (Rust validation additionally enforces nonempty IDs,
finite floats, complete coordinate pairs and local timestamp syntax):

```sql
CREATE TABLE metadata(key TEXT PRIMARY KEY,value TEXT NOT NULL);
CREATE TABLE source(source_id TEXT PRIMARY KEY,kind TEXT NOT NULL,
 identity TEXT NOT NULL,sha256 TEXT,details_json TEXT NOT NULL);
CREATE TABLE station(station_id TEXT PRIMARY KEY,provider_code TEXT NOT NULL,
 station_code TEXT NOT NULL,name TEXT NOT NULL,latitude REAL,longitude REAL,
 source_id TEXT NOT NULL REFERENCES source(source_id),
 UNIQUE(provider_code,station_code),
 CHECK((latitude IS NULL AND longitude IS NULL) OR
 (latitude IS NOT NULL AND longitude IS NOT NULL AND
 latitude BETWEEN -90 AND 90 AND longitude BETWEEN -180 AND 180)));
CREATE TABLE observation_series(series_id TEXT PRIMARY KEY,
 station_id TEXT NOT NULL REFERENCES station(station_id),
 variable TEXT NOT NULL CHECK(variable IN ('rain','level','flow')),
 state TEXT NOT NULL CHECK(state IN ('approved','curated','raw')),
 unit TEXT NOT NULL,interval_seconds INTEGER,created_at TEXT NOT NULL,
 source_id TEXT NOT NULL REFERENCES source(source_id),
 CHECK((variable='rain' AND unit='mm' AND interval_seconds=3600) OR
 (variable='level' AND unit='cm' AND interval_seconds IS NULL) OR
 (variable='flow' AND unit='m3/s' AND interval_seconds IS NULL)));
CREATE TABLE observation(series_id TEXT NOT NULL REFERENCES observation_series(series_id),
 observed_at TEXT NOT NULL,value NUMERIC,
 PRIMARY KEY(series_id,observed_at),
 CHECK(value IS NULL OR typeof(value) IN ('integer','real')));
CREATE TABLE review_set(review_set_id TEXT PRIMARY KEY,created_at TEXT NOT NULL,
 source_id TEXT NOT NULL REFERENCES source(source_id),description TEXT NOT NULL);
CREATE TABLE observation_replacement(
 review_set_id TEXT NOT NULL REFERENCES review_set(review_set_id),
 station_id TEXT NOT NULL REFERENCES station(station_id),
 variable TEXT NOT NULL CHECK(variable='rain'),observed_at TEXT NOT NULL,
 value REAL,reason TEXT NOT NULL,
 PRIMARY KEY(review_set_id,station_id,variable,observed_at));
CREATE TABLE station_exclusion(
 review_set_id TEXT NOT NULL REFERENCES review_set(review_set_id),
 exclusion_id TEXT NOT NULL,station_id TEXT NOT NULL REFERENCES station(station_id),
 start_at TEXT NOT NULL,end_at TEXT NOT NULL,reason TEXT NOT NULL,
 PRIMARY KEY(review_set_id,exclusion_id),CHECK(start_at<end_at));
CREATE TABLE station_offset(station_id TEXT PRIMARY KEY REFERENCES station(station_id),
 mini_id INTEGER NOT NULL CHECK(mini_id>0),offset_m REAL NOT NULL,
 source_id TEXT NOT NULL REFERENCES source(source_id));
CREATE INDEX observation_time ON observation(observed_at,series_id);
CREATE INDEX observation_selection ON
 observation_series(station_id,variable,state,created_at);
PRAGMA user_version=1;
```

Add index `observation(observed_at,series_id)` and
`observation_series(station_id,variable,state,created_at)`. Reject NaN/infinity
in every populated floating field in Rust; nullable fields must use Option, not
floating NaN as an internal sentinel. Do not rely on SQLite NaN coercion to NULL.
Nonnegative rainfall is not an ingestion constraint: preserve finite suspicious
raw measurements for review; QC's negative-value policy is not expanded here.
Unknown units, invalid interval lengths, duplicate keys, orphan references,
malformed JSON and inconsistent timestamp metadata fail with identified row/key.

Canonical levels are signed integer centimetres, exactly as selected by the
user. Use `ObservationValue::LevelCm(i32)` in memory and SQLite INTEGER physical
values. Rainfall/flow remain `ObservationValue::RainMm(f64)` and
`ObservationValue::FlowM3s(f64)`. The common `observation.value NUMERIC` column
preserves integer physical storage (REAL affinity would coerce level inserts to
REAL). On every read/write, join the parent series variable: level requires
SQLite `typeof(value)='integer'` and i32 range; rain/flow accept integer or real
numeric storage and decode to f64 with finite validation. NULL remains missing.
Reject nonintegral level storage rather than rounding during canonical reads.
SQL's numeric-type CHECK rejects text/blob values; Rust's variable-aware
validation is required because a SQLite CHECK cannot join the series table.
Use rusqlite integer binding for levels and verify `typeof` after insertion;
never route LevelCm through an f64 binding. NUMERIC affinity may store integral
rain/flow values as INTEGER, which is acceptable and still decodes as f64.

The regression converter preserves already integral legacy centimetres directly.
For fractional finite legacy cm, round to nearest integer centimetre with ties
to even, then range-check i32; reject overflow, infinity and NaN without clipping.
Record source unit/value and conversion/rounding policy in converter provenance,
including affected-row count and maximum change. Example expectations: 1.5→2,
2.5→2, -1.5→-2, -2.5→-2, -0.4→0 cm. Conversion from other source units belongs
to future adapters, never implicit canonical read coercion. Signed i32 NetCDF
level storage and explicit missing masks follow the shared result contract;
no f64 placeholder level array is canonical. Station `offset_m` stays finite
signed f64 metres, preserving fractional-centimetre reference precision for
model's conversion-before-final-rounding. No level conversion is applied during
rainfall preparation. Coordinates may be
unknown for level-only stations; selected rainfall stations require finite
coordinates. Geometry validation checks each supplied offset mini_id against the
selected basin IDs in app/model preparation, not an unenforceable cross-file FK.

The nine tables above each serve a current boundary: station/series/sample retain
selection and original values; review_set groups the explicitly selected saved
instructions; replacement and exclusion represent distinct point/interval
semantics; station_offset supplies model reference conversion; source deduplicates
file/converter provenance shared by stations/series/reviews/offsets; metadata
holds only format identity and timezone. No operation/artifact registry is added
here: step 3 owns that separate SQLite. No second schema-version field exists.

## In-memory and callable contract

Store owns `Station`, `ObservationSeries`, `ObservationSample`, `ReviewSet`,
`ObservationReplacement`, `StationExclusion`, `StationOffset`, and
`ObservationInputs`. Each contains the columns above in typed form, with
`Option<ObservationValue>` missing observations and explicit source references;
rainfall review replacement/effective values remain `Option<f64>`. `ObservationInputs`
contains station/series/sample vectors, selected review set and source records;
it is detached from SQLite connections and usable in ordinary unit tests.
`EffectiveObservation` additionally retains `raw_value`, `raw_present`,
`effective_value`, `replacement` (none/null/numeric), `excluded`, chosen
`series_id`, and applied instruction IDs. Never overwrite the raw vector.

Public store entry points: `ObservationStore::open_read_only(path)`,
`read_selection(selection, interval, review_set_id)`,
`read_station_offsets(station_ids)`, `create(path,time_zone)` and
`write_batch(batch)`. `read_selection` opens one read transaction and loads eligible station/series
candidates plus all their samples within the interval, the selected review rows,
source records and selected station offsets into detached owned vectors before
ending the transaction. Pure analysis ranks candidates and discards nonselected
series samples in memory. This bounded extra read prevents a change between
series ranking and sample loading; no SQLite connection/transaction crosses the
analysis boundary. Return offsets alongside inputs to app for its snapshot.
Creation/batch persistence support canonical producers and
fixture validation; no Python-schema reader. Public analysis entry points:

- `select_observation_series(candidates, explicit_series_ids)` returns one
  selected rain series per station or a structured ambiguity error.
- `detect_suspect_precipitation(samples, policy)` returns QC findings.
- `effective_observations(inputs, timeline)` returns effective values and review
  application diagnostics.
- `prepare_observed_forcing(inputs, timeline, grid_spec, basin_geometry,
  interpolation, qc_policy)` returns `ObservedPreparation` containing findings,
  effective rows, working `GridSeries`, historical `BasinForcing`, and provenance.

Concrete public signatures (the owning facade re-exports these exact types):

```rust
pub fn select_observation_series(
    candidates: &[ObservationSeries], explicit_series_ids: &[String],
) -> Result<Vec<ObservationSeries>, ObservationError>;
pub fn detect_suspect_precipitation(
    samples: &[SelectedObservation], policy: &PrecipitationQcPolicy,
) -> Result<Vec<PrecipitationQcFinding>, ObservationError>;
pub fn effective_observations(
    inputs: &ObservationInputs, timeline: &IntervalAxis,
) -> Result<Vec<EffectiveObservation>, ObservationError>;
pub fn prepare_observed_forcing(
    inputs: &ObservationInputs, timeline: &IntervalAxis,
    grid: &RegularGridSpec, basins: &BasinGeometry,
    interpolation: &IdwSettings, qc: &PrecipitationQcPolicy,
) -> Result<ObservedPreparation, ObservationError>;
```

`IntervalAxis { time_zone: String, bounds: Vec<TimeInterval> }` contains
`TimeInterval { start: chrono::NaiveDateTime, end: chrono::NaiveDateTime }`.
SQLite uses the local labels above. NetCDF time and bounds use integer seconds
relative to a recorded operation-local origin and a separate IANA timezone
attribute; never label these numbers as UTC epoch seconds.
`RegularGridSpec { requested_bbox: [f64;4], resolution_degrees: f64,
latitudes: Vec<f64>, longitudes: Vec<f64>, effective_bbox: [f64;4] }` stores
validated coordinates rather than rebuilding floating axes during reading.
`IdwSettings { nearest_stations: usize, power: f64 }`.
`BasinGeometry { mini_ids: Vec<i64>, longitudes: Vec<f64>, latitudes: Vec<f64>,
source: Provenance }` preserves explicit order.
`GridSeries { timeline: IntervalAxis, grid: RegularGridSpec, values: Vec<f64>,
valid: Vec<bool>, provenance: Provenance }` and
`BasinForcing { timeline: IntervalAxis, basins: BasinGeometry, values: Vec<f64>,
valid: Vec<bool>, provenance: Provenance }` validate vector lengths by checked
multiplication before indexing. Invalid-mask values serialize using the shared
NetCDF fill convention; in-memory consumers consult validity, never assume zero.

`ObservationInputs { stations: Vec<Station>, series: Vec<ObservationSeries>,
samples: Vec<ObservationSample>, review: ReviewSet, sources: Vec<Source> }`;
`ObservationSample { series_id: String, observed_at: chrono::NaiveDateTime, value: Option<ObservationValue> }`;
`StationOffset { station_id: String, mini_id: i64, offset_m: f64, source_id: String }`.
`ObservationValue` is the enum `RainMm(f64) | FlowM3s(f64) | LevelCm(i32)`.
Its variant must match the referenced series variable. ReviewSet includes its replacement/exclusion vectors; selected-observation rows
join sample, station/series IDs and coordinates before QC. `ObservedPreparation`
contains `effective: Vec<EffectiveObservation>`, `qc: Vec<PrecipitationQcFinding>`,
`working: GridSeries`, `basin: BasinForcing`, and `provenance: Provenance`.
App request/report stay owned by app; canonical inputs/results stay owned by store.

All return typed Result values identifying station/series/time or target on
failure; no printing, current-clock selection, registry lookup or filesystem
access in analysis. QC findings are successful diagnostic outputs, not automatic
replacements or fatal errors. Missing usable coverage is a preparation error.

`GridSeries` uses time-major `[time,latitude,longitude]` f64 values and explicit
validity mask; ascending coordinates; EPSG:4326; operation timezone and explicit
(start,end] bounds. `BasinForcing` uses `[time,basin]`, explicit mini_id vector,
centroid coordinates and identical time bounds. Prepared forcing requires every
value finite/valid. These shared names/layouts must agree with step 2b and
step 1's comparison checkpoints before either track is accepted.

## Exact scientific processing

1. Filter the explicitly requested providers and/or stations; an explicitly
   selected unknown ID is an error. Rank eligible series independently for each
   station/variable: approved, curated, raw, then created_at descending.
   An explicit series ID overrides ranking but must belong to the selected
   station/variable. Equal winning rank and creation time is an ambiguity error
   requiring explicit selection; do not make database scan order a policy.
   Never fill a preferred-series gap from a lower-ranked series.
2. Read rainfall endpoints strictly after historical start and through historical
   end, inclusive. This increment supports already normalized hourly rain totals
   only: each sample means `(endpoint-3600s,endpoint]`, units mm. Require endpoints
   on the requested hourly timeline. Reject off-grid samples and wrong interval
   metadata; do not aggregate, split, or infer raw gauge periods here.
3. Run QC on selected raw samples. Default policy: value strictly >50 mm;
   sequence length >=5 with each value strictly >0 and <1 mm and endpoint
   differences exactly one hour within a station. Missing samples, equal bounds,
   nonqualifying samples and gaps break sequences. A sequence finding is emitted
   for every member; station/start/ordinal identifies its sequence. Findings are
   ordered station, endpoint, type. Threshold/lower/upper must be finite,
   threshold>=0, 0<=lower<upper and minimum length>=1. Request fields are `high_threshold_mm`, `sequence_lower_mm`,
   `sequence_upper_mm`, and `sequence_min_length`. Policy is explicit and
   persisted. No implicit review instructions are generated.
4. Apply the one explicitly selected review set: replacements first, exclusions
   second. A NULL replacement removes availability; zero remains usable. A
   numeric replacement may insert a missing sample only for an explicitly
   selected preferred station/series, using its known metadata; retain
   raw_present=false. Instructions outside the selected time window remain
   retained but are reported as out of selection. Unknown/unselected stations
   in an in-window replacement fail. Exclusion means `(start,end]`; overlapping
   exclusions compose by union and override replacements. Report counts and
   instruction IDs actually applied, inactive, and inserted. No-review is an
   explicit empty review set, never a mutable 'latest' review lookup.
5. Construct the shared working grid with closed cell footprints touching the
   requested bbox. For axis [a,b] and resolution r>0, n=floor((b-a)/r)+2 and
   center[i]=a-r/2+i*r for i=0..n-1. Effective edges are first center-r/2 and
   last center+r/2. Require finite ordered geographic bounds/resolution. Preserve
   requested and effective bbox separately. For the saved bbox/r this yields
   83 latitude rows and 102 longitude columns; verify against the checkpoint.
6. Station-to-grid: for each hourly endpoint discard missing effective values
   and invalid coordinates. Compute Euclidean distance in longitude/latitude
   degrees (no projection/geodesic substitution). Select min(k,usable_count)
   nearest stations, k>=1; power finite >0, fixture k=5/p=2. For nonzero
   distances compute sum(value/d^power)/sum(1/d^power). For exact coincidence
   return mean of selected coincident values. No usable station at an endpoint
   is an error identifying that endpoint, never a zero field.
7. Grid-to-basin: flatten grid latitude-major, longitude-fast; preserve supplied
   explicit basin order. Precompute nearest min(k,grid_count) indices by degree
   distance, replacing exactly zero distance by 1e-12 for weights d^-power.
   Apply weights each time and renormalize over finite selected cells. Zero
   usable weight is a target/time coverage error. The working grid must already
   have complete finite coverage; this check catches malformed reused inputs.
   Process chunks without changing arithmetic/source order; chunk size affects
   memory only. Do not serialize f32 engine rainfall in analysis.

Legacy NumPy argpartition/argsort does not specify a portable tie choice when
more than k sources are equally distant. Rust uses distance then station_id
(station kernel) or flattened grid index (basin kernel) ascending. Step 1 must
record tied cutoffs in the saved fixture and compare the selected neighbors.
If tied unequal values change results, qualify and document that scientific
difference explicitly; do not enlarge tolerances to hide it. Preserve the two
coincidence rules above rather than replacing both with a generic IDW helper.

## Persistence and app/CLI integration

Persist `observed-working.nc` and `observed-basin.nc` using the shared canonical
NetCDF v1 layout in step 2b: unpacked f64 precipitation in mm, explicit mask,
coordinates/IDs, interval bounds and operation timezone. Keep scientific
checkpoints before any packing; no 0.001 rounding in this pipeline. Save selected
raw/effective rows and QC findings with the preparation's retained canonical
observation SQLite subset; effective rows/findings may be deterministic JSON
checkpoints rather than extending the authoritative source schema.

Provenance records chosen station/series IDs and metadata, source hashes and
retained identity, explicit review-set rows, offset records, operation timezone,
requested timeline, original units, grid requested/effective bounds, coordinate
order, both IDW algorithm identifiers/k/power/tie policy, geometry ID/hash,
implementation/schema version, applied/inactive instructions, and output hashes.
Step 3 registers these files in the prepared manifest and retains them at
execution startup. A mutable source path plus checksum is not the snapshot.

The integrated `app::prepare(request)` workflow in step 3 resolves the
observations selection/settings, reads canonical inputs in one store read transaction, invokes this track's
pure `prepare_observed_forcing`, and asks store to save its artifacts once for
all scenarios. Expose the analysis API independently for callers/tests using
in-memory canonical values; no separate observation-only application command is
needed. Step 3's `mgb-ops operation prepare` CLI exercises this same path.

The observed section of step 3's strict preparation request contains observations
path, review_set_id (or explicit no-review), provider_codes/station_ids/optional
series_ids, time_zone, interval_start/end (operation-local ISO labels),
step_seconds=3600, geometry selection, bbox, resolution_degrees, nearest_stations,
power and QC policy. Require at least one explicit provider/station selector,
nonempty selected stations and explicit output. Resource paths follow existing
workspace semantics. App adds a dedicated preparation request type with
`deny_unknown_fields`; existing inspection Config remains unchanged. main.rs
adds only the integrated preparation subcommand; src/lib.rs and the architecture
allowlist expose the integrated workflow and intended canonical/analysis APIs.
Step 3 owns staging/new-destination checks and complete preparation publication;
failure preserves source SQLite and prior preparation. No observation-specific
publication protocol or duplicate manifest format is introduced here.

## Fixture conversion, implementation sequence, and acceptance

1. Step 1 retains original evidence and creates the revised target
   `/workspace/regression-rs-hourly-r2`. Add `convert-observations.py` there only,
   never a Rust runtime Python dependency. Open frozen original SQLite read-only,
   inspect its declared variable units, preserve source IDs/state/creation times,
   retain local timestamps and attach the declared operation timezone, and write a
   new canonical observations.sqlite. Retain source file SHA256, command and
   script hash. Copy the six saved review rows explicitly; do not rerun QC as
   automatic authorization. Import necessary level station metadata/offsets from
   the qualified VB.NET source with station identity mapping and provenance.
2. Implement store schema/typed validation/read-write round trips; synthetic
   tests cover level `typeof(value)=integer` after persistence, exact negative/zero/i32 bounds round trips, fractional canonical level rejection, nearest/ties-even converter examples above, positive/negative i32 overflow rejection, and NULL distinct from zero; also FK failures, duplicate IDs/endpoints, invalid units/nonfinite
   values, version rejection, rollback, explicit zero and absent offsets,
   local-label/timezone preservation and original-source immutability.
3. Implement pure ranking/review/QC, then shared grid and both IDW kernels.
   Tests cover ranking with gaps/ties/explicit IDs; NULL versus zero; numeric
   insertion; exclusion endpoints/precedence; threshold equality; five versus
   four low samples; 0/1 bounds; missing hour; unknown station; off-grid time;
   no source; fewer than k sources; exact coincidences; tied kth neighbors;
   row/column ordering; closed footprint; missing-weight renormalization; basin
   order; and identical chunked/unchunked values.
4. Add Terrio persistence and app/CLI wiring. Synthetic numerical expectations include two sources (0,0)=2 and (2,0)=6: target (1,0) with k=2/p=2 gives 4; target (0,0) in station interpolation gives exactly 2. A single usable source of 0 gives an all-zero valid field. Closed bbox (0,0,2,1), r=1 gives longitude [-0.5,0.5,1.5,2.5], latitude [-0.5,0.5,1.5], effective bbox [-1,-1,3,2]. Use temporary synthetic canonical
   files to prove round trips retain complete arrays/masks/times/IDs and staging
   failure never replaces an existing preparation. Library tests run without
   SQLite connections for analysis; integrated preparation CLI succeeds without Python/providers/MGB.
   Re-run the existing inspection immutability tests.
5. Convert/review saved fixture and compare complete selected/raw/effective
   observations, QC matches, the six NULL rows, coordinates, working grid and
   basin forcing with step 1 checkpoints. Require exact identity/time/masks and
   qualified full-array numerical tolerances. Separate raw f64 algorithm
   differences from any historical packed NetCDF quantization. Require every
   selected historical endpoint and basin covered, and exact common forcing
   for all scenario consumers. Offsets round-trip signed/zero; missing remains
   unknown; two stations sharing a mini may retain different offsets.
6. Run formatting, Clippy, full Rust tests and independent architecture gate as
   [required](../../AGENTS.md). Update supported data/usage contracts and
   [migration checklist](../migration/summary.md) only when implemented.

Step 2a acceptance supplies reusable historical `BasinForcing`, its working-grid
checkpoint, immutable selections/reviews/provenance, and station/offset records.
Engine boundary timing and measured tolerances remain step 1 qualification gates;
Terrio NetCDF capability remains step 2b qualification. These are explicit
prerequisites, not permission to invent scientific behavior during implementation.
Keep this specification until that evidence and scope are accepted.
