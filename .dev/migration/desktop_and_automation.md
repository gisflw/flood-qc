# Desktop and automation

## Owns

- A Rust desktop interface on Windows and Linux over the shared core.
- Operational maps, review/editing, calibration editors, execution status,
  result comparisons, bulletins, and completed-run management.
- Scheduled operations and email distribution, also accessible through the CLI.

## Starting references

Python: `apps/ops_dashboard/` for the current operational flow and presentation.
VB.NET forms and plugins for the added tools; `Form20`, `Form16`, and `Form17`
for scheduling and email behavior.

## Implementation direction

The CLI is the first interface and the basis for automation. CLI, desktop, and
scheduler use shared app workflows in the library for configuration/path
resolution, selection, execution snapshots, scenario coordination, failure
handling, export, and run outcomes. Interface controls may call intended
public core APIs but do not duplicate operational lifecycle rules. Hydrological
algorithms stay in analysis/model; database and file-lifecycle implementations
stay in store. The [initial GUI draft plan](../plans/desktop_gui.md) records the
selected framework direction and first-milestone behavior.

At execution startup, freeze input values or immutable versions. Model stages
canonical outputs; app records independent run/scenario outcomes and output paths
in the generated per-run JSON manifest for GUI consumption. A failed scenario retains
diagnostics and successful siblings' outputs; other runs are unchanged. Operators
may copy files elsewhere for long-term storage; add no save flags or retention
manager. These rules apply equally to interactive and scheduled
operations. Reusable report/chart rendering belongs in analysis's private
submodules; interactive widgets and GUI state belong in the interface.

Introduce a working desktop interface after the first shared preparation,
execution, and decoded-results path is available, before implementing all core
functionality. Check framework and platform feasibility early, then extend the
GUI alongside each added capability. Scheduling and email follow the operations
they automate.

Support independent regional workspaces and sharing completed runs. Live
multi-operator editing is outside the agreed sharing requirement. Keep scheduling
and email configuration local, and prevent overlapping jobs from corrupting an
operation. Preserve accessibility and useful failure/status feedback in the UI.

## Evidence of completion

For each available operation, verify CLI and GUI behavior against the same shared
core results. Complete this check incrementally on Windows and Linux. Verify
scheduled execution and configured email delivery, including failure reporting,
as automation is added. The completed interfaces must cover the functionality
checklist; the GUI is not deferred until that checklist is complete.
