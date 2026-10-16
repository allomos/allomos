//! Handoffs (Task-A03-06): the short notes a session leaves so that whoever continues, an agent
//! or a person, does not start blind. One small file per session in `handoffs/`, beside `tasks/`,
//! naming the tasks it covers; the outline comes from the project's `handoff-template.md`. What
//! every session must know whatever its task lives in `project-notes.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use allomos_core::Problem;
use serde::Serialize;

use crate::{Lock, write_atomic, write_failed};

pub const TEMPLATE_FILE: &str = "handoff-template.md";
pub const NOTES_FILE: &str = "project-notes.md";
pub const HANDOFFS_DIR: &str = "handoffs";

/// How long writing a handoff waits for another writer.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The default outline of one handoff. Written for any reader, because the writer does not know
/// who will take over (owner, 2026-10-12): part 1 in plain words, part 2 to restart the work.
pub const DEFAULT_TEMPLATE: &str = "# Handoff

## For anyone, in plain words

- **What was being done:**
- **Where it stopped:**
- **Verified, and not verified yet:**
- **Waiting on whom:**
- **The next step (one):**

## Restart details

- **Checkout:** <!-- worktree, branch, last commit, pushed or not -->
- **Uncommitted changes:** <!-- file, and why; or none -->
- **Running:** <!-- what, port, how to restart it, safe to stop; or none -->
- **Approvals in force:** <!-- what is approved, and what is not -->
- **Already done, do not repeat:** <!-- operations with an outside effect that must not run twice (an email sent, a deployment, SQL applied, a hosted resource or remote created or changed); or none -->

<!-- Keep each line short. Evidence (numbers, logs, versions) goes in the task's description, not here. -->
";

/// What `alm next` and `alm show` add when they show a task's handoff (Task-A03-23, approver
/// 2026-10-16): the rules for the one taking over, not only for the one writing. `alm` checks
/// nothing itself; the agent does ("alm tells; the agent checks").
pub const TAKING_OVER: &str = "Taking over from a handoff:
1. Check the real state before trusting the handoff: the worktree, branch and running processes it names (`git worktree list`, `git status -sb`, listening ports). What the previous session still holds is not yours to change or remove; if unsure, ask the person (in a quiet run, park the question with `alm wait`).
2. What the handoff lists as already done is not done again. A change of agent, account or session keeps the approvals and what was done; it does not authorize repeating it.
";

/// The default `project-notes.md`.
pub const DEFAULT_NOTES: &str = "# Project notes

What every session must know, whatever its task. Keep it short, and change lines in place.

## Running

Nothing.

## Pitfalls

None yet.

## Where credentials live

Never their values. None yet.

## Agents set up

AI agents whose rule files follow AGENTS.md (`alm guide agent-files`): agent, its file, date.
None yet.
";

/// What `alm next` and `alm handoff` show of `project-notes.md` (Task-A03-20): facts only, read
/// from the file; `alm` judges nothing about them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectNotes {
    /// `project-notes.md`, relative to the taskbag.
    pub file: String,
    /// The full path, for the agent to open and edit.
    pub path: PathBuf,
    /// False when the taskbag has no `project-notes.md`: said plainly, not an error.
    pub found: bool,
    /// The day the file last changed on this computer (its modification time), `YYYY-MM-DD` (UTC).
    pub changed: Option<String>,
    /// The text of its `## Running` section; `None` when there is no such section.
    pub running: Option<String>,
    /// The text of its `## Pitfalls` section; `None` when there is no such section.
    pub pitfalls: Option<String>,
}

/// The taskbag's project notes, as far as they can be read.
pub fn project_notes(root: &Path) -> ProjectNotes {
    let path = root.join(NOTES_FILE);
    let text = crate::read_text(&path).ok().map(|t| crate::lf(&t));
    let changed = fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| {
            let (y, m, d) = civil_from_days((d.as_secs() / 86_400) as i64);
            format!("{y:04}-{m:02}-{d:02}")
        });
    ProjectNotes {
        file: NOTES_FILE.to_string(),
        path,
        found: text.is_some(),
        changed: text.as_ref().and(changed),
        running: text.as_deref().and_then(|t| notes_section(t, "running")),
        pitfalls: text.as_deref().and_then(|t| notes_section(t, "pitfalls")),
    }
}

