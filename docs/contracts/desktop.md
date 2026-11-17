# Desktop monitoring

Build/run the optional native interface:

```sh
cargo run --locked --features desktop --bin mgb-ops-desktop -- /region/config/workspace.toml
```

With no argument, reopen the most recent project or use Open TOML. Standard
`config/workspace.toml` locations infer their workspace; other locations ask for
a folder and remember the association. Relative resource paths still resolve
against that workspace under the [workspace contract](workspace.md).

Opening and Refresh/F5 are read-only. Missing engine prerequisites, caches,
geometry or observations report findings without preventing unrelated available
data from loading. Work runs in a background worker. Newer requests supersede
older replies; replacement failures retain existing display data with feedback.
Geometry snapshots are versioned by their configured files. Unchanged sources
reuse prepared geometry; a failed replacement retains the previous matching
topology/geometry together while unrelated sources update. Retained sources are
identified in the controls. The map camera and valid selection survive refresh.
Closing a project releases
its displayed data and stops its loader.

## Data and selection

The interface reads configured regional mini segments/catchments and MINI.gtp,
`data/history.sqlite`, `data/forecasts.sqlite`, and backend-discovered rainfall
checkpoints and run manifests. Run state is displayed, including diagnostics and
successful scenarios from partially failed attempts. Missing/corrupt scenario
outputs produce findings rather than hiding usable siblings.

Select a station/MINI through search, the location list or the map. Entering a
positive numeric search selects that MINI, even without local topology. Stations
without coordinates remain selectable. Station targets precede river/catchment
targets. River hit tolerance is wider than its stroke; catchment containment
excludes holes. Alt-click inspects rainfall without changing the location.
Linked summaries, the table and three charts use shared backend queries.
Missing station offsets stay unknown; an explicit metadata link supplies the mini.
Station colors identify known model links and unknown associations.

Choose the map scenario and chart scenario overlays from one loaded run.
Run/scenario selection associates the corresponding retained rainfall checkpoint;
negative signed hours select its observed checkpoint and positive hours its
scenario checkpoint. Missing remembered/requested resources report errors and
are not replaced by another run or scenario. An explicitly selected standalone
rainfall checkpoint remains independently inspectable.

Signed hours and explicit start/end controls use shared hourly-window semantics.
Both display the actual interval. Apply selects the entered rainfall and chart
windows. Reference times and timestamps are local to the displayed timezone.
Without a run, the initial display window uses the current project-local hour
and the preceding week; this is a display preference, not an operational recipe.
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

Resizable controls, map and lower chart/table panes use English or Brazilian
Portuguese. Automatic interface preferences remember recent workspace associations,
camera, layers, local files, selection, run/scenarios and time controls without a
new project-file schema. Source errors identify the retained view separately.

Rainfall, flow and level panels share time navigation and cursor position. Bars
use actual hourly interval edges. History and forecast are visually distinct;
masks break curves, and zero and signed values remain valid. Level thresholds
and forecast reference markers are shown. The observed-series control and scenario
checkboxes control chart sources; legends identify those sources. CSV uses the
[shared series export](series.md). Complete PNG export combines all three panels,
localized titles, local-time axes, units, reference/threshold annotations and
legends at 1×, 2× or 3×, replacing its destination atomically.

## Current qualification limits

Functional tests and a regional virtual X11 inspection pass on Linux. PNG checks
use Vulkan llvmpipe software rendering. These do not establish operator-hardware
performance, long-session memory behavior, HiDPI/input packaging, Wayland or
Windows compatibility. Performance/memory thresholds and hardware acceptance
remain open in the
[GUI plan](../../.dev/plans/desktop_gui.md#remaining-monitoring-qualification).
Basemap equivalence to the legacy Carto
style is not qualified. The desktop does not yet expose preparation/execution,
configuration editing or the assistant; those follow monitoring acceptance.
