# SQLite qualification harness

The application uses Turso 0.9.0-pre.6 with defaults disabled for its SQLite
registry. The additional opt-in harness exercises transaction and process
failure behavior independently of the production schema. The dependency audit
below records source inspection, not a blanket native-library exception.

## Run

From the repository root:

~~~sh
cargo test --locked --features sqlite-qualification --test sqlite_qualification -- --ignored
cargo clippy --locked --features sqlite-qualification --all-targets -- -D warnings
~~~

The child test is a subprocess fixture, inactive without its parent's
environment. Databases and logs use `runs/sqlite-qualification/`. Child waits
are bounded and guards reap children on failure. The earlier Parquet comparison
was a historical experiment; its benchmark is no longer part of this harness.
The migration checklist records measured results and their limits.

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

Inspect production default, desktop and qualification metadata separately, filtering for
Linux and Windows. Inspect actual Linux executable linkage as additional
evidence, recognizing existing native deviations in the repository rules.

## Limits

Process kills at registration/commit boundaries do not cover every possible
write interleaving or physical power loss. A commit-boundary kill accepts either
complete revision; an acknowledged commit must survive. The exclusive-open
protocol does not qualify experimental multiprocess WAL. Source scanning and
linkage are bounded evidence, not proof about every runtime path.
The [migration checklist](../../docs/migration/summary.md) owns results and
current evidence and remaining qualification gaps.
