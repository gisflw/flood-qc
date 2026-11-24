# Desktop and CLI assistant specification

This is the third increment of the [assistant plan](../plans/assistant.md).
It adds operator interfaces to the [shared session](assistant-session.md) and
[agent command layer](assistant-protocol.md). Both interfaces select an agent
directly, stream responses, and provide the same draft review and apply action.

## Desktop and CLI entry points

Add an Assistant pane to the desktop with agent selection, a transcript,
prompt entry, permission decisions, Stop, and conversation reset. Bind it to
the open workspace session and respect its run selection and open draft.
Run protocol I/O and data queries off the UI thread. Keep the map and other
desktop tools responsive while the agent works. Show local times without zone
names or UTC offsets in the assistant controls, following the desktop status
strip convention. [Desktop guidance](../../AGENTS.md#desktop-conventions)

Add an interactive `mgb-ops assistant` command with explicit workspace and
agent selection and options for a config, a saved run, and an existing draft.
It creates an independent session and defaults to following the workspace's
active generation when no saved run is selected. Resolve paths from the
invocation directory using the existing CLI conventions. Use the terminal for
streamed text, permission answers, and draft review. A session without a draft
supports workspace and run queries. An explicit Save action uses the existing
draft save workflow and the opened draft path. Stop or interrupt cancels an
active prompt. Reset closes the conversation and keeps the working draft.
On quit, confirm departure if unsaved draft edits would be lost. On end of
input, close the conversation and report any unsaved edits being discarded.
Closing the conversation cleans up the child process. Use the existing
one-shot commands for scripted operations.

Keep the host transcript in memory. Explain at setup that the selected agent
and its provider may receive prompts and tool results, and that native file,
shell, and network permissions depend on the installed agent's configuration.
Closing or changing the workspace, changing agents, and exiting revoke the old
tool connection. Preserve the desktop's dirty-draft departure prompts.

## Draft review and apply

Add a discoverable `draft.propose_edit` command to the shared catalog. The
agent submits typed changes with the identity and revision from `draft.read`.
The initial edit surface is the observed and forecast day horizons for
version 2 drafts and forecast asset selection for an existing scenario by
its index in the reviewed revision. Omit fields to keep their current values.
Reject unsupported fields and support incomplete drafts. Validate an offered
asset ID against the draft's selected catalog and reject an asset change if it
makes existing correction selections incompatible. Use the ordinary draft
validation and anchor rules. Keep scenario structure, instruction values and
provenance, workspace and catalog paths, accepted source identities, and
visualization under host control.
Changing a selection still requires ordinary source acceptance before use.

Validate against a snapshot before showing the review. Show the current and
proposed values for each changed field and identify the affected scenario.
Distinguish invalid edits from readiness findings that can remain on an
incomplete draft. An unchanged proposal reports that there are no changes to
apply. The tool call returns when review is staged. The host reports Apply
and Reject outcomes in the interface. A pending review remains available
when its turn completes normally. Keep one pending proposal per conversation
and ask the operator to resolve it before staging another. Follow-up prompts
can continue while a review is pending.

The host provides Apply and Reject. Apply uses the pending candidate from the
current conversation and rechecks session identity, draft identity, revision,
and draft job state when committing it. A stale proposal produces a conflict
with no changes and asks for a new proposal from the current draft. A busy
draft can be retried after the active job finishes if the revision still
matches. Rejection, cancellation or failure of the originating turn, and
conversation closure clear any pending proposal. A successful Apply updates
the in-memory draft and normal dirty state. An applied edit stays in the
working draft if the turn is later cancelled. Saving is a separate operator
action.

## Acceptance

- In desktop and CLI, run the same sequence against fixtures. Inspect the
  workspace, summarize the run, query a retained series, inspect the working
  draft, and apply a reviewed edit. Compare the domain results and working
  draft state across interfaces.
- Exercise an edit spanning multiple supported fields, a forecast asset
  change, an incomplete draft, unsupported fields, invalid input, an unchanged
  proposal, rejection, and a stale proposal after a manual edit or draft
  replacement. Verify that legacy drafts retain explicit period bounds and
  that an incompatible asset change preserves corrections. Verify that Apply
  changes only the working draft and that Save writes only the draft document.
  Check that dependent actions still require explicit source acceptance
  following a selection change.
- Simulate workspace changes, agent changes, and quit with pending work.
  Exercise Stop during streaming, a permission request, and a turn with a
  staged proposal. Verify normal desktop departure behavior, responsiveness,
  and suppression of late events. Verify that a
  proposal survives normal turn completion and that attempts to stage another
  proposal leave the pending review intact. In the CLI, cover interrupt and
  end of input while awaiting a permission decision, and quit and end of input
  with unsaved applied edits.
- Perform the standard Rust and desktop checks. Run explicit Linux UI smoke
  checks and record the rendering environment. Record Windows evidence
  separately. [Validation guidance](../../AGENTS.md#changes-and-validation)
