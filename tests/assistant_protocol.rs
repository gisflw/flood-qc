#![cfg(unix)]

use mgb_ops::app::{
    AgentPreset, Assistant, AssistantEvent, AssistantOptions, AssistantPermissionKind,
    AssistantStage, AssistantStopReason, InspectionRequest, Session,
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    thread,
    time::{Duration, Instant},
};
use tempfile::TempDir;

fn start(mode: &str) -> (TempDir, Assistant) {
    fs::create_dir_all("runs").unwrap();
    let root = tempfile::tempdir_in("runs").unwrap();
    let path = root.path().canonicalize().unwrap();
    let workspace = path.join("fixture-workspace");
    fs::create_dir(&workspace).unwrap();
    let directory = path.join(mode);
    fs::create_dir(&directory).unwrap();
    let executable = path.join("fake-acp");
    fs::write(&executable, include_str!("fixtures/assistant_agent.sh")).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let session = Session::new(InspectionRequest {
        workspace,
        invocation_dir: path,
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    });
    let assistant = Assistant::start_with_options(
        session,
        AgentPreset::Codex,
        AssistantOptions {
            executable: Some(if mode == "missing-executable" {
                executable.with_extension("missing")
            } else {
                executable
            }),

            helper_executable: Some(if mode == "missing-helper" {
                directory.join("missing-helper")
            } else {
                env!("CARGO_BIN_EXE_mgb-ops").into()
            }),
            working_directory: Some(directory),
        },
    )
    .unwrap();
    (root, assistant)
}

fn event(assistant: &mut Assistant) -> AssistantEvent {
    let end = Instant::now() + Duration::from_secs(8);
    loop {
        assert!(Instant::now() < end, "assistant event timeout");
        if let Some(event) = assistant.try_event() {
            return event;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn ready(assistant: &mut Assistant) {
    let mut ready = false;
    let mut connected = false;
    while !ready || !connected {
        match event(assistant) {
            AssistantEvent::Ready => ready = true,
            AssistantEvent::ToolsConnected => connected = true,
            other => panic!("unexpected event during setup: {other:?}"),
        }
    }
}

fn finish(assistant: &mut Assistant, reason: AssistantStopReason) -> Vec<AssistantEvent> {
    let mut events = Vec::new();
    loop {
        match event(assistant) {
            AssistantEvent::Finished(finished) => {
                assert_eq!(finished, reason);
                return events;
            }
            AssistantEvent::Failed(error) => panic!("{error}"),
            other => events.push(other),
        }
    }
}

fn close(assistant: &mut Assistant) {
    assistant.close();
    assert!(assistant.wait_closed(Duration::from_secs(5)));
    assert!(assistant.try_event().is_none());
}

#[test]
fn stdio_mcp_streaming_and_follow_up_share_one_session() {
    let (_root, mut assistant) = start("agent");
    ready(&mut assistant);
    assistant.prompt("inspect".into()).unwrap();
    assert!(assistant.prompt("second in flight".into()).is_err());
    let events = finish(&mut assistant, AssistantStopReason::EndTurn);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantEvent::Thought(value) if value == "thinking"))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, AssistantEvent::Text(value) if value == "inspected"))
    );
    assert!(events.iter().any(|event| matches!(event, AssistantEvent::ToolCallCompleted { command: Some(id), ok: true, .. } if id == "workspace.inspect")));
    assistant.prompt("again".into()).unwrap();
    assert!(
        finish(&mut assistant, AssistantStopReason::EndTurn)
            .iter()
            .any(|event| matches!(event, AssistantEvent::Text(value) if value == "follow-up"))
    );
    close(&mut assistant);
}

