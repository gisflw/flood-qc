# Agent Guidance

This repository is moving toward a library-first architecture. Treat `src/mgb_ops`
as the primary product: an installable Python package whose modules and functions
can be used directly from notebooks, scripts, and data-flow style orchestration.

## Architecture Direction

- Keep the Python library as the core interface. Prefer reusable functions with
  explicit path, database, settings, and time inputs over behavior hidden behind
  command-line parsing or UI state.
- Keep GUI layer thin. `apps/ops_dashboard/` should handle Streamlit rendering and session state,
  then call library/dashboard-support functions.
- Do not add Streamlit, Folium component, or UI-session dependencies to core
  library modules. UI dependencies belong in `apps/ops_dashboard/` or in clearly
  transitional dashboard support code.
- Avoid coupling notebook-friendly functions to `argparse`, Streamlit session
  state, subprocess launch behavior, or print-only results. If a workflow needs
  those behaviors, put them in the CLI/app wrapper.
