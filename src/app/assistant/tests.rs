use super::{
    bridge::{Bridge, write_message},
    commands::{self, Output},
    *,
};
use crate::app::{InspectionRequest, OperationalRun, RunSelection};
use std::{
    fs,
    io::BufReader,
    net::TcpStream,
    path::Path,
    time::{Duration, Instant},
};

fn temp() -> tempfile::TempDir {
    fs::create_dir_all("runs").unwrap();
    tempfile::tempdir_in("runs").unwrap()
}

fn session(workspace: &Path) -> Session {
    let root = workspace.canonicalize().unwrap();
    Session::new(InspectionRequest {
        workspace: root.clone(),
        invocation_dir: root,
        config: None,
        gpkg_path: None,
        engine_executable: None,
        engine_input_dir: None,
    })
}

fn open_setup(session: &Session) {
    fs::write(
        session.config(),
        "[run]\ntime_zone='UTC'\n[spatial]\nbounds=[-1.0,-1.0,1.0,1.0]\n",
    )
    .unwrap();
    session.open_run_setup(
        OperationalRun::open_workspace(session.workspace().into(), chrono::Utc::now()).unwrap(),
    );
}

fn result(output: commands::Result) -> Value {
    match output.unwrap() {
        Output::Data(data) => data,
        Output::Proposal(_) => panic!("Expected data"),
    }
}

#[test]
fn catalog_discovery_and_incomplete_edit_guards() {
    let dir = temp();
    let session = session(dir.path());
    let results = commands::search("run parameters");
    assert!(
        results["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["id"] == "run.propose_parameters")
    );
    let empty = serde_json::json!({});
    let no_setup = serde_json::json!({"parameters":null});
    assert_eq!(
        result(commands::run(&session, "run.parameters", empty.clone())),
        no_setup
    );
    assert!(commands::run(&session, "run.summary", empty.clone()).is_err());
    open_setup(&session);
    let read = result(commands::run(&session, "run.parameters", empty));
    let revision = &read["revision"];
    for invalid in [
        serde_json::json!({"revision":revision, "forecast_days":0}),
        serde_json::json!({"revision":revision, "workspace":"elsewhere"}),
        serde_json::json!({"revision":revision, "scenarios":[]}),
    ] {
        assert!(commands::run(&session, "run.propose_parameters", invalid).is_err());
    }
    let args = serde_json::json!({"revision":revision,"lookback_days":0,"forecast_days":3});
    let Output::Proposal(proposal) =
        commands::run(&session, "run.propose_parameters", args).unwrap()
    else {
        panic!("Expected proposal");
    };
    assert_eq!(proposal.review.changes.len(), 2);
    let job = session.start_workspace_job().unwrap();
    assert!(
        session
            .update_run_setup(&proposal.revision, proposal.parameters.clone())
            .is_err()
    );
    drop(job);
    session
        .update_run_setup(&proposal.revision, proposal.parameters.clone())
        .unwrap();
    assert_eq!(
        session
            .run_setup()
            .unwrap()
            .operation
            .parameters()
            .forecast_days,
        Some(3)
    );
    assert!(
        session
            .update_run_setup(&proposal.revision, proposal.parameters)
            .is_err()
    );
}

#[test]
fn active_selection_rejects_aliases_and_keeps_saved_reads_fixed() {
    let dir = temp();
    let session = session(dir.path());
    assert_eq!(
        session.run_snapshot().read(None, |_, _| Ok(())).unwrap(),
        None
    );
    let saved = session.workspace().join("saved");
    fs::create_dir(&saved).unwrap();
    session
        .select_run(RunSelection::Saved(saved.clone()))
        .unwrap();
    let selected = session.run_snapshot();
    let identity = selected
        .read(None, |root, id| {
            assert_eq!(root, saved);
            Ok(id.to_owned())
        })
        .unwrap()
        .unwrap();

    let first = crate::store::new_run_stage_for(session.workspace(), "test-first").unwrap();
    fs::write(first.join("manifest.json"), b"{}").unwrap();
    crate::store::publish_run(session.workspace(), &first).unwrap();
    assert!(
        session
            .select_run(RunSelection::Saved(first.clone()))
            .is_err()
    );
    #[cfg(unix)]
    {
        let alias = session.workspace().join("alias");
        std::os::unix::fs::symlink(&first, &alias).unwrap();
        assert!(session.select_run(RunSelection::Saved(alias)).is_err());
    }
    assert_eq!(
        session
            .run_snapshot()
            .read(None, |_, id| Ok(id.to_owned()))
            .unwrap(),
        Some(identity)
    );
    session.select_run(RunSelection::Active).unwrap();
    let second = crate::store::new_run_stage_for(session.workspace(), "test-second").unwrap();
    fs::write(second.join("manifest.json"), b"{}").unwrap();
    let reader = session.run_snapshot();
    let workspace = session.workspace().to_path_buf();
    let second_path = second.clone();
    let (start, release) = sync_channel(0);
    let (completed, status) = sync_channel(1);
    let publication = thread::spawn(move || {
        release.recv().unwrap();
        completed
            .send(crate::store::publish_run(&workspace, &second_path))
            .unwrap();
    });
    let first_id = reader
        .read(None, |root, id| {
            assert_eq!(root, first);
            start.send(()).unwrap();
            assert!(status.recv_timeout(Duration::from_millis(70)).is_err());
            assert_eq!(fs::read(root.join("manifest.json")).unwrap(), b"{}");
            Ok(id.to_owned())
        })
        .unwrap();
    status.recv().unwrap().unwrap();
    publication.join().unwrap();
    let second_id = reader
        .read(None, |root, id| {
            assert_eq!(root, second);
            Ok(id.to_owned())
        })
        .unwrap();
    assert_ne!(first_id, second_id);
    assert!(reader.read(Some(&first), |_, _| Ok(())).is_err());
}

fn call(endpoint: &str, credential: &str, message: Value) -> Option<Value> {
    let mut stream = TcpStream::connect(endpoint).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    write_message(
        &mut stream,
        &serde_json::json!({"credential":credential,"message":message}),
    )
    .ok()?;
    let bytes = bounded_line(&mut BufReader::new(stream)).ok()??;
    serde_json::from_slice(&bytes).ok()
}

fn tool(id: &str, arguments: Value) -> Value {
    serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"run_command","arguments":{"id":id,"arguments":arguments}
    }})
}

