# Current worktrees and historical Git retirement record

Current state checked on 2026-10-21. The three worktrees share one Git object
database at `/workspace/legacy_python/.git` and all track their corresponding
`origin` branches.

## Current worktrees

| Folder | Branch | Current commit |
| --- | --- | --- |
| `/workspace/mgb_ops` | `main` (`origin/main`) | `1494090e1208a018271f61d28ad0df4d240e4ad7` |
| `/workspace/legacy_python` | `legacy/python` (`origin/legacy/python`) | `7ecc86a21416b7601384715062ec4c41557d9313` |
| `/workspace/legacy_vbnet` | `legacy/vbnet` (`origin/legacy/vbnet`) | `10b61520d43512e8a5923989f38c0745398b01c5` |

All three worktrees were clean before this documentation refresh; these edits
leave the `main` worktree modified. `legacy/python` contains 145 reachable
commits. `legacy/vbnet` contains 264 tracked files, including its
retained-file inventory at `references/inventory.csv`; the full inventory at
`references/inventory.full.csv` is local-only and ignored by Git.

## Historical retirement details (2026-10-15)

The original Python tip was `476602e1b7a0f3d70ffc9ce8400761020ebe37e9`, and
the original VB.NET snapshot tip was
`349eee89ca5db4cf7befb7fb2b5f30ea6c1da2e5`. The original record described 144
Python commits and 822 retained VB.NET source files; those counts and tips are
historical, not current state. The VB.NET reference has since been reduced to a
code-focused snapshot, with its current contents documented by its tracked
inventory and local-only full inventory.

The `main` branch has an independent parentless root and contains migration
documentation, metadata, and ignore rules; Rust implementation has not started.

## Inspecting references

Run these from any of the three worktrees to inspect the shared histories:

```sh
git worktree list
git log legacy/python
git show legacy/vbnet:README.md
git show legacy/vbnet:references/inventory.csv
git show legacy/vbnet:references/redactions.json
git show legacy/vbnet:references/source_gaps.md
```

The VB.NET reference worktree is `/workspace/legacy_vbnet`. Its source paths
begin with `Sistema_de_Previsao_RSH_2025_versao0.99/`. The
[VB.NET responsibility map](vbnet_map.md) uses paths relative to that source
root; the [Python map](python_map.md) uses paths within the Python checkout.

## Model baseline and verification

[Model baseline metadata](model_baseline.json) records the selected external
runner's identity and SHA-256 plus the files in the original VB.NET `MGB/Input/`
folder as recorded for the retirement. Those original paths and hashes are
historical provenance; binaries and extensive datasets remain outside Git.
Executable/input compatibility and hot-start behavior have not been tested;
the two existing input folders must not be treated as interchangeable.

[Machine-readable retirement metadata](retirement_metadata.json) separates
current worktree state from the October 15 retirement record. Historical
manifest fingerprints and checks describe the preparation then; they have not
been rerun as part of this current-state update.

The October 15 preparation did not update remote refs. The current worktrees
now track `origin/main`, `origin/legacy/python`, and `origin/legacy/vbnet`.
The unrelated `rs_hydro_forecast` checkout still has its local configuration
edit and untracked inventory script; these are outside this repository's
worktree state.
