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
`assets/objects/forecasts/`, beside lightweight metadata. Use
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

A forecast version identifies provider, model, product, UTC cycle, physical
grid, operation timezone and source identity. Its readable ID includes a digest.
Repeated registration of the same version reuses it after verification; revised
source data creates a new version and leaves prior versions available. Selection
of a product and cycle uses the newest registered matching version.

```text
assets/objects/forecasts/<asset-id>/
├── metadata.toml               Identity, coverage and checksums
├── native.nc                   Normalized native intervals and grid
└── source.nc or source.grib2  Retained source
```

Reuse validates the native artifact even after the original download is removed.
Derived scenario grids belong to preparations and results. Ordered edits are
[scenario parameters](operations.md#scenarios); the manifest retains their
values and base identities.

## Pure preparation

Preview and execution apply the [scenario transformation](operations.md#scenarios)
to the selected native base.
Basin interpolation preserves geometry order and supplied numerical settings.
The pure analysis layer has no provider or filesystem access.

`prepare_zero_forecast` constructs valid zero basin forcing on the supplied axis
without a provider artifact. It does not insert a catalog asset.

NetCDF stores unpacked f64 mm values, u8 validity masks, explicit coordinate and
interval-bound arrays, integer mini IDs, schema/provenance metadata and operation
timezone. Invalid rainfall payloads serialize as NaN with mask zero; valid zero
remains distinct. Time is i64 seconds from a recorded local origin, without a UTC
suffix. Readers validate values, masks, coordinate metadata, dimensions and schema.
Scenario NetCDF materializations are disposable; the working-grid footprint stays fixed.

Linux is the qualification platform for saved normalization and forecast
forcing. See [acquisition](acquisition.md) for the live provider workflow.