fn content(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
#[ignore = "requires permission to bind a loopback TCP listener"]
fn endpoint_revokes_old_credentials_and_cancels_waiting_requests() {
    let dir = temp();
    let session = session(dir.path());
    open_setup(&session);
    let (events, received) = sync_channel(8);
    let tools = Arc::new(Tools::new(session.clone()));
    let bridge = Bridge::start(tools.clone(), events.clone().into()).unwrap();
    let (endpoint, token) = bridge.connection();
    let initialize = serde_json::json!({"jsonrpc":"2.0","method":"initialize","id":1,"params":{"protocolVersion":"2025-06-18"}});
    assert!(call(endpoint, "wrong", initialize.clone()).is_none());
    let reply = call(endpoint, token, initialize.clone()).unwrap();
    assert_eq!(reply["result"]["protocolVersion"], "2025-06-18");
    let connected = matches!(
        received.recv_timeout(Duration::from_secs(2)),
        Ok(AssistantEvent::ToolConnected)
    );
    assert!(connected);

    let args =
        serde_json::json!({"revision":session.run_setup().unwrap().revision,"forecast_days":5});
    tools.begin(5);
    let proposed = call(
        endpoint,
        token,
        tool("run.propose_parameters", args.clone()),
    )
    .unwrap();
    assert_eq!(content(&proposed)["status"], "staged");
    tools.finish(5, false);
    tools.begin(6);
    let second = call(
        endpoint,
        token,
        tool("run.propose_parameters", args.clone()),
    )
    .unwrap();
    assert_eq!(content(&second)["findings"][0]["code"], "review_pending");
    tools.finish(6, true);
    assert!(tools.state().proposal.is_some());
    tools.state().proposal = None;
    tools.begin(7);
    let proposed = call(endpoint, token, tool("run.propose_parameters", args)).unwrap();
    assert_eq!(content(&proposed)["status"], "staged");
    tools.finish(7, true);
    assert!(tools.state().proposal.is_none());

    tools.begin(8);
    let mut held = TcpStream::connect(endpoint).unwrap();
    held.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !tools
        .state()
        .sockets
        .values()
        .any(|(turn, _)| *turn == Some(8))
    {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    tools.finish(8, true);
    tools.begin(9);
    let _ = write_message(
        &mut held,
        &serde_json::json!({"credential":token,"message":tool("run.parameters",serde_json::json!({}))}),
    );
    assert!(
        bounded_line(&mut BufReader::new(held))
            .ok()
            .flatten()
            .is_none()
    );
    let endpoint = endpoint.to_owned();
    let token = token.to_owned();
    drop(bridge);

    let replacement = Bridge::start(Arc::new(Tools::new(session)), events.into()).unwrap();
    let (next_endpoint, _) = replacement.connection();
    assert!(call(next_endpoint, &token, initialize.clone()).is_none());
    assert!(call(&endpoint, &token, initialize).is_none());
}

#[cfg(unix)]
#[test]
fn fake_acp_agent_exercises_followups_permissions_cancellation_and_failures() {
    let root = temp();
    let session = session(root.path());
    let script = root.path().join("agent.sh");
    fs::write(&script, r#"
read -r line || exit
case "$1" in
malformed) printf '%s\n' 'not JSON'; exit;;
version) printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":2}}'; exit;;
esac
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
read -r line || exit
case "$1" in
signin) printf '%s\n' '{"jsonrpc":"2.0","id":2,"error":{"code":-32000,"message":"Sign in required"}}'; exit;;
esac
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"fixture"}}'
while IFS= read -r line; do
  case "$line" in
    *session/prompt*)
      id=${line#*\"id\":}; id=${id%%,*}
      printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"streamed text"}}}}'
      case "$line" in
        *permission*|*no-deny*)
          if [ "${line#*no-deny}" != "$line" ]; then
            options='[{"kind":"allow_once","optionId":"allow","name":"Allow once"}]'
          else
            options='[{"kind":"allow_once","optionId":"allow","name":"Allow once"},{"kind":"reject_once","optionId":"deny","name":"Reject once"}]'
          fi
          printf '{"jsonrpc":"2.0","id":"permission","method":"session/request_permission","params":{"sessionId":"fixture","toolCall":{"title":"fixture","rawInput":{"path":"/selected/path","command":"read fixture"}},"options":%s}}\n' "$options"
          IFS= read -r answer || exit
          case "$answer" in
            *session/cancel*)
              IFS= read -r answer || exit
              case "$answer" in *cancelled*) reason=cancelled;; *) exit 5;; esac;;
            *optionId*deny*) reason=end_turn;;
            *) exit 6;;
          esac;;
        *ignore*) while IFS= read -r ignored; do :; done; exit;;
        *) reason=end_turn;;
      esac
      printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"%s"}}\n' "$id" "$reason";;
  esac
