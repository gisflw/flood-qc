# Assistant

The desktop and interactive CLI connect an installed Codex ACP adapter, Gemini
CLI, or Claude Agent ACP adapter to an explicitly selected MGB Ops workspace.
Install the agent separately and sign in using its own workflow.

| Agent | Executable on `PATH` |
| --- | --- |
| Codex | `codex-acp` |
| Gemini CLI | `gemini` |
| Claude | `claude-agent-acp` |

Your selected agent and its provider may receive prompts and tool results.
The installed agent's configuration governs its native file, shell, and
network access. MGB Ops keeps the host transcript in memory.

## Start a conversation

In the desktop, open a workspace, choose **View → Assistant**, and select an agent.
The pane follows the project, the run selected in **Files**, and the working
draft. A session without a draft can still inspect the workspace and query
runs.

```sh
mgb-ops assistant --workspace /regional/workspace --agent codex
```

The other CLI agent values are `gemini` and `claude`. Use `--config` to
select a configuration file, `--run` to select a saved run directory or bundle,
and `--draft` to open an existing draft. Relative arguments use the invocation
directory. With no saved run selected, the session follows the active
generation. Each launch creates an independent session.

Enter a prompt to stream replies. Follow-ups use the same conversation.
When the agent asks for permission, inspect the operation details and choose
a one-time answer. **Stop** cancels the active turn. **Reset** closes the
conversation and keeps the working draft. Closing or switching the workspace,
switching agents, or exiting closes the conversation and revokes its tool
connection. The terminal provides `/stop` and `/reset`; interrupt also stops
the active turn.

## Workspace and run queries

The agent can inspect workspace prerequisites, summarize the selected run,
look up station or mini-basin IDs, query a series, and read the working draft
summary. Run summaries retain findings for scenarios whose results could not
be read. Run and draft summaries return up to 64 scenarios. Location search
returns up to 50 matches and indicates whether more exist.

Series queries use the selected scenario's retained timezone and whole-hour
local bounds. Each query allows up to 168 points. The corresponding window
is at most 168 hours for rainfall or 167 hours for flow and level, whose
endpoints are included. Ask for a narrower window when it exceeds the limit.
An earlier run identity can be passed back to a query to detect when the
selected run has changed. MCP command results above 96 KiB return a
response-limit finding.

## Review draft edits

The agent can propose observed and forecast day horizons for version 2 drafts
or a forecast asset for an existing scenario. Review each current and proposed
value before choosing **Apply** or **Reject**. A pending proposal remains
available after a normally completed turn. A newer draft revision requires a
new proposal. If a draft job is active, retry Apply after it finishes.

Apply changes the working copy. Save explicitly to write the draft. The CLI
provides `/apply`, `/reject`, and `/save` for the opened draft. Use the
ordinary [draft workflows](drafts.md#editing-source-acceptance-and-previews)
to accept selected inputs before preparing or running. On `/quit`, the CLI
asks before discarding unsaved edits. End of input closes the conversation
and reports any discarded edits.
