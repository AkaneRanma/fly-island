use super::*;
use serde_json::json;

const ID: &str = "session-00000000-0000-4000-8000-000000000001";

fn fixture() -> Value {
    json!({"version":7,"record":{
        "identity":{"formatVersion":4,"createdAt":100,"cwd":"D:\\fictional-project","isSeeded":false},
        "rows":{
            "title":{"ver":1,"seq":12,"val":"Fictional session"},
            "sessionListMetadata":{"ver":1,"seq":12,"val":{"blank":false,"lastPromptAt":1000}},
            "turnBoundary":{"ver":2,"seq":12,"val":{"openTurnStartSeq":5,"lastTurn":1}},
            "turnOutline":{"ver":2,"seq":12,"val":{"turns":[{"turn":1,"seq":5,"prompt":"Example prompt","response":"Example response"}]}},
            "modelSelection":{"ver":2,"seq":12,"val":{"lastUsed":{"provider":"deepseek-official","model":"deepseek-v4","reasoningEffort":"high"}}},
            "userQuestions":{"ver":2,"seq":12,"val":{"questions":{"active":[]}}}
        }
    }})
}

#[test]
fn parses_known_and_newer_checkpoints_but_never_reshaped_rows() {
    let mut value = fixture();
    let session = parse_checkpoint(&value, ID, 1100, 1200, true).unwrap();
    assert_eq!(session.provider, "dsh");
    assert_eq!(session.title.as_deref(), Some("Fictional session"));
    assert_eq!(session.phase, "running");
    assert_eq!(session.model.as_deref(), Some("deepseek-v4"));
    assert_eq!(session.owner_prompt_at, Some(1000));
    assert!(!session.closed);
    // A newer DSH bumps generations without reshaping: still listed.
    value["version"] = json!(9);
    value["record"]["identity"]["formatVersion"] = json!(5);
    value["record"]["rows"]["turnBoundary"]["ver"] = json!(3);
    assert_eq!(parse_checkpoint(&value, ID, 1100, 1200, true).unwrap().phase, "running");
    // Reshaped rows are skipped, not guessed.
    value["record"]["rows"]["turnBoundary"]["val"] = json!({"open": true});
    assert!(parse_checkpoint(&value, ID, 1100, 1200, true).is_none());
    value = fixture();
    value["record"]["rows"]["sessionListMetadata"]["val"] = json!({"isBlank": false});
    assert!(parse_checkpoint(&value, ID, 1100, 1200, true).is_none());
    value = fixture();
    value["version"] = json!(2);
    assert!(parse_checkpoint(&value, ID, 1100, 1200, true).is_none());
    value = fixture();
    value["record"]["rows"]["turnBoundary"]["ver"] = json!(1);
    assert!(parse_checkpoint(&value, ID, 1100, 1200, true).is_none());
    assert!(parse_checkpoint(&fixture(), "../../example", 1100, 1200, true).is_none());
}

#[test]
fn falls_back_to_the_title_input_of_current_dsh() {
    let mut value = fixture();
    value["record"]["rows"]["title"] = Value::Null;
    value["record"]["rows"]["titleInput"] = json!({"ver":3,"seq":1,"val":{"first":{"seq":1,"text":"First prompt"},"count":1,"lastSeq":1}});
    assert_eq!(parse_checkpoint(&value, ID, 1100, 1200, true).unwrap().title.as_deref(), Some("First prompt"));
}

