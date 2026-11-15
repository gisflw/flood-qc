# Initial desktop GUI draft plan

Status: draft for review. Product choices below were agreed during the planning
interview; implementation details and delivery sequence remain proposals.
No GUI implementation or platform qualification is implied.

## Outcome and references

Introduce a Rust desktop interface over the existing saved-data operational path.
Operators open an existing configured regional workspace, inspect inputs, edit
draft instructions/settings, preview changes, prepare and execute scenarios, and
compare/export results. GUI and CLI use the same public library behavior.

Use Python as the data-visualization reference and VB.NET as the functionality
reference. Do not reproduce their layouts, window structure, click behavior, or
color schemes. References:

- [Python source map](../../references/python_map.md), especially dashboard
  monitoring, charts, forecast preview, observation review, and correction views.
- [VB.NET source map](../../references/vbnet_map.md), especially execution,
  post-processing, exports, and the broader operational functionality inventory.
- [Architecture](../../docs/architecture.md) and existing
  [operation contract](../../docs/contracts/operations.md).

## Agreed first-milestone behavior

- Rust with egui; walkers supplies map view/navigation. Retain English and
  Brazilian Portuguese. Keep operational data independent of interface language.
- Open existing configured workspaces; show inspection findings. Workspace
  creation and configuration editing are deferred.
- Forms and buttons invoke public Rust APIs directly. No shell-command wrapper,
  embedded command prompt, or required command syntax.
- Support saved forecast normalization, operation/scenario settings, preparation,
  execution, status/log inspection, and result inspection.
- Edit observation replacements/exclusions and forecast correction instructions.
  Edit/remove individual instructions or discard a whole draft. No step-by-step
  undo/redo. Station metadata and direct rainfall-cell/sample editors are deferred.
- Preview unregistered edits and run trials from them. Registration is a separate
  explicit action; running a trial must not register its edits in source catalogs.
- Save/reopen drafts as versioned JSON, shared by GUI, CLI, and agent callers.
  Include optional visualization state separately from the operational recipe.
- Pause editing during execution; browsing maps/charts and status remains usable.
- Preserve existing concurrent scenario execution and the workspace lock that
  rejects overlapping independent operations in one workspace. No GUI-only
  serialization of scenarios and no expansion of workspace concurrency.
- Cancel the whole operation, rather than individual scenarios. Do not retain a
  cancelled execution in run history. The working assumption is to discard that
  execution's generated outputs while retaining its draft and reusable prepared
  inputs; ordinary failure diagnostics retain their existing behavior.
- Export displayed charts/maps as PNG and selected series as CSV.

Map/chart scope:

- Stations, river network, catchments, observed/forecast rainfall, and corrected
  forecast previews; selected-interval rainfall accumulation and area summaries.
- One selected station or basin at a time, with rainfall, flow, and level charts.
- Compare observations with scenarios across multiple runs, including trial runs.
  Identify each series by run, scenario, and forecast reference time. Preserve
  units, operation timezone, missing values, and station-level reference semantics.
- Show station reference levels where available. Import/read support for those
  records is part of the visualization work, not assumed to exist already.
- Save selected location, visible scenarios/runs, map position, and chart time
  range as optional visualization state in the draft.
- Initial interpretation of the agreed basemap scope: online basemap, with local
  hydrological layers usable when tiles are unavailable. Local tile packaging is
  deferred. Choose a provider and honor its attribution/use requirements during
  implementation.

## Shared API and draft design

Use typed Rust draft values, not JSON manipulation as the operational API. JSON
is the serialization boundary. Reuse existing observation/correction types and
validation/calculation functions wherever their semantics fit.

The draft holds a schema version, workspace/source references, operation settings,
scenario definitions, ordered instruction values, and required reasons/authorship.
Large observations, forecast grids, geometry, and results remain in their existing
formats. A saved draft is a recipe with edits, not a self-contained data archive.
Define workspace-relative path resolution consistently with existing requests.
Missing/replaced sources must produce actionable findings, not silent substitution.

Separate editing validation from readiness for preview, registration, preparation,
and execution: a draft may be incomplete while being edited. Validate all trust
boundaries, unsupported schema versions, source identities, instruction ordering,
units, bounds, and missing-value semantics before the relevant action. Save drafts
without truncating a previously valid file if saving fails.

Loading, editing, validating, previewing, saving, preparing from a draft, and
registering selected draft instructions must be available through intended public
APIs and thin CLI entry points. CLI users need not supply visualization state.
Define deterministic registration results and surface partial failure honestly;
do not imply one transaction across independent catalogs.

Refactor the existing file-based preparation entry into a thin adapter over a
typed preparation workflow. Preserve existing TOML requests and callers. Both
registered selections and supplied draft instruction values feed the same
scientific processing and retained-input lifecycle. Freeze actual selected values
and dependencies before execution; record draft edits in the run recipe without
requiring their registration in the original catalogs.

## Ownership and desktop integration

Follow existing owners; do not introduce a separate operational framework:

- `app`: draft/workspace resolution and workflow coordination, registration,
  preparation, execution progress/cancellation and outcomes.
