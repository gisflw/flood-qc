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

## Data and selection

The interface reads configured regional mini segments/catchments and MINI.gtp,
`data/history.sqlite`, `data/forecasts.sqlite`, and backend-discovered rainfall
checkpoints and run manifests. Run state is displayed, including diagnostics and
successful scenarios from partially failed attempts. Missing/corrupt scenario
outputs produce findings rather than hiding usable siblings.

Select a station or MINI on the map or through Map search. Entering a positive
numeric search selects that MINI even without local topology. Stations without
coordinates remain selectable through search. Station targets precede river and
catchment targets. River hit tolerance is wider than its stroke, and catchment
containment excludes holes. Alt-click inspects rainfall independently of selection.
Station colors identify explicit model links and unknown associations.

Charts has editable MINI ID and Station ID fields. A map or search selection
fills its ID and the explicitly linked counterpart when unique. If the
counterpart is unknown or ambiguous, its field keeps its current value.
Enter or Apply submits the edited IDs. Model series use the selected MINI and
observations use the selected station. The level chart uses retained station
levels when the run records the matching MINI and offset. It labels raw MINI
levels and uses a separate scale when both datums have data. Station thresholds
use the station scale. An available side can be viewed independently.

Choose the retained run in Files, the map scenario in Map, and the chart overlays
in Charts. Negative signed map hours select the run's observed checkpoint and
positive hours select its scenario checkpoint. A standalone checkpoint can be
chosen in Map. Missing sources are reported independently.

Apply the rainfall and chart windows in their respective pages. Both use shared
hourly-window semantics. Map timestamps use the displayed timezone and chart
timestamps use the run timezone or the project observation timezone. Without a
run, the initial display window uses the current project-local hour and the
preceding week. Chart loading runs independently of map and layer reads. Charts
reports loading, data availability, and errors for the requested pair. If a
replacement fails, a retained comparison keeps its original displayed IDs and
exports.
Rainfall inspection returns the accumulated original numeric cell, including NA
and supplied zero. Display opacity does not alter accumulation or recreate its
texture. Area/rain summaries use validated upstream topology and local mini areas.

## Display and exports

The map includes attributed raster basemaps, river segments, catchments, stations,
upstream fill, accumulated rainfall and a numeric legend. Operational layers have
visibility/order/opacity controls. Local vector files add display-only layers
with visibility, ordering, color and opacity controls; they do not alter inputs.
Polygon parts retain their holes. Overview river simplification is display-only;
close zoom uses original paths. Rainfall uses projected latitude strips to keep
pixel edges aligned in Web Mercator, rather than stretching four image corners.
Labels appear only at suitable zoom/spacing and do not intercept selection.

The ribbon, left sidebar and center pages use English or Brazilian Portuguese. Automatic interface preferences remember recent workspace associations,
camera, layers, local files, selection, run/scenarios and time controls without a
new project-file schema. Source errors identify the retained view separately.

Rainfall, flow and level panels share time navigation and cursor position. Bars
use actual hourly interval edges. History and forecast are visually distinct;
masks break curves, and zero and signed values remain valid. Level thresholds
and forecast reference markers are shown. The observed-series control and scenario
checkboxes control chart sources; legends identify those sources. Charts exports the displayed snapshot. CSV keeps model and observation location
IDs on each row using the [shared series export](series.md) format. Complete PNG
export identifies the pair and combines all three panels,
localized titles, local-time axes, units, reference/threshold annotations and
legends at 1×, 2× or 3×, replacing its destination atomically.

## Layout and settings

The ribbon has Project, Draft, View, Processing and Settings tabs with labeled
groups of visible actions. The single left sidebar resizes and collapses.
Files contains the retained run selector, provenance, assets and findings.
The ribbon opens Workspace Settings, Interface Settings, Prepare inputs,
Execute MGB, Run All and Jobs as tabs in the same sidebar. Closing a form
preserves its current inputs. Jobs can also reopen from the status bar.

Map and Charts initially appear side by side. The map has an inset, bordered
viewport below its controls. Data contains location summaries and the selected
series table. Drag a center tab to join a group or split at an edge;
a shaded preview shows its destination. Each axis splits at most once along a
branch, supporting one area, rows, columns, three areas and a 2×2 grid, with
resizable dividers and at most four areas. Each page has one instance; moving or
closing pages preserves their view state and shared selection. Linked background
updates never focus hidden pages. Floating windows are unsupported.

