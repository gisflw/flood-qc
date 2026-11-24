# Assistant command and protocol specification

This is the second increment of the [assistant plan](../plans/assistant.md).
It uses the [shared session](assistant-session.md) to connect an installed
agent to the same data and workflow functions that MGB Ops uses for operators.
The [interface increment](assistant-interfaces.md) adds operator interaction
and draft proposals.

## Command discovery and invocation

Expose two MCP tools modeled on RustGIS. Use `search_commands` to find commands
by name or purpose and return their stable IDs, descriptions, and argument
schemas. Use `run_command` with a command ID and typed JSON arguments. One
catalog supplies metadata and dispatch. Add entries alongside the workflows
they expose and use their existing types and validation. Unknown IDs and
invalid arguments return structured findings. Existing CLI and desktop entry
points can continue to call the underlying workflows directly.
[RustGIS command tools](https://github.com/opengeos/RustGIS/blob/463133bd9d1b6108f2cc5fe8bbd211d401384734/crates/assistant/src/tools.rs#L53-L75)

Start with the following read commands. Use session selections to resolve
storage paths. Return identifiers and domain findings where the agent needs
them to make a follow-up call.

| Command | Inputs and result |
| --- | --- |
| `workspace.inspect` | Inspect the selected workspace and return its existing prerequisite findings. |
| `run.summary` | Return the selected run's identity, reference, scenario IDs, and available results. Retain healthy scenarios alongside per-scenario findings. |
| `location.search` | Find station names and IDs or mini-basin IDs in the selected workspace. Return matches in stable identifier order up to the result limit and indicate if more matches exist. |
| `series.query` | Query the selected run by station or mini-basin ID, scenario, variable, and explicit window. Accept an optional expected run identity from a prior result. |
| `draft.read` | Return a bounded summary of the open draft and its opaque identity and revision, or report that no draft is open. Include its schema version, reference and timezone, period settings, and scenario indices, IDs and selected asset IDs. |

Use the selected scenario's retained result timezone to interpret explicit
local window bounds. Use the existing series API to preserve local times,
units, missing values, valid zeros, and raw versus referenced level semantics.
A result identifies the resolved run, scenario, location, timezone, and window.
Document the response limit and check the maximum possible point count for
the window before reading series values where possible. For an oversized
window, return a finding that asks for a narrower window. Resolve the run
through the session on each call. Compare any expected run identity before
reading series values, including after a change between active and saved
selection. On a mismatch, return a stale-run finding.
[Series contract](../../docs/contracts/series.md)

Command admission follows the scope of the shared session. Read commands
execute directly. The later draft proposal command returns a proposed change
to the host for review. Existing app workflows own anchor, input acceptance,
validation, and execution rules. The agent interface adds no command-specific
permission settings. Existing operator workflows handle saves, source
acceptance, registration, preparation, and run launch.

## Agent startup and tool connection

Run one installed Codex ACP adapter, Gemini CLI in ACP mode, or Claude Agent
ACP adapter as a child process for each conversation. Use the agent's own
authentication. Negotiate ACP v1 over standard I/O. Keep preset discovery,
process supervision, and protocol translation behind a shared library API
usable without the desktop feature. App owns the command catalog and tool
dispatch. The interfaces own rendering and user input. Exercise this layer
through a headless test driver before the operator interfaces are available.

Start transport qualification with an MCP stdio helper declared in
`session/new`. A helper connects back to the originating app session through
a local endpoint with an ephemeral credential. Verify all three agents against
the same transport before finalizing the bridge. Bind any listener to loopback,
pass credentials outside command-line arguments, and restrict connection and
request lifetimes to the conversation. Close the endpoint on reset, workspace
change, agent change, or shutdown. Invalidate the old connection and suppress
in-flight results on closure. Tool calls use the MGB Ops session's explicit
workspace selection. Default the agent working directory to a separate local
directory. Agent-native file, shell, and network access remains governed by
the installed agent's configuration.
[ACP session setup](https://agentclientprotocol.com/protocol/v1/session-setup)

Surface executable discovery, protocol mismatch, sign-in, tool connection,
and subprocess failures distinctly. Treat a completed ACP handshake and a
working tool connection as separate milestones. Include the agent and failed
stage in diagnostics and keep credentials out of diagnostics.

## Conversation lifecycle

Stream reply chunks, tool status, and permission requests through typed
events. Allow one prompt in flight per conversation. Send follow-ups through
the same ACP session. When the agent asks for permission, show the operation
details it supplies, including full commands and paths where available.
Present the available one-time allow and deny choices and reply with the
selected option ID. If a request offers no one-time deny choice, explain the
reason and cancel the turn so the operator can always decline. ACP permission
decisions do not apply a draft proposal.

Stop sends ACP cancellation, resolves outstanding permission prompts as
cancelled, and withdraws pending host requests for the turn. Drop results from
cancelled host requests. Wait for the prompt to return a cancelled result within
a bounded grace period before accepting another prompt. On timeout or process
failure, close the conversation and reap owned processes. A closed session
cannot deliver late tool results, approval requests, or reply chunks to a
replacement session. Bound queued messages and diagnostics to keep slow or
malformed processes from blocking the host.
[ACP prompt turns](https://agentclientprotocol.com/protocol/v1/prompt-turn) ·
[ACP permission requests](https://agentclientprotocol.com/protocol/v1/tool-calls)

## Acceptance

- Fake-agent tests exercise all lifecycle states, follow-up, streamed content,
  permission answers, a permission request without a one-time deny option,
  malformed output, cancellation during permission or a tool request, and
  termination when an agent ignores Stop.
- Command tests exercise discovery and invocation from the same catalog,
  unknown IDs, invalid arguments, empty session state, ambiguous location
  matches and truncation, per-scenario findings, and stale expected run
  identities after publication and selection changes. Cover series timezone,
  units, missing values, valid zero, and windows at and above the limit.
  Verify that revoked credentials and late requests from an old conversation
  cannot access a replacement session.
- On Linux, qualify the same tool transport with Codex, Gemini CLI, and Claude.
  Record each version and demonstrate handshake, streamed reply, a confirmed
  host-side tool call, follow-up, cancellation, and cleanup against disposable
  data under `runs/` using explicitly configured accounts. Verify that tool
  results identify the correct session. Record unavailable agents and other
  blockers as qualification gaps. Run the standard Rust checks and record
  Windows evidence separately.
  [Validation guidance](../../AGENTS.md#changes-and-validation)
