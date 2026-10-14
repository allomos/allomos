//! Changes made in the app's task pane (Task-A01-18): the same rules, lock and history as `alm`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use allomos_core::ops::{self, Edit};
use allomos_core::{Problem, State};
use allomos_store::{Actor, DEFAULT_DIR, LOCK_FILE, Lock, change_task, events, init, load};

fn tmp(name: &str) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("allomos-edits-{name}-{n}"));
    fs::create_dir_all(&p).unwrap();
    p
}

/// A taskbag with area A01, an iteration I01, an unheld task A01-01 and A01-02 held by an agent.
fn taskbag(name: &str) -> (PathBuf, PathBuf) {
    let dir = tmp(name);
    let root = dir.join(DEFAULT_DIR);
    init(&root, "p", "P").unwrap();
    let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_iteration(&ops::new_iteration(&bag, "I01", "One", Default::default()).unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why.")).unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_task(&ops::new_task(&bag, "A01-02", "A01", "Other", None).unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.save_task(
        &ops::claim(&bag, "A01-02", Some("claude/x"), false)
            .unwrap()
            .task,
    )
    .unwrap();
    drop(lock);
    (dir, root)
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

const APP: Actor<'static> = Actor {
    by: Some("Will"),
    via: "app",
};

fn edit(root: &Path, id: &str, e: Edit) -> Result<allomos_store::Changed, Problem> {
    change_task(root, APP, Duration::from_secs(5), |bag| {
        ops::edit(bag, id, APP.by, &e)
    })
}

