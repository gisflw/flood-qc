# Saved forecast normalization and forcing

`forecast normalize` reads an explicitly selected saved GRIB2 file, returns
canonical native rainfall intervals and retains the original source plus a native
NetCDF artifact in a separate forecast catalog. Use [forecast acquisition](acquisition.md#forecast-acquisition)
to fetch and register a provider cycle.

```sh
mgb-ops forecast normalize --workspace /regional/workspace --request normalize.toml --json
```

Workspace and request-file arguments resolve from the invocation directory.
Paths inside the strict TOML request resolve from the workspace. Unknown keys,
unsupported schema/product/provider combinations and invalid settings fail.

```toml
schema_version = 1
provider = "ecmwf"
model = "ifs"
product = "fc"
cycle_utc = "2026-07-20T00:00:00Z"
operation_timezone = "America/Sao_Paulo"
source = "sources/ifs.grib2"
crop_bbox = [-79.13530, -50.61209, -28.67245, -10.08729]
catalog = "data/forecasts.sqlite"
artifact_root = "data/forecasts"
```

All keys are required. Supported pairs are `ecmwf`/`ifs` and `noaa`/`gfs`, product
`fc`. The cycle is RFC3339 UTC with whole seconds; the operation timezone is an
IANA name. Crops are finite geographic west/south/east/north bounds and cannot
cross the antimeridian. Retained artifacts must remain inside the workspace;
source files and the catalog may use explicit absolute paths.

JSON output is one report containing source/asset IDs, retained source/native
paths, native shape, effective bounds and canonical timeline. The CLI exits 0
on success, 1 on operation/output failure and 2 for invalid CLI arguments.
`app::normalize_forecast` exposes the same file-based workflow without printing.
`app::normalize_forecast_document` accepts a typed `NormalizeForecastDocument`
with the same fields and validation, resolving its paths from the supplied
workspace. Both entry points produce the same canonical values and catalog
identities; callers do not need to create a temporary request file.
`adapters::normalize_saved_forecast` returns canonical values and normalization
evidence independently of persistence.

## Native values and persistence

Normalization validates the requested cycle and accumulated surface rainfall
metadata. It selects start-zero cumulative fields, checks duplicates before
collapsing them, differences consecutive cumulative fields and clips finite
negative differences to zero. NOAA rolling fields do not participate in that
differencing. ECMWF metres convert to mm; NOAA kg/m2 totals are mm. Lead zero
provides initial cumulative state without creating a zero-duration interval.
Missing operands remain missing; infinities and conflicting duplicates fail.

Native fields retain their original interval lengths and increasing geographic
coordinates. Cropping retains cells whose footprints touch the requested bounds.
Canonical labels use the operation timezone; source cycle remains UTC. Local
axes crossing timezone-offset changes remain unsupported.

Terrio at the qualified revision owns GRIB/NetCDF I/O, with CCSDS and NetCDF/HDF5
features and no GDAL dependency. The adapter additionally checks raw GRIB2 grid
sections because the driver otherwise pads/crops later fields to the first grid.
Changed grids, malformed messages and unsupported multi-field messages fail.

`store::ForecastCatalog` owns a version-1 SQLite catalog with exactly `asset` and
`correction` tables. It preserves immutable source/native files, hashes, sizes,
normalization metadata and canonical coverage. Source IDs identify content and
normalization settings; asset IDs identify catalog records. These are distinct
from legacy asset labels. Loading verifies retained content and canonical schema.
Generated working grids and basin forcing are not additional catalog assets.

Corrections contain an asset ID, lead-hour bounds, latitude/longitude pixel
shifts, rotation, positive multiplier and audit fields. Callers select explicit
ordered correction IDs. Unknown/duplicate IDs and cross-asset selections fail.
`load_selected` reads the asset and selected correction values in one transaction.
Editor and reason are optional; supplied text is preserved. `open_writable`
opens an existing catalog without creating it. `register_corrections` commits a
selected batch atomically, reusing identical existing values and rejecting
conflicting values under an existing ID. See the shared
[draft registration workflow](drafts.md#explicit-registration) for selection and
independent catalog outcomes.

## Pure preparation

`analysis::prepare_forecast` accepts canonical native values, selected correction
values and an explicit target axis, working grid, ordered basins and IDW settings.
It has no database/provider access. Corrections apply in supplied order to native
intervals wholly inside their lead-hour window. Partial overlaps stay unchanged
and affected interval indices are reported. Shifts and nearest-pixel sampling use
nearest rounding with ties to even; translation precedes rotation, which expands
the output footprint and preserves missing cells. Corrections precede uniform
hourly splitting and separable linear working-grid interpolation. Preparation
fails missing time/spatial coverage and retains native/transformed checkpoints.
Basin interpolation preserves geometry order and supplied numerical settings.

`prepare_zero_forecast` constructs valid zero basin forcing on the supplied axis
without a provider artifact. It does not insert a catalog asset.

NetCDF stores unpacked f64 mm values, u8 validity masks, explicit coordinate and
interval-bound arrays, integer mini IDs, schema/provenance metadata and operation
timezone. Invalid rainfall payloads serialize as NaN with mask zero; valid zero
remains distinct. Time is i64 seconds from a recorded local origin, without a UTC
suffix. Readers validate values, masks, coordinate metadata, dimensions and schema.
Transformed frames persist separately because their footprints can differ.

Linux is the qualification platform for saved normalization and forecast
forcing. See [acquisition](acquisition.md) for the live provider workflow.
