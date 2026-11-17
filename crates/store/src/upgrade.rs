//! Upgrading an older taskbag to the format this Allomos writes.

use super::*;

/// What [`upgrade`] did: the format before and after, the main folder's new name, and how the
/// labels moved (decision 0020).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Upgraded {
    pub from: String,
    pub to: String,
    pub folder: Option<String>,
    /// Tasks whose labels changed.
    pub tasks: usize,
    /// How many tasks carry each label after the upgrade, by label.
    pub labels: std::collections::BTreeMap<String, usize>,
    /// Tasks that lost a dropped label (`drafted`, `implemented`).
    pub dropped: usize,
    /// Tasks that were in the old `blocked` state, each with the state it has now (decision
    /// 0026): the one it had before it was blocked, from its history.
    pub unblocked: Vec<(String, String)>,
    /// The checkpoint made right after the upgrade (`checkpoints/<name>.json`), which fixes the
    /// history recorded before signing; `None` without a signing key.
    pub checkpoint: Option<String>,
}

/// Upgrade an older taskbag to `allomos/9`, which only a person asks for (approver,
/// 2026-10-25). From `allomos/5`: `project_folder` becomes the first of `project_folders`, named
/// after the folder (decision 0019). From `allomos/5` or `allomos/6`: the ladder becomes the label
/// table and each task's `label` its `labels` (decision 0020, In detail 8): `drafted` and
/// `implemented` are dropped, `locally verified` and `deployed and verified` become "… by agent",
/// `human verified` (or `owner verified`) becomes `deployed and verified by human` when the task's
/// history shows `deployed and verified` before it, else `locally verified by human`, and a task
/// whose history has unreviewed AI decisions gets `AI decisions to review`. From anything before
/// `allomos/9` (decision 0026): a task in the old `blocked` state gets back the state it had
/// before it was blocked ([`state_before_blocked`]); its blockers stay. `None` when it is already
/// up to date.
pub fn upgrade(root: &Path, actor: Actor, wait: Duration) -> Result<Option<Upgraded>, Problem> {
    versions::write_allowed(root)?;
    let lock = Lock::acquire(root, wait)?;
    let (bag, _) = taskbag_of(root, lock.load())?;
    if bag.project.current() {
        return Ok(None);
    }
    let base = setup::project_folder_of(root, &bag.project)
        .and_then(|f| f.file_name().map(|n| n.to_string_lossy().into_owned()));
    let mut project = allomos_core::folders::upgraded(&bag.project, base.as_deref())
        .unwrap_or(bag.project.clone());
    project.format = FORMAT.to_string();
    let (history, _) = events::list(root);
    let deployed: std::collections::BTreeSet<String> = history
        .iter()
        .filter(|e| e.kind == Kind::Task)
        .filter(|e| {
            e.changes.iter().any(|c| {
                matches!(c.field.as_str(), "label" | "labels")
                    && c.to.as_ref().is_some_and(|v| {
                        v.as_str() == Some("deployed and verified")
                            || v.as_array().is_some_and(|a| {
                                a.iter()
                                    .any(|x| x.as_str() == Some("deployed and verified"))
                            })
                    })
            })
        })
        .filter_map(|e| e.id.clone())
        .collect();
    let unreviewed = decision::pending(&history);
    let mut changed = Vec::new();
    let mut dropped = 0;
    // From `allomos/7` on, labels already have their shape: only signing starts.
    let labels_done = allomos_core::format_version(&bag.project.format)
        .is_some_and(|n| n >= allomos_core::LABELS_FROM);
    for task in bag.tasks.iter().filter(|_| !labels_done) {
        let path = root.join("tasks").join(&task.id).join("task.json");
        let old = fs::read_to_string(&path)
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").and_then(|l| l.as_str()).map(String::from));
        if old
            .as_deref()
            .is_some_and(|o| allomos_core::labels::upgraded_label(o, false).is_none())
        {
            dropped += 1;
        }
        let labels = allomos_core::labels::upgraded_labels(
            old.as_deref(),
            deployed.contains(&task.id),
            unreviewed
                .get(&task.id)
                .is_some_and(|d| d.iter().any(|x| x.to_review())),
        );
        let mut next = task.clone();
        next.labels = labels;
        allomos_core::labels::add_found(&mut project, &next.labels);
        changed.push(next);
    }
    // Decision 0026: `blocked` is no longer a state. Each task still in it gets back the state it
    // had before it was blocked, with what it waited on then.
    let mut unblocked = Vec::new();
    let mut restored: std::collections::BTreeMap<String, Vec<allomos_core::event::Change>> =
        Default::default();
    for task in bag.tasks.iter().filter(|t| t.old_blocked()) {
        let mut next = match changed.iter().position(|c| c.id == task.id) {
            Some(i) => changed.remove(i),
            None => task.clone(),
        };
        let was = state_before_blocked(&history, &next);
        let text = |v: &Option<String>| v.clone().map(serde_json::Value::String);
        let mut changes = vec![allomos_core::event::Change {
            extra: Default::default(),
            field: "state".into(),
            from: Some(serde_json::json!("blocked")),
            to: Some(serde_json::json!(was.state.as_str())),
        }];
        for (field, from, to) in [
            ("waiting_on", &next.waiting_on, &was.waiting_on),
            ("who", &next.who, &was.who),
        ] {
            if from != to {
                changes.push(allomos_core::event::Change {
                    extra: Default::default(),
                    field: field.into(),
                    from: text(from),
                    to: text(to),
                });
            }
        }
        changes.push(event::reason_change(if was.found {
            "upgrade: blocked is no longer a state (decision 0026); this is the state it had before it was blocked, from its history"
        } else {
            "upgrade: blocked is no longer a state (decision 0026); its history does not say what it was before, so it is ready (in progress when held)"
        }));
        next.extra.remove("state");
        next.state = was.state;
        next.waiting_on = was.waiting_on;
        next.who = was.who;
        unblocked.push((next.id.clone(), next.state.as_str().to_string()));
        restored.insert(next.id.clone(), changes);
        changed.push(next);
    }
    let before = Project {
        format: bag.project.format.clone(),
        ..bag.project.clone()
    };
    let event = actor.event(
        "upgrade",
        Kind::Project,
        Some(&project.id),
        event::changes(Some(&before), &project, &[]),
    );
    lock.record(&event, |l| l.save_project(&project))?;
    let mut tasks = 0;
    let mut counts = std::collections::BTreeMap::new();
    for next in &changed {
        for l in &next.labels {
            *counts.entry(l.clone()).or_insert(0) += 1;
        }
        // Every file is rewritten in the new shape, so the old `label` field goes.
        let raw_label = fs::read_to_string(root.join("tasks").join(&next.id).join("task.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|v| v.get("label").cloned())
            .filter(|v| !v.is_null());
        let to = (!next.labels.is_empty()).then(|| serde_json::json!(next.labels));
        let mut changes = Vec::new();
        if !labels_done && (raw_label.is_some() || to.is_some()) {
            changes.push(allomos_core::event::Change {
                extra: Default::default(),
                field: "labels".into(),
                from: raw_label,
                to,
            });
        }
        if !changes.is_empty() {
            tasks += 1;
        }
        changes.extend(restored.remove(&next.id).unwrap_or_default());
        let event = actor.event("upgrade", Kind::Task, Some(&next.id), changes);
        lock.record(&event, |l| l.save_task(next))?;
    }
    if labels_done {
        counts.clear();
        for task in &bag.tasks {
            for l in &task.labels {
                *counts.entry(l.clone()).or_insert(0) += 1;
            }
        }
    }
    // The `upgrade` event is the first signed one; a checkpoint fixes everything written before
    // it (decision 0021 § 4, events before signing: option A). Without a key store, none.
    let checkpoint = checkpoint::make(root, None).ok().map(|m| m.file);
    Ok(Some(Upgraded {
        from: bag.project.format.clone(),
        to: project.format.clone(),
        folder: project.project_folders.first().map(|f| f.name.clone()),
        tasks,
        labels: counts,
        dropped,
        unblocked,
        checkpoint,
    }))
}

/// What a task in the old `blocked` state goes back to on the upgrade to `allomos/9`.
struct BeforeBlocked {
    state: allomos_core::State,
    waiting_on: Option<String>,
    who: Option<String>,
    /// Its history said which state it had; otherwise the fallback.
    found: bool,
}

/// The state `task` had before it was blocked (decision 0026, detail l): from the last change in
/// its history that made it `blocked`, the state before that change, with what it waited on then.
/// A state that no longer fits is corrected: in progress needs a holder and a holder means in
/// progress, waiting needs what it waits on. With nothing in the history: in progress when
/// someone holds it, else ready.
fn state_before_blocked(history: &[allomos_core::event::Event], task: &Task) -> BeforeBlocked {
    use allomos_core::State;
    let held = task.holder.is_some();
    let fallback = if held {
        State::InProgress
    } else {
        State::Ready
    };
    let text = |v: &Option<serde_json::Value>| {
        v.as_ref()
            .and_then(|v| v.as_str())
            .filter(|t| !t.trim().is_empty())
            .map(String::from)
    };
    let found = history
        .iter()
        .filter(|e| e.kind == Kind::Task && e.id.as_deref() == Some(task.id.as_str()))
        .filter(|e| {
            e.changes.iter().any(|c| {
                c.field == "state" && c.to.as_ref().and_then(|v| v.as_str()) == Some("blocked")
            })
        })
        .max_by(|a, b| a.at.cmp(&b.at));
    let Some(event) = found else {
        return BeforeBlocked {
            state: fallback,
            waiting_on: None,
            who: None,
            found: false,
        };
    };
    let from = |field: &str| {
        event
            .changes
            .iter()
            .find(|c| c.field == field)
            .and_then(|c| text(&c.from))
    };
    let waiting_on = from("waiting_on");
    let state = match from("state").as_deref() {
        Some("backlog") if !held => Some(State::Backlog),
        Some("ready") if !held => Some(State::Ready),
        Some("waiting") if waiting_on.is_some() => Some(State::Waiting),
        Some("in_progress") if held => Some(State::InProgress),
        _ => None,
    };
    match state {
        Some(State::Waiting) => BeforeBlocked {
            state: State::Waiting,
            waiting_on,
            who: from("who"),
            found: true,
        },
        Some(state) => BeforeBlocked {
            state,
            waiting_on: None,
            who: None,
            found: true,
        },
        None => BeforeBlocked {
            state: fallback,
            waiting_on: None,
            who: None,
            found: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::ops;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// Decision 0026, detail l: the upgrade to `allomos/9` gives every task in the old `blocked`
    /// state the state it had before it was blocked, from its history; with nothing there, ready
    /// (in progress when held). Blockers stay.
    #[test]
    fn the_upgrade_gives_blocked_tasks_their_earlier_state() {
        let dir = tmp("upgrade-blocked");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().taskbag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let ids = ["A01-01", "A01-02", "A01-03", "A01-04", "A01-05", "A01-06"];
        for id in ids {
            let bag = lock.load().taskbag.unwrap();
            lock.create_task(&ops::new_task(&bag, id, "A01", id, None).unwrap())
                .unwrap();
        }
        // As an `allomos/8` taskbag had them: `blocked` as a state, with what the history said.
        // 02: backlog before. 03: waiting on Ann before. 04: no history. 05: held, no history.
        // 06: ready before, blocked twice (the later change counts).
        let actor = Actor::alm(Some("s"));
        let change_of =
            |field: &str, from: Option<&str>, to: Option<&str>| allomos_core::event::Change {
                extra: Default::default(),
                field: field.into(),
                from: from.map(|v| serde_json::json!(v)),
                to: to.map(|v| serde_json::json!(v)),
            };
        let mut bag = lock.load().taskbag.unwrap();
        for t in bag.tasks.iter_mut().filter(|t| t.id != "A01-01") {
            let changes = match t.id.as_str() {
                "A01-02" => vec![change_of("state", Some("backlog"), Some("blocked"))],
                "A01-03" => vec![
                    change_of("state", Some("waiting"), Some("blocked")),
                    change_of("waiting_on", Some("the licence"), None),
                    change_of("who", Some("Ann"), None),
                ],
                "A01-06" => vec![change_of("state", Some("ready"), Some("blocked"))],
                _ => vec![],
            };
            if t.id == "A01-05" {
                t.holder = Some("b".into());
            }
            t.state = allomos_core::State::Unknown;
            t.extra.insert("state".into(), serde_json::json!("blocked"));
            t.blocked_by = vec!["A01-01".into()];
            if changes.is_empty() {
                lock.save_task(t).unwrap();
            } else {
                let event = actor.event("block", Kind::Task, Some(&t.id), changes);
                lock.record(&event, |l| l.save_task(t)).unwrap();
            }
        }
        drop(lock);
        let project = root.join(PROJECT_FILE);
        let text = fs::read_to_string(&project).unwrap();
        fs::write(&project, text.replace(FORMAT, "allomos/8")).unwrap();
        let bag = load(&root).taskbag.unwrap();
        assert!(bag.tasks.iter().filter(|t| t.old_blocked()).count() == 5);
        // Not upgraded yet: such a task is not changed, and blockers are not either.
        let refused = change_task(&root, Actor::alm(None), Duration::ZERO, |bag| {
            ops::make_ready(bag, "A01-02", None).map(|o| (o, "ready"))
        });
        assert_eq!(refused.unwrap_err().code(), "blocked_state_old");

        let up = upgrade(&root, Actor::alm(Some("s")), Duration::ZERO)
            .unwrap()
            .unwrap();
        assert_eq!((up.from.as_str(), up.to.as_str()), ("allomos/8", FORMAT));
        let got: Vec<(&str, &str)> = up
            .unblocked
            .iter()
            .map(|(id, state)| (id.as_str(), state.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                ("A01-02", "backlog"),
                ("A01-03", "waiting"),
                ("A01-04", "ready"),
                ("A01-05", "in_progress"),
                ("A01-06", "ready"),
            ]
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.taskbag.unwrap();
        let t = |id: &str| bag.tasks.iter().find(|t| t.id == id).unwrap().clone();
        assert_eq!(
            (
                t("A01-03").waiting_on.as_deref(),
                t("A01-03").who.as_deref()
            ),
            (Some("the licence"), Some("Ann"))
        );
        for id in &ids[1..] {
            assert_eq!(t(id).blocked_by, ["A01-01"], "{id}");
            assert!(ops::is_blocked(&bag, &t(id)), "{id}");
            assert!(!t(id).extra.contains_key("state"), "{id}");
        }
        let e = &events::history(&root, "A01-02").events[0];
        assert_eq!(e.command, "upgrade");
        assert!(
            allomos_core::event::reason(e)
                .unwrap()
                .starts_with("upgrade:")
        );
        // Up to date now: nothing more to upgrade.
        assert!(
            upgrade(&root, Actor::alm(Some("s")), Duration::ZERO)
                .unwrap()
                .is_none()
        );
        fs::remove_dir_all(dir).unwrap();
    }
}
