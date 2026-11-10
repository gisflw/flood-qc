# Agent guidance

Read [README.md](README.md) for setup, [the architecture](docs/architecture.md)
for ownership rules, [the workspace contract](docs/contracts/workspace.md) for
supported behavior, and [the migration summary](docs/migration/summary.md) for
current status and next work. The external MGB executable remains the numerical engine.

## Documentation rules

Keep repository documentation focused on current usage, durable design decisions
and their reasons, migration scope, and irreplaceable provenance. Give each fact
one authoritative home and link to it elsewhere.

- `README.md`: setup, basic usage, and navigation.
- `docs/architecture.md`: lasting ownership, dependency, and lifecycle decisions.
- `docs/contracts/`: externally supported behavior, configuration, and formats.
- `docs/migration/`: capability status and migration scope; the checklist lives in
  `summary.md`. Record brief validation evidence, gaps, and intentional departures there.
- `references/`: legacy source maps, frozen expectations, provenance, and reproduction.
- `.dev/plans/` and `.dev/changes/`: temporary plans and specifications for active work.
- `.dev/reports/`: disposable implementation/review results, retained for Git history
  when useful. These are historical records, not architectural authority or required reading.

Before creating a document, identify its unique reader need and check whether an
existing document already serves it. Do not create a completion report by default.
When a report is useful or requested, put it under `.dev/reports/` and keep it brief.
Keep private helpers, checker allowlists, parser mechanics, and test procedures in
code/tests. Omit agent/model names, fixing-round counts, installation narratives,
and temporary paths from project documentation. Keep necessary scientific provenance
and numerical caveats with the reference they qualify.

Remove plans/specifications after acceptance or abandonment, retaining only unique
durable information in its proper home and repairing links. Keep a specification
while its acceptance remains disputed. Reports may remain for a history commit and
be removed later; do not relocate them into permanent documentation. Do not promote
implementation workarounds into architectural requirements. Apply these rules to
delegated work too. Resolve links relative to the containing document.

## Changes and validation

Follow the architecture; add modules only for implemented behavior. Use Rust privacy
and the architecture test to enforce ownership. Keep checker mechanics beside the
checker; test and review behavioral responsibilities as well as import direction.
Owned Cargo targets forbid unsafe code; dependencies may use unsafe internally.
Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`,
and the independently runnable `cargo test --test architecture` for Rust changes.
Documentation-only changes need link/content checks, not a Rust test rerun.

Update the migration checklist when capabilities, evidence, blockers, or next work
change. Distinguish planned work from implemented behavior. Keep platform evidence
explicit: Linux first, Windows unverified until checked. Prerequisite inspection
cannot establish engine execution, numerical equivalence, or hydrological validity.

## Sources and experiments

Legacy worktrees are `../legacy_python/` (`legacy/python`) and `../legacy_vbnet/`
(`legacy/vbnet`); use the maps in `references/`. The regional runtime is
`../rs_hydro_forecast/`; frozen regression artifacts are outside Git at
`/workspace/regression-rs-hourly-r1` (see [the case record](references/regression/rs-hourly/README.md)).
Preserve legacy sources, operational data, historical provenance, and frozen evidence.
Use disposable workspaces for experiments; keep large data, binaries, and credentials
outside Git.
