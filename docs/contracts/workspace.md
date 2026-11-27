# Workspace and inspection contract

Inspection requires an explicit workspace; neither the working directory nor
environment selects one. Paths may be absolute, outside the workspace, or symlinks.
This is a local workspace selection contract, not a path sandbox.

## Commands

```sh
mgb-ops config example
mgb-ops workspace inspect --workspace /regional/workspace
mgb-ops workspace inspect --workspace /regional/workspace --json
```

Inspection accepts `--config`, `--gpkg-path`, `--engine-executable`, and
`--engine-input-dir`, each with one path value. Workspace is required. Unknown
arguments and repeated single-value options are usage errors. `--help` and
`--version` are supported.

## Library API

The `mgb_ops` crate re-exports `inspect`, `InspectionRequest`, `InspectionReport`,
`FindingStatus`, `FindingCode`, and `example_config`. `example_config()` returns
the editable TOML as a `String`.
`read_workspace_config(path)` and `write_workspace_config(path, config)` use the
same typed TOML schema (`WorkspaceConfig` and its typed sections).
Paths inside the document remain unchanged. Saving validates nonempty configured
paths and atomically replaces the destination; a failed save preserves its bytes.
`inspect(request)` returns an `InspectionReport`, including expected failures;
it does not print output or choose an exit code. Findings are available through
`report.findings`, with their type exposed as `mgb_ops::app::Finding`. Finding
status and code fields use the public enums; their serialized strings are
specified below. Reports implement Serde serialization for the JSON
representation described below.

Construct `InspectionRequest` with a `workspace` path, an absolute `invocation_dir`,
and `config`, `gpkg_path`, `engine_executable`, and `engine_input_dir` overrides
as `Option<PathBuf>` (`None` uses the configuration/default rules below).
Capture `invocation_dir` at the interface boundary, for example with
`std::env::current_dir()`, and handle capture failure there. The library uses the
supplied directory without capturing or validating it; a relative directory does
not provide the absolute report-path guarantee. Resource overrides remain relative
to the workspace, just like CLI overrides.

## Configuration

Rust reads TOML, never Python's `config/custom.yaml`. Generate an editable example:

```sh
mgb-ops config example > /selected/workspace/config.toml
```

The command prints commented TOML to stdout; shell redirection owns file creation.
The parent directory must already exist. The example geometry is a placeholder
and must be edited for the region. Inspection always requires a configuration
file, even when flags supply every setting. Its default location is
`<workspace>/config.toml`; `--config` selects another file.

| Key | Default | Invocation override |
| --- | --- | --- |
| `spatial.mini_layers` | Required nonempty string | `--gpkg-path` |
| `model.executable` | Linux `model/engine`, Windows `model/engine.exe` | `--engine-executable` |
| `model.input_dir` | `model/Input` | `--engine-input-dir` |

Precedence is defaults, file, then explicit flags. Omitted tables are permitted
when defaults/flags supply all required values. Reject unknown keys at every
level, duplicate keys, malformed TOML, and wrong types before applying overrides.
Overrides can replace missing/empty strings but cannot repair malformed syntax
or wrong types. Reject all-whitespace effective paths; preserve whitespace in
other paths. Do not echo source configuration lines or unrelated values in errors.
There are no profiles, includes, environment/dotenv loading, generic parameter
flags, implicit variable/tilde expansion, YAML fallback, or implicit migration.

## Operating layout

Select the same workspace directory in the CLI and desktop.

```text
workspace/
├── config.toml                  Routine defaults
├── source/                      Regional source material
├── model/                       Installed engine and editable baseline Input
├── downloads/                   Original responses where retained
├── assets/
│   ├── registry.sqlite          Immutable station data revisions and metadata
│   ├── objects/                 Versioned file inputs and forecasts/
│   └── selections/              Resolved input-reference manifests
├── runs/                        Dated attempt manifests and diagnostics
├── work/                        Disposable execution files
└── results/
    ├── active.toml              Active successful result pointer
    └── <run-id>/                Results and rainfall, with optional kept sets
```