done
"#).unwrap();
    let helper = std::env::current_exe().unwrap();
    let script = script.canonicalize().unwrap();
    let launch = |mode: &str| {
        Conversation::launch(
            session.clone(),
            helper.clone(),
            protocol::Launch {
                executable: "sh".into(),
                args: vec![script.clone().into_os_string(), mode.into()],
            },
            Bridge::disconnected,
            None,
        )
    };
    let mut conversation = launch("normal");
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Handshake)
    });
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Ready)
    });
    conversation.prompt("normal".into()).unwrap();
    event(
        &mut conversation,
        |event| matches!(event, AssistantEvent::Text(text) if text == "streamed text"),
    );
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: false })
    });

    conversation.prompt("permission".into()).unwrap();
    let AssistantEvent::Permission {
        request,
        operation,
        options,
    } = event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Permission { .. })
    })
    else {
        unreachable!()
    };
    assert_eq!(operation["rawInput"]["path"], "/selected/path");
    assert_eq!(options[1].id, "deny");
    conversation.answer(request, "deny".into()).unwrap();
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: false })
    });

    conversation.prompt("permission".into()).unwrap();
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Permission { .. })
    });
    conversation.stop().unwrap();
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::PermissionCleared)
    });
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: true })
    });

    conversation.prompt("no-deny".into()).unwrap();
    event(
        &mut conversation,
        |event| matches!(event, AssistantEvent::ToolStatus { title: Some(text), .. } if text.contains("no one-time deny")),
    );
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: true })
    });

    conversation.prompt("ignore".into()).unwrap();
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Text(_))
    });
    conversation.stop().unwrap();
    event(&mut conversation, |event| {
        matches!(
            event,
            AssistantEvent::Failed {
                stage: AssistantStage::Cancellation,
                closed: true,
                ..
            }
        )
    });
    drop(conversation);

    for (mode, expected) in [
        ("version", AssistantStage::Protocol),
        ("malformed", AssistantStage::Protocol),
        ("signin", AssistantStage::SignIn),
    ] {
        let mut conversation = launch(mode);
        event(
            &mut conversation,
            |event| matches!(event, AssistantEvent::Failed { stage, closed:true,..} if *stage == expected),
        );
    }
}

