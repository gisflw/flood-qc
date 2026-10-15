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

Support independent regional workspaces and sharing completed runs. Live
multi-operator editing is outside the agreed sharing requirement. Keep scheduling
and email configuration local, and prevent overlapping jobs from corrupting an
operation. Preserve accessibility and useful failure/status feedback in the UI.

## Evidence of completion

Perform the complete operation through the CLI and through the desktop on both
platforms. Verify scheduled execution and configured email delivery, including
failure reporting. The numerical results must agree across interfaces.
