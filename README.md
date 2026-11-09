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
3. **Implement incrementally.** Follow the responsibility files in
   [implementation plans](docs/migration/summary.md).

## Repository structure

- `docs/migration/`: broad migration roadmap and responsibility areas.
- `docs/reports/`: completed work, reviews, and validation evidence.
- `references/`: legacy source maps, provenance, and compact regression evidence.
- `.dev/`: temporary implementation documentation; `plans/` holds general plans
  for agreed behavior awaiting implementation, and `changes/` holds detailed
  specifications ready for implementation.

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

First establish a minimal reproducible regression target covering the main
operational steps and a checklist of functionality to port. Reach the complete
operation through small, working increments, refining checks during implementation.
Check platform feasibility early. Leave legacy tools that cannot be mapped to
source behavior aside; they do not block this process.
