# MGB Ops Rust successor

MGB Ops inspects regional workspaces, prepares forcing from saved observations
and forecasts, and runs the external MGB numerical engine in isolation. The CLI
and optional native desktop interface use the shared Rust library for workspace
runs, previews, and analysis of the active run and explicitly saved runs.

## Development

Install Rust with rustup; `rust-toolchain.toml` selects Rust 1.99.1 with rustfmt
and Clippy. Dependency versions are recorded in `Cargo.lock`; no minimum Rust
version is qualified. Toolchain updates are qualified separately.
Linux is the validation platform; Windows remains unverified.

Both build configurations require the netCDF C library and its development
files. On Debian or Ubuntu, install `libnetcdf-dev` and `pkg-config`.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test architecture
cargo build
```

Build an optimized CLI in `target/release/`, or add `desktop` to build both binaries.

```sh
cargo build --locked --release
cargo build --locked --release --features desktop --bins
```

Routine checks use synthetic fixtures and do not require Python, regional data,
provider access, or MGB. The architecture gate also runs under `cargo test`.

## Usage

| Task | Start with | Details |
| --- | --- | --- |
| Inspect a regional workspace | `workspace inspect` | [Configuration and checks](docs/contracts/workspace.md) |
| Acquire ANA observations and ECMWF/NOAA forecasts | `observation acquire`, `forecast acquire`, or Desktop **Operation → Acquire inputs** | [Acquisition and Run All](docs/contracts/acquisition.md) |
| Acquire standalone IMERG, BNDMET rain and wind, or ONS hourly flow files | `acquisition` | [Standalone acquisition](docs/contracts/acquisition.md#standalone-source-acquisition) |
| Normalize a saved forecast | `forecast normalize` | [Forecast inputs and catalog](docs/contracts/forecasts.md) |
| Capture, save, or replay run inputs | `inputs capture`, `inputs save`, `inputs execute` | [Run input sets](docs/contracts/inputs.md) |
| Prepare and execute a workspace operation | `operation prepare`, then `operation execute` | [Execution paths and retained artifacts](docs/contracts/operations.md#run-from-a-workspace) |
| Run an operation and update the rainfall display | Desktop **Operation → New run → Run All** | [Operational run setup](docs/contracts/desktop.md#controlled-jobs) |
| Monitor rainfall or compare explicitly saved runs | Desktop **Map → Rainfall**, or **Review and runs → Run comparison** | [Desktop controls](docs/contracts/desktop.md#common-tasks) |
| Generate and view an inundation mask | `inundation process`, then desktop **Map → Inundation** | [Inundation inputs and processing](docs/contracts/inundation.md) |
| Inspect a workspace or review a run parameter change with an installed agent | `assistant` or desktop **Pages → Assistant** | [Assistant setup and usage](docs/contracts/assistant.md) |
| Review observations, export a bulletin, share runs, or schedule a job | CLI or desktop workflows | [Saved-data workflows](docs/contracts/saved-data.md) |

### Inspect a workspace

For a workspace without a configuration file, set the workspace path, create
the root directory, and generate example TOML. Set the regional
`spatial.mini_layers` in that file before inspecting.

```sh
workspace=/regional/workspace
mkdir -p "$workspace"
cargo run -- config example > "$workspace/config.toml"
```

```sh
cargo run -- workspace inspect --workspace "$workspace"
cargo run -- workspace inspect --workspace "$workspace" --json
```

The built `target/debug/mgb-ops` can run from other directories. See the
[workspace contract](docs/contracts/workspace.md) for configuration, overrides,
path rules, findings, and inspection limits. Library callers use the same
[inspection API](docs/contracts/workspace.md#library-api).

### Run an operation and retain its inputs

Set the routine run window and providers in the workspace’s `config.toml`.
Import station metadata as described in the [acquisition
contract](docs/contracts/acquisition.md). Run All acquires configured inputs,
freezes the selected data and settings, executes MGB and publishes the results.

```sh
cargo run -- operation run-all --workspace "$workspace"
cargo run -- inputs save --workspace "$workspace" --output /archive/july20-inputs
```

Choose a new directory under an existing parent for the independent save. When
replacing unsaved inputs, supply `--save-inputs-to PATH` or `--discard-inputs`.
Past attempts retain manifests. Export a completed run to keep its full results.
For a later replay with a matching installed engine, see [run input
sets](docs/contracts/inputs.md). For one-run changes, see [scenario
selection](docs/contracts/operations.md#scenarios).

## Desktop interface

```sh
workspace=/regional/workspace
cargo run --locked --features desktop --bin mgb-ops-desktop -- "$workspace"
```

Use **Open workspace** to open another project. The desktop keeps the map visible
alongside Layers and the right-hand tabs and forms. See the
[desktop task index](docs/contracts/desktop.md#common-tasks) and
[qualification limits](docs/contracts/desktop.md#current-qualification-limits).

## Project guidance

- [Architecture](docs/architecture.md): product scope, ownership, lifecycle, and enforcement.
- [Series and CSV](docs/contracts/series.md): rainfall summaries, chart queries and selected-series exports.
- [Local geometry](docs/contracts/spatial.md): mini topology, map coordinates and station links.
- [Canonical observations](docs/contracts/observations.md): Parquet 2, review selection, and observed forcing APIs.
- [Migration summary](docs/migration/summary.md): temporary capability tracking, validation status, and next work.
- [Agent guidance](AGENTS.md): change and documentation rules.
- [Legacy worktree record](references/retirement_record.md), [Python map](references/python_map.md),
  and [VB.NET/plugin map](references/vbnet_map.md): retained source references.
- [RS regression case](references/regression/rs-hourly/README.md): frozen evidence and reproduction.

Temporary migration tracking lives under `docs/migration/`. Plans, specifications,
and implementation results live under `.dev/`. Current behavior lives in
contracts, lasting decisions in architecture, and scientific evidence/provenance
in references.
