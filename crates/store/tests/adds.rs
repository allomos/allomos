//! New areas, iterations and tasks made in the app (Task-A01-19): the same rules, lock and history
//! as `alm add`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use allomos_core::ops::IterationDetails;
use allomos_core::{Problem, State};
use allomos_store::{
    Actor, DEFAULT_DIR, LOCK_FILE, Lock, NewTask, add_area, add_iteration, add_task, events, init,
    load,
};

const APP: Actor<'static> = Actor {
    by: Some("Will"),
    via: "app",
};
const WAIT: Duration = Duration::from_secs(5);

fn taskbag(name: &str) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("allomos-adds-{name}-{n}"));
    let root = dir.join(DEFAULT_DIR);
    init(&root, "p", "P").unwrap();
    root
}

/// Every file in the taskbag and its contents, except the lock.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().unwrap() != LOCK_FILE {
                out.insert(p.clone(), fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
}

/// The events written since `from`, as (command, kind, id, by, via).
fn events_since(root: &Path, from: usize) -> Vec<(String, String, String, String, String)> {
    events::list(root).0[from..]
        .iter()
        .map(|e| {
            (
                e.command.clone(),
                format!("{:?}", e.kind).to_lowercase(),
                e.id.clone().unwrap_or_default(),
                e.by.clone().unwrap_or_default(),
                e.via.clone(),
            )
        })
        .collect()
}

fn ev(command: &str, kind: &str, id: &str) -> (String, String, String, String, String) {
    (
        command.into(),
        kind.into(),
        id.into(),
        "Will".into(),
        "app".into(),
    )
}

fn task(id: Option<&str>, area: &str, iteration: Option<&str>) -> NewTask {
    NewTask {
        id: id.map(String::from),
        area: area.into(),
        title: "Do it".into(),
        description: Some("Why.".into()),
        iteration: iteration.map(String::from),
    }
}

#[test]
fn each_new_item_writes_one_event_and_a_planned_task_two() {
    let root = taskbag("events");
    let start = events::list(&root).0.len();

    // Without an ID: the next free one, as `alm add` takes.
    let area = add_area(&root, APP, WAIT, None, "Tracking").unwrap().item;
    assert_eq!(area.id, "A01");
    assert_eq!(events_since(&root, start), [ev("add area", "area", "A01")]);

    let details = IterationDetails {
        release: Some("v0.1"),
        start: Some("2026-10-15"),
        end: Some("2026-10-30"),
    };
    let it = add_iteration(&root, APP, WAIT, None, "First", details)
        .unwrap()
        .item;
    assert_eq!(
        (it.id.as_str(), it.release.as_deref()),
        ("I01", Some("v0.1"))
    );
    assert_eq!(
        events_since(&root, start + 1),
        [ev("add iteration", "iteration", "I01")]
    );

    // A task without an iteration: one event, in backlog.
    let t = add_task(&root, APP, WAIT, &task(None, "A01", None))
        .unwrap()
        .item;
    assert_eq!(
        (t.id.as_str(), t.state, t.iteration.clone()),
        ("A01-01", State::Backlog, None)
    );
    assert_eq!(
        events_since(&root, start + 2),
        [ev("add task", "task", "A01-01")]
    );

    // With an iteration: added, then planned, under one lock: two events, as typing both.
    let t = add_task(&root, APP, WAIT, &task(Some("A01-07"), "A01", Some("I01")))
        .unwrap()
        .item;
    assert_eq!(
        (t.id.as_str(), t.iteration.as_deref()),
        ("A01-07", Some("I01"))
    );
    assert_eq!(
        events_since(&root, start + 3),
        [
            ev("add task", "task", "A01-07"),
            ev("plan", "task", "A01-07")
        ]
    );
    let all = events::list(&root).0;
    let plan = all.last().unwrap();
    assert_eq!(plan.changes.len(), 1);
    assert_eq!(plan.changes[0].field, "iteration");

    // The files say the same: `alm list` reads these.
    let bag = load(&root).taskbag.unwrap();
    let saved = bag.tasks.iter().find(|t| t.id == "A01-07").unwrap();
    assert_eq!(saved.iteration.as_deref(), Some("I01"));
    assert_eq!(saved.description, "Why.\n");
    assert_eq!(bag.iterations[0].end.as_deref(), Some("2026-10-30"));
}

