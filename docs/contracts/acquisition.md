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
**New run → Run All** executes an acquisition-enabled recipe. Jobs shows provider
progress, committed results, and failures. Leave the reference blank for the
current hour in the acquisition timezone, or supply a local, hour-aligned
`YYYY-MM-DDTHH:00:00` value with `--reference-time`. Cancellation stops after
the current bounded request or write and preserves committed data. NOAA spacing
is interruptible.

Workspace commands read `config.toml`. Set the enabled providers under
`[acquisition]` and the forecast bounds under `[spatial]`. Defaults are 90
observation days, 30 days per request window, a 15-second timeout, four forecast
cycles, and a 2.0 buffer fraction. The adapters aggregate rain by sum and level
and flow by mean. Use `--reference-time` to override `run.reference_time`.

```sh
mgb-ops observation import-inventory --workspace /regional/workspace --input /regional/workspace/source/history_station_inventory.csv
```

Import the ANA station inventory before acquisition. Use `observation
export-inventory --workspace /regional/workspace --output inventory.csv` to
export the saved inventory for editing. Per-station variables are
stored in `operation.sqlite`. Acquire commands write to that database and retain
originals under `downloads/`. CLI JSON reports station or provider outcomes.
Standalone acquisition keeps successful sibling results on failure or cancellation.


## Explicit acquisition requests

Each optional provider section enables its corresponding update. The example
shows the defaults plus a required model bounding box for forecasts.

```toml
schema_version = 1
time_zone = "America/Sao_Paulo"

[ana]
inventory = "data/source/history_station_inventory.csv"
database = "data/observations.sqlite"
request_days = 90
fetch_window_days = 30
timeout_seconds = 15
station_codes = []

[ana.aggregation]
rain = "sum"
level = "mean"
flow = "mean"

[forecasts]
providers = ["ecmwf", "noaa"]
model_bbox = [-58.95016, -34.40217, -48.85759, -26.29721]
catalog = "data/forecasts.sqlite"
artifact_root = "data/forecasts"
lookback_cycles = 12
buffer_fraction = 2.0
```

Choose one or both sections. Forecast bounds use west, south, east, north in
degrees and must match the operation's working bounds when used by Run All.
The forecast buffer expands each edge by its axis span times the fraction.
Buffered bounds must fit within the geographic limits. Forecast providers are
unique and processed in the specified order. ANA uses a maximum request window
of 30 days. An empty station list selects the entire ANA inventory. The listed
aggregation methods may be set to `sum`, `mean`, or `last`.

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

Run All resolves the selected reference once, acquires the enabled inputs, then
freezes the input selection and executes. Default scenarios use the latest
covering local forecast for each enabled forecast provider. Missing coverage is
reported before execution. Use the [operation selection options](operations.md)
to select a zero or pinned forecast scenario. A failed or cancelled run leaves
the previous active generation available.
