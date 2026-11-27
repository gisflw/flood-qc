# Rust SQLite qualification

This increment qualifies a foundation for the broader
[registered inputs and runs plan](../plans/registered-inputs-and-runs.md).
It does not implement its production registry, migration, or operating layout.
The migration checklist owns current results and remaining gaps.

## Candidate and isolation

Pin Turso 0.9.0-pre.6 with defaults disabled, enabled only through the explicit
sqlite-qualification feature and consumed by its integration harness.
Keep application sources free of Turso references and preserve the forbidden
unsafe-code lint. Audit default, desktop and qualification graphs, including
Windows, and inspect sources/build scripts and actual Linux linkage separately.
No native application-library exception is granted.

The [harness notes](../../tests/support/sqlite-qualification.md) own commands,
schema mechanics, source-audit findings and limits. Disposable evidence belongs
under runs/sqlite-qualification/.

## Required behavior

- Demonstrate retained revisions of measurements, metadata, ordered edits,
  exclusions, offsets, reference levels, provenance and full/partial coverage.
  Preserve zero, missing versus absent rows, signed integer levels, fractional
  values, units and timezone information.
- Register one station acquisition atomically with its provenance and coverage,
  including successful empty responses. Roll back invalid batches and explicit
  or dropped transactions without changing earlier successful station commits.
- Hold an exclusive cross-process OS lock for the complete database lifetime,
  including readers. Drop all database handles before the guard. Verify
  contention and reopening after close and process death.
- Enable and verify foreign keys and synchronous FULL on every connection.
  Keep dynamic extension loading, experimental multiprocess WAL and MVCC off.
- Kill subprocesses during partial registration, before commit, at its boundary,
  and after acknowledged commit. Recover complete revisions with valid
  integrity and foreign keys, never mixed measurements and coverage.
- Compare release ingestion, fresh-handle and warm window reads, rolling updates
  and historical selection against equivalent Parquet workloads in separate
  processes. Verify selected values and report runtime and peak memory.

## Acceptance and failure

Run formatting, default Clippy/tests, architecture checks and qualification
Clippy/tests. Record failed criteria or repeatable performance regressions in
the migration checklist; do not promote or silently substitute the engine.
Windows graph inspection does not establish Windows execution/linkage.
Process termination does not establish physical power-loss durability.
Follow the repository's final release build and startup-validation handoff.

The broader plan retains the selected TOML format, local reference-hour run
names, retention policy, explicit conversion, and IMERG partition/selection
decisions. Production implementation and crate restructuring remain separate.
