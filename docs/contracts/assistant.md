# MGB Ops assistant

The assistant can inspect the selected workspace, query retained model results,
and propose changes to the current run parameters. It uses a locally installed ACP
adapter and the agent's own sign-in. Put one of these executables on `PATH`.

| Agent | Executable |
| --- | --- |
| Codex | `codex-acp`. [Adapter setup](https://github.com/agentclientprotocol/codex-acp#installation) |
| Gemini CLI | `gemini --acp`. [ACP mode](https://github.com/google-gemini/gemini-cli/blob/main/docs/cli/acp-mode.md) |
| Claude | `claude-agent-acp`. [Adapter setup](https://github.com/agentclientprotocol/claude-agent-acp#standalone-executables) |

The selected agent and its provider may receive prompts and tool results. The
installed agent's settings govern its native file, shell, and network permissions.
The desktop saves conversations and drafts locally, separately for each workspace,
in the application's private data directory. The CLI keeps its transcript in memory.
MGB Ops revokes the tool connection when a conversation closes. Each saved desktop
conversation has a separate working directory; connection credentials are temporary.

## Desktop and CLI

In the desktop, open **Pages → Assistant** and select an agent. The
**Model** and **Reasoning effort** controls show the values offered by the
installed adapter. Set them before sending a prompt or between turns. A model
change can change the available effort values. The desktop remembers confirmed
choices per agent and restores them when the adapter still offers them.

Type in the composer at the bottom. It grows with the message up to a height limit
and scrolls within the editor after that. Press **Enter** to send, or
**Shift+Enter** to insert a newline. Replies appear under **Assistant** and render
Markdown as they stream, including headings, lists, code, links, and tables.
Wide tables wrap within the transcript and show their rows as labeled fields
when columns would be too
narrow. A loading icon appears while waiting for the first reply text.
Expand **Thinking** to read available adapter summaries and tool progress for
that turn. Summaries render Markdown; tool rows distinguish progress, completion,
and failure. Use message copy controls to copy the prompt or reply Markdown.
The assistant uses the open workspace, the run source selected
in **Files**, and the current run parameters. **Stop** cancels the active prompt.
Closing the tab keeps the conversation open. **New chat** starts another
conversation and keeps the current run parameters. **History** lists saved chats
for the workspace and offers selection, rename, and deletion. The last viewed
chat and unsent draft return after restart. **Load older messages** reveals earlier
turns without deleting stored history.

The desktop asks the adapter to resume a saved session when supported. If the
session cannot be resumed, **Continue with transcript** starts a new session with
available prior user/assistant text. The interface reports omitted older context;
tool state and approvals are not replayed. Interrupted turns remain marked.
Deleting a chat removes its MGB Ops history and working directory; installed agents
may retain their own session records.

Start the interactive CLI with an explicit workspace and agent.

```sh
mgb-ops assistant --workspace /regional/workspace --agent codex
mgb-ops assistant --workspace /regional/workspace --agent gemini \
  --config config.toml --saved-run saved-run
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
prompt, `:reset` to start a new conversation, and `:quit` to leave.

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
| `run.parameters` | The current parameters, revision, and available forecast cycles. |
| `run.propose_parameters` | A proposed reference, horizon, or ordered scenario change for operator review. |

Series windows use the retained scenario's timezone. The [series
contract](series.md) defines timestamp, unit and missing-value behavior.

## Reviewing run parameters

The assistant can propose the reference hour, horizons and the ordered scenario
list, including corrections on pinned cycles. Review the changes and choose
**Apply** or **Reject** in the desktop, or use `:apply` or `:reject` in the CLI.
Apply updates the current session. A proposal becomes stale when the form
changes or the workspace closes. Submit the run from **New run** or the
[operation CLI](operations.md#run-from-a-workspace).
