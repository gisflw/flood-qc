# Desktop GUI implementation tasks

Status: desktop work is planned. Typed preparation, drafts, registration,
controlled trials, geometry, reference imports, summaries and CSV queries are
available as headless capabilities. The [GUI specification](desktop_gui.md)
records product behavior; this file defines work boundaries and acceptance.

## Delivery order

| ID | Deliverable | Depends on |
| --- | --- | --- |
| G01 | Qualify native rendering, map overlays and PNG path | None |
| G02 | Typed preparation with preserved TOML behavior | None |
| G03 | Shared drafts, source acceptance and previews | G02 |
| G04 | Optional audit fields and explicit registration | G03 |
| G05 | Shared disposable trials, status and cancellation | G02, G03 |
| G06 | Geometry/topology and reference-level import/read APIs | None |
| G07 | Shared visualization queries, area summaries and CSV | G03, G06 |
| G08 | Working operational desktop with activity tabs | G01, G03, G04, G05 |
| G09 | Complete maps/charts, PNG and reference import UI | G06, G07, G08 |
| G10 | End-to-end acceptance and current documentation | G01–G09 |

G02–G07 headless capabilities are implemented; GUI expansion waits for G01.
Dependencies describe deliverables, not a requirement to create all proposed
modules upfront. G09 connects the G06/G07 APIs to interface controls. G08 is a usable
operational increment, not a placeholder shell waiting for every visualization.

## Common definition of done

- Preserve scientific arithmetic/units/time/reference semantics and the external
  engine. No duplicated GUI scientific or lifecycle rules.
- Reach changed callers, public exports, CLI paths, Cargo/feature configuration,
  architecture enforcement, fixtures and documentation. Keep current behavior
  distinct from planned behavior in contracts and migration tracking.
- Add focused behavioral checks for nontrivial logic and actual trust-boundary
  failures; avoid tests that only mirror widget construction.
- Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
  `cargo test --locked`, and `cargo test --locked --test architecture` for Rust
  changes. When desktop exists, also run Clippy/tests/architecture with
  `--features desktop` and build the desktop binary. Ordinary builds remain
  headless and usable without optional GUI dependencies.
- Use synthetic fixtures in routine tests. Representative RS/manual engine and
  graphics acceptance is explicit and separate. Disposable experiments belong
  under gitignored `runs/`; preserve legacy/runtime/frozen sources.
- Update affected contracts when behavior lands, migration evidence/gaps as they
  change, and this task's acceptance state without writing routine completion
  reports. Record Linux evidence; never infer Windows qualification.

## G01 — Native rendering and export qualification

**Goal:** Prove the selected egui/eframe/walkers/egui_plot stack can render the
actual kinds of regional geometry, rainfall and series required by this milestone.

**No-goals:** Full application, draft persistence, execution, new GIS framework,
Windows acceptance, installers or offline tile packaging.

**Scope:** Feature-gated native binary, compatible locked dependency versions,
rendering backend, representative geometry/CRS and rainfall overlays, linked
charts, OSM identification/caching/attribution and native PNG capture. Try the
existing available reader/dependency facilities before adding a GIS dependency;
add a dependency only for a concrete unsupported requirement. Temporary capture
code goes under `runs/`; retain only useful implemented interface code.

**Acceptance criteria:**

1. Default CLI/library build remains usable without a display; explicit desktop
   feature builds and launches a native window. Record the qualified versions,
   backend and build prerequisites rather than assuming latest versions match.
2. Plot known regional station/river/catchment coordinates and a rainfall frame;
   verify alignment for geographic and representative projected geometry. Show
   zero/missing rainfall distinctly and select a single location.
3. Demonstrate linked time axes/cursors and side-by-side maps with synchronized
   navigation. Labels and essential controls remain reachable at 1366x768.
4. Capture valid PNG at 1x/2x/3x, preserving extent, overlays, text and attribution;
   verify exported pixels, not merely file creation. No wider-area tile prefetch.
5. With tiles unavailable, local layers/selection/charts remain usable. Inspect
   tile requests/cache behavior against the OSM policy; routine tests use fake
   tile responses, not bulk requests to the public service.
6. On the RS dataset record actual hardware/OS/backend, dataset size and observed
   pan/zoom/selection/export behavior. If mandatory behavior fails, report the
   specific blocker and revise the plan before broad GUI implementation.

