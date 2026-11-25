# Saved forecast normalization and forcing

Normalize a saved GRIB2 payload using the workspace time zone, spatial bounds
and acquisition buffer.

```sh
mgb-ops forecast normalize --workspace /region --source /region/downloads/ifs.grib2 \
  --provider ecmwf --model ifs --product fc --cycle 2026-07-20T00:00:00Z
```

Use `ecmwf` with `ifs` or `noaa` with `gfs`, product `fc`. The cycle
uses whole seconds in UTC. Workspace and source arguments resolve from the
invocation directory. Registration stores the normalized NetCDF under
`assets/` and references it through `operation.sqlite`. Use
[forecast acquisition](acquisition.md#forecast-acquisition) to fetch provider
cycles directly.

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

The workspace catalog indexes provider, model, product, cycle, coverage, and
the retained file locations. Operators select forecasts by those labels.
Working grids and basin forcing belong to their preparation or run.
Forecast corrections are [scenario parameters](operations.md#scenarios)
and their values are retained in the operation manifest.

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
