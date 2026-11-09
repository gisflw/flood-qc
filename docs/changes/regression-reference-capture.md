# Regression reference capture specification

Status: reference captured and repeated November 9, 2026; revision 1 verified.
This specifies reference capture, not the Rust port. See the
[review report](../reports/regression-reference-capture.md) and
[established case](../../references/regression/rs-hourly/README.md).

## Goal and scope

Freeze
one qualified RS operation, correct observations, prepare and execute four
scenarios with the available external engine, and preserve decoded results and
stage comparisons. The agreed requirements below govern scenario definitions
and exclusions.

### Agreed case and scenarios

Prefer the saved `2026-07-20T00:00:00Z` forecast cycle with operation reference
`2026-07-20T00:00:00-03:00` in `America/Sao_Paulo`. Use the full RS model domain,
hourly timesteps, and nominal 10-day observation / five-day forecast settings.
Qualify actual endpoints and coverage rather than assuming 240/120 samples.
If this candidate lacks valid inputs, compatible state, or representative sequence
corrections, investigate another saved date with both providers and record why;
do not invent data or fetch replacements.

| Scenario | Forecast forcing |
| --- | --- |
| `zero` | Zero forecast rainfall, retaining corrected observations. |
| `ecmwf-raw` | Pinned normalized ECMWF asset without edits. |
| `noaa-raw` | Pinned normalized NOAA asset without edits. |
| `ecmwf-corrected` | The same ECMWF asset, multiplied by 1.5, translated +2° latitude / −3° longitude, then rotated +45° counterclockwise about the original grid center, across the full selected forecast window. |

Verify source resolution before encoding displacement: on a 0.25° grid the
legacy instruction uses +8 latitude cells and −12 longitude cells. Preserve
nearest-neighbor sampling, the expanded footprint, and nodata handling. Include
an actual repeated-0.2 sequence in observation-review evidence; the matching rule
still includes every consecutive value strictly between 0 and 1 mm.

Start at saved normalized inputs and stop at decoded results. Acquisition,
normalization/registration regression, analysis, bulletins, other exports, GUI,
Windows validation, new hot-start generation/chaining, and completed-run sharing
are deferred. Rust comparisons follow in a later increment.

Use existing `legacy_python` APIs interactively or through documented invocation
snippets. Do not add maintained capture/replay tooling, a new CLI, dependencies,
or Rust implementation. Preserve enough data and exact invocation instructions
to repeat capture when needed.

## Confirmed implementation decisions

- Keep the capture specification and small versioned evidence in `mgb_ops`.
- Execute the existing sibling `legacy_python` checkout; record its exact commit
  and any behavior-affecting changes rather than assuming the historical tip.
- Preserve both the SQLite history/forecast registry needed by the legacy
  loaders and a versioned case manifest describing the captured evidence.
- Store large data and binaries outside Git. Local copy-based retrieval is
  sufficient; record the source location and portable relative file paths,
  sizes, and SHA-256 checksums. No shared hosting service is required.
- Run only in disposable workspaces, with explicit reference times, forecast
  asset identities, and private model Input/Output directories.
- Preserve the entire source `/workspace/rs_hydro_forecast` directory in place:
  do not execute against it, rewrite its configuration, publish its current-run
  artifact/caches, or alter its model inputs, state, and outputs.
- Keep a complete consistent SQLite history snapshot; do not trim observations
  or supporting tables. Use SQLite's backup API to include committed WAL data,
  rather than copying the database file alone. Rebase only the working copy;
  retain the original snapshot and record every path change.
- For the candidate case, review `(2026-07-09T23:00:00-03:00,
  2026-07-20T00:00:00-03:00]`. This includes July 10 00:00, the first model
  observation timestamp. The nominal observation setting remains 10 days.
- Nodata replacements exclude the affected station readings from hourly IDW;
  remaining usable stations supply rainfall. An hour with no usable stations or
  a non-finite observed working grid fails preparation. Do not fill nodata with
  zero or change interpolation policy.
- Preserve saved discharge, level, substitution files, and assimilation controls
  unchanged. Qualify their coverage/alignment and record which are consumed;
  do not rebuild them from history for this milestone.
- Carry the required saved initial-state files unchanged into every scenario
  and the repeat capture. Hot-start generation/rewriting, save-date control
  writes, output-to-input state promotion, and scenario chaining are out of
  scope. Recompilation to enable those behaviors is not a capture prerequisite.
  Record existing state identities and any temporal mismatch as qualification
  evidence; do not relabel or regenerate state to make the case pass.

## Preserved files and case manifest

Use `references/regression/rs-hourly/` in `mgb_ops` for the small established-case
record. Create these files only when evidence is available:

