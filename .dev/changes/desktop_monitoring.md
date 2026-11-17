# Desktop monitoring implementation plan

Status: planned; no desktop implementation or rendering qualification is claimed.
Decision record: November 17, 2026.

Deliver the legacy Python monitoring experience as the first milestone of the
Rust desktop application. Preserve its responsive map, linked location selection,
rainfall inspection, summaries and hydrological charts. Use egui with rsplot;
Galileo is the preferred map renderer pending the qualification gates below.
Walkers remains the fallback, not a second renderer to ship.

The [desktop GUI specification](../plans/desktop_gui.md) remains authoritative for
the broader first delivery: project configuration, monitoring, preparation,
execution and assistant behavior. This plan specifies implementation order and
technology qualification for its monitoring milestone; it does not remove the
remaining delivery requirements or claim new backend capabilities.

## Decisions and scope

| Decision | Status and consequence |
| --- | --- |
| Monitoring first | Agreed. Reproduce useful legacy behavior before adding processing and assistant interfaces. |
| Rust desktop implementation | Agreed direction. Prefer native Rust UI/rendering without Python, JavaScript or a webview in the operational display path. Platform libraries and graphics drivers are still dependencies. |
| egui application shell | Working choice. Use eframe with wgpu so map and chart integrations can share the rendering device. Qualify the complete dependency combination before implementation. |
| rsplot charts | Selected. Verify the hydrological comparison rather than substituting egui_plot or a basic line-chart demonstration. |
| Galileo map | Preferred, not finally qualified. Retained geometry and redraw-controlled map textures fit the workload; cache policy, precision and dependency compatibility are gates. |
| Walkers | Fallback if Galileo qualification requires disproportionate maintenance. Do not build a renderer abstraction or maintain both implementations. |
| Legacy behavior | Acceptance reference, including interaction and performance. Reuse its successful caching and independent update patterns, without reproducing its Panel layout or JavaScript wiring. |
| Backend ownership | Unchanged. Configuration, formats, identity, CRS conversion, topology, accumulation and scientific series stay in shared Rust APIs. |

### First milestone

- Open a configured regional project and inspect available observations, static
  geometry, retained rainfall and scenario results. Missing caches or engine
  prerequisites must not prevent inspection of unrelated available data.
- Select a retained run and map scenario; overlay scenarios from that run in
  charts. Include successful scenario outputs from partially failed attempts.
- Render basemaps, stations, river segments, catchments, accumulated rainfall and
  the selected upstream basin with appropriate controls and legends.
- Maintain one consistent station/mini selection across map, search/table,
  summary and three hydrological chart panels.
- Preserve signed-hour rainfall controls and add explicit interval selection
  using equivalent shared backend semantics.
- Preserve English and Brazilian Portuguese labels, units, actual timestamps,
  source/run identity and actionable loading/error status.
- Restore display preferences and refresh sources without losing the map extent
  or valid selection. Keep available data usable while replacement data loads.

Project-opening conveniences and display persistence follow the existing GUI
specification; they are not a new project-file schema. Start with resizable map
and side/bottom panes. The exact arrangement remains an implementation choice;
no docking framework is required by this milestone.

### Subsequent work

Expose project/settings editing, retained preparation, Execute and Run all, then
the assistant behavior under the existing specification. Preserve its explicit
review/approval and result-loading rules when implementing the assistant.
Monitoring completion alone does not complete that broader delivery.

QC/correction editors, synchronized forecast-comparison maps, comparisons across
runs, acquisition, hot starts, calibration editors, bulletins, scheduling and
email retain their existing deferred scope. Do not introduce inactive controls,
a general GIS application, plugin machinery or another operational store.

## Existing behavior and workload

Use these legacy files as the behavioral reference:

- [Monitoring composition](../../../legacy_python/apps/ops_dashboard/views/monitoring.py)
  for controls, selection updates, scenario overlays, refresh and chart export.
- [Map construction](../../../legacy_python/apps/ops_dashboard/services/deckgl.py)
  for layer order, symbols, labels, opacity, inspection and identity lookup.
- [Cached loaders](../../../legacy_python/apps/ops_dashboard/services/loaders.py)
  for source-version invalidation and display-only river simplification.
- [Chart construction](../../../legacy_python/apps/ops_dashboard/views/charts.py)
  for rainfall bars, scenario styles, thresholds, reference time and legends.
- [Dashboard state](../../../legacy_python/apps/ops_dashboard/state.py) for the
  selection and source-refresh behavior connecting those views.

