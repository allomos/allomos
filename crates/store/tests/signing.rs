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
    let (made, witnessed) = checkpoint::checkpoint(&root, Duration::ZERO, Some("abc123")).unwrap();
    assert_eq!(made.heads, 1);
    // The outside witness is off: nothing sent, nothing waiting.
    assert_eq!(witnessed, checkpoint::Witnessed::default());
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

/// The outside witness (Task-A01-wood982): a checkpoint names its service, waits while the
/// service cannot be reached, gets its receipt later, and `alm audit` matches the receipt to the
/// checkpoint's fingerprint. The service is a recorded reply (tests/fixtures/tsa-reply.der).
#[test]
fn the_outside_witness_countersigns_checkpoints_and_the_audit_matches_them() {
    let _turn = turn();
    let root = taskbag("witness");
    let project = root.join("project.json");
    let mut p: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&project).unwrap()).unwrap();
    p["audit"] = serde_json::json!({ "witness": { "service": "https://tsa.test/tsr" } });
    fs::write(&project, serde_json::to_string_pretty(&p).unwrap()).unwrap();

    // Offline: the checkpoint is made and signed, and waits.
    // SAFETY: as in `turn`.
    unsafe { std::env::set_var("ALLOMOS_TSA_REPLY", "offline") };
    let (made, witnessed) = checkpoint::checkpoint(&root, Duration::ZERO, None).unwrap();
    assert!(witnessed.receipts.is_empty());
    assert_eq!(witnessed.waiting.len(), 1);
    assert_eq!(witnessed.waiting[0].service, "https://tsa.test/tsr");
    let f = trail::audit(&root);
    assert!(f.checkpoints[0].signed);
    assert_eq!(f.checkpoints[0].waiting, ["https://tsa.test/tsr"]);
    assert_eq!(
        f.receipts_missing,
        [format!("{}: https://tsa.test/tsr", made.file)]
    );

    // A reply for another fingerprint is not kept: it still waits, and says why.
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/tsa-reply.der");
    unsafe { std::env::set_var("ALLOMOS_TSA_REPLY", fixture) };
    let w = checkpoint::send_waiting(&root, Duration::ZERO).unwrap();
    assert!(w.receipts.is_empty());
    assert!(
        w.waiting[0]
            .why
            .as_deref()
            .unwrap()
            .contains("different fingerprint")
    );

    // A receipt added to a checkpoint leaves its signature whole.
    let path = root.join(&made.file);
    let mut c: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    c["receipts"] = serde_json::json!([{ "service": "https://other.test/", "token": "AAAA" }]);
    fs::write(&path, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let f = trail::audit(&root);
    assert!(f.checkpoints[0].signed);
    assert!(!f.checkpoints[0].witnessed[0].readable);
    assert_eq!(
        f.receipts_not_matching,
        [format!("{}: https://other.test/", made.file)]
    );

    // The fingerprint the recorded reply countersigns: the receipt comes back and matches.
    c["sha256"] = serde_json::json!(allomos_store::tsa::FIXTURE_SHA256);
    c["receipts"] = serde_json::json!([]);
    fs::write(&path, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let w = checkpoint::send_waiting(&root, Duration::ZERO).unwrap();
    assert_eq!(w.receipts.len(), 1, "{:?}", w.waiting);
    assert!(w.waiting.is_empty());
    assert!(checkpoint::waiting(&root).is_empty());
    let f = trail::audit(&root);
    let r = &f.checkpoints[0].witnessed[0];
    assert!(r.readable && r.matches && r.whole, "{r:?}");
    assert_eq!(
        r.signer.as_deref(),
        Some("O=Allomos test, CN=Allomos test TSA")
    );
    assert!(!r.expires_within_a_year);
    assert!(f.receipts_missing.is_empty() && f.receipts_not_matching.is_empty());
    assert!(f.receipt_signature_check.is_some());

    // Turned off: a waiting fingerprint is no longer sent.
    let off = serde_json::json!({ "format": p["format"], "id": p["id"], "name": p["name"] });
    fs::write(&project, serde_json::to_string_pretty(&off).unwrap()).unwrap();
    let mut c: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    c["receipts"] = serde_json::json!([]);
    fs::write(&path, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    assert_eq!(
        checkpoint::send_waiting(&root, Duration::ZERO).unwrap(),
        checkpoint::Witnessed::default()
    );
    assert_eq!(checkpoint::waiting(&root).len(), 1);
    fs::write(&project, serde_json::to_string_pretty(&p).unwrap()).unwrap();
    checkpoint::send_waiting(&root, Duration::ZERO).unwrap();

    // The checkpoint's fingerprint changed after the receipt: they no longer match.
    let mut c: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    c["sha256"] = serde_json::json!("00".repeat(32));
    fs::write(&path, serde_json::to_string_pretty(&c).unwrap()).unwrap();
    let f = trail::audit(&root);
    assert!(!f.checkpoints[0].witnessed[0].matches);
    assert_eq!(f.receipts_not_matching.len(), 1);
    unsafe { std::env::remove_var("ALLOMOS_TSA_REPLY") };
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

/// Task-A01-stem986: removing personal details from a signed history. The redacted events no
/// longer match their signatures; `alm audit` lists them as redacted, from the fingerprints the
/// redaction recorded, not as changed or as a gap. A packed month's events are redacted in place.
#[test]
fn a_redaction_is_listed_not_found_as_tampering() {
    use allomos_store::redact::{self, Request};
    let _turn = turn();
    // SAFETY: as in `turn`.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-09-10T09:00:00Z") };
    let root = taskbag("redacted");
    note(&root, "call <PHONE>");
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-10-28T09:00:00Z") };
    let packed = Lock::acquire(&root, Duration::from_secs(5))
        .unwrap()
        .pack_history()
        .unwrap();
    assert_eq!(packed.len(), 1);
    note(&root, "fixed");
    unsafe { std::env::remove_var("ALLOMOS_NOW") };
    let fields = vec!["handoff_note".to_string()];
    let r = redact::redact(
        &root,
        APP,
        Duration::from_secs(5),
        "A01-01",
        &Request {
            fields: &fields,
            events: &[],
            reason: Some("a phone number written by mistake"),
            approved_by: Some("approver"),
        },
    )
    .unwrap();
    let rd = r.redaction.expect("something was removed");
    // one, two, three, the phone number (all packed), and the `from` of "fixed".
    assert_eq!(rd.files.len(), 5, "{:?}", rd.files);
    let all = fs::read_dir(root.join("events"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect::<String>();
    assert!(!all.contains("PHONE"), "the old value is gone");
    assert!(all.contains("\"fixed\""), "the current value stays");
    assert!(all.contains("a phone number written by mistake"));
    let f = trail::audit(&root);
    assert!(f.changed.is_empty(), "{:?}", f.changed);
    assert!(f.breaks.is_empty(), "{:?}", f.breaks);
    assert_eq!(f.redacted.len(), 5);
    assert!(f.redacted.iter().all(|x| x.redaction == rd.id));
    // The taskbag still loads, and a second time removes nothing.
    assert!(allomos_store::load(&root).issues.is_empty());
    let again = redact::redact(
        &root,
        APP,
        Duration::from_secs(5),
        "A01-01",
        &Request {
            fields: &fields,
            events: &[],
            reason: Some("again"),
            approved_by: None,
        },
    )
    .unwrap();
    assert!(again.redaction.is_none());
    // A real change after the redaction is still found: here, to the redaction's own event.
    let by = f.redacted[0].by_event.clone();
    let text = fs::read_to_string(root.join(&by)).unwrap();
    fs::write(root.join(&by), text.replace("by mistake", "on purpose")).unwrap();
    assert_eq!(trail::audit(&root).changed, [by]);
}

/// Settings → This computer (Task-A01-tail719): the key's ID and public key, matching the one in
/// the taskbag's `keys/`, the public file to export, and the last checkpoint seen per project.
#[test]
fn this_computer_shows_its_key_and_the_checkpoints_seen() {
    let _turn = turn();
    let root = taskbag("this-computer");
    let this = allomos_store::this_computer();
    assert_eq!(this.store, "folder");
    let key = this.key.expect("the taskbag's changes made a key");
    assert!(root.join("keys").join(format!("{}.json", key.id)).is_file());
    let export: serde_json::Value =
        serde_json::from_str(&allomos_store::public_key_export(&key)).unwrap();
    assert_eq!(export["public_key"], serde_json::json!(key.public_key));
    assert!(export.get("owner").is_none());
    let (made, _) = checkpoint::checkpoint(&root, Duration::from_secs(5), None).unwrap();
    let _ = trail::audit(&root);
    let project = fs::read_to_string(root.join("project.json")).unwrap();
    let project: serde_json::Value = serde_json::from_str(&project).unwrap();
    let seen = checkpoint::seen();
    let mine = seen
        .iter()
        .find(|s| serde_json::json!(s.project) == project["id"])
        .expect("audit remembered the checkpoint");
    assert_eq!(mine.checkpoint, made.file);
}

/// Replace key… (Task-A01-ring220): the new key is this computer's; each taskbag given that has
/// the old key gets a `retire key` event signed by the old key naming the new one, the old key
/// file kept and marked retired, and the new key's `add key`. The old key's events still check,
/// and later changes are signed with the new key. A taskbag without the old key is left alone.
#[test]
fn replacing_the_key_retires_the_old_one_and_keeps_its_events_checkable() {
    let _turn = turn();
    let root = taskbag("replace-key");
    let old = allomos_store::this_computer().key.expect("a key").id;
    let elsewhere = tmp("replace-key-not-a-taskbag");
    let done = allomos_store::replace_key(
        &[root.clone(), elsewhere.clone()],
        APP,
        Duration::from_secs(5),
    )
    .unwrap();
    assert_eq!(done.old, old);
    assert_ne!(done.new.id, old);
    assert_eq!(
        allomos_store::this_computer().key.map(|k| k.id),
        Some(done.new.id.clone())
    );
    let outcomes: Vec<_> = done.taskbags.iter().map(|t| t.outcome).collect();
    assert_eq!(outcomes, ["replaced", "not_here"]);

    let old_file: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("keys").join(format!("{old}.json"))).unwrap(),
    )
    .unwrap();
    assert!(old_file["retired"].is_string());
    assert!(
        root.join("keys")
            .join(format!("{}.json", done.new.id))
            .is_file()
    );

    note(&root, "after the new key");
    let (events, _) = trail::raw_events(&root);
    let retire = events
        .iter()
        .find(|(_, e)| e["command"] == "retire key")
        .expect("a retire key event");
    assert_eq!(retire.1["sig"]["key"], serde_json::json!(old));
    assert_eq!(retire.1["replaced_by"], serde_json::json!(done.new.id));
    // Who made it, as for any change (the person's pseudonym).
    assert!(retire.1["by"].is_string());
    let last = &events.last().unwrap().1;
    assert_eq!(last["sig"]["key"], serde_json::json!(done.new.id));

    let f = trail::audit(&root);
    assert!(f.changed.is_empty(), "{:?}", f.changed);
    assert!(f.unknown_key.is_empty() && f.breaks.is_empty() && f.forks.is_empty());
    assert!(f.unsigned.is_empty());
    let keys: Vec<_> = f.chains.iter().map(|c| c.key.clone()).collect();
    assert!(keys.contains(&old) && keys.contains(&done.new.id));

    // Replacing again retires the second key the same way; three chains, none broken.
    let again = allomos_store::replace_key(&[root.clone()], APP, Duration::from_secs(5)).unwrap();
    assert_eq!(
        (again.old.as_str(), again.taskbags[0].outcome),
        (done.new.id.as_str(), "replaced")
    );
    let f = trail::audit(&root);
    assert_eq!(f.chains.len(), 3);
    assert!(f.breaks.is_empty() && f.changed.is_empty());
}

/// The outside witness's "how often" (Task-A01-bake791), as the app's clock and `alm` ask: daily
/// makes one when the last is a day old, two asking at once make one between them, offline the
/// fingerprint waits; requests to one service stay over a second apart, across calls too;
/// approvals only at an approval; asked and off, nothing.
#[test]
fn how_often_makes_checkpoints_by_themselves() {
    use allomos_store::cadence::{self, Trigger};
    let _turn = turn();
    let root = taskbag("cadence");
    let project = root.join("project.json");
    let set_when = |when: Option<&str>| {
        let mut p: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&project).unwrap()).unwrap();
        p["audit"] = match when {
            Some(w) => {
                serde_json::json!({ "witness": { "service": "https://tsa.test/tsr", "when": w } })
            }
            None => serde_json::Value::Null,
        };
        fs::write(&project, serde_json::to_string_pretty(&p).unwrap()).unwrap();
    };
    let count = || checkpoint::list(&root).0.len();
    // SAFETY: as in `turn`.
    unsafe {
        std::env::set_var("ALLOMOS_TSA_REPLY", "offline");
        std::env::set_var("ALLOMOS_NOW", "2026-11-01T09:00:00Z");
    }
    set_when(Some("daily"));

    // None yet: two asking at once (the app and `alm`) make one.
    let a = std::thread::scope(|s| {
        let one = s.spawn(|| cadence::auto(&root, Duration::from_secs(5), Trigger::Change));
        let two = s.spawn(|| cadence::auto(&root, Duration::from_secs(5), Trigger::Change));
        [one.join().unwrap().unwrap(), two.join().unwrap().unwrap()]
    });
    assert_eq!(count(), 1);
    assert_eq!(
        a.iter()
            .filter(|x| x.as_ref().is_some_and(|x| x.made.is_some()))
            .count(),
        1
    );
    // Offline: made here, its fingerprint waiting.
    assert_eq!(checkpoint::waiting(&root).len(), 1);

    // Later the same day (the test clock): none.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-11-01T23:59:00Z") };
    let a = cadence::auto(&root, Duration::ZERO, Trigger::Change).unwrap();
    assert!(a.is_none_or(|a| a.made.is_none()));
    assert_eq!(count(), 1);
    // A day after: one.
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-11-02T09:00:00Z") };
    let a = cadence::auto(&root, Duration::ZERO, Trigger::Change)
        .unwrap()
        .unwrap();
    assert!(a.made.is_some());
    assert_eq!(a.when, "daily");
    assert_eq!(count(), 2);

    // At approvals: a change makes none; two approvals one after the other make two, their
    // requests to the service more than a second apart.
    set_when(Some("approvals"));
    assert!(
        cadence::auto(&root, Duration::ZERO, Trigger::Change)
            .unwrap()
            .is_none()
    );
    cadence::auto(&root, Duration::ZERO, Trigger::Approval)
        .unwrap()
        .unwrap();
    let started = std::time::Instant::now();
    cadence::auto(&root, Duration::ZERO, Trigger::Approval)
        .unwrap()
        .unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(1000),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(count(), 4);

    // Only when asked, or off: nothing made, nothing sent.
    set_when(Some("asked"));
    unsafe { std::env::set_var("ALLOMOS_NOW", "2026-11-09T09:00:00Z") };
    for t in [Trigger::Change, Trigger::Approval] {
        assert!(cadence::auto(&root, Duration::ZERO, t).unwrap().is_none());
    }
    set_when(None);
    for t in [Trigger::Change, Trigger::Approval] {
        assert!(cadence::auto(&root, Duration::ZERO, t).unwrap().is_none());
    }
    assert_eq!(count(), 4);
    unsafe {
        std::env::remove_var("ALLOMOS_TSA_REPLY");
        std::env::remove_var("ALLOMOS_NOW");
    }
}
