//! Signed events, checkpoints and the audit (Task-A01-gusto404, decision 0021 § 3): removing or
//! changing an event shows where the history breaks. The key is kept in a test settings folder
//! (`ALLOMOS_SETTINGS_DIR`), never in the Keychain; the tests take turns, since the clock and the
//! folder are set for the whole process.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use allomos_core::ops::{self, Edit};
use allomos_store::{Actor, DEFAULT_DIR, Lock, change_task, checkpoint, init, trail};

static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

const APP: Actor<'static> = Actor {
    by: Some("Will"),
    via: "app",
};

fn tmp(name: &str) -> PathBuf {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!("allomos-signing-{name}-{n}"));
    fs::create_dir_all(&p).unwrap();
    p
}

/// Take a turn, with this process's settings folder (and so its key) chosen.
fn turn() -> std::sync::MutexGuard<'static, ()> {
    let guard = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let settings =
        std::env::temp_dir().join(format!("allomos-signing-settings-{}", std::process::id()));
    // SAFETY: the tests take turns ([`TURN`]); nothing else reads the environment meanwhile.
    unsafe {
        std::env::set_var("ALLOMOS_SETTINGS_DIR", &settings);
        std::env::remove_var("ALLOMOS_NOW");
    }
    guard
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

/// A new taskbag with one task and three notes on it.
fn taskbag(name: &str) -> PathBuf {
    let root = tmp(name).join(DEFAULT_DIR);
    // Its own project ID: this computer remembers the last checkpoint seen per project.
    init(&root, name, "P").unwrap();
    let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
        .unwrap();
    let bag = lock.load().taskbag.unwrap();
    lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
        .unwrap();
    drop(lock);
    for t in ["one", "two", "three"] {
        note(&root, t);
    }
    root
}

fn event_files(root: &Path) -> Vec<String> {
    trail::raw_events(root)
        .0
        .into_iter()
        .map(|(f, _)| f)
        .collect()
}

#[test]
fn every_event_is_signed_and_chained() {
    let _turn = turn();
    let root = taskbag("clean");
    let keys: Vec<_> = fs::read_dir(root.join("keys")).unwrap().collect();
    assert_eq!(keys.len(), 1);
    let (events, _) = trail::raw_events(&root);
    // The first event is `init`; the key it was signed with is added right after.
    assert_eq!(events[0].1["command"], "init");
    assert_eq!(events[1].1["command"], "add key");
    assert!(events.iter().all(|(_, e)| e.get("sig").is_some()));
    // The first event starts the chain; each next one names the one before.
    assert!(events[0].1["sig"]["prev"].is_null());
    for w in events.windows(2) {
        assert_eq!(
            w[1].1["sig"]["prev"],
            allomos_core::canonical::fingerprint(&w[0].1)
        );
    }
    // `sig` comes last, after the event's usual fields.
    let text = fs::read_to_string(root.join(&events[2].0)).unwrap();
    assert!(text.trim_start().starts_with("{\n  \"at\""), "{text}");
    let f = trail::audit(&root);
    assert_eq!(f.events, events.len());
    assert_eq!(f.signed, events.len());
    assert_eq!(f.before_signing, 0);
    assert!(f.changed.is_empty() && f.breaks.is_empty() && f.forks.is_empty());
    assert!(f.unsigned.is_empty() && f.unknown_key.is_empty());
    assert_eq!(f.chains.len(), 1);
    assert_eq!(f.chains[0].events, events.len());
    // The taskbag still loads without a note about the new files or fields.
    let loaded = allomos_store::load(&root);
    assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
}

#[test]
fn a_removed_event_shows_where_the_chain_breaks() {
    let _turn = turn();
    let root = taskbag("removed");
    let files = event_files(&root);
    // init, add key, then the three notes.
    let (gone, next) = (&files[3], &files[4]);
    fs::remove_file(root.join(gone)).unwrap();
    let f = trail::audit(&root);
    assert!(f.changed.is_empty());
    assert_eq!(f.breaks.len(), 1, "{:?}", f.breaks);
    assert_eq!(&f.breaks[0].file, next);
    assert_eq!(f.breaks[0].after.as_ref(), Some(&files[2]));
    assert!(!f.breaks[0].after_changed);
}

#[test]
fn a_changed_event_is_found() {
    let _turn = turn();
    let root = taskbag("changed");
    let files = event_files(&root);
    let path = root.join(&files[2]);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("\"one\""), "{text}");
    fs::write(&path, text.replace("\"one\"", "\"uno\"")).unwrap();
    let f = trail::audit(&root);
    assert_eq!(f.changed, [files[2].clone()]);
    // Its fingerprint changed too, so the next event's link no longer meets it.
    assert_eq!(f.breaks.len(), 1);
    assert_eq!(f.breaks[0].file, files[3]);
    assert!(f.breaks[0].after_changed);
    // Re-indenting changes nothing: the signature is over the canonical form.
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
    let f = trail::audit(&root);
    assert!(f.changed.is_empty() && f.breaks.is_empty());
}