#[cfg(unix)]
#[test]
fn acp_settings_are_confirmed_and_refresh_dependent_choices() {
    let root = temp();
    let workspace = session(root.path());
    let script = root.path().join("settings-agent.sh");
    fs::write(&script, r#"
IFS= read -r line || exit
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1}}'
IFS= read -r line || exit
model_a='{"id":"choose_model","category":"model","type":"select","currentValue":"a","options":[{"group":"all","options":[{"value":"a","name":"Model A"},{"value":"b","name":"Model B"}]}]}'
model_b='{"id":"choose_model","category":"model","type":"select","currentValue":"b","options":[{"value":"a","name":"Model A"},{"value":"b","name":"Model B"}]}'
effort_a='{"id":"choose_effort","category":"thought_level","type":"select","currentValue":"low","options":[{"value":"low","name":"Low"}]}'
effort_b='{"id":"choose_effort","category":"thought_level","type":"select","currentValue":"high","options":[{"value":"high","name":"High"},{"value":"xhigh","name":"Extra high"}]}'
case "$1" in
  legacy)
    printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"fixture","models":{"currentModelId":"legacy-a","availableModels":[{"modelId":"legacy-a","name":"Legacy A"},{"modelId":"legacy-b","name":"Legacy B"}]}}}';;
  *)
    printf '{"jsonrpc":"2.0","id":2,"result":{"sessionId":"fixture","configOptions":[%s,%s]}}\n' "$model_a" "$effort_a";;
esac
while IFS= read -r line; do
  id=${line#*\"id\":}; id=${id%%,*}
  case "$line" in
    *session/set_config_option*choose_model*)
      if [ "$1" = timeout ]; then continue; fi
      printf '{"jsonrpc":"2.0","id":%s,"result":{"configOptions":[%s,%s]}}\n' "$id" "$model_b" "$effort_b"
      printf '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"other","update":{"sessionUpdate":"config_option_update","configOptions":[%s,%s]}}}\n' "$model_a" "$effort_a";;
    *session/set_config_option*choose_effort*)
      printf '{"jsonrpc":"2.0","id":%s,"error":{"code":-32602,"message":"Effort rejected"}}\n' "$id"
      printf '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"config_option_update","configOptions":[%s,%s]}}}\n' "$model_a" "$effort_a";;
    *session/set_model*legacy-b*)
      printf '{"jsonrpc":"2.0","id":%s,"result":{}}\n' "$id";;
    *session/prompt*)
      printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"agent_thought_chunk","content":{"type":"text","text":"adapter summary test-credential"}}}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"tool_call","toolCallId":"tool-1","title":"read test-credential","status":"in_progress"}}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"tool_call_update","toolCallId":"tool-1","status":"completed"}}}'
      printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"answer"}}}}'
      printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$id";;
    *) exit 1;;
  esac
