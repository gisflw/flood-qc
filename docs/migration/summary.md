# Migration summary

## Objective

Build a Rust successor to `mgb_ops` using its existing data flow and the additional
operational capabilities found in the parallel VB.NET/Python system. Preserve
both implementations as references in the same Git repository.
The Python reference is checked out at `/workspace/legacy_python/` on
`legacy/python`; the curated VB.NET reference is at `/workspace/legacy_vbnet/`
on `legacy/vbnet`. Rust development uses the independent `main` worktree at
`/workspace/mgb_ops`.

The application is CLI-first. A Windows/Linux desktop application uses the same
Rust capabilities and is introduced early, then extended with the core. All
intended functionality belongs in the Rust package. The external MGB executable
remains the numerical engine; `rs_hydro_forecast` is the target runtime workspace.

## Current migration state

The Linux saved-data operational path is complete:
workspace inspection, canonical observations/QC/replacements, saved forecast
normalization/corrections, reusable prepared inputs, isolated real MGB execution,
and raw/station integer-centimetre results through shared library/CLI workflows.
Full saved scientific arrays and engine streams match exactly, including a repeat
from retained inputs after the disposable original sources were removed. The
[qualification record](../../references/regression/rs-hourly/rust-qualification.md)
retains source/driver identities, comparison evidence and numerical caveats.

The shared library, CLI, desktop, and scheduler use the workspace run
workflow. The [operation contract](../contracts/operations.md) describes automatic
scenarios, run-local forecast corrections, and frozen preparations. A Linux
offline RS preparation using saved inputs selected zero, ECMWF, and NOAA and
recorded six precipitation flags in the workspace database. The integrated
Linux default and desktop suites, Clippy, standalone architecture check, and
desktop build passed. Rendered checks for the new run form and Windows
qualification remain open.

Current usage lives in the [workspace](../contracts/workspace.md),
[observation](../contracts/observations.md),
[forecast](../contracts/forecasts.md) and
[operation](../contracts/operations.md) contracts. The accepted operational
specifications are retired. The [saved-data contract](../contracts/saved-data.md) records the new operator paths.
Provider adapters and the engine-dependent restart gaps follow their rows below.

The November 21 migration review at `b6ae27a` passed the Linux saved RS regression
and found a desktop build blocker in the inundation date labels. The assistant
implementation corrected the build error and passed the Linux desktop fixture
checks. Rendered smoke checks still require a display and graphics adapter.
See the [diagnosis](../../.dev/reports/migration-diagnosis.md).

Use `not started`, `planned`, `in progress`, `blocked`, and `complete` for the
scope stated in each row. `Planned` means an agreed delivery plan exists, not
that code works. Completion requires acceptance evidence for that stated scope;
record unverified platforms and remaining gaps explicitly.

