//! Requests from the panel reach the existing loop; no agent is launched in this check.
use super::*;
use crate::flow::Scope;
use std::fs;

#[test]
fn requeue_request_changes_only_its_task_and_keeps_the_loop_running() {
    let dir = std::env::temp_dir().join(format!("allomos-requeue-{}", new_session_id()));
    fs::create_dir_all(&dir).unwrap();
    let root = allomos_store::init(&dir.join("project"), "test", "Synthetic").unwrap();
    let registry_dir = dir.join("monitors");
    fs::create_dir_all(&registry_dir).unwrap();
    let mut run = Run::start(Scope::All, vec![], vec![], true, 30, now_ms());
    run.given_up = vec!["A01-7".into(), "A01-8".into()];
    run.failures.insert("A01-7".into(), 3);
    run.failures.insert("A01-8".into(), 3);
    let options = Options {
        taskbag: root,
        language: "en".into(),
        run,
        alm: None,
        updated_from: None,
    };
    let d = registry_dir.clone();
    let handle = std::thread::spawn(move || super::run(&d, "first", options).unwrap());
    let until = |condition: &dyn Fn() -> bool| {
        let start = Instant::now();
        while !condition() {
            assert!(
                start.elapsed() < Duration::from_secs(15),
                "monitor did not settle"
            );
            std::thread::sleep(Duration::from_millis(30));
        }
    };
    until(&|| registry::read_resume(&registry_dir, "first").is_some());
    let request: Request = serde_json::from_str(r#"{"action":"requeue","id":"A01-7"}"#).unwrap();
    registry::ask(&registry_dir, "first", &request).unwrap();
    registry::ask(&registry_dir, "first", &request).unwrap();
    until(&|| {
        registry::read_resume(&registry_dir, "first").is_some_and(|r| r.run.given_up == ["A01-8"])
    });
    let record = registry::read_resume(&registry_dir, "first").unwrap();
    assert!(!record.run.failures.contains_key("A01-7"));
    assert_eq!(record.run.failures["A01-8"], 3);
    assert!(!record.run.stopping);
    assert!(!handle.is_finished());
    registry::ask(&registry_dir, "first", &Request::StopNow).unwrap();
    until(&|| handle.is_finished());
    handle.join().unwrap();
    fs::remove_dir_all(dir).unwrap();
}
