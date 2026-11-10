# Rust foundation implementation results

November 10, 2026. Historical validation results; current status and remaining
work live in [the migration summary](../../docs/migration/summary.md).

Linux validation used Rust/Cargo 1.99.1 on Debian 12, x86_64, uid 1000.
Formatting, Clippy with warnings denied, eight integration tests, the standalone
architecture gate, and a debug build passed. Routine Cargo checks need no Python,
regional data, provider access, or MGB.

A disposable copy of the RS geometry, executable, and twelve required inputs was
inspected from outside the workspace. Text/JSON success, missing MINI.gtp, invalid
TOML, an external geometry symlink, and unreadable geometry produced expected
statuses. Inventories compared names, kinds, bytes, symlink targets, modification
times, and directory entries, including SQLite sidecars and the external target.
All six cases preserved measured contents; fourteen original resources were
unchanged. MGB was never launched.

Subsequent structural review reproduced two architecture-checker false positives:
a same-owner re-export using `crate::<owner>` and an exclamation mark in ordinary
printed text. These remain unresolved; passing existing tests is not complete
architectural acceptance. The active [specification](../changes/rust-foundation.md)
is retained until that review is settled.

Windows, runtime compatibility, content/state validity, execution, numerical
Rust equivalence, and hydrological validity remain unverified. The one-off manual
acceptance script was retired; its results above are historical evidence, not a
maintained integration test.
