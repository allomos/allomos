//! Renaming, changing and deleting areas and iterations (Task-A01-29): the same rules, lock and
//! history for `alm` and the app.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use allomos_core::Problem;
use allomos_core::event::Kind;
use allomos_core::ops::{self, IterationChange, IterationDetails};
use allomos_store::{
    Actor, DEFAULT_DIR, LOCK_FILE, Lock, NewTask, add_area, add_iteration, add_task,
    change_iteration, change_task, delete_area, delete_iteration, events, init, load, rename_area,
};

const APP: Actor<'static> = Actor {
    by: Some("Will"),
    via: "app",
};
const WAIT: Duration = Duration::from_secs(5);

fn taskbag(name: &str) -> (PathBuf, PathBuf) {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("allomos-renames-{name}-{n}"));
    let root = dir.join(DEFAULT_DIR);
    init(&root, "p", "P").unwrap();
    add_area(&root, APP, WAIT, Some("A01"), "Tracking").unwrap();
    add_area(&root, APP, WAIT, Some("A02"), "Charts").unwrap();
    let details = IterationDetails {
        release: Some("v0.1"),
        start: Some("2026-10-01"),
        end: Some("2026-10-10"),
    };
    add_iteration(&root, APP, WAIT, Some("I01"), "One", details).unwrap();
    add_iteration(&root, APP, WAIT, Some("I02"), "Two", Default::default()).unwrap();
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

fn task(id: &str, area: &str, iteration: Option<&str>) -> NewTask {
    NewTask {
        id: Some(id.into()),
        area: area.into(),
        title: "Do it".into(),
        description: None,
        iteration: iteration.map(String::from),
    }
}