/// The text under the first `## ` heading that starts with `name` (any case), up to the next
/// heading of the same or a higher level; trimmed. `None` when there is no such heading.
fn notes_section(text: &str, name: &str) -> Option<String> {
    let mut lines = text.lines();
    lines.find(|l| {
        l.strip_prefix("## ")
            .is_some_and(|h| h.trim().to_lowercase().starts_with(name))
    })?;
    let body: Vec<&str> = lines
        .take_while(|l| !(l.starts_with("## ") || l.starts_with("# ")))
        .collect();
    Some(body.join("\n").trim().to_string())
}

/// One handoff file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Handoff {
    /// Path relative to the taskbag, `handoffs/<name>.md`.
    pub file: String,
    /// The session or person who wrote it.
    pub writer: String,
    /// The tasks it covers; empty for a session with no task.
    pub tasks: Vec<String>,
    /// When it was first written and last changed, `YYYY-MM-DDTHH:MMZ` (UTC).
    pub written: String,
    pub updated: String,
    /// The first line that says something, for lists.
    pub summary: String,
    /// The text below the header.
    pub body: String,
}

/// The project's template; the default when the file is missing.
pub fn template(root: &Path) -> String {
    crate::read_text(&root.join(TEMPLATE_FILE)).unwrap_or_else(|_| DEFAULT_TEMPLATE.to_string())
}

/// Replace the project's template. An empty text is refused.
pub fn set_template(root: &Path, text: &str) -> Result<(), Problem> {
    if text.trim().is_empty() {
        return Err(Problem::TemplateEmpty);
    }
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let mut text = text.to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }
    write_atomic(&lock.root().join(TEMPLATE_FILE), &text)
}

/// Every handoff, most recently changed first.
pub fn list(root: &Path) -> Vec<Handoff> {
    let mut all: Vec<Handoff> = fs::read_dir(root.join(HANDOFFS_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
        .filter_map(|p| {
            let text = fs::read_to_string(&p).ok()?;
            let name = p.file_name()?.to_str()?.to_string();
            Some(parse(&format!("{HANDOFFS_DIR}/{name}"), &text))
        })
        .collect();
    all.sort_by(|a, b| b.updated.cmp(&a.updated).then(b.file.cmp(&a.file)));
    all
}

/// The most recent handoff that names `task`.
pub fn latest_for(root: &Path, task: &str) -> Option<Handoff> {
    list(root)
        .into_iter()
        .find(|h| h.tasks.iter().any(|t| t == task))
}

/// Read one handoff file: a header between `---` lines (`writer:`, `tasks:`, `written:`,
/// `updated:`), then the text. A file without a header is all text, by an unknown writer.
/// `\r\n` line endings (a checkout on Windows) read the same as `\n`.
pub fn parse(file: &str, text: &str) -> Handoff {
    let text = &crate::lf(text);
    let mut h = Handoff {
        file: file.to_string(),
        writer: String::new(),
        tasks: Vec::new(),
        written: String::new(),
        updated: String::new(),
        summary: String::new(),
        body: text.to_string(),
    };
    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---\n") {
            for line in rest[..end].lines() {
                let Some((key, value)) = line.split_once(':') else {
                    continue;
                };
                let value = value.trim();
                match key.trim() {
                    "writer" => h.writer = value.to_string(),
                    "tasks" => {
                        h.tasks = value
                            .split(',')
                            .map(str::trim)
                            .filter(|t| !t.is_empty())
                            .map(String::from)
                            .collect()
                    }
                    "written" => h.written = value.to_string(),
                    "updated" => h.updated = value.to_string(),
                    _ => {}
                }
            }
            h.body = rest[end + 5..].trim_start_matches('\n').to_string();
        }
    }
    if h.updated.is_empty() {
        h.updated = h.written.clone();
    }
    h.summary = summary(&h.body);
    h
}

/// The first line with something in it: not a heading, not a comment, not an empty template field
/// (a hint in `<!-- -->` does not count).
fn summary(body: &str) -> String {
    body.lines()
        .map(without_comments)
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.trim_start_matches(['-', '*', ' ']).to_string())
        .map(|l| {
            // "**What was being done:** baking" → "What was being done: baking"
            l.replace("**", "")
        })
        .find(|l| !l.ends_with(':'))
        .unwrap_or_default()
}

