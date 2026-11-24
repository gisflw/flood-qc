# MGB Ops assistant draft plan

Let operators use Codex, Gemini CLI, or Claude from the desktop and CLI to
inspect a selected MGB Ops workspace, answer questions about saved runs, and
propose edits to an operation draft. The operator selects the agent directly.
The first pass covers inspection, series queries, and draft editing through
operator review. Starting operations from the assistant and attaching a CLI
process to a live desktop session are later decisions.

## Shared application session

Add an explicit, process-local session in `app` around the workflows that need
shared context. Start with the resolved workspace and configuration selection,
the run source, and the draft being edited. The desktop and its assistant use
the same session. A CLI assistant invocation starts an independent session from
explicit selections and can keep it for follow-up turns. Existing one-shot CLI
commands keep their arguments, path resolution, and output contracts. Move
shared behavior into the session only where an assistant or another interface
needs it. [Workspace contract](../../docs/contracts/workspace.md)

Keep map widgets, layout, display preferences, and transient form inputs in the
desktop. App owns the draft revision and applies validated edits. Read requests
use a consistent session snapshot with an explicit active generation or a
selected saved run. Where the desktop follows the active run, resolve the
generation at tool call time and identify it in the result. A selected saved run
stays selected. Use the existing active-run readers, operation controls, and
workspace locks. [Architecture](../../docs/architecture.md) ·
[Operations contract](../../docs/contracts/operations.md#active-run-and-rainfall-cache)

## ACP and MCP

Use Agent Client Protocol (ACP) over standard I/O for the local agent process.
Use Model Context Protocol (MCP) to give that process a small set of MGB Ops
tools bound to the session that launched it. RustGIS uses this split to let a
local agent call tools against the open project. Adapt its ACP client and
run-scoped MCP connection as design references. Qualify one local MCP transport
across all three agents before fixing the launcher and bridge design.
[RustGIS ACP client](https://github.com/opengeos/RustGIS/blob/463133bd9d1b6108f2cc5fe8bbd211d401384734/crates/assistant/src/acp.rs) ·
[RustGIS MCP integration](https://github.com/opengeos/RustGIS/blob/463133bd9d1b6108f2cc5fe8bbd211d401384734/docs/mcp.md)

| Agent | First-pass launch path |
| --- | --- |
| Codex | Installed Codex ACP adapter. [Codex ACP](https://github.com/agentclientprotocol/codex-acp) |
| Gemini CLI | Installed Gemini CLI in ACP mode. [Gemini CLI ACP mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md) |
| Claude | Installed Claude Agent ACP adapter. [Claude Agent ACP](https://github.com/agentclientprotocol/claude-agent-acp) |

Use the agents' own sign-in. Record the versions that pass the common smoke
tests and report missing executables, authentication failures, and unsupported
capabilities separately. Support streamed replies, follow-up turns in the same
conversation, permission prompts, and cancellation in both interfaces. Keep
conversation history in memory for the first pass. Workspace changes, agent
changes, and exit close the conversation, revoke the tool connection, and clean
up the owned agent process. Keep protocol and process handling separate from
workflow decisions and UI rendering.

Scope each MCP connection to its originating session. Offer typed tools using
known selections and identifiers. Give the agent only the context needed for
the request, and explain during setup that prompts and tool results may reach
the selected provider. The tool connection controls access to MGB Ops
workflows. The installed agent's own file, shell, and network permissions
depend on its configuration and capabilities.

## First-pass tools

1. Inspect the selected workspace and summarize findings, with the limits of
   prerequisite inspection preserved.
2. Summarize the selected saved run or current active generation and list its
   scenarios. Query bounded station or mini-basin series through the existing
   series API. Include the run identity, scenario, units, time window, and
   findings. Preserve missing values and valid zero.
   [Series contract](../../docs/contracts/series.md)
3. Propose a typed edit to the open draft. Start with a small edit surface such
   as draft horizons or forecast asset selection. Show the current and proposed
   values and any validation findings. Apply only after the operator accepts
   through the host interface. Recheck the workspace, draft identity, and
   revision at apply time so a delayed proposal cannot overwrite a newer edit.
   Applying updates the in-memory draft. The operator can save it explicitly
   through the existing draft workflow. Preserve anchor and source acceptance
   rules. [Draft contract](../../docs/contracts/drafts.md)

## Delivery increments

1. Extract the session context needed for workspace inspection, run selection,
   and draft editing. Test equivalent inputs through CLI and desktop workflows,
   including saved-run selection, workspace changes, and stale draft edits.
2. Implement the ACP client and read-only MCP tools. Use a deterministic fake
   agent in normal tests, then verify handshake, streaming, a real tool call,
   follow-up, and cancellation with each installed agent on Linux. Exercise
   timeout, adapter exit, and cleanup without provider access in the fake-agent
   tests.
3. Add the desktop assistant and interactive CLI entry point, then wire draft
   proposal review and explicit apply into both. Demonstrate a session that
   identifies a run, queries a series, and applies a reviewed draft edit. Check
   rejection and stale proposals as well as successful apply. Verify that the
   desktop stays responsive during a turn and that project switching cannot
   deliver a late result into the newly opened workspace.

Write an implementation specification for each increment in
`../changes/`. Use the repository's standard Rust and desktop validation for
implementation changes. Qualify Linux first and record Windows separately.
Provider smoke tests use disposable data under `runs/` on an explicitly
configured account. The assistant checks exercise application workflows and
agent integration. Numerical engine qualification remains in the existing
regression case. [Validation guidance](../../AGENTS.md#changes-and-validation) ·
[RS regression case](../../references/regression/rs-hourly/README.md)