done
"#).unwrap();
    let script = script.canonicalize().unwrap();
    let helper = std::env::current_exe().unwrap();
    let (wake, woke) = std::sync::mpsc::channel();
    let launch = |mode: &str| {
        let wake = wake.clone();
        Conversation::launch(
            workspace.clone(),
            helper.clone(),
            protocol::Launch {
                executable: "sh".into(),
                args: vec![script.clone().into_os_string(), mode.into()],
            },
            Bridge::disconnected,
            Some(Arc::new(move || {
                let _ = wake.send(());
            })),
        )
    };
    let mut modern = launch("modern");
    woke.recv_timeout(Duration::from_secs(4)).unwrap();
    let AssistantEvent::Settings(initial) = event(&mut modern, |event| {
        matches!(event, AssistantEvent::Settings(_))
    }) else {
        unreachable!()
    };
    assert_eq!(initial.model.unwrap().choices[1].label, "Model B");
    assert_eq!(initial.effort.unwrap().selected, "low");
    event(&mut modern, |event| matches!(event, AssistantEvent::Ready));
    modern.set_option(SettingKind::Model, "b".into()).unwrap();
    let AssistantEvent::SettingChanged {
        settings,
        error: None,
    } = event(&mut modern, |event| {
        matches!(event, AssistantEvent::SettingChanged { .. })
    })
    else {
        unreachable!()
    };
    assert_eq!(settings.model.unwrap().selected, "b");
    assert_eq!(settings.effort.unwrap().selected, "high");
    modern
        .set_option(SettingKind::Effort, "low".into())
        .unwrap();
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::SettingChanged { error: Some(error), .. } if error == "The agent no longer offers that value"),
    );
    modern
        .set_option(SettingKind::Effort, "xhigh".into())
        .unwrap();
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::SettingChanged { settings, error: Some(error) } if error == "Effort rejected" && settings.effort.as_ref().is_some_and(|value| value.selected == "high")),
    );
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::Settings(settings) if settings.effort.as_ref().is_some_and(|value| value.selected == "low")),
    );
    modern.prompt("question".into()).unwrap();
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::Thought(thought) if thought == "adapter summary [redacted]"),
    );
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::ToolStatus { id: Some(id), title: Some(title), status: Some(status) } if id == "tool-1" && title == "read [redacted]" && status == "in_progress"),
    );
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::ToolStatus { id: Some(id), title: None, status: Some(status) } if id == "tool-1" && status == "completed"),
    );
    event(
        &mut modern,
        |event| matches!(event, AssistantEvent::Text(answer) if answer == "answer"),
    );
    event(&mut modern, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: false })
    });

    let mut legacy = launch("legacy");
    let AssistantEvent::Settings(initial) = event(&mut legacy, |event| {
        matches!(event, AssistantEvent::Settings(_))
    }) else {
        unreachable!()
    };
    assert!(initial.effort.is_none());
    assert_eq!(initial.model.unwrap().selected, "legacy-a");
    event(&mut legacy, |event| matches!(event, AssistantEvent::Ready));
    legacy
        .set_option(SettingKind::Model, "legacy-b".into())
        .unwrap();
    event(
        &mut legacy,
        |event| matches!(event, AssistantEvent::SettingChanged { settings, error: None } if settings.model.as_ref().is_some_and(|value| value.selected == "legacy-b")),
    );

    let mut stalled = launch("timeout");
    event(&mut stalled, |event| matches!(event, AssistantEvent::Ready));
    stalled.set_option(SettingKind::Model, "b".into()).unwrap();
    event(&mut stalled, |event| {
        matches!(
            event,
            AssistantEvent::Failed {
                stage: AssistantStage::Configuration,
                closed: true,
                ..
            }
        )
    });
}

