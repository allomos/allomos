//! Handoffs (Task-A03-06): what a session leaves so that whoever continues, an agent or a
//! person, does not start blind. Since Task-A03-polar770 (approver 2026-10-22) a handoff lives in
//! the task itself: its handoff note, its description's **Evidence** and its history
//! (`alm handoff`, `allomos_core::ops::hand_off`); there are no handoff files. An older taskbag's
//! `handoffs/` folder is only found and reported ([`legacy`]), for **Update the taskbag…** to move
//! what is still useful into the tasks. What every session must know whatever its task lives in
//! `project-notes.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

pub const NOTES_FILE: &str = "project-notes.md";
/// The folder older taskbags kept handoff files in (Task-A03-06 to Task-A03-polar770).
pub const HANDOFFS_DIR: &str = "handoffs";
/// Their template, which went with them.
pub const OLD_TEMPLATE_FILE: &str = "handoff-template.md";

/// What `alm next` and `alm show` add when they show a task's handoff (Task-A03-23, approver
/// 2026-10-16): the rules for the one taking over, not only for the one writing. `alm` checks
/// nothing itself; the agent does ("alm tells; the agent checks").
pub const TAKING_OVER: &str = "Taking over from a handoff:
1. Check the real state before trusting the handoff note: the working copy, branch and running processes it names (as your version control tool shows them; in Git: `git worktree list`, `git status -sb`; and listening ports). What the previous session still holds is not yours to change or remove; if unsure, ask the person (in a quiet run, park the question with `alm wait`).
2. What the task's Evidence lists as already done is not done again. A change of agent, account or session keeps the approvals and what was done; it does not authorize repeating it.
";

/// The default `project-notes.md`.
pub const DEFAULT_NOTES: &str = "# Project notes

What every session must know, whatever its task. Keep it short, and change lines in place.

## Running

Nothing.

## Pitfalls

None yet.

## Repositories

Where the code and this taskbag are kept online, one list item each, so an agent working from a
copy elsewhere (in the cloud, say) can find both: `- Code: https://github.com/<owner>/<repo>`.
The address only: never a token or a personal detail. None yet.

## Where credentials live

Never their values, and no personal details (email addresses, account names): write a
placeholder such as `<ACCOUNT_EMAIL>` and how to look it up. None yet.

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
    /// The list items of its `## Repositories` section (Task-A03-30): where the code and the
    /// taskbag are kept online, as written; `None` when there is no such section.
    pub repositories: Option<Vec<String>>,
}

/// The taskbag's project notes, as far as they can be read.
pub fn project_notes(root: &Path) -> ProjectNotes {
    // Without `.` and `..` (Task-A03-32): a relative `--taskbag` used from a worktree would
    // otherwise print as `.../worktree/../../../../taskbag/...`.
    let path = crate::tidy(root).join(NOTES_FILE);
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
        repositories: text
            .as_deref()
            .and_then(|t| notes_section(t, "repositories"))
            .map(|s| list_items(&s)),
    }
}

/// The entries under the project notes' `## Agents set up` section (Task-A03-24): each list item
/// (`- ` or `* `) as written, without its marker; empty when the file or the section is missing.
/// The app reads only this, for which agents **Update the taskbag…** offers (Task-A03-27).
pub fn agents_set_up(root: &Path) -> Vec<String> {
    let Some(text) = crate::read_text(&root.join(NOTES_FILE))
        .ok()
        .map(|t| crate::lf(&t))
    else {
        return Vec::new();
    };
    list_items(&notes_section(&text, "agents set up").unwrap_or_default())
}

