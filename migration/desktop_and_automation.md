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

The CLI is the first interface and the basis for automation. Desktop controls
call the same Rust capabilities; they do not own hydrological rules or become
prerequisites for unattended execution. Choose the desktop framework during
implementation, keeping the application in Rust.

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
