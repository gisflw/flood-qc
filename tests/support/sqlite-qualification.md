# SQLite qualification harness

This opt-in harness uses Turso 0.9.0-pre.6 with defaults disabled. Its append-only
schema qualifies SQL primitives, not a supported workspace or production schema.
The architecture gate checks isolation, not dependency-language compliance.

## Run

From the repository root:

~~~sh
cargo test --locked --features sqlite-qualification --test sqlite_qualification -- --ignored --skip sqlite_parquet_comparison
cargo test --locked --release --features sqlite-qualification --test sqlite_qualification -- --ignored --nocapture
cargo clippy --locked --features sqlite-qualification --all-targets -- -D warnings
~~~

The benchmark requires release mode. The child test is a subprocess fixture,
inactive without its parent's environment. Databases and logs use
runs/sqlite-qualification/. Child readiness/exit waits are bounded, and guards
kill and reap children on failure. Every database open holds the exclusive OS
lock until all handles drop. Pollster drives safe async calls with query timeouts.

Five paired benchmark repetitions alternate backend order in fresh processes.
Each backend receives 440 stations with 362 samples each (159,280 rows), queries
the same 32-station/72-hour window, retains missing samples, and appends 24 samples
to one station. Nine warm reads yield a median. Selected identities, timestamps,
masks and values are compared exactly; digests must match across processes.
SQL reads revision 1 after the append. This does not measure overlapping
historical revisions or full forcing preparation. Fresh-handle reads do not
clear the operating-system page cache. Linux peak RSS includes fixture and
validation allocations. Measurements use runs/sqlite-qualification/benchmark.json.
Both rolling-update timings include opening their store. The Parquet reader
uses the existing full selection/validation API, while SQL uses the minimal
prototype's sample query. These timings compare matched selected values through
those paths, not equivalent complete operational APIs or an isolated engine
algorithm. They cannot establish end-to-end forecast performance.

## Pinned dependency audit

Cargo.lock owns exact source checksums. Review source and features when changing
the candidate; a lockfile includes dormant optional and custom-cfg packages.

- Turso defaults off omit mimalloc, FTS, SIMD and sync/network features.
  Its binding selects SDK encryption and pure-Rust crypto.
- Aegis 0.9.22 enables pure-rust. Its build script returns before compiler
  detection or C compilation. Dormant C sources remain in its source archive.
- SDK build dependencies include bindgen 0.69.5 and clang-sys 1.9.1 runtime
  loading. SDK never invokes bindgen; its build script returns outside Windows.
  This clang-sys configuration does not link libclang.
- Windows SDK resource compilation uses the platform resource compiler;
  extension support links OS advapi32. Windows build/linkage remain untested.
- Linux io-uring 0.7.16 has no features, disabling its bindgen-generation path.
  Its selected implementation provides an OS syscall interface.
- Other examined candidate-only build scripts generate Rust constants/tables,
  compiler/platform configuration or metadata: bigdecimal, branches, crc32c,
  ICU4X data, memoffset, prettyplease, rustix and uncased. Core/ext build scripts
  likewise compile no C application algorithms. Record effective ICU4X and
  BigDecimal environment overrides when reproducing generated configuration.
- Core contains a dynamic library loader. SDK starts with extension loading
  disabled; SQL checks that flag before loading. The harness verifies rejection
  of load_extension and never enables loading.

Inspect default, desktop and qualification metadata separately, filtering for
Linux and Windows. Inspect actual Linux executable linkage as additional
evidence, recognizing existing native deviations in the repository rules.

## Limits

Process kills at registration/commit boundaries do not cover every possible
write interleaving or physical power loss. A commit-boundary kill accepts either
complete revision; an acknowledged commit must survive. The exclusive-open
protocol does not qualify experimental multiprocess WAL. Source scanning and
linkage are bounded evidence, not proof about every runtime path.
The [migration checklist](../../docs/migration/summary.md) owns results and
adoption status.