#[test]
fn every_field_changes_and_is_recorded_under_the_persons_name() {
    let (dir, root) = taskbag("fields");
    let edits = [
        Edit::Label {
            label: "implemented".into(),
        },
        Edit::NextDependency {
            text: Some("the owner's check".into()),
        },
        Edit::HandoffNote {
            text: Some("Read the card first.".into()),
        },
        Edit::Iteration {
            iteration: Some("I01".into()),
        },
        Edit::Title {
            title: "Do it well".into(),
        },
        Edit::Description {
            text: "A new *why*.".into(),
        },
        Edit::WaitingOn {
            text: "an answer".into(),
        },
        Edit::State {
            state: State::Ready,
            waiting_on: None,
            who: None,
            blocked_by: vec![],
        },
        Edit::State {
            state: State::Waiting,
            waiting_on: Some("a reply on the licence".into()),
            who: Some("Ann".into()),
            blocked_by: vec![],
        },
        Edit::State {
            state: State::Blocked,
            waiting_on: None,
            who: None,
            blocked_by: vec!["A01-02".into()],
        },
        Edit::State {
            state: State::Ready,
            waiting_on: None,
            who: None,
            blocked_by: vec![],
        },
        Edit::BlockedBy {
            ids: vec!["A01-02".into()],
        },
        Edit::State {
            state: State::Done,
            waiting_on: None,
            who: None,
            blocked_by: vec![],
        },
    ];
    let count = edits.len();
    for e in edits {
        edit(&root, "A01-01", e).unwrap();
    }
    let t = load(&root)
        .taskbag
        .unwrap()
        .tasks
        .into_iter()
        .find(|t| t.id == "A01-01")
        .unwrap();
    assert_eq!(t.label.as_deref(), Some("implemented"));
    assert_eq!(t.next_dependency.as_deref(), Some("the owner's check"));
    assert_eq!(t.handoff_note.as_deref(), Some("Read the card first."));
    assert_eq!(t.iteration.as_deref(), Some("I01"));
    assert_eq!(t.title, "Do it well");
    assert_eq!(t.description, "A new *why*.\n");
    assert_eq!(t.state, State::Done);
    assert!(t.blocked_by.is_empty() && t.who.is_none() && t.waiting_on.is_none());
    assert_eq!(
        fs::read_to_string(root.join("tasks/A01-01/description.md")).unwrap(),
        "A new *why*.\n"
    );

    // One event per change, by the person, through the app, named after the alm command.
    let h = events::history(&root, "A01-01");
    assert_eq!(h.events.len(), count);
    assert!(
        h.events
            .iter()
            .all(|e| e.by.as_deref() == Some("Will") && e.via == "app")
    );
    let commands: Vec<&str> = h.events.iter().rev().map(|e| e.command.as_str()).collect();
    assert_eq!(
        commands,
        [
            "label", "needs", "note", "plan", "title", "describe", "wait", "ready", "wait",
            "block", "ready", "block", "done"
        ]
    );

    // An empty description removes the file.
    edit(&root, "A01-01", Edit::Description { text: "  ".into() }).unwrap();
    assert!(!root.join("tasks/A01-01/description.md").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_refused_edit_changes_no_file() {
    let (dir, root) = taskbag("refused");
    let before = snapshot(&root);
    let refusals = [
        // Someone else holds it: the holder rule, as `alm label` gives it.
        (
            "A01-02",
            Edit::Label {
                label: "implemented".into(),
            },
        ),
        (
            "A01-02",
            Edit::Description {
                text: "mine now".into(),
            },
        ),
        (
            "A01-01",
            Edit::Label {
                label: "not on the ladder".into(),
            },
        ),
        ("A01-01", Edit::Title { title: " ".into() }),
        (
            "A01-01",
            Edit::Iteration {
                iteration: Some("I09".into()),
            },
        ),
        (
            "A01-01",
            Edit::State {
                state: State::Waiting,
                waiting_on: None,
                who: None,
                blocked_by: vec![],
            },
        ),
        (
            "A01-01",
            Edit::State {
                state: State::InProgress,
                waiting_on: None,
                who: None,
                blocked_by: vec![],
            },
        ),
        // Blocked needs another task of this taskbag (decision 0016).
        (
            "A01-01",
            Edit::State {
                state: State::Blocked,
                waiting_on: None,
                who: None,
                blocked_by: vec![],
            },
        ),
        (
            "A01-01",
            Edit::BlockedBy {
                ids: vec!["A01-01".into()],
            },
        ),
        (
            "A01-01",
            Edit::BlockedBy {
                ids: vec!["A09-09".into()],
            },
        ),
        (
            "A09-09",
            Edit::Title {
                title: "Nope".into(),
            },
        ),
    ];
    for (id, e) in refusals {
        let bag = load(&root).taskbag.unwrap();
        let want = ops::edit(&bag, id, APP.by, &e).unwrap_err();
        assert_eq!(edit(&root, id, e).unwrap_err(), want);
        assert_eq!(snapshot(&root), before, "{want:?} changed a file");
    }
    assert_eq!(
        edit(
            &root,
            "A01-02",
            Edit::Label {
                label: "implemented".into()
            }
        )
        .unwrap_err(),
        Problem::TaskHeld {
            id: "A01-02".into(),
            holder: "claude/x".into()
        }
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_app_waits_for_the_lock_then_takes_it() {
    let (dir, root) = taskbag("lock");
    // Another writer (alm takes exactly this lock) holds the taskbag for a moment.
    let held = Lock::acquire(&root, Duration::ZERO).unwrap();
    let before = snapshot(&root);

    // A short wait gives up with `taskbag_busy` and changes nothing.
    let busy = change_task(&root, APP, Duration::from_millis(50), |bag| {
        ops::edit(bag, "A01-01", APP.by, &Edit::Title { title: "x".into() })
    });
    assert!(matches!(busy, Err(Problem::TaskbagBusy { .. })));
    assert_eq!(snapshot(&root), before);

    // The usual wait: the change happens once the other writer lets go.
    let other = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        drop(held);
    });
    let start = Instant::now();
    edit(
        &root,
        "A01-01",
        Edit::Title {
            title: "After the wait".into(),
        },
    )
    .unwrap();
    assert!(start.elapsed() >= Duration::from_millis(250));
    other.join().unwrap();
    let bag = load(&root).taskbag.unwrap();
    assert_eq!(bag.tasks[0].title, "After the wait");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_too_old_taskbag_is_not_changed() {
    let (dir, root) = taskbag("older");
    let project = root.join("project.json");
    let text = fs::read_to_string(&project).unwrap();
    fs::write(
        &project,
        text.replace(allomos_core::FORMAT, "allomos/2").as_bytes(),
    )
    .unwrap();
    let before = snapshot(&root);
    assert!(matches!(
        edit(&root, "A01-01", Edit::Title { title: "x".into() }),
        Err(Problem::TaskbagTooOld { .. })
    ));
    assert_eq!(snapshot(&root), before);
    fs::remove_dir_all(dir).unwrap();
}
