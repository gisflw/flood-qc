# Agent Guidance

This repository is moving toward a library-first architecture. Treat `src/mgb_ops`
as the primary product: an installable Python package whose modules and functions
can be used directly from notebooks, scripts, and data-flow style orchestration.

## Architecture Direction

- Keep the Python library as the core interface. Prefer reusable functions with
  explicit path, database, settings, and time inputs over behavior hidden behind
  command-line parsing or UI state.
- Keep CLI and GUI layers thin. `src/mgb_ops/cli/` should parse arguments,
  resolve runtime paths/settings, call library functions, and print results.
  `apps/ops_dashboard/` should handle Streamlit rendering and session state,
  then call library/dashboard-support functions.
- Do not add Streamlit, Folium component, or UI-session dependencies to core
  library modules. UI dependencies belong in `apps/ops_dashboard/` or in clearly
  transitional dashboard support code.
- Treat `mgb_ops.reporting.ops_dashboard_*` as dashboard support, not as the
  future public reporting interface. Operational publication/report generation
  remains a separate library capability to mature later.
- Avoid coupling notebook-friendly functions to `argparse`, Streamlit session
  state, subprocess launch behavior, or print-only results. If a workflow needs
  those behaviors, put them in the CLI/app wrapper.

## Current Layer Model

- Core library: `mgb_ops.common`, `mgb_ops.storage`, `mgb_ops.ingest`,
  `mgb_ops.qc`, and `mgb_ops.model`.
- Transitional dashboard support: `mgb_ops.reporting.ops_dashboard_data`,
  `mgb_ops.reporting.ops_dashboard_forecast`, and
  `mgb_ops.reporting.ops_dashboard_map`.
- Thin interfaces: `mgb_ops.cli` and `apps/ops_dashboard`.

## Working Conventions

- Preserve local-first operation: regional workspaces own `data/`, `logs/`, and
  `mgb_runner/`; SQLite remains the operational baseline.
- Keep docs aligned with the library-first direction. If code changes introduce
  a new workflow, document the Python function/module path first and CLI/UI
  invocation second.
- Keep tests focused on stable library behavior and layer boundaries. The
  existing package-boundary tests are intentional guardrails.