#[test]
fn each_change_writes_one_event() {
    let (dir, root) = taskbag("events");
    let count = || events::list(&root).0.len();
    let start = count();

    let area = rename_area(&root, APP, WAIT, "A01", "Tracking work")
        .unwrap()
        .item;
    assert_eq!(
        (area.id.as_str(), area.name.as_str()),
        ("A01", "Tracking work")
    );
    assert_eq!(count(), start + 1);
    let e = events::list(&root).0.pop().unwrap();
    assert_eq!(
        (
            e.command.as_str(),
            e.kind,
            e.id.as_deref(),
            e.by.as_deref(),
            e.via.as_str()
        ),
        ("rename area", Kind::Area, Some("A01"), Some("Will"), "app")
    );
    assert_eq!(e.changes.len(), 1);
    assert_eq!(
        (
            e.changes[0].field.as_str(),
            e.changes[0].from.clone(),
            e.changes[0].to.clone()
        ),
        (
            "name",
            Some("Tracking".into()),
            Some("Tracking work".into())
        )
    );

    let change = IterationChange {
        end: Some("2026-10-20".into()),
        release: Some(String::new()),
        ..Default::default()
    };
    let it = change_iteration(&root, APP, WAIT, "I01", &change)
        .unwrap()
        .item;
    assert_eq!(
        (it.end.as_deref(), it.release.as_deref()),
        (Some("2026-10-20"), None)
    );
    assert_eq!(count(), start + 2);
    let e = events::list(&root).0.pop().unwrap();
    assert_eq!(
        (e.command.as_str(), e.kind, e.id.as_deref()),
        ("change iteration", Kind::Iteration, Some("I01"))
    );
    let fields: Vec<&str> = e.changes.iter().map(|c| c.field.as_str()).collect();
    assert_eq!(fields, ["end", "release"]);
    assert!(e.changes[1].to.is_none(), "the release was cleared");

    // A change that changes nothing writes no event.
    change_iteration(&root, APP, WAIT, "I01", &change).unwrap();
    assert_eq!(count(), start + 2);

    let gone = delete_iteration(&root, APP, WAIT, "I02").unwrap().item;
    assert_eq!(gone.id, "I02");
    assert_eq!(count(), start + 3);
    let e = events::list(&root).0.pop().unwrap();
    assert_eq!(
        (e.command.as_str(), e.kind, e.id.as_deref()),
        ("delete iteration", Kind::Iteration, Some("I02"))
    );
    assert!(
        e.changes
            .iter()
            .any(|c| c.field == "name" && c.to.is_none())
    );

    delete_area(&root, APP, WAIT, "A02").unwrap();
    assert_eq!(count(), start + 4);
    assert_eq!(events::list(&root).0.pop().unwrap().command, "delete area");

    // The files say the same: `alm list` reads these.
    let bag = load(&root).taskbag.unwrap();
    assert_eq!(
        bag.areas
            .iter()
            .map(|a| (a.id.as_str(), a.name.as_str()))
            .collect::<Vec<_>>(),
        [("A01", "Tracking work")]
    );
    assert_eq!(
        bag.iterations
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["I01"]
    );
    assert_eq!(bag.iterations[0].end.as_deref(), Some("2026-10-20"));
    assert!(!root.join("areas/A02.json").exists());
    assert!(!root.join("iterations/I02.json").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn deleting_a_non_empty_area_or_iteration_is_refused_and_changes_no_file() {
    let (dir, root) = taskbag("refused");
    // Only a done task in A01 / I01, only a dropped one in A02 / I02: still "in" them.
    add_task(&root, APP, WAIT, &task("A01-01", "A01", Some("I01"))).unwrap();
    add_task(&root, APP, WAIT, &task("A02-01", "A02", Some("I02"))).unwrap();
    change_task(&root, APP, WAIT, |bag| {
        ops::finish(bag, "A01-01", Some("Will"), None).map(|o| (o, "done"))
    })
    .unwrap();
    change_task(&root, APP, WAIT, |bag| {
        ops::drop_task(bag, "A02-01", Some("Will")).map(|o| (o, "drop"))
    })
    .unwrap();
    let before = snapshot(&root);

    let cases: Vec<(Result<(), Problem>, Problem)> = vec![
        (
            delete_area(&root, APP, WAIT, "A01").map(|_| ()),
            Problem::AreaNotEmpty {
                id: "A01".into(),
                count: "1".into(),
            },
        ),
        (
            delete_area(&root, APP, WAIT, "A02").map(|_| ()),
            Problem::AreaNotEmpty {
                id: "A02".into(),
                count: "1".into(),
            },
        ),
        (
            delete_iteration(&root, APP, WAIT, "I01").map(|_| ()),
            Problem::IterationNotEmpty {
                id: "I01".into(),
                count: "1".into(),
            },
        ),
        (
            delete_iteration(&root, APP, WAIT, "I02").map(|_| ()),
            Problem::IterationNotEmpty {
                id: "I02".into(),
                count: "1".into(),
            },
        ),
        (
            delete_area(&root, APP, WAIT, "A09").map(|_| ()),
            Problem::UnknownArea { area: "A09".into() },
        ),
        (
            delete_iteration(&root, APP, WAIT, "I09").map(|_| ()),
            Problem::UnknownIteration {
                iteration: "I09".into(),
            },
        ),
        (
            rename_area(&root, APP, WAIT, "A01", " ").map(|_| ()),
            Problem::AreaNameMissing,
        ),
        (
            rename_area(&root, APP, WAIT, "A09", "x").map(|_| ()),
            Problem::UnknownArea { area: "A09".into() },
        ),
        (
            change_iteration(
                &root,
                APP,
                WAIT,
                "I01",
                &IterationChange {
                    name: Some(" ".into()),
                    ..Default::default()
                },
            )
            .map(|_| ()),
            Problem::IterationNameMissing,
        ),
    ];
    for (got, want) in cases {
        assert_eq!(got, Err(want));
    }
    assert_eq!(snapshot(&root), before, "a refusal changes no file");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_bad_date_is_refused_with_the_same_code_as_alm_add_iteration() {
    let (dir, root) = taskbag("dates");
    let before = snapshot(&root);
    let added = add_iteration(
        &root,
        APP,
        WAIT,
        Some("I03"),
        "Three",
        IterationDetails {
            end: Some("31/10/2026"),
            ..Default::default()
        },
    )
    .map(|_| ());
    let changed = change_iteration(
        &root,
        APP,
        WAIT,
        "I01",
        &IterationChange {
            end: Some("31/10/2026".into()),
            ..Default::default()
        },
    )
    .map(|_| ());
    let want = Err(Problem::InvalidDate {
        value: "31/10/2026".into(),
    });
    assert_eq!(added, want);
    assert_eq!(changed, want);
    assert_eq!(snapshot(&root), before, "a refusal changes no file");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn changes_wait_for_alms_lock() {
    let (dir, root) = taskbag("lock");
    let held = Lock::acquire(&root, Duration::ZERO).unwrap();
    let short = Duration::from_millis(20);
    assert!(matches!(
        rename_area(&root, APP, short, "A01", "x"),
        Err(Problem::TaskbagBusy { .. })
    ));
    assert!(matches!(
        delete_iteration(&root, APP, short, "I02"),
        Err(Problem::TaskbagBusy { .. })
    ));
    drop(held);
    assert!(rename_area(&root, APP, short, "A01", "x").is_ok());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn the_iteration_form_arrives_as_json() {
    let c: IterationChange =
        serde_json::from_str(r#"{"name":"Two","release":"","start":"2026-10-15","end":""}"#)
            .unwrap();
    assert_eq!(c.release.as_deref(), Some(""));
    assert!(serde_json::from_str::<IterationChange>(r#"{"order":3}"#).is_err());
}
