# MGB Ops Rust successor

MGB Ops is a Rust successor with a shared library and CLI. It provides regional
workspace inspection, canonical saved observations and forecast normalization,
forcing preparation, and isolated MGB execution from retained prepared inputs.
The external MGB executable remains the numerical engine.

## Development

Install Rust with rustup; `rust-toolchain.toml` selects Rust 1.99.1 with rustfmt
and Clippy. Dependency versions are recorded in `Cargo.lock`; no minimum Rust
version is qualified. Toolchain updates are qualified separately.
Linux is the validation platform; Windows remains unverified.

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

Normalize an explicitly selected saved forecast, then prepare and execute a
separate strict operation request:

```sh
cargo run -- forecast normalize --workspace /regional/workspace --request normalize.toml --json
cargo run -- operation prepare --workspace /regional/workspace --request operation.toml --output prepared --json
cargo run -- operation execute --workspace /regional/workspace --prepared prepared --json
```

See the [forecast request](docs/contracts/forecasts.md) and
[operation request and retained artifacts](docs/contracts/operations.md).
Preparation does not launch MGB; execution creates an independent run with logs,
results and a JSON manifest. Live acquisition and new hot starts remain deferred.

## Project guidance

- [Architecture](docs/architecture.md): product scope, ownership, lifecycle, and enforcement.
- [Saved forecasts](docs/contracts/forecasts.md): normalization command, catalog, and forcing APIs.
- [Canonical observations](docs/contracts/observations.md): SQLite, review selection, and observed forcing APIs.
- [Saved operations](docs/contracts/operations.md): preparation/execution commands, snapshots and results.
- [Migration summary](.dev/migration/summary.md): temporary capability tracking, validation status, and next work.
- [Agent guidance](AGENTS.md): change and documentation rules.
- [Legacy worktree record](references/retirement_record.md), [Python map](references/python_map.md),
  and [VB.NET/plugin map](references/vbnet_map.md): retained source references.
- [RS regression case](references/regression/rs-hourly/README.md): frozen evidence and reproduction.

Temporary migration tracking, plans, specifications, and implementation results
live under `.dev/`. Current behavior lives in contracts, lasting decisions in
architecture, and scientific evidence/provenance in references.