| Capability and reference behavior | Delivery step | State, evidence, and gaps |
| --- | --- | --- |
| [Frozen RS hourly reference](../../references/regression/rs-hourly/README.md) | 1: consolidated reference capture | Complete: canonical fixtures and staged expectations for four scenarios, exact fresh repeat, direct source normalization equality and shared history-only level recovery. Linux only; acquisition, Windows, state qualification and Rust equivalence remain outside this evidence. |
| [Rust workspace inspection, shared library, and CLI foundation](core_and_cli.md) | 2: read-only foundation | Complete for Linux inspection: strict TOML, path semantics, text/JSON findings, source preservation and architecture checks pass. Hosted CI and Windows remain unverified. Numerical execution acceptance is separate below. |
| [Workspace layout and operational database](../contracts/workspace.md#operating-layout) | Workspace follow-up | The CLI and desktop use one root configuration and one SQLite catalog, with original downloads and normalized assets in separate directories. SQLite stores automatic precipitation exclusions and active generation state. The Linux RS fixture prepared both saved forecast scenarios offline and retained its earlier active generation. Linux tests cover reading the existing active index and retaining earlier attempt records. Live acquisition against the reorganized fixture and Windows remain open. |
| [Live CLI run output](../contracts/operations.md#controlled-actions-and-cancellation) | Saved-run CLI follow-up | Complete for direct saved-run commands on Linux. Synthetic checks cover live history and scenario output, JSON stream routing, engine failure, SIGINT, and publication with blocked or closed console output. Retained engine logs match the fixture bytes. Windows remains unverified. |
| [Live CLI download progress](../contracts/acquisition.md#cli-download-progress) | Acquisition CLI follow-up | Linux fixtures cover ANA spinner refresh and cleanup, redirected station summaries, JSON stream routing, forecast cycle fallback and reuse, and the transition to MGB in Run All. Live provider and Windows checks were outside this console validation. |
| [History, assets, and persisted operation data](core_and_cli.md) | 3 onward: grow store with each operational capability | Complete for the saved path: canonical observation and forecast/correction SQLite, forcing/result NetCDF, frozen prepared dependencies and per-run JSON. Legacy conversion remains regression-only; broader imports follow acquisition. |
| [Canonical observations, QC, and observed forcing](../contracts/observations.md) | 3, step 2a: observed forcing | Complete: canonical SQLite, explicit ordered replacements, ranking/QC and pure interpolation; full saved arrays, selected rows and findings exact. Timestamp fixture conversion and numerical provenance retained in the qualification record. |
| [Saved forecast normalization and forecast forcing](../contracts/forecasts.md) | 3, step 2b: forecast forcing | Complete: qualified driver and metadata/grid validation, immutable catalog, ordered corrections and library/CLI workflow. Full native, working, basin and transformed-frame comparisons exact; masks, coordinates and decoded local times/bounds verified. |
| [Saved RS integration, execution, and decoded results](../contracts/operations.md) | 3, integration and step 3: first operational Rust path | Complete on Linux: reusable frozen preparation, OS lock, isolated scenarios, history-only level recovery, raw/station integer centimetres and per-trial manifests. Two saved executions match full arrays and raw streams exactly after original sources are removed. Parallel scenario staging and faster preparation preserved prepared and published payload hashes across the [RS performance trials](../../.dev/reports/operation-performance.md). Lifecycle, failure, and integrity checks pass. The separate state row records new capture evidence. Windows and hydrological validity remain unqualified. |
| [Canonical saved input intake](../contracts/saved-data.md#canonical-data-and-review) | 5 | Complete for canonical inputs. Transactional observation and inventory updates, immutable forecast NetCDF registration, and validated wind and reservoir JSON import. Repeat, collision, and rollback checks pass. |
| [ANA, ECMWF, and NOAA acquisition](../contracts/acquisition.md) | Adapter increment | Implemented in the library, CLI, and desktop, including ANA inventory bootstrap and CSV replay, immutable forecast registration, and acquisition-enabled Run All. Linux live checks acquired one ANA station-day, one complete ECMWF 00/12 product cycle and one complete NOAA GFS cycle. Repeating both forecast updates reused the registered assets. Linux Xvfb with software Vulkan exercised desktop CSV import, forecast reuse, and Run All publication with a synthetic engine. Windows remains unverified. |
| [Standalone IMERG, BNDMET rain and wind, and ONS hourly flow](../contracts/acquisition.md#standalone-source-acquisition) | Adapter increment | Standalone downloads and normalization to CSV, wind JSON, or gridded NetCDF. Linux checks acquired and normalized two ONS reservoir series across midnight. BNDMET returned HTTP 403, and IMERG catalog discovery succeeded while authenticated granule acquisition remained unverified. Offline fixtures cover BNDMET and IMERG normalization, IMERG redirect handling, and preservation of valid ONS readings when a measurement is invalid. Windows, workspace catalog registration, and processing integration remain open. |
| [Remaining acquisition and legacy-source imports](acquisition.md) | Later adapter increments | GSMaP, reservoir schedules, station-inventory refresh, and remaining provider/file imports are deferred. |
| [Broader QC, review, corrections, and availability](review_and_corrections.md) | 5 | Complete for canonical data. Flow instructions in the initialized schema, effective series, 72-hour and monthly availability, leave-one-out and all-station rainfall QC, and desktop instruction previews. |
| [Additional forcing and state capture](model_and_hotstarts.md) | 5 | Complete for the supplied Linux RS build. Qualified QOBS, QSUBST and wind formats, captured six state files through an isolated operation after original inputs were moved, and verified retained file digests. |
| [Hot-start consumption and alternate routing](model_and_hotstarts.md) | 5 | Blocked on a compatible executable. The supplied inertial build ignored both the flag and the state bytes. Explicit unsupported requests fail during preparation. |
| [Geometry and calibration editors](calibration_editors.md) | 6 | Complete for the selected engine formats on Linux. Representative byte and validation checks pass. A real RS operation opened the selected private Cell.hig and paruso.hig and decoded results after the original selections were moved. |
| [Numerical analysis and post-processing](analysis_and_reports.md) | 7 | Complete for retained Rust data. AR(1), recent bias, manual offsets, bands, peaks and thresholds, rating tables, cross-run comparisons, CSV/PNG batches, and return-period GeoJSON use shared series. |
| [Modeled inundation](../contracts/inundation.md) | 7 | Complete for the frozen Linux RS case. Regional HAND and MINI GeoTIFFs were provisioned once. All four saved scenarios in RS Hydro have validated masks. Reusing unchanged tiles reduced the raw ECMWF generation and readback time from 232 to 137 seconds, with identical native and overview arrays. See the [RS evidence](../../references/regression/rs-hourly/inundation.md) for pixel, time, bundle, and desktop verification. Windows and comparison with an independent flood footprint remain unverified. |
| [Bulletins and document/data exports](analysis_and_reports.md) | 7 | Complete for configured retained Rust runs. PDF, HTML, Markdown, CSV, and PNG checks pass. Independent NetCDF and SQLite reads reproduced selected regional rainfall and stage values. |
| [Completed-run regeneration and sharing](completed_runs.md) | 7 | An explicitly saved bundle opens as a comparison and can regenerate output with a matching preparation and engine. Workspace run status indexes the active and archived manifests. Saved preparations can be executed independently of later changes to sources. |
| [Desktop interface](desktop_and_automation.md) | 4, then throughout | In progress for full platform qualification. The existing map and runners expose canonical intake, review, calibration, analysis, bulletins, sharing, and scheduling. Linux Xvfb and software Vulkan smoke checks opened all new panes together and exercised geometry and scheduler settings while the map stayed visible. The saved RS desktop check verified ECMWF rainfall checkpoint selection and matching scenario colors in plots, legends, and cursor readouts. Linux Xvfb with software Vulkan checked English Dark and Brazilian Portuguese Light layouts. The Light check exercised expanded layers, the regional rainfall legend, and a populated synthetic comparison. Processing uses workspace defaults, an editable reference and scenarios, and the shared preparation and execution path. Charts, modeled inundation, and rainfall follow the same [active generation](../contracts/operations.md#active-run-and-rainfall-cache) after each successful execution. Operators open explicitly saved runs for comparison. Synthetic checks cover source routing, valid zero, masks, replacement and failure retention. Observation review saves ordered edits to SQLite. Forecast correction values travel with their run scenarios. The [ribbon and map comparison update](../contracts/desktop.md#layout-and-settings) passed the Rust desktop suite and Linux Xvfb checks on software Vulkan. English Dark and Brazilian Portuguese Light checks covered inline settings, overflowing tabs, synchronized map gestures, the swipe divider, side-specific cell inspection and totals, upstream area, and active-cache replacement. A later Linux Xvfb check against the saved RS workspace verified the permanent Map tab, blue rainfall scale, upstream totals, hover and run readouts in the bottom stripe, and taller charts. A subsequent Linux Xvfb check verified the editable rainfall scale, NASA Blue Marble tiles and attribution, station hover, and the one-line footer. Regression tests exercise exterior-only basin boundaries and reuse of visible geometry during short pans. A repeat Linux Xvfb probe on the saved RS workspace found lower median UI time during short drags in Single, Double, and Swipe. The probe used software Vulkan. Display-scale-aware river detail further reduced median intervals between presented frames during short drags from 168 to 42 ms in Single, 276 to 42 ms in Double, and 173 to 33 ms in Swipe under the same software Vulkan setup. Windows, Wayland and operator hardware remain unverified. |
| [Right-pane docking, chart cursor, and assistant controls](../contracts/desktop.md#layout-and-settings) | Desktop interaction update | Implemented and checked on Linux. Xvfb with Vulkan llvmpipe exercised closing and reopening tabs, reordering and four-group docking, chart pinning with synthetic observations, and multiline chat with a fake ACP adapter. English Light and Brazilian Portuguese Dark layouts were checked. The adapter fixture verified remembered model and effort choices after app restart. The PNG export check passed on llvmpipe. Live provider, operator hardware, and Windows qualification remain open. |
| [Desktop pane fit and chart cursor](../contracts/desktop.md#layout-and-settings) | Desktop interaction follow-up | Implemented. Dock pages clip to their assigned group, wrap controls, and show a thicker vertical scrollbar when needed. Data rows, Markdown tables, and bulletin previews adapt to narrow panes. Transcript layout skips off-screen turns, and the Data table renders visible rows. In the earlier 80-turn CPU layout fixture, the plain-text view took 2.1 ms, Markdown across all turns took 31.5 ms, and Markdown with off-screen turns skipped took 0.7 ms. Linux Xvfb with software Vulkan verified chart labels and wide Markdown content in side-by-side groups and exercised hover, pin, and release on a synthetic station series. Over 13 keystrokes per case, median input-to-framebuffer time was 13 ms with the pointer stationary and 19 ms while it moved across the adjacent chart. Operator hardware and Windows remain unverified. |
| [Desktop chat composer and dock appearance](../contracts/assistant.md#desktop-and-cli) | Desktop interaction follow-up | Implemented rounded dock groups with gaps and a composer that grows to a height limit, then scrolls internally. The desktop uses the patch release of eframe that avoids spinning while waiting for a Wayland redraw. Linux checks used software Vulkan with Xvfb and nested Weston. X11 checks exercised stacked and side-by-side groups, the composer cap, caret scrolling, and streaming a Markdown table from a local adapter fixture. All 13 keystrokes appeared without pointer movement in each trial, including when the pointer was parked over the map. Median input-to-framebuffer times were 13–15 ms on X11 and about 22 ms on nested Wayland. The older build also updated on nested Wayland. The operator traced the reported stall to Remmina. Operator hardware and Windows remain unverified. |
| [Desktop ribbon and scrollbars](../contracts/desktop.md#layout-and-settings) | Desktop interaction follow-up | Project opens workspace settings as a dock page. The Layout group under View holds the language button, a paired appearance control, and reset button. Automation is in the Operation group. Scrollable pages reveal their vertical bars on hover. Desktop tests cover settings tab persistence, clipping, and scrollbar hover. Linux Xvfb with software Vulkan verified the Settings page, Automation placement, and the ribbon controls for appearance and reset. |
| [Scheduling and email](desktop_and_automation.md) | 7 | Complete for local saved-data jobs. Injected-clock and controlled SMTP tests cover due slots, restart and overlap, and delivery failures. External relay delivery remains unverified. |
| [MGB Ops assistant](../contracts/assistant.md) | Shared session, ACP and read tools, operator interfaces, then [desktop interactions](../contracts/assistant.md#desktop-and-cli) | In progress for qualification. The desktop and CLI share workspace and run selection and can review proposed run parameters. The command catalog, ACP supervisor, conversation-local MCP bridge, and operator review are implemented. Linux fake-agent and synthetic-data checks cover the ACP lifecycle, stale run parameter proposals, and selected-run queries. The CLI reports missing adapters at discovery. The initial check lacked loopback access, installed adapters, and graphics. The loopback credential-revocation fixture now passes, and the desktop interaction update adds rendered evidence with a fake adapter. Live provider qualification remains open. Both Clippy configurations, the architecture gate, and the desktop build passed. Earlier assistant checks encountered the denied SMTP listener and timing failures in existing cache and operation checks. Windows remains unverified. |

Split these broad rows as individual capabilities are implemented, preserving
their reference links, delivery responsibility, actual state, evidence, and next
work or gaps. Include runtime scripts and bundled plugins. Update the checklist
when implementation, verification, blockers, or the next increment changes, as
required by [AGENTS.md](../../AGENTS.md).

Linux is the first validation platform. Windows remains a build target, with
checks deferred until later development; no Windows validation infrastructure is
a foundation gate. Engine prerequisite inspection is not proof of successful
execution. Reference repeatability is not Rust equivalence or hydrological
validation.

## Operational flow

```text
Acquire observations and forecasts
    -> normalize canonical data and register selected artifacts through store
    -> inspect, validate, and correct data
    -> freeze selected data, corrections, settings, calibration, geometry, and state
    -> analysis prepares canonical forcing; model serializes engine inputs
    -> execute forecast scenarios and stage decoded canonical outputs
    -> record run/scenario outcomes and output locations in per-run JSON manifests
    -> inspect and post-process results
    -> produce bulletins and data exports
    -> optionally copy generated files for long-term storage or share completed runs
```

App owns the operation and its status/failure policy; the
[architecture](../architecture.md) defines frozen working inputs and independent run outcomes.
Calibration editors support geometry, channel/Manning parameters, and soil parameters.
Scheduling and email automate the same operational capabilities.

## Scope

Lasting product commitments live in the
[architecture](../architecture.md#product-direction). The coverage below
tracks their migration from the reference systems.

Retain the implemented `mgb_ops` acquisition, QC, corrections, analysis, scenarios,
reporting, and regional-workspace behavior. Add the other system's acquisition
and operational features, including satellite rainfall, wind, reservoir inputs,
hot starts, calibration editors, richer post-processing, archive comparisons,
availability views, and relevant imports/exports.

Include behavior currently implemented in `rs_hydro_forecast` scripts, such as
station-inventory updates and bulletin generation, in the functionality to port.
The runtime workspace must not retain Python scripts as required implementations
of migrated capabilities.

Exclude ONS meteorological forecast products. Use both bulletin families as
complementary references; provide PDF, Markdown, HTML, and relevant CSV exports.

Use the existing executable and compatible inputs in `rs_hydro_forecast` for the
initial regression target, retaining its calibration, geometry, and required
initial state. Porting calibration/geometry editors does not require replacing
that baseline or establishing regression evidence for the editors. Generate
time-dependent inputs in Rust and reproduce the VB.NET hot-start lifecycle as
those capabilities are implemented.

Run artifacts record the recipe and references for regenerating inputs and outputs
when dependencies remain available; operators choose which generated files to copy.
Sharing concerns completed operations, not simultaneous editing. Every cache is
optional; no cache-packaging design is required at this stage.

## Delivery approach

The responsibility files below describe feature coverage, not Rust modules or
implementation details. Follow the [architecture](../architecture.md) and extend
the frozen regression case as capabilities arrive. Keep the working operational
path intact. Calibration/geometry editors use functional acceptance rather than
requiring replacement of the regression baseline.

Plan and specify active work under `.dev/`; implementation results may live under
`.dev/reports/`. Retire accepted plans/specifications, keep current evidence/gaps in
this checklist, and preserve unique scientific provenance with its reference.

| File | Responsibility |
| --- | --- |
| [Core and CLI](core_and_cli.md) | Workspaces, configuration, persistence foundations, and operation commands. |
| [Acquisition](acquisition.md) | Adapters acquire/normalize canonical data; app decides persistence through store. |
| [Review and corrections](review_and_corrections.md) | QC, manual observation changes, and rainfall transformations. |
| [Model and hot starts](model_and_hotstarts.md) | Input generation, state lifecycle, scenario execution, and output decoding. |
| [Calibration editors](calibration_editors.md) | Geometry, channel/Manning, and soil-parameter editing. |
| [Analysis and reports](analysis_and_reports.md) | Post-processing, comparisons, maps, bulletins, and exports. |
| [Completed runs](completed_runs.md) | Regeneration, archival, and sharing. |
| [Desktop and automation](desktop_and_automation.md) | Early Windows/Linux interface, incremental GUI coverage, scheduling, and email. |

## Suggested increments

1. Establish the [minimal reproducible regression target](initial_regression_tests.md)
   from `rs_hydro_forecast`, covering the main operational steps, and the
   functionality checklist. This captures reference evidence before Rust
   replacement; it does not require implementing the Rust application first.
2. Establish the [Rust foundation](../contracts/workspace.md): a
   shared library, read-only workspace inspection CLI, documented contracts, and
   enforced architecture/quality rules. Validate on a disposable RS workspace on
   Linux; defer Windows checks. Add persistence as operational capabilities need
   it rather than creating the entire storage design upfront.
3. Reproduce the saved preparation, real MGB execution, and decoded-results path
   in Rust. Compare with the regression target, carrying the required initial
   state unchanged. Generate execution-start input snapshots and per-run
   JSON manifests through shared app workflows/store; keep failure
   diagnostics and successful sibling outputs without changing other runs. Hot-start
   generation and chaining follow later.
4. Introduce a working GUI over that shared core. Perform the available operation
   through CLI and GUI with consistent results. Extend the GUI alongside every
   subsequent capability rather than waiting for the complete core.
5. Add acquisition, QC, corrections, remaining forcing inputs, and hot-start
   handling incrementally. Validate each addition, extend the regression target
   where relevant, and keep the established operational path working.
6. Port calibration/geometry editors. Define functional acceptance during their
   implementation; no regression target or baseline replacement is required.
7. Complete analysis, complementary bulletins, exports, completed-run sharing,
   scheduling, and email, updating the GUI throughout. Demonstrate the completed
   workflows on the supported platforms and account for the functionality
   checklist.

Keep a short record of implemented capabilities and the next increment; avoid a
detailed speculative backlog. Basic analysis or exports may be connected earlier
to exercise the working operation; the last increment completes their coverage.