/// Each list item (`- ` or `* `) of a section as written, without its marker; other lines are
/// left out.
fn list_items(section: &str) -> Vec<String> {
    section
        .lines()
        .filter_map(|l| {
            let l = l.trim_start();
            l.strip_prefix("- ").or_else(|| l.strip_prefix("* "))
        })
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect()
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

/// The handoff files an older taskbag keeps in `handoffs/` (Task-A03-polar770): their paths
/// relative to the taskbag (`handoffs/<name>.md`), sorted. Empty when there is no such folder or
/// it holds no files (a `.keep` and other hidden files do not count). Their contents are not
/// read: whether they are still useful is the agent's to check ("alm tells; the agent checks").
pub fn legacy(root: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(root.join(HANDOFFS_DIR))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .filter(|name| !name.starts_with('.'))
        .map(|name| format!("{HANDOFFS_DIR}/{name}"))
        .collect();
    files.sort();
    files
}

/// Changes when the text of [`agent_move_handoffs_request_in`] changes.
pub const MOVE_HANDOFFS_REQUEST_VERSION: u32 = 1;

/// What **Update the taskbag…** asks when an older taskbag still keeps handoff files in
/// `handoffs/` (Task-A03-polar770, approver 2026-10-22: "legacy handoff folder should trigger a
/// taskbag update"), in an interface language (`en`, `zh-CN`, `ja`; English otherwise): write what
/// is still useful in each file into the tasks it names (`alm handoff`, evidence with
/// `alm describe`), skip tasks that are done or dropped, then remove the files and the folder,
/// after the person's Go ahead in the app. `files`: [`legacy`]'s list.
pub fn agent_move_handoffs_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    files: &[String],
) -> String {
    let header = crate::convert::migration_header_in(language, project, taskbag);
    let version = MOVE_HANDOFFS_REQUEST_VERSION;
    let folder = crate::tidy(taskbag).join(HANDOFFS_DIR);
    let folder = crate::shown(&folder);
    let list: String = files
        .iter()
        .map(|f| format!("- `{f}`"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let template = OLD_TEMPLATE_FILE;
    match language {
        "zh-CN" => format!(
            "请把这个任务包里旧的交接文件搬进各自的任务，然后删掉它们（搬交接请求 v{version}）。

{header}

交接现在直接写在任务里：`alm handoff <id> --as <会话名> --text \"<停在哪里、下一步、在等谁>\"`（或 `--from <file.md>`），证据加 `--evidence \"<不可重复执行的操作>\"`。任务包的 `{folder}` 文件夹里还有这些旧的交接文件：

{list}
1. 逐个读这些文件，对照它点名的每个任务（`alm show <id>`）和真实状态（工作副本、分支、正在运行的进程）。`alm` 不判断内容，由你检查。
2. 每个任务，只把仍然有用、任务里还没有的内容写进去：停在哪里和下一步用 `alm handoff`，已经做过、不能再做的对外操作写进描述的 **Evidence**（`alm handoff --evidence`，或 `alm describe`）。任务已完成或已放弃的，不用写。过时的内容不要搬。
3. 然后删掉这些文件、`{template}`（如果有）和 `{folder}` 文件夹，用你的版本控制工具保存并共享。
4. 先用 `alm confirm` 在 Allomos 应用里给用户看完整计划（每个文件写进哪个任务、写什么，哪些不搬）。用户选择“继续”之前，什么都不要改。
"
        ),
        "ja" => format!(
            "このタスクバッグに残っている古い引き継ぎファイルを、それぞれのタスクに移してから削除してください（引き継ぎの移し入れの依頼 v{version}）。

{header}

引き継ぎは今、タスクそのものに書きます：`alm handoff <id> --as <セッション名> --text \"<どこで止まったか、次の一歩、誰を待っているか>\"`（または `--from <file.md>`）。証拠は `--evidence \"<繰り返してはいけない操作>\"` で加えます。タスクバッグの `{folder}` フォルダーには、まだ次の古い引き継ぎファイルがあります：

{list}
1. ファイルを一つずつ読み、名前の挙がっている各タスク（`alm show <id>`）と実際の状態（作業コピー、ブランチ、動いているプロセス）と照らし合わせます。`alm` は内容を判断しません。確かめるのはあなたです。
2. タスクごとに、まだ役に立ち、タスクにまだ書かれていないことだけを書きます：どこで止まったかと次の一歩は `alm handoff`、すでに行った、繰り返してはいけない外部への操作は説明の **Evidence**（`alm handoff --evidence`、または `alm describe`）に。完了または取りやめたタスクには書きません。古くなった内容は移しません。
3. それからこれらのファイル、`{template}`（あれば）と `{folder}` フォルダーを削除し、バージョン管理ツールで保存して共有します。
4. まず `alm confirm` で計画全体（どのファイルをどのタスクに何を書くか、何を移さないか）を Allomos アプリでユーザーに見せます。ユーザーが「進める」を選ぶまで何も変えないでください。
"
        ),
        _ => format!(
            "Please move this taskbag's old handoff files into their tasks, then remove them (move-handoffs request v{version}).

{header}

A handoff is now written into the task itself: `alm handoff <id> --as <session name> --text \"<where it stopped, the next step, who it waits on>\"` (or `--from <file.md>`), with `--evidence \"<operations that must not run twice>\"` for evidence. The taskbag's `{folder}` folder still holds these old handoff files:

{list}
1. Read each file and compare it with every task it names (`alm show <id>`) and with the real state (working copies, branches, running processes). `alm` judges nothing in them: you check.
2. For each task, write in only what is still useful and not in the task yet: where it stopped and the next step with `alm handoff`; operations with an outside effect that were done and must not run twice under the description's **Evidence** (`alm handoff --evidence`, or `alm describe`). Write nothing for a task that is done or dropped. Do not move what is out of date.
3. Then remove these files, `{template}` (if there is one) and the `{folder}` folder, and save and share that with your version control tool.
4. First show the whole plan to the person in the Allomos app with `alm confirm` (which file goes into which task and what it says, and what is not moved). Change nothing until the person chooses Go ahead.
"
        ),
    }
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
    }

    /// Task-A03-polar770: a new taskbag has project notes, and no handoff files or template.
    #[test]
    fn init_makes_the_notes_and_no_handoff_files() {
        let root = tmp("init");
        init(&root, "p", "P").unwrap();
        assert!(root.join(NOTES_FILE).is_file());
        assert!(!root.join(HANDOFFS_DIR).exists());
        assert!(!root.join(OLD_TEMPLATE_FILE).exists());
        assert!(legacy(&root).is_empty());
    }

    /// Task-A03-polar770: an older taskbag's handoff files are found, not read; `.keep` and an
    /// empty folder do not count.
    #[test]
    fn old_handoff_files_are_found() {
        let root = tmp("legacy");
        init(&root, "p", "P").unwrap();
        fs::create_dir_all(root.join(HANDOFFS_DIR)).unwrap();
        fs::write(root.join(HANDOFFS_DIR).join(".keep"), "").unwrap();
        assert!(legacy(&root).is_empty());
        fs::write(root.join(HANDOFFS_DIR).join("b.md"), "x").unwrap();
        fs::write(root.join(HANDOFFS_DIR).join("a.md"), "not even a header").unwrap();
        assert_eq!(legacy(&root), ["handoffs/a.md", "handoffs/b.md"]);
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
    fn agents_set_up_are_the_list_items_of_that_section() {
        let root = tmp("agents");
        init(&root, "p", "P").unwrap();
        // A new taskbag: the section says "None yet." and lists nobody.
        assert!(agents_set_up(&root).is_empty());
        fs::write(
            root.join(NOTES_FILE),
            "## Running\r\n- not an agent\r\n\r\n## Agents set up\r\n\r\nAgent, its file, date.\r\n\r\n- Claude Code: `CLAUDE.md` imports AGENTS.md. 2026-10-16.\r\n* Codex: reads AGENTS.md.\r\n-   \r\n\r\n## Pitfalls\r\n- not an agent either\r\n",
        )
        .unwrap();
        assert_eq!(
            agents_set_up(&root),
            [
                "Claude Code: `CLAUDE.md` imports AGENTS.md. 2026-10-16.",
                "Codex: reads AGENTS.md."
            ]
        );
        // No file: nobody is set up, not an error.
        fs::remove_file(root.join(NOTES_FILE)).unwrap();
        assert!(agents_set_up(&root).is_empty());
    }

    #[test]
    fn repositories_are_the_list_items_of_that_section() {
        let root = tmp("repos");
        init(&root, "p", "P").unwrap();
        // A new taskbag: the section is there and names none.
        assert_eq!(project_notes(&root).repositories, Some(Vec::new()));
        fs::write(
            root.join(NOTES_FILE),
            "## Repositories\r\n\r\nWhere they are.\r\n\r\n- Code: https://github.com/o/app\r\n* Taskbag: https://github.com/o/app-tasks\r\n\r\n## Pitfalls\r\n- not a repository\r\n",
        )
        .unwrap();
        assert_eq!(
            project_notes(&root).repositories.unwrap(),
            [
                "Code: https://github.com/o/app",
                "Taskbag: https://github.com/o/app-tasks"
            ]
        );
        // An older file without the section: `None`, for `alm next` to say so.
        fs::write(root.join(NOTES_FILE), "## Running\nNothing.\n").unwrap();
        assert_eq!(project_notes(&root).repositories, None);
    }

    /// The taking-over rules, in plain words (Task-A03-23).
    #[test]
    fn the_taking_over_rules_say_check_and_do_not_repeat() {
        let flat = TAKING_OVER.split_whitespace().collect::<Vec<_>>().join(" ");
        for words in [
            "Check the real state before trusting the handoff note",
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
    fn the_move_handoffs_request_lists_the_files_and_waits_for_go_ahead() {
        let root = tmp("request");
        init(&root, "p", "P").unwrap();
        let files = ["handoffs/a.md".to_string()];
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_move_handoffs_request_in(language, root.parent().unwrap(), &root, &files);
            for words in [
                "`handoffs/a.md`",
                "alm handoff <id> --as",
                "--evidence",
                "alm confirm",
                "handoff-template.md",
            ] {
                assert!(r.contains(words), "{language}: {words}\n{r}");
            }
        }
    }
}
