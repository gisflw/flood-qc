# Rust saved-path qualification

The accepted reference and its scientific limits are described in the
[case record](README.md). Qualification used independent disposable copies;
the external frozen bundle and legacy/runtime inputs were preserved.

## Driver and source identity

GRIB2 and NetCDF/HDF5 use Terrio 0.8.0 at Git revision
`29fd6e78ba290e37e58aac16df61883e8ed4aed3`, default features disabled, features
`grib` and `netcdf-hdf5`. `Cargo.lock` retains the complete dependency identity.
No GDAL runtime is used by the Rust path.

| Saved source | SHA256 | Decoded bands | Cropped cells per band | Peak RSS, KiB |
| --- | --- | ---: | ---: | ---: |
| ECMWF | `6a46222b3d1ca77009b397f8b995792442b2ba70b07fbe55621b7e001ce58893` | 85 | 33,292 | 402,164 |
| NOAA | `2c01f36868b35d92c1aa1b8a04cd0b9bdd6135cdc9a27c8dcc2c5ec10d57f7c4` | 162 | 32,724 | 80,344 |

Every cropped source value, mask and adjacent difference matched ecCodes exactly;
maximum absolute, relative and adjacent-difference errors were zero. Cycle,
parameter codes, surfaces, statistical method, time units and start/end leads
were checked for every band. ECMWF local parameter 193 requires the explicit
qualified table interpretation. The adapter requires meteorological discipline 0,
the qualified local-table token, supported single statistical range and surface.
Synthetic bitmap coverage keeps valid zero separate from missing rainfall.

The driver applies the first grid to later bands and can silently pad/crop a
changed grid. The adapter therefore validates GRIB2 framing and equal raw grid
sections before decoding. Changed-grid, malformed/truncated and unsupported
multi-field messages fail. Regression mutations also reject foreign discipline
and local-table/product metadata.

## Scientific comparisons

Full normalized native arrays matched exactly: ECMWF `84×163×203` and NOAA
`81×162×202`. Raw ECMWF, raw NOAA and corrected ECMWF each matched the reference
working grid `121×83×102` and basin array `121×8649`. All 41 transformed corrected
ECMWF frames matched. The correction uses +8 latitude pixels, −12 longitude
pixels, 45° rotation, multiplier 1.5 and hours 0–126, before hourly splitting.

Observed preparation matched all working-grid `241×83×102` and basin
`241×8649` values, validity masks, coordinates, ordered mini IDs, decoded local
labels and bounds. All 2,415 selected raw/effective rows, six QC findings and six
ordered NULL instructions matched. The saved case had no unresolved ranking ties
or packed-grid readback difference. General forcing remains unpacked f64.

Power-two IDW uses direct multiplication for the squared distance; `powf(2.0)`
introduced small rounding differences. The retained synthetic bit-pattern test
protects the qualified arithmetic. Comparisons did not relax numerical tolerance.
Time origins may differ between files; decoded labels and bounds must agree.
Rust source IDs and processing records describe Rust normalization and are not
expected to reproduce Python implementation metadata.

The captured observations fixture had 31 creation labels using a space separator.
A disposable copy changed only those separators to `T` for the canonical reader.
The source SHA256 was
`f0c25466e38afcd95d8087bffe8436e89ca75d52a683b2ff95fc136197502f4b`;
the canonical copy SHA256 was
`12c52684ae666239b501dc9c9745b94b154606e0d14a413e74ea631878d9db73`.
The frozen converter SHA256 was
`995224ee916045b31432d6f956134896384ea372c33fed6de93647b088e3883d`.
Its separate level conversion recorded 91,063 changes with maximum change
0.5 cm, using nearest integer centimetres with ties to even.

## Execution and source-independent repeat

The public Rust normalization, preparation and execution workflows ran the real
retained engine for zero, raw ECMWF, raw NOAA and corrected ECMWF, then repeated
execution from the same prepared input after the disposable original observation
and forecast catalogs, forecast artifacts, baseline Input and executable were
removed. Both executions completed all four scenarios. Each run used a shared
history-only execution to recover the overwritten historical level prefix.

An independent comparison checked every joined f64 forcing value, full basin
results `362×8649`, station results `362×101`, all variable masks, mini IDs,
coordinates, offsets, ordered station identities, decoded local times and rainfall
bounds. Every comparison was exact. CHUVABIN and each of the four original
scenario streams matched the frozen capture byte for byte, in both runs. The
engine SHA256 was
`263381e450484e5bc345e269f21c54e84d89afdff97ca2d9752eb951907f88eb`.
Canonical raw and station levels were compared as signed i32 centimetres.

Rust metadata edits preserve baseline whitespace/comments and replace numeric
tokens; the Python capture rewrites whole numeric rows. Qualified timing fields
agree, but metadata file bytes and their derived snapshot hashes can differ.
Each Rust snapshot hash was independently recomputed using the five-file recipe
in the case record. Rust's geometry hash identifies serialized canonical geometry;
the capture's geometry hash identifies the original MINI.gtp bytes. Ordered IDs
and coordinates matched, and the original geometry identity is retained among
manifest input references. Implementation-specific artifact names, source IDs
and processing metadata are not numerical comparisons.

Reproduce the complete explicit real-case test on Linux:

```sh
MGB_RS_REFERENCE=/workspace/regression-rs-hourly cargo test --locked --release --test rs_saved -- --ignored --nocapture
```

It fails on missing prerequisites rather than skipping, and retains its disposable
qualification directory for inspection. Remove that directory after review.
The equivalent debug command is supported but slower.

## Retained checks and limits

Synthetic tests retain driver-independent contracts for f64 rainfall, i64
labels/bounds/IDs, u8 masks, integer centimetre results and metadata. They reject
contradictory masks and invalid integer payloads. Tiny generated GRIB fixtures
include reproducible generation commands and hashes in their fixture README.
Routine tests need no provider, regional workspace, Python or MGB installation.

These checks establish Linux numerical equivalence for the stated saved case.
They do not establish acquisition, Windows support, new-state compatibility or
hydrological validity. The saved state remains an empirical regression input.
