#!/bin/bash
set -euo pipefail

read_line() { IFS= read -r line; }

update() {
    printf '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fake","update":{"sessionUpdate":"%s","content":{"type":"text","text":"%s"}}}}\n' "$1" "$2"
}

finish() {
    printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"%s"}}\n' "$prompt" "$1"
}

permission() {
    printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fake","update":{"sessionUpdate":"tool_call","toolCallId":"local","title":"Local operation","status":"pending","rawInput":{"command":"inspect full path","path":"/synthetic data/file"}}}}'
    printf '{"jsonrpc":"2.0","id":"p","method":"session/request_permission","params":{"sessionId":"fake","toolCall":{"toolCallId":"local","title":"Local operation"},"options":[%s]}}\n' "$1"
}

cancelled() {
    local seen_cancel=0 seen_permission=0
    while (( !seen_cancel || !seen_permission )); do
        read_line
        case $line in
            *'"method":"session/cancel"'*) seen_cancel=1 ;;
            *'"id":"p"'*'"outcome":"cancelled"'*) seen_permission=1 ;;
            *) exit 11 ;;
        esac
    done
    update agent_message_chunk late-cancelled-output
    finish cancelled
}

mcp_send() { printf '%s\n' "$1" >&"${TOOL[1]}"; }
mcp_read() { IFS= read -r -u "${TOOL[0]}" reply; }

read_line
[[ $line == *'"method":"initialize"'*'"protocolVersion":1'* ]]
case "$PWD" in
    */wrong-version)
        printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":2}}'
        exit ;;
    */signin)
        printf '%s\n' '{"jsonrpc":"2.0","id":1,"error":{"code":-32000,"message":"Authentication required"}}'
        exit ;;
    */exit)
        exit 4 ;;
esac
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"mcpCapabilities":{}}}}'
read_line
[[ $line == *'"method":"session/new"'* ]]
port_regex='"--assistant-mcp-bridge","([0-9]+)"'
token_regex='"value":"([a-f0-9]{64})"'
helper_regex='"command":"([^"]+)"'
[[ $line =~ $port_regex ]]
port=${BASH_REMATCH[1]}
[[ $line =~ $token_regex ]]
token=${BASH_REMATCH[1]}
[[ $line =~ $helper_regex ]]
helper=${BASH_REMATCH[1]}
# The bridge credential is deliberately passed only through the child environment.
coproc TOOL { MGB_OPS_ASSISTANT_TOKEN=$token "$helper" --assistant-mcp-bridge "$port"; }
mcp_send '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26","clientInfo":{"name":"fake","version":"1"},"capabilities":{}}}'
mcp_read
[[ $reply == *'"protocolVersion":"2025-03-26"'* ]]
mcp_send '{"jsonrpc":"2.0","method":"notifications/initialized"}'
mcp_send '{"jsonrpc":"2.0","id":2,"method":"tools/list"}'
mcp_read
[[ $reply == *'"name":"search_commands"'* && $reply == *'"name":"run_command"'* ]]
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"fake"}}'

prompt_re='"id":([0-9]+)'
allow_once='{"kind":"allow_once","name":"Allow once","optionId":"once"}'
reject_once='{"kind":"reject_once","name":"Deny once","optionId":"deny"}'
allow_always='{"kind":"allow_always","name":"Always","optionId":"always"}'

while read_line; do
    [[ $line == *'"method":"session/prompt"'*'"sessionId":"fake"'* && $line =~ $prompt_re ]]
    prompt=${BASH_REMATCH[1]}
    case $line in
        *'"text":"inspect"'*)
            mcp_send '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"search_commands","arguments":{"query":"workspace"}}}'
            mcp_read
            [[ $reply == *workspace.inspect* && $reply == *arguments_schema* ]]
            mcp_send '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"run_command","arguments":{"id":"workspace.inspect","arguments":{}}}}'
            mcp_read
            [[ $reply == *'"isError":false'* && $reply == *fixture-workspace* ]]
            update agent_thought_chunk thinking
            update agent_message_chunk inspected
            finish end_turn
            ;;
        *'"text":"permission"'*)
            permission "$allow_once,$reject_once,$allow_always"
            read_line
            [[ $line == *'"id":"p"'*'"outcome":"selected"'* ]]
            if [[ $line == *'"optionId":"once"'* ]]; then
                update agent_message_chunk allowed
            elif [[ $line == *'"optionId":"deny"'* ]]; then
                update agent_message_chunk denied
            else
                exit 12
            fi
            finish end_turn
            ;;
        *'"text":"no-deny"'*)
            permission "$allow_always"
            cancelled
            ;;
        *'"text":"stop-permission"'*)
            permission "$allow_once,$reject_once"
            cancelled
            mcp_send '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"run_command","arguments":{"id":"workspace.inspect","arguments":{}}}}'
            mcp_read
            [[ $reply == *'No active assistant turn'* ]]
            ;;
        *'"text":"malformed"'*)
            printf 'invalid ACP\n'
            ;;
        *'"text":"hang"'*)
            update agent_message_chunk hanging
            read_line
            [[ $line == *'"method":"session/cancel"'* ]]
            while read_line; do :; done
            ;;
        *'"text":"interrupt-read"'*)
            mcp_send '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"run_command","arguments":{"id":"series.query","arguments":{"location":{"mini":1},"scenario_id":"blocked","variable":"rain","start":"2026-10-03T00:00:00","end":"2026-10-03T01:00:00"}}}}'
            update agent_message_chunk waiting
            read_line
            [[ $line == *'"method":"session/cancel"'* ]]
            finish cancelled
            # A detached read must never forward its eventual payload after Stop.
            if IFS= read -r -t 1 -u "${TOOL[0]}" reply; then
                exit 13
            fi
            ;;
        *)
            update agent_message_chunk follow-up
            finish end_turn
            ;;
    esac
done
