# Agent guidance

Read [the repository rules and exceptions](docs/contracts/rules-exceptions.md),
[README.md](README.md) for setup, [the architecture](docs/architecture.md)
for ownership rules, [the workspace contract](docs/contracts/workspace.md) for
supported behavior, and, while migration is active,
[the migration summary](docs/migration/summary.md) for current status and next work.
The external MGB executable remains the numerical engine.

## Delegation conventions

Proactively delegate read-only investigations of unfamiliar subjects to sub-agents
when summarized findings are sufficient. Parallelize independent investigations
where practical.

Typical use cases include:
- Exploring unfamiliar areas of the codebase.
- Researching external libraries, technologies and documentation.
- Gathering context before planning or making architectural decisions.
- Building a conceptual understanding of the system before writing architectural documentation.

Sub-agents reasoning set: {6.1 Terra high}

## Documentation and validation

Follow the [documentation rules](docs/contracts/rules-exceptions.md#documentation-rules)
and [validation requirements](docs/contracts/rules-exceptions.md#validation-and-evidence).
Keep one authoritative home for each fact. The architecture owns module and
lifecycle decisions; the rules document owns cross-cutting requirements and
exceptions. Update the migration checklist when capabilities or evidence change.
Do not weaken `unsafe_code = "forbid"` or extend a C-backed exception implicitly.

## Sources and experiments

Legacy worktrees are `../legacy_python/` (`legacy/python`) and `../legacy_vbnet/`
(`legacy/vbnet`); use the maps in `references/`. The regional runtime is
`/workspace/mgb-ops/target/rs_hydro_forecast/`; frozen regression artifacts are outside Git at
`/workspace/mgb-ops/target/regression-rs-hourly/` (see [the case record](references/regression/rs-hourly/README.md)).
The regional workspace is `/workspace/mgb-ops/target/workspace/`.
Follow the [preservation and experiment rules](docs/contracts/rules-exceptions.md#sources-experiments-and-handoff).
The `target/` directory contains external data as well as build output: never
run an unrestricted `cargo clean` or remove the whole directory.

## Handoff

After finishing the work and required validation, delete only the debug build
and build release versions of both the operating desktop and CLI:

```sh
cargo clean --profile dev
cargo build --locked --release --features desktop --bins
```

Verify the release CLI and report the result of desktop startup validation.
Keep platform and graphics limitations explicit. Give the operator the terminal
commands from the repository root; adding the release directory to PATH makes
`mgb-ops` and `mgb-ops-desktop` resolve to these builds:

```sh
export PATH="$PWD/target/release:$PATH"
mgb-ops --help
mgb-ops-desktop /selected/workspace
```

Do not run a debug build after this final cleanup. Preserve external data under
`target/`, and leave disposable validation outputs under `runs/` . After merging PRs, reconcile the local checkout and preserve pending work  
before reporting the repository state.
