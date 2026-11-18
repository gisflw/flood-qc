# Desktop layout and runners specification

Implement a RustGIS-style ribbon, single sidebar and dockable linked monitoring
pages, with working configuration and preparation/execution forms over the existing
backend. This increment implements part of the broader [GUI plan](../plans/desktop_gui.md)
and does not replace or narrow that plan. Requirements here are planned behavior;
the [desktop contract](../../docs/contracts/desktop.md) describes current behavior.

Preserve existing monitoring, exports, backend validation, workspace path semantics,
source acceptance and execution lifecycle. Use the shared workspace configuration,
operation draft and controlled preparation/execution APIs. Use the reference/horizon
authoring format in the shared backend; do not duplicate
period derivation or scientific processing in the GUI.

## Ribbon and pane layout

Use [RustGIS](https://github.com/opengeos/RustGIS) as the visual reference.
A top tab strip switches between visible labeled groups of compact actions.
Ribbon actions open or focus existing pages and forms.

| Region | Contents and behavior |
| --- | --- |
| Top ribbon | Project and draft files, page and sidebar visibility, Reset layout, processing launchers and settings launchers. |
| Left sidebar | Files, Workspace Settings, Interface Settings, Prepare inputs, Execute MGB, Run All and Jobs. Files owns the retained-run picker and provenance. Forms use labeled sections in scrollable tabs. |
| Center workspace | Dockable Map, Charts and Data. The bordered map viewport sits below Map controls. Charts owns comparison fields, source/window controls and exports. Data contains the series table and location summaries. |

The sidebar resizes and collapses. Launching a form opens or focuses its own tab
and reveals the sidebar. Initially Files is active and Map and Charts appear
side by side. Data can reopen from the ribbon.

### Center docking and linked pages

Dragging a center tab offers a visible drop preview for joining a tab group or
splitting an area. Support one area, two columns, two rows, three-area layouts
(one area beside two stacked areas, or the corresponding rotated arrangement),
and a 2×2 grid. Limit the center to two rows and two columns, with at most four
visible areas. Dividers are resizable, and each area can contain multiple tabs
with one active tab. Moving the last tab out of an area removes the empty area.
Tabs stay inside the main window; floating windows are outside this delivery.

There is one instance per page. Ribbon actions focus a page if open or reopen
it if closed. Map, Charts and Data share the loaded run state. A map pick updates the MINI ID
and Station ID fields through explicit links when unique and otherwise retains
the counterpart. Users can edit either field and submit with Enter or Apply.
Charts compares the selected model MINI and observation station. Use the retained
station datum for linked levels with a matching offset and label raw MINI levels
when no matching result exists. Chart loading is independent of map rainfall and
layer loading. A selection change updates linked pages in the background without
activating a hidden tab, changing focus or rearranging the layout. Closing or
moving a page does not clear shared selection or its view state.

### Settings and saved layout

In the Interface Settings tab, use a language dropdown with English and Brazilian Portuguese
and an appearance control with System, Light and Dark. Apply changes immediately
and remember them app-wide. System follows the platform appearance. Apply the
theme consistently to the ribbon, panes, forms, tables and charts while keeping
data colors and labels readable.

Remember sidebar width, visibility, open and active tabs, and center docking
layout separately for each project, using automatic interface preferences rather
than another project-file schema. Reset layout restores the initial arrangement
above while preserving selections, loaded results and unfinished form inputs.
Keep page/pane reopening, focus navigation and actions keyboard accessible;
icon controls need accessible names and tooltips. Narrow windows must retain
usable controls through pane collapse and scrolling rather than clipped forms.

## Configuration editing and unsaved work

The Settings ribbon opens or focuses Workspace Settings in the sidebar. Edit the supported workspace TOML fields, currently geometry
path and engine executable/input-directory paths, with explicit Save and Revert.
Do not add unsupported configuration fields or place period/scenario settings in
workspace TOML. Operation settings remain in the shared runner draft.
Use the existing backend read/save APIs, validation, atomic replacement and path
semantics. Failed saves preserve destination bytes and leave edits available.

Center pages and sidebar forms have close actions and can be reopened from
the ribbon or navigation. Closing a form hides it while preserving unfinished
inputs for the current project session; it does not save or discard them.
Before closing a project, switching projects or quitting, offer Save, Discard
or Cancel for unsaved workspace configuration and operation-draft changes,
including changes in hidden forms. Cancel or a failed save keeps the current
project and edits available. Discard restores the corresponding saved baseline.
View/layout preference changes alone do not make operational settings dirty.

## Processing settings

Keep period, scenarios and output stations readily accessible. Group paths and
advanced numerical settings clearly, initialized from existing project
configuration where available. Do not silently select forecast inputs.

| Group | Controls |
| --- | --- |
| Period | One reference date/time, timezone, observed days before and forecast days after. Display backend-derived coverage read-only; hourly steps are fixed. |
| Observations | SQLite path, providers, optional station/series selection and ordered existing replacement instructions. |
| Forecast scenarios | Scenario ID, zero/raw/corrected rainfall, forecast asset and ordered existing corrections. |
| Output stations | Explicit stations for station-level model results. |
| Spatial preparation | Geometry file, rainfall grid extent/resolution, interpolation station count and distance power. |
| QC settings | High-rainfall threshold, sequence bounds and minimum sequence length. Findings do not automatically apply corrections. |
| Model | Executable, baseline Input directory and thread count. Existing baseline files supply calibration and initial state. |
| Prepared inputs | Selected retained preparation, with its period, scenarios and provenance. |

Select existing QC/correction instructions now; plan their editors separately.
Settings use backend validation and path rules. Saving configuration or a draft,
preparing and executing must not implicitly register instructions or scenarios.

Prepare inputs and Run All edit one shared operation draft. Changes in either
form immediately appear in the other; tab closure does not create a second
recipe or discard edits. Provide explicit draft saving through the existing JSON
draft contract, including incomplete settings. Execute MGB separately selects a
retained preparation and displays its frozen period, scenarios and provenance;
editing the shared draft does not alter that preparation.

Provide New run, Open, Save and Save As for the one active draft per project.
A New run always starts fresh, even if a matching saved draft exists. Fix its
reference immediately to the current hour in the selected operation timezone;
it does not advance while editing. Initialize available workspace paths, leaving
operational source/scenario choices unselected. Use the shared reference/horizon
format and its exact inclusive-hour convention (10 observed days and 5 forecast
days at midnight produce 241 and 121 labels); do not reinterpret day horizons.

Save drafts under `config/drafts/operation-20260720T000000-0300.json`, using the
reference's local timestamp and UTC offset. Remember at most one active saved
draft per project. Before reading that draft, establish the candidate run reference
from the current hour in the selected operation timezone, or an explicitly requested
reference. Never derive this candidate from the remembered file or path. On reopening,
restore it only for an exact candidate reference and IANA timezone match; never
fall back to the latest older draft. Validate the
content match rather than relying on the filename: zones can share an offset.
A changed reference changes the save destination and preserves the previous file.
Save As must also retain the reference anchor in its filename. Protect existing
destinations from accidental overwrite. Save/Discard/Cancel protects unsaved work
before New run or Open replaces a draft; failed saves keep it active. Incomplete
settings remain saveable once their reference and timezone identify the draft.
No automatic saving of operational edits. Legacy recipes retain their exact
coverage; convert to day horizons only when lossless, otherwise explain the
incompatibility and preserve the original for existing backend workflows.

Provide **Accept/Refresh selected inputs**, with visible readiness and changed-source
feedback, separately from monitoring Refresh/F5. F5 only refreshes monitoring.
Accepting inputs uses the existing explicit shared-backend API. Prepare and Run
All must not implicitly accept changed inputs or register instructions.

## Processing actions

| Action | Behavior |
| --- | --- |
| Prepare input data | Validate settings, freeze selected inputs and generate retained rainfall inputs without launching MGB. Make generated rainfall layers available automatically. |
| Execute MGB | Execute a selected retained preparation. Show which preparation is used; subsequent settings/source changes do not alter it. |
| Run all | Prepare current settings and execute MGB through one controlled backend workflow. |

Successful button-triggered execution automatically loads the completed run into
linked pages without changing their active tabs, focus or docking layout.
The map remains usable during processing. Show stage progress, per-scenario
status, expandable logs, Cancel and actionable validation/failure messages. Use
indeterminate progress where the backend supplies no fraction. Honor workspace
locking and cancellation/cleanup rules; success requires final manifest
persistence. Failure must not replace displayed completed results with incomplete
outputs. Preserve existing preparations and unrelated results.

Keep Jobs available in the sidebar for stage progress, per-scenario
status, scrollable logs, outcomes and Cancel. Switching or closing a runner form
must not stop its job or lose access to this status. Allow one active processing
job per workspace, with useful feedback when another is requested, while keeping
monitoring responsive and honoring backend workspace locks. Closing or collapsing
a pane is not cancellation. Project changes or application exit during processing
offer **Cancel job and leave** and **Stay**. Stay preserves the job and current
project. Cancel job and leave requests controlled cancellation, keeps the project
and Jobs status available until all workers/engines stop and cleanup finishes,
then resolves unsaved-work Save/Discard/Cancel before leaving. A cleanup failure
keeps the project available with an actionable error. Never detach an engine or
release its workspace before cleanup. If the job finishes before cancellation,
preserve its completed result and continue normal unsaved-work handling.

Later backend actions such as Fetch data can join the processing interface.
Do not build an additional plugin framework or inactive acquisition controls now.

## Acceptance and validation

- The ribbon presents grouped actions in the agreed compact style. Settings applies
  English/Brazilian Portuguese and System/Light/Dark immediately and remembers them
  app-wide. Labels, icons, focus indication and data displays remain readable.
- Map and Charts initially appear side by side; Data contains the existing table
  and location summaries. Dragging supports tab groups and all specified center
  arrangements through 2×2, with drop previews and resizable dividers. Only center
  pages dock; each page and form has a single instance.
- The sidebar resizes and collapses. Workspace and Interface Settings open in
  separate tabs alongside runner forms. Reopening preserves inputs.
- Map clicks update both chart fields where the explicit link is unique.
  Manual pairing, source-specific CSV rows and retained chart identity are verified.
  A stalled map read leaves chart loading responsive, and only the latest result applies.
- Per-project layout survives reopening, including preferences written by the
  former layout. Reset restores side-by-side Map and Charts with Files on the left,
  preserving selections, results and unfinished inputs.
- Prepare and Run All share one draft. Execute uses explicitly selected retained
  inputs. Preparation does not launch MGB; Run All performs both stages. Successful
  manual execution loads results without changing focus or docking layout.
- New run starts fresh with a fixed current-hour reference. New/Open/Save/Save As
  preserve dirty-work handling, exact reference+zone restoration and anchored
  filenames. A changed reference preserves the old file; destination collisions
  require explicit overwrite. Incomplete drafts round-trip without selecting inputs.
- Period controls have one reference and two day counts; displayed coverage matches
  CLI/backend labels exactly. Legacy coverage is never rounded during conversion.
- Accept/Refresh selected inputs shows readiness and source changes. F5 does not
  accept operational inputs; Prepare and Run All fail until required inputs are accepted.
- Leaving during processing offers Cancel job and leave / Stay. Cancellation waits
  for workers, engines and cleanup, then handles dirty work; failures keep the project.
- Jobs keeps progress, logs, outcomes and Cancel available independently of runner
  visibility. One active processing job is allowed per workspace. Monitoring stays
  responsive; cancellation and failure preserve unrelated data and usable results.
- Config saves validate supported fields and preserve destination bytes on failure.
  Save/Discard/Cancel protects unsaved configuration and drafts, including closed
  forms, before leaving a project or quitting. Failed saves and Cancel retain edits.

Run the required Rust and desktop checks in [AGENTS.md](../../AGENTS.md) when
implementing this increment. Add focused behavioral coverage for docking limits,
single instances, shared state, preference restoration, unsaved-work handling and
runner lifecycle. Inspect the layout interactively in both languages and themes,
including narrow windows and keyboard navigation. Compare runner results with the
same shared-library/CLI actions. Record Linux evidence separately from unverified
Windows/Wayland and distinguish software rendering from operator-hardware checks.

## Outside this increment

Do not add QC, forecast correction or other new operational pages, duplicate page
instances, floating dock windows, acquisition tools or the assistant. Those remain
under the broader plan and their own implementation specifications. With Map,
Charts and Data as the three pages in this increment, validate the four-area
capacity with a test fixture rather than adding an inactive product page.

## Backend references

- [Workspace configuration](../../docs/contracts/workspace.md)
- [Operation drafts](../../docs/contracts/drafts.md)
- [Preparation and execution](../../docs/contracts/operations.md)
- [Spatial data](../../docs/contracts/spatial.md)
- [Series and exports](../../docs/contracts/series.md)
