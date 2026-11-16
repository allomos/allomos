//! A task's change type, risk and what it touches, in the history (Task-A01-palm011, decision
//! 0021 § 1.1): recording them (`alm classify`) under the taskbag's lock, and the emergencies
//! still to be approved afterwards. The rules are in [`allomos_core::classify`].

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use allomos_core::classify::{self, Classification, Given, Unapproved};
use allomos_core::event::Kind;
use allomos_core::{Issue, Problem, Task, ops};

use crate::events::{self, Actor};
use crate::lock_for_change;

/// What counts as high, medium and low risk (decision 0027 § 1), as an agent reads it before it
/// records a task's risk with `alm classify`: in the AGENTS.md section and in
/// `alm guide workflow` (Task-A00-nectar077). H1–H7 are fixed for every project; a project adds
/// rules of its own in its settings and cannot take these away. alm only tells them: the agent
/// judges, and alm compares nothing with them.
pub const RISK_RULES: &str = "  - High: H1 an outside effect (a hosted resource, SQL on a real target, a deployment, a secret,
    sending email, creating a repository, adding or changing a remote, other people's data);
    H2 a release; H3 a change of data format (the format of the files of tasks, events or the
    taskbag; a migration); H4 removing something (a feature, a command, data, files);
    H5 permissions and security (credentials, pairing, sign-in, signing, who may do what);
    H6 a change of shared rules (AGENTS.md, `alm guide`); H7 making `alm` do one more check of
    its own.
  - Medium: M1 a large change (say 500 lines or more, or 3 modules or more); M2 a check that
    was not run, or finishing with `--without-human-check`; M3 words and look that users see;
    M4 the principle of how something is built (the kind of AI decision the approver reviews).
  - Low: everything else: only documents, internal tidying or tests, with every check passed.
";

/// Today in UTC, `YYYY-MM-DD` (`ALLOMOS_NOW` fixes it in tests).
fn today() -> String {
    events::now_utc().chars().take(10).collect()
}

/// Every task's classification, by task ID.
pub fn by_task(root: &Path) -> BTreeMap<String, Classification> {
    classify::recorded(&events::list(root).0)
}

/// Every emergency change without an approval recorded afterwards, on today's date.
pub fn unapproved(root: &Path) -> Vec<Unapproved> {
    classify::unapproved(&by_task(root), &crate::approvals::approvals(root), &today())
}

/// What [`set`] did.
#[derive(Debug, Clone, PartialEq)]
pub struct Classified {
    pub task: Task,
    /// The classification after it, as the history now reads it.
    pub classification: Classification,
    /// Whether an event was written (false: nothing changed).
    pub changed: bool,
    /// Problems found in the taskbag's files while loading it.
    pub issues: Vec<Issue>,
}

/// Classify task `id`: one `classify` event, under the lock, when something changes; the task's
/// files do not change.
pub fn set(
    root: &Path,
    actor: Actor,
    wait: Duration,
    id: &str,
    given: &Given,
) -> Result<Classified, Problem> {
    let (lock, bag, issues) = lock_for_change(root, wait)?;
    let recorded = by_task(root);
    let (changes, _) = classify::set(&bag, &recorded, id, actor.by, given, &today())?;
    let task = ops::find_task(&bag, id)?;
    let changed = !changes.is_empty();
    if changed {
        let event = actor.event(classify::COMMAND, Kind::Task, Some(&task.id), changes);
        lock.record(&event, |_| Ok(()))?;
    }
    let classification = by_task(root).remove(&task.id).unwrap_or_default();
    Ok(Classified {
        task,
        classification,
        changed,
        issues,
    })
}
