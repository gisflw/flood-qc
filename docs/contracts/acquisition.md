# Acquisition and input updates

Acquire ANA observations, ECMWF IFS precipitation, and NOAA GFS precipitation in a
selected workspace. The catalogs feed the existing observation and forecast
selection workflows.

```sh
mgb-ops observation acquire --workspace /regional/workspace --json
mgb-ops observation import-ana-csv --workspace /regional/workspace --input saved/observed.csv --json
mgb-ops forecast acquire --workspace /regional/workspace --json
mgb-ops operation run-all --workspace /regional/workspace --json
```

In desktop Processing, **Acquire inputs** exposes the first three actions and
**New run → Run All** acquires the configured inputs and executes the run. Jobs shows provider
progress, committed results, and failures. Leave the reference blank to use
`run.reference_time`. When that setting is `now`, acquisition uses the current
local hour. Supply a local, hour-aligned `YYYY-MM-DDTHH:00:00` value with
`--reference-time` to override it. Cancellation stops after
the current bounded request or write and preserves committed data. NOAA spacing
is interruptible.

Workspace commands read `config.toml`. Set the enabled providers under
`[acquisition]` and the forecast bounds under `[spatial]`. Defaults are 90
observation days, 30 days per request window, a 15-second timeout, four forecast
cycles, and a 2.0 buffer fraction. The adapters aggregate rain by sum and level
and flow by mean.

```sh
mgb-ops observation import-inventory --workspace /regional/workspace --input /regional/workspace/source/history_station_inventory.csv
```

Import the ANA station inventory before acquisition. Use `observation
export-inventory --workspace /regional/workspace --output inventory.csv` to
export the saved inventory for editing. Per-station variables are
stored in `operation.sqlite`. Acquire commands write to that database and retain
originals under `downloads/`. CLI JSON reports station or provider outcomes.
Standalone acquisition keeps successful sibling results on failure or cancellation.

## CLI download progress

For `observation acquire`, `forecast acquire`, and the download phase of
`operation run-all`, the CLI shows live progress on stdout in text mode and
stderr with `--json`. A terminal shows the active ANA station, request window,
row count, and a requesting or loading spinner. Redirected output and
`TERM=dumb` use one completion line per station. Forecast output lists each
attempted cycle and the provider's outcome. A terminal also shows the current
forecast lead. Completion lines report whether the station had data and whether
each forecast asset was registered or reused.

The forecast buffer expands each edge by its axis span times the configured
fraction. The expanded bounds must fit within geographic limits. Forecast
providers are unique and processed in configured order. ANA reads station
selection and enabled variables from the database.

## ANA observations

The regional inventory supplies `provider_code`, `station_code`,
`station_name`, `latitude`, `longitude`, and `observed_variables`. Names and
coordinates update in place for known stations. Station codes match after
trimming whitespace and leading zeros. Variables are `rain`, `level`, and
`flow`, or `none` for a station that receives no requests. The acquisition
workflow reads ANA rows from mixed-provider inventories and retains existing
station identities.

