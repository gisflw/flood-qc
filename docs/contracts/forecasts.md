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
`assets/forecasts/`, beside lightweight metadata. Use
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

Logical identity uses provider, model, product, cycle, physical grid and operation
timezone. Equivalent normalized values reuse one asset across source formats;
requested crop metadata remains provenance. Revised values require a forced
refresh.

Folder discovery reads metadata for provider, model, product, cycle, coverage, and
the retained file locations. Operators select forecasts by those labels.
Working grids and basin forcing belong to their preparation or run.
Ordered edits are [scenario parameters](operations.md#scenarios);
the operation manifest retains their values and base identities.

| Asset metadata | Purpose |
| --- | --- |
| Provider, model, product, UTC cycle | Operator selection and source-cycle provenance |
| Physical grid and operation timezone | Logical asset identity alongside provider labels and cycle |
| Valid interval coverage | Decide whether a base covers the requested forecast window |
| Normalized artifact path, byte length and SHA-256 | Verify the canonical NetCDF used by preparation |
| Source format, identity, retained path and digest | Retain source provenance independently of normalized-grid reuse |
| Normalization settings and crop metadata | Explain how the native canonical values were produced |

```text
assets/forecasts/<asset-id>/
├── metadata.json               Discovery, identity, coverage and integrity
├── native.nc                   Normalized native intervals and grid
└── ...                         Retained source payload where present
```

The tree illustrates the payload roles; a forced refresh can publish different
retained filenames through the metadata. Readers follow the recorded paths.
Reuse validates the normalized artifact and works after the original download
is removed. Derived scenario grids belong to preparations and results, outside
the acquired asset catalog.

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
