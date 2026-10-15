# Acquisition and normalization

## Owns

All implemented data acquisition and useful import paths from both references,
except ONS meteorological forecast products:

- ANA observations, including the acquisition paths represented in both systems.
- INMET rainfall, ECMWF IFS forecasts, and NOAA GFS forecasts.
- IMERG/GSMAP rainfall and virtual stations.
- Wind observations, hydraulic/reservoir observations, and reservoir schedules.
- SOMAR and other supported non-ONS rainfall/file imports.

ONS hydraulic observations and reservoir schedules remain included.

## Starting references

Python: `src/mgb_ops/adapters/`, `src/mgb_ops/workflows/observed.py`, and
`src/mgb_ops/workflows/forecast.py`.

VB.NET/plugin acquisition locations are listed in the
[source map](../references/vbnet_map.md), including `Form3`, `Form5`, `Form7`,
`FormWind`, and the ANA, INMET, IMERG/GSMAP, and wind plugins.

## Implementation direction

Retain the Python flow: acquire source artifacts, normalize them, then register
or load them into the history/assets layer. Add sources to that flow rather than
copying the form-driven orchestration. Preserve units, timestamps, aggregation,
station identity, forecast intervals, and missing-data semantics explicitly.

Inventory exact products and supported file variants while implementing each
source. Keep credentials local and outside completed-run exports.

## Evidence of completion

Representative saved responses/files verify normalization without network access.
Live checks verify each integrated provider when credentials and service access
are available. Failures preserve successfully acquired data and report gaps.