#[test]
fn permission_ids_and_cancellation_are_enforced() {
    let (_root, mut assistant) = start("agent");
    ready(&mut assistant);
    for (choice, expected) in [("once", "allowed"), ("deny", "denied")] {
        assistant.prompt("permission".into()).unwrap();
        assert!(matches!(
            event(&mut assistant),
            AssistantEvent::ToolStatus(_)
        ));
        let AssistantEvent::Permission(permission) = event(&mut assistant) else {
            panic!("expected a permission request")
        };
        assert_eq!(
            permission.tool_call["rawInput"]["path"],
            "/synthetic data/file"
        );
        assert_eq!(permission.options.len(), 2);
        assert_eq!(
            permission.options[0].kind,
            AssistantPermissionKind::AllowOnce
        );
        assert_eq!(
            permission.options[1].kind,
            AssistantPermissionKind::RejectOnce
        );
        assert!(
            assistant
                .answer_permission(permission.request, "always")
                .is_err()
        );
        assistant
            .answer_permission(permission.request, choice)
            .unwrap();
        assert!(
            finish(&mut assistant, AssistantStopReason::EndTurn)
                .iter()
                .any(|event| matches!(event, AssistantEvent::Text(value) if value == expected))
        );
    }
    assistant.prompt("no-deny".into()).unwrap();
    let events = finish(&mut assistant, AssistantStopReason::Cancelled);
    assert!(events.iter().any(
        |event| matches!(event, AssistantEvent::Notice(text) if text.contains("no one-time deny"))
    ));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, AssistantEvent::Permission(_)))
    );
    assistant.prompt("stop-permission".into()).unwrap();
    assert!(matches!(
        event(&mut assistant),
        AssistantEvent::ToolStatus(_)
    ));
    let AssistantEvent::Permission(permission) = event(&mut assistant) else {
        panic!("expected a permission request")
    };
    assistant.stop();
    assert!(
        assistant
            .answer_permission(permission.request, "once")
            .is_err()
    );
    assert!(finish(&mut assistant, AssistantStopReason::Cancelled).is_empty());
    close(&mut assistant);
}

#[test]
fn malformed_output_and_unacknowledged_stop_close_and_reap_the_agent() {
    for (prompt, stage) in [
        ("malformed", AssistantStage::Protocol),
        ("hang", AssistantStage::Timeout),
    ] {
        let (_root, mut assistant) = start("agent");
        ready(&mut assistant);
        assistant.prompt(prompt.into()).unwrap();
        if prompt == "hang" {
            assert!(matches!(event(&mut assistant), AssistantEvent::Text(_)));
            assistant.stop();
            assert!(assistant.prompt("too early".into()).is_err());
        }
        let AssistantEvent::Failed(failure) = event(&mut assistant) else {
            panic!("expected a terminal error")
        };
        assert_eq!(failure.stage, stage);
        assert!(!assistant.is_running());
        assert!(assistant.wait_closed(Duration::from_secs(5)));
    }
}

#[test]
fn setup_failures_preserve_protocol_sign_in_and_subprocess_stages() {
    for (mode, stage) in [
        ("missing-executable", AssistantStage::Executable),
        ("missing-helper", AssistantStage::ToolConnection),
        ("wrong-version", AssistantStage::Protocol),
        ("signin", AssistantStage::SignIn),
        ("exit", AssistantStage::Subprocess),
    ] {
        let (_root, mut assistant) = start(mode);
        let AssistantEvent::Failed(failure) = event(&mut assistant) else {
            panic!("expected startup failure")
        };
        assert_eq!(failure.stage, stage);
        assert!(assistant.wait_closed(Duration::from_secs(5)));
    }
}

#[test]
fn stop_withdraws_a_read_blocked_by_active_run_publication() {
    let (root, mut assistant) = start("agent");
    let cache = root.path().join("fixture-workspace/data/cache/run");
    fs::create_dir_all(&cache).unwrap();
    let lock = fs::File::create(cache.join("access.lock")).unwrap();
    lock.lock().unwrap();
    ready(&mut assistant);
    assistant.prompt("interrupt-read".into()).unwrap();
    assert!(matches!(event(&mut assistant), AssistantEvent::Text(value) if value == "waiting"));
    // Allow the read to reach the cache guard before cancellation.
    thread::sleep(Duration::from_millis(100));
    assistant.stop();
    assert!(finish(&mut assistant, AssistantStopReason::Cancelled).is_empty());
    lock.unlock().unwrap();
    assistant.prompt("inspect".into()).unwrap();
    assert!(finish(&mut assistant, AssistantStopReason::EndTurn).iter().any(|event|
        matches!(event, AssistantEvent::ToolCallCompleted { command: Some(id), ok: true, .. } if id == "workspace.inspect")
    ));
    close(&mut assistant);
}
