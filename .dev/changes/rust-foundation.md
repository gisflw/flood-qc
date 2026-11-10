# Rust foundation specification

Status: specified, implementation pending. Architecture revised November 10,
2026. Rust implementation requires a subsequent implementation task. This refines
the [agreed foundation plan](../plans/rust-foundation.md). Product decisions were
settled in discussion; the concrete contracts and development mechanism below
make them implementable. Neither the specification nor its acceptance criteria
are evidence of an implemented Rust capability.

## Outcome and boundaries

Establish one Rust Cargo package with a shared library and thin CLI. Its first
operational capability inspects an explicitly selected regional workspace
without changing it. Validate on Linux using a disposable RS workspace; Windows
remains a product target with validation deferred.

Preserve the plan's four core modules: store owns canonical contracts and private
storage implementations; adapters, analysis, and model independently depend on
store and never import each other. Shared application workflows live in the
library's `app` module, which owns configuration/path resolution and composes the
core modules. Core modules receive explicit inputs and do not discover global
configuration. Materialize only modules with implemented responsibilities.
Provider access, execution, decoding, storage migration, desktop, and numerical
equivalence remain outside this increment.

## Agreed configuration behavior

- Use TOML. Generate an example configuration that users can edit.
- `mgb-ops config example` prints commented TOML to stdout. Shell redirection
  selects the destination; the application does not write or overwrite a file.
- Default location: `<workspace>/config/workspace.toml`.
- Support `--config <path>` to select another file.
- Inspection requires the selected configuration file even when CLI flags supply
  every resource path. Do not silently fall back when it is missing.
- Reject unknown TOML keys, including in nested tables, to catch spelling errors.
  The generated example contains only currently supported settings.
- Precedence: built-in defaults, then configuration file, then explicit CLI
  flags. Validate the resulting effective configuration.
- Give settings dedicated flags such as `--engine-executable` and `--gpkg-path`;
  there is no generic `--params` flag.
- Expose only settings relevant to each command. Do not generate flags for all
  application settings or implement future operational parameters now.
- Overrides affect the invocation only and never rewrite configuration.
- Make the engine executable configurable, with platform-specific defaults.
  The current RS Linux executable is `mgb_runner/MGB_Inercial_Prev_RSH`;
  the Python Windows convention is
  `mgb_runner/MGB_Inercial_PrevRS_FORTRAN.exe`. Windows availability and execution
  have not been verified.
- Require `spatial.gpkg_path`; there is no RS-specific geometry default. An
  explicit `--gpkg-path` can supply this value for the invocation. The generated
  example uses a commented, clearly labeled example geometry path.
- `engine.input_dir` defaults to `mgb_runner/Input`; override it with
  `--engine-input-dir <directory>`.
- Resolve relative paths in TOML and CLI resource overrides against the selected
  workspace. Resolve explicit `--workspace` and `--config` paths against the
  invocation directory. These rules do not change when the config is elsewhere.

TOML replaces Python's `config/custom.yaml` for Rust configuration. Do not
silently consume or rewrite YAML. The disposable RS acceptance setup must
explicitly create its TOML configuration; no general YAML migration command
is required by the agreed plan.

## Agreed inspection behavior

- Require an explicitly selected workspace; do not infer one from the current
  directory, environment variables, or an RS-specific location.
- Check readable configuration, geometry, executable, and the exact bounded set
  of static engine inputs and preparation templates specified below.
- Treat absent history, caches, and previous runs as informational rather than
  inspection failures. Do not establish database connections to inspect contents;
  geometry readability uses only a bounded read of file bytes.
- Do not validate geometry contents or complete stored datasets in this increment.
- Return structured findings and actionable failures from the shared library;
  CLI code owns argument parsing and presentation.
- An unsuccessful requested validation produces an unsuccessful process status.
- Default CLI output is readable text; `--json` presents the same structured
  library findings. Findings include check name, status, path, and an actionable
  explanation.
- Report independent problems together rather than stopping at the first missing
  resource. Configuration failures may prevent dependent checks.
- Exit `0` when inspection passes, `1` when inspection fails (including missing
  or invalid configuration), and `2` for invalid CLI arguments. Informational
  findings do not cause failure.
- The engine must be a readable, nonempty regular file, with executable
  permission on Linux. Shared-library resolution and binary compatibility are
  deferred to the execution increment and explicitly reported as unchecked.
- Never launch the engine, fetch provider data, resolve live forecast sources,
  apply corrections, create databases, or create output/cache/log directories.