/// `line` without `<!-- … -->` parts.
fn without_comments(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn header(h: &Handoff) -> String {
    format!(
        "---\nwriter: {}\ntasks: {}\nwritten: {}\nupdated: {}\n---\n\n",
        h.writer,
        h.tasks.join(", "),
        h.written,
        h.updated
    )
}

/// What [`write`] did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Written {
    pub handoff: Handoff,
    /// The full path, for the agent to open and fill in.
    pub path: PathBuf,
    /// False when this session's file was there already and was updated.
    pub created: bool,
}

/// Write `writer`'s handoff (`alm handoff`, the app's form). The session's newest file is updated
/// when it has one, otherwise a new file is made from the template. `fresh`: always a new file (a
/// person writing in the app has no session; each handoff is its own). `tasks`: replaces the list of
/// tasks it covers (each must exist); `body`: replaces the text. With neither, a new file gets the
/// template and an existing one only its time.
pub fn write(
    root: &Path,
    writer: &str,
    tasks: Option<&[String]>,
    body: Option<&str>,
    known_tasks: &[String],
    fresh: bool,
) -> Result<Written, Problem> {
    let writer = writer.trim();
    if writer.is_empty() {
        return Err(Problem::HolderMissing);
    }
    if let Some(missing) = tasks
        .unwrap_or_default()
        .iter()
        .find(|t| !known_tasks.contains(t))
    {
        return Err(Problem::TaskNotFound {
            id: missing.clone(),
        });
    }
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let now = utc_now();
    let existing = list(lock.root())
        .into_iter()
        .find(|h| !fresh && h.writer == writer);
    let created = existing.is_none();
    let mut h = match existing {
        Some(h) => h,
        None => Handoff {
            file: free_name(
                lock.root(),
                &format!("{}-{}", file_stamp(&now), slug(writer)),
            ),
            writer: writer.to_string(),
            tasks: Vec::new(),
            written: now.clone(),
            updated: now.clone(),
            summary: String::new(),
            body: template(lock.root()),
        },
    };
    if let Some(tasks) = tasks {
        h.tasks = tasks.to_vec();
    }
    if let Some(body) = body {
        h.body = body.to_string();
    }
    if !h.body.ends_with('\n') {
        h.body.push('\n');
    }
    h.updated = now;
    h.summary = summary(&h.body);
    let path = lock.root().join(&h.file);
    let dir = lock.root().join(HANDOFFS_DIR);
    fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
    write_atomic(&path, &format!("{}{}", header(&h), h.body))?;
    Ok(Written {
        handoff: h,
        path,
        created,
    })
}

/// Remove a handoff whose work is finished (`alm handoff --remove`, Task-A03-13). `file` is its
/// name as `alm handoffs` prints it (`handoffs/<name>.md`) or just `<name>.md`. The file is
/// deleted; Git history keeps it (owner, 2026-10-13).
pub fn remove(root: &Path, file: &str) -> Result<Handoff, Problem> {
    let name = file.trim();
    let name = name
        .strip_prefix(&format!("{HANDOFFS_DIR}/"))
        .unwrap_or(name);
    let missing = || Problem::HandoffNotFound {
        file: file.to_string(),
    };
    if name.is_empty()
        || name.contains(['/', '\\'])
        || name.starts_with('.')
        || !name.ends_with(".md")
    {
        return Err(missing());
    }
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let path = lock.root().join(HANDOFFS_DIR).join(name);
    let text = fs::read_to_string(&path).map_err(|_| missing())?;
    fs::remove_file(&path).map_err(|e| write_failed(&path, e))?;
    crate::note_written(&path);
    Ok(parse(&format!("{HANDOFFS_DIR}/{name}"), &text))
}

/// `handoffs/<base>.md`, or `<base>-2.md`, … when that is taken.
fn free_name(root: &Path, base: &str) -> String {
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{base}.md")
        } else {
            format!("{base}-{n}.md")
        };
        if !root.join(HANDOFFS_DIR).join(&name).exists() {
            return format!("{HANDOFFS_DIR}/{name}");
        }
        n += 1;
    }
}

/// A writer's name as part of a file name: `claude/a03-06` → `claude-a03-06`.
fn slug(writer: &str) -> String {
    let s: String = writer
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        "session".to_string()
    } else {
        s
    }
}

