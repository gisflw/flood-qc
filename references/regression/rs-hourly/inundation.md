# RS Hydro inundation evidence

## Static rasters

The regional `rs_hydro_forecast` workspace supplies the original ASCII grid at
`mgb_runner/Input/HAND_Flood.asc` and the `mini_catchments` layer in
`data/source/rs_hydro.gpkg`. The one-time GDAL provisioning writes tiled
`data/source/HAND_Flood.tif` and `data/source/catchments.tif`. The first retains
the native affine grid, values and `-9999` declaration. The second uses pixel
centres to assign the polygons' `mini_id` on that exact grid. The TIFFs and
their input identities are stored with regional data.

The ASCII grid has no declared CRS. The RS fallback assigns SAD69 from the
catchment geometry. Across 58,634,875 positive-HAND cells, the SAD69 assignment
left 10,085 unassigned, compared with 27,142 when the raster was assumed WGS84
and the polygons transformed to that CRS. The two assignments disagreed on
1,071,084 cells, including cells unassigned under one assumption.

## Engine and threshold

In the frozen case `HANDTUDO.MGB` contains 8,649 little-endian `float32`
depths per row. The history-only run supplies 241 historical rows. Each
scenario places its 121 forecast rows at the beginning of a history-sized
stream and retains the unaffected history suffix. Across the case, `HANDTUDO`
equals the raw stage minus the channel-bed elevation, clipped at zero. The
engine's float32 bed reference is `Z0 − channel_depth`, with `Z0` supplied by
`COTA_AREA.FLP` and channel depth supplied by the run's `Cell.hig`.

The mask uses `HANDTUDO − channel_depth` on positive-HAND cells, assuming the
HAND heights share the `Z0` reference. With the rasterized catchments, the
per-MINI area gained from 1 to 10 m above `Z0` correlated at 0.93 with the
model's elevation–area curves. A one-metre shift improved the fit slightly.
The threshold uses no empirical shift. The original preprocessing catchment
raster and a trusted dated footprint are unavailable. These checks support
internal consistency of the modeled estimate. The geographic footprint and
vertical cutoff remain unverified against an independent flood map.

## Linux qualification

The [saved RS regression](rust-qualification.md) now verifies the assembled
flood sources in both real-engine repeats against the original history and
forecast streams. An independent NetCDF read checked 12,523,752 captured
depth values across four scenarios in one repeat.

The `ecmwf-raw` product retained all 362 hourly valid times in the run's
`America/Sao_Paulo` timeline. Independent NetCDF and GeoTIFF reads reproduced
every cell at the first and last history hours and the first and last forecast
hours. There were 58,624,790 eligible cells per frame. Wet cells numbered
865,809 at the first history hour, 866,099 at the boundary and first forecast
hour, and 868,376 at the last forecast hour. The comparison also checked
overview block counts against the native mask. Processing and full readback
took 247 seconds with 1.12 GiB peak RSS. The compressed file was 1,461,195,985
bytes. With the release desktop reader and local caches, metadata opened in
1.5 ms, the complete 16× overview read in 6.6 ms, and a native 256×256 tile
took 41–44 ms across the history boundary and late forecast. Bundle export and
import preserved the run manifest, flood sources, and mask byte for byte.

The native desktop on Linux Xvfb with software Vulkan showed the missing-mask
state, loaded the published result after refresh, and drew the layer in Single,
Double, and Swipe. Dragging reached the last forecast hour. The Left key then
stepped back one hour with the new time shown in the run timezone. This
graphics check used software rendering.

## Reusing unchanged tiles

The raw ECMWF run had 39,180 eligible cells that changed state at least once.
Only 350 of 395,295 adjacent hours across eligible 256 × 256 tiles changed a
cell. The processor now computes stages once per MINI and hour, classifies
static cells once, and recalculates tile overviews when a cell changes. The
same saved case took 137 seconds including full readback, down from 232
seconds in the earlier RS Hydro workspace run. All 362 native masks and both
overview series matched the earlier output exactly. The file size was unchanged.
Independent reads matched the original HAND depths, all valid times, and three
complete native frames per scenario across the four-scenario RS Hydro run. All
four masks opened through the desktop scenario selector on Linux software Vulkan.

The saved target workspace uses the original July 20 run as its single active
generation. The later copy supplied the four flood sources and masks after the
canonical results and all 16 engine streams matched byte for byte. Migration
preserved the original run manifest and each mask's embedded provenance. The
desktop opened the active rainfall and charts and displayed all four masks.
Dragging the inundation slider reached July 25 01:00 local time on Linux Xvfb
with software Vulkan.