#[test]
fn refusals_have_alms_codes_and_write_nothing() {
    let root = taskbag("refusals");
    add_area(&root, APP, WAIT, Some("A01"), "Tracking").unwrap();
    add_iteration(&root, APP, WAIT, Some("I01"), "One", Default::default()).unwrap();
    add_task(&root, APP, WAIT, &task(Some("A01-01"), "A01", None)).unwrap();
    let before = snapshot(&root);

    let bad_date = IterationDetails {
        start: Some("next week"),
        ..Default::default()
    };
    let cases: Vec<(Result<(), Problem>, Problem)> = vec![
        (
            add_area(&root, APP, WAIT, Some("A01"), "Again").map(|_| ()),
            Problem::DuplicateAreaId { id: "A01".into() },
        ),
        (
            add_area(&root, APP, WAIT, Some("bad id"), "x").map(|_| ()),
            Problem::InvalidId {
                id: "bad id".into(),
            },
        ),
        (
            add_area(&root, APP, WAIT, None, " ").map(|_| ()),
            Problem::AreaNameMissing,
        ),
        (
            add_iteration(&root, APP, WAIT, Some("I01"), "Again", Default::default()).map(|_| ()),
            Problem::DuplicateIterationId { id: "I01".into() },
        ),
        (
            add_iteration(&root, APP, WAIT, None, "Two", bad_date).map(|_| ()),
            Problem::InvalidDate {
                value: "next week".into(),
            },
        ),
        (
            add_task(&root, APP, WAIT, &task(Some("A01-01"), "A01", None)).map(|_| ()),
            Problem::DuplicateTaskId {
                id: "A01-01".into(),
            },
        ),
        (
            add_task(&root, APP, WAIT, &task(None, "A09", None)).map(|_| ()),
            Problem::UnknownArea { area: "A09".into() },
        ),
        (
            add_task(&root, APP, WAIT, &task(Some("X-1"), "A09", None)).map(|_| ()),
            Problem::UnknownArea { area: "A09".into() },
        ),
        // The plan step is checked before the task is written: neither happens.
        (
            add_task(&root, APP, WAIT, &task(None, "A01", Some("I09"))).map(|_| ()),
            Problem::UnknownIteration {
                iteration: "I09".into(),
            },
        ),
        (
            add_task(
                &root,
                APP,
                WAIT,
                &NewTask {
                    title: " ".into(),
                    ..task(None, "A01", None)
                },
            )
            .map(|_| ()),
            Problem::TaskTitleMissing,
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, Err(want));
    }
    assert_eq!(snapshot(&root), before, "a refusal changes no file");
}

#[test]
fn the_app_waits_for_alms_lock() {
    let root = taskbag("lock");
    let held = Lock::acquire(&root, Duration::ZERO).unwrap();
    assert!(matches!(
        add_area(&root, APP, Duration::from_millis(20), None, "x"),
        Err(Problem::TaskbagBusy { .. })
    ));
    drop(held);
    assert!(add_area(&root, APP, Duration::from_millis(20), None, "x").is_ok());
}

#[test]
fn the_new_task_form_arrives_as_json() {
    let n: NewTask = serde_json::from_str(
        r#"{"id":null,"area":"A01","title":"t","description":"","iteration":"I01"}"#,
    )
    .unwrap();
    assert_eq!((n.id, n.iteration.as_deref()), (None, Some("I01")));
    assert!(
        serde_json::from_str::<NewTask>(r#"{"area":"A01","title":"t","state":"ready"}"#).is_err()
    );
}
