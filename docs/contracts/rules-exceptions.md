# Repository rules and exceptions

This is the authoritative home for repository-wide engineering rules and their
exceptions. A dependency already present in the tree is not automatically an
approved exception. Record the affected behavior, reason, limits, evidence, and
condition for retiring each exception. An exception does not change unrelated
rules or establish precedent for other dependencies.

## Rule map

| Subject | Authoritative home |
| --- | --- |
| Engineering rules, dependency policy, exceptions, documentation and validation | This document |
| Product scope, module ownership, allowed dependencies and execution lifecycle | [Architecture](../architecture.md) |
| Supported commands, configuration, file formats and operator behavior | [Workspace](workspace.md) and the other documents in this directory |
| Desktop time presentation and interaction | [Desktop contract](desktop.md) |
| Setup, basic usage and navigation | [README](../../README.md) |
| Agent delegation, local source locations and final build handoff | [AGENTS.md](../../AGENTS.md) |
| Temporary capability progress, blockers, platform evidence and next work | [Migration checklist](../migration/summary.md) |
| Frozen expectations, scientific provenance and reproduction | [References](../../references/regression/rs-hourly/README.md) and the other records under `references/` |

Keep each fact in one authoritative home and link to it elsewhere. Preserve the
scope of broader plans when specifying one implementation increment.

## Implementation and ownership

Use the shared Rust library for CLI and desktop workflows. The external MGB
executable remains the numerical engine. Follow the architecture's ownership
table and lifecycle rules; add modules only for implemented behavior. Keep core
scientific processing independent of presentation, acquisition orchestration,
and implicit configuration or working-directory selection.

Rust privacy and the architecture gate enforce source boundaries. Keep checker
mechanics, allowlists, supported syntax and fixtures beside the checker. Extend
the checker for legitimate syntax rather than weakening an ownership rule to
accommodate a bypass. Review behavioral responsibility and lifecycle correctness
as well as import direction; passing the syntax gate establishes only its
documented source-level guarantees.

Define schemas, units, interval bounds and missing-value semantics when first
used. Preserve valid zero, unknown values, timezones, original inputs and
scientific provenance. Desktop controls display local times; timezone names and
UTC offsets appear only in the bottom status strip, as specified in the desktop
contract.

## Rust safety and dependency policy

Owned Rust code must contain no `unsafe`, including library, binaries, tests,
examples and build scripts. Keep Cargo's `unsafe_code = "forbid"`; do not weaken
the lint or move unsafe code into a local helper crate or generated wrapper.
Dependencies may use unsafe internally, but every API called by owned code must
be safe.

Application implementations must be Rust. Direct or transitive dependencies
must not compile, bundle, statically link, dynamically load or call C/C++
application libraries outside the approved exceptions below. A safe Rust API,
vendored build, optional feature or system installation does not itself grant
an exception. The rule covers default and desktop builds, normal/build/test
dependencies, and Linux and Windows targets.

Review the resolved feature and dependency graph, implementation sources and
build scripts when adding, updating or reconfiguring dependencies. Cargo's
package-local unsafe lint does not inspect dependency implementation languages.
Native-link metadata is a useful lead, not a complete inventory: system-library
dependencies and dynamically loaded libraries also need review. Record actual
platform linkage evidence separately from merely resolving its dependency graph.

The dependency rule covers the Rust/Cargo application and its build/test crates.
Independent external reference-capture tools may use recorded native decoders
for scientific qualification. Keep those tools out of the application runtime
and ordinary test prerequisites, and preserve their source and decoder identity
with the reference. They do not grant an application-library exception.

## Approved C-backed exceptions

| Rust/native path | Permitted scope | Reason and retirement condition |
| --- | --- | --- |
| `netcdf` → `netcdf-sys` → netCDF-C | Inundation-mask generation and validation, including input fixtures used solely to qualify that workflow | Incremental compressed writes keep regional masks within bounded memory. Replace when the Rust writer meets the qualification below. |
| `netcdf-sys` → `hdf5-metno-sys` → HDF5 | NetCDF-4 storage for the permitted mask I/O and qualification scope | Required by the approved NetCDF-4 implementation; retire with the native writer. |
| `netcdf-sys` → `libz-sys` → zlib/DEFLATE | Compression for the permitted mask I/O and qualification scope | Required for the current compressed HDF5 chunks; retire with the native writer. |
| Rust standard library and OS bindings such as `rustix`, `libc`, `windows` and `windows-sys` | Files, processes, sockets, clocks, signals and other platform services through safe APIs | Platform integration is permitted. This does not authorize C-backed application algorithms. |
| X11/XCB, XKB and Wayland bindings, including `x11-dl`, `xkbcommon-dl`, `wayland-sys` and the Wayland logging shim | Native window and keyboard integration through safe APIs, including libX11, libxcb, libxkbcommon and libwayland-client | Required platform integration for the selected desktop backends. |
| `wgpu` platform bindings and graphics loaders, including Vulkan/EGL and Windows graphics APIs | Rendering through safe Rust APIs | Graphics-driver integration is permitted; software-rendering evidence and hardware evidence remain distinct. |

Exact crate versions and enabled features are owned by
[Cargo.toml](../../Cargo.toml) and [Cargo.lock](../../Cargo.lock). Ordinary
scientific-array I/O and desktop mask reads continue through Rust. The separately
executed MGB numerical engine is the architectural engine boundary, not an
in-process Rust dependency or permission to add other C-backed application code.

### NetCDF replacement qualification

The pinned Rust writer behind Terrio currently materializes the complete encoded
file and does not expose incremental compressed output. Regional masks contain
tens of gigabytes of decoded pixels, so replacing the current writer with that
API would increase memory and storage requirements substantially.