- Preserve source bytes, directory entries, and modification times on success
  and failure, including SQLite sidecars. Access-time changes caused by ordinary
  filesystem reads are outside this guarantee.
- Inspection success establishes only its enumerated prerequisite checks;
  it does not establish engine execution, state validity, or numerical validity.

## CLI contract

The package name is `mgb-ops`, library name `mgb_ops`, and binary name `mgb-ops`.
Supported commands:

```text
mgb-ops config example
mgb-ops workspace inspect --workspace <directory>
    [--config <file>] [--gpkg-path <file>] [--engine-executable <file>]
    [--engine-input-dir <directory>] [--json]
```

`--workspace` is required for inspection. Flags shown on following lines are
optional; the line breaks above are notation rather than shell continuations.
Support normal `--help` and `--version`. Reject unknown arguments and repeated
single-value options. Do not add initialization, migration, or execution commands.
The example command needs no workspace, does no workspace I/O, and exits `0`
after successfully writing TOML. Report output/serialization errors on stderr
and exit `1`; invalid CLI arguments exit `2`. Help/version exit `0`.

## Configuration and path contract

The initial schema contains only `spatial.gpkg_path`, `engine.executable`, and
`engine.input_dir`. Operational settings enter the schema when their commands
exist; unknown keys are rejected and a config file is mandatory.

| Key | Type | Default | Inspection override |
| --- | --- | --- | --- |
| `spatial.gpkg_path` | Nonempty string path | Required in effective configuration | `--gpkg-path` |
| `engine.executable` | Nonempty string path | Platform-specific path stated above | `--engine-executable` |
| `engine.input_dir` | Nonempty string path | `mgb_runner/Input` | `--engine-input-dir` |

Omitted tables are permitted when defaults/flags supply all required values.
Reject malformed TOML, duplicate keys, unknown keys, and wrong value types even
when an override targets that key. Apply overrides to parsed optional fields,
then validate required/nonempty effective values; an override may replace a
missing or empty string value, but cannot repair malformed TOML or a wrong type.
Reject all-whitespace paths without silently trimming nonempty paths. Do not
add config inclusion, profiles, environment loading, dotenv, or YAML fallback.

The generated example is valid, commented TOML with a clearly labeled editable
geometry example and both engine defaults explained. Emit the executable default
for the current platform. Parsing the example must succeed; resource existence
is not promised until the operator edits it for the workspace. A representative
Linux example is:

```toml
[spatial]
# Example only: replace with your regional geometry file.
gpkg_path = "data/source/your_region.gpkg"

[engine]
# Linux default; Windows default: mgb_runner/MGB_Inercial_PrevRS_FORTRAN.exe
executable = "mgb_runner/MGB_Inercial_Prev_RSH"
input_dir = "mgb_runner/Input"
```

Path bases are agreed above. Allow absolute paths, paths outside the workspace,
and symlinks; this is an explicitly selected local workspace, not a path sandbox.
Do not implicitly expand environment variables or `~` in configuration or
resource override values. Shell expansion before CLI parsing remains shell behavior.
The library receives the invocation directory explicitly and does not call
`set_current_dir`; the CLI captures that directory once.

Make paths absolute without requiring the target to exist, so missing-file
diagnostics still contain useful locations. Preserve filesystem semantics of
symlinks and `..` rather than collapsing components lexically. Display the
absolute lookup path; resolving every path through `canonicalize` is not required.
Follow symlinks for file-kind/readability checks; dangling links fail the resource
check. On Windows use native path semantics; do not reinterpret a Windows absolute
path as a native absolute path on Linux.

## Exact prerequisite checks

The selected workspace and engine input directory must be accessible directories.
The config must be a readable UTF-8 TOML regular file. Geometry, engine, and every
required input below must be readable, nonempty regular files. Open files read-only
and perform a bounded read to establish readability; do not hash or parse large
geometry/calibration files. On Linux also check the executable permission bits;
report this as a permission-bit check, not proof that execution is permitted by
mount options, ACLs, security policy, architecture, or the dynamic loader.

Required names are relative to `engine.input_dir`, with the casing shown:

| Group | Required files |
| --- | --- |
| Preparation templates | `PARHIG.hig`, `Previsao.meta`, `MINI.gtp` |
| Calibration and geometry | `albiaf.hig`, `paruso.hig`, `Cell.hig`, `COTA_AREA.FLP`, `face.con`, `Azimute_Mini.txt` |
| Supporting parameters | `medias.cli`, `FCORPLU.txt`, `curvas_chave_parametros.txt` |