**Validation:** Focused coordinate/mask checks plus representative manual graphics
and export inspection. Documentation is feasibility evidence, not acceptance.

## G02 — Typed preparation without scientific changes

**Goal:** Let TOML callers and future typed drafts prepare through one workflow.

**No-goals:** JSON drafts, interface widgets, new forcing algorithms, new hot
starts or changed numerical behavior.

**Scope:** `app/request.rs`, `app/prepare.rs`, preparation request/callers, intended
public exports and existing CLI/fixtures. Separate parsing, readiness validation,
resolution and orchestration only as needed to accept explicit instruction values.

**Acceptance criteria:**

1. Existing `operation prepare --request ...` TOML syntax, path rules and error
   behavior remain supported through a thin adapter to typed preparation.
2. Typed inputs can supply ordered observation/forecast instruction values
   without first registering them. Existing ID selections resolve to the same
   typed workflow; duplicates, unknown IDs and cross-asset corrections still fail.
3. Equivalent registered/supplied instructions produce identical scientific
   arrays, selected values and frozen dependencies. Preserve actual instructions,
   settings, state, offsets and engine identity, not mutable selectors alone.
4. Failed validation does not launch MGB, register edits or modify sources.
   Frozen preparation remains executable after original sources change/disappear.

**Validation:** Existing saved preparation/regression checks and one direct parity
case using overlapping ordered edits, missing values and a corrected forecast.
Preserve public callers while refactoring, rather than adding parallel science.

## G03 — Draft lifecycle, accepted sources and previews

**Goal:** Provide the shared, incomplete-editable recipe used by GUI/CLI/callers.

**No-goals:** Embedded datasets, autosave/crash recovery, undo history, a generic
patch language or registration/execution hidden inside preview.

**Scope:** Typed draft and versioned JSON in app/store, action-specific findings,
load/save/import/edit/validate/Refresh inputs/preview APIs, public exports and
thin CLI actions. Include new/JSON/TOML/prepared/retained-run/cache-recipe origins.

**Acceptance criteria:**

1. Round trips retain incomplete fields, scenario/instruction order, optional
   audit text and optional view state. Unknown operational fields/versions fail;
   invalid view state can be omitted with a finding. No implicit source selection.
2. New drafts expose missing settings; importing/copying preserves explicit dates
   and selections. Relative paths follow the specification. Missing source files
   do not prevent saving an incomplete recipe.
3. Changed selected content blocks dependent new preview/preparation until
   Refresh inputs; missing content is actionable. Unrelated catalog additions do
   not invalidate selected immutable content. Frozen preparations still execute.
4. Add/edit/remove/reorder and discard preserve last-match-wins processing;
   discard restores the saved/initial baseline without reversing catalog writes.
5. Failed save/Save As preserves prior file bytes and the working saved baseline.
   View-only changes do not trigger operational dirty warnings.
6. Explicit previews return effective/raw rows, QC findings and corrected frames
   using shared analysis; they register nothing. Edits invalidate the preview;
   stale background responses cannot become current. Preparation uses the current
   draft regardless of the displayed preview.

**Validation:** JSON/source/save-failure fixtures and API/CLI parity; compare
preview values against existing scientific functions. Do not test UI internals.

## G04 — Optional audit text and selective registration

**Goal:** Register selected edits explicitly while allowing anonymous, reason-free
trials and registrations.

**No-goals:** Rewriting/deleting registered instructions, mandatory identities,
cross-database transactions or a general transaction-recovery system.

**Scope:** Observation/forecast validation and writes, app registration workflow,
draft-to-catalog mappings, public/CLI actions, affected tests/contracts.

**Acceptance criteria:**

1. Empty/omitted names and reasons work for all draft actions and registration.
   Supplied text survives. Preserve validation of values/IDs/time/units; do not
   weaken scientific checks while removing audit-text requirements.
2. Editing a registered instruction creates a draft copy/new registration
   identity. Deselect/removal leaves the original catalog record/raw data intact.
3. Only selected instructions register; all selected writes within a catalog
   commit together. One invalid item rolls back that catalog's batch.
