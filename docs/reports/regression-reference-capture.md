# Regression reference capture report

November 9, 2026. The RS hourly reference is complete: four scenarios, a fresh
repeat, and eight decoded NetCDF artifacts. All repeated arrays match exactly.
The [case record](../../references/regression/rs-hourly/README.md) links the
manifest, expectations, and reproduction instructions. The full external bundle
is `/workspace/regression-rs-hourly-r1`.

## Delivered evidence

| Scenarios | Each decoded artifact |
| --- | --- |
| Zero rain, raw ECMWF, raw NOAA, corrected ECMWF | 362 hourly timestamps × 8,649 mini-basins; precipitation, flow, and level |

Each scenario has 241 observed and 121 forecast steps, from July 10 00:00 to
July 25 01:00 in `America/Sao_Paulo`. The bundle retains frozen inputs, complete
history, explicit registry bindings, six NULL observation replacements, working
grids, transformed ECMWF grids, prepared inputs, raw flow/level streams, decoded
results, execution logs, checksums, and full-array comparisons.

Capture preparation aligns stale forecast metadata in disposable inputs and
joins separate engine output streams for decoding. The engine overwrites the
start of its observed-level file; an independent recovery run snapshots those
real levels before overwrite. The retained raw streams are unchanged. The bundle
contains the small interception shim and exact reproduction commands. Neither
the legacy Python package nor the numerical engine was changed.

## Review and validation

Implementation was delegated to a new GPT-6.1 Terra agent at medium effort with
Ponytail full. The parent reviewed the capture code and independently checked
all eight artifacts against raw outputs and recovered levels: complete time and
cell alignment, 241/121 segment labels, finite arrays, identical observed
prefixes, and exact capture/repeat equality. Forecast forcing and results differ
between scenarios. One fix round completed provenance and execution records.
All 3,749 original artifact sizes and SHA-256 checksums passed independent
verification. The archive was later compacted by retaining unchanged inputs and
byte-identical repeat files once and removing the unused flood map, ancillary
engine outputs, repeat grids, and disposable decoding staging. Fresh capture and
repeat runs were compared with the archived reference after compaction.
Completion left all 29,834 operational source files' recorded sizes, modification
times, and modes unchanged.

The compact bundle contains 152 files (about 533 MiB). A complete fresh capture
and repeat reproduced the archived arrays and selected raw checkpoints; a modified
flow value was also checked to ensure the comparison rejects a mismatch.

NetCDF packing uses a 0.001 scale factor. Measured raw-to-decoded error is
recorded separately from exact repeat equality; float32 flow packing reaches
about 0.00224 m³/s. Full-precision raw outputs are retained.

## Limits and cleanup

Saved initial-state files are retained unchanged, but consumption and temporal
compatibility remain unverified. This capture establishes Linux reference
behavior; Windows, Rust equivalence, and hydrological calibration validation
were not performed.

The original SQLite backup changed the operational shared-memory sidecar's
modification time; its pre-backup bytes were not hashed. Completion reused the
frozen snapshot and did not reopen that source database. The superseded failed
attempts, failed-case record, and temporary probes were removed after validating
the final bundle.
