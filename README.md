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

After validation, follow the [release handoff](AGENTS.md#handoff) to remove debug
build artifacts and leave both release binaries ready for terminal use.

Routine checks use synthetic fixtures and do not require Python, regional data,
provider access, or MGB. The architecture gate also runs under `cargo test`.

## Usage

| Task | Start with | Details |
| --- | --- | --- |
| Inspect a regional workspace | `workspace inspect` | [Configuration and checks](docs/contracts/workspace.md) |
| Acquire ANA observations and ECMWF/NOAA forecasts | `observation acquire`, `forecast acquire`, or Desktop **Processing → Operation → Acquire inputs** | [Acquisition and Run All](docs/contracts/acquisition.md) |
| Acquire standalone IMERG, BNDMET rain and wind, or ONS hourly flow files | `acquisition` | [Standalone acquisition](docs/contracts/acquisition.md#standalone-source-acquisition) |
| Normalize a saved forecast | `forecast normalize` | [Forecast inputs and catalog](docs/contracts/forecasts.md) |
| Capture or replay registered inputs | `inputs capture`, `inputs execute` | [Registered inputs](docs/contracts/inputs.md) |
| Prepare and execute a workspace operation | `operation prepare`, then `operation execute` | [Execution paths and retained artifacts](docs/contracts/operations.md#run-from-a-workspace) |
| Run an operation and update the rainfall display | Desktop **Processing → Operation → New run → Run All** | [Operational run setup](docs/contracts/desktop.md#controlled-jobs) |
| Monitor rainfall or compare explicitly saved runs | Desktop **Map → Rainfall**, or **Processing → Review and runs → Run comparison** | [Desktop controls](docs/contracts/desktop.md#common-tasks) |
| Generate and view an inundation mask | `inundation process`, then desktop **Map → Inundation** | [Inundation inputs and processing](docs/contracts/inundation.md) |
| Inspect a workspace or review a run parameter change with an installed agent | `assistant` or desktop **Workspace → Pages → Assistant** | [Assistant setup and usage](docs/contracts/assistant.md) |
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

For a visual tour, start with the [workspace tree and data roles](docs/contracts/workspace.md#operating-layout),
then follow [station acquisition](docs/contracts/acquisition.md#ana-observations),
[input capture and replay](docs/contracts/inputs.md#capture-export-and-replay),
and [execution](docs/contracts/operations.md#run-from-a-workspace).
The [architecture flow](docs/architecture.md#operational-data-flow) explains
which module owns each stage.

Set the routine run window and providers in the workspace’s `config.toml`.
Import station metadata as described in the [acquisition
contract](docs/contracts/acquisition.md). Run All acquires configured inputs,
registers the selected data and settings, executes MGB and publishes the results.

```sh
cargo run -- operation run-all --workspace "$workspace"
```

Registered versions and attempt manifests remain available for later replay.
Keep or export a completed run to retain its full results. For independent
input export and replay with a matching engine, see [registered inputs](docs/contracts/inputs.md).
For one-run changes, see [scenario selection](docs/contracts/operations.md#scenarios).

## Desktop interface

Linux desktop startup needs a working X11 or Wayland display, its window and
keyboard libraries, and a graphics driver supported by WGPU. X11 includes the
XKB runtime and keyboard data (`libxkbcommon-x11-0` and `xkb-data` on Debian).
A successful compilation alone does not establish that these runtime services
are available. Software-rendered qualification is recorded separately from
operator-hardware evidence.

```sh
workspace=/regional/workspace
cargo run --locked --features desktop --bin mgb-ops-desktop -- "$workspace"
```

Use **Open workspace** to open another project. The desktop keeps the map visible
alongside Layers and the right-hand tabs and forms. See the
[desktop task index](docs/contracts/desktop.md#common-tasks) and
[qualification limits](docs/contracts/desktop.md#current-qualification-limits).

### Linux application launcher

The desktop registers the HGE icons and an application entry for your account on
its first launch, including when run directly or through Cargo. It repairs icon
associations in an existing entry while retaining custom launch commands. The
application draws its own title bar and window controls in Light and Dark modes.
Windows desktop builds embed the HGE executable icon.

For a fixed installation path, build the release desktop and install the binary,
HGE icons, and application entry:

```sh
install -Dm755 target/release/mgb-ops-desktop "$HOME/.local/bin/mgb-ops-desktop"
install -Dm644 assets/desktop/br.ufrgs.hge.MgbOps.desktop \
  "$HOME/.local/share/applications/br.ufrgs.hge.MgbOps.desktop"
for size in 32 48 64 128 256; do
  install -Dm644 "assets/branding/hge-$size.png" \
    "$HOME/.local/share/icons/hicolor/${size}x${size}/apps/br.ufrgs.hge.MgbOps.png"
done
```

Ensure `$HOME/.local/bin` is on your graphical session's `PATH`. MGB Ops then
appears in the application menu; pin it or copy its installed application entry
to the desktop using your desktop environment. It reopens the most recent
workspace. The [HGE emblem](assets/branding/README.md) and
[emoji artwork](assets/icons/README.md) have separate provenance.

## Project guidance

- [Repository rules and exceptions](docs/contracts/rules-exceptions.md): safety, dependencies, documentation, validation and native-library exceptions.
- [Architecture](docs/architecture.md): product scope, ownership, lifecycle, and enforcement.
- [Series and CSV](docs/contracts/series.md): rainfall summaries, chart queries and selected-series exports.
- [Local geometry](docs/contracts/spatial.md): mini topology, map coordinates and station links.
- [Canonical observations](docs/contracts/observations.md): SQLite revisions, review selection, and observed forcing APIs.
- [Migration summary](docs/migration/summary.md): temporary capability tracking, validation status, and next work.
- [Agent guidance](AGENTS.md): change and documentation rules.
- [Legacy worktree record](references/retirement_record.md), [Python map](references/python_map.md),
  and [VB.NET/plugin map](references/vbnet_map.md): retained source references.
- [RS regression case](references/regression/rs-hourly/README.md): frozen evidence and reproduction.

Temporary migration tracking lives under `docs/migration/`. Plans, specifications,
and implementation results live under `.dev/`. Current behavior lives in
contracts, lasting decisions in architecture, and scientific evidence/provenance
in references.
