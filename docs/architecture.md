# Architecture

## Overview

The target architecture is library-first. `src/mgb_ops` is the primary product:
an installable Python package that exposes reusable modules and functions for
notebooks, scripts, and data-flow style orchestration.

The base remains local-first, file-oriented, and organized around reproducible
artifacts on disk:

- `<workspace>/data/history.sqlite` as the persistent history database;
- `<workspace>/data/runs/<run_id>.sqlite` as the closed context of an operational run;
- `<workspace>/mgb_runner/Input` and `<workspace>/mgb_runner/Output` as the direct input and output paths used by the MGB runner;
- `<workspace>/data/source/` for immutable user-provided inputs;
- `<workspace>/data/downloads/` for raw provider artifacts and normalized fetch outputs;
- `<workspace>/data/cache/` for disposable analysis and intermediate artifacts;
- `<workspace>/data/processed/` for reusable derived outputs;
- `<workspace>/data/reports/` for report and publication artifacts.

## Layer Model

The package is split by responsibility rather than by user interface:

| Module | Architectural boundary |
| --- | --- |
| `mgb_ops.common` | Shared domain types, paths, settings, runtime conveniences, logging, and time utilities |
| `mgb_ops.assets` | Contracts, validation, and I/O for external file formats, including packaged SQL schemas, canonical forecast NetCDF, and spatial GeoPackage layers |
| `mgb_ops.adapters` | Provider-specific acquisition and translation, such as ANA, INMET, and ECMWF |
| `mgb_ops.storage` | SQLite bootstrap, repositories, observed CSV persistence, and external-asset registration/resolution |
| `mgb_ops.analysis` | Reusable read-only queries, projections, interpolation, resampling, and aggregation |
| `mgb_ops.edit` | Forecast correction operations and correction persistence |
| `mgb_ops.qc` | Validation rules, checks, and structured QC results |
| `mgb_ops.model` | MGB input preparation, execution, and output production |
| `mgb_ops.workflows` | Use-case orchestration across adapters, storage, assets, and model capabilities |
| `apps/ops_dashboard` | Panel rendering, callbacks, session state, caching, and UI-specific presentation |

An asset module owns the shape and serialization of a file, not its database
registration or operational use. Storage owns persistence and registry lookups.
Analysis owns read-only derivation from explicit inputs. Thus GeoPackage
validation/loading and the forecast NetCDF contract live in `assets`, while
rainfall interpolation and grid accumulation live in `analysis`.

The existing `analysis.forecast` and `analysis.timeseries` surfaces combine
explicit read-only loading with computation. They remain supported read-only
analysis APIs; database writes and registry ownership still belong to
`storage`.

Core domain functions accept explicit paths, databases, settings, schema paths,
asset bases, credentials, and times. They return structured summaries or domain
objects instead of depending on console output, process environment, `.env`, or
workspace globals. `mgb_ops.common` is the only library area that may provide
convenience runtime helpers for resolving workspaces, settings, and `.env`
values.

### Dependency Direction

Apps call workflows or focused library APIs. Workflows compose provider
adapters and domain modules. Adapters, storage, analysis, edit, QC, and model
code may depend on common types and asset contracts, but library code must not
depend on the app. Provider access stays out of storage and assets, UI state
stays out of the library, and peer-module cycles should be avoided.

Subprocess execution is a model capability when exposed through an explicit
plan and structured result. CLI parsing, Panel state, and interface-only output
remain in wrappers.

## Implemented Status

The repository currently provides:

- history and run schema bootstrap;
- operational ingestion of observed ANA and INMET data through normalized per-station CSV artifacts and storage-owned SQLite loading;
- forecast grid ingestion with ECMWF defaults, spatial clipping, and generic asset registration in the history database;
- hourly rainfall preparation for MGB from observations and ECMWF forecasts;
- real or dry-run MGB runner execution through library functions.

It does not yet provide the full end-to-end workflow for:

- operational INMET ingestion hardening;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete run assembly in `<workspace>/data/runs/<run_id>.sqlite`;
- operational reports.

## Architectural Decisions

### Python Library as the Primary Interface

New operational behavior should be implemented first as importable Python
functions. This keeps notebook exploration, automated data flows, and tests
aligned around one implementation.

### SQLite as the Baseline

SQLite is the operational baseline to reduce external dependencies, keep backup
simple, and preserve local auditability. The history and run schemas remain
explicit in SQL.

### History + One File Per Run

The contract remains:

- `<workspace>/data/history.sqlite` for persistent history;
- `<workspace>/data/runs/<run_id>.sqlite` for the closed context of a run.

Bootstrap for this model is implemented. Complete operational run
materialization is not finalized yet.

### Observations in Long Format

Observations are stored in long format, with one series per relevant combination
of `station_id`, variable, and state. `station_id` is a canonical string
`provider:station_code`, such as `ana:74100000` or `inmet:A801`. Provider
fetchers write normalized CSV files with `station_id`, `provider_code`,
`station_code`, `observed_at`, `variable_code`, `value`, and `state`; the shared
CSV importer is the common persistence path into history SQLite.

### External Assets Outside the Database

Rasters, vectors, and MGB binaries remain outside SQLite. The database stores
metadata and relative paths. Operational forecast grids are registered as
canonical CF-style NetCDF files; ECMWF GRIB2 remains adapter-internal source
material and is not the registered history asset. Complete MGB outputs also
remain outside SQLite.

### QGIS as a Complement

QGIS remains a complementary client for generated artifacts. The canonical
layout reserves `data/processed/` for stable derived outputs, including
processed spatial layers, although that consolidation is still incomplete.

### Isolated MGB Runner

The MGB executable and artifacts remain isolated in `<workspace>/mgb_runner`,
under the responsibility of the user/region, while runner and preparation logic
lives in `src/mgb_ops/model/` and can be invoked directly from Python.

## Target Architecture vs Current State

Some decisions remain canonical targets but are not fully materialized yet:

- a mature notebook-friendly function surface for every operational workflow;
- operational reporting as an importable library capability;
- `data/processed/` as the location for reusable derived outputs;
- `data/cache/` as the location for disposable analysis and intermediate artifacts;
- `<workspace>/data/runs/` as an artifact actively used in the daily operational cycle;
- `.toml` as a possible future configuration format, still under evaluation.

In the meantime, the system still preserves and consumes some legacy artifacts,
especially in the spatial domain.