Settings offers English/Brazilian Portuguese and System/Light/Dark, applied
immediately and remembered app-wide. Interface preferences remember each project's
sidebar width, visibility, tabs and docking. Old right-pane tabs migrate to
the sidebar. Reset layout restores the initial Map/Charts arrangement and Files
tab while preserving results, selections and unfinished edits. Tab navigation and ribbon actions are keyboard
accessible; Ctrl+1/2/3 reopens/focuses Map/Charts/Data, Ctrl+S saves the draft, Ctrl+Q
requests exit, and F5 refreshes monitoring.

## Workspace settings and operation drafts

Workspace Settings edits only the supported geometry and engine paths, using the
shared typed configuration read/save API. Save validates and atomically replaces
TOML; Revert restores the loaded/saved baseline. Failed saves retain edits and
existing destination bytes.

Prepare inputs and Run All edit one shared version 2 [operation draft](drafts.md).
They expose reference/timezone/day horizons and read-only shared-backend coverage,
observation paths/provider/station/series selections, ordered existing replacements,
scenarios/forecast assets/ordered corrections, output stations, geometry/grid,
interpolation, QC and engine settings. Load existing instructions/assets reads the
selected catalogs independently without registering anything; an unavailable source
reports its finding while choices from another available source remain usable.
Selection toggles distinguish an unset field from an explicitly empty list (for
example, no replacement instructions or no output stations). Existing imported instruction values
remain preserved; their editors belong to later work.

New run always creates a fresh recipe with a fixed current-hour reference in the
selected operation timezone. Available workspace paths initialize without selecting
providers, assets or scenarios. Draft Open establishes a candidate reference and IANA
timezone before reading the file; it defaults to the current hour and permits an
explicit reference. Content must match both exactly. Automatic reopening considers
only the remembered active draft with the independently established current-hour
candidate in the separately remembered operation timezone; it never derives that
candidate from the saved file or falls back to older drafts. Legacy day-horizon conversion is
lossless or fails with the original file preserved for existing CLI workflows.

Draft Save writes under `config/drafts/` using
`operation-YYYYMMDDTHHMMSS±HHMM.json`. Save As permits a different destination while
requiring that reference-anchored suffix. Changing the reference chooses a new
anchored destination and preserves the old file. Existing destinations require
explicit overwrite unless they are the active saved file. Incomplete drafts remain
saveable; operational edits are never automatically saved. Explicit
Accept/Refresh selected inputs accepts selected content through the shared backend;
Check readiness displays backend findings, including changed/unaccepted sources.
Monitoring F5 never accepts operational inputs. Preparing and running never
implicitly accept sources or register instructions.

Before New/Open replaces a draft, or closing/switching a project or quitting, Save,
Discard and Cancel cover dirty configuration and draft edits, including hidden
forms. Cancel and failed saves retain the active project and edits. Preference
changes alone do not make operational work dirty.

## Controlled jobs

Prepare validates/freezes current accepted draft values and retains rainfall inputs
without launching MGB; generated rainfall becomes available on the map. Execute
MGB selects a retained preparation independently of the draft and can display its
frozen recipe, period, scenarios and provenance. Run All uses the shared controlled
draft preparation/execution workflow.

Operational file work and processing run on a dedicated worker, separately from
monitoring loading. One operational job runs at a time per workspace, with backend
workspace locking. Jobs retains stages, per-scenario status, bounded expandable log
tails, completion/failure and Cancel independently of runner visibility. Progress
is indeterminate when the backend supplies no fraction. The map remains usable.
Successful manual execution loads completed results into linked pages without
changing tabs, focus or docking; failures retain the previous usable display.

Leaving during a job offers Cancel job and leave or Stay. Cancellation retains the
project until workers/engines stop and controlled cleanup finishes, then resolves
unsaved work before leaving. Cleanup failures retain the project with an error.
If completion wins the cancellation race, completed results remain available.
Closing/collapsing panes never cancels a job.

## Current qualification limits

Functional tests and a regional virtual X11 inspection pass on Linux. PNG checks
use Vulkan llvmpipe software rendering. These do not establish operator-hardware
performance, long-session memory behavior, HiDPI/input packaging, Wayland or
Windows compatibility. Performance/memory thresholds and hardware acceptance
remain open in the
[GUI plan](../../.dev/plans/desktop_gui.md#remaining-monitoring-qualification).
Basemap equivalence to the legacy Carto style is not qualified. The assistant,
acquisition and instruction editors remain outside the current interface.