This list is the initial RS layout contract, not a universal MGB contract. Do not
scan the directory and require every file found there or make the list configurable.
The retained [engine trace](../../references/regression/rs-hourly/README.md) and
`../legacy_python/src/mgb_ops/model/prepare_mgb_rainfall.py` are the source basis:
the trace identifies engine-opened static inputs, and preparation uses `MINI.gtp`.
The Python source path above is relative to the repository root.

Do not require or inspect rainfall binaries, observed flow/level files, wind,
hot-start/state files, or run-specific dates and horizons inside templates.
Report forcing, saved state, and input contents as unchecked. Do not require
`HAND_Flood.asc`, backup copies, or files used only by other engine variants.

Report existence only for `data/history.sqlite`, `data/cache`, and `data/runs`.
Their absence is informational; do not open stores or inspect their contents.
If their metadata is inaccessible, report that as information rather than making
it an inspection prerequisite. Do not inspect `current_run.sqlite` or sidecars.

Engine file and executable-permission checks are agreed above. Do not infer
runtime availability from the frozen reference's recorded dependencies alone.

## Findings and failure contract

Return an inspection report for expected configuration and filesystem failures;
do not use process exits or print from the library. The report has `ok`,
`workspace`, `config`, and an ordered `findings` list. Paths are absolute lookup
paths as specified above. `ok` is false if any finding has status `fail`.
Each finding contains:

| Field | Contract |
| --- | --- |
| `check` | Stable check identifier, such as `config`, `geometry`, `engine.executable`, or `engine.input.MINI.gtp` |
| `status` | `pass`, `fail`, `info`, or `not_checked` |
| `code` | `ok`, `missing`, `wrong_kind`, `unreadable`, `empty`, `invalid_config`, `not_executable`, `present`, `absent`, `metadata_unavailable`, `prerequisite_failed`, or `outside_scope` |
| `path` | Absolute resource path, or null for checks without a single resource |
| `message` | Human-readable finding and, on failure, a concrete repair suggestion |

Use the same report fields for JSON output with lowercase status/code values.
Messages may improve without changing check identifiers or status/code meanings;
scripts should use structured fields. JSON output is one document followed by a
newline, with no progress text or color. Native Rust APIs retain `PathBuf`; JSON
renders path strings lossily when native paths are not Unicode. No lossless
non-Unicode JSON path round trip is promised in this increment.

Order findings by workspace, config, geometry, executable, engine input directory,
required inputs in the table's stated order, optional stores, and scope limitations.
For each resource, report its first failing condition in this order: existence,
kind, readability, nonempty contents, then executable permission if applicable.
Report every independent resource failure after configuration resolves. If a
workspace/configuration failure prevents resolution, report the root failure and
one dependent-checks `not_checked` finding with code `prerequisite_failed`.
If the input directory fails, report one prerequisite finding for its children;
continue geometry, engine, optional-store, and scope findings.

Always include `outside_scope` findings covering geometry/input content validity,
forcing, saved state, runtime compatibility, and real execution. Expected scope
limitations do not make `ok` false. Do not echo source config lines or full config
contents; name invalid keys and locations without exposing unrelated values.

Text output presents the workspace, each finding, and a pass/fail summary with
an explicit statement of the bounded inspection scope. Inspection reports go to
stdout, including reports with failed checks. CLI usage and output failures go
to stderr. JSON mode also returns structured inspection failures on stdout;
invalid CLI arguments remain ordinary usage errors with exit `2`.

## Shared library and module boundaries

Expose a small request/report API at the library boundary. The request contains
workspace path, invocation directory, optional config path, and the three optional
resource overrides. The library resolves and validates all of them; CLI parsing
does not implement defaults, config merging, or resource checks. Example TOML
is available through the library without embedding CLI concepts in the schema.

`app` owns TOML loading, merging, paths, workspace/config/geometry findings,
optional-store presence, and the inspection request/report API. Configuration and
path helpers stay private within app; do not add a top-level workspace module.
`model` owns engine/input prerequisite checks and the required-file list,
accepting explicit paths rather than application settings. Model returns its own
prerequisite findings; app assembles them into the inspection report and
determines `ok`. Model must not import application report/configuration types.
The library root may re-export intended APIs but contains no composition logic
or shared implementation types. The binary only captures arguments/invocation
directory, calls the library, renders text or JSON, and maps outcomes to statuses.