#[test]
fn source_log_of_any_generation_counts() {
    let root = std::env::temp_dir().join(format!("focusd-dsh-log-{}", std::process::id()));
    let shard = root.join("--D-fictional--");
    let _ = fs::remove_dir_all(&root);
    for (id, file) in [("session-a", "session.v4.jsonl.zstd"), ("session-b", "session.v5.jsonl.zst"), ("session-c", "notes.txt")] {
        fs::create_dir_all(shard.join(id)).unwrap();
        fs::write(shard.join(id).join(file), b"").unwrap();
    }
    let shards = vec![shard];
    assert!(has_source_log(&shards, "session-a"));
    assert!(has_source_log(&shards, "session-b"));
    assert!(!has_source_log(&shards, "session-c"));
    assert!(!has_source_log(&shards, "session-missing"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn differentiates_waiting_continued_done_stale_and_exited() {
    let mut value = fixture();
    value["record"]["rows"]["userQuestions"]["val"]["questions"]["active"] = json!([{"state":"open"}]);
    assert_eq!(parse_checkpoint(&value, ID, 1100, 9999999, true).unwrap().phase, "waiting");
    value["record"]["rows"]["userQuestions"]["val"]["questions"]["active"][0]["state"] = json!("continued");
    assert_eq!(parse_checkpoint(&value, ID, 1100, 9999999, true).unwrap().phase, "stale");
    value["record"]["rows"]["turnBoundary"]["val"]["openTurnStartSeq"] = Value::Null;
    assert_eq!(parse_checkpoint(&value, ID, 1100, 1200, true).unwrap().phase, "completed");
    assert!(parse_checkpoint(&fixture(), ID, 1100, 1200, false).unwrap().closed);
    value["record"]["rows"]["sessionListMetadata"]["val"]["blank"] = json!(true);
    assert!(parse_checkpoint(&value, ID, 1100, 1200, true).is_none());
}

#[test]
fn merges_imported_sessions_by_id_and_retains_newest_owner() {
    let mut sessions = HashMap::new();
    let old = parse_checkpoint(&fixture(), ID, 1000, 1200, false).unwrap();
    let mut new = parse_checkpoint(&fixture(), ID, 1100, 1200, true).unwrap();
    new.host.as_mut().unwrap().name = "DSH Desktop (dataelement)".into();
    merge_session(&mut sessions, new);
    merge_session(&mut sessions, old);
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[ID].host.as_ref().unwrap().name, "DSH Desktop (dataelement)");
    assert!(!sessions[ID].closed);
}

#[test]
fn a_pending_tool_approval_waits_instead_of_running() {
    let mut session = parse_checkpoint(&fixture(), ID, 1100, 1200, true).unwrap();
    apply_approval(&mut session, None);
    assert_eq!(session.phase, "running");
    let approval = super::super::dsh_approvals::PendingApproval { tool: Some("pwsh".into()), asked_at: 5000 };
    apply_approval(&mut session, Some(approval));
    assert_eq!(session.phase, "waiting");
    assert_eq!(session.updated_at, 5000);
    assert_eq!(session.current_tool.as_ref().map(|tool| tool.name.as_str()), Some("pwsh"));
}

#[test]
fn accepts_only_loopback_web_addresses() {
    assert_eq!(
        web_entry_url("http://127.0.0.1:58527/?token=abc\r\n"),
        Some("http://127.0.0.1:58527/?token=abc".to_string())
    );
    // Trimmed, and a lone trailing slash is dropped; the browser opens either form.
    assert_eq!(web_entry_url("  http://localhost:8080/  \n"), Some("http://localhost:8080".to_string()));
    assert_eq!(web_entry_url("https://[::1]:443"), Some("https://[::1]:443".to_string()));
    // Anything that would open a site the user did not start stays out.
    assert_eq!(web_entry_url("http://evil.example.com:80/"), None);
    assert_eq!(web_entry_url("file:///C:/Windows/System32/calc.exe"), None);
    assert_eq!(web_entry_url("javascript:alert(1)"), None);
    assert_eq!(web_entry_url("http://127.0.0.1/"), None);
    assert_eq!(web_entry_url("http://127.0.0.1:0/"), None);
    assert_eq!(web_entry_url("http://127.0.0.1:58527/?token=a b"), None);
    assert_eq!(web_entry_url(""), None);
}

#[test]
fn a_recorded_address_keeps_the_earlier_file_when_both_are_equally_old() {
    let stamp = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    assert!(fresher(None, stamp));
    // url.txt (the one carrying the token) is listed first; plain-url.txt written in the same
    // second must not take its place.
    assert!(!fresher(Some(&stamp), stamp));
    assert!(fresher(Some(&stamp), stamp + Duration::from_secs(1)));
}

#[test]
fn an_origin_without_a_path_still_answers_the_liveness_probe() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("fixture listener");
    let port = listener.local_addr().expect("fixture address").port();
    assert!(web_host_alive(&format!("http://127.0.0.1:{port}")));
    assert!(web_host_alive(&format!("http://127.0.0.1:{port}/?token=abc")));
    drop(listener);
    assert!(!web_host_alive(&format!("http://127.0.0.1:{port}")), "a closed host is not opened");
}

#[test]
fn recognises_the_recorded_page_by_its_window_or_tab_title() {
    assert!(shows_page("灵动岛效果对比 — DeepSeek Harness - Google Chrome"));
    assert!(shows_page("DeepSeek Harness"));
    assert!(!shows_page("GitHub — Mozilla Firefox"));
    assert!(!shows_page(""));
}
