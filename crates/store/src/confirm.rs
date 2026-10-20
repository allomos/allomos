//! Questions an AI agent asks the person through the Allomos app (Task-A05-02).
//!
//! `alm confirm` leaves a question in `<project>/.allomos/requests/<id>.json` and waits. The
//! app, which watches the folder it shows, lists the question as a popup (and marks it seen); the
//! person's answer is written next to it as `<id>.answer`. These files are working files, never
//! committed: the folder holds a `.gitignore` that ignores everything in it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use allomos_core::Problem;
use serde::{Deserialize, Serialize};

/// Where questions wait, inside the project folder.
pub const REQUESTS_DIR: &str = ".allomos/requests";
/// A plan longer than this is refused: a popup is for reading, not for a whole report.
pub const MAX_PLAN_BYTES: usize = 64 * 1024;

/// A question waiting for the person's answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfirmRequest {
    pub id: String,
    /// One line, shown as the popup's heading.
    pub title: String,
    /// What will happen if the person agrees, in Markdown.
    pub plan: String,
    /// Who asks (`--as`), if known.
    #[serde(default)]
    pub from: Option<String>,
    /// When it was asked, in seconds since 1970.
    pub asked: u64,
    /// When the asker stops waiting. A question past this is no longer shown, so a popup never
    /// outlives an agent whose command was stopped.
    pub until: u64,
    /// What shows the plan is sound (checks run, results), in Markdown, shown under the plan
    /// (`--evidence`, Task-A03-37).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// The task the approval is for (`--task`): a Go ahead is recorded in its history.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    /// A fingerprint of exactly what the person is shown (title, plan and evidence): a Go ahead
    /// counts only for it (Task-A03-37).
    #[serde(default)]
    pub fingerprint: String,
}

/// The fingerprint of what a question shows: its title, plan and evidence.
pub fn shown_fingerprint(title: &str, plan: &str, evidence: Option<&str>) -> String {
    allomos_core::request::fingerprint(&format!(
        "{title}\u{0}{plan}\u{0}{}",
        evidence.unwrap_or("")
    ))
}

/// What the asker adds to a question besides its title and plan.
#[derive(Debug, Clone, Copy, Default)]
pub struct Extra<'a> {
    pub evidence: Option<&'a str>,
    pub task: Option<&'a str>,
}

pub fn requests_dir(project: &Path) -> PathBuf {
    project.join(REQUESTS_DIR)
}

/// IDs are made here (digits and dashes), so anything else is refused rather than used as a path.
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_digit() || c == '-')
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Write `text` to `path` in one step: a reader never sees half a file.
fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, text)?;
    crate::replace_file(&tmp, path)
}

/// Make the requests folder, with a `.gitignore` that keeps everything in it out of Git (its
/// folders too, such as `progress/`).
pub(crate) fn ensure_requests_dir(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        fs::write(
            &ignore,
            "# Allomos's questions to the person. Never commit them.\n*\n",
        )?;
    }
    Ok(())
}

/// Write `text` to `path` in one step (for `progress`).
pub(crate) fn write_file_whole(path: &Path, text: &str) -> io::Result<()> {
    write_whole(path, text)
}

/// Leave a question for the person in `project`. Returns it with its new ID.
pub fn post_confirm(
    project: &Path,
    title: &str,
    plan: &str,
    from: Option<&str>,
    wait_secs: u64,
    extra: Extra,
) -> io::Result<ConfirmRequest> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = requests_dir(project);
    ensure_requests_dir(&dir)?;
    let request = ConfirmRequest {
        id: format!(
            "{}-{}-{}",
            now(),
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ),
        title: title.to_string(),
        plan: plan.to_string(),
        from: from.map(String::from),
        asked: now(),
        until: now().saturating_add(wait_secs),
        evidence: extra.evidence.map(String::from),
        task: extra.task.map(String::from),
        fingerprint: shown_fingerprint(title, plan, extra.evidence),
    };
    let text = serde_json::to_string_pretty(&request).map_err(io::Error::other)?;
    write_whole(&dir.join(format!("{}.json", request.id)), &text)?;
    Ok(request)
}

/// The questions in `project` still waiting for an answer and still in time, oldest first. Each is marked seen, so
/// `alm confirm` knows an app is showing it.
pub fn pending_confirms(project: &Path) -> Vec<ConfirmRequest> {
    let dir = requests_dir(project);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut found: Vec<ConfirmRequest> = entries
        .filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let id = name.strip_suffix(".json")?;
            if !valid_id(id) || dir.join(format!("{id}.answer")).exists() {
                return None;
            }
            let request: ConfirmRequest =
                serde_json::from_str(&fs::read_to_string(e.path()).ok()?).ok()?;
            (request.id == id && request.until >= now()).then_some(request)
        })
        .collect();
    found.sort_by(|a, b| a.asked.cmp(&b.asked).then_with(|| a.id.cmp(&b.id)));
    for request in &found {
        let _ = fs::write(dir.join(format!("{}.seen", request.id)), "");
    }
    found
}

