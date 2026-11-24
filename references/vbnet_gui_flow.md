# VB.NET operator workflow

This inventory follows the retained Windows Forms entry point, its visible
menus, the embedded Quick Start guide, and the tools the menus launch. The
[source map](vbnet_map.md) indexes the processing code behind those screens.
The inventory describes the September 2026 source snapshot. Screen behavior
in the deployed Windows build still needs a live walkthrough.

## What the operator sees first

The main window is a map with **Camadas** on the left, map and result shortcuts
along the top, and cursor coordinates in the status bar. The menu order in the
source is **Início → SIG → Plugins → Previsão → Automoção**. `SIG` is inserted
at startup, which explains its absence from the Windows Forms designer menu.
`Automoção` is the spelling in the source. Map navigation starts in Pan.

| Entry | What it does |
| --- | --- |
| **Início → Início Rápido** | Opens an illustrated guide. It tells operators to go straight to **Previsão** and work down the menu. |
| **Início → Preferências** | Opens **Configuração** (`Form2`). Update the telemetry station list. Set telemetry, reservoir, meteorological download, automatic forecast, temporary working and results export directories. The **Database** tab exports, replaces, merges or compacts station history. |
| **Início → Sobre** | Shows the regional project, institutions and version information. |
| **SIG → Arquivo** | Open a local shapefile, clear loaded layers, or exit. |
| **SIG → Navegação** | Pan, wheel zoom, zoom in/out, drag a zoom rectangle, or zoom to full extent. The map toolbar also exposes Pan, box zoom, and full extent. |
| **SIG → Consulta → Identificar** | Click a feature to display its attributes. This is separate from the result-picking mode. |
| **SIG → Camadas** and the **Camadas** panel | Toggle visibility, reorder, zoom to the selected layer, or remove it. Double-clicking a layer zooms to it. The layer menu also exposes geometry-specific styles for polygon fill/outline, line color/width and point color/size/outline. |

When present, the app loads local layers for urban areas, the RS boundary,
basin outlines, rivers, reservoirs, lakes and lagoons, rainfall stations,
conventional and telemetric SGB stations, hydropower plants, and water-level
stations. It fits their extent on startup. The result tools pick a `Mini` from
the **Rios** layer. A generic Identify click queries vector attributes instead.

