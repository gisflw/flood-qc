# MGB Ops Rust successor

This independent `main` history starts the Rust successor with migration
documentation only. Rust implementation has not started. The Python and curated
VB.NET references are available on `legacy/python` and `legacy/vbnet` in this
repository, checked out in sibling worktrees.

See the [worktree record](references/retirement_record.md) for current folders,
branch tips, upstreams, and historical retirement details.

## Read and execute in this order

1. **Use the frozen references.** Review the [retirement record](references/retirement_record.md),
   then use the [Python map](references/python_map.md) and
   [VB.NET and plugin map](references/vbnet_map.md) to locate behavior.
2. **Understand the migration.** Read the [migration summary](migration/summary.md).
3. **Implement incrementally.** Follow the responsibility files in
   [implementation plans](migration/summary.md).

## Agreed direction

- Keep the Python and VB.NET references on their independent legacy branches;
  `main` is the independent Rust successor history.
- Keep the data flow from `mgb_ops`, adding capabilities from the VB.NET system
  and its Python plugins. These systems were developed in parallel.
- Implement the application entirely in Rust, except for the external MGB
  executable. Build the CLI first; the desktop app is another interface.
- Support Windows and Linux, independent regional workspaces, scheduling, and email.
- Start with the executable in `rs_hydro_forecast/mgb_runner` and the calibration
  and geometry from the VB.NET system's `MGB/Input`.
- Reproduce the VB.NET hot-start behavior through reverse engineering.
- Include acquisition capabilities from both systems, except ONS meteorological
  forecast products. Include calibration editors.
- Use both systems' complementary bulletins as references. Export PDF,
  Markdown, HTML, and relevant CSV datasets.
- Share completed runs with enough information to regenerate inputs and outputs.
  All caches are optional; cache packaging is not a planning concern here.
- Decide behavioral disagreements and detailed technology choices during development.

The first version must support a complete operation. Reach it through small,
working increments rather than implementing every responsibility at once.
