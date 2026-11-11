# MGB Ops Rust successor

MGB Ops is a Rust successor with a shared library and CLI. It currently provides
read-only inspection of explicitly selected regional workspaces. The external MGB
executable remains the numerical engine; inspection never launches it.

## Development

Install Rust with rustup; `rust-toolchain.toml` selects Rust 1.99.1 with rustfmt
and Clippy. Dependency versions are recorded in `Cargo.lock`; no minimum Rust
version is qualified. Toolchain updates are qualified separately.
Linux inspection has been validated; Windows remains unverified.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test architecture
cargo build
```

Routine checks use synthetic fixtures and require neither Python, regional data,
providers, nor MGB. The architecture gate also runs under `cargo test`. GitHub
Actions runs formatting, Clippy, the full test suite, and the independent
architecture test on Linux for pushes and pull requests. Branch protection is
deferred.

## Usage

Create the workspace's `config` directory, print an example, and edit its geometry
path before inspection:

```sh
cargo run -- config example > /regional/workspace/config/workspace.toml
cargo run -- workspace inspect --workspace /regional/workspace
cargo run -- workspace inspect --workspace /regional/workspace --json
```

The built `target/debug/mgb-ops` can run from other directories. See the
[workspace contract](docs/contracts/workspace.md) for configuration, overrides,
path rules, findings, and inspection limits. Library callers use the same
[inspection API](docs/contracts/workspace.md#library-api).

## Project guidance

- [Architecture](docs/architecture.md): product scope, ownership, lifecycle, and enforcement.
- [Migration summary](.dev/migration/summary.md): temporary capability tracking, validation status, and next work.
- [Agent guidance](AGENTS.md): change and documentation rules.
- [Legacy worktree record](references/retirement_record.md), [Python map](references/python_map.md),
  and [VB.NET/plugin map](references/vbnet_map.md): retained source references.
- [RS regression case](references/regression/rs-hourly/README.md): frozen evidence and reproduction.

Temporary migration tracking, plans, specifications, and implementation results
live under `.dev/`. Current behavior lives in contracts, lasting decisions in
architecture, and scientific evidence/provenance in references.