[Main window handlers](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form1.vb) · [Menu and toolbar designer](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form1.Designer.vb) · [Quick Start image resource](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form24.resx#L121)

## The seven forecast steps

The embedded Quick Start guide explicitly lists this order. It says the forms
set dates automatically, operators should update the data and assess its quality
in the input screens, and raw flow and level should be reviewed before
post-processing. The forms expose separate actions within each step.

| Step in **Previsão** | Operator actions and handoff |
| --- | --- |
| **1. Chuva Observada** (`Form7`) | Review the station table. Update recent rainfall online, from offline hourly text, or from SOMAR files. View a selected station's pluviogram, export station locations as a shapefile, inspect monthly and last-72-hour availability, run **Verificação** for spatial consistency, or open the hourly editor. Choose the observation interval, mini-basin geometry and IDW options, then interpolate observed rain for MGB. Options include a missing-period notice, text export and updating the internal store with interpolated data. |
| **2. Chuva Prevista** (`Form21`) | The visible submenu entry is **Dados SINTEGRE ONS (txt)**. The screen also exposes zero rain, already-interpolated text, binary and forecast-grid file paths, plus old/new ECMWF, GEFS and ETA40 ONS choices. Select a forecast source, geometry and interval, adjust daily-to-hourly disaggregation where applicable, and produce model-ready forecast rainfall with optional text output. The forecast-rainfall editor is a separate plugin entry. The existing Rust migration excludes ONS meteorological forecasts. The visible step still has equivalents for supported forecast assets and zero rain. |
| **3. Vento** (`FormWind`) | Open the INMET wind downloader, import speed and direction into the Guaíba and Lagoa dos Patos wind table, review the data, select the start and end hours, and export wind inputs for the model. |
| **4. Vazão Observada** (`Form3`) | Update recent flow and water levels from online or offline sources, or load both from a directory. Select a station to view a hydrograph or level chart, edit hourly values, inspect monthly and 72-hour availability, or export station locations. Choose an export interval and produce `QOBS.MGB`; individual station text exports are optional. |
| **5. Operação de Reservatórios** (`Form5`) | Download observed plant flows or update them from local files, a configured directory, or ONS CSV. Inspect a plant hydrograph and monthly availability, edit a point or station series, or export station locations. For the forecast horizon, choose observed-only operation, load a standard schedule or single-column hourly schedule, or use the available linear, logarithmic or exponential extrapolation controls. Set the interval and export reservoir flow substitutions. **Rodar modelo sem UHEs** exports inputs with no reservoir substitutions and closes the form. The Quick Start guide calls out the choice between estimated future plant flows and letting the model compute future flows from observed-only inputs. |
| **6. Rodar Previsão** (`Form8`) | Review or browse the observed-flow, reservoir-flow, observed-rain and forecast-rain inputs and their coverage. Set run dates/hours, hydrological forecast horizon, basin and applicable plant choices. Choose the inertial forecast or Muskingum–Cunge with assimilation. Hot-start options let the operator use or save initial conditions and set their times. The form shows processing status and has options to retain observed-flow and calculated results for viewing. |
| **7. Resultados** (`Form1`, `Form4`, `Form12`, `Form23`) | Choose what to inspect or export, then click a river reach for the reach-specific actions. Standard basin panels and the forecast-horizon rainfall summary open directly. The detailed reach chart and export choices are listed below. |

The rainfall, flow and reservoir screens share two review patterns. A station
and date range can open **Edição Manual** (`Form9`), an hourly grid whose Save
action updates station history. **Disponibilidade Mensal** (`Form6`) and the
last-72-hours view (`Form13`) show coverage matrices with legends and image
exports. The 72-hour checks are exposed for rainfall, flow and water level.
The **Verificação** button in observed rainfall opens the bundled spatial
consistency tool.

[Form responsibilities and source paths](vbnet_map.md#how-to-use-this-map) · [Quick Start image resource](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form24.resx#L121)

## Results and handoff

| Entry | What it does |
| --- | --- |
| **Hidrograma do trecho** | Choose raw calculated flow or post-processed flow, then click a river. |
| **Cotagrama do trecho** | Choose raw calculated level, referenced level, or post-processed level, then click a river. The result toolbar offers shortcuts for raw and post-processed flow and for referenced and post-processed level. |
| **Pluviograma do trecho** | Click a river reach to inspect its rainfall series. |
| **Hidrogramas padronizados** | Open **Geral** or a board for Baixo Uruguai, Médio Uruguai, Alto Uruguai, Ijuí–Ibicuí–Quaraí, Jacuí–Taquari, Sinos–Caí–Gravataí, or Negro–Costa Doce. Regional boards have a Save action for the standard figures. |
| **Chuva no horizonte de previsão** | Display the average rainfall over the forecast horizon. |
| **Gravar hidrograma/cotagrama/pluviograma do trecho** | Choose the export type, click a reach, and save that series to a text file. |
| **Exportar rodada manual** | Copy the model input and output files to a dated folder under the configured export location. |
| **Desativar visualização de resultados** | Leave reach-picking mode and return the map tool to Pan. |

The current reach result form (`Form23`) combines a flow or level chart with an
upstream rainfall chart and controls for past and future accumulation windows.
It can center the chart around the forecast, reset the zoom extent, update
post-processing parameters and bias, show percentage uncertainty bands,
copy forecast values or timestamp/value pairs, save or copy a figure, and
export the result or rainfall series. `Form11` is an older chart form with
similar controls. The flow-with-threshold command has a handler, but its
standalone menu entry is explicitly hidden in the designer.

[Result selection and export handlers](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form1.vb#L900) · [Reach result form](../../legacy_vbnet/Sistema_de_Previsao_RSH_2025_versao0.99/FONTE_SISTEMA_PREV_RSH_2026_09_02/Telemetria/Form23.Designer.vb)

## Plugins and automation

| Entry | Operator workflow and source evidence |
| --- | --- |
| **Plugins → Calibração MGB → Geometrias** | Open `MINI.gtp` in a separate geometry editor, edit parameters in a grid, or use shortcuts for Manning roughness globally or by sub-basin and for channel widths and depths from equations or a CSV table. Save an edited geometry file. |
| **Plugins → Calibração MGB → Parâmetros do Solo** | Open or create a soil-parameter file, navigate basins and parameter tables, copy parameters between basins or rows, and save. |
| **Plugins → Download de dados → Download Hidrotelemetria ANA / Download Plu/Flu API ANA** | Launch two ANA acquisition tools. The first prompts for a start and end hour in a console, downloads station data and reports completion. The second entry launches an executable whose screen and exact controls are unavailable in this snapshot. |
| **Plugins → Download de dados → Download Plu INMET** | Pick a station-code text file, destination and start/end dates, then run the downloader. |
| **Plugins → Download de dados → Download Plu IMERG e GSMAP** | Load the virtual stations, inspect their database coverage, select a period, download satellite data, extract station values, then update the database. The tool also has temporary-file cleanup. |
| **Plugins → Download de dados → Download Vento INMET** | Open the wind downloader, which also has an entry from **Vento**. The separate tool downloads and converts wind data and has its own plot view. |
| **Plugins → Processamento de dados → Dados hidráulicos históricos ONS (.csv)** | Launch the historical hydraulic CSV processor. The launch is confirmed. The executable's internal interface is unavailable in this snapshot. This is distinct from the ONS rainfall forecast import. |
| **Plugins → Processamento de dados → Editor de chuva prevista** | Launch `EditorPrevChuva.exe`. The launch is confirmed, while the editor's controls and save behavior need a live walkthrough. |
| **Plugins → Visualização de resultados → Hidrogramas de Ensemble** | Open the bundled Hydrograph Viewer. Select a station and retained forecasts, compare archived runs with observations and optional simulated series, adjust individual/global bias and chart presentation, and annotate the view. Export or copy a station chart, or export CSV/PNG in batches for a basin or station list. |
| **Automoção → Centro de Comando** (`Form20`) | Use **Liga/Desliga** and scheduled times for input stages, the two routing methods, saving results and email. The timer opens the corresponding workflows. **Emitir** opens the bulletin figure workflow. The attendant email screen (`Form16`) shows sender/destination, subject and message with a send action. |
| **Automoção → Rodar em Batelada** (`Form14`) | Select the observation, reservoir and run stages, configure a zero-rain or additional rainfall cases and source folders, choose `.dat` or `.zip` forecast input and the results destination, then start the batch. The checked cases run and export under separate forecast labels. |

The retained MapWindow plugin (`Class1`) exposes additional historical entry
points, including an alternative forecast import (`Form10`), threshold
hydrographs, direct email and a return-period exceedance map (`Form18`). The
map tool takes a base shapefile, model input and forecast output, then writes a
shapefile. The earlier `Form15` scheduler, `Form17` email form and `Form19`
forecast-rain form remain as source references. Their presence does not show
that they appear in the standalone app. The retained inventory also lists
executable-only rainfall bulletin tools.

[Form and plugin source index](vbnet_map.md) · [Legacy source gaps](../../legacy_vbnet/references/source_gaps.md)

## What this suggests for the tabbed desktop

Use the Portuguese **Previsão** names and order as the starting point for the
ribbon actions. Open each task in the existing tabbed workspace, with the map
available beside it. Put observation review, editing and availability under
their related input step, where the old app exposes them. Use an active results
action followed by a reach click as a familiar path into Charts, while keeping
the existing linked map, chart and data selection. Make the on-screen reach
results, availability plots, comparison views and bulletin previews interactive.
Their exported images and reports remain separately selectable outputs.

**Início** maps to project opening, preferences and Quick Start. **SIG** maps
to the map tools and the left layer panel. **Plugins** is where operators can
find acquisition, calibration, forecast-rain editing and archive comparison.
**Automação** is where they can find scheduling, batch runs and distribution.
That gives the desktop familiar places to find occasional tools without adding
extra steps to the forecast sequence.

The current Rust desktop provides Map, Charts and Data, a shared operation
draft, Prepare, Execute, Run All, Jobs and selected-series exports. It can
select existing observation replacements and forecast corrections in the draft.
The underlying source-acceptance, validation, preparation and execution rules
still belong to the shared library. Live acquisition, wind and reservoir
forcing, new hot starts, editors, cross-run comparisons, bulletins and
automation each have separate backend work or product decisions in the
migration plan. The legacy screens identify where their supported actions
belong in the desktop workflow.

[Current desktop contract](../docs/contracts/desktop.md) · [Migration status](../docs/migration/summary.md)
