# Desktop interface

Build/run the optional native interface:

```sh
cargo run --locked --features desktop --bin mgb-ops-desktop -- /region/config/workspace.toml
```

With no argument, reopen the most recent project or use Open TOML. Standard
`config/workspace.toml` locations infer their workspace; other locations ask for
a folder and remember the association. Relative resource paths still resolve
against that workspace under the [workspace contract](workspace.md).

Project opening and monitoring Refresh/F5 are read-only. Missing engine prerequisites, caches,
geometry or observations report findings without preventing unrelated available
data from loading. Work runs in a background worker. Newer requests supersede
older replies; replacement failures retain existing display data with feedback.
Geometry snapshots are versioned by their configured files. Unchanged sources
reuse prepared geometry; a failed replacement retains the previous matching
topology/geometry together while unrelated sources update. Retained sources are
identified in the controls. The map camera and valid selection survive refresh.
Closing a project releases
its displayed data and stops its loader after submitted reads and exports finish;
waiting remains asynchronous and keeps the current project available. A committed
project switch blocks operational edits and saves until its loader succeeds or
fails. Failure retains the old project and edits; ordinary F5 monitoring refreshes
remain independent of runner work.

## Common tasks

| Task | Control |
| --- | --- |
| Open a workspace or import saved data | **Project → Open TOML** or **Import data** |
| Compare map rainfall | **Map → Rainfall** and **Map view** above the map |
| Step through modeled inundation | Use **Map → Inundation** |
| Inspect location data or a run | **View → Charts**, **Data**, or **Files** |
| Review observation availability and QC | **View → Observation review** |
| Compare runs, prepare a bulletin, or share a run | **View → Run comparison**, **Bulletins**, or **Share runs** |
| Edit staged observation replacements and forecast corrections | **Draft → Review instructions** |
| Run an editable draft | **Draft → Run experiment** |
| Run a saved operation TOML at a selected reference | **Processing → New run** |
| Set forcing and state options | **Processing → Forcing and state** |
| Start scheduling or monitor and cancel a job | **Processing → Automation** or **Jobs** |
| Edit workspace settings | **Settings → Workspace Settings** |
| Edit geometry or soil | **Processing → Calibration** |

