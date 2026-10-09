//! Questions an AI agent asks the person through the FlipHopper app (Task-A05-02).
//!
//! `fh confirm` leaves a question in `<project>/.fliphopper/requests/<id>.json` and waits. The
//! app, which watches the folder it shows, lists the question as a popup (and marks it seen); the
//! person's answer is written next to it as `<id>.answer`. These files are working files, never
//! committed: the folder holds a `.gitignore` that ignores everything in it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use fliphopper_core::Problem;
use serde::{Deserialize, Serialize};

/// Where questions wait, inside the project folder.
pub const REQUESTS_DIR: &str = ".fliphopper/requests";
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
    fs::rename(&tmp, path)
}

/// Leave a question for the person in `project`. Returns it with its new ID.
pub fn post_confirm(
    project: &Path,
    title: &str,
    plan: &str,
    from: Option<&str>,
    wait_secs: u64,
) -> io::Result<ConfirmRequest> {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = requests_dir(project);
    fs::create_dir_all(&dir)?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        fs::write(
            &ignore,
            "# FlipHopper's questions to the person. Never commit them.\n*\n",
        )?;
    }
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
    };
    let text = serde_json::to_string_pretty(&request).map_err(io::Error::other)?;
    write_whole(&dir.join(format!("{}.json", request.id)), &text)?;
    Ok(request)
}

/// The questions in `project` still waiting for an answer and still in time, oldest first. Each is marked seen, so
/// `fh confirm` knows an app is showing it.
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

/// Record the person's answer to question `id` in `project`.
pub fn answer_confirm(project: &Path, id: &str, yes: bool) -> Result<(), Problem> {
    let dir = requests_dir(project);
    if !valid_id(id) || !dir.join(format!("{id}.json")).is_file() {
        return Err(Problem::RequestNotFound { id: id.to_string() });
    }
    let path = dir.join(format!("{id}.answer"));
    write_whole(&path, if yes { "yes" } else { "no" }).map_err(|e| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}

/// The person's answer to question `id`, once there is one.
pub fn read_answer(project: &Path, id: &str) -> Option<bool> {
    let text = fs::read_to_string(requests_dir(project).join(format!("{id}.answer"))).ok()?;
    match text.trim() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
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
        let p = std::env::temp_dir().join(format!("fliphopper-confirm-{name}-{n}"));
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
        )
        .unwrap();
        // A question whose asker stopped waiting is not shown.
        let gone = post_confirm(&dir, "Old", "-", None, 0).unwrap();
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
        assert_eq!(read_answer(&dir, &asked.id), None);

        // The app lists it (and marks it seen), then the person answers.
        assert_eq!(pending_confirms(&dir), [asked.clone()]);
        assert!(was_seen(&dir, &asked.id));
        answer_confirm(&dir, &asked.id, false).unwrap();
        assert_eq!(read_answer(&dir, &asked.id), Some(false));
        // An answered question is no longer pending.
        assert!(pending_confirms(&dir).is_empty());

        withdraw_confirm(&dir, &asked.id);
        withdraw_confirm(&dir, &gone.id);
        assert_eq!(fs::read_dir(requests_dir(&dir)).unwrap().count(), 1); // only .gitignore
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_or_odd_ids_are_refused() {
        let dir = tmp("ids");
        for id in ["1-2-3", "../project", "", "a"] {
            assert_eq!(
                answer_confirm(&dir, id, true),
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
