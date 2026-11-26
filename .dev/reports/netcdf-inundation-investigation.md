# Inundation and NetCDF investigation

Investigated merged main `18db31c` on Linux, with netCDF-C 4.9.3 and HDF5 1.14.5.
Production behavior and frozen reference artifacts are unchanged.

## Confirmed descriptor inheritance failure

The native mask descriptor lacks `FD_CLOEXEC`. A child executed while the writer
is open inherits it and keeps its HDF5 file lock after the parent closes the
writer. Native read-only reopening then returns `-101`; reopening succeeds after
the child exits. A temporary probe using the application's actual mask writer
and Rust `Command` reproduced this deterministically. An independent C probe
failed in all three unprotected runs and succeeded in all three close-on-exec
control runs, including while the executed child remained alive.

The intermittent debug-library failure occurred in 2 of 10 unmodified parallel
runs. Temporary error context localized matching failures to native reopening
for payload validation, after successful mask creation, writes and close.
Individual intermittent failures were not traced to a particular child PID.
Fifteen pre-merge `7120d5f` debug-library repeats passed; this does not establish
that the native descriptor issue was introduced by the merged PRs.
Twenty release mask-suite repeats, five release library repeats at 16 threads,
and five at default parallelism passed. A smaller C-only concurrency probe also
passed 1,700 file cycles despite HDF5 missing-file diagnostics. Those diagnostics
alone are insufficient to identify the failed operation.

The current CLI mask-generation path does not launch children, and the desktop
only reads masks. The parallel unit-test harness permits overlap between native mask I/O and
unrelated child launches. The four mask lifecycle checks
now run in a separate integration-test executable with the same synthetic
values and assertions. Ten default-parallel mask-suite repeats passed, as did
the full default-parallel suite. The native library and production behavior
are unchanged.

A production descriptor-inheritance repair, if needed for an in-process caller,
would require atomic prevention at native file creation or isolation from child
spawning. A later `fcntl` alone leaves a race. Test isolation does not repair the
native library limitation.

## Regression coverage

At investigation time, the ignored [saved RS regression](../../tests/rs_saved.rs)
checked engine streams and assembled `flood_source.nc` depths without generating
or comparing masks. A separate [RS mask regression](../../tests/rs_inundation.rs)
now generates all four scenarios from frozen depths and compares every native
and overview pixel at all 362 times against an independent GDAL/NumPy oracle.
The [corrected-rule reference](../../references/regression/rs-hourly/inundation.md#automated-corrected-rule-reference)
records source identities and successful Linux qualification. Historical counts
for the earlier HAND rule were not reused as goldens. An independently observed
geographic footprint remains outside this numerical regression evidence.

## Why two NetCDF implementations exist

`netcdf` 0.12.1 is the direct native wrapper used for incremental, compressed,
chunked mask writes and native payload validation. Terrio brings `netcdf-reader`
0.12.1 for pure Rust scientific-array reading; Terrio also writes ordinary arrays.
This is not two versions of one crate or two native netCDF libraries. The native
wrapper already holds a global mutex around C calls; that cannot stop descriptor
inheritance across `exec`.

The pinned Terrio writer materializes the complete NetCDF file as a byte vector
and sets HDF5 deflate to none. Replacing the native tile writer with that API
would sacrifice bounded-memory compressed mask output. The recorded regional
case has 58.6 million eligible cells across 362 times, already over 21 GB of
one-byte mask values before counting other cells, overviews and encoding copies.

The writer capability gap is the reason for the
[scoped NetCDF exception](../../docs/contracts/rules-exceptions.md#approved-c-backed-exceptions).
Retiring it requires a qualified streaming/compressed Rust writer. Consolidating
ordinary scientific I/O on the native stack would exceed that exception. No
replacement or performance qualification was performed in this investigation.

## Wider dependency audit

The merged-main dependency graph also includes C-backed Zstandard through
Parquet, and AWS-LC and ring through TLS. Those implementations need review and
replacement under the same policy. The preceding operational SQLite dependency
was removed by the merged Parquet change. OS and graphics-driver bindings are
permitted through safe APIs; package-local `unsafe_code = "forbid"` does not
enforce dependency implementation language. Manifest native-link declarations
help identify candidates, but their presence or absence alone is not a complete
source audit.
