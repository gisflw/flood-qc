# Architecture

## Overview

The target architecture is library-first. `src/mgb_ops` is the primary product:
an installable Python package that exposes reusable modules and functions for
notebooks, scripts, and data-flow style orchestration. The CLI and Streamlit
dashboard are operational adapters over that package.

The base remains local-first, file-oriented, and organized around reproducible
artifacts on disk:

- `<workspace>/data/history.sqlite` as the persistent history database;
- `<workspace>/data/runs/<run_id>.sqlite` as the closed context of an operational run;
- `<workspace>/mgb_runner/Input` and `<workspace>/mgb_runner/Output` as the local mirror of the MGB runner;
- `<workspace>/data/interim/` for collected or intermediate artifacts.

## Layer Model

### Core Library

These modules should remain usable directly from Python without CLI or UI state:

- `src/mgb_ops/common/`: shared contracts, paths, settings, logging, and time utilities;
- `src/mgb_ops/storage/`: SQLite bootstrap and repository contracts;
- `src/mgb_ops/ingest/`: collection and registration of observations and forecasts;
- `src/mgb_ops/model/`: preparation of MGB inputs, model execution, and output export;
- `src/mgb_ops/qc/`: QC and review rules, still incomplete in this phase.

Core domain functions in `storage`, `ingest`, `qc`, and `model` accept explicit paths, databases, settings, schema paths, asset bases, and times. They should return structured summaries or domain objects instead of depending on `argparse`, Streamlit session state, console output, process environment, `.env`, or workspace globals.

`mgb_ops.common` is the only library area that may provide convenience runtime helpers for resolving workspaces, settings, and `.env` values. CLI and dashboard code may use those helpers before calling explicit-input domain functions.

### Thin Interfaces

- `src/mgb_ops/cli/`
  Parses command-line arguments, resolves runtime paths/settings, calls library
  functions, and prints results.
- `apps/ops_dashboard/`
  Owns Streamlit rendering, session state, and dashboard interaction. It should
  call package functions rather than becoming the place where core workflow
  behavior lives.

### Dashboard Support

Dashboard-specific support code lives under `apps/ops_dashboard/support/`. These
helpers keep querying, map assembly, and forecast preview logic out of
`apps/ops_dashboard/app.py`, but they remain owned by the dashboard layer.

## Implemented Status

The repository currently provides:

- history and run schema bootstrap;
- operational ingestion of ANA observations;
- ECMWF grid ingestion, spatial clipping, and GRIB registration in the history database;
- hourly rainfall preparation for MGB from observations and ECMWF forecasts;
- real or dry-run MGB runner execution through library functions and the CLI wrapper;
- Streamlit dashboard for observations, MGB series, and ECMWF forecast preview/manual correction.

It does not yet provide the full end-to-end workflow for:

- operational INMET ingestion hardening;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete run assembly in `<workspace>/data/runs/<run_id>.sqlite`;
- operational reports.

## Architectural Decisions

### Python Library as the Primary Interface

New operational behavior should be implemented first as importable Python
functions. CLI commands and dashboard interactions should be thin consumers of
those functions.

This keeps notebook exploration, automated data flows, tests, CLI use, and GUI
use aligned around one implementation.

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
of station, variable, and state. This design is already in use in the history
database and dashboard.

### External Assets Outside the Database

Rasters, vectors, and MGB binaries remain outside SQLite. The database stores
metadata and relative paths. This applies to both ECMWF GRIB files and complete
MGB outputs.

### Streamlit as an Operational Adapter

Streamlit remains useful for operational triage and manual forecast correction,
but it is not the core architecture. Today the dashboard reads directly from:

- `<workspace>/data/history.sqlite`;
- MGB runner binaries;
- accumulated rasters in `<workspace>/data/interim/`;
- legacy spatial artifacts still used by the map.

Dashboard code should continue to keep Streamlit-specific concerns in
`apps/ops_dashboard/`.

### QGIS as a Complement

QGIS remains a complementary client for generated artifacts. The canonical
layout reserves `data/spatial/` for stable processed layers, although that
consolidation is still incomplete.

### Isolated MGB Runner

The MGB executable and artifacts remain isolated in `<workspace>/mgb_runner`,
under the responsibility of the user/region, while runner and preparation logic
lives in `src/mgb_ops/model/` and can be invoked directly from Python or through
the `mgb-ops` CLI wrapper.

## Target Architecture vs Current State

Some decisions remain canonical targets but are not fully materialized yet:

- a mature notebook-friendly function surface for every operational workflow;
- dashboard support code separated from future operational reporting;
- `data/spatial/` as the location for processed spatial assets;
- `data/timeseries/` as the location for processed operational series;
- `<workspace>/data/runs/` as an artifact actively used in the daily operational cycle;
- `.toml` as a possible future configuration format, still under evaluation.

In the meantime, the system still preserves and consumes some legacy artifacts,
especially in the spatial domain.