| File | Content |
| --- | --- |
| `manifest.json` | Versioned case identity, provenance, operation, registry bindings, file inventory, comparison contracts, and capture/repeat outcomes. |
| `expectations.json` | Compact checkpoint counts, dimensions, representative values, missingness, and measured repeat-comparison results. |
| `README.md` | Local bundle location, copy instructions, prerequisites, exact legacy API invocation snippets, and evidence interpretation. |

No registry service, manifest framework, or separate schema validator is needed.
Use JSON objects, arrays, strings, finite numbers, booleans, and null; represent
missing numeric readings as null in summaries rather than JSON NaN. The manifest
contract is defined here:

| Field | Contract |
| --- | --- |
| `schema_version` | Integer `1`; change when field meaning or required structure changes. |
| `case_id`, `revision` | Stable case string `rs-hourly`, positive integer revision. A frozen revision is not overwritten with new expectations. |
| `status` | `qualified` or `verified`; only successful complete repeat comparison permits `verified`. Failed attempts remain evidence outside the accepted manifest. |
| `provenance` | Exact main, Python, runtime, and any consulted VB.NET commits; dirty-file identities and saved patches needed to reconstruct relevant behavior; Python/package versions, OS/architecture, engine runtime requirements. Do not store secret environment values. |
| `bundle` | Recorded local source location and inventory paths relative to the external bundle root. Another machine may supply a different root. |
| `operation` | Explicit reference/cycle times, timezone, timestep, nominal 10/5 settings, actual review/preparation/export bounds and endpoint conventions, counts, selected observation providers, spatial and interpolation settings. Record local naive values passed to legacy APIs alongside their unambiguous offset/UTC meanings. |
| `registry` | Original and rebased SQLite snapshot paths and hashes, schema identity, selected asset records and relative paths, provider codes, cycle/validity metadata, preferred-series selection, and exact path-rebase changes. Retain unselected records without claiming their external files are captured. |
| `scenarios` | Four ordered objects with stable IDs `zero`, `ecmwf-raw`, `noaa-raw`, `ecmwf-corrected`; positions 0–3; legacy kinds, pinned source IDs/paths, correction fields, and checkpoint artifact paths. Both ECMWF scenarios bind to the same source asset. |
| `observation_review` | Rule bounds/minimum length, review interval, replacement artifact, all affected station/time keys and original values, sequence memberships, repeated-0.2 evidence, and unchanged surrounding values. |
| `initial_state` | Saved files/controls actually used, hashes and recorded times, compatibility evidence and limitations, and the unchanged common initial-state identity supplied independently to every scenario. |
| `files` | Objects with unique bundle-relative `path`, `role`, byte `size`, and `sha256`; classify source, registry, configuration, geometry/calibration, auxiliary forcing, state, instruction, prepared input, raw output, decoded output, and log evidence. Hashes identify preserved bytes; numerical equivalence uses checkpoint rules. |
| `comparisons` | Per-checkpoint artifact paths, variables, dimension order, cell/time alignment, units, mask rules, exact comparisons, justified numerical tolerances, and an explicit list of ignored metadata fields. No blanket metadata exclusion. |
| `executions` | Capture/repeat IDs, per-scenario command, working directory, relevant nonsecret environment, start/end times, duration, exit status, logs, generated file identities, and comparison outcome. |
| `limitations` | Case-specific known limits and deferred scope; no implied Windows or Rust verification. |

Outside Git, preserve a bundle with `frozen/`, `capture/`, and `repeat/`:

- `frozen/` holds the original consistent SQLite snapshot, selected normalized
  forecasts, spatial files, effective configuration, consumed engine inputs,
  unchanged saved state, executable, and reproducibility environment
  evidence. Preserve original and path-rebased records separately.
- `capture/` preserves observation-review and path-rebase evidence, the
  operation SQLite, working grids, changed prepared inputs, the raw flow/level
  streams needed for decoding, decoded NetCDFs, commands, and logs. Unchanged
  inputs are stored once in `frozen/`.
- `repeat/` preserves its operation, decoded NetCDFs, commands, logs, and
  comparison evidence. Byte-identical files are stored once under `capture/`;
  repeat grids, ancillary outputs, and temporary decoding files are reconstructed
  when needed. Preserve input/state hashes before execution and post-execution
  files separately where their bytes change.
- Start repeat from `frozen/`, never from capture's mutated Input or caches.
  All file paths in portable evidence resolve under the supplied bundle root;
  record execution-specific absolute paths as provenance only.

The full history snapshot is preserved even though only the selected normalized
assets need to be copied. Capture must not resolve an unselected asset or fall
back to any file in the operational source directory.

### Capture adaptations

