# Desktop interface

Build/run the optional native interface:

```sh
cargo run --locked --features desktop --bin mgb-ops-desktop -- /region
```

With no argument, reopen the most recent project or use **Open workspace** to
select its directory. The desktop loads `config.toml` from that directory.

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

Use **Pages → Assistant** to open an [agent conversation](assistant.md) in the
current workspace.

| Task | Control |
| --- | --- |
| Open a workspace or import saved data | **Title strip → Open workspace** or **Monitoring → Import data** |
| Compare map rainfall | **Map → Rainfall** and **Map view** above the map |
| Step through modeled inundation | Use **Map → Inundation** |
| Inspect location data or a run | **Pages → Charts**, **Data**, or **Files** |
| Review observation availability and QC | **Review and runs → Observation review** |
| Compare runs, prepare a bulletin, or share a run | **Review and runs → Run comparison**, **Bulletins**, or **Share runs** |
| Edit observations | **Review and runs → Observation review** |
| Run a workspace operation at a selected reference | **Operation → New run** |
| Set forcing and state options | **Operation → Forcing and state** |
| Schedule jobs or monitor and cancel a job | **Operation → Automation** or **Review and runs → Jobs** |
| Edit workspace settings | **Title strip → Settings** |
| Edit geometry or soil | **Calibration** |

