use mgb_ops::app::{
    AgentPreset, Assistant, AssistantEvent, AssistantOptions, AssistantStopReason,
    InspectionRequest, Session,
};
use std::{
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

#[test]
#[ignore = "requires an explicitly configured provider account and a disposable workspace"]
fn provider_smoke() {
    let directory = std::env::current_dir().unwrap();
    let workspace = PathBuf::from(
        std::env::var_os("MGB_OPS_SMOKE_WORKSPACE")
            .expect("set MGB_OPS_SMOKE_WORKSPACE to a disposable workspace"),
    );
    assert!(
        directory
            .join(&workspace)
            .canonicalize()
            .unwrap()
            .starts_with(directory.join("runs").canonicalize().unwrap())
    );
    let working = PathBuf::from(
        std::env::var_os("MGB_OPS_SMOKE_CWD")
            .expect("set MGB_OPS_SMOKE_CWD to a separate directory under runs"),
    );
    assert!(
        directory
            .join(&working)
            .canonicalize()
            .unwrap()
            .starts_with(directory.join("runs").canonicalize().unwrap())
    );
    assert_ne!(
        workspace.canonicalize().unwrap(),
        working.canonicalize().unwrap()
    );
    let agent = match std::env::var("MGB_OPS_SMOKE_AGENT").unwrap().as_str() {
        "codex" => AgentPreset::Codex,
        "gemini" => AgentPreset::Gemini,
        "claude" => AgentPreset::Claude,
        _ => panic!("set MGB_OPS_SMOKE_AGENT to codex, gemini or claude"),
    };
    let session = Session::new(InspectionRequest {
        workspace,
        invocation_dir: directory,
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    let mut assistant = Assistant::start_with_options(
        session,
        agent,
        AssistantOptions {
            executable: std::env::var_os("MGB_OPS_SMOKE_EXECUTABLE").map(PathBuf::from),
            helper_executable: Some(env!("CARGO_BIN_EXE_mgb-ops").into()),
            working_directory: Some(working),
        },
    )
    .unwrap();

    wait_for(&mut assistant, Duration::from_secs(80), |event| {
        matches!(event, AssistantEvent::Ready)
    });
    println!("ACP session ready");
    assistant.prompt(
        "Use the MGB Ops MCP tools, not shell or file-system tools. Use search_commands to find workspace.inspect and run_command to execute workspace.inspect. In one sentence, give the workspace name from its report and whether configuration was found.".into()
    ).unwrap();
    let mut confirmed = false;
    let mut streamed = false;
    wait_for(&mut assistant, Duration::from_secs(180), |event| {
        match event {
            AssistantEvent::ToolCallCompleted {
                command: Some(command),
                ok: true,
                ..
            } if command == "workspace.inspect" => confirmed = true,
            AssistantEvent::Text(text) if !text.is_empty() => streamed = true,
            _ => {}
        }
        matches!(
            event,
            AssistantEvent::Finished(AssistantStopReason::EndTurn)
        )
    });
    assert!(
        confirmed && streamed,
        "expected a confirmed host-side read and streamed reply"
    );
    assistant
        .prompt("Follow up by naming the workspace in the preceding answer.".into())
        .unwrap();
    let mut follow_up = false;
    wait_for(&mut assistant, Duration::from_secs(120), |event| {
        if matches!(event, AssistantEvent::Text(text) if !text.is_empty()) {
            follow_up = true;
        }
        matches!(
            event,
            AssistantEvent::Finished(AssistantStopReason::EndTurn)
        )
    });
    assert!(follow_up);
    assistant
        .prompt("Print the integers from 1 to 2000 in order, one number per line.".into())
        .unwrap();
    wait_for(&mut assistant, Duration::from_secs(90), |event| {
        matches!(event, AssistantEvent::Text(_))
    });
    assistant.stop();
    wait_for(&mut assistant, Duration::from_secs(8), |event| {
        matches!(
            event,
            AssistantEvent::Finished(AssistantStopReason::Cancelled)
        )
    });
    assistant.close();
    assert!(assistant.wait_closed(Duration::from_secs(5)));
    println!("ACP cancellation acknowledged and process reaped");
}

fn wait_for(
    assistant: &mut Assistant,
    timeout: Duration,
    mut condition: impl FnMut(&AssistantEvent) -> bool,
) {
    let end = Instant::now() + timeout;
    loop {
        assert!(
            Instant::now() < end,
            "Timed out waiting for the selected assistant event"
        );
        match assistant.try_event() {
            Some(AssistantEvent::Failed(error)) => panic!("{error}"),
            Some(AssistantEvent::Permission(permission)) => panic!(
                "Smoke prompt requested native tool permissions for {}",
                permission.tool_call
            ),
            Some(event) => {
                let matched = condition(&event);
                if !matches!(
                    event,
                    AssistantEvent::ToolStatus(_) | AssistantEvent::Thought(_)
                ) {
                    println!("{event:?}");
                }
                if matched {
                    return;
                }
                assert!(
                    !matches!(event, AssistantEvent::Finished(_)),
                    "Unexpected turn completion"
                );
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
    }
}