#[cfg(unix)]
fn event(
    conversation: &mut Conversation,
    predicate: impl Fn(&AssistantEvent) -> bool,
) -> AssistantEvent {
    let deadline = Instant::now() + Duration::from_secs(6);
    loop {
        assert!(
            Instant::now() < deadline,
            "Timed out waiting for an ACP event"
        );
        if let Some(event) = conversation.try_event() {
            if predicate(&event) {
                return event;
            }
            let still_open = !matches!(event, AssistantEvent::Failed { closed: true, .. });
            assert!(still_open, "{event:?}");
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn series_queries_keep_masks_units_and_detect_stale_sources() {
    use crate::store::{
        BasinGeometry, IntervalAxis, OperationManifest, Provenance, ResultSeries, ScenarioRecord,
        StationResults, TimeInterval, write_result_series, write_station_results,
    };
    use chrono::{Duration, NaiveDateTime};

    let dir = temp();
    let session = session(dir.path());
    let first = crate::store::new_run_stage_for(session.workspace(), "series-first").unwrap();
    let reference: NaiveDateTime = "2026-02-01T00:00:00".parse().unwrap();
    let start = reference - Duration::hours(1);
    let end = reference + Duration::hours(1);
    let timeline = IntervalAxis {
        time_zone: "America/Sao_Paulo".into(),
        bounds: vec![
            TimeInterval {
                start,
                end: reference,
            },
            TimeInterval {
                start: reference,
                end,
            },
        ],
    };
    let provenance = Provenance {
        source_ids: vec!["retained".into()],
        processing: serde_json::json!({"reference_time":reference,"engine_sha256":"a".repeat(64),"snapshot_sha256":"b".repeat(64)}),
    };
    let result_dir = first.join("results/main");
    fs::create_dir_all(&result_dir).unwrap();
    write_result_series(
        &result_dir.join("basin.nc"),
        &ResultSeries {
            timeline: timeline.clone(),
            basins: BasinGeometry {
                mini_ids: vec![7],
                latitudes: vec![-30.],
                longitudes: vec![-50.],
                source: Provenance {
                    source_ids: vec![],
                    processing: Value::Null,
                },
            },
            time_segment: vec![0, 1],
            precipitation: vec![0., 0.],
            precipitation_valid: vec![true, false],
            flow: vec![0., 3.],
            flow_valid: vec![true, true],
            raw_level_cm: vec![120, 140],
            raw_level_valid: vec![true, true],
            provenance: provenance.clone(),
        },
    )
    .unwrap();
    write_station_results(
        &result_dir.join("station.nc"),
        &StationResults {
            timeline,
            time_segment: vec![0, 1],
            station_ids: vec!["ana:7".into()],
            mini_ids: vec![Some(7)],
            offsets_m: vec![Some(1.0)],
            level_cm: vec![20, 40],
            level_valid: vec![true, true],
            provenance,
        },
    )
    .unwrap();
    let scenario = |id: &str| ScenarioRecord {
        id: id.into(),
        state: "completed".into(),
        started_at: None,
        finished_at: None,
        exit_code: None,
        signal: None,
        error: None,
        stdout: None,
        stderr: None,
        basin_result: None,
        station_result: None,
    };
    let manifest = OperationManifest {
        schema_version: 1,
        preparation_sha256: None,
        state: "completed".into(),
        recipe: Value::Null,
        inputs: vec![],
        scenarios: vec![scenario("broken"), scenario("main")],
        created_at: reference.to_string(),
        finished_at: None,
        error: None,
    };
    let bytes = serde_json::to_vec(&manifest).unwrap();
    fs::write(first.join("manifest.json"), &bytes).unwrap();
    let saved = session.workspace().join("saved");
    fs::create_dir(&saved).unwrap();
    fs::write(saved.join("manifest.json"), &bytes).unwrap();
    crate::store::publish_run(session.workspace(), &first).unwrap();
    let summary = result(commands::run(
        &session,
        "run.summary",
        serde_json::json!({}),
    ));
    assert_eq!(summary["scenarios"][0]["basin_results"], false);
    assert_eq!(summary["scenarios"][1]["basin_results"], true);
    assert_eq!(summary["findings"].as_array().unwrap().len(), 1);
    let identity = &summary["run"];
    let query = |variable: &str, location: Value, start, end| serde_json::json!({"location":location,"scenario":"main","variable":variable,"start":start,"end":end,"expected_run":identity});
    let mini = serde_json::json!({"mini":7});
    let station = serde_json::json!({"station":"ana:7"});
    let rainfall = result(commands::run(
        &session,
        "series.query",
        query("rain", mini.clone(), start, end),
    ));
    let report = &rainfall["report"];
    assert_eq!(report["time_zone"], "America/Sao_Paulo");
    assert_eq!(report["series"][0]["unit"], "mm");
    assert_eq!(report["series"][0]["points"][0]["value"], 0.0);
    assert_eq!(report["series"][0]["points"][1]["value"], Value::Null);
    let raw = result(commands::run(
        &session,
        "series.query",
        query("level", mini.clone(), start, end),
    ));
    assert_eq!(raw["level_reference"], "raw_basin");
    assert_eq!(raw["report"]["series"][0]["points"][0]["value"], 120.);
    let referenced = result(commands::run(
        &session,
        "series.query",
        query("level", station, start, end),
    ));
    assert_eq!(referenced["level_reference"], "station_offset");
    assert_eq!(referenced["report"]["series"][0]["unit"], "cm");
    assert_eq!(referenced["report"]["series"][0]["points"][0]["value"], 20.);
    let at_limit = commands::run(
        &session,
        "series.query",
        query("flow", mini.clone(), start, start + Duration::hours(999)),
    );
    let over_limit = commands::run(
        &session,
        "series.query",
        query("flow", mini.clone(), start, start + Duration::hours(1000)),
    );
    assert!(at_limit.is_ok());
    let Err(finding) = over_limit else {
        panic!("Expected a limit finding");
    };
    let finding = serde_json::to_value(finding).unwrap();
    assert_eq!(finding["code"], "series_limit");

    session.select_run(RunSelection::Saved(saved)).unwrap();
    let stale = commands::run(
        &session,
        "series.query",
        query("rain", mini.clone(), start, end),
    );
    let Err(finding) = stale else {
        panic!("Expected a stale run");
    };
    let finding = serde_json::to_value(finding).unwrap();
    assert_eq!(finding["code"], "stale_run");
    session.select_run(RunSelection::Active).unwrap();
    let second = crate::store::new_run_stage_for(session.workspace(), "series-second").unwrap();
    fs::write(second.join("manifest.json"), bytes).unwrap();
    crate::store::publish_run(session.workspace(), &second).unwrap();
    let stale = commands::run(&session, "series.query", query("rain", mini, start, end));
    let Err(finding) = stale else {
        panic!("Expected a stale run after publication");
    };
    let finding = serde_json::to_value(finding).unwrap();
    assert_eq!(finding["code"], "stale_run");
}

#[test]
fn location_search_returns_sorted_ambiguous_station_matches() {
    use crate::store::{ObservationBatch, ObservationStore, Station};
    let dir = temp();
    let session = session(dir.path());
    let mut store =
        ObservationStore::create(&crate::app::operation_database(session.workspace()), "UTC")
            .unwrap();
    let station = |number: &str| Station {
        station_id: format!("ana:{number}"),
        provider_code: "ana".into(),
        station_code: number.into(),
        name: "Passo Fundo".into(),
        latitude: None,
        longitude: None,
    };
    store
        .write_batch(&ObservationBatch {
            stations: vec![station("200"), station("100")],
            ..ObservationBatch::default()
        })
        .unwrap();
    let args = serde_json::json!({"query":"PASSO","limit":1});
    let limited = result(commands::run(&session, "location.search", args));
    assert_eq!(limited["matches"][0]["station"], "ana:100");
    assert_eq!(limited["more"], true);
    let args = serde_json::json!({"query":"passo"});
    let all = result(commands::run(&session, "location.search", args));
    assert_eq!(all["matches"][1]["station"], "ana:200");
    assert_eq!(all["more"], false);
}

#[test]
fn pending_review_tracks_its_origin_without_a_socket() {
    let dir = temp();
    let session = session(dir.path());
    open_setup(&session);
    let (events, observed) = sync_channel(8);
    let tools = Arc::new(Tools::new(session.clone()));
    let bridge = Bridge::disconnected(tools.clone(), events.clone().into()).unwrap();
    let (_, credential) = bridge.connection();
    let environment = bridge.environment(session.workspace()).unwrap();
    let hidden_credential = !environment.to_string().contains(credential);
    assert!(hidden_credential);
    let args =
        serde_json::json!({"revision":session.run_setup().unwrap().revision,"forecast_days":4});
    let message = tool("run.propose_parameters", args);
    let call =
        |credential| bridge.fixture_tool(credential, events.clone().into(), &message["params"]);
    tools.begin(3);
    assert!(call("wrong").is_none());
    assert_eq!(call(credential).unwrap().unwrap()["status"], "staged");
    let staged = matches!(observed.try_recv(), Ok(AssistantEvent::ReviewStaged));
    assert!(staged);
    tools.finish(3, false);
    tools.begin(4);
    let rejection = call(credential).unwrap().unwrap_err();
    assert_eq!(
        serde_json::to_value(rejection).unwrap()["code"],
        "review_pending"
    );
    tools.finish(4, true);
    assert!(tools.state().proposal.is_some());
    tools.state().proposal = None;
    tools.begin(5);
    assert_eq!(call(credential).unwrap().unwrap()["status"], "staged");
    tools.finish(5, true);
    assert!(tools.state().proposal.is_none());
    tools.close();
    assert!(call(credential).is_none());
}

#[cfg(unix)]
#[test]
fn saved_session_loads_with_stable_cwd_and_suppresses_replayed_output() {
    let root = temp();
    let session = session(root.path());
    let cwd = root.path().join("stable");
    fs::create_dir(&cwd).unwrap();
    let cwd = cwd.canonicalize().unwrap();
    let script = root.path().join("resume.sh");
    fs::write(&script, r#"
read -r line || exit
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"loadSession":true}}}'
read -r line || exit
case "$line" in *session/load*fixture*) ;; *) exit 4;; esac
printf '%s\n' '{"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"fixture","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"replayed"}}}}'
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{}}'
while IFS= read -r line; do
 case "$line" in *session/prompt*) id=${line#*\"id\":}; id=${id%%,*}; printf '{"jsonrpc":"2.0","id":%s,"result":{"stopReason":"end_turn"}}\n' "$id";; esac
done
"#).unwrap();
    let mut conversation = Conversation::launch_saved(
        session,
        std::env::current_exe().unwrap(),
        protocol::Launch {
            executable: "sh".into(),
            args: vec![script.canonicalize().unwrap().into_os_string()],
        },
        Bridge::disconnected,
        None,
        Some((cwd, Some("fixture".into()))),
    );
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Ready)
    });
    let no_replay = !matches!(conversation.try_event(), Some(AssistantEvent::Text(_)));
    assert!(no_replay);
    conversation.prompt("follow up".into()).unwrap();
    event(&mut conversation, |event| {
        matches!(event, AssistantEvent::Finished { cancelled: false })
    });
}