Only app and model need foundation behavior. Do not create placeholder store,
adapters, or analysis modules, or a general reporting/configuration framework.
Record their future ownership in the architecture document: store's canonical
representations work independently of databases/files; adapters normalize into
them without deciding persistence; analysis prepares forcing and renders reports;
model serializes supplied forcing and decodes staged canonical outputs. App
coordinates these capabilities and calls store to persist or publish them.
SQLite and NetCDF remain starting choices; concrete schemas and file layouts
follow their operational increments. No new time, unit, rainfall interval, or
missing-value schema is introduced here; define those when first used.

Document the plan's execution-start snapshot and complete-set publication
requirements as constraints for the next operational increment. At that stage,
app freezes actual inputs or immutable versions, coordinates execution status and
failures, and asks store to publish only a complete scenario set. Failed attempts
retain staged outputs/diagnostics and leave previous published results official.
Do not implement snapshots, operational persistence, scenario execution, or
publication in the foundation.

## Development setup and enforcement

Use Rust edition 2024 and the stable toolchain, with rustfmt and Clippy documented
as prerequisites. Commit `Cargo.lock`. Record the exact Rust/Cargo versions used
in Linux acceptance evidence; do not claim an untested minimum Rust version or
pin a speculative future toolchain in this spec.

Direct runtime dependencies: `clap` with derive support for arguments, `serde`
with derive support for typed configuration/results, `toml` for configuration,
and `serde_json` for JSON output. Use standard-library errors and filesystem I/O;
no async runtime, database/geometry library, configuration framework, or generic
error wrapper is needed. Test-only dependencies are `tempfile` for isolated
fixtures and `syn` with full syntax/visitor support for the architecture check.
Select compatible maintained releases during implementation and lock them.

Use `[lints.rust] unsafe_code = "forbid"` in the one-package manifest so the rule
covers owned library, binary, integration-test, and example targets; dependencies
may use unsafe code. Require rustfmt, Clippy warnings as errors, relevant tests,
and the architecture check. Do not enable blanket pedantic lints.

### Architecture check mechanism

Implement one Rust integration test, `tests/architecture.rs`, using `syn` to
inspect all owned library sources, including app, the root facade, inline
modules, and test/cfg code. Also inspect owned binary/interface source files
(initially the CLI) for references to the library's intended APIs. The check runs
under `cargo test` and needs no installed external checker or Python. Enforce it
now, with syntax fixtures for future owners even before those modules exist.

| Source owner | Allowed other library owners |
| --- | --- |
| `store` | None |
| `adapters` | `store` |
| `analysis` | `store` |
| `model` | `store` |
| `app` | `store`, `adapters`, `analysis`, `model` |
| CLI/desktop interfaces | Intended public APIs of `app`, `store`, `adapters`, `analysis`, `model` |

External libraries do not add internal edges. No core module may import app or
interface code, and app may not import interface code. Configuration discovery
belongs inside app. Interface argument handling, rendering, and invocation
directory capture remain allowed; operational sequencing and publication policy
stay in app. The graph check enforces imports; tests/review enforce those
behavioral responsibilities and the requirement to accept supplied data without
registry I/O.

Keep the library root to module declarations, intentional API re-exports,
documentation, and lint attributes. Function/type definitions, type aliases, and
helper logic must live in their owning module. Code inside library owners must
name the owner directly, including when a root API facade exists. Root re-exports
serve external callers and cannot bypass internal ownership. Check re-export
origins; do not allow owners to re-export another owner's API as their own.

Make cross-owner references explicit `crate::<owner>::...` paths; interface
references use `mgb_ops` public paths. Check imports, grouped imports, type
references, and expressions; check aliases at their import. Relative self/super
paths must stay inside the same owner. Reject cross-owner glob imports,
`extern crate self` aliases, unsupported source inclusions such as `include!`,
and macro-generated dependencies that the checker cannot inspect. Reject
unexpected top-level library owners rather than leaving them unchecked. Keep
these source conventions explicit instead of building a general name resolver.

Inspect macro token paths and derive/helper attributes as well as ordinary Rust
paths. For the supported Serde/Clap derives, parse path-bearing attributes or
reject unsupported forms; never assume derives cannot create internal edges.
For example, `#[serde(default = "crate::model::default_value")]` inside adapters
creates a forbidden edge even though the path is encoded as a string. Supported
same-owner attributes and allowed cross-owner attributes must pass. Uncheckable
attribute/generated-source forms must fail rather than silently escape the gate.
Document the supported forms with the checker. This remains a bounded source
architecture gate, not general compiler-level dependency analysis. Extend its
coverage and fixtures before introducing unsupported forms.