Read-only inspection of the regional `data/source/rs_hydro.gpkg` found:

| Layer | Features | Coordinate vertices | Largest feature |
| --- | ---: | ---: | ---: |
| `mini_segments` | 8,649 | 873,934 | 134 vertices |
| `mini_catchments` | 8,649 | 945,112 | 300 vertices |

These counts precede display simplification and describe the inspected regional
file, not universal limits. Its catchments contain no interior holes. The two
layers together have 1,819,046 vertices; qualification must use regional geometry,
not just a basemap with a handful of markers. Do not commit regional data.

The legacy performance patterns to preserve are versioned bounded caches,
display-only simplified river paths, rainfall images separate from numeric
lookup data, opacity changes without recalculation, and selection updates that
do not rebuild static map layers. Source refresh retains the camera.

## Renderer assessment and qualification gates

The following are source observations from diagnosis, not measured performance
results. Upstream branches and published crates differ. Recheck the exact
versions/revisions selected for implementation and retain their identities with
qualification evidence.

### Galileo preference

Galileo's egui integration renders into a GPU texture using egui's device/queue
and reuses that texture until the map requests redraw. It avoids CPU image
readback for map presentation. This is attractive when rsplot hover/crosshair
updates repaint the UI while the map is stationary.

Its feature layers retain prepared GPU bundles and support resolution-dependent
representations. Initial preparation still traverses the layer, and the inspected
feature-layer path has no feature-level viewport filtering. Updating a feature
can rebuild other features sharing its bundle. Retention trades memory and
preparation cost for reuse; background preparation and loading responsiveness
must be demonstrated, not inferred from the rendering API.

Gates before adopting Galileo:

1. **Dependency compatibility:** build the selected rsplot, egui/eframe,
   egui-wgpu, wgpu and Galileo revisions together with one compatible renderer
   type stack. Diagnosis found rsplot using egui 0.36 and both candidate map
   branches using 0.37. Do not assume latest releases interoperate or solve the
   mismatch by accidentally running two GPU integrations. Prefer compatible
   supported releases; assess any necessary narrow upstream update explicitly.
2. **Memory:** establish practical bounded tile/geometry/texture retention for
   long operator sessions. The inspected raster provider has 5,000-entry caches
   for tile states and decoded images. This is capacity, not upfront allocation
   or proof of actual VRAM usage. Confirm CPU/GPU residency, eviction and public
   configuration before choosing a limit; avoid shipping that default unchecked.
3. **Precision and alignment:** check rivers, catchment edges, shared boundaries
   and rainfall at regional and close zoom. A reported f32 tessellation issue can
   affect geographic geometry away from the projection origin. Verify the pinned
   code rather than treating an issue report as a reproduced failure.
4. **Interaction:** prove selection tolerance, highlighting, map redraw requests,
   source replacement, resizing and HiDPI behavior with the actual egui layout.
5. **Maintenance:** prefer an upstream fix or small compatible change. If adopting
   Galileo requires substantial private renderer maintenance, reconsider walkers
   before building more product functionality on it.

### Walkers fallback

Walkers supplies raster basemaps, optional vector/MVT drawing and PMTiles, with
direct egui overlays. Its inspected source-data and styled-tile caches each have
256-entry limits, not byte budgets. Vector GPU uploads are reused for retained
drawables and retired when unowned or unused.

The current GeoJSON helper uses an R-tree to find visible features, but projects
and rebuilds their drawing data during rendering. At regional overview much of
our geometry is visible. A custom retained operational overlay is possible but
would make geometry preparation, zoom-correct pixel widths and invalidation our
responsibility. The widget participates in egui repaints even with retained GPU
uploads. Its GeoJSON helper does not supply the complete selection behavior.

If Galileo fails its gates, measure walkers with simplified regional layers and
the required interactions. Check reported clipped-overlay drift in the chosen
layout. A fixed map pane reduces exposure but is not a correctness test. Keep
offline PMTiles as an available capability, not a new milestone requirement.

Do not use an upstream WASM vector-performance report as a native benchmark.
Neither library promises complete compatibility with the legacy Carto style.
Qualify the desired basemap/style or record an accepted raster equivalent.

## Monitoring rendering and interaction

### Layer behavior

