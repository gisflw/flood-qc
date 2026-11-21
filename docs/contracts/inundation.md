# Inundation masks

MGB Ops can produce a modeled inundation mask for a successful scenario from
the RS engine layout. The input rasters are regional workspace files. Processing
uses the chosen run, scenario, and raster paths directly.

## Required rasters

- A single-band HAND GeoTIFF with heights in metres above the floodplain
  reference.
- A single-band integer catchments GeoTIFF on the same pixel grid and CRS.
  Positive pixel values are model MINI IDs. Zero and NoData provide no coverage.

Both grids must be north-up and declare SAD69 (`EPSG:4618`) or WGS84
(`EPSG:4326`). Dimensions and affine alignment must match. An unknown positive
catchment ID fails validation. Use `0` or a NoData value below `2^53` in magnitude
for 64-bit integer catchment grids. Provision these static rasters once per regional
model. See [RS raster provisioning](../../references/regression/rs-hourly/inundation.md)
for the sources and assumptions used for RS Hydro.

## Produce a mask

Build the processor with the `inundation-write` feature and the netCDF C library.
For example, on Debian or Ubuntu install `libnetcdf-dev` and `pkg-config`, then
build with `cargo build --locked --release --features inundation-write`.

```sh
mgb-ops inundation process \
  --run /region/data/cache/run/<generation> --scenario <scenario-id> \
  --hand /region/data/source/HAND_Flood.tif \
  --catchments /region/data/source/catchments.tif
```

Use the `run_root` returned by execution or the active generation selected in
`data/cache/run/active.json`. The chosen scenario must be complete and have `results/<scenario>/flood_source.nc`.
Supported new runs capture this compact source from the history-only and
scenario engine streams. It retains the time and MINI axes and the channel
depths supplied to that run. Existing runs without the source remain available
for their other results. Run the operation again to capture it.

The processor writes `results/<scenario>/flood_mask.nc`. Add `--json` for the
structured processing result. Repeating the same inputs reuses the existing
product. Use `--replace` to publish a product from changed inputs. Failed or
cancelled processing leaves any existing product available.

## Mask and display

`inundation_mask(time, y, x)` is a categorical NetCDF-4 variable on the original
grid. `0` is eligible and dry, `1` is wet, and `255` is excluded or unavailable.
Wet cells must have `HAND > 0`, valid coverage, and a modeled stage strictly
above their HAND height. On the RS layout the stage in this comparison is
`HANDTUDO − channel_depth`. Cells with HAND equal to zero remain excluded at
every hour. Time coordinates use
the run's local valid times. The desktop shows the timezone in the bottom strip. The
product records the input identities and threshold method. Coarser display
overviews represent wet area as a fraction of each block.

Open **Map → Modeled inundation** in the desktop and choose a scenario from
the active run. Use the arrows to step one hour, drag the slider, or enter a local
valid hour as `YYYYMMDD-HH` and press Enter or **Go**. The controls show the
selected hour and the forecast boundary. The display reads the mask independently
of the source rasters. Both NetCDF files are included in a shared bundle when
present at export.
When the workspace replaces the active run, it retires its old masks and flood
sources. The attempt manifest retains the input paths and digests for reprocessing.
