//! The public CLI leaves the same requests as both desktop views.
use allomos_monitor::{
    flow::{Run, Scope},
    registry::{self, Held, Record, Request},
};
use std::{fs, process::Command};

#[test]
fn stop_now_and_graceful_stop_leave_distinct_requests_for_only_the_named_run() {
    let dir = std::env::temp_dir().join(format!(
        "allomos-cli-stop-{}",
        allomos_monitor::runner::new_session_id()
    ));
    fs::create_dir_all(&dir).unwrap();
    let root = allomos_store::init(&dir.join("project"), "test", "Synthetic").unwrap();
    let settings = dir.join("settings");
    let monitors = settings.join("monitors");
    fs::create_dir_all(&monitors).unwrap();
    let mut held = Vec::new();
    for id in ["first", "second"] {
        let record = Record {
            format: registry::FORMAT,
            id: id.into(),
            pid: std::process::id(),
            taskbag: root.clone(),
            project: dir.join("project"),
            language: "en".into(),
            log: registry::log_path(&monitors, id),
            alive: 0,
            run: Run::start(Scope::All, vec![], vec![], true, 30, 0),
            program: registry::Program::default(),
        };
        held.push(
            Held::take(&registry::lock_path(&monitors, id))
                .unwrap()
                .unwrap(),
        );
        fs::write(
            registry::record_path(&monitors, id),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
    }
    for (extra, expected) in [(vec![], Request::Stop), (vec!["--now"], Request::StopNow)] {
        let out = Command::new(env!("CARGO_BIN_EXE_alm"))
            .args(["monitor", "stop", "--run", "first", "--taskbag"])
            .arg(&root)
            .args(extra)
            .env("ALLOMOS_SETTINGS_DIR", &settings)
            .env_remove("ALLOMOS_TASKBAG")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(registry::take_requests(&monitors, "first"), [expected]);
        assert!(registry::take_requests(&monitors, "second").is_empty());
        assert!(root.join("project.json").exists());
    }
    for lock in held {
        lock.release();
    }
    fs::remove_dir_all(dir).unwrap();
}