/// `2026-10-12T11:30Z` → `2026-10-12-1130`.
fn file_stamp(stamp: &str) -> String {
    stamp.replace('T', "-").replace([':', 'Z'], "")
}

/// Now, in UTC, as `YYYY-MM-DDTHH:MMZ`. Tests may fix it with `ALLOMOS_NOW`.
pub fn utc_now() -> String {
    if let Ok(fixed) = std::env::var("ALLOMOS_NOW") {
        return fixed;
    }
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60
    )
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, init};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-handoff-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p.join(DEFAULT_DIR)
    }

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(20_738), (2026, 10, 12));
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(file_stamp("2026-10-12T11:30Z"), "2026-10-12-1130");
        assert_eq!(slug("claude/a03-06 handoffs"), "claude-a03-06-handoffs");
    }

    #[test]
    fn init_makes_the_template_notes_and_folder() {
        let root = tmp("init");
        init(&root, "p", "P").unwrap();
        assert_eq!(template(&root), DEFAULT_TEMPLATE);
        assert!(root.join(NOTES_FILE).is_file());
        assert!(root.join(HANDOFFS_DIR).join(".keep").is_file());
        assert!(list(&root).is_empty());
    }

    #[test]
    fn project_notes_give_running_pitfalls_and_the_day_they_changed() {
        let root = tmp("notes");
        init(&root, "p", "P").unwrap();
        let notes = project_notes(&root);
        assert!(notes.found);
        assert_eq!(notes.running.as_deref(), Some("Nothing."));
        assert_eq!(notes.pitfalls.as_deref(), Some("None yet."));
        // The file's modification time, as a day.
        let day = std::fs::File::options()
            .write(true)
            .open(root.join(NOTES_FILE))
            .unwrap();
        day.set_modified(UNIX_EPOCH + std::time::Duration::from_secs(20_738 * 86_400 + 600))
            .unwrap();
        assert_eq!(project_notes(&root).changed.as_deref(), Some("2026-10-12"));
        // A project's own headings and line endings; a section that is missing is `None`.
        fs::write(
            root.join(NOTES_FILE),
            "# Notes\r\n\r\n## Running now\r\n\r\n- a server on :8080\r\n\r\n### detail\r\nport\r\n\r\n## Current truth\r\nx\r\n",
        )
        .unwrap();
        let notes = project_notes(&root);
        assert_eq!(
            notes.running.as_deref(),
            Some("- a server on :8080\n\n### detail\nport")
        );
        assert_eq!(notes.pitfalls, None);
        // No file: said plainly, not an error.
        fs::remove_file(root.join(NOTES_FILE)).unwrap();
        let notes = project_notes(&root);
        assert!(!notes.found);
        assert_eq!((notes.changed, notes.running), (None, None));
    }

    #[test]
    fn one_file_per_session_covering_several_tasks() {
        let root = tmp("write");
        init(&root, "p", "P").unwrap();
        let known = vec!["A1-01".to_string(), "A1-02".to_string()];
        let first = write(&root, "claude/run", None, None, &known, false).unwrap();
        assert!(first.created);
        assert!(
            first.handoff.file.starts_with("handoffs/")
                && first.handoff.file.ends_with("-claude-run.md")
        );
        let text = fs::read_to_string(&first.path).unwrap();
        assert!(
            text.starts_with("---\nwriter: claude/run\ntasks: \n"),
            "{text}"
        );
        assert!(text.contains("## For anyone, in plain words"));

        // The same session again: the same file, now covering two tasks with its own words.
        let body = "# Handoff\n\n- **What was being done:** baking bread\n";
        let again = write(&root, "claude/run", Some(&known), Some(body), &known, false).unwrap();
        assert!(!again.created);
        assert_eq!(again.path, first.path);
        assert_eq!(again.handoff.summary, "What was being done: baking bread");
        assert_eq!(list(&root).len(), 1);
        assert_eq!(latest_for(&root, "A1-02").unwrap().writer, "claude/run");

        // Another session: its own file; a task it does not name keeps the older handoff.
        write(&root, "codex/x", Some(&known[..1]), None, &known, false).unwrap();
        assert_eq!(list(&root).len(), 2);
        assert_eq!(latest_for(&root, "A1-02").unwrap().writer, "claude/run");
        assert_eq!(latest_for(&root, "A9-99"), None);

        // Unknown tasks and nameless writers are refused.
        let bad = ["A9-99".to_string()];
        assert_eq!(
            write(&root, "codex/x", Some(&bad), None, &known, false)
                .unwrap_err()
                .code(),
            "task_not_found"
        );
        assert_eq!(
            write(&root, " ", None, None, &known, false)
                .unwrap_err()
                .code(),
            "holder_missing"
        );
    }

    #[test]
    fn a_handoff_is_removed_and_others_stay() {
        let root = tmp("remove");
        init(&root, "p", "P").unwrap();
        let known = vec!["A1-01".to_string(), "A1-02".to_string()];
        let both = write(&root, "claude/both", Some(&known), None, &known, false).unwrap();
        write(&root, "codex/one", Some(&known[..1]), None, &known, false).unwrap();
        write(&root, "codex/none", None, None, &known, false).unwrap();

        // Removed by its listed name or its bare name; then it is gone from the list.
        let gone = remove(&root, &both.handoff.file).unwrap();
        assert_eq!(gone.writer, "claude/both");
        assert!(!both.path.exists());
        assert_eq!(list(&root).len(), 2);
        let name = list(&root)[0]
            .file
            .trim_start_matches("handoffs/")
            .to_string();
        remove(&root, &name).unwrap();
        assert_eq!(list(&root).len(), 1);

        // Missing files and anything outside handoffs/ are refused.
        for bad in [
            &both.handoff.file[..],
            "../project.json",
            "handoffs/.keep",
            "x.json",
            "",
        ] {
            assert_eq!(
                remove(&root, bad).unwrap_err().code(),
                "handoff_not_found",
                "{bad}"
            );
        }
        assert!(root.join("project.json").is_file());
    }

    #[test]
    fn a_person_writes_a_new_handoff_each_time() {
        let root = tmp("fresh");
        init(&root, "p", "P").unwrap();
        let a = write(&root, "will", None, Some("one"), &[], true).unwrap();
        let b = write(&root, "will", None, Some("two"), &[], true).unwrap();
        assert!(a.created && b.created);
        assert_ne!(a.path, b.path);
        assert_eq!(list(&root).len(), 2);
    }

    /// The template asks for what must not run twice (Task-A03-23).
    #[test]
    fn the_template_lists_what_was_already_done() {
        let restart = DEFAULT_TEMPLATE.split("## Restart details").nth(1).unwrap();
        assert!(restart.contains("- **Already done, do not repeat:**"));
        assert!(restart.contains("must not run twice"));
        assert!(restart.contains("or none"));
    }

    /// The taking-over rules, in plain words (Task-A03-23).
    #[test]
    fn the_taking_over_rules_say_check_and_do_not_repeat() {
        let flat = TAKING_OVER.split_whitespace().collect::<Vec<_>>().join(" ");
        for words in [
            "Check the real state before trusting the handoff",
            "git worktree list",
            "listening ports",
            "still holds is not yours to change or remove",
            "if unsure, ask the person",
            "already done is not done again",
            "keeps the approvals and what was done; it does not authorize repeating it",
        ] {
            assert!(flat.contains(words), "{words}");
        }
    }

    #[test]
    fn an_unfilled_template_has_no_summary() {
        assert_eq!(summary(DEFAULT_TEMPLATE), "");
        let half = DEFAULT_TEMPLATE.replace("- **Running:** <!--", "- **Running:** oven on <!--");
        assert_eq!(summary(&half), "Running: oven on");
    }

    #[test]
    fn a_file_without_a_header_is_still_listed() {
        let h = parse("handoffs/old.md", "Stopped at the oven.\n");
        assert_eq!(h.summary, "Stopped at the oven.");
        assert!(h.tasks.is_empty() && h.writer.is_empty());
    }

    #[test]
    fn the_template_can_be_replaced_but_not_emptied() {
        let root = tmp("template");
        init(&root, "p", "P").unwrap();
        set_template(&root, "# Ours\n\n- Where:").unwrap();
        assert_eq!(template(&root), "# Ours\n\n- Where:\n");
        assert_eq!(
            set_template(&root, "  \n").unwrap_err().code(),
            "template_empty"
        );
    }
}