Give the checker tiny syntax fixtures covering every allowed edge, forbidden
core-to-core edges, core-to-app and app-to-interface edges, grouped/aliased and
relative references, inline/cfg/test modules, root/helper/re-export bypasses,
allowed and forbidden derive-attribute paths, and unsupported inclusion/macro
forms. Verify actual owned source coverage, not just fixture parsing. Rust
privacy independently protects private implementation submodules; narrow exports
and thin interfaces remain explicit review requirements. Keep the architecture
contract, checker, and fixtures consistent when dependency rules change.

Required commands from the package root:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test --test architecture
```

The final command makes the architecture gate independently runnable; the full
test command already includes it. No Windows CI, cross-compilation infrastructure,
Python installation, live provider access, or RS data is required for routine checks.

Implementation references: [Cargo package lints](https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section),
[Clap derive](https://docs.rs/clap/latest/clap/_derive/),
[typed TOML parsing](https://docs.rs/toml/latest/toml/fn.from_str.html), and
[Syn syntax visitors](https://docs.rs/syn/latest/syn/visit/index.html).

## Acceptance

Keep fixtures small and generated inside temporary directories; fake nonempty
geometry/engine/input files suffice because content validity is outside scope.
Do not copy large operational data into unit tests or execute a fixture engine.

| Check | Required evidence |
| --- | --- |
| Config example | Valid TOML; correct platform default; comments identify editable geometry; no workspace required or changed. |
| Effective configuration | Defaults/file/flags precedence, mandatory file even with overrides, required geometry, nested unknown keys, malformed/duplicate keys, wrong types, missing/empty values, explicit YAML rejection. |
| Paths | Inspect from another directory; external config still uses workspace resource bases; absolute paths, spaces, symlinks and dangling links; missing resources have useful absolute paths. |
| Resource checks | One passing synthetic workspace; missing, wrong-kind, unreadable, and empty required resources; aggregate multiple independent failures; input-directory failure suppresses child noise; missing execute bits fail on Linux. |
| Operational stores | Absent history/cache/runs are informational; present SQLite and sidecars remain unopened and unchanged. Forcing/state absent does not fail. |
| Output and exits | Text/JSON represent the same report; JSON parses; failures and unchecked scope have defined codes; invalid invocation exits `2`, inspection failure `1`, successful inspection `0`. |
| Source preservation | Before/after inventory checks relative names, file kinds, symlink targets, file bytes, modification times, and directory entries on success and failure. Include existing SQLite sidecars and external symlink targets. Exclude access times. |
| Architecture | Checker covers app, core owners, the root facade, and owned interfaces; fixtures cover all allowed edges and reject forbidden edges, aliases/re-exports/root helpers, derive-attribute paths, and unsupported source generation. |

Run unreadable-file checks as an unprivileged user where permission restrictions
are effective; do not claim a chmod-based test proves unreadability under root.
The fixture binary can be a sentinel that would write a marker if launched;
assert no marker appears. This checks the no-execution requirement without MGB.

Separately prepare a disposable RS copy with an explicit TOML file pointing to
its geometry, existing executable, and `Input/`. Do not run a Rust migration or
fall back to live YAML. Run inspection from outside the workspace in text and
JSON modes. Require pass status for the bounded checks; demonstrate a missing
required input and invalid-config failure on the copy, and verify preservation
around each invocation. Test setup changes occur before each baseline inventory.
Inspect preserved symlink targets too; never use hard-linked disposable files
for destructive failure setup. Keep the original RS source and frozen reference
unchanged. MGB is never launched during this increment.

Record exact commands, Rust/Cargo and Linux environment, report examples,
preservation evidence, and known limits in a foundation report under
`docs/reports/`. Windows, dynamic linking, real execution, numerical equivalence,
and hydrological validity remain explicitly unverified.

## Documentation and completion

During implementation establish `docs/architecture.md` with the module ownership,
privacy/dependency rules, canonical store contracts, and app lifecycle requirements
for the next operational increment. Establish durable `docs/contracts/workspace.md`
for config, path, and source-ownership semantics and
`docs/contracts/inspection.md` for checks, findings, CLI, and limits. The README
must include development setup, actual check commands, and short example/inspection
usage. Keep implementation instructions consistent with these contracts.

After implementation and verification, update the migration checklist with actual
Linux evidence, intentional YAML/TOML departures, and remaining gaps. Retain
durable behavior in contracts and validation evidence in the report when retiring
temporary planning/specification documents. Next comes the saved RS preparation,
real execution, and decoded-results increment. Documentation alone does not
complete the foundation.