Reassess the Rust writer when updating scientific I/O dependencies. Retire the
native exception when a Rust implementation supports bounded-memory incremental
writes, compatible chunking and compression, the supported mask metadata and
overview formats, and passes regional value, memory and performance checks.
The [inundation reference](../../references/regression/rs-hourly/inundation.md)
owns numerical evidence; the migration checklist owns current progress.

The native SDK has a known Linux descriptor-inheritance limitation: a child
launched while a native writer is open can retain its file lock after the parent
closes the writer. Isolating independent tests removes harness interference; it
does not repair the native SDK. Current qualification does not establish safe
overlap between native writing and unrelated launches in the same process.

### Existing deviations without approval

These paths are present in the resolved application graph. Listing them records
noncompliance; it does not extend the NetCDF or platform exceptions.

| Existing path | Native implementation | Status |
| --- | --- | --- |
| `reqwest` → Rustls → `aws-lc-rs` → `aws-lc-sys` | AWS-LC C and assembly | Unapproved application-crypto dependency; replacement or an explicit scoped exception remains open. |
| `lettre` and TLS verification → `ring` | C and assembly crypto primitives | Unapproved application-crypto dependency; replacement or an explicit scoped exception remains open. |

System netCDF builds can additionally link optional DAP/network, XML, filter,
compression, TLS and authentication libraries. The observed Linux system build
includes libraries from the libcurl, libxml2, libsz/libaec, Zstandard, Brotli,
liblzma, OpenSSL and GnuTLS/nettle families, with further transitive libraries.
This extra application-library linkage is an unapproved deployment-specific
deviation even when those optional functions are never invoked. It is not
covered by the narrow native-writer exception. Audit the actual release linkage
and any loadable filters; system closure compliance remains open. Cargo feature
unification currently includes both AWS-LC and ring, although their direct
feature-enabling routes differ. Windows native runtime linkage remains
unverified until checked on Windows.

## Documentation rules

Before writing, establish the responsibilities, relationships, invariants and
non-obvious behavior. Before creating a document, identify its unique reader
need and check for an existing document at the same planning level.

| Location | Content |
| --- | --- |
| `README.md` | Setup, basic usage and navigation |
| `docs/architecture.md` | Lasting scope, ownership, dependency, lifecycle and enforcement design decisions and their reasons |
| `docs/contracts/` | Supported behavior, configuration and formats; this file owns repository governance |
| `docs/migration/` | Temporary migration scope, capability progress, evidence, gaps and next work; checklist in `summary.md` |
| `references/` | Legacy source maps, frozen expectations, irreplaceable provenance and reproduction |
| `.dev/plans/` | Broader functionality plans spanning implementation increments; retain their full scope |
| `.dev/changes/` | One increment's scope, interactions, acceptance criteria and validation |
| `.dev/reports/` | Brief disposable implementation/review records; historical evidence, not architecture or required reading |

A broader plan does not replace an implementation specification. Put an
increment's specification in `.dev/changes/` and link to its broader plan; do
not narrow or repurpose the plan without an explicit request to change it.
Do not create a completion report by default.

Retire plans and specifications after their entire scope is accepted or
abandoned. Acceptance of one increment does not retire its broader plan, and a
specification stays while acceptance is disputed. Before retiring migration
documents, preserve unique durable decisions in architecture, supported behavior
in contracts, and scientific provenance in references. Repair links and remove
temporary navigation. Reports may remain for Git history and later be deleted;
do not move them into permanent documentation.

Keep private helpers, checker allowlists, parser mechanics and test procedures
in code/tests. Omit agent/model names, fixing-round counts, installation
narratives and temporary paths from project documentation. Keep scientific
caveats with the reference they qualify. Do not turn implementation workarounds
into architectural requirements. Resolve links relative to their containing
document and apply these rules to delegated work too.

## Validation and evidence

Use the locked dependency graph and selected toolchain. For Rust changes run:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test architecture
```

For desktop changes also run:

```sh
cargo clippy --locked --features desktop --all-targets -- -D warnings
cargo test --locked --features desktop
cargo build --locked --features desktop --bin mgb-ops-desktop
```

Clippy uses its default group plus `dbg_macro`, `todo` and `unimplemented`.
Lint exceptions must be narrowly scoped and give a concrete reason. Documentation
changes require link and content checks; documentation alone does not require
rerunning Rust tests. Graphics tests remain explicit ignored checks.

While migration is active, update the checklist when capabilities, evidence,
blockers or next work change. Distinguish planned from implemented behavior.
State platform evidence explicitly: Linux first; Windows remains unverified
until checked. Prerequisite inspection establishes neither engine execution,
numerical equivalence nor hydrological validity. Preserve frozen comparison
requirements when an execution environment changes; record discrepancies and
limits rather than silently weakening scientific acceptance.

## Sources, experiments and handoff

[AGENTS.md](../../AGENTS.md#sources-and-experiments) records current local source
and external data locations. Preserve legacy sources, operational data,
historical provenance and frozen evidence. Keep large data, binaries and
credentials outside Git. Disposable captures, repeats, probes and validation
experiments, including scripts, logs and review outputs, belong under this
repository's gitignored `runs/`; remove them after review. Preserve accepted
frozen reference artifacts separately.

After validation, follow the [handoff](../../AGENTS.md#handoff): remove only debug
build artifacts, build both release binaries, and provide working terminal
commands. Never remove the entire `target/` tree while it holds external
workspaces or frozen data. Report checks and material qualification limits
accurately, and reconcile the local checkout after merging its PRs.