| Layer | Rendering and update policy | Interaction |
| --- | --- | --- |
| Basemap | Retained tile source; attribution visible; selected source persists. | Pan/zoom without resetting on other updates. |
| Rainfall | Georeferenced colored texture with transparent missing cells and legend; replace when source/window changes. | Inspect the original numeric cell; adjust opacity without reaccumulation or texture recreation. |
| Upstream basin | Translucent fill and outline derived from validated topology and associated catchments. | Show selected membership and area/rainfall summaries without deriving drainage from polygons. |
| Catchments | Retained multipart geometry with valid exterior/hole relationships and controlled visibility. | Select a mini through polygon containment. |
| River segments | Display-only simplified paths, pixel-sized strokes and retained mini identity. | Hover and click with a tolerance wider than the visible stroke. |
| Stations | Stable coordinates, status colors, contrasting outline and bounded screen size. Update attributes independently of geometry. | Tooltips and usable screen-space targets; coordinate-less stations remain searchable/selectable. |
| Rainfall labels | Readable pixel-sized text and offsets, visible above the appropriate zoom threshold. | Avoid labels intercepting station selection; assess overlap at actual density. |
| Highlight | Small separate selection/hover overlay where practical. | Selection must not invalidate the full network. |

Respect display layer ordering, visibility, opacity, styling and legends from the
GUI specification. Adding a local display layer must not change operational
geometry or model inputs. Renderer adapters consume backend geometry directly;
do not add a GeoJSON serialization round trip merely to reach a helper API.

### Geometry and projection

Preserve authoritative coordinates and identities. Simplification belongs only
to the display representation and must not alter topology, area weights or
scientific calculations. Use validated backend upstream membership and local
mini areas for summaries; never infer drainage from shapes or measure area in
geographic degrees.

The existing map reader flattens polygon exterior/interior rings and multipart
geometry into path lists. Before filled catchments or general local layers are
implemented, determine whether its return type must preserve explicit polygon
and ring relationships. Identify any necessary shared backend change separately,
including all callers, exports, fixtures and contracts it affects. Current
regional polygons without holes do not qualify general polygon handling.

Rainfall grids require a correct geographic-to-display transformation. Regular
latitude spacing is nonlinear in Web Mercator: stretching an image between four
projected corners can misalign interior cells. Qualify an appropriately warped
mesh or display reprojection, including pixel-edge/center conventions, north/south
orientation, transparent missing cells and zoom alignment. Choose the smallest
correct representation; no tile server is required merely to show a local grid.
Display resampling must not change backend values returned by inspection.

### Selection and refresh

Keep one application selection carrying explicit station and/or mini identity
and applicable run/scenario context. Coordinate-less station selection is valid;
coordinates must not invent a station-to-mini association or level offset.

Define and test overlap priority, normally station before river/catchment, with
rainfall inspection available without silently changing the selected location.
Convert screen tolerances to the layer's coordinate system correctly at the
current zoom/DPI. Galileo's current feature queries scan all features and are not
exact hit tests against rendered symbols. Start with measurement; add a spatial
index only if the actual hover/click workload needs it. Avoid expensive geometry
scans on every pointer movement without evidence they meet the response budget.

Source refresh replaces only affected data and retains map extent and valid
selection. Missing remembered resources receive explicit feedback. Keep the last
valid view during loading; on failure distinguish retained data from new data
instead of silently substituting another source or scenario.

## rsplot hydrological charts

rsplot is the selected chart library. Its scientific widgets, wgpu integration,
linked-axis support and export facilities must be qualified against this set:

- Three aligned rainfall, flow and level panels with one visible time interval,
  shared time navigation and a coordinated cursor/hover readout.
- Rainfall bars representing actual interval widths, with observation/basin and
  scenario distinctions; do not treat interval rainfall as instantaneous points.
- Observed, historical simulation and forecast sections, multiple scenario
  overlays, consistent colors/styles and controllable legends.
- Water-level reference thresholds and reference-time markers with readable
  localized labels and units: mm, m³/s and cm.
- Actual local timestamps and backend timezone semantics, including irregular
  samples, masked values, valid zero/signed values and unknown offsets. Rendering
  must not interpolate or connect missing intervals into supplied measurements.
- Existing chart-start control and high-resolution PNG export, including the
  legacy 1×/2×/3× choices. Export a complete readable chart, not merely a screenshot
  of its data region. Verify supported resolution, fonts and labels explicitly.

Test rsplot's bar/interval representation, legend behavior, missing-data breaks,
linked hover and export before writing a larger chart adapter. Document gaps
against the pinned revision. Implement only the small monitoring-specific pieces
that remain; do not introduce new scientific fitting or correction tools.