See [saved-data workflows](saved-data.md) for request formats and outputs and
[controlled jobs](#controlled-jobs) for execution and cancellation.

## Data and selection

The interface reads configured regional mini segments/catchments and MINI.gtp,
`data/history.sqlite`, `data/forecasts.sqlite`, the [active rainfall
cache](operations.md#active-run-and-rainfall-cache), and the workspace's active run.
Charts, Files and Map open the active generation automatically. While viewing
the active run, a newly published generation updates all three views and clears
the previous scenario, inundation frame, and tiles. Files shows run status and diagnostics, including successful
scenarios from partially failed attempts. Missing or corrupt scenario outputs
produce findings and available siblings remain usable.

Use **Files → Open comparison run** to open a retained run or exported bundle.
The Files selector keeps explicitly opened runs available until the workspace
closes. Choose **Active run** to return to the managed generation.
**Share runs → Import bundle** copies a bundle to a new folder at the selected
location outside managed storage. Import leaves the active run and current view
as they were.

| Interaction | Effect |
| --- | --- |
| Left-click a station or MINI | Highlight it on the map and show its ID and available metadata in the map pane. The active tab and chart comparison stay as they were. |
| Search by ID or station name in **Data** | Select the matching location, including stations without coordinates. A positive numeric ID can select a MINI without local topology. The active tab and chart comparison stay as they were. |
| Right-click a feature and choose **View time series** | Open Charts and fill the feature ID and its unique linked counterpart. The counterpart field is cleared if there is no unique link. |
| Choose **Chart selected feature** in an empty Charts pane | Plot the currently selected feature using the same ID rules as **View time series**. |
| Hover over the map | See the feature label and accumulated rainfall for the cell under the pointer in the bottom stripe. Double and Swipe identify the hovered side and scenario. |
| Enter or choose **Apply** after editing chart IDs | Submit an explicit MINI and station pairing. |

Visible station targets take precedence over rivers and catchments. River hit
tolerance is wider than the stroke, and catchment containment excludes holes.
Station colors distinguish explicit model links from unknown associations. The
**Data** tab also offers feature actions for the selected location through
keyboard navigation.
Model series use the selected MINI and
observations use the selected station. The level chart uses retained station
levels when the run records the matching MINI and offset. It labels raw MINI
levels and uses a separate scale when both datums have data. Station thresholds
use the station scale. An available side can be viewed independently.

Choose a scenario and period in **Rainfall** on the **Map** tab. Reference, timezone and available
coverage come from the active cache. Negative signed hours end at the reference
and use observations. Positive hours start at the reference and use the selected
forecast. Explicit start/end bounds can span both. Cells with incomplete
coverage display as NA. Scenario changes apply immediately to the current period.
Use **Apply** to commit a period edit. A successful **Processing → New run → Run All** supplies
the new active cache and refreshes the open view. Use Refresh/F5 to load a generation published by a separate process while
the desktop is idle.

Enable **Inundation** under **Map → Layers** to see the displayed run's scenario
mask and controls. Choose the scenario and step through model hours with the slider or arrow keys while it has
focus. The control starts at the first forecast hour and displays the model's
local time. Double and Swipe use the same inundation frame on both sides. Use
Refresh/F5 after generating a mask for an open run. Refresh resets the scenario
and inundation frame. See
[inundation processing](inundation.md) for the command and regional inputs.

Choose model scenario overlays in Charts. Chart timestamps use
the run or project observation timezone. Without a run, the initial chart window
uses the current project-local hour and the preceding week. Chart loading runs independently of map and layer reads. Charts
reports loading, data availability, and errors for the requested pair. If a
replacement fails, a retained comparison keeps its original displayed IDs and
exports.
Rainfall inspection returns the accumulated original numeric cell, including NA
and supplied zero. Display opacity does not alter accumulation or recreate its
texture. The map's **Upstream basin rainfall over period** shows the area-weighted
mean accumulation in mm, coverage, and period for each displayed side. The
rainfall total and selected basin area include the outlet catchment and all its
upstream catchments. The totals use frozen MINI topology and local areas.
Project topology supplies the area when a frozen summary is unavailable. Load
upstream basin uses the clicked MINI or the station's linked MINI when
topology is available. Layers names the loaded basin by its outlet MINI and
allows it to be hidden or cleared independently of later selections. Replacing
a basin clears the previous one as loading starts. Project changes release it.

## Display and exports

The map includes attributed raster basemaps, river segments, catchments, stations,
an upstream basin outline and accumulated rainfall. The map shows the rainfall scale when a
loaded overlay is visible. The Map tab shows a linear blue bar from 1 mm to the configured maximum, which defaults to 100 mm. Colors at the
endpoints also cover values outside that range. An NA swatch marks incomplete
coverage. Use the checkbox in a layer row to toggle visibility. Expand the row
with **+** for order and opacity controls. Expanded local vector rows also offer
color and removal controls. Local vector layers affect only the display.
Polygon parts retain their holes. At regional zoom, river paths are simplified
with at most half a display pixel of error. Close zoom and river hit testing use
the original paths. Rainfall uses projected latitude strips to keep
pixel edges aligned in Web Mercator, rather than stretching four image corners.
Labels appear only at suitable zoom/spacing and do not intercept selection.

The ribbon, left column and right pane use English or Brazilian Portuguese. Automatic interface preferences remember recent workspace associations,
camera, layers, local files, selection, run/scenarios and time controls without a
new project-file schema. Source errors identify the retained view separately.

Rainfall, flow and level panels share time navigation and cursor position. The
interactive charts scroll vertically on short windows and open with padding
around the Y ranges. Bars
use actual hourly interval edges. History and forecast are visually distinct;
masks break curves, and zero and signed values remain valid. Level thresholds
and forecast reference markers are shown. The observed-series control and scenario
checkboxes control chart sources. Plots, legends, and cursor readouts share the
same source colors, which stay stable when toggling scenarios within a run.
Hover over a plot to update the readings below the charts. They are grouped by
variable and remain visible after the pointer leaves the plot.
Charts exports the displayed snapshot. CSV keeps model and observation location
IDs on each row using the [shared series export](series.md) format. Complete PNG
export identifies the pair and combines all three panels,
localized titles, local-time axes, units, reference/threshold annotations and
legends at 1×, 2× or 3×, replacing its destination atomically.

## Layout and settings

The ribbon has Project, Draft, View, Processing and Settings tabs with labeled
groups of actions. The map stays visible in the center with linked basemap
attribution in the bottom stripe when tiles are enabled. NASA Blue Marble provides a satellite basemap at regional scale.
**Map view** offers **Single**, **Double**, and **Swipe**. Double synchronizes the
center and zoom of two side-by-side maps. Swipe compares the overlays at the same
scale and location using a draggable divider. Focus the divider and use the arrow
keys to move it. Both comparison modes show left and right scenario and period
controls in Rainfall. The views share the basemap, layers, and selected feature.
The one-line bottom stripe shows map hover coordinates and rainfall, along with the displayed run's reference time and timezone,
observed and forecast horizons, and available scenarios. The summary comes from
the loaded run and its frozen recipe. Day horizons appear only when the saved
hourly boundaries convert losslessly. Available scenarios have successful,
readable basin results. Unavailable fields in a loaded run show a dash.

The resizable right pane defaults to Map. Its permanent Map tab contains Layers
and Rainfall. Charts, Data, and Files are also available. Data
offers location search, feature actions and the displayed series table. Files
contains the run selector, provenance, assets and findings. Use the
ribbon to open New run, experiment forms and Jobs in the same pane. Closing a form
tab preserves its inputs. Jobs opens from the View ribbon.

The **Settings** ribbon contains the interface and workspace controls. English or
Brazilian Portuguese and Light or Dark apply immediately and are remembered
app-wide. Light is the default. Preferences remember each project's right pane, map view, divider position,
and applied rainfall settings. Projects saved before map comparisons open in Single
with their previous rainfall settings. Projects saved with the previous docking
layout open with the default right pane. Reset layout restores the default right
pane and keeps results, selections and unfinished edits. Tab navigation and ribbon
actions are keyboard accessible. Ctrl+1 focuses **Map view**, Ctrl+2/3 opens Charts/Data,
Ctrl+S saves the draft, Ctrl+Q requests exit, and F5 refreshes monitoring.

## Workspace settings and operation drafts

Workspace Settings edits only the supported geometry and engine paths, using the
shared typed configuration read/save API. Save validates and atomically replaces
TOML; Revert restores the loaded/saved baseline. Failed saves retain edits and
existing destination bytes.

The Draft ribbon’s Run experiment form edits one shared
[operation draft](drafts.md). They show its fixed reference and timezone,
editable day horizons for version 2 drafts, and read-only coverage from the shared backend,
observation paths/provider/station/series selections, ordered existing replacements,
scenarios/forecast assets/ordered corrections, output stations, geometry/grid,
interpolation, QC and engine settings. Load existing instructions/assets reads the
selected catalogs independently without registering anything; an unavailable source
reports its finding while choices from another available source remain usable.
Selection toggles distinguish an unset field from an explicitly empty list (for
example, no replacement instructions or no output stations).

New draft fixes the reference and timezone from the selected Processing setup.
With no Processing setup, it uses the current hour in the workspace timezone.
Available workspace paths initialize without selecting providers, assets or
scenarios. Draft Open reads the selected file with its stored reference, timezone
and coverage. Automatic reopening considers only the remembered active draft with
the current-hour reference in the separately remembered operation timezone.
Version 1 drafts retain their explicit coverage when opened.

Draft Save writes under `config/drafts/` using
`operation-YYYYMMDDTHHMMSS±HHMM.json`. Save As permits a different destination while
requiring that reference-anchored suffix. Existing destinations require
explicit overwrite unless they are the active saved file. Incomplete drafts remain
saveable; operational edits are never automatically saved. Explicit
Accept/Refresh selected inputs accepts selected content through the shared backend;
Check readiness displays backend findings, including changed/unaccepted sources.
Monitoring F5 never accepts operational inputs. Preparing and running never
implicitly accept sources or register instructions.

**Draft → Review instructions** edits staged observation replacements and
forecast corrections. Select the items to register and use **Commit selected corrections**.
The two catalog outcomes report their registered IDs or their individual errors.
Working selections and their order are preserved. An unchanged retry reuses the
same IDs. Later edits keep the successful registration as their origin.

Before New/Open replaces a draft, or closing/switching a project or quitting, Save,
Discard and Cancel cover dirty configuration and draft edits, including hidden
forms. Cancel and failed saves retain the active project and edits. Preference
changes alone do not make operational work dirty.

## Controlled jobs

Processing → New run opens a standalone form. Select an existing [operation
TOML](operations.md#request). The reference defaults to the current hour in the
recipe timezone and can be edited for a historical run. The form recalculates
the two hourly periods as you edit. Check readiness validates the saved selections,
including forecast coverage of the requested period. Select a recipe with matching
forecast assets and corrections for the reference you intend to run.

Accept/Refresh selected inputs explicitly accepts those inputs for the current
reference. Changing the reference or selecting a recipe requires fresh acceptance.
Run All prepares and executes with the shared controlled pipeline and publishes
the new active run and its rainfall cache on success. Preparation is temporary. The operation
keeps any open draft and its unsaved edits independent. The Draft ribbon’s
experiment action uses the accepted draft values.

Operational file work and processing run on a dedicated worker, separately from
monitoring loading. One operational job runs at a time per workspace, with backend
workspace locking. Jobs retains stages, per-scenario status, bounded expandable log
tails, completion/failure and Cancel independently of runner visibility. Progress
is indeterminate when the backend supplies no fraction. The map remains usable.
Successful manual execution loads completed results into linked pages without
changing the active tab or focus; failures retain the previous usable display.

Leaving during a job offers Cancel job and leave or Stay. Cancellation retains the
project until workers/engines stop and controlled cleanup finishes, then resolves
unsaved work before leaving. Cleanup failures retain the project with an error.
If completion wins the cancellation race, completed results remain available.
Closing a form tab never cancels a job.

## Current qualification limits

Functional tests and a regional virtual X11 inspection pass on Linux. PNG checks
use Vulkan llvmpipe software rendering. These do not establish operator-hardware
performance, long-session memory behavior, HiDPI/input packaging, Wayland or
Windows compatibility. Performance/memory thresholds and hardware acceptance
remain open. Basemap equivalence to the legacy Carto style is unqualified.

The saved-data forms use the same validated workflows as the CLI.
Live acquisition and the assistant are later capabilities.