Object names are readable stable IDs; checksums verify content. Station data
and related coverage publish in one SQLite revision. Metadata and application
records use TOML, with the null encoding described in [registered inputs](inputs.md#manifest-and-payload).
Wind observations and reservoir schedules use TOML; gridded inputs, forcing and
results use NetCDF. Regional geometry retains its native spatial format.

Run IDs use the local operational reference hour as `YYYYMMDD-HH`; repeated
attempts receive distinct suffixes. Full timestamps and timezone are retained
in the manifest. Runs refer to shared registered inputs; they do not retain a
separate permanent copy for each attempt. Failed runs retain diagnostics and
leave active results unchanged. Use [result retention](operations.md#active-run-and-rainfall-cache)
and [portable input exports](inputs.md#capture-export-and-replay) when needed.

Initialize with `mgb-ops workspace init --workspace /region`. Conversion of a
legacy workspace is explicit: first export an interchange with
`tools/export_legacy_registry.py`, then use `mgb-ops workspace convert` with
that interchange and a new destination. The source must be quiescent and have
its existing regular `.operation.lock`; conversion never creates a source lock.
The Parquet bridge also locks its existing `access.lock` and verifies that the
selected generation and every input member remain unchanged. The source is preserved:

```sh
python3 tools/export_legacy_registry.py /old/workspace /tmp/legacy-interchange
mgb-ops workspace convert --source /old/workspace --workspace /new/workspace \
  --interchange /tmp/legacy-interchange
```

The standalone Python bridge requires `pyarrow` for legacy Parquet inputs; it is
not a runtime dependency. It also accepts independently saved legacy input sets
and preserves their indexed payload for conversion. See the utility's help for
arguments and supported legacy forms. There is no automatic schema migration
framework.

## Path resolution

Resolve relative `--workspace` and explicit `--config` against the working
directory from which the command is invoked. Inspection never changes it. Resolve
resource values in TOML and overrides against the selected workspace, even when
the configuration lives elsewhere.

Reports retain absolute lookup paths for missing as well as existing resources.
Do not lexically collapse symlinks and `..`; preserve filesystem lookup semantics.
Follow symlinks when checking resource kind/readability. Dangling links fail.
Use native platform path semantics; Windows-looking paths are not Windows absolute
paths when running on Linux. JSON uses lossy strings for non-Unicode paths and
promises no lossless round trip.

## Source ownership

Inspection reads only. It never rewrites configuration, creates outputs/caches/
logs, acquires providers, applies corrections, or
launches MGB. Geometry and required inputs receive bounded read-only byte checks,
not format parsing. Optional stores receive metadata checks only.

Preserve directory entries, symlink targets, file bytes, and modification times
on success and failure, including external symlink targets.
Access times from ordinary reads are outside the guarantee. Operational data,
legacy worktrees, and frozen regression evidence remain sources, not test fixtures.
Use disposable copies with no hard links for failure experiments.

## Bounded checks

Workspace and engine input directory must be accessible directories. Configuration
must be a readable UTF-8 TOML regular file. Geometry, executable, and the following
inputs must be readable, nonempty regular files. Check existence, kind,
readability, nonempty contents, then Linux executable bits, reporting each
resource's first failure. Read only a bounded amount of prerequisite file bytes.

Required filenames relative to `engine.input_dir`, in report order:

| Group | Files |
| --- | --- |
| Preparation templates | `PARHIG.hig`, `Previsao.meta`, `MINI.gtp` |
| Calibration/geometry | `albiaf.hig`, `paruso.hig`, `Cell.hig`, `COTA_AREA.FLP`, `face.con`, `Azimute_Mini.txt` |
| Supporting parameters | `medias.cli`, `FCORPLU.txt`, `curvas_chave_parametros.txt` |

This is the initial RS layout, not a universal MGB contract. The engine trace in
the [RS reference](../../references/regression/rs-hourly/README.md) and legacy
rainfall preparation establish the list. Do not require every discovered input,
`HAND_Flood.asc`, backups, forcing binaries, observed flows/levels, wind, saved
state, or run-specific dates/horizons.

Report metadata presence of the optional registry, workspace cache, and runs directory. Absence or inaccessible metadata is informational. Inspection
does not load those stores or determine which generation is active.

Always report unchecked geometry/input contents, forcing, saved state, runtime
compatibility, and actual execution. Linux executable-bit checks do not establish
loader/shared-library availability, architecture compatibility, mount permissions,
ACLs, security policy, or successful execution. Windows checks are deferred.

## Findings and output

Reports contain `ok`, `workspace`, `config`, and ordered `findings`, including
expected configuration/filesystem failures.
Each finding contains `check`, `status`, `code`, `path` (absolute or null), and
`message`. Messages include a repair suggestion for failures. Scripts should use
stable identifiers/status/codes rather than matching message text.

| Field | Values |
| --- | --- |
| `status` | `pass`, `fail`, `info`, `not_checked` |
| `code` | `ok`, `missing`, `wrong_kind`, `unreadable`, `empty`, `invalid_config`, `not_executable`, `present`, `absent`, `metadata_unavailable`, `prerequisite_failed`, `outside_scope` |

Stable `check` identifiers:

| Identifier | Meaning |
| --- | --- |
| `workspace`, `config`, `geometry` | Selected workspace, effective configuration, and geometry file |
| `dependent_checks` | Checks suppressed by workspace/configuration failure |
| `engine.executable`, `engine.input_dir` | Engine executable and required input directory |
| `engine.input.<filename>` | One required input, using the exact filename from the bounded-checks table |
| `engine.inputs` | Individual input checks suppressed by input-directory failure |
| `optional.database`, `optional.cache`, `optional.runs` | Metadata presence of the three optional resources |
| `scope.contents`, `scope.forcing`, `scope.state`, `scope.runtime`, `scope.execution` | The five unchecked areas, always reported |

Order is workspace, config, geometry, executable, input directory, required inputs
in the table order, optional stores, then scope limits. Independent failures are
aggregated. Workspace/config failure produces its root failure plus one dependent
`not_checked`/`prerequisite_failed` finding. An input-directory failure suppresses
individual child failures but leaves independent checks active. Scope limitations
are always present and never cause failure. `ok` is false exactly when a finding
has status `fail`.

Text output presents workspace, findings, and a bounded-scope pass/fail summary.
JSON is the same report as one document followed by a newline, without progress
text/color. Inspection reports, including failures, go to stdout. Usage/output
errors go to stderr. Exit codes: `0` success/help/version, `1` failed inspection or
output, `2` invalid arguments. The example command requires no workspace and does
no workspace I/O.

Passing inspection proves only these prerequisites. It does not prove engine
execution, numerical equivalence, state validity, or hydrological validity.
