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

Left-click a station or MINI on the map to highlight it and identify it in Layers.
Use Data search to select locations by ID or station name, including stations
without coordinates. Entering a positive numeric search selects that MINI even
without local topology. Visible station targets precede rivers and catchments.
River hit tolerance is wider than its stroke, and catchment containment excludes
holes. Alt-click inspects rainfall independently of selection. Station colors
identify explicit model links and unknown associations.

Right-click a feature for its actions. View time series opens Charts for that
feature, fills its ID and the explicitly linked counterpart when unique, and
clears the counterpart field when the link is unavailable or ambiguous. Left-click
and search selections leave the chart comparison and the active tab unchanged.
Charts has editable MINI ID and Station ID fields for manual pairing. Enter or
Apply submits those IDs. Data also offers the feature actions for its selected
location, including through keyboard navigation.
Model series use the selected MINI and
observations use the selected station. The level chart uses retained station
levels when the run records the matching MINI and offset. It labels raw MINI
levels and uses a separate scale when both datums have data. Station thresholds
use the station scale. An available side can be viewed independently.

Choose the retained run in Files, the map scenario in Rainfall, and the chart
overlays in Charts. Negative signed map hours select the run's observed
checkpoint and positive hours select its scenario checkpoint. Rainfall can also
use a standalone checkpoint. Missing sources are reported independently.

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
texture. Area/rain summaries use validated upstream topology and local MINI areas.
Load upstream basin uses the clicked MINI or the station's linked MINI when
topology is available. Layers names the loaded basin by its outlet MINI and
allows it to be hidden or cleared independently of later selections. Replacing
a basin clears the previous one as loading starts. Project changes release it.

## Display and exports

The map includes attributed raster basemaps, river segments, catchments, stations,
upstream fill, accumulated rainfall and a numeric legend. Operational layers have
visibility/order/opacity controls. Local vector files add display-only layers
with visibility, ordering, color and opacity controls; they do not alter inputs.
Polygon parts retain their holes. Overview river simplification is display-only;
close zoom uses original paths. Rainfall uses projected latitude strips to keep
pixel edges aligned in Web Mercator, rather than stretching four image corners.
Labels appear only at suitable zoom/spacing and do not intercept selection.

The ribbon, left column and right pane use English or Brazilian Portuguese. Automatic interface preferences remember recent workspace associations,
camera, layers, local files, selection, run/scenarios and time controls without a
new project-file schema. Source errors identify the retained view separately.

Rainfall, flow and level panels share time navigation and cursor position. Bars
use actual hourly interval edges. History and forecast are visually distinct;
masks break curves, and zero and signed values remain valid. Level thresholds
and forecast reference markers are shown. The observed-series control and scenario
checkboxes control chart sources. Plots, legends, and cursor readouts share the
same source colors, which stay stable when toggling scenarios within a run.
Charts exports the displayed snapshot. CSV keeps model and observation location
IDs on each row using the [shared series export](series.md) format. Complete PNG
export identifies the pair and combines all three panels,
localized titles, local-time axes, units, reference/threshold annotations and
legends at 1×, 2× or 3×, replacing its destination atomically.

## Layout and settings

The ribbon has Project, Draft, View, Processing and Settings tabs with labeled
groups of actions. The map stays visible in the center with zoom controls and
basemap attribution. The fixed left column shows the selected retained run's
`reference_time`, `observed_horizon_days`, `forecast_horizon_days` and available
scenario IDs, with scrollable Layers below. The summary comes from the loaded
run and its frozen recipe. Day horizons appear only when the saved hourly
boundaries convert losslessly. Available scenarios have successful, readable
basin results. Unavailable fields show a dash.

The resizable right pane starts on Charts and also has Data, Rainfall and Files
tabs. Data offers location search, summaries and the displayed series table.
Rainfall controls the map's scenario, time window and checkpoint. Files contains
the retained run selector, provenance, assets and findings. Use the ribbon to
open Workspace Settings, Interface Settings, Prepare inputs, Execute MGB, Run All
and Jobs in the same pane. Closing a form tab preserves its inputs. Jobs can
also reopen from the status bar.

Settings offers English/Brazilian Portuguese and System/Light/Dark, applied
immediately and remembered app-wide. Preferences remember each project's right
pane width and tabs. Projects saved with the previous docking layout open with
the default pane. Reset layout restores the default pane and keeps results,
selections and unfinished edits. Tab navigation and ribbon actions are keyboard
accessible. Ctrl+1 focuses the map zoom controls, Ctrl+2/3 opens Charts/Data,
Ctrl+S saves the draft, Ctrl+Q requests exit, and F5 refreshes monitoring.

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

The saved-data ribbon actions add canonical import, observation review,
instruction previews, forcing and state settings, geometry and soil editors,
run comparison and sharing, bulletins, and automation. Use **Project** for import,
**Draft** for instructions, **View** for review and outputs, **Settings** for
calibration, and **Processing** for forcing and Jobs. Task forms use the same
validated workflows as the CLI. See [saved-data workflows](saved-data.md) for
inputs and configuration. Live acquisition and the assistant are later capabilities.
