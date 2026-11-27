# Shared registered inputs and run manifests

Status: planned direction; intention only. This plan records the intended
workspace organization and storage behavior for future implementation. The
[current architecture](../../docs/architecture.md),
[workspace contract](../../docs/contracts/workspace.md) and
[input-set contract](../../docs/contracts/inputs.md) still describe implemented
behavior.

## Intended outcome

Register each distinct input version once in shared workspace storage. Separate
run manifests retain the exact input references, selections and resolved
parameters needed to reproduce each attempt, without keeping another permanent
copy of its inputs for every run.

Return canonical station observations and their associated metadata to SQLite.
Keep gridded inputs as canonical NetCDF objects and retain other file-based
dependencies in their appropriate formats. SQLite selection must satisfy the
[dependency policy](../../docs/contracts/rules-exceptions.md#rust-safety-and-dependency-policy);
this plan grants no additional native-library exception.

The CLI and desktop share this behavior. The external MGB executable remains
the numerical engine.

## Intended workspace layout

The following tree illustrates roles and readable names. TOML extensions show
the format being evaluated, rather than an accepted format change. Exact object
names and time partitioning remain to be decided.

```text
workspace/
├── config.toml                         Routine workspace defaults
├── source/                             Regional source material
├── model/
│   ├── engine                          Installed numerical engine
│   └── Input/                          Editable baseline source
├── downloads/                          Original responses, where retained
├── assets/
│   ├── registry.sqlite                 Registrations and versioned tabular data
│   └── objects/
│       ├── imerg-early-v07-rs-20261126-r01/
│       │   └── grid.nc
│       ├── ecmwf-ifs-rs-20261127-00Z-r01/
│       │   └── grid.nc
│       ├── mgb-baseline-rs-r03/
│       │   └── Input/
│       └── geometry-rs-r02/
│           └── mini.gpkg
├── runs/
│   ├── 20261127-12.toml                 Run references, settings and outcome
│   └── 20261127-12-02.toml              Illustrative repeat at the same hour
├── work/
│   └── 20261127-12-02/                  Disposable execution working files
└── results/
    ├── active.toml                     Selects the successful result set
    └── 20261127-12/                     Validated results, rainfall and evidence
```

## Identity and metadata

Use human-readable, stable object IDs in folder names. Retain checksums as
metadata for verification and reuse. A changed input is a new version; existing
references continue to identify the original values.

Run manifest names use `YYYYMMDD-HH`. Repeated attempts must have distinct names
without overwriting earlier attempts. Decide whether the hour represents the
operational reference or execution time, its timezone convention, and the suffix
for repeated attempts. Preserve full timestamps and timezone information in the
manifest regardless of the filename.

Evaluate TOML consistently for application-owned persisted manifests and metadata,
including object descriptions, acquisition records and the active-result pointer.
Preserve explicit missing-value and exclusion meanings when evaluating the
format. Provider responses and external protocols retain their required formats.

## Storage and retention intentions

- Registration atomically publishes a complete, validated input update.
  Measurements, provenance and associated acquisition coverage must agree;
  the intended unit of registration remains to be specified.
- Repeated acquisition can reuse an existing input version. Revised measurements
  or products preserve earlier versions needed by retained runs.
- Each run identifies its exact observations, station metadata, offsets,
  reference levels, QC and ordered edits, gridded sources, geometry, baseline,
  calibration and initial state. It also records resolved processing and
  scenario parameters and the selected engine identity.
- Later acquisition, review or baseline editing cannot change the meaning of an
  existing run manifest. Run references select retained versions, rather than
  whatever data is currently latest.
- Shared inputs remain available while retained runs depend on them. Removing
  disposable working files must not remove registered inputs or run history.
- Generated forcing and writable engine inputs belong in `work/` and can be
  regenerated. Ordinary runs do not require a second persistent input snapshot
  area alongside the registry and objects.
- Validated results become active together. Failed or cancelled attempts retain
  their manifest and useful diagnostics while preserving the previous successful
  results. Keeping only the latest successful results is the proposed default;
  retention of additional results is a separate operator choice.
- Independent exports include the referenced data needed outside the original
  workspace. A manifest alone depends on continued access to its registered
  inputs. Matching engine and runtime requirements still apply to replay.

## IMERG as a registered observation source

Promote IMERG from standalone acquisition to registered gridded observations
available for operational run selection. Reuse the existing canonical NetCDF
grid representation, preserving precipitation units, interval bounds, validity
and source provenance. The [standalone acquisition contract](../../docs/contracts/acquisition.md#standalone-source-acquisition)
describes the current behavior; registration and operational processing are
future work.

Decide the registration interval and regional partitioning, handling of revised
or incomplete products, and how IMERG is selected alongside station-derived
rainfall. Daily regional objects are an option to evaluate. The shared grid
format does not by itself settle combination, precedence, QC or missing-data
policies.

## Remaining decisions and related work

Settle the metadata format, naming conventions, registration scope, retention
choices and transition policy for existing Parquet workspaces and independently
saved input sets. Preserve operational data and frozen reference evidence while
making those decisions. Returning station observations to SQLite does not imply
changing every other tabular input format.

The separate crate-refactoring task should preserve the ownership responsibilities
in the architecture while this storage direction is developed. Exact crate
boundaries and implementation sequencing are outside this workspace plan.

Future increments should follow the repository's
[planning and documentation rules](../../docs/contracts/rules-exceptions.md#documentation-rules),
keeping implementation specifications separate from this broader intention.
