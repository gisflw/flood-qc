# Registered run inputs

The workspace registers immutable versions of selected inputs and records their
identities in a run manifest. Station data, metadata, corrections, offsets,
flags and acquisition coverage are stored in `assets/registry.sqlite` as
revisions. Forecast grids, IMERG grids, model baselines and other file inputs
are shared objects under `assets/objects/`; object metadata includes checksums.
A change creates another version. A retained manifest continues to select its
original revision and objects after acquisition or editing.

## Manifest and payload

`assets/selections/` holds selection manifests referencing the exact observation
revision, registered files, resolved recipe and executable identity. Dated
attempt manifests in `runs/` record outcomes. These manifests depend on their
workspace's registry; they are not standalone archives. Generated forcing and
writable engine inputs are disposable working files. Input versions needed by
retained manifests remain available.

Application-owned TOML records use a `[record]` envelope. TOML has no null;
explicit null within dynamic records is represented by a tagged
`__mgb_value = "null"` table. Objects containing that reserved key are escaped,
so an absent field, a null, and a valid zero remain distinct. Formats required by
providers and external protocols are unchanged.

## Defaults and experiments

Workspace TOML supplies routine defaults; the shared session owns current
experiments and revision-checked job admission. Saving settings is explicit.
A run records resolved settings and does not supply defaults to a later run.
Historical rainfall explicitly selects stations or registered IMERG; see
[acquisition](acquisition.md#registered-imerg).

## Capture, export and replay

Capture registers selected inputs and records their identities; Run All adds
materialization, execution and publication. There is no current-input copy or
save/discard decision. Independent export copies the selected SQLite snapshot,
referenced objects and recipe into a verified portable input set. It does not
include the executable. Regeneration requires an installed executable matching
its recorded identity and a compatible engine platform and runtime. See the
[regional execution evidence](../../references/regression/rs-hourly/rust-qualification.md#current-execution-environment).

## Retention and failure behavior

Registered inputs and attempt manifests persist independently of disposable
working files and results. A failed or cancelled attempt keeps useful diagnostics
and leaves the previous successful results active. Successful results and
rainfall become active together. The latest successful results are retained by
default; explicitly keep additional results or export them before replacement.
See [execution and retention](operations.md#active-run-and-rainfall-cache).

Legacy independently saved inputs use the same external interchange exporter.
Register its verified canonical data with `mgb-ops inputs convert --workspace
/region --interchange /tmp/legacy-inputs`. The original set remains untouched;
its exact observation subset receives its own immutable SQLite object so absent
values cannot be filled from unrelated workspace observations.
