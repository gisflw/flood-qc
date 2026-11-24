# MGB Ops Rust successor

MGB Ops inspects regional workspaces, prepares forcing from saved observations
and forecasts, and runs the external MGB numerical engine in isolation. The CLI
and optional native desktop interface use the shared Rust library for operation
drafts, previews, and analysis of the active run and explicitly saved runs.

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

Build optimized CLI and desktop binaries in `target/release/`:

```sh
cargo build --locked --release --features desktop --bins
```

Routine checks use synthetic fixtures and do not require Python, regional data,
provider access, or MGB. The architecture gate also runs under `cargo test`.

## Usage

| Task | Start with | Details |
| --- | --- | --- |
| Inspect a regional workspace | `workspace inspect` | [Configuration and checks](docs/contracts/workspace.md) |
| Normalize a saved forecast | `forecast normalize` | [Forecast inputs and catalog](docs/contracts/forecasts.md) |
| Prepare and execute a saved recipe | `operation prepare`, then `operation execute` | [Execution paths and retained artifacts](docs/contracts/operations.md#choose-an-execution-path) |
| Run an operation and update the rainfall display | Desktop **Processing → New run → Run All** | [Operational run setup](docs/contracts/desktop.md#controlled-jobs) |
| Edit, preview, or run an experiment | `draft` commands or **Draft → Run experiment** | [Draft workflow](docs/contracts/drafts.md#editing-source-acceptance-and-previews) |
| Monitor rainfall or compare explicitly saved runs | Desktop **Map → Rainfall**, or **View → Run comparison** | [Desktop controls](docs/contracts/desktop.md#common-tasks) |
| Generate and view an inundation mask | `inundation process`, then desktop **Map → Inundation** | [Inundation inputs and processing](docs/contracts/inundation.md) |
| Review observations, export a bulletin, share runs, or schedule a job | CLI or desktop workflows | [Saved-data workflows](docs/contracts/saved-data.md) |

### Inspect a workspace

For a workspace without a configuration file, set the workspace path, create
its configuration directory, and generate example TOML. Set the regional
`spatial.gpkg_path` in that file before inspecting.

```sh
workspace=/regional/workspace
mkdir -p "$workspace/config"
cargo run -- config example > "$workspace/config/workspace.toml"
```

```sh
cargo run -- workspace inspect --workspace "$workspace"
cargo run -- workspace inspect --workspace "$workspace" --json
```

The built `target/debug/mgb-ops` can run from other directories. See the
[workspace contract](docs/contracts/workspace.md) for configuration, overrides,
path rules, findings, and inspection limits. Library callers use the same
[inspection API](docs/contracts/workspace.md#library-api).

### Prepare and execute a saved recipe

If your operation uses a saved GRIB2 forecast, normalize it with
[an example normalization request](docs/contracts/forecasts.md).

```sh
cargo run -- forecast normalize --workspace "$workspace" --request normalize.toml --json
```

Create `operation.toml` from the [example operation request](docs/contracts/operations.md#request).
Set the asset ID returned by normalization if you are using that forecast.
Prepare to a new destination and execute the preparation.

```sh
cargo run -- operation prepare --workspace "$workspace" --request operation.toml --output prepared --json
cargo run -- operation execute --workspace "$workspace" --prepared prepared --json
```

Success replaces the workspace active run, including rainfall and scenario
results. Past attempts keep a replay manifest. Export a bundle before replacement
to keep full results.
[Controlled cancellation](docs/contracts/operations.md#controlled-actions-and-cancellation)
covers SIGINT and the commit boundary.

### Run an experiment

Use a [draft](docs/contracts/drafts.md) to save incomplete settings and ordered
edits. Once the operation is complete and saved as `draft.json`, explicitly
accept the selected inputs and run the experiment.

```sh
cargo run -- draft refresh --workspace "$workspace" --draft draft.json --json
cargo run -- draft run --workspace "$workspace" --draft draft.json --json
```

## Desktop interface

```sh
workspace=/regional/workspace
cargo run --locked --features desktop --bin mgb-ops-desktop -- "$workspace/config/workspace.toml"
```

Use **Open TOML** to open another project. The desktop keeps the map visible
alongside Layers and the right-hand tabs and forms. See the
[desktop task index](docs/contracts/desktop.md#common-tasks) and
[qualification limits](docs/contracts/desktop.md#current-qualification-limits).

## Project guidance

- [Architecture](docs/architecture.md): product scope, ownership, lifecycle, and enforcement.
- [Series and CSV](docs/contracts/series.md): rainfall summaries, chart queries and selected-series exports.
- [Local geometry](docs/contracts/spatial.md): mini topology, map coordinates and station links.
- [Canonical observations](docs/contracts/observations.md): SQLite, review selection, and observed forcing APIs.
- [Migration summary](.dev/migration/summary.md): temporary capability tracking, validation status, and next work.
- [Agent guidance](AGENTS.md): change and documentation rules.
- [Legacy worktree record](references/retirement_record.md), [Python map](references/python_map.md),
  and [VB.NET/plugin map](references/vbnet_map.md): retained source references.
- [RS regression case](references/regression/rs-hourly/README.md): frozen evidence and reproduction.

Temporary migration tracking, plans, specifications, and implementation results
live under `.dev/`. Current behavior lives in contracts, lasting decisions in
architecture, and scientific evidence/provenance in references.