In disposable scenario inputs, align `Previsao.meta` to July 20 01:00 local and
121 forecast steps. The saved October/361-step control is retained in `frozen/`.
Join the engine's separate observed and forecast flow streams in decoding
staging; preserve original engine files unchanged.

The engine overwrites the first 121 observed-level records when forecasting.
For each capture/repeat, an independent same-forcing zero-scenario run snapshots
the 241 observed-level records immediately before the rewind, using the external
bundle's small Linux interception shim. Check its complete observed flow and
unaffected 120 level records against all four scenarios, then join the recovered
observed levels with each scenario's raw forecast levels for decoding. Preserve
the shim source, invocation, snapshot, and raw recovery outputs with the bundle.
These are capture adaptations; they do not change the legacy package or engine.

## Existing contracts to use

Paths in this section are relative to `legacy_python`.

| Surface | Existing owner | Capture requirement |
| --- | --- | --- |
| Forecast registry | `src/mgb_ops/assets/forecast_registry.py`, `assets/history.py` | Preserve selected asset IDs, providers, cycle and validity metadata, and resolvable paths. Rebase paths only in the disposable copy. |
| Operation and instructions | `assets/current_run.py` | Persist a fresh `CurrentRunArtifact` with four explicit scenario references, forecast corrections, and `ObservedReplacement` entries whose value is `None`. |
| Preferred observations | `assets/effective_observations.py` | Preserve preferred-series selection and provider filtering. Loading uses local naive timestamps with `(start, end]` selection. |
| Sequence detection | `qc/precipitation.py` | Use only `sequence_matches`: at least five consecutive hourly values strictly between 0 and 1 mm. Do not apply threshold matches. |
| Forecast edits | `edit/forcing.py` | Preserve geographic intent and actual cell-based encoded instruction, coordinates, masks, and transformed footprint. |
| Preparation/execution | `workflows/scenario_orchestrator.py`, `model/prepare_mgb_meta.py`, `model/prepare_mgb_rainfall.py`, `model/mgb_execution.py` | Reuse the artifact-aware path so persisted observation replacements reach the common observed working grid. |
| Decoding | `model/export_mgb_outputs.py` | Capture precipitation, flow, and level with their actual units, time axes, cell order, segments, masks, and complete values. |

Do not use `workflows/scenarios.py::build_current_artifact` unchanged to establish
the case: it resolves forecast assets automatically, seeds threshold matches,
and replaces sequence matches with zero. Construct explicit references and
instructions using the existing artifact types and persistence API instead.

## Qualification findings

1. The current `mgb_runner/Input/CI.meta` and `DATA.hot` both report August 28,
   2026 at 00:00; the candidate simulation begins July 10. These files alone
   do not prove which state is actually consumed. Identify the consumed saved
   state and record compatibility and limitations. If it prevents valid
   execution, report failed qualification and consider another saved case under
   the plan's fallback rule; do not expand this target into state generation.
   Current controls: `HOT.HOT = 1` (VB.NET's "Usar Hot Start"), `DATA.hot =
   2026-08-28 00`, and `CI.meta = 2026-08-28 0`. Reading an initial state and
   producing a new state are separate behaviors. Static inspection of the
   current ELF finds `hotstart_` and the message `GRAVANDO CONDICOES INICIAIS`,
   but no direct call to `hotstart_` in disassembly; state production appears
   inactive. Enabling or verifying state production belongs to the later
   hot-start increment and is not an acceptance gate for this target.
   No Fortran engine source was found in the available workspace.
2. `PARHIG.hig` references observed discharge, levels, and substitution inputs;
   `PARAM_ASSIMILA.txt` enables updates. Determine which files are consumed and
   establish coverage/alignment without altering the saved auxiliary inputs.
3. The batch orchestrator deletes its staging directory on scenario failure.
   The capture procedure must preserve failed work without relying on that
   cleanup path, using the existing lower-level preparation/execution APIs.
4. Actual state compatibility, case coverage, repeated-0.2 evidence, engine
   prerequisites, exact endpoints, and numerical repeatability remain empirical
   qualification results. Do not populate them with assumed successful values.

## Verification contract

Capture and a second execution from a fresh copy must both complete all four
scenarios. Preserve full arrays and raw outputs outside Git, with compact
expectation summaries in Git. Compare identities, structure, alignment, and
missingness exactly; establish numerical rules from repeated execution and
storage precision. Failure, missing prerequisites, and stale output reuse do
not establish a baseline.

All four scenarios now have raw outputs and decoded precipitation, flow, and
level arrays with 362 hourly timestamps and 8,649 mini-basins. Capture and fresh
repeat match exactly in values, masks, coordinates, and segment labels. The
versioned case records the saved-state limitations and the original SQLite
backup's shared-memory-sidecar modification-time exception. Later capture and
repeat use only the frozen snapshot and do not reopen the operational database.