#[test]
fn checkpoints_find_a_removed_last_event_and_a_rewritten_history() {
    let _turn = turn();
    let root = taskbag("checkpoint");
    let made = checkpoint::checkpoint(&root, Duration::ZERO, Some("abc123")).unwrap();
    assert_eq!(made.heads, 1);
    let c: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join(&made.file)).unwrap()).unwrap();
    assert_eq!(c["git_commit"], "abc123");
    let f = trail::audit(&root);
    assert_eq!(f.checkpoints.len(), 1);
    assert!(f.checkpoints[0].signed);
    assert_eq!(f.changed_since_checkpoint, Some(false));
    assert!(f.rewritten.is_none());

    // The chain's last event is gone: no later event says so, the checkpoint does.
    let files = event_files(&root);
    let last = files.last().unwrap().clone();
    fs::remove_file(root.join(&last)).unwrap();
    let f = trail::audit(&root);
    assert!(f.breaks.is_empty());
    assert_eq!(f.checkpoints[0].heads_missing, [last]);
    assert_eq!(f.changed_since_checkpoint, Some(true));

    // A changed checkpoint no longer matches its signature.
    let path = root.join(&made.file);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("abc123", "def456")).unwrap();
    assert!(!trail::audit(&root).checkpoints[0].signed);

    // The checkpoint this computer saw is gone: the history was rewritten since.
    fs::remove_file(&path).unwrap();
    let f = trail::audit(&root);
    let (file, _) = f.rewritten.expect("rewritten");
    assert_eq!(file, made.file);
    // And it stays said.
    assert!(trail::audit(&root).rewritten.is_some());
}

#[test]
fn packing_a_month_keeps_the_signatures() {
    let _turn = turn();
    // SAFETY: as in `turn`.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-09-10T09:00:00Z") };
    let root = taskbag("packed");
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-10-28T09:00:00Z") };
    note(&root, "October");
    let packed = Lock::acquire(&root, Duration::from_secs(5))
        .unwrap()
        .pack_history()
        .unwrap();
    assert_eq!(packed.len(), 1);
    unsafe { std::env::remove_var("ALLOMOS_NOW") };
    let f = trail::audit(&root);
    assert!(f.events > 5);
    assert!(f.changed.is_empty(), "{:?}", f.changed);
    assert!(f.breaks.is_empty(), "{:?}", f.breaks);
}

#[test]
fn upgrading_from_allomos_7_starts_signing_and_fixes_what_came_before() {
    let _turn = turn();
    let root = taskbag("upgrade");
    // Make it an `allomos/7` taskbag with an unsigned history.
    for (f, _) in trail::raw_events(&root).0 {
        fs::remove_file(root.join(f)).unwrap();
    }
    fs::remove_dir_all(root.join("keys")).unwrap();
    let project = root.join("project.json");
    let text = fs::read_to_string(&project).unwrap();
    fs::write(&project, text.replace("allomos/8", "allomos/7")).unwrap();
    // SAFETY: as in `turn`. Without a key store, `allomos/7` events are unsigned anyway.
    unsafe { std::env::remove_var("ALLOMOS_SETTINGS_DIR") };
    note(&root, "before");
    note(&root, "before again");
    turn_again();
    let up = allomos_store::upgrade(&root, APP, Duration::from_secs(5))
        .unwrap()
        .unwrap();
    assert_eq!(
        (up.from.as_str(), up.to.as_str()),
        ("allomos/7", "allomos/8")
    );
    assert_eq!(up.tasks, 0, "labels stay as they are");
    let checkpoint = up.checkpoint.expect("a checkpoint after the upgrade");
    note(&root, "after");
    let f = trail::audit(&root);
    assert_eq!(f.before_signing, 2);
    assert!(f.unsigned.is_empty());
    assert!(f.changed.is_empty() && f.breaks.is_empty());
    assert_eq!(f.checkpoints[0].file, checkpoint);
    // The first signed event is the upgrade, then the key it was signed with.
    let (events, _) = trail::raw_events(&root);
    let signed: Vec<_> = events
        .iter()
        .filter(|(_, e)| e.get("sig").is_some())
        .collect();
    assert_eq!(signed[0].1["command"], "upgrade");
    assert_eq!(signed[1].1["command"], "add key");
}

/// Choose the settings folder again (inside a turn already held).
fn turn_again() {
    let settings =
        std::env::temp_dir().join(format!("allomos-signing-settings-{}", std::process::id()));
    // SAFETY: as in `turn`.
    unsafe { std::env::set_var("ALLOMOS_SETTINGS_DIR", &settings) };
}
