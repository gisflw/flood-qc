# Self-contained run inputs

Replace operational SQLite completely with typed Parquet 2 tables, NetCDF grids,
and JSON manifests/pointers. Retain the pure Rust GeoPackage reader for spatial
inputs. There is no in-product conversion, compatibility mode, or SQL fallback.

TOML owns routine defaults; the shared API session owns experiments and atomic
job admission; each run freezes effective values, canonical inputs, mutable model
configuration and executable identity. Acquired canonical observations and
normalized forecasts are reusable. Raw downloads, derived scenarios and outputs
are disposable. The current input set can be saved independently. Replacement
and desktop departure require save/discard/cancel; unattended callers supply an
explicit policy before replacing unsaved inputs.

Scenarios have user IDs, an acquired forecast/zero/earlier scenario base, and
ordered hourly half-open period edits on the workspace grid. Edits support a
multiplier, translation, counterclockwise rotation and an optional output box.
Preparation and preview must share these semantics across every interface.

Acceptance requires source-independent regeneration with a matching installed
engine, canonical numeric equality for unchanged cases, failure/retention and
stale-session checks, no SQLite crate in either dependency graph, physical
Parquet 2 verification, and all repository Rust/desktop checks. Compare paired
release workloads against the pre-change implementation on the same host;
any repeatable slowdown requires correction. Keep the original frozen evidence
unchanged. Current platform qualification remains explicit.