See [saved-data workflows](saved-data.md) for request formats and outputs and
[controlled jobs](#controlled-jobs) for execution and cancellation.

## Data and selection

The interface reads configured regional mini segments/catchments and MINI.gtp,
`operation.sqlite`, the [active rainfall
cache](operations.md#active-run-and-rainfall-cache), and the workspace's active run.
Charts, Files and Map open the active generation automatically. While viewing
the active run, a newly published generation updates all three views. Map overlays
retain the previous display until the new generation is ready. Files shows the displayed
run's status and provenance. Missing or corrupt scenario outputs produce
findings. For previous attempts, see the [run history](operations.md#active-run-and-rainfall-cache).

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
| Hover over the map | Highlight the station or MINI under the pointer and see its ID beside the pointer. The bottom stripe shows accumulated cell rainfall; Double and Swipe identify the hovered side and scenario. |
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
Use **Apply** to commit a period edit. A successful **Operation → New run → Run All** supplies
the new active cache and refreshes the open view. Use Refresh/F5 to load a generation published by a separate process while
the desktop is idle.

Enable **Inundation** under **Map → Layers** to see the displayed run's scenario
mask and controls. Step through available frames with the previous/next buttons,
slider, or arrow keys while the slider has focus. The calendar and time selector
jump to an available local timestamp; unavailable dates are disabled. The control
starts at the first forecast hour. Double and Swipe use the same frame on both
sides. Refresh/F5 reloads generated masks and retains a valid scenario and frame. See
[inundation processing](inundation.md) for the command and regional inputs.

Open **Settings** on the Charts toolbar to edit the location, date window,
and model scenario overlays. Chart timestamps use
the run or project observation timezone. Without a run, the initial chart window
uses the current project-local hour and the preceding week. Chart loading runs independently of map and layer reads. Charts
reports loading, data availability, and errors for the requested pair. If a
replacement fails, a retained comparison keeps its original displayed IDs and
exports.
Rainfall inspection returns the accumulated original numeric cell, including NA
and supplied zero. Display opacity does not alter accumulation or recreate its
texture. The selected MINI or linked-station card's **Upstream basin rainfall over
period** shows the area-weighted mean accumulation in mm, scenario, and period
for each displayed side. The
rainfall total and selected basin area include the outlet catchment and all its
upstream catchments. The totals use frozen MINI topology and local areas.
Project topology supplies the area when a frozen summary is unavailable. Load
upstream basin uses the clicked MINI or the station's linked MINI when
topology is available. The top map summary card names the loaded basin by its
outlet MINI and offers **Clear basin**, independently of later selections. Its
layer controls visibility. Replacing
a basin retains the previous one until its replacement succeeds. Project changes release it.

## Display and exports

The map includes attributed raster basemaps, river segments, catchments, stations,
an upstream basin outline and accumulated rainfall. Expand the Rainfall layer
with **+** to show its
linear colour bar, initially blue, and editable scale maximum. The scale runs
from 1 mm to the configured maximum, which defaults to 100 mm. The expanded
layer also shows the actual loaded grid minimum and maximum in mm for each
displayed side, including valid zero; an entirely unknown grid shows NA. These
values describe the loaded data independently of the colour limits. Colors at the
endpoints also cover values outside that range. Incomplete coverage remains
transparent and inspection reports NA; the rainfall card omits coverage and NA badges.
Use the checkbox in a layer row to toggle visibility. Click a colour swatch to
open the colour matrix or custom picker. Rainfall changes its scale hue; stations
have separate linked and unknown-link colours. Expand the row with **+** for
order, opacity, and applicable stroke-thickness controls in display points.
Catchment divisions have a default 1-point outline, including hole boundaries;
an explicitly saved zero thickness disables their strokes. Upstream basins offer
Outline or Filled; fills preserve holes and separate parts.
Local vector rows also offer removal controls. Local vector layers affect only the display.
Polygon parts retain their holes. At regional zoom, river paths are simplified
with at most half a display pixel of error. Close zoom and river hit testing use
the original paths. Rainfall uses projected latitude strips to keep
pixel edges aligned in Web Mercator, rather than stretching four image corners.
Labels appear only at suitable zoom/spacing and do not intercept selection.

The ribbon, left column and right pane use English or Brazilian Portuguese.
Automatic interface preferences remember file-dialog locations separately by task
across restarts, as well as recent workspace associations,
camera, layers, local files, selection, run/scenarios and time controls without a
new project-file schema. File dialogs start from the current field path when
supplied, otherwise the
remembered task location, then the workspace. Missing locations fall back to an
existing ancestor; cancelling leaves the remembered location unchanged. Source
errors identify the retained view separately.

Rainfall, flow and level panels use coordinated source colors, restrained major
grids, and theme-aware surfaces, and share time navigation and cursor position. The
interactive charts scroll vertically on short windows and open with padding
around the Y ranges. Bars
use actual hourly interval edges. History and forecast are visually distinct;
masks break curves, and zero and signed values remain valid. Level thresholds
and forecast reference markers are shown. The observed-series control and scenario
checkboxes control chart sources. Plots, legends, and cursor readouts share the
same source colors, which stay stable when toggling scenarios within a run.
Hover over a plot to see the crosshair and its value label and update the
readings below the charts. The first primary click pins the crosshair and the
shared time across all three plots. The next primary click resumes hover updates.
The readings are grouped by variable and remain visible after the pointer
leaves the plot. Loading a replacement comparison clears the pin.
The **Export** toolbar control exports the displayed snapshot. CSV keeps model and observation location
IDs on each row using the [shared series export](series.md) format. Complete PNG
export identifies the pair and combines all three panels,
localized titles, a light report background, local-time axes, units, reference/threshold annotations and
legends at 1×, 2× or 3×, replacing its destination atomically.

## Layout and settings

The native window title bar uses the operating system’s controls and colors.
Below it, a title strip shows the HGE emblem and MGB Ops title, followed by compact
Open workspace, Recent workspaces, Settings, and Close project icons. Hover
reveals localized action labels; icons also expose accessible names. The ribbon
uses bundled color emoji in
Monitoring, Pages, Review and runs, Layout, Operation, and Calibration subgroups. Scroll horizontally when the
window is narrow; keyboard focus brings actions into view. The map stays visible
in the center with linked basemap
attribution in the bottom stripe when tiles are enabled. NASA Blue Marble provides a satellite basemap at regional scale.
**Map view** offers **Single**, **Double**, and **Swipe**. Double synchronizes the
center and zoom of two side-by-side maps. Swipe compares the overlays at the same
scale and location using a draggable divider. Focus the divider and use the arrow
keys to move it. Both comparison modes show left and right scenario and period
controls in Rainfall. The views share the basemap, layers, and selected feature.
The wheel zooms without Ctrl, and the middle or primary button pans. On-map
buttons in the upper-right corner zoom in/out and fit the regional extent. The uncovered map background
matches the right dock groups in Light and Dark appearances.

Map updates retain the last successful rainfall, basin, or inundation display
while replacements load or fail. Retained displays keep their original source
and time labels. Inundation switches visible tiles together after the new frame
is ready, including fully transparent results. A committed workspace switch
releases the previous workspace's overlays.
The one-line bottom stripe shows rainfall under the map pointer, along with the displayed run's reference time and timezone,
observed and forecast horizons, and available scenarios. The summary comes from
the loaded run and its frozen settings. Day horizons appear only when the saved
hourly boundaries convert losslessly. Available scenarios have successful,
readable basin results. Unavailable fields in a loaded run show a dash.

The resizable right pane defaults to Map. The Map tab contains Layers and
Rainfall. Data offers location search, feature actions and the displayed series
table. Narrow panes group each row's time, series, value, unit, and variable into
two columns. Files contains the run selector, provenance, assets and findings. Open
pages and forms from the ribbon. Closing a tab preserves its page state.
Drag tabs along a strip to reorder them, onto another strip or group to move
them, or onto a highlighted edge to split the group. Groups can be stacked,
side by side, or arranged in a four-cell grid. Each group has a rounded border
and space between it and neighboring groups. With three groups, one spans a row
or column. Ribbon actions open tabs in the most recently focused group
or focus the group that already contains the tab. Each page stays within its
group. Hover over scrollable content to reveal its vertical scrollbar. Wide
tables can present rows as labeled fields in narrow groups.

The **Layout** group has a language button, a Light/Dark control,
and **Reset layout**. The control highlights the active appearance. Language and appearance changes apply immediately and
are remembered app-wide. Light is the default. Preferences remember each project's right pane, map view, divider position,
and applied rainfall settings. Projects saved before map comparisons open in Single
with their previous rainfall settings. Projects saved with a flat right pane open with their tabs in one group. Reset layout restores the default right
pane and keeps results, selections and unfinished edits. Tab navigation and ribbon
actions are keyboard accessible. Ctrl+1 focuses **Map view**, Ctrl+2/3 opens Charts/Data,
Ctrl+S saves workspace settings, Ctrl+Q requests exit, and F5 refreshes monitoring.
Reading text outside map and chart canvases supports drag selection and keyboard
copy. Double-click reading text to select a word and open Copy/Select all actions.
Editable fields retain their ordinary selection and editing behavior.

## Workspace settings and run parameters

**Title strip → Settings** edits the geometry and engine paths in
`config.toml`. Save validates and atomically replaces the file. Revert restores
the last saved settings. Closing or switching a workspace prompts about
unsaved workspace settings.

**Operation → New run** opens the workspace defaults. Edit the local reference
hour and day horizons. The form shows the resulting coverage and scenarios.
Use **Restore defaults** to select zero future rain and the latest usable raw
cycle per product. Add, remove, reorder or pin scenarios for the current run.
Add a corrected scenario for a specific cycle to edit its transformations and
preview the raw and corrected grids on the same scale. Run changes are
temporary. **Save preparation** freezes the selected inputs for later execution.

**Review and runs → Observation review** shows availability and spatial rainfall checks.
Load, edit, and save the ordered workspace observation edits from the same page.

## Controlled jobs

**Run All** acquires the configured inputs, prepares and executes, and publishes
the results and rainfall cache together on success. Use **Operation → Acquire
inputs** to update the catalog independently. See [acquisition](acquisition.md).

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

Functional tests and a synthetic virtual X11 inspection pass on Linux. PNG checks
use Vulkan llvmpipe software rendering. These do not establish operator-hardware
performance, long-session memory behavior, HiDPI/input packaging, Wayland or
Windows compatibility. Performance/memory thresholds and hardware acceptance
remain open. Basemap equivalence to the legacy Carto style is unqualified.

The saved-data and acquisition forms use the same validated workflows as the
CLI. See the [assistant contract](assistant.md) for assistant adapter settings.
