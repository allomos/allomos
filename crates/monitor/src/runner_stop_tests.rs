//! End-to-end loop checks using synthetic taskbags, never a person's project.
use super::*;
use crate::flow::Scope;
use std::fs;
use std::process::Command;
use std::time::Instant;

fn until(mut yes: impl FnMut() -> bool) {
    let start = Instant::now();
    while !yes() {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "monitor did not settle"
        );
        std::thread::sleep(Duration::from_millis(30));
    }
}

#[test]
fn waiting_can_exit_without_touching_agents_tasks_or_other_runs() {
    let dir = std::env::temp_dir().join(format!("allomos-stop-now-{}", new_session_id()));
    fs::create_dir_all(&dir).unwrap();
    let root = allomos_store::init(&dir.join("project"), "test", "Synthetic").unwrap();
    let work = dir.join("project/unfinished.txt");
    fs::write(&work, "unfinished agent work").unwrap();
    let task_dir = root.join("tasks/A01-1");
    fs::create_dir_all(&task_dir).unwrap();
    let task = serde_json::json!({
        "id": "A01-1", "title": "Synthetic held task", "area": "A01",
        "state": "in_progress", "holder": "synthetic-session",
        // Still beating: a session at work (a lost one would leave the run, A03-husky861).
        "heartbeat": {"by": "synthetic-session",
            "at": allomos_core::heartbeat::format_utc(now_ms() / 1000)}
    });
    let task_path = task_dir.join("task.json");
    let original = serde_json::to_vec_pretty(&task).unwrap();
    fs::write(&task_path, &original).unwrap();
    let registry_dir = dir.join("monitors");
    fs::create_dir_all(&registry_dir).unwrap();
    // A harmless process stands for an already opened agent. The loop never owns its lifecycle.
    let mut agent = Command::new("sleep").arg("60").spawn().unwrap();
    let launch = |id: &str| {
        let mut run = Run::start(Scope::All, vec![], vec![], true, 30, now_ms());
        run.stopping = true;
        run.working.push(Working {
            agent: "codex_cli".into(),
            id: "A01-1".into(),
            at: now_ms(),
            session_id: None,
            tab: None,
            seen_held: true,
            seen: String::new(),
            changed: now_ms(),
            holder: Some("synthetic-session".into()),
            prior: None,
            prior_session: None,
        });
        let options = Options {
            taskbag: root.clone(),
            language: "en".into(),
            run,
            alm: None,
            updated_from: None,
        };
        let (d, id) = (registry_dir.clone(), id.to_string());
        std::thread::spawn(move || super::run(&d, &id, options).unwrap())
    };
    let first = launch("first");
    let second = launch("second");
    until(|| {
        registry::read_resume(&registry_dir, "first").is_some_and(|r| r.run.working.len() == 1)
    });
    until(|| registry::read_resume(&registry_dir, "second").is_some());
    assert!(
        registry::read_resume(&registry_dir, "first")
            .unwrap()
            .run
            .stopping
    );
    assert!(
        !first.is_finished(),
        "a task at work must not finish itself"
    );
    // The same JSON request is sent by both desktop monitor views.
    let request: Request = serde_json::from_str(r#"{"action":"stop_now"}"#).unwrap();
    registry::ask(&registry_dir, "first", &request).unwrap();
    // A competing check/undo cannot override the request to exit.
    registry::ask(&registry_dir, "first", &Request::UndoStop).unwrap();
    registry::ask(&registry_dir, "first", &Request::Check).unwrap();
    until(|| first.is_finished());
    first.join().unwrap();
    assert!(!registry::record_path(&registry_dir, "first").exists());
    assert!(registry::read_resume(&registry_dir, "first").is_none());
    assert!(!registry::held(&registry::lock_path(
        &registry_dir,
        "first"
    )));
    assert!(!second.is_finished());
    assert!(
        agent.try_wait().unwrap().is_none(),
        "agent must keep running"
    );
    assert_eq!(fs::read(&task_path).unwrap(), original, "no task mutation");
    assert_eq!(fs::read_to_string(&work).unwrap(), "unfinished agent work");

    // The other monitor still follows its task and exits normally when it is finished.
    let mut done = task;
    done["state"] = serde_json::json!("done");
    fs::write(&task_path, serde_json::to_vec_pretty(&done).unwrap()).unwrap();
    registry::ask(&registry_dir, "second", &Request::Stop).unwrap();
    until(|| second.is_finished());
    second.join().unwrap();
    assert!(registry::read_resume(&registry_dir, "second").is_none());
    assert!(agent.try_wait().unwrap().is_none());
    agent.kill().unwrap();
    agent.wait().unwrap();
    fs::remove_dir_all(dir).unwrap();
}
