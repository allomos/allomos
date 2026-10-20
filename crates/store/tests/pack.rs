//! Packing the history of earlier months (Task-A01-59). The clock is set for the whole process
//! (`ALLOMOS_NOW`), so the tests take turns ([`CLOCK`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use allomos_core::ops::{self, Edit};
use allomos_store::{Actor, DEFAULT_DIR, Lock, change_task, events, init};

/// Held by each test while it sets the clock.
static CLOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

const APP: Actor<'static> = Actor {
    by: Some("Will"),
    via: "app",
};

fn tmp(name: &str) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("allomos-pack-{name}-{n}"));
    fs::create_dir_all(&p).unwrap();
    p
}

fn files(root: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(root.join("events"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();
    v.sort();
    v
}

fn note(root: &Path, text: &str) {
    change_task(root, APP, Duration::from_secs(5), |bag| {
        ops::edit(
            bag,
            "A01-01",
            APP.by,
            &Edit::HandoffNote {
                text: Some(text.into()),
            },
        )
    })
    .unwrap();
}

fn pack(root: &Path) -> Vec<events::Packed> {
    Lock::acquire(root, Duration::from_secs(5))
        .unwrap()
        .pack_history()
        .unwrap()
}

/// A taskbag with history in August and September 2026, as of `now`.
fn with_history(name: &str) -> PathBuf {
    let root = tmp(name).join(DEFAULT_DIR);
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-08-10T09:00:00Z") };
    init(&root, "p", "P").unwrap();
    let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
        .unwrap();
    drop(lock);
    note(&root, "August");
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-09-15T09:00:00Z") };
    note(&root, "September one");
    note(&root, "September two");
    root
}

#[test]
fn earlier_months_are_packed_from_the_28th_and_read_as_before() {
    let _clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = with_history("one");
    // October 27th: too early, nothing packed.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-10-27T09:00:00Z") };
    note(&root, "October");
    let before = events::list_with_files(&root);
    let history = events::history(&root, "A01-01");
    assert!(pack(&root).is_empty());
    let singles = files(&root);
    assert!(singles.iter().all(|f| f.ends_with(".json")));

    // October 28th: August and September are packed; October stays.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-10-28T09:00:00Z") };
    let packed = pack(&root);
    let months: Vec<&str> = packed.iter().map(|p| p.month.as_str()).collect();
    assert_eq!(months, ["2026-08", "2026-09"]);
    assert_eq!(packed[1].changes, 2);
    assert_eq!(packed[1].file, "events/2026-09.events");
    let after = files(&root);
    assert!(after.contains(&"2026-08.events".to_string()));
    assert!(after.contains(&"2026-09.events".to_string()));
    assert!(
        after
            .iter()
            .filter(|f| f.ends_with(".json"))
            .all(|f| f.starts_with("2026-10"))
    );
    // Read back: the same events, under the same names, and the same task history.
    assert_eq!(events::list_with_files(&root), before);
    assert_eq!(events::history(&root, "A01-01"), history);

    // Packing again changes nothing.
    let pack_text = fs::read_to_string(root.join("events/2026-09.events")).unwrap();
    assert!(pack(&root).is_empty());
    assert_eq!(
        fs::read_to_string(root.join("events/2026-09.events")).unwrap(),
        pack_text
    );

    // An older Allomos reads only `.json` files there: it skips the packed months, no error.
    let old_reader: Vec<String> = files(&root)
        .into_iter()
        .filter(|f| f.ends_with(".json"))
        .collect();
    assert!(old_reader.iter().all(|f| f.starts_with("2026-10")));
    for f in &old_reader {
        let text = fs::read_to_string(root.join("events").join(f)).unwrap();
        assert!(allomos_core::parse_event(f, &text).is_ok());
    }

    // A single file of a packed month that arrives later (say, merged from another computer) is
    // taken into that month's file on the next packing.
    let late = root.join("events/2026-09-20-120000-000-latexx.json");
    let mut e = before.0.last().unwrap().1.clone();
    e.at = "2026-09-20T12:00:00Z".into();
    fs::write(&late, serde_json::to_string_pretty(&e).unwrap()).unwrap();
    let again = pack(&root);
    assert_eq!(again.len(), 1);
    assert_eq!((again[0].changes, again[0].removed), (3, 1));
    assert!(!late.exists());
    assert_eq!(events::list(&root).0.len(), before.0.len() + 1);
}

#[test]
fn two_computers_packing_the_same_month_write_the_same_file() {
    let _clock = CLOCK.lock().unwrap_or_else(|e| e.into_inner());
    let a = with_history("a");
    // The second computer has the same history: a copy of the taskbag.
    let b = tmp("b").join(DEFAULT_DIR);
    fs::create_dir_all(b.join("events")).unwrap();
    for entry in fs::read_dir(&a).unwrap() {
        let p = entry.unwrap().path();
        let to = b.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &to);
        } else {
            fs::copy(&p, &to).unwrap();
        }
    }
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-10-29T09:00:00Z") };
    pack(&a);
    pack(&b);
    for month in ["2026-08.events", "2026-09.events"] {
        assert_eq!(
            fs::read(a.join("events").join(month)).unwrap(),
            fs::read(b.join("events").join(month)).unwrap(),
            "{month}"
        );
    }
}

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let p = entry.unwrap().path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_dir(&p, &dest);
        } else {
            fs::copy(&p, &dest).unwrap();
        }
    }
}
