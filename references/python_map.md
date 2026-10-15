# Python reference map

Current source: `mgb_ops/`. Frozen reference: `legacy/python` once created.
Paths below are relative to that repository root.

The operational data flow is the migration's starting point. Existing code and
tests are stronger evidence than status paragraphs in older documentation.

| Responsibility | Main reference paths |
| --- | --- |
| CLI and operation sequence | `apps/cli/__init__.py` |
| Workspace paths and configuration | `src/mgb_ops/config/` |
| Provider acquisition | `src/mgb_ops/adapters/`, `src/mgb_ops/workflows/observed.py`, `src/mgb_ops/workflows/forecast.py` |
| Persistence and artifact contracts | `src/mgb_ops/assets/`, especially `sql/`, `history.py`, `observations.py`, `current_run.py`, `spatial_grid.py` |
| Rainfall QC and manual review | `src/mgb_ops/qc/`, `src/mgb_ops/assets/effective_observations.py`, `apps/ops_dashboard/views/observed_qc.py` |
| Forecast transformations | `src/mgb_ops/edit/forcing.py`, `apps/ops_dashboard/views/corrections.py` |
| Run preparation and scenarios | `src/mgb_ops/workflows/scenarios.py`, `src/mgb_ops/workflows/scenario_orchestrator.py` |
| MGB inputs, execution, and output decoding | `src/mgb_ops/model/` |
| Basin, time-series, and rainfall analysis | `src/mgb_ops/analysis/` |
| Level alignment | `src/mgb_ops/analysis/adjust.py` |
| Reports | `src/mgb_ops/workflows/reporting.py`, `src/mgb_ops/assets/report.py`, `docs/reports.md` |
| Desktop feature reference via current dashboard | `apps/ops_dashboard/views/`, `apps/ops_dashboard/state.py`, `apps/ops_dashboard/services/` |
| Behavioral checks | `tests/api/`, `tests/integration/`, `tests/engine/`, `tests/unit/`, `tests/ui/`, `tests/e2e/` |

## Runtime reference

`rs_hydro_forecast/` is a separate regional workspace, not the library source:

- `config/custom.yaml`: regional settings.
- `data/source/`: station inventory, reference levels, floods, and GeoPackage.
- `data/history.sqlite`: observation history and registered assets.
- `data/cache/current_run.sqlite`: saved operation and review instructions.
- `mgb_runner/`: selected executable and current input/output layout.
- `generate_taquari_report.py` and `data/reports/`: bulletin examples.

The inspected published run refers to July 20, 2026, although acquisition extends
into October. Treat it as an example case, not proof of current forecast freshness.
