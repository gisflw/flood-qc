# Historical Git retirement specification

This records the October 15, 2026 preparation plan. It is historical: the
current worktree folders, branch tips, upstreams, and VB.NET snapshot inventory
are recorded in the [current worktree record](retirement_record.md).

## Result

Use the existing `mgb_ops` repository for all three lines of history:

| Branch | Purpose |
| --- | --- |
| `legacy/python` | Frozen existing Python implementation with its complete commit history, unchanged. |
| `legacy/vbnet` | A disconnected reference snapshot of VB.NET and its bundled plugins, with source maps and ignore rules. |
| `main` | Rust successor starting with a clean tree and a new parentless root commit, disconnected from both legacy histories. |

The branch names were proposed in this specification and are now established.
The reference branches have since received commits; see the current record for
their present tips.

## 1. Freeze Python

The inspected Python commit is
`476602e1b7a0f3d70ffc9ce8400761020ebe37e9` (`improve dashboard`).
The working tree was clean when this plan was prepared.

Before implementation, verify the intended tip and point `legacy/python` at it.
Preserve every existing Python commit and the current files without cleanup,
squashing, or restructuring. The complete existing history remains reachable
through this legacy branch when `main` is restarted.

## 2. Prepare the VB.NET snapshot

Source directory: `Sistema_de_Previsao_RSH_2025_versao0.99` beside `mgb_ops`.
Prepare a curated copy in an isolated staging directory. Preserve the original
relative layout so project resources and source references remain understandable.
Create a parentless reference commit and bring it into the same repository as
`legacy/vbnet`. Do not replace the original operational directory or add a second
maintained repository.

Keep:

- VB.NET application and plugin source, Python plugin source, solutions,
  projects, dependency manifests, and packaging specifications.
- Application resources, icons, manuals, and useful operational documentation.
- Model calibration, geometry, station definitions, configuration examples,
  and manageable representative input files.
- A snapshot README and the reference maps, with source locations, omissions,
  and components for which source could not be found.

Exclude with a branch-owned `.gitignore` and a reviewed staging inventory:

- Compiled executables, DLLs, debug symbols, and other build products.
- Bundled Python environments, third-party package trees, `bin`, `obj`,
  `build`, `dist`, IDE state, and bytecode caches.
- Model and plugin outputs, logs, temporary files, downloaded archives,
  extensive telemetry histories, and large forcing/input datasets.
- Credentials, local `.env` files, and embedded secrets discovered in source.
  Replace secret values with placeholders in the reference copy.

Use path-specific exclusions where needed. Do not blanket-ignore `.txt`, `.csv`,
`.hig`, `.gtp`, `.mgb`, or `.HOT`: these extensions also hold essential reference
inputs. Record excluded large inputs in an inventory with their original path
and purpose. Missing source for executable-only tools must be documented.

## 3. Preserve the model baseline

Record the executable identity from `rs_hydro_forecast/mgb_runner` and identify
the selected calibration/geometry baseline in the other system's `MGB/Input`.
Keep executable files and extensive datasets outside Git. Retain enough metadata
to locate the baseline and understand how to regenerate test material.

Do not assume the two current input folders or their hot-start files are
interchangeable. Check the selected files against the chosen executable during
the model implementation work.

## 4. Restart main cleanly

After verifying both reference branches, recreate `main` as an orphan history
with a new parentless root commit and a clean tree. It must inherit neither the
Python files nor any Python commits. Deleting files in a normal descendant commit
does not satisfy this requirement: the Rust history must start independently.

Add the migration documentation and new Rust project files deliberately to this
new history. Keep all existing Python commits on `legacy/python` and the curated
VB.NET snapshot on `legacy/vbnet`; neither reference history is merged into `main`.

## Completion check

Both legacy branches resolve to recorded commits in the same repository. Every
existing Python commit remains reachable through `legacy/python`, unchanged.
The new `main` has a parentless root, no shared ancestry with either reference
branch, and no inherited Python source files.
The VB.NET snapshot has its documentation and ignore rules, useful
resources are retained, and generated data, bundled dependencies, and secrets are
absent from the staged snapshot. Existing operational folders remain intact.
