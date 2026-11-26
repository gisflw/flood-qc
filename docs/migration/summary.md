# Migration summary

## Objective

Build a Rust successor to `mgb_ops` using its existing data flow and the additional
operational capabilities found in the parallel VB.NET/Python system. Preserve
both implementations as references in the same Git repository.
The Python and curated VB.NET references live on `legacy/python` and
`legacy/vbnet`. See [source locations](../../AGENTS.md#sources-and-experiments)
for the regional workspace and frozen artifacts.

The application is CLI-first. A Windows/Linux desktop application uses the same
Rust capabilities and is introduced early, then extended with the core. All
intended functionality belongs in the Rust package. The external MGB executable
remains the numerical engine; `rs_hydro_forecast` is the target runtime workspace.

## Current migration state

The [dependency and Rust safety policy](../contracts/rules-exceptions.md#rust-safety-and-dependency-policy)
is established, but dependency compliance remains open. The native mask writer
has a [scoped exception](../contracts/rules-exceptions.md#approved-c-backed-exceptions) while the Rust
writer lacks incremental compressed output. Reassess that capability when
updating scientific I/O dependencies. Other C-backed compression and TLS
implementations still need replacement with Rust implementations. Audit the
enabled direct and transitive dependencies in CLI and desktop builds on both
target platforms; existing usage alone is not an exemption.
Preserve supported file formats and processing behavior during replacement.

The previous Linux implementation qualified the frozen RS case against saved
arrays and raw engine streams, including a repeat after removing disposable
source inputs. Those comparisons describe the storage and native-grid
correction order used for that capture. The [qualification
record](../../references/regression/rs-hourly/rust-qualification.md) preserves the
driver identities, comparisons and numerical caveats.

The [input-set workflow](../contracts/inputs.md) implements the clean
storage break: Parquet 2 observations and forcing tables, NetCDF forecast bases,
and JSON run and input manifests. Scenarios apply ordered edits to hourly working
grids. Linux paired release checks completed with exact unchanged forcing/result
arrays and no established repeatable slowdown; see [input-set
qualification](#input-set-qualification). Independent saved-input regeneration,
replacement, corruption and failure retention have automated coverage.

Current Linux validation passed formatting, both Clippy configurations, both
full test suites at normal parallelism, the standalone architecture gate and
the desktop build. Native mask lifecycle checks are isolated from unit tests
that launch children. The explicit RS mask regression matched all 4,344 decoded
native and overview frame digests. The release CLI passed version/help checks;
the release desktop opened and closed normally under Xvfb with Mesa llvmpipe,
and the explicit PNG export check passed on that software renderer. These are
software-rendering checks; operator hardware and Windows remain unverified.

The strict frozen engine-stream check does not pass on the current host. Direct
execution of the frozen engine and byte-identical frozen inputs reproduces the
same drift without Rust; the [execution-environment
record](../../references/regression/rs-hourly/rust-qualification.md#current-execution-environment)
records the evidence and limits. The frozen byte requirement is unchanged.
Windows, operator hardware and rendering of the new input-decision dialog remain
unverified.
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
| [Workspace layout and run input retention](../contracts/inputs.md) | Input-set change | Implemented on Linux. TOML, revision-checked API sessions, current inputs, independent saves and per-run manifests have distinct lifecycles. Automated checks cover independent regeneration with a matching installed engine, explicit replacement decisions and incomplete-cache recovery. |
| [Live CLI run output](../contracts/operations.md#controlled-actions-and-cancellation) | Saved-run CLI follow-up | Complete for direct saved-run commands on Linux. Synthetic checks cover live history and scenario output, JSON stream routing, engine failure, SIGINT, and publication with blocked or closed console output. Retained engine logs match the fixture bytes. Windows remains unverified. |
| [Live CLI download progress](../contracts/acquisition.md#cli-download-progress) | Acquisition CLI follow-up | Linux fixtures cover ANA spinner refresh and cleanup, redirected station summaries, JSON stream routing, forecast cycle fallback and reuse, and the transition to MGB in Run All. Live provider and Windows checks were outside this console validation. |
| [History, assets, and persisted operation data](core_and_cli.md) | Input-set change | Implemented on Linux. Observation metadata and samples use Parquet 2, grids use NetCDF, and JSON manifests select active data and results. Default and desktop dependency graphs contain no SQLite package; CLI linkage has no SQLite library. Spatial GeoPackages use the pure Rust reader. |
| [Canonical observations, QC, and observed forcing](../contracts/observations.md) | Input-set change | Implemented on Linux. Ranking, ordered replacements, QC and interpolation consume columnar assets. Matched raw-series reports and unchanged preparation arrays agree exactly with the preceding implementation; null, missing and zero, concurrent readers, rollback and revision precedence have automated coverage. |
| [Saved forecast normalization and forecast forcing](../contracts/forecasts.md) | Input-set change | Implemented on Linux. Folder discovery selects normalized bases independently of raw downloads. Zero, forecast and earlier-scenario bases share the hourly working-grid recipe. Tests cover ordered edits, preview equality, cross-format reuse, force replacement and atomic rollback. An offline ECMWF cycle reused its asset with the raw source removed. Frozen native-grid correction results remain historical evidence. |
| [Saved RS integration, execution, and decoded results](../contracts/operations.md) | Saved-data execution | Current-host Linux checks reproduce the preceding implementation’s complete unchanged forcing and decoded result arrays for zero, ECMWF and NOAA. Saved-input repeats and real engine execution succeed. Cross-environment frozen byte equivalence remains unqualified as explained above; Windows and hydrological validity remain unqualified. |
| [Canonical saved input intake](../contracts/saved-data.md#canonical-data-and-review) | 5 | Implemented on Linux. Canonical observations/inventory, normalized forecast registration and explicit wind/reservoir JSON import produce the new layout. Automated checks cover repeat, collision, corruption, replacement and rollback. |
| [ANA, ECMWF, and NOAA acquisition](../contracts/acquisition.md) | Adapter increment | Implemented in the library, CLI, and desktop, including ANA inventory bootstrap and CSV replay, normalized forecast registration, and acquisition-enabled Run All. Linux live checks acquired one ANA station-day, one complete ECMWF 00/12 product cycle and one complete NOAA GFS cycle. Repeating both forecast updates reused the registered assets. Linux Xvfb with software Vulkan exercised desktop CSV import, forecast reuse, and Run All publication with a synthetic engine. Windows remains unverified. |
| [Standalone IMERG, BNDMET rain and wind, and ONS hourly flow](../contracts/acquisition.md#standalone-source-acquisition) | Adapter increment | Standalone downloads and normalization to CSV, wind JSON, or gridded NetCDF. Linux checks acquired and normalized two ONS reservoir series across midnight. BNDMET returned HTTP 403, and IMERG catalog discovery succeeded while authenticated granule acquisition remained unverified. Offline fixtures cover BNDMET and IMERG normalization, IMERG redirect handling, and preservation of valid ONS readings when a measurement is invalid. Windows, workspace catalog registration, and processing integration remain open. |
| [Remaining acquisition and legacy-source imports](acquisition.md) | Later adapter increments | GSMaP, reservoir schedules, station-inventory refresh, and remaining provider/file imports are deferred. |
| [Broader QC, review, corrections, and availability](review_and_corrections.md) | 5 | Complete for canonical data. Flow instructions in the initialized schema, effective series, 72-hour and monthly availability, leave-one-out and all-station rainfall QC, and desktop instruction previews. |
| [Additional forcing and state capture](model_and_hotstarts.md) | 5 | Complete for the supplied Linux RS build. Qualified QOBS, QSUBST and wind formats, captured six state files through an isolated operation after original inputs were moved, and verified retained file digests. |
| [Hot-start consumption and alternate routing](model_and_hotstarts.md) | 5 | Blocked on a compatible executable. The supplied inertial build ignored both the flag and the state bytes. Explicit unsupported requests fail during preparation. |
| [Geometry and calibration editors](calibration_editors.md) | 6 | Complete for the selected engine formats on Linux. Representative byte and validation checks pass. A real RS operation opened the selected private Cell.hig and paruso.hig and decoded results after the original selections were moved. |
| [Numerical analysis and post-processing](analysis_and_reports.md) | 7 | Complete for retained Rust data. AR(1), recent bias, manual offsets, bands, peaks and thresholds, rating tables, cross-run comparisons, CSV/PNG batches, and return-period GeoJSON use shared series. |
| [Modeled inundation](../contracts/inundation.md) | 7 | Implemented for the frozen Linux RS case. HAND heights are converted from decimetres to metres, with valid zero and negative heights eligible under the catchment stage threshold. All four active RS scenario masks were regenerated on Linux, and independent NetCDF reads verified the conversion provenance and 1,724,676 wet cells at the first history hour. Earlier pixel counts qualify the previous rule. Mask processing and its automated checks are included in the standard CLI and desktop-enabled builds. The standard Linux release CLI replaced the ECMWF mask in a disposable copy of the active RS run and recorded the active manifest digest. Regional HAND and MINI GeoTIFFs were provisioned once. All four saved scenarios in RS Hydro have validated masks. Reusing unchanged tiles reduced the raw ECMWF generation and readback time from 232 to 137 seconds, with identical native and overview arrays. See the [RS evidence](../../references/regression/rs-hourly/inundation.md) for pixel, time, bundle, and desktop verification. Windows and comparison with an independent flood footprint remain unverified. Linux default-parallel tests pass with mask lifecycle checks in a separate integration-test process, avoiding interference from child-spawning unit tests. The explicit corrected-rule RS mask regression now compares every native and overview pixel at all 362 times across all four frozen scenarios, using independent GDAL/NumPy expectations. All 4,344 decoded frame digests match on Linux. Native descriptor inheritance remains a known library limitation; see the [investigation](../../.dev/reports/netcdf-inundation-investigation.md). |
| [Bulletins and document/data exports](analysis_and_reports.md) | 7 | Complete for configured retained Rust runs. PDF, HTML, Markdown, CSV, and PNG checks pass. In the previous storage layout, independent NetCDF and SQLite reads reproduced selected regional rainfall and stage values. |
| [Completed-run regeneration and sharing](completed_runs.md) | 7 | Implemented on Linux. Completed output bundles support comparison and may include matching independent inputs for regeneration with an installed engine. Tests cover standalone/bundle replay, recipe and engine binding, tampered payload rejection, and retention after failure. |
| [Desktop interface](desktop_and_automation.md) | 4, then throughout | In progress for full platform qualification. The existing map and runners expose canonical intake, review, calibration, analysis, bulletins, sharing, and scheduling. Linux Xvfb and software Vulkan smoke checks opened all new panes together and exercised geometry and scheduler settings while the map stayed visible. The saved RS desktop check verified ECMWF rainfall checkpoint selection and matching scenario colors in plots, legends, and cursor readouts. Linux Xvfb with software Vulkan checked English Dark and Brazilian Portuguese Light layouts. The Light check exercised expanded layers, the regional rainfall legend, and a populated synthetic comparison. Processing uses workspace defaults, an editable reference and scenarios, and the shared preparation and execution path. Charts, modeled inundation, and rainfall follow the same [active generation](../contracts/operations.md#active-run-and-rainfall-cache) after each successful execution. Operators open explicitly saved runs for comparison. Synthetic checks cover source routing, valid zero, masks, replacement and failure retention. Observation review saves ordered edits to the canonical observation store. Ordered scenario edits travel with their run recipes. The [ribbon and map comparison update](../contracts/desktop.md#layout-and-settings) passed the Rust desktop suite and Linux Xvfb checks on software Vulkan. English Dark and Brazilian Portuguese Light checks covered inline settings, overflowing tabs, synchronized map gestures, the swipe divider, side-specific cell inspection and totals, upstream area, and active-cache replacement. A later Linux Xvfb check against the saved RS workspace verified the permanent Map tab, blue rainfall scale, upstream totals, hover and run readouts in the bottom stripe, and taller charts. A subsequent Linux Xvfb check verified the editable rainfall scale, NASA Blue Marble tiles and attribution, station hover, and the one-line footer. Regression tests exercise exterior-only basin boundaries and reuse of visible geometry during short pans. A repeat Linux Xvfb probe on the saved RS workspace found lower median UI time during short drags in Single, Double, and Swipe. The probe used software Vulkan. Display-scale-aware river detail further reduced median intervals between presented frames during short drags from 168 to 42 ms in Single, 276 to 42 ms in Double, and 173 to 33 ms in Swipe under the same software Vulkan setup. Windows, Wayland and operator hardware remain unverified. |
| [Right-pane docking, chart cursor, and assistant controls](../contracts/desktop.md#layout-and-settings) | Desktop interaction update | Implemented and checked on Linux. Xvfb with Vulkan llvmpipe exercised closing and reopening tabs, reordering and four-group docking, chart pinning with synthetic observations, and multiline chat with a fake ACP adapter. English Light and Brazilian Portuguese Dark layouts were checked. The adapter fixture verified remembered model and effort choices after app restart. The PNG export check passed on llvmpipe. Live provider, operator hardware, and Windows qualification remain open. |
| [Desktop pane fit and chart cursor](../contracts/desktop.md#layout-and-settings) | Desktop interaction follow-up | Implemented. Dock pages clip to their assigned group, wrap controls, and show a thicker vertical scrollbar when needed. Data rows, Markdown tables, and bulletin previews adapt to narrow panes. Transcript layout skips off-screen turns, and the Data table renders visible rows. In the earlier 80-turn CPU layout fixture, the plain-text view took 2.1 ms, Markdown across all turns took 31.5 ms, and Markdown with off-screen turns skipped took 0.7 ms. Linux Xvfb with software Vulkan verified chart labels and wide Markdown content in side-by-side groups and exercised hover, pin, and release on a synthetic station series. Over 13 keystrokes per case, median input-to-framebuffer time was 13 ms with the pointer stationary and 19 ms while it moved across the adjacent chart. Operator hardware and Windows remain unverified. |
| [Desktop chat composer and dock appearance](../contracts/assistant.md#desktop-and-cli) | Desktop interaction follow-up | Implemented rounded dock groups with gaps and a composer that grows to a height limit, then scrolls internally. The desktop uses the patch release of eframe that avoids spinning while waiting for a Wayland redraw. Linux checks used software Vulkan with Xvfb and nested Weston. X11 checks exercised stacked and side-by-side groups, the composer cap, caret scrolling, and streaming a Markdown table from a local adapter fixture. All 13 keystrokes appeared without pointer movement in each trial, including when the pointer was parked over the map. Median input-to-framebuffer times were 13–15 ms on X11 and about 22 ms on nested Wayland. The older build also updated on nested Wayland. The operator traced the reported stall to Remmina. Operator hardware and Windows remain unverified. |
| [Desktop ribbon and scrollbars](../contracts/desktop.md#layout-and-settings) | Desktop interaction follow-up | Project opens workspace settings as a dock page. The Layout group under View holds the language button, a paired appearance control, and reset button. Automation is in the Operation group. Scrollable pages reveal their vertical bars on hover. Desktop tests cover settings tab persistence, clipping, and scrollbar hover. Linux Xvfb with software Vulkan verified the Settings page, Automation placement, and the ribbon controls for appearance and reset. |
| [Desktop chat history](../contracts/assistant.md#desktop-and-cli) and [map interactions](../contracts/desktop.md#layout-and-settings) | Desktop interaction follow-up | Implemented. Linux headless checks cover focused Enter submission to a fake ACP adapter, saved history and native session replay, text carry-over, credential revocation, Unicode text actions, wheel and middle-button gestures in all map modes, style restoration, basin holes, and retained rainfall/inundation replacements and failures. Default and desktop tests, both Clippy configurations, formatting, the standalone architecture gate, and the desktop build passed. The explicit PNG graphics check could not acquire a graphics adapter in this environment. Native-window rendering, operator hardware, live ACP providers, and Windows remain unverified. Map/chart canvas text selection is deferred. |
| [Desktop appearance and map presentation](../contracts/desktop.md#layout-and-settings) and [assistant controls](../contracts/assistant.md#desktop-and-cli) | Desktop presentation follow-up | Implemented. Formatting, default and desktop Clippy/tests, the standalone architecture gate, and the desktop build passed. Linux tests cover remembered dialog locations across restarts, preserved storage identity, assistant update paragraphs and Enter submission through a fake ACP adapter, station hover, rainfall extrema, and cached catchment strokes. Xvfb with software Vulkan checked English and Brazilian Portuguese Light/Dark layouts, narrow panes, native window controls and icon identity, rainfall expansion and loaded-grid ranges, station pointer labels and rainfall-only footer inspection, catchment divisions, and the loaded upstream card and Clear action on synthetic data. PNG export checks passed at 1×, 2×, and 3× on Vulkan llvmpipe. The separate regional fixture could not run because its external workspace was absent. Wayland, operator hardware, and Windows remain unverified for this update. |
| [Desktop rendering and controls](../contracts/desktop.md#layout-and-settings) and [assistant input](../contracts/assistant.md#desktop-and-cli) | Desktop correction follow-up | Implemented. Catchment projection, display detail, and bounded mesh preparation run outside UI frames, with cancellation and stale-result rejection. The explicit Linux debug fixture processed 10,000 catchments with 1,010,000 ring points; background meshes took 95 ms and paint/tessellation during short pans took 44–48 ms. A 1,200,001-point zigzag simplification took 25 ms. Linux Xvfb and nested Weston with software Vulkan checked the custom title strip, compact layer controls, ribbon groups, icon registration, native Enter/Shift+Enter with a fake ACP adapter, paragraph rendering, and Light/Dark appearances. Regression checks cover geometry holes, palette persistence, cancelled rendering, and remembered dialog precedence. PNG exports passed at 1×–3× on llvmpipe. Formatting, both Clippy/test configurations, standalone architecture checks, and the desktop build passed; final interface checks also cover the refreshed prompt text layout. The real regional workspace, operator hardware, live providers, and Windows remain unverified for this follow-up. |
| [Desktop visual refinements](../contracts/desktop.md#layout-and-settings) and [message copy controls](../contracts/assistant.md#desktop-and-cli) | Desktop appearance follow-up | Implemented. The title strip omits the workspace name and uses smaller window controls. The uncovered map matches the surrounding pane backing, and message copy icons are smaller and translucent at rest. Headless Linux checks verified Light/Dark colors, copy hover/focus and confirmation states, clipboard output, and title-strip gestures and window commands. Formatting, both Clippy/test configurations, the standalone architecture check, and the desktop build passed. Native-window rendering, operator hardware, and Windows remain unverified for this update. |
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

## Input-set qualification

Formatting, both Clippy configurations with warnings denied, the default suite
(175 passed), desktop-enabled suite (238 passed), independent architecture gate
(9 passed), and desktop binary build passed on Linux. The explicit PNG export
check also passed on Vulkan llvmpipe (Mesa 25.0.7, LLVM 19.1.7); this is software
rendering evidence. Native rendering of the new input-decision dialog, operator
hardware and Windows remain unverified. The strict regional engine-stream check
has the separate execution-environment limitation recorded above.

Linux release measurements on 2026-11-25 compared this change with `a29e319`
on the same host, alternating execution order. The matched regional observation
fixture contained 1,595,647 samples from 2020–2026 and 440 stations. Preparation
and execution used the same zero, ECMWF and NOAA scenarios. Independent NetCDF
reads compared full forcing and result arrays, masks, coordinates and labels;
all unchanged values matched exactly. Ordered-edit semantics have separate
synthetic checks, since the historical corrected scenario used a different order.

| Workload | Paired repetitions | Previous median | Parquet/input-set median |
| --- | ---: | ---: | ---: |
| Regional preparation | 20 | 11.752 s | 9.597 s |
| Prepared real-engine execution | 7 | 28.703 s | 28.588 s |
| Rolling station update, first batch | 80 | 93.73 ms | 93.15 ms |
| Rolling station update, second batch | 80 | 96.37 ms | 92.75 ms |
| Cold-process raw series, first batch | 80 | 10.39 ms | 10.91 ms |
| Cold-process raw series, second batch | 80 | 10.62 ms | 10.73 ms |

Preparation’s median improved by 18.3%; execution remained within observed
variation. Raw-series reports matched in all 160 pairs. Their small positive
paired median differences had bootstrap 95% intervals spanning zero in both
batches; means and 95th percentiles were lower. Rolling-update medians showed no
repeatable regression; one Parquet outlier means universal tail equivalence is
not established. These checks qualify the measured workloads, not every archive
or hardware configuration. Parent-process peak RSS rose from 304 to 322 MiB for
preparation and from 1,284 to 1,307 MiB for execution, excluding engine children.

A five-day regional scenario preview took 152 ms cold and 22 ms warm median
(24.7 ms warm maximum), with the same checksum over 21 calls. A captured input
set held 174 MiB of indexed payload; 200 independent copies would occupy about
34 GiB, excluding small manifests and any separately retained outputs.

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

The run manifest records resolved inputs and outcomes. [Input
sets](../contracts/inputs.md) carry the selected source values, model inputs and
recipe for regeneration. Operators decide which input sets and results to save.

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
