# MGB Ops Rust successor

This independent `main` history starts the Rust successor with migration
documentation only. Rust implementation has not started. The Python and curated
VB.NET references are available on `legacy/python` and `legacy/vbnet` in this
repository, checked out in sibling worktrees.

See the [worktree record](references/retirement_record.md) for current folders,
branch tips, upstreams, and historical retirement details.

The first [regression reference](references/regression/rs-hourly/README.md)
captures four RS scenarios and a fresh repeat through decoded results. All
repeated arrays match exactly. Large artifacts are preserved outside Git at
`/workspace/regression-rs-hourly-r1`; see the
[review report](docs/reports/regression-reference-capture.md) for validation and
the documented adaptations to the legacy capture path.

## Read and execute in this order

1. **Use the frozen references.** Review the [retirement record](references/retirement_record.md),
   then use the [Python map](references/python_map.md) and
   [VB.NET and plugin map](references/vbnet_map.md) to locate behavior.
2. **Understand the migration.** Read the [migration summary](docs/migration/summary.md).
3. **Implement incrementally.** Start with the detailed
   [Rust foundation specification](.dev/changes/rust-foundation.md), which
   refines the agreed [plan](.dev/plans/rust-foundation.md). Track delivered
   capabilities, evidence, gaps, and the next increment in the
   [migration summary](docs/migration/summary.md).

## Repository structure

- `docs/migration/`: broad migration roadmap and responsibility areas.
- `docs/architecture.md` and `docs/contracts/`: durable architecture and contracts
  to establish with the Rust foundation.
- `docs/reports/`: completed work, reviews, and validation evidence.
- `references/`: legacy source maps, provenance, and compact regression evidence.
- `.dev/`: disposable working documentation, not long-lived project records.
  Use descriptive filenames without numbering; `plans/` holds agreed behavior
  awaiting implementation, and `changes/` holds detailed implementation
  specifications.

Keep durable results in `docs/reports/` or `references/` when retiring temporary
implementation documentation. See [AGENTS.md](AGENTS.md) for agent guidance.

## Agreed direction

- Keep the Python and VB.NET references on their independent legacy branches;
  `main` is the independent Rust successor history.
- Keep the data flow from `mgb_ops`, adding capabilities from the VB.NET system
  and its Python plugins. These systems were developed in parallel.
- Implement the application entirely in Rust, except for the external MGB
  executable. All intended functionality belongs in the Rust package, including
  behavior currently in regional runtime scripts. Build the CLI first, introduce
  the desktop early, and extend both interfaces alongside the shared core.
- Use one Cargo package with four core modules: `store` owns canonical contracts
  and private persistence; `adapters`, `analysis`, and `model` independently
  depend on store and never import each other. Store's data contracts are usable
  without database connections or filesystem state.
- Share operational workflows in the library's `app` module. It owns
  configuration/path resolution, selection, preparation/scenario coordination,
  failure handling, and publication policy. CLI and desktop use those workflows;
  configuration helpers remain private within app.
- Analysis prepares canonical forcing and renders reports; model serializes
  prepared inputs, executes the engine, and stages decoded canonical outputs.
  At execution startup, freeze input values or immutable versions. Publish only
  a complete requested scenario set through store; failed attempts retain
  diagnostics/staged outputs and preserve the previous published set. These
  operational requirements follow the read-only foundation increment.
- Enforce the dependency graph automatically from the foundation, including
  application/root bypasses and derive attributes. SQLite and NetCDF remain
  starting choices; settle exact schemas and file layouts with each capability.
- Support Windows and Linux, independent regional workspaces, scheduling, and email.
- Use `rs_hydro_forecast` as the target runtime workspace. Establish the initial
  regression target from its existing executable and compatible inputs,
  including its calibration and geometry.
- Reproduce the VB.NET hot-start behavior through reverse engineering.
- Include acquisition capabilities from both systems, except ONS meteorological
  forecast products. Port calibration/geometry editors as functionality with
  functional acceptance; no editor regression or baseline replacement is required.
- Use both systems' complementary bulletins as references. Export PDF,
  Markdown, HTML, and relevant CSV datasets.
- Share completed runs with enough information to regenerate inputs and outputs.
  All caches are optional; cache packaging is not a planning concern here.
- Decide behavioral disagreements and detailed technology choices during development.

The initial regression reference is established, and the first Rust foundation
specification is complete; implementation has not started. Reach the complete operation
through small, working increments, updating the capability checklist and its
evidence as development proceeds. Validate on Linux first and defer Windows
checks until later development. Leave legacy tools that cannot be mapped to
source behavior aside; they do not block this process.
