# Architecture

## Overview

The base is local-first, file-oriented, and organized around reproducible artifacts on disk. The main flow currently depends on:

- `<workspace>/data/history.sqlite` as the persistent history database;
- `<workspace>/mgb_runner/Input` and `<workspace>/mgb_runner/Output` as the local mirror of the MGB runner;
- `<workspace>/data/interim/` for collected or intermediate artifacts;
- `mgb-ops dashboard` as the operational interface entry point.

Components are separated by domain:

- `src/ingest/`: collection and registration of observations and forecasts;
- `src/model/`: preparation of inputs and MGB execution;
- `src/storage/`: SQLite bootstrap and contracts;
- `src/reporting/`: dashboard support and query products;
- `src/qc/`: QC and review rules, still incomplete in this phase.

## Implemented Status

The repository currently provides:

- history and run schema bootstrap;
- operational ingestion of ANA observations;
- ECMWF grid ingestion, spatial clipping, and GRIB registration in the history database;
- hourly rainfall preparation for MGB from observations and ECMWF forecasts;
- real or dry-run MGB runner execution through `mgb-ops model run`;
- Streamlit dashboard for observations, MGB series, and ECMWF forecast preview/manual correction.

It does not yet provide the full end-to-end workflow for:

- operational INMET ingestion;
- automatic QC for observations;
- manual correction of observed rainfall;
- complete run assembly in `<workspace>/data/runs/<run_id>.sqlite`;
- operational reports.

## Architectural Decisions

### SQLite as the Baseline

SQLite is the operational baseline to reduce external dependencies, keep backup simple, and preserve local auditability. The history and run schemas remain explicit in SQL.

### History + One File Per Run

The contract remains:

- `<workspace>/data/history.sqlite` for persistent history;
- `<workspace>/data/runs/<run_id>.sqlite` for the closed context of a run.

Bootstrap for this model is implemented. Complete operational run materialization is not finalized yet.

### Observations in Long Format

Observations are stored in long format, with one series per relevant combination of station, variable, and state. This design is already in use in the history database and dashboard.

### External Assets Outside the Database

Rasters, vectors, and MGB binaries remain outside SQLite. The database stores metadata and relative paths. This applies to both ECMWF GRIB files and complete MGB outputs.

### Streamlit as the Main UI

Streamlit remains the main interface for operational triage. Today it reads directly from:

- `<workspace>/data/history.sqlite`;
- MGB runner binaries;
- accumulated rasters in `<workspace>/data/interim/`;
- legacy spatial artifacts still used by the map.

### QGIS as a Complement

QGIS remains a complementary client for generated artifacts. The canonical layout reserves `data/spatial/` for stable processed layers, although that consolidation is still incomplete.

### Isolated MGB Runner

The MGB executable and artifacts remain isolated in `<workspace>/mgb_runner`, under the responsibility of the user/region, while runner and preparation logic lives in `src/model/` and is invoked by the `mgb-ops` CLI.

## Target Architecture vs Current State

Some decisions remain canonical targets but are not fully materialized yet:

- `data/spatial/` as the location for processed spatial assets;
- `data/timeseries/` as the location for processed operational series;
- `<workspace>/data/runs/` as an artifact actively used in the daily operational cycle;
- `.toml` as a possible future configuration format, still under evaluation.

In the meantime, the system still preserves and consumes some legacy artifacts, especially in the spatial domain.
