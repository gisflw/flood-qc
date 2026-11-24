# MGB Ops assistant

The assistant can inspect the selected workspace, query retained model results,
and propose edits to an open operation draft. It uses a locally installed ACP
adapter and the agent's own sign-in. Put one of these executables on `PATH`.

| Agent | Executable |
| --- | --- |
| Codex | `codex-acp`. [Adapter setup](https://github.com/agentclientprotocol/codex-acp#installation) |
| Gemini CLI | `gemini --acp`. [ACP mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md) |
| Claude | `claude-agent-acp`. [Adapter setup](https://github.com/agentclientprotocol/claude-agent-acp#standalone-executables) |

The selected agent and its provider may receive prompts and tool results. The
installed agent's settings govern its native file, shell, and network permissions.
MGB Ops keeps its transcript in memory and revokes its tool connection when the
conversation closes. The agent starts in a separate local working directory.

## Desktop and CLI

In the desktop, open **View → Assistant**, select an agent, and send a prompt.
The assistant uses the open workspace, the run source selected in **Files**,
and the working draft. **Stop** cancels the active prompt. **Reset conversation**
closes the agent connection and keeps the working draft.

Start the interactive CLI with an explicit workspace and agent.

```sh
mgb-ops assistant --workspace /regional/workspace --agent codex
mgb-ops assistant --workspace /regional/workspace --agent gemini \
  --config config.toml --saved-run saved-run --draft draft.json
```

Use `codex`, `gemini`, or `claude` for `--agent`. Explicit paths follow
the [workspace path rules](workspace.md#path-resolution). Both interfaces follow
the active run by default. Selecting an independently retained run pins the
selection to that directory or bundle. A workspace without an active run can
still be inspected.

Enter prompts as lines of text. A permission request shows the operation and the
agent's offered one-time choices. Select a numbered choice in the CLI or its
button on the desktop. If the agent supplies no one-time deny option, MGB Ops
cancels the prompt. Use `:stop` or Ctrl+C in the CLI to cancel an active
prompt, `:reset` to start a new conversation, and `:quit` to leave. The CLI
asks before discarding unsaved edits on quit and reports discarded edits when
input ends.

## Commands available to the agent

The session-bound MCP tools `search_commands` and `run_command` expose the
command IDs and their argument schemas. Read commands use the current session
selection when invoked.

| Command | Result |
| --- | --- |
| `workspace.inspect` | The ordinary [prerequisite findings](workspace.md#findings-and-output). |
| `run.summary` | An opaque run identity, reference time, scenario IDs and availability findings. |
| `location.search` | Matches by station name, station ID, or MINI ID. Station IDs sort first, followed by numeric MINI IDs. The default limit is 20 and the maximum is 100. |
| `series.query` | One scenario, location and variable in an explicit local window, with a limit of 1,000 points. An expected run identity from a prior result detects a changed selection or active generation. |
| `draft.read` | The open draft's version, reference, period and up to 100 scenario entries. |
| `draft.propose_edit` | A proposed change for operator review. |

Series windows use the retained scenario's timezone. The [series
contract](series.md) defines timestamp, unit and missing-value behavior.

## Reviewing draft changes

The agent can propose day horizons on version 2 drafts and select forecast assets
for existing scenarios. The review shows the current and proposed values, the
affected scenarios, and readiness findings. A review remains available after a
successful prompt, and can coexist with follow-up prompts. Resolve it before
staging another review.

Select **Apply** or **Reject** in the desktop, or enter `:apply` or `:reject`
in the terminal. Apply updates the working draft. If the draft changed since
review, request a fresh proposal. If a draft job is active, retry after it
finishes. Save explicitly from the **Draft** ribbon or with `:save` in the CLI.
The CLI saves to the opened draft path. Changed input selections still require
[explicit source acceptance](drafts.md#editing-source-acceptance-and-previews)
before dependent work can use them.