Alternatives considered: egui_plot offers useful interactions but insufficient
confidence in the required presentation/export; ruviz has a rich but image-backed
egui adapter; Plotters/egui-plotter provide drawing flexibility but require more
application interaction code; Plotly/ECharts require JavaScript for embedded
interactive rendering. None replaces the selected rsplot in this plan.

## Ownership, concurrency and resource lifetime

Keep the [architecture](../../docs/architecture.md) and
[workspace](../../docs/contracts/workspace.md),
[spatial](../../docs/contracts/spatial.md) and
[series](../../docs/contracts/series.md) contracts unchanged unless a specific
missing backend capability is established.

Use an optional desktop interface target with GUI dependencies excluded from
ordinary library/CLI builds. Follow the existing single-package architecture;
do not create a new workspace or owner hierarchy in advance. Interfaces call
intended public APIs. Adapt the architecture gate only where an implemented
interface genuinely needs support, retaining its ownership checks.

Keep file/SQLite/NetCDF reads, accumulation, expensive geometry preparation and
other blocking work outside the UI update path. Workers return data to UI-owned
state; share immutable geometry/series where useful instead of cloning them into
each pane. Coalesce requests and discard obsolete results when selections or
sources change quickly. Do not let older worker results overwrite newer state.

Keep GPU device/resource ownership with the selected integrations. Retain static
geometry and reuse textures; bound tile, decoded-data, series, accumulation and
GPU caches. Release replaced project/scenario resources and stop workers on
close. Cache keys include relevant source versions, selected interval and
run/scenario identity. Selection-only changes must not rebuild static layers;
opacity-only changes must not reread data or regenerate rainfall images.

Use straightforward state and message passing. No generic task framework,
renderer plugin interface or speculative background service is required.

## Delivery sequence

All steps below are planned. This change adds documentation only.

1. **Qualify the dependency/rendering combination.** Select compatible rsplot and
   Galileo revisions, egui/eframe and wgpu; demonstrate one window with map and
   chart using the intended GPU integration. Assess basemap compatibility and
   resolve cache configuration, precision and release gaps. Record revisions,
   necessary patches and unresolved platform limitations. Reconsider walkers
   only if the Galileo gates fail.
2. **Resolve narrowly identified backend gaps.** Check polygon ring/part identity
   and any actual missing rainfall/selection query. Inspect all affected callers
   before changing shared APIs, preserve CLI behavior, and qualify with synthetic
   fixtures. Do not move backend semantics into GUI code to bypass a gap.
3. **Build project opening and asynchronous data loading.** Read configuration
   under existing path rules, associate the workspace, discover available
   artifacts, report absent/invalid resources and expose loading/error status.
   Opening must not acquire data, prepare inputs, migrate stores or launch MGB.
4. **Implement the operational map.** Add static layers, stations/labels,
   accumulated rainfall, legends, controls, exact inspection and consistent
   selection. Preserve camera state and independent layer invalidation.
5. **Implement linked summaries, tables and rsplot charts.** Exercise scenario
   overlays and coordinate-less stations; verify values against shared queries,
   missing data and reference levels; complete PNG export.
6. **Complete refresh, preferences and usability.** Restore view settings,
   preserve valid selections, exercise rapid changes and project switching,
   localize labels, and check keyboard operation and accessible status feedback.
7. **Qualify monitoring parity.** Compare the same regional data and interactions
   against the Python tab, then record accepted evidence and remaining gaps.
   Update the migration checklist without claiming the broader desktop delivery.

Keep disposable probes, captures and their scripts/logs under gitignored `runs/`;
preserve legacy sources, operational data and frozen references. Do not commit
regional datasets, binaries or temporary qualification tools as product code.

## Acceptance and validation

### Functional checks

- [ ] A configured project opens and reopens with available data; absent caches,
  invalid engine prerequisites and failed scenarios do not hide unrelated data.
- [ ] Regional geometry retains correct mini/station identities. Polygon fixtures
  cover holes, multipart shapes and boundaries even though the inspected region
  has no holes.
- [ ] Rainfall texture alignment agrees with known grid cells at overview/close
  zoom; inspection returns exact backend values, including missing and zero.
- [ ] Equivalent signed-hour and explicit intervals return matching backend
  intervals, accumulations and coverage.
