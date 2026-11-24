# Agent guidance

Read [README.md](README.md) for setup, [the architecture](docs/architecture.md)
for ownership rules, [the workspace contract](docs/contracts/workspace.md) for
supported behavior, and, while migration is active,
[the migration summary](docs/migration/summary.md) for current status and next work.
The external MGB executable remains the numerical engine.

## Desktop conventions

Show timezone names and UTC offsets only in the bottom status strip.
Display local times without timezone information in the other desktop controls.

## Documentation rules

Keep permanent documentation focused on current usage, lasting product scope,
durable design decisions and their reasons, and irreplaceable provenance. Give
each fact one authoritative home and link to it elsewhere.

- `README.md`: setup, basic usage, and navigation.
- `docs/architecture.md`: lasting product scope, ownership, dependency, lifecycle,
  and enforcement decisions.
- `docs/contracts/`: externally supported behavior, configuration, and formats.
- `docs/migration/`: temporary migration scope, capability progress, evidence, gaps,
  and next work; the checklist lives in `summary.md`. Retire it after migration.
- `references/`: legacy source maps, frozen expectations, provenance, and reproduction.
- `.dev/plans/`: broader functionality plans spanning multiple implementation
  increments; preserve their full scope as individual changes are specified.
- `.dev/changes/`: specifications for a specific implementation increment,
  including its scope, interactions, acceptance criteria and validation.
- `.dev/reports/`: disposable implementation/review results, retained for Git history
  when useful. These are historical records, not architectural authority or required reading.

Before creating a document, identify its unique reader need and check whether an
existing document at the same planning level already serves it. A broader plan
does not replace an implementation specification: put the latter in `.dev/changes/`
and link to the plan. Do not rewrite, narrow or repurpose a broader plan to record
one implementation increment unless the user explicitly requests a change to the
broader plan itself. Do not create a completion report by default.
When a report is useful or requested, put it under `.dev/reports/` and keep it brief.
Keep private helpers, checker allowlists, parser mechanics, and test procedures in
code/tests. Omit agent/model names, fixing-round counts, installation narratives,
and temporary paths from project documentation. Keep necessary scientific provenance
and numerical caveats with the reference they qualify.

Remove plans/specifications after their full scope is accepted or abandoned;
acceptance of one implementation increment does not retire its broader plan.
Retain only unique
durable information in its proper home and repairing links. Keep a specification
while its acceptance remains disputed. Before removing migration documents, retain
unique lasting scope/design decisions in architecture, supported behavior in
contracts, and scientific evidence/provenance in references; remove temporary
navigation links. Reports may remain for a history commit and be removed later;
do not relocate them into permanent documentation. Do not promote
implementation workarounds into architectural requirements. Apply these rules to
delegated work too. Resolve links relative to the containing document.

## Changes and validation

Follow the [architecture and its enforcement rules](docs/architecture.md#enforcement);
add modules only for implemented behavior. Keep checker mechanics beside the
checker; test and review behavioral responsibilities as well as import direction.
Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
`cargo test --locked`, and the independently runnable
`cargo test --locked --test architecture` for Rust changes.
For desktop changes, also run `cargo clippy --locked --features desktop
--all-targets -- -D warnings`, `cargo test --locked --features desktop`, and
`cargo build --locked --features desktop --bin mgb-ops-desktop`. Graphics tests
remain explicit, ignored checks; distinguish software rendering from hardware evidence.
Clippy uses its default group plus `dbg_macro`, `todo`, and `unimplemented`;
exceptions must be narrowly scoped and include a concrete reason. Documentation-only
changes need link/content checks, not a Rust test rerun.

While migration is active, update its checklist when capabilities, evidence,
blockers, or next work change. Distinguish planned work from implemented behavior.
Keep platform evidence explicit: Linux first, Windows unverified until checked. Prerequisite inspection
cannot establish engine execution, numerical equivalence, or hydrological validity.

## Sources and experiments

Legacy worktrees are `../legacy_python/` (`legacy/python`) and `../legacy_vbnet/`
(`legacy/vbnet`); use the maps in `references/`. The regional runtime is
`../rs_hydro_forecast/`; frozen regression artifacts are outside Git at
`/workspace/regression-rs-hourly` (see [the case record](references/regression/rs-hourly/README.md)).
Preserve legacy sources, operational data, historical provenance, and frozen evidence.
Run disposable captures, repeats, probes and validation experiments under this
repository's gitignored `runs/` directory, including their scripts, logs and
review outputs. Do not scatter them in `/tmp` or sibling workspace directories;
remove them after review. Preserve accepted frozen reference artifacts separately.
Keep large data, binaries, and credentials outside Git.
