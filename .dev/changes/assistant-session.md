# Shared application session draft specification

This is the first increment of the [assistant plan](../plans/assistant.md).
Introduce the session that the desktop assistant and interactive CLI will use.
This increment connects existing desktop workflows to the session and provides
the headless entry points that the later CLI assistant will call.

## Ownership

App owns the process-local session and its workflow identity. The session
holds the resolved workspace and config selection, the run selection, and an
optional working draft. Construct it explicitly for each workspace open or
headless invocation. Use existing app resolution rules for paths and overrides.
Preserve explicit CLI workspace selection and external or symlinked config
paths. A draft's embedded workspace never changes the selected workspace.
[Architecture](../../docs/architecture.md#ownership) ·
[Workspace contract](../../docs/contracts/workspace.md)

The desktop owns the open forms, map and chart presentation, unsaved workspace
configuration form fields, job presentation, and departure dialogs. Its forms
edit the working draft through the session. Route worker replacements and direct
form edits through that same boundary. Keep one-shot CLI argument, path, and
output contracts. The session boundary supplies the later assistant increment
with context and guarded updates. ACP, MCP, and assistant interfaces follow in
their own increments.

## Run selection

Represent following the active generation and selecting an independently retained
run directory or bundle as distinct choices. Use the desktop Files run selector
as the session selection. Chart overlays remain presentation state. Reject
managed generation directories and their aliases as saved selections. A missing
active run is a valid state. When following active, each read resolves the
generation and reads the required artifacts under the existing shared
run-cache guard, then returns that generation's identity with the result.
Hold the guard through the artifact read so publication cannot retire the files
mid-query. Compose lock-aware readers
without reacquiring the same guard. Saved-run selection stays fixed across
active publication, and the desktop's active rainfall cache keeps following
publication independently.
[Operations contract](../../docs/contracts/operations.md#active-run-and-rainfall-cache) ·
[Desktop run selection](../../docs/contracts/desktop.md#data-and-selection)

Retain the desktop's existing per-scenario findings and partial results during
run loading. A failed read returns an error or finding for the selected source.
Desktop workers that were queued for a specific generation check that identity
before reading and refresh as needed. Capture session selections before sending
work to a background thread. Do file I/O on the worker's snapshot and discard
late results when their originating session or selection has been replaced.

## Draft lifecycle

Assign a new draft identity whenever the operator opens or creates a working
draft, even when the draft content matches the previous one.
Maintain a monotonic revision for changes to the working draft, including
direct form edits, refresh or source acceptance, and discard when it changes
the draft. Advance it when saving fixes the anchor and changes the edit rules.
A change followed by a reversal still advances the revision. Use
session identity, draft identity, and expected revision together when applying
an update produced from a snapshot. A missing or stale draft returns a conflict
without modifying the current draft. Map and chart presentation changes leave
the working revision alone.

Keep the existing draft anchor, schema, baseline, dirty state, and acceptance
rules. Provide a shared app entry point for validating a proposed working
document against the current draft. Reuse the draft schema and edit rules and
support incomplete drafts. Readiness for preparation remains a separate check.
Serialize the revision and draft job checks with applying an update so a job
cannot start between the checks and the commit. Apply changes only to the
working copy. Saving, accepting sources, registering instructions, preparing,
and starting an operation remain explicit workflows.
Keep the current desktop job and departure checks around actions with durable
effects. A late worker result check cannot undo a completed write.
[Draft contract](../../docs/contracts/drafts.md)

## Acceptance

- Explicit desktop and headless selections resolve the same workspace and config
  and give equivalent findings. Cover a missing config, a config outside the
  workspace, and a mismatched draft workspace.
- A concurrent publication leaves a read wholly within one active generation.
  Cover no active run, saved-run selection during publication, attempts to
  select a managed generation directly or by alias, and existing desktop
  behavior for a partially unreadable scenario set.
- Direct form edits and background draft updates use the revision boundary.
  Cover opening an identical draft, an edit followed by a reversal, discard,
  a save that fixes the anchor, and attempted application while a draft-changing
  job is pending. A rejected update leaves the working draft and saved bytes intact.
- A result from a closed or previous session cannot update the newly opened
  workspace. Failed workspace opening retains the last usable desktop state.
- Run the repository's Rust and desktop checks. Validate Linux first and
  record Windows evidence separately.
  [Validation guidance](../../AGENTS.md#changes-and-validation)