- [ ] Map, search/table, summary and charts agree on the selected location;
  stations lacking coordinates or offsets remain correctly represented.
- [ ] Same-run scenario overlays, observation/history/forecast segmentation,
  thresholds, timestamps, missing values and exported charts match requirements.
- [ ] Refresh and failed replacement retain the camera and valid display state
  with explicit source/error feedback; obsolete results cannot overwrite it.
- [ ] Visibility/order/opacity, local display layers, labels and basemap selection
  work without changing operational inputs.
- [ ] English/Portuguese labels, keyboard actions, normal-size layouts, resize,
  HiDPI and accessible status/error feedback are usable.

### Performance and memory checks

Agree explicit thresholds on representative operator hardware before acceptance;
no frame-rate, latency or memory target has yet been established. Compare release
builds with the legacy monitoring tab using the same dataset, visible layers,
viewport, scenarios and warmed/cold cache conditions. Separate tile-network delay
from local rendering and query time.

Measure loading/first usable view, geometry preparation/upload, pan/zoom frame
times including slow frames, station/river hover and click latency, rainfall
window/scenario switching, chart navigation/hover, and idle CPU. Specifically
measure chart interaction with a stationary map to establish whether Galileo
reuses the map texture as intended.

Track process memory and available GPU memory evidence through repeated
pan/zoom, source refresh, rainfall/scenario switches and project close/reopen.
Verify eviction and return to a bounded working set rather than demanding exact
baseline RSS after allocator reuse. Test configured cache policy and resource
release; entry counts alone do not prove byte bounds or absence of leaks.

Use the 8,649-feature regional layers at overview and close zoom, plus synthetic
geometry correctness fixtures. Confirm simplification does not create visible
selection mismatches or alter scientific values. Accept no performance claim
based solely on upstream demos, software rendering or a successful build.

### Platform and repository checks

Linux is first: qualify Wayland and X11 independently on operator hardware and
record the GPU/backend. Label virtual/software-rendering checks separately.
Windows remains a target and unverified until exercised. Confirm packaging/fonts,
file dialogs, DPI and input on each qualified platform; Linux evidence does not
establish Windows behavior.

For implementation, run the required Rust format, locked Clippy, test and
independent architecture checks, plus builds/tests enabling the desktop target
and checks that library/CLI builds do not require GUI dependencies. Add focused
tests for nontrivial state transitions, selection, stale worker results and
geometry/value correctness; use interactive checks for actual rendering/input.
This documentation-only change requires relative-link/content checks, not a
Rust test rerun.

## Upstream evidence to recheck at qualification

- Galileo: [egui integration](https://github.com/galileo-map/galileo/blob/main/galileo-egui/src/egui_map.rs),
  [feature layers and picking](https://github.com/galileo-map/galileo/blob/main/galileo/src/layer/feature_layer/mod.rs),
  [bundle invalidation](https://github.com/galileo-map/galileo/blob/main/galileo/src/layer/feature_layer/bundle_store.rs),
  [raster caches](https://github.com/galileo-map/galileo/blob/main/galileo/src/layer/raster_tile_layer/provider.rs),
  [cache issue](https://github.com/galileo-map/galileo/issues/359),
  [precision issue](https://github.com/galileo-map/galileo/issues/179),
  [release compatibility issue](https://github.com/galileo-map/galileo/issues/344).
- Walkers: [GeoJSON helper](https://github.com/podusowski/walkers/blob/main/walkers_extras/src/geojson.rs),
  [retained drawables](https://github.com/podusowski/walkers/blob/main/walkers/src/canvas.rs),
  [tile cache](https://github.com/podusowski/walkers/blob/main/walkers/src/io/tiles_io.rs),
  [styled cache](https://github.com/podusowski/walkers/blob/main/walkers/src/tiles/styling.rs),
  [GPU upload lifetime](https://github.com/podusowski/walkers/blob/main/walkers/src/render/gpu.rs),
  [clipping issue](https://github.com/podusowski/walkers/issues/516),
  [WASM performance report](https://github.com/podusowski/walkers/issues/583).
- Charts: [rsplot capabilities and status](https://github.com/physwkim/rsplot),
  [rsplot dependencies](https://github.com/physwkim/rsplot/blob/main/Cargo.toml).

After acceptance or abandonment, retire this plan under the repository's
documentation rules. Preserve accepted lasting decisions in architecture,
supported behavior in contracts, and necessary evidence with its reference;
do not promote temporary upstream workarounds into permanent requirements.
