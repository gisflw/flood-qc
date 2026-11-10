# Analysis, post-processing, and reports

## Owns

- Basin and station analysis, rainfall accumulations, peaks, thresholds, and maps.
- Rating-curve conversion, flow/level error correction, manual bias, and
  percentage bands supported by the reference system.
- Scenario comparison and comparison with archived forecasts.
- Complementary content from both existing bulletin families.
- PDF, Markdown, HTML, and relevant CSV data exports.

## Starting references

Python: `src/mgb_ops/analysis/`, `src/mgb_ops/workflows/reporting.py`,
`src/mgb_ops/assets/report.py`, dashboard charts/maps, and the Taquari report.

Runtime: `rs_hydro_forecast/generate_taquari_report.py`; port required behavior
into Rust and keep regional selections/configuration in the workspace.

VB.NET: `Form3`, `Form11`, `Form12`, `Form12b`, `Form23`, rainfall-bulletin tools,
and `Plugins/HydrographViewer/`. The shapefile/exceedance tool is mapped in
`Form18`/`Class1`; confirm its active integration when implementing GIS exports.

## Implementation direction

Use shared numerical results and report content across interfaces and formats.
Analysis owns QC, corrections, forcing calculations, diagnostics, comparisons,
report construction, and reusable document/chart rendering through private
submodules. It depends on store's canonical contracts, never adapters or model
internals. Calculations accept supplied data without requiring database or
filesystem access. App obtains selected data, calls analysis, passes prepared
forcing to model, and coordinates durable saving/export/publication through store.
Interactive widgets and GUI state stay in the interface; PDF/HTML/Markdown/chart
rendering must also work without the GUI.

Study both bulletin examples before deciding their combined content and layout.
Retain the useful manual chart adjustments and data exports from the references.
Treat percentage bands according to their implemented definition; do not imply
they are statistical confidence intervals without evidence.

Decide conflicting post-processing behavior during development with a concrete
example. Keep computation separate from chart rendering and document units and
time conventions at their boundaries.

## Evidence of completion

One completed operation produces the required bulletin content in all three
document formats and selected CSV datasets. Check key values against reference
calculations and inspect rendered reports. Archived forecasts can be compared.