/// Record the person's answer to question `id` in `project`. A Go ahead names the fingerprint of
/// what the person was shown (`shown`; without it, the question's own), so it counts only for
/// that plan (Task-A03-37).
pub fn answer_confirm(
    project: &Path,
    id: &str,
    yes: bool,
    shown: Option<&str>,
) -> Result<(), Problem> {
    let dir = requests_dir(project);
    let request = valid_id(id)
        .then(|| fs::read_to_string(dir.join(format!("{id}.json"))).ok())
        .flatten()
        .and_then(|text| serde_json::from_str::<ConfirmRequest>(&text).ok())
        .ok_or_else(|| Problem::RequestNotFound { id: id.to_string() })?;
    let path = dir.join(format!("{id}.answer"));
    let text = if yes {
        format!("yes {}", shown.unwrap_or(&request.fingerprint))
    } else {
        "no".to_string()
    };
    write_whole(&path, &text).map_err(|e| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}

/// The person's answer to question `id`, once there is one. A Go ahead given for something other
/// than `fingerprint` (the plan changed after the person saw it) counts as no: they were not
/// shown it (Task-A03-37). A bare `yes` comes from an Allomos before that and counts.
pub fn read_answer(project: &Path, id: &str, fingerprint: &str) -> Option<bool> {
    let text = fs::read_to_string(requests_dir(project).join(format!("{id}.answer"))).ok()?;
    match text.trim().split_once(' ') {
        Some(("yes", shown)) => Some(shown == fingerprint),
        _ => match text.trim() {
            "yes" => Some(true),
            "no" => Some(false),
            _ => None,
        },
    }
}

/// Whether an app has shown question `id`.
pub fn was_seen(project: &Path, id: &str) -> bool {
    requests_dir(project).join(format!("{id}.seen")).exists()
}

/// Remove question `id` and its answer: it is settled, or no one answered in time.
pub fn withdraw_confirm(project: &Path, id: &str) {
    if !valid_id(id) {
        return;
    }
    let dir = requests_dir(project);
    for ext in ["json", "seen", "answer"] {
        let _ = fs::remove_file(dir.join(format!("{id}.{ext}")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-confirm-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn a_question_is_asked_seen_answered_and_withdrawn() {
        let dir = tmp("flow");
        let asked = post_confirm(
            &dir,
            "Move tasks",
            "- 3 areas\n- 5 tasks",
            Some("claude"),
            60,
            Extra::default(),
        )
        .unwrap();
        // A question whose asker stopped waiting is not shown.
        let gone = post_confirm(&dir, "Old", "-", None, 0, Extra::default()).unwrap();
        let mut old = gone.clone();
        old.until = 1;
        fs::write(
            requests_dir(&dir).join(format!("{}.json", old.id)),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        assert!(
            fs::read_to_string(requests_dir(&dir).join(".gitignore"))
                .unwrap()
                .contains("\n*\n")
        );
        assert!(!was_seen(&dir, &asked.id));
        assert_eq!(read_answer(&dir, &asked.id, &asked.fingerprint), None);

        // The app lists it (and marks it seen), then the person answers.
        assert_eq!(pending_confirms(&dir), [asked.clone()]);
        assert!(was_seen(&dir, &asked.id));
        answer_confirm(&dir, &asked.id, false, None).unwrap();
        assert_eq!(
            read_answer(&dir, &asked.id, &asked.fingerprint),
            Some(false)
        );
        // An answered question is no longer pending.
        assert!(pending_confirms(&dir).is_empty());

        withdraw_confirm(&dir, &asked.id);
        withdraw_confirm(&dir, &gone.id);
        assert_eq!(fs::read_dir(requests_dir(&dir)).unwrap().count(), 1); // only .gitignore
        fs::remove_dir_all(&dir).unwrap();
    }

    /// A Go ahead counts only for what the person was shown (Task-A03-37).
    #[test]
    fn a_go_ahead_counts_only_for_the_plan_shown() {
        let dir = tmp("scope");
        let extra = Extra {
            evidence: Some("- 58 tests pass"),
            task: Some("A03-37"),
        };
        let asked = post_confirm(&dir, "Move tasks", "- 5 tasks", None, 60, extra).unwrap();
        assert_eq!(asked.evidence.as_deref(), Some("- 58 tests pass"));
        assert_eq!(asked.task.as_deref(), Some("A03-37"));
        assert_eq!(
            asked.fingerprint,
            shown_fingerprint("Move tasks", "- 5 tasks", Some("- 58 tests pass"))
        );
        assert_ne!(
            asked.fingerprint,
            shown_fingerprint("Move tasks", "- 6 tasks", Some("- 58 tests pass"))
        );
        assert_eq!(pending_confirms(&dir), [asked.clone()]);
        // Shown the question as it is: yes.
        answer_confirm(&dir, &asked.id, true, Some(&asked.fingerprint)).unwrap();
        assert_eq!(read_answer(&dir, &asked.id, &asked.fingerprint), Some(true));
        // A yes given for another version of the plan counts as no.
        answer_confirm(&dir, &asked.id, true, Some("0123456789abcdef")).unwrap();
        assert_eq!(
            read_answer(&dir, &asked.id, &asked.fingerprint),
            Some(false)
        );
        // An older app's bare yes still counts.
        fs::write(
            requests_dir(&dir).join(format!("{}.answer", asked.id)),
            "yes",
        )
        .unwrap();
        assert_eq!(read_answer(&dir, &asked.id, &asked.fingerprint), Some(true));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_or_odd_ids_are_refused() {
        let dir = tmp("ids");
        for id in ["1-2-3", "../project", "", "a"] {
            assert_eq!(
                answer_confirm(&dir, id, true, None),
                Err(Problem::RequestNotFound { id: id.into() })
            );
        }
        // A file that is not a question is ignored.
        fs::create_dir_all(requests_dir(&dir)).unwrap();
        fs::write(requests_dir(&dir).join("notes.json"), "{}").unwrap();
        assert!(pending_confirms(&dir).is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
