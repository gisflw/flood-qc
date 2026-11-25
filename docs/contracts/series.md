# Rainfall summaries, comparisons and CSV

Shared headless APIs supply the values used by charts and selected-series exports.
They preserve actual local timestamps, masks, valid zero and signed levels.
They do not interpolate, resample or fill gaps with measurements.

## Time windows and accumulation

`resolve_hourly_window(time_zone, window)` resolves `HourlyWindow::Explicit`
start/end bounds or `Relative { end, hours }` lookback bounds. Bounds must be
ordered whole hours in a valid operation timezone. Ambiguous/nonexistent labels,
timezone transitions and overflowing arithmetic fail. The returned interval is
the actual selection, including for relative shortcuts.

`signed_rainfall_window(time_zone, reference_time, hours)` selects observations
from reference time plus negative hours to reference time, or forecasts from
reference time to reference time plus positive hours. It returns the source kind
and the same validated interval as an equivalent explicit window. Zero hours fail.

`accumulated_rain(results, topology, outlet, upstream, window)` uses canonical
`ResultSeries` precipitation and validated local mini areas. It includes the
selected mini, and optionally all its upstream minis. The result reports total
and fully covered area in km², covered and expected area-hours, and `mean_mm`.
That mean is present only when every selected mini/hour is covered. It is the
area-weighted mean of mini accumulations, not a sum of depths. Missing minis,
missing hours and masked cells cannot become zero or a complete mean. Aggregate
overflow fails. Topology and map identities follow the [spatial contract](spatial.md).

`accumulated_grid_rain(grid, window)` and `accumulated_grid_frame(frame, window)`
return per-cell accumulated values and covered/expected hours. Native multi-hour
intervals remain intact. An interval cut by the selection is not prorated or
counted as fully covered; incomplete cells have absent totals. Missing cells
remain distinct from supplied zero. These APIs perform no new interpolation.

## Series queries

`query_series(&SeriesQuery)` takes one `Location::Station(id)` or `Mini(id)`, an
explicit timezone/window, and ordered `SeriesSelection` values. Model selections
identify a trial directory, scenario and rain/level/flow variable. Observation
selections identify a canonical observation asset directory, explicit series and variable, whether
to use effective values, and ordered replacement IDs. Optional supplied
`replacements` replace those IDs with explicit replacement values without catalog writes.
Rain and flow support effective values. Level uses recorded observations.

Rain selects complete canonical intervals within the window, excluding its start
label and including its end. Instantaneous level and flow include both visible
endpoints. Effective flow applies ordered replacements at exact hourly labels
and includes recorded off-hour samples. See the [observation contract](observations.md#selection-reviews-and-preparation)
for replacement semantics. Coordinates are needed for interpolation, not
station-identity selection or a data-only chart.

Model mini levels are explicitly raw; station levels use retained `StationResults`
centimetres and supplied mini/offset metadata. Unknown offsets do not become zero.
Rain uses mm, level cm and flow m³/s (`m3/s` in CSV/API). Result points retain
history/forecast segmentation. Model series retain run/scenario and forecast
reference time. Different valid reference times remain comparable at actual
local timestamps. Incompatible timezone, location, coordinates or station
reference metadata produce findings instead of misleading overlays.

A report contains the resolved window, valid series and per-selection findings.
A missing/corrupt selected scenario does not hide valid unrelated comparisons.
Queries read selected canonical outputs without requiring deleted trial working
inputs. They do not rewrite caches or register a run.

## CLI and CSV

```sh
mgb-ops series query --request series.json --json
mgb-ops series export --request series.json --output selected.csv --json
```

A model rainfall request has this shape:

```json
{
  "location": {"mini": 1},
  "time_zone": "America/Sao_Paulo",
  "window": {
    "kind": "explicit",
    "start": "2026-07-20T00:00:00",
    "end": "2026-07-21T00:00:00"
  },
  "selections": [{
    "kind": "model",
    "root": "/regional/workspace/cache/run/<generation>",
    "scenario_id": "zero",
    "variable": "rain"
  }]
}
```

Use the execution report's `run_root` or an explicitly saved bundle root for
model selections. Paths resolve from the invocation directory, including paths inside this query
request. The request file's location does not select a workspace. JSON mode emits
`{ok:true,result:...}` or `{ok:false,error:...}` on stdout. A successful query can
contain per-selection findings; export rejects findings rather than silently
omitting requested series. Operational failure exits 1, argument failure 2 and
success 0. Progress does not mix with JSON stdout.

`export_series_csv(&query, destination)` exports precisely the selected series
and resolved interval, ordered by timestamp then requested series order. UTF-8
CSV uses comma delimiters, proper quoting and decimal points. Column order is:

```text
timestamp,time_zone,location_kind,location_id,series_kind,run_id,scenario_id,forecast_reference_time,variable,unit,value
```

Observation rows leave inapplicable run/scenario/reference fields empty. Missing
values are empty; zero and signed values are numeric. Formatting preserves source
precision and is independent of interface language. A successful export atomically
replaces its destination; failure preserves existing bytes.
