# VB.NET and bundled Python reference map

Current source root: `Sistema_de_Previsao_RSH_2025_versao0.99/`.

Operational datasets, manuals, and standalone visual assets remain local at their original paths; see [local-only data and asset paths](local_data.md).
Frozen reference: `legacy/vbnet` once prepared.

The main VB.NET project is under
`FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/`; the form paths below are relative
to that directory. Plugin paths are relative to the source root.

| Responsibility | Reference |
| --- | --- |
| Application entry points and menu wiring | `Form1.vb`, `Form1.Designer.vb`, `Sistema MGB-IPH_RSH.vbproj` |
| Station configuration and history export/merge/compaction | `Form2.vb` |
| Observed levels/flows, rating curves, and QOBS preparation | `Form3.vb` |
| Reservoir observations, schedules, extrapolation, and QSUBST preparation | `Form5.vb` |
| Availability matrices and recent-data checks | `Form6.vb`, `Form13.vb` |
| Observed rainfall, interpolation, and SOMAR import | `Form7.vb` |
| Model preparation, routing selection, hot starts, and initial-state persistence | `Form8.vb` |
| Manual station-series edits | `Form9.vb` |
| Additional rainfall-file import behavior | `Form10.vb`, non-ONS paths in `Form21.vb` |
| Flow/level post-processing, bias, percentage bands, and series export | `Form11.vb`, `Form23.vb` |
| Standard bulletin figures | `Form12.vb`, `Form12b.vb` |
| Batch operations and archived results | `Form14.vb`, `Form20.vb` |
| Scheduling and email | `Form20.vb`, `Form16.vb`, `Form17.vb`; `Form15.vb` is additional reference |
| Wind import, editing, and model export | `FormWind.vb`, `Plugins/Vento/plugin/` |
| Geometry/Manning/channel editing | `Plugins/Form_Editor/Form_Editor/` |
| Soil-parameter editing | `Plugins/Form_parcal/Form_parcal/` |
| ANA acquisition | `Plugins/DownloadTelemetriaANA/`, `Plugins/Download_API_ANA/` |
| INMET rainfall acquisition | `Plugins/Executavel_baixadordadosINMET/main_v3.py` |
| IMERG/GSMAP and virtual stations | `Plugins/Executavel_baixador_IMERG_GSMAP/main_baixador_virtuais/` |
| Spatial consistency checks | `Plugins/Executavel_consistencia_espacial/Consistencia_espacial_v1/` |
| Forecast rainfall editor | `Plugins/EditorPrevChuva/`; the inspected entry point is executable-only |
| Archived forecast overlays, individual biases, annotations, and batch exports | `Plugins/HydrographViewer/hydrograph_engine.py`, `hydrograph_widget.py`, `csv_exporter.py` |
| ONS hydraulic observations and reservoir data | `Plugins/ONS/`, related import paths in `Form5.vb` |
| Rainfall bulletin products | `Plugins/Chuva_boletim4/`, `Plugins/Chuva_boletim6/`, `Plugins/Figuras_Boletim_Chuva_Acum/` |
| Return-period exceedance shapefile tool | `Form18.vb`, wired through `Class1.vb`'s MapWindow plugin path |
| Baseline calibration, geometry, station definitions, and manuals | `MGB/Input/`, root station/configuration files, `manual/` |

## How to use this map

For each migrated feature, follow the UI handler into its calculation and file
writes, then record only the inputs, behavior, outputs, and a representative
example needed for implementation. Inspect bundled Python plugins as part of
the same system. Find source for executable-only tools where possible; otherwise
use manuals and reproducible input/output examples.

ONS meteorological forecasts are excluded from the migration. ONS hydraulic
observations and reservoir schedules remain in scope. Do not treat commented
code, dormant menu items, or version notes as verified behavior. The available
source snapshot and deployed binaries may differ.

Snapshot path: `Sistema_de_Previsao_RSH_2025_versao0.99/`. Retained-file hashes: `references/inventory.csv`.
