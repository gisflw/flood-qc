# Local Git retirement record

Prepared on 2026-10-15. These three histories share one Git object database at
`/workspace/mgb_ops/.git`; the new worktrees are not independent clones.

## Workspace locations

| Folder | Branch or state | Preserved commit |
| --- | --- | --- |
| `/workspace/mgb_ops` | Frozen `legacy/python` | `476602e1b7a0f3d70ffc9ce8400761020ebe37e9` |
| `/workspace/mgb_ops_vbnet_reference` | Frozen `legacy/vbnet` | `349eee89ca5db4cf7befb7fb2b5f30ea6c1da2e5` |
| `/workspace/mgb_ops_rust` | New documentation-only `main` | Resolve with `git rev-parse main` |
| `/workspace/Sistema_de_Previsao_RSH_2025_versao0.99` | Original operational folder; no Git checkout added | Original files preserved |
| `/workspace/rs_hydro_forecast` | Existing `master`, including its local `config/custom.yaml` edit | `a30b72cae0207e5b2fcfd48a1b70ca00052f97a1` |
| `/workspace/mgb_ops_rust_plan` | Original planning folder | Original files preserved |

The Python branch preserves all 144 original commits and the original
checkout files. Its former `origin/main` upstream was removed. Neither frozen
reference branch receives Rust changes.

The VB.NET branch is a curated parentless snapshot with 822 retained
original files and its own snapshot documentation. All 23420 original files
have inventory entries, including excluded environments/build trees and large
inputs. Inventory entries contain original hashes and snapshot hashes for
retained files. Identified secrets in 14 source/configuration files were
redacted in the copy only, including station password columns and prefilled FTP
password controls. Dynamic credential expressions and unredacted files retain
their original content.

`main` starts with a new parentless commit and inherits no Python/VB.NET source
or history. It contains migration documentation, records, metadata, and ignore
rules; Rust implementation has not started.

## Inspecting references

Run these from any of the three worktrees:

```sh
git worktree list
git log legacy/python
git show legacy/vbnet:README.md
git show legacy/vbnet:references/inventory.csv
git show legacy/vbnet:references/redactions.json
git show legacy/vbnet:references/source_gaps.md
```

Within the VB.NET reference worktree, source paths begin with
`Sistema_de_Previsao_RSH_2025_versao0.99/`. The
[VB.NET responsibility map](vbnet_map.md) uses paths relative to that source
root; the [Python map](python_map.md) uses paths within the Python checkout.

## Model baseline and verification

[Model baseline metadata](model_baseline.json) records the selected external
runner's identity and SHA-256 plus all 50 files in the original VB.NET
`MGB/Input/` folder. Binaries and extensive datasets remain outside Git.
Executable/input compatibility and hot-start behavior have not been tested;
the two existing input folders must not be treated as interchangeable.

[Machine-readable retirement metadata](retirement_metadata.json) records the
frozen commit IDs, source-manifest fingerprints, and verification criteria.
Verification checks exact Python commit reachability, independent roots, clean
worktrees, valid documentation links, Git connectivity, and original file
contents/modes/symlinks against pre-retirement SHA-256 manifests. The original
operational folders and planning folder are preserved; the existing regional
configuration edit remains intact.

Both delegated tasks used one Luna/high pass, followed by the primary agent's
review and corrections to credential handling, complete inventory coverage,
and missing-source documentation. Superseded draft Git objects from preparation
were removed without pruning original history or unrelated objects.

Everything remains local. `origin/main` and all pre-existing remote-tracking
refs remain unchanged, and the new `main` has no upstream. No application build,
Rust scaffold, model execution, or compatibility test was performed.