For each selected station, request inclusive calendar dates where an enabled
variable has no saved raw value that day, plus the reference date. Each completed
station retains `observed.csv` with columns `provider_code`, `station_code`,
`observed_at`, `variable_code`, and `value`. Source response XML uses
`YYYYMMDD__YYYYMMDD.xml` beside the CSV under
`downloads/ana/<attempt>/<station>/`. An unsuccessful station still reports
any XML already captured. [ANA ServiceANA operation](https://telemetriaws1.ana.gov.br/serviceana.asmx?op=DadosHidrometeorologicos)

Source timestamps are reduced to minutes and normalized to São Paulo local
time for CSV. Import converts to the database's declared timezone and uses
hour-ending buckets. Rain is in millimetres, level in centimetres, and flow in
cubic metres per second. Level averages are rounded to an integer with ties to
even. Empty source values yield no sample. Incoming values revise the station's
unambiguous raw series in one transaction. Higher-ranked series, manual
replacements, and already prepared run inputs retain their values. The regular
[canonical JSON intake](saved-data.md#canonical-data-and-review) still rejects
conflicting raw samples.

## Forecast acquisition

- ECMWF IFS Open Data 0.25° `fc` surface `tp` uses 00 and 12 UTC cycles, with
  0–144 h every three hours and 150–360 h every six hours. Acquire individual
  fields using the published index and ranges, then crop locally.
  [ECMWF product schedules](https://confluence.ecmwf.int/spaces/DAC/pages/272310539/ECMWF%2Bopen%2Bdata%2Breal-time%2Bforecasts%2Bfrom%2BIFS%2Band%2BAIFS),
  [ECMWF byte-range protocol](https://confluence.ecmwf.int/spaces/DAC/pages/738041083/ECMWF%2Bopen%2Bdata%2Bgetting%2Bthe%2Bdata)
- NOAA NOMADS GFS 0.25° surface APCP uses 00, 06, 12, and 18 UTC cycles, with
  3–240 h every three hours and 252–384 h every twelve hours. Request the
  buffered region with at least ten seconds between requests. A fresh cycle
  takes at least 15 minutes before transfer and processing time.
  [NOAA GRIB Filter guidance](https://nomads.ncep.noaa.gov/info.php?page=gribfilter)

Start with the latest scheduled cycle at or before the reference. An unavailable
cycle falls back within the configured lookback. Invalid GRIB2, conflicting
assets, and local persistence errors are reported for that provider. Reuse
verifies the retained files, complete native schedule, requested crop, and
timezone. Otherwise the completed source and normalized native grid register
under immutable IDs in the [forecast catalog](forecasts.md#native-values-and-persistence).
A failed provider leaves successfully registered sibling forecasts available.

## Run All

Run All resolves the reference once, then acquires and registers inputs before
[selecting scenarios](operations.md#scenarios) and freezing the run. Forecasts
that remain unavailable appear in the acquisition report. Automatic selection
uses the usable catalog cycles. Explicit selections require their chosen cycle
and coverage. A failed or cancelled run leaves the previous active generation
available.

## Standalone source acquisition

Use `acquisition` to acquire and normalize files for later ingestion. Select a
new output directory under an existing parent. These commands do not open a
workspace, register an asset, update SQLite, or change Run All.

```sh
mgb-ops acquisition bndmet --product rain --station A801 \
  --start 2026-11-21 --end 2026-11-22 --output /regional/workspace/downloads/inmet-rain
mgb-ops acquisition bndmet --product wind --station A801 \
  --start 2026-11-21 --end 2026-11-22 --output /regional/workspace/downloads/inmet-wind
mgb-ops acquisition ons --reservoir RIGAR --reservoir JIPREA \
  --start 2026-11-21 --end 2026-11-22 --output /regional/workspace/downloads/ons-flow
mgb-ops acquisition imerg --bbox=-59,-35,-48,-26 \
  --start 2026-11-22T00:00:00Z --end 2026-11-22T01:00:00Z \
  --output /regional/workspace/downloads/imerg
```

The output retains original responses under `raw/`. The JSON report lists
retained files and issues, and `acquisition.json` records the request, output
timezone, and paths relative to the output directory. The default output
timezone is `America/Sao_Paulo`. An acquisition with reported issues exits
unsuccessfully and can still contain normalized files from successful
requests. If interrupted or unable to finish, the attempt keeps original
files already written. Choose a new directory when retrying.

- **BNDMET** requires `INMET_API_KEY` in the environment. Dates are inclusive
  São Paulo calendar dates. Repeat `--station` to request multiple stations.
  Rain (`I175`) produces `rain-<station>.csv` with millimetres summed into
  hour-ending buckets. Identical readings at the same instant count once.
  Conflicting readings at that instant fail the request window. Wind
  (`I111` speed in m/s and `I113` direction in degrees) produces
  [canonical wind JSON](saved-data.md#forcing-state-and-calibration) named
  `wind-<station>.json`, with series ID `inmet:<station>`. Samples need two
  valid measurements at the same hourly instant to form a wind vector.
  [DECEA BNDMET API access](https://ajuda.decea.mil.br/base-de-conhecimento/como-solicitar-e-preencher-o-formulario-da-api-bndmet/)
- **ONS** downloads monthly public hourly CSVs for the inclusive São Paulo
  calendar dates requested. Repeat `--reservoir` with ONS `id_reservatorio`
  values. `flow-<reservoir>.csv` contains inflow and defluence in m³/s with
  separate station codes `<reservoir>:inflow` and `<reservoir>:outflow`.
  Source labels at 23:59 map to the following midnight. Other unexpected
  non-hourly labels and conflicting duplicate hours are reported and omitted.
  Invalid flow measurements are reported and recorded as missing. Inflow can
  be signed. ONS may revise previously published observations.
  [ONS hourly reservoir data](https://dados.ons.org.br/dataset/dados_hidrologicos_ho)
- **IMERG** uses an Earthdata Login user token in `EARTHDATA_TOKEN` with
  GES DISC access. It discovers half-hourly Early Run V07 granules through
  NASA CMR, retains the native HDF5 and crops them locally. The time window
  uses inclusive start and exclusive end UTC hour boundaries, up to 31 days.
  The output size is bounded by the selected area and duration. The bbox is
  west, south, east, north and includes native cells with positive overlap.
  `imerg.nc` uses the canonical [grid series](series.md), with hourly
  precipitation depths in mm, time bounds, and a validity mask. The depth is
  half the sum of the two rates in mm/h, and a missing cell in either half
  makes that cell missing for the hour. A missing granule blocks NetCDF output.
  [NASA IMERG documentation](https://gpm.nasa.gov/resources/documents/imerg-v07-technical-documentation),
  [Earthdata user tokens](https://urs.earthdata.nasa.gov/documentation/for_users/user_token)

Rain and flow CSV files use `provider_code,station_code,observed_at,variable_code,value`.
Local hour labels are `YYYY-MM-DDTHH:00:00`. A blank value is missing and a
numeric zero is valid. The provider codes are `inmet` and `ons`. Provider
credentials are read from the environment and omitted from retained files.
