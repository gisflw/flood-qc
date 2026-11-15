# Desktop GUI specification

Status: product decisions confirmed in chat on November 15, 2026; implementation
not started. This specifies the initial milestone, not current supported behavior.
The [task specifications](desktop_gui_tasks.md) define delivery and acceptance.
Rendering, dependency compatibility and platform behavior remain unqualified.

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
- Pause editing the shared working operation during execution; browsing
  maps/charts, comparisons, exports, and status remains usable.
- Preserve existing concurrent scenario execution and the workspace lock that
  rejects overlapping independent operations in one workspace. No GUI-only
  serialization of scenarios and no expansion of workspace concurrency.
- Cancel the whole operation, rather than individual scenarios. Trial execution
  produces disposable cache artifacts, not registered outputs or a durable run
  history entry. Cancellation/failure reports an error: retain successful scenario
  results and debugging logs, discard incomplete scenario outputs. Retain the
  draft and previously completed reusable preparation. Do not add a cancellation
  recovery workflow or automatic retries.
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
- Use OpenStreetMap Standard, with visible attribution, application-specific
  User-Agent, HTTPS and policy-compliant local caching. No bulk downloads or local
  tile packaging. Hydrological layers remain usable when tiles are unavailable.
  Follow the [OSM tile policy](https://operations.osmfoundation.org/policies/tiles/).

## Confirmed operator workflow

Decisions confirmed during specification review on November 15, 2026:

- Each application window selects one workspace. Switching workspace prompts to
  save or discard unsaved edits and is blocked during execution. Initial run
  comparisons stay within that workspace.
- Each window has one working operation with activity tabs sharing its draft and
  selections, following Python's shared-state activity model. Tabs do not own
  independent operation recipes. Place maps/charts beside relevant controls and
  retain a persistent workspace/draft indicator. The activity tabs are Monitoring
  & Results, Observation Review, Forecast Corrections, and Operation & Execution.
  Saved forecast normalization belongs in Forecast Corrections. Operation
  settings, scenarios, preparation and logs belong in Operation & Execution.
- Start from a new draft, saved draft JSON, an existing operation TOML, or a copy
  of a prepared/completed run or successful trial-cache recipe. Copying retains explicit dates and
  selections until the operator changes them.
- Run prepares the current draft and then executes its frozen preparation.
  Prepare only is a separate action. Every run identifies its frozen preparation.
- Successive trials remain separate disposable cache entries selectable for
  comparison, without automatic cache cleanup in this milestone. Manual
  copying/export is sufficient for preservation; do not add a Save run action.
  Editing a draft marks its previous preparation outdated. Run prepares again;
  an explicit Execute existing preparation action reruns frozen inputs.
- Replacing or closing an idle modified working operation offers Save, Discard,
  or Keep open. Switching activity tabs preserves edits without a save prompt.
  Window closure during execution offers Cancel execution and close or Keep open;
  closure waits for cancellation and cleanup to complete. Execution does not
  continue in the background after closure.
- Freeze edits to the shared working operation during execution. Activity tabs
  remain available for browsing, comparisons, exports, and status/log inspection.
  No second preparation or execution starts from that window while it executes.
  Apply the same busy protection during preparation. No background work continues
  after normal window closure; automatic crash recovery remains deferred.
- Editing a registered instruction creates a draft copy. Registration creates a
  new catalog record; removing an instruction from the draft only deselects it.
  Existing registered records remain intact.
- Registration acts on explicitly selected draft instructions, committing the
  selected batch atomically within each catalog. Report outcomes independently
  across observation and forecast catalogs. Retrying after partial success must
  not duplicate successfully registered records.
- Operator name and reason are optional for every action, including preview,
  trial preparation/execution and registration. Saving incomplete edits is allowed.
  Remove current nonempty-reason requirements from shared store validation and
  update affected contracts/tests; do not impose them in an interface instead.
- New drafts use available workspace settings, leaving missing settings visibly
  incomplete. Forecasts and scenarios require explicit selection; do not select
  a latest forecast automatically.
- Reopening drafts reports missing or changed selected sources. Changed data
  requires explicit Refresh inputs acceptance before preview/preparation. Refresh
  invalidates derived previews/preparations; existing frozen preparations remain
  independently executable. Missing data cannot be repaired by silent substitution.
- Discard changes restores the last saved draft or, for a never-saved draft, its
  initial state. It does not reverse registration or remove completed runs.
- Draft saves include optional visualization state by default: map position,
  selected location, visible runs/scenarios and chart range. Offer omission of that
  state. View-only changes do not trigger unsaved-operation warnings.
- Cancel also covers preparation, including Run's preparation stage. Discard that
  attempt's temporary outputs without changing the draft or an already completed
  preparation.
- Runs are inexpensive and operators must be able to perform repeated trials
  without registering outputs. Do not require run-history registration, a keep
  decision, or a recovery/cleanup interface merely to execute. On cancellation or
  failure retain only successful scenario results and the debugging log from the
  attempt; discard incomplete scenario outputs and temporary working files.
  Separate cache entries support cross-run comparison; operators manually
  copy/export anything they want to preserve.

## Confirmed visualization behavior

- Select one location through a map click or searchable station/mini-basin
  selector; share the selection across activity tabs. A station uses its linked
  mini-basin where available. Show missing links explicitly rather than guessing.
- Offer explicit rainfall start/end times and relative shortcuts such as 24 hours
  before/after the operation reference time. Display the resolved interval and use
  canonical hourly rainfall interval semantics.
- Area summaries support either the selected mini-catchment or its full upstream
  drainage area, with the chosen boundary highlighted.
- Include the selected mini in its upstream area. Rainfall summaries use an
  area-weighted mean of mini-catchment accumulations, not a sum of depths. Show
  missing temporal/spatial coverage explicitly; never equate missing rain with
  zero or label a partial total/mean complete. No partial-interval prorating.
- Display supplied attention, alert, flood and severe thresholds and historical
  flood marks. These are distinct from fixed model station offsets. Missing
  references omit the corresponding lines.
- Read the configured GeoPackage's `mini_segments` and `mini_catchments` layers;
  use `MINI.gtp` model topology for upstream selection. Report missing/invalid
  layers without blocking unrelated activities.
- Import regional threshold/flood CSV formats into Rust-owned storage through
  shared API/CLI and a GUI import action. Editing reference records is deferred.
- Validate both CSVs before atomically replacing imported threshold/flood records.
  Report unmatched station codes as skipped. Invalid input or a failed commit
  preserves existing reference records and observations.
- Stack rainfall, level, then flow plots with a shared time axis, zoom range and
  cursor, observation/scenario visibility controls and each run's reference time.
- Compare runs at actual timestamps in the operation timezone, without filling
  gaps, interpolation or resampling. Label different forecast reference times.
  Incompatible locations or level references produce findings rather than
  misleading overlays.
- Export a displayed map or chart group to PNG with legends, units, location,
  time interval and required map attribution. Offer 1x/2x/3x resolution, default 1x.
- Export selected series over the visible chart interval as UTF-8 CSV, one row per
  timestamp/series, including location, run/scenario, forecast reference time,
  timezone, variable, unit and value. Missing values are empty fields. Numeric
  formatting is independent of interface language.
- Start in English and allow Brazilian Portuguese switching for the application
  session. Translate application labels/messages; preserve user-authored text,
  identifiers and external diagnostics.
- Use the RS dataset on Linux at 1366x768 with ordinary display scaling as the
  representative layout/responsiveness acceptance environment. Record the actual
  hardware/software used; these display choices do not establish a CPU/RAM floor.

## Shared API and draft design

### Confirmed correction interaction

- Observation Review includes configurable QC scans, point/period replacements,
  missing-value exclusions, flagged-sequence replacement and station exclusion
  for a selected period. Display raw and effective values together. Exclusion
  uses existing replacement instructions, not a new station-exclusion entity.
- Defer leave-one-out diagnostics to broader analysis.
- Forecast Corrections uses existing ordered translation, rotation and positive
  multiplication instructions with explicit lead-time windows. Original and
  corrected maps appear side by side with synchronized navigation and matching
  color scales.
- Apply preview explicitly performs expensive recalculation. Edits mark the
  displayed preview outdated. Preparation uses the current validated draft,
  independently of whether that draft has been previewed.

Use typed Rust draft values, not JSON manipulation as the operational API. JSON
is the serialization boundary. Reuse existing observation/correction types and
validation/calculation functions wherever their semantics fit.

The draft holds a schema version, workspace/source references, operation settings,
scenario definitions, ordered instruction values, and optional reasons/authorship.
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

## Boundary contracts for implementation

### Draft JSON and action readiness

Use a version-1 JSON document with `schema_version`, `workspace`, `sources`,
`operation`, `instructions`, and optional `visualization` sections. `operation`
covers all existing operation-request settings, with incomplete fields explicitly
representable. `instructions` holds actual values, stable draft identities,
ordering and optional registered origins; `sources` records accepted identities
of selected content. Do not fingerprint the entire SQLite file to identify a
selected immutable record: unrelated catalog additions must not invalidate it.

Reject unsupported versions and unknown operational fields. Invalid optional view
state may be omitted with a finding without changing the recipe. A saved workspace
reference never silently selects or replaces the explicitly chosen workspace.
Resolve relative source paths against that workspace, not the draft file's parent.
Resolve CLI workspace/draft/output paths from the invocation directory, retaining
existing external-path support and native filesystem semantics.

Saving an incomplete draft does not require available sources. Use a
same-directory temporary file and atomic replacement; failure preserves an
existing destination. Save As updates the saved baseline only after success.
Missing view-state locations/cache entries produce a finding and a usable view.
Recheck relevant source identities at action boundaries and freeze consistent
selected values during preparation. An old worker result must not replace a
newer draft's preview. Unrelated incomplete fields need not block a local preview.

Optional reason text may use an empty string in existing non-null storage columns.
Do not require a schema migration just to allow blank reasons or invent a
mandatory observation-author field. Preserve supplied audit text.

### Registration identity and outcomes

Register an explicitly selected batch, returning stable draft-to-catalog ID
mappings and independent catalog outcomes. Identical previously written values
return the same mapping on retry; conflicting values under an existing identity
fail. Changing an instruction requires a new registration identity. Preserve
effective values/order after registration and never select an instruction twice
merely because it has just been registered. No global cross-catalog transaction.

### Trial artifacts and progress

Use a separate cache entry per attempt with stable attempt/scenario identities,
preparation identity, resolved recipe, reference time, units/reference metadata
and validated successful results, including canonical outputs and original engine
streams for successful scenarios. Cache metadata supports discovery and recipe
copying, not a permanent workspace run registry. A draft recipe alone is not an
archive; copying/regeneration still requires available dependencies and engine.

The new shared trial lifecycle deliberately changes current execution retention.
Preserve existing saved runs as readable/copyable sources; do not migrate, delete
or rewrite them. Keep CLI command syntax and existing prepared-input compatibility.
Current manifests may supply cache metadata where their semantics fit; do not
build a parallel GUI history database or a new archive framework.

Show structured stage/scenario state and live stdout/stderr with bounded reads.
Use stage progress or indeterminate activity if the engine provides no reliable
fraction; do not invent precise percentages/ETAs. Cancellation and failure are
error outcomes with successful sibling caches and debugging logs preserved.
Once a completed result has committed, a late cancel does not erase it. OS cleanup
errors are ordinary reported errors; no recovery wizard or cleanup-retry workflow.
Do not report child termination or deletion that did not occur.

### Reference and geometry formats

Use Python's `src/mgb_ops/assets/databases.py::load_station_reference_levels` as
the regional CSV reference: threshold columns include `station_code`,
`station_name`, `level_atention`, `level_alert`, `level_flood`, `level_severe`;
historical events use `station_code`, `level_registered`, and ISO `date`.
Support UTF-8 with optional BOM and comma-delimited quoting. Values are supplied
centimetres; preserve finite source precision. Blank thresholds omit lines;
invalid numbers/dates and duplicate reference keys fail. Resolve provider/station
codes explicitly and report skipped unmatched codes. Import replaces only
threshold/flood records, without changing observation samples, offsets or stations.
Existing canonical observation databases require a tested, explicit transactional
upgrade for reference tables; read-only opens never perform migration.

Validate mini IDs, CRS and geometry, and transform projected coordinates correctly
for the map. Read MINI topology/areas through the existing ownership boundaries;
validate missing references, cycles and correspondence with map IDs. Multiple
geometry parts for one mini may be valid and must not double-count its area.
Missing layers disable their dependent views, not the application. Missing station
coordinates still allow selection through search; missing mini links/offsets are
not inferred from nearest geometry.

### Chart and export values

At a station use its supplied mini association for modeled rain/flow and its
referenced station level; at a mini-basin explicitly label raw model level. Keep
history/forecast segments and reference times identifiable. Do not apply AR(1).
All rainfall/time controls display actual bounds; reject unsupported non-hourly
inputs without silently rounding. Do not interpolate gaps or introduce new
resampling/prorating to make comparisons look complete.

CSV column order is `timestamp`, `time_zone`, `location_kind`, `location_id`,
`series_kind`, `run_id`, `scenario_id`, `forecast_reference_time`, `variable`,
`unit`, `value`. Observations leave inapplicable run/scenario/reference fields
empty. Use proper CSV quoting, comma delimiters and decimal points, with stable
ordering by timestamp then series selection order. Preserve source precision,
zero and negative levels independently of display rounding. Instantaneous series
include both visible endpoints; rainfall retains canonical interval-end selection
semantics. PNG resolution changes preserve the displayed geographic/time extent;
include attribution and identify an outdated preview. Do not fetch a wider tile
area or conceal unavailable tiles for an export. Protect existing destinations
on export failure and confirm overwrites in the GUI.

### Public workflows and launch

Provide typed public actions with thin CLI coverage for draft load/save/import/
edit/validate, Refresh inputs, previews, selected registration, draft preparation/
Run, existing-preparation execution, reference import and series CSV export.
CLI editing may replace a validated draft document; no field-patching language is
required. CLI JSON mode returns structured results/findings without progress text
on stdout, following existing exit-code conventions. Native PNG capture is a
desktop action using shared content/query values; no headless screenshot service.

Use an optional `desktop` Cargo feature and `mgb-ops-desktop` binary in the same
package, with the existing CLI remaining the default run target. First delivery
is a Cargo-built Linux executable plus launch instructions. Installers, automatic
updates and Windows qualification are deferred. Retain keyboard navigation,
visible focus, named controls, searchable alternatives to map selection and
status distinctions that do not rely only on color.

## Ownership and desktop integration

Follow existing owners; do not introduce a separate operational framework:

- `app`: draft/workspace resolution and workflow coordination, registration,
  preparation, execution progress/cancellation and outcomes.
- `store`: draft serialization, canonical persistence, geometry/reference-data
  loading, run artifacts and file lifecycle.
- `adapters`: reference CSV parsing/normalization without deciding persistence.
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
workers before deleting incomplete artifacts or releasing the workspace lock.
Report errors without a dedicated recovery interface. Preserve successful scenario
cache results, debugging logs, other runs, source catalogs, saved drafts, and
completed reusable prepared trees. Add CLI interruption support using the same
behavior, without requiring a persistent run-history entry. Align existing
execution contracts with the agreed cache/failure behavior; do not silently impose
the old durable-run retention policy on GUI trials.

## Delivery sequence

Follow [G01–G10](desktop_gui_tasks.md#delivery-order). Qualify rendering first;
refactor typed preparation, drafts, registration and trial control in shared core;
add data readers/analysis, connect the operational GUI, complete visualization,
and accept shared GUI/CLI/API behavior. Task goals, no-goals, dependencies,
affected surfaces and acceptance criteria have one home in that file.

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
  scenario execution, and finalization; no surviving processes, durable trial
  history entry, or damage to other runs/prepared inputs. Keep successful scenario
  cache results and debugging logs on cancellation/failure; discard incomplete
  scenario outputs.
- Correct geographic alignment, accumulation intervals, units, missing values,
  station reference levels, and identification/alignment of cross-run series.
- Responsive interaction, tile-unavailable operation, keyboard accessibility,
  bilingual labels/messages, view restoration, and valid PNG/CSV exports.

Run the repository's required Rust formatting, Clippy, full tests, and independent
architecture gate for implementation changes. Use focused behavioral tests and
representative manual desktop acceptance rather than mirroring widget code.

Retain Linux-first qualification and Windows as a target. This plan does not add
Windows acceptance to the initial milestone; record it as unverified until checked.

## Deferred scope and qualification gates

Deferred: AI chat, command palette/prompt, workspace creation/configuration,
acquisition, new hot starts, broader post-processing, calibration/geometry editors,
bulletins/reports, scheduling/email, completed-run sharing, multi-location charts,
undo history, automatic crash recovery, local basemap tile packaging,
leave-one-out diagnostics, AR(1), reference-record editing, Save run/archive
management, automatic cache cleanup, installers, and automatic updates.

Still to qualify through the [delivery tasks](desktop_gui_tasks.md): concrete
dependency versions/backend, geometry/CRS coverage, rendering performance,
cancellation/process control, reference import upgrades, and native image export.
Keep private helper/type/widget details in implementation; any required change
to specified public behavior must be brought back for a decision. Inspection of
[walkers overlays/maps](https://docs.rs/walkers/latest/walkers/struct.Map.html)
and [linked chart axes/cursors](https://docs.rs/egui_plot/latest/egui_plot/struct.Plot.html)
supports a spike, not a completed feasibility qualification.