- `store`: draft serialization, canonical persistence, geometry/reference-data
  loading, run artifacts and file lifecycle.
- `analysis`: effective-data previews, corrections, accumulation, selected-area
  and series comparisons, export content and reusable rendering where applicable.
- `model`: engine execution/process control, including stopping owned processes.
- Desktop: egui widgets, walkers overlays, interactive selection/view state,
  language, user action dispatch, and presentation caches.

Add a desktop interface under the existing interface ownership rules, with GUI
dependencies optional so ordinary library/CLI builds remain usable without them.
Prefer eframe as the egui host and egui_plot for charts, subject to compatible
dependency versions and a representative-data spike. Use background workers and
structured progress/results so rendering does not block on I/O or model work.
Do not add a daemon, IPC layer, or GUI-specific copy of scientific processing.

Expose named interface actions for selection, time range, visible runs/scenarios,
and opening workflows. Future chat can inspect state and invoke the same actions;
operational edits/execution use the public core APIs. This is a boundary to retain,
not a requirement to build an agent tool registry or AI runtime now.

Cancellation must cover preparation of execution workers, the history stage,
concurrent scenario engines, and finalization. Stop/reap owned processes and join
workers before deleting this execution's artifacts or releasing its workspace
lock. Report cancellation completion or cleanup failure accurately. Preserve
other runs, source catalogs, saved drafts, and reusable prepared trees. Add CLI
interruption support using the same cancellation behavior; the API returns a
cancelled outcome without requiring a persistent history entry.

## Proposed delivery sequence

1. **Validate the rendering path.** Build a small egui/eframe/walkers/egui_plot
   slice using representative regional geometry, a rainfall frame, and series.
   Check coordinate alignment, missing data, one-location selection, responsiveness,
   PNG capture including required map attribution, and operation without tiles.
   Qualify dependency versions before extending the interface.
2. **Implement shared drafts.** Typed drafts, JSON round trips, instruction editing,
   validation, effective previews, explicit registration, and thin CLI coverage.
   Refactor typed preparation and prove unregistered trial preparation uses the
   same science and snapshots as registered equivalents.
3. **Implement shared execution control.** Structured status/log access and whole
   operation cancellation, retaining scenario parallelism and workspace locking.
   Complete process/worker cleanup and cancellation outcome checks before wiring
   desktop controls.
4. **Connect the operational GUI.** Workspace inspection, source selection and
   saved normalization, forms, draft save/reopen/discard, previews, registration,
   prepare/execute/cancel, status/logs, and clear validation/failure feedback.
5. **Complete visualization and exports.** Geometry/reference-data APIs, map
   overlays and accumulation, area summaries, one-location charts, cross-run
   comparisons, optional view-state restoration, PNG/CSV, and both languages.
6. **Accept the workflow.** Exercise a complete saved-data case through GUI and
   CLI using the same draft and inputs; compare scientific artifacts and exports.
   Keep the existing saved operational path working throughout delivery.

## Acceptance and affected surfaces

Implementation reaches app preparation/request/execution APIs, store formats and
readers, analysis previews/queries/exports, model process control, root public
exports, CLI callers, the new desktop interface, Cargo feature/dependency config,
architecture enforcement, tests/fixtures, contracts, and migration tracking.

Meaningful checks include:

- Draft round trips, ordered instruction edits/removal/discard, incomplete drafts,
  invalid input, missing sources, failed saves, and optional view-state handling.
- GUI/CLI/public-API parity for draft previews and trial preparation; originals
  remain unchanged until registration, and snapshots retain actual applied values.
- Existing scenario concurrency/lock behavior; cancellation during history,
  scenario execution, and finalization; no surviving processes, cancelled history
  entry, or damage to other runs/prepared inputs. Keep ordinary failure coverage.
- Correct geographic alignment, accumulation intervals, units, missing values,
  station reference levels, and identification/alignment of cross-run series.
- Responsive interaction, tile-unavailable operation, keyboard accessibility,
  bilingual labels/messages, view restoration, and valid PNG/CSV exports.

Run the repository's required Rust formatting, Clippy, full tests, and independent
architecture gate for implementation changes. Use focused behavioral tests and
representative manual desktop acceptance rather than mirroring widget code.

Retain Linux-first qualification and Windows as a target. This plan does not add
Windows acceptance to the initial milestone; record it as unverified until checked.

## Deferred and unresolved implementation details

Deferred: AI chat, command palette/prompt, workspace creation/configuration,
acquisition, new hot starts, broader post-processing, calibration/geometry editors,
bulletins/reports, scheduling/email, completed-run sharing, multi-location charts,
undo history, automatic crash recovery, and local basemap tile packaging.

Still to validate: dependency compatibility and rendering performance; geometry
and reference-level source coverage; exact API/schema/CLI shapes; cancellation
behavior on each platform; and native image export. Documentation inspection of
[walkers overlays/maps](https://docs.rs/walkers/latest/walkers/struct.Map.html)
and [linked chart axes/cursors](https://docs.rs/egui_plot/latest/egui_plot/struct.Plot.html)
supports a spike, not a completed feasibility qualification.