4. Partial success across catalogs reports exact mappings/outcomes. Retry returns
   existing mappings for identical successful writes, creates no duplicates and
   rejects conflicting payloads under an existing identity.
5. Effective instruction order/values and subsequent trial results are unchanged
   by registration. Preview/preparation/trial never register implicitly.

**Validation:** Blank-audit positive cases, invalid-value negative cases, rollback,
partial cross-catalog failure and repeated retry. Compare source bytes/selected
rows where preservation is promised.

## G05 — Disposable trial execution, progress and cancellation

**Goal:** Support repeated cheap trials with successful scenario caches and logs,
without registering runs or building a durable run-management subsystem.

**No-goals:** Save run/history database, automatic retention, recovery wizard,
automatic retries, per-scenario cancellation, detached jobs or expanded workspace
concurrency. Do not add a separate legacy/new execution-mode framework.

**Scope:** App execution/Run coordinator, store cache/file lifecycle, model process
control, structured progress/log API, CLI interruption, existing execution callers,
manifests/result discovery and lifecycle tests. Reuse existing layouts/helpers
where they satisfy cache semantics; exact private cache filenames are not a new
public contract. Preserve old saved artifacts without migration/deletion.

**Acceptance criteria:**

1. Run prepares current draft then executes; Prepare only stops before MGB.
   Execute existing preparation creates a fresh attempt from verified frozen
   inputs, including when the draft is now different or sources are unavailable.
2. Distinct attempts expose successful scenario results (canonical outputs and
   original engine streams), recipe and preparation identity for comparisons
   without catalog registration or permanent history. No keep
   prompt, Save run action or automatic cache cleanup.
3. Concurrent scenarios still run concurrently. The workspace OS lock rejects
   overlapping independent execution and remains held until children/workers stop
   and file work finishes. Source data and unrelated attempts remain untouched.
4. Progress identifies preparation/history/scenario/finalization; logs can be
   tailed with bounded reads while work proceeds. Unknown progress is displayed
   honestly. JSON stdout remains a structured result, separate from progress.
5. Cancel during preparation/history/scenario/finalization stops/reaps owned
   processes and joins workers. No new reusable partial preparation; preserve any
   earlier complete preparation. Normal closure cannot leave engines running.
6. Cancel/failure returns an error, retains only validated successful scenario
   cache artifacts and debugging logs, and discards incomplete scenario outputs
   and temporary working trees. Zero successes leaves logs, not a fake successful
   run. No cancelled-run history entry. A late cancel preserves committed success.
7. Nonzero exit, spawn failure, malformed output and persistence/cleanup errors
   remain visible and never claim false success. No dedicated recovery workflow.
   A manual rerun makes a separate attempt.

**Validation:** Synthetic sleeping/failing engine helpers test each cancellation
stage, successful siblings, no surviving children, lock release and preservation.
Keep real saved-case scientific acceptance separate from these process tests.

## G06 — Geometry/topology and station reference data

**Goal:** Supply canonical read/query values for local maps, upstream areas and
reference-level chart lines without relying on Python at runtime.

**No-goals:** Geometry/calibration/station editors, general GIS conversion suite,
nearest-station inference or reference-data editing.

**Scope:** Store geometry/topology/reference readers and schema evolution;
adapters parse regional CSVs, analysis computes upstream membership, app coordinates
imports. Reuse/extend existing MINI parsing without divergent scientific readers.
Expose intended APIs and reference-import CLI.

**Acceptance criteria:**

1. Read configured mini segments/catchments with validated IDs, geometry and CRS;
   align projected/geographic coordinates. Multi-part minis remain valid without
   duplicating weights. Fail dependent queries clearly on unsupported formats.
2. MINI topology/areas supply upstream membership including the outlet. Detect
   cycles, unknown references, ID mismatches and invalid/nonpositive areas. Do not
   approximate geodesic area using degree-space polygon area.
3. Return explicit station coordinates/mini links/offset availability. Missing
   coordinates or links do not trigger guessed relationships or offsets.
4. Import the specified regional threshold/historical CSVs, including BOM,
   spelling, units and dates. Report skipped station codes; reject duplicate keys,
   invalid values/dates. Blank thresholds remain absent, explicit zero valid.
