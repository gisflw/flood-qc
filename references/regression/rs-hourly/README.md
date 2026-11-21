# RS hourly regression reference

The offline bundle is `/workspace/regression-rs-hourly`, outside Git. Its `case.json` fixes the operation, ordered scenarios, selected observations, corrections and complete application time axis. `manifest.json` records the source, interpreter, package, decoder and executable identities and retained-file hashes. `comparison.json` records the full capture/repeat comparison. The independently copied `frozen/` inputs contain the original history backup, native forecast grids, original GRIB downloads, runner inputs, engine and provenance; the capture does not read operational data.

The reference is July 20, 2026 00:00 in `America/Sao_Paulo`. The application axis contains 241 historical and 121 forecast hourly labels, in that order, for 8,649 explicitly identified mini-basins. The four scenarios are zero, raw ECMWF, raw NOAA and corrected ECMWF. The ordered six NULL replacements and 101 station-offset records are retained in the canonical observations fixture. The forecast correction is applied to the native ECMWF grid before interval splitting and working-grid interpolation.

Each capture writes canonical SQLite/NetCDF fixtures and staged checkpoints. Working-grid checkpoints retain f64 values before the legacy NetCDF packing; basin and combined f64 checkpoints retain the values actually presented to engine serialization after the legacy grid's packed readback. CHUVABIN and the engine streams are retained byte-for-byte. Canonical basin and station levels are rounded to integer centimetres from unquantized engine metre values, with station offsets applied before rounding. Legacy packed result exports are retained separately. The capture records engine text timestamps as provenance; result labels follow the Python application axis.

The engine overwrites the first 121 historical level rows in its combined historical stream. One history-only execution supplies all 241 historical level rows, and capture verifies historical flow and the unaffected historical-level suffix against every scenario. Forecast levels come from each original forecast stream. The numerical engine, geometry, calibration, ancillary files and initial state are carried unchanged.

## Reproduce

Use Linux and a clean `legacy_python` checkout at revision `15981642a7476fb583f63aeb89cdb1472cfd6b30`, with its Python environment and native dependencies recorded in the bundle manifest. Capture output paths must not already exist. These commands use no operational data:

```sh
/workspace/legacy_python/.venv/bin/python /workspace/regression-rs-hourly/capture.py --case /workspace/regression-rs-hourly/case.json --output /path/to/fresh/capture
/workspace/legacy_python/.venv/bin/python /workspace/regression-rs-hourly/capture.py --case /workspace/regression-rs-hourly/case.json --output /path/to/fresh/repeat
/workspace/legacy_python/.venv/bin/python /workspace/regression-rs-hourly/compare.py --actual /path/to/fresh/repeat --expected /path/to/fresh/capture
/workspace/legacy_python/.venv/bin/python /workspace/regression-rs-hourly/compare.py --self-check
```

For later Rust acceptance, compare the appropriate stage with the accepted `capture/` as expected, for example:

```sh
/workspace/legacy_python/.venv/bin/python /workspace/regression-rs-hourly/compare.py --actual /path/to/rust/results --expected /workspace/regression-rs-hourly/capture --stage observed
```

Stages are `observed`, `normalization`, `forecast`, `engine` and `results`. Comparison is exact for arrays, masks, metadata, timestamps, identities and raw streams; creation/history metadata is excluded. A mismatch produces JSON and a nonzero exit. Direct ecCodes decoding and cropping reproduced both saved native arrays exactly. Fresh capture and repeat matched, historical outputs agreed across all scenarios, and each nonzero scenario differed from zero in forecast precipitation, flow and level. The comparator self-check rejected value, timestamp, missingness, nonfinite and truncated-stream mutations.

The execution records preserve invocation, environment, status, stdout and stderr. The `snapshot_sha256` in result files hashes the concatenated hex SHA256 digests of PARHIG, Previsao, CHUVABIN, geometry and engine, in that order. Full file inventory and measured comparison details remain in the external bundle.

The saved state is an empirical regression input; its temporal compatibility and hydrological validity are not established by repeatability. This evidence is Linux-only. Windows, acquisition and state generation remain unverified. Rust saved-path numerical equivalence is recorded separately below. The [inundation evidence](inundation.md) records the additional HAND stream and regional raster checks.

The [Rust qualification record](rust-qualification.md) records driver/source
identity, saved-path comparisons and numerical caveats for the successor.
