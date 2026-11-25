# Run input sets

The [workspace](workspace.md#operating-layout) keeps reusable observations and
normalized forecasts, and operators can select imported wind and reservoir
tables. See [acquisition](acquisition.md) for coverage, refresh and reuse.
After normalization, runs use the canonical inputs independently of raw downloads.

## Manifest and payload

A run freezes its selected input values and effective configuration into one
self-contained set under `cache/inputs/`. The set includes observations and review
instructions, station metadata/offsets, normalized forecast bases, selected wind
and reservoir data, model inputs/calibration/initial state, and an executable
identity. `input-set.json` indexes every retained file by path, length and SHA-256
and records the resolved recipe. Each run has its own copies. Derived scenarios,
interpolated forcing and results can be regenerated and are excluded from a saved
input set. The executable must be installed separately and match the recorded
identity. Exact numerical replay also depends on the engine’s platform and
runtime, which are not packaged in the input set. See the [regional execution
evidence](../../references/regression/rs-hourly/rust-qualification.md#current-execution-environment).

```text
<input-set>/
├── input-set.json              Identity, resolved recipe and complete payload index
├── baseline/
│   └── Input/                  Frozen model configuration, calibration and state
└── sources/
    ├── recipe.json                 Indexed copy of the resolved recipe
    ├── observations/               Selected canonical data, flags and edits
    ├── forecasts/                  Selected normalized bases and metadata
    ├── wind-observations.parquet    Optional selected wind table
    └── reservoir-schedules.parquet  Optional selected reservoir table
```

The tree shows retained payload categories. A materialized preparation also
contains generated files; see [prepared execution files](operations.md#prepared-dependencies-and-run-files).

| `input-set.json` field | Meaning |
| --- | --- |
| `version` | Input-set format version |
| `id`, `created_at` | Set identity and creation label |
| `engine` | Executable reference: `path`, `kind`, `size` in bytes and `sha256`; executable bytes are excluded from the saved set |
| `recipe` | Resolved configuration, selected data, ordered scenario recipes and engine settings |
| `files` | Complete retained payload index; each entry has `path`, `kind`, `size` and `sha256` |

Opening or replaying a set verifies its payload index, recipe and engine binding.
Files containing only hashes or mutable source paths cannot substitute for
retained input values.

## Defaults and experiments

TOML supplies defaults to the shared application session. Session experiments
are revision checked and captured atomically with job admission. Saving workspace
settings is an explicit action. A past run never silently supplies defaults.
The run manifest records resolved values and its input-set identity, including
configuration that may later change.

| Source | Responsibility | Persistence rule |
| --- | --- | --- |
| Workspace TOML | Routine defaults, including configured scenarios | Only through an explicit settings save |
| Shared session | Current experiment parameters and revision-checked job admission | Session changes do not save TOML |
| Frozen recipe and run manifest | Resolved historical values and identities | Historical records do not update TOML or supply defaults |

## Capture, save and replay

```sh
mgb-ops inputs capture --workspace /region --json
mgb-ops inputs save --workspace /region --output /archive/inputs-july20
mgb-ops inputs open --inputs /archive/inputs-july20
mgb-ops inputs execute --workspace /region --inputs /archive/inputs-july20 \
  --engine /region/model/engine --discard-inputs --json
mgb-ops inputs discard --workspace /region
```

```mermaid
flowchart LR
    accTitle: Input capture, independent saving and replay
    accDescr: Acquisition and selection freeze current inputs. Inputs may be saved independently or materialized for execution. Saved inputs can be replayed with a matching installed engine.
    acquire[Acquire and select canonical data] --> current[Freeze current input set]
    current --> save[Verify and sync independent save]
    current --> prepare[Materialize forcing and engine inputs]
    save --> replay[Replay with matching installed engine]
    replay --> prepare
    prepare --> execute[Execute and validate]
    execute --> publish[Publish successful results]
```

Capturing inputs acquires configured sources and stops before MGB execution.
Run All composes acquisition, capture, materialization, execution and publication.
Before replacing unsaved current inputs, callers must choose save or discard.
Use `--save-inputs-to PATH` or `--discard-inputs` with Run All, `inputs
capture`, `inputs execute`, `operation execute` or `run regenerate`. Omitting
both returns `InputDecisionRequired` when the workspace has unsaved inputs. Automation steps use `inputs = { action = "discard" }` or
`inputs = { action = "save", destination = "..." }`. The desktop offers
save/discard/cancel before a new run and when closing or switching workspaces.

## Retention and failure behavior

Current inputs are an atomically published cache: they survive ordinary process
failures, but may be lost or incomplete after a power loss. Saving makes an
independent verified copy and syncs it before marking the inputs saved. Explicit
Discard also resets an incomplete input cache after a power loss. Discarding removes current inputs;
run manifests remain, but regeneration requires a saved input set. A failed engine
run retains its newly frozen inputs and leaves the previous successful results
active. Inputs and successful results therefore have independent lifecycles.
Users choose how long to retain independently saved inputs and exported results.

| Action or outcome | Current inputs | Active successful results |
| --- | --- | --- |
| Save inputs | Remain current and are marked saved after verified independent publication | Unchanged |
| Discard inputs | Removed; past manifests remain | Unchanged |
| Capture new inputs after resolving retention | New frozen set becomes current | Unchanged |
| Engine execution fails | Newly frozen set remains available | Previous successful generation remains active |
| Execution and publication succeed | Frozen set remains current | New results and rainfall become active together |
| Export completed results | Unchanged | Independent result bundle is retained at the chosen destination |

Saving inputs supports regeneration; exporting results preserves completed
outputs. Neither action implies the other. A completed-run bundle may include
matching inputs through the [sharing workflow](saved-data.md#sharing-and-automation).