5. Validate both files and replace reference records in one transaction. Failure
   leaves prior references/samples/offsets/stations unchanged. Explicit upgrade of
   an existing canonical database preserves its contents; read-only open never
   upgrades and new readers support previously qualified databases.
6. References can be read detached from storage for charting; no mandatory
   references or assumption that a threshold is a model offset.

**Validation:** Small geographic/projected, multi-part and topology fixtures;
CSV duplicate/unmatched/invalid cases; old-database upgrade/rollback preservation.
Check representative RS geometry/reference coverage explicitly, not by inspection
of filename presence alone.

## G07 — Shared summaries, comparisons and CSV

**Goal:** Make all displayed scientific values and selected-series exports
available through canonical headless queries/calculations.

**No-goals:** Leave-one-out, AR(1), rating curves, new forcing algorithms,
multi-location charts, gap filling, resampling or bulletins.

**Scope:** Analysis effective/accumulated rainfall, area weighting and coverage,
station/basin series alignment, store readers, app query/export composition,
public APIs and CSV CLI.

**Acceptance criteria:**

1. Explicit/relative hourly windows resolve to visible actual bounds. Rainfall
   interval selection matches existing canonical semantics; partial native
   intervals are not silently prorated or counted as complete.
2. Single-mini/upstream area means use validated mini areas and accumulated
   rainfall. An unequal-area fixture proves weighting; missing space/time proves
   coverage is explicit and incomplete totals never masquerade as complete/zero.
3. Query selected observed/effective and modeled rain/level/flow for one station
   or mini across attempts/scenarios. Preserve masks, valid zero, signed levels,
   units, history/forecast segmentation, timezone and distinct reference times.
4. Reject/furnish findings for incompatible location/reference metadata rather
   than overlaying unrelated values. Referenced station levels and raw mini
   levels remain distinct; missing offsets do not become zero.
5. Export precisely selected series and visible interval to the specified CSV
   columns/order, quoting and missing/numeric semantics. Preserve precision;
   language does not change data. API and CLI exports have the same values.
6. Failed export preserves existing destination bytes. Missing/corrupt individual
   caches produce actionable findings without hiding valid unrelated comparisons.

**Validation:** Hand-computed two-mini weighting/time-bound cases, missing data,
signed levels/reference incompatibility and parsed CSV round trips/overwrite
failure. Read actual canonical outputs as well as in-memory fixtures.

## G08 — Operational desktop and shared working state

**Goal:** Deliver a usable saved-data operation through the four activity tabs.

**No-goals:** Independent drafts per tab, shell wrappers, acquisition, completed
visualization beyond the G01 slice, AI runtime or archive management.

**Scope:** Desktop state/actions/workers, workspace picker/inspection, forms,
draft lifecycle, source selection/normalization, QC/edit controls, preview/
registration, preparation/execution/cancel and status/log presentation.

**Acceptance criteria:**

1. One workspace/working operation per window, four specified tabs sharing state;
   switching activities preserves edits/selection. Workspace/draft/dirty identity
   remains visible. No independent forecast recipes accidentally owned by tabs.
2. New/open/import/copy/Save/Save As/Discard use G03. Save/Discard/Keep open protects
   changed content on replacement/workspace switch/close; view-only changes do
   not prompt. No workspace/configuration editor.
3. Forms cover every supported preparation setting and zero/raw/corrected
   scenarios without command syntax. Explicit sources/forecast selections,
   saved normalization and action-specific repair findings are usable.
4. QC scans, point/period/sequence replacements, exclusion, instruction order and
   original/effective display work. Explicit preview/registration use G03/G04;
   name/reason are visibly optional. Stale previews are clearly marked.
5. Run/Prepare only/Execute existing preparation/Cancel use G05. Shared draft
   edits freeze while busy; maps/status/comparisons remain browsable. Workspace
   switching/second work is blocked. Normal close waits for requested cancellation.
6. Background work never performs model/I/O/scientific processing in the render
   callback or applies stale results. Repeated previews/exports do not accumulate
   unbounded workers/log text. Failed tasks return the interface to a usable state.
7. English and Portuguese owned labels/messages and keyboard navigation work
   throughout this increment, not only after visualization. User text/IDs and
   numeric recipe values survive language switching unchanged.

**Validation:** Focused state-transition/action tests and manual saved-data
operation on Linux; no widget snapshot suite. Demonstrate immediate tab/status
interaction while a controlled background task is still active.

## G09 — Full visualization and native exports

**Goal:** Complete agreed maps, selected-area summaries, chart comparisons,
reference import and native image exports in the operational desktop.

**No-goals:** General layout designer, new map providers/offline packaging,
reference editor, multi-location comparison or publication-report rendering.

**Scope:** G06/G07 APIs connected to walkers/egui_plot, presentation caches,
optional view-state restoration, reference-import form, PNG and CSV actions.

**Acceptance criteria:**

1. Stations, rivers, catchments and observed/forecast/corrected rainfall are
   aligned; visibility/opacity, legends, search/click selection and missing values
   work. Individual/upstream boundaries and weighted coverage summaries match G07.
2. Forecast original/corrected maps share navigation and color scales. Chart
   plots are rainfall, level, then flow, with linked time/zoom/cursor, reference
   markers and identifiable selected scenario/attempt lines.
3. Reference import reports counts/skipped codes/errors. Supplied thresholds and
   dated historical marks display correctly; missing references omit lines and
   offsets never become threshold values.
4. Save/reopen restores optional view state where sources remain; missing caches
   produce a finding. View-only changes leave operational dirty state unchanged.
5. PNG at all three resolutions preserves current extent/units/legends/location/
   interval/attribution, excludes irrelevant controls and identifies stale preview
   content. CSV uses G07's selected series/range, with explicit destination and
   overwrite confirmation. Verify actual contents.
6. Network unavailable/blocked tiles leave local layers and chart interaction
   usable. No bulk fetching. Loading/corrupt comparison entries do not freeze the
   frame or discard other visible results.
7. At the agreed display size, keyboard focus and searchable alternatives work,
   translated labels remain reachable and status is not color-only. Record RS
   hardware/backend/data sizes and observed responsiveness, without inventing
   unqualified platform/performance claims.

**Validation:** Manual RS map/selection/export inspection plus focused coordinate,
view-state, selection and export-content checks. Reuse G01 qualification and G07
scientific tests instead of duplicating their algorithms in GUI tests.

## G10 — Milestone acceptance and documentation

**Goal:** Establish a complete shared-core saved-data workflow with evidence and
accurate supported-behavior documentation.

**No-goals:** Windows certification, live acquisition, hydrological/state
qualification, expanding deferred features or a routine completion report.

**Scope:** End-to-end GUI/CLI/API case, scientific regression, all required gates,
launch/setup documentation, contracts, architecture and migration tracking.

**Acceptance criteria:**

1. Open the same workspace/draft via GUI and CLI/API; review/preview unregistered
   edits, prepare and execute the same scenarios. Compare selected instructions,
   full scientific arrays/masks/time/units/reference metadata and canonical result
   values. Compare content, not generated timestamps/paths/attempt identifiers.
2. Repeat trials without registration or Save run; compare successful scenarios
   across cache entries. Register only selected edits, then prove equivalent
   results. Source raw observations/forecasts remain unchanged.
3. Demonstrate draft reopen/discard, explicit changed-source refresh, frozen-input
   rerun after source removal, cancellation, one failed scenario with healthy
   siblings, workspace contention, offline tiles, both languages and PNG/CSV.
4. Existing saved RS qualification remains exact through the refactored core.
   Retain the initial-state/hydrological caveat. Old retained runs/preparations
   remain readable; new execution retention matches the specification.
5. Default and desktop-feature quality gates pass. Manual acceptance records
   Linux environment/display/hardware/backend/dataset and observations. Explicitly
   retain Windows as unverified and hosted CI evidence only when actually run.
6. README documents optional build/launch dependencies and commands. Contracts
   describe implemented JSON/actions/reference import/cache/optional-audit
   behavior; architecture describes durable ownership/lifecycle decisions. Update
   migration rows for delivered versus still-deferred scope. Repair links and
   retire accepted plans under AGENTS.md without promoting implementation
   workarounds or duplicating scientific evidence.

**Validation:** One representative shared-draft GUI/CLI/API acceptance exercise,
the saved-case regression, focused negative cases above and the required gates.
Keep frozen evidence separate from disposable experiments; do not rerun broad
checks repeatedly without a new change/failure justifying it.
