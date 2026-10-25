//! Is this project up to date with this Allomos (Task-A01-25)? Two things carry a version: the
//! taskbag's format (`"format": "allomos/<n>"` in `project.json`) and Allomos's section in the
//! project's `AGENTS.md` / `CLAUDE.md` (its heading, `## Tasks: Allomos (v<n>)`).
//!
//! Allomos only *checks*. It never rewrites the section or the taskbag's contents by itself. A
//! taskbag in any format below this Allomos's is not read: an AI agent moves its tasks into a new
//! taskbag, the same way another task system is moved in (Task-A05-02). The app renames the old
//! folder aside and creates the new taskbag ([`move_aside`]); the agent reads the old files, moves
//! everything through `alm`, and removes the old folder only as part of the plan the person
//! confirms with `alm confirm` (owner, 2026-10-15; Task-A01-45, replacing A01-32's guided
//! migration).

use std::fs;
use std::path::{Path, PathBuf};

use allomos_core::{FORMAT, FORMAT_VERSION, Problem, format_version};
use serde::Serialize;
use serde_json::Value;

use crate::shown;
use crate::{AGENT_SECTION_HEADING, PROJECT_FILE, picked_folder, tells_agents};

/// The version of Allomos's section that this Allomos writes. Raise it whenever the section's
/// wording changes, so projects with the older wording are asked to renew it.
///
/// 2: the handoff lines (Task-A03-06).
/// 3: the rule for finishing a task, and `alm next` listing every free ready task (Task-A03-15).
/// 4: `waiting` (on a person) and `blocked` (by another task), seven states (Task-A01-31); the
/// human check: `--without-human-check` with its steps, `alm human`, `alm checked` (Task-A03-18).
/// 5: no `--push`; `alm` never runs Git and prints the Git commands for each change, and a claim
/// is pushed before the work starts (decision 0017, Task-A01-38).
/// 6: `alm title`, `alm describe` and `alm note` change a task's text (Task-A01-37).
/// 7: the project notes (`project-notes.md`): read them at the start, keep them true, and when to
/// update them; every new piece of work gets its task (Task-A03-20).
/// 8: routine choices are the agent's, each recorded with `alm ai-decided` for the approver to
/// review; `alm reviewed` (Task-A03-19, owner rule A00-29).
/// 9: the dividing line between routine choices and questions for the approver, and the rule for a
/// run that goes on without the person (a quiet run): never wait; decide and record routine
/// choices, park a real question with `alm wait` and go on, no `alm confirm` (Task-A00-29; how
/// that mode is chosen: Task-A03-21).
/// 10: before an automatic or loop run, the person chooses quiet or normal mode from the words
/// `alm guide run-modes` gives, and the choice goes in the handoff (Task-A03-21).
/// 11: every task has a base branch by its real name; `alm claim --branch` and `alm add task
/// --branch` tell alm the branch the agent is on, and alm says which base to start from and merge
/// into (Task-A03-22).
/// 12: shared rules live only in AGENTS.md; an agent not yet set up runs `alm guide agent-files`
/// once; after changing shared rules, tell the person which sessions still use the old ones
/// (Task-A03-24).
/// 13: a goal run: given a goal, the agent follows `alm guide goal`, adds the areas and iterations
/// and readies its own tasks, the person having approved in advance by submitting the goal;
/// outside effects still wait for the approver (Task-A03-25-01).
/// 14: card files live beside their tasks (`tasks/<id>/card.md`); an agent that finds tasks
/// linking to card files outside the taskbag reports them (`alm cards found`) and proposes moving
/// them in (`alm guide move-cards`) (Task-A00-30).
/// 15: the person can start that with **Update the taskbag…** in the app, one button for an older
/// taskbag and for card files, whose request `alm move-in` prints (Task-A01-45).
/// 16: a task whose work is finished and only waits for a person's hands-on check ends with
/// `alm done --without-human-check` in quiet and normal mode alike; `alm wait` is only for a real
/// question, an approval or an outside effect (Task-A03-31).
/// 17: personal details (email addresses, account names and the like) are kept out of the project
/// notes and every other file like credentials, written as placeholders (Task-A03-29).
/// 18: the section asks the agent to save and share with "your version control tool", naming
/// Git only as the example (Task-A01-60).
/// 19: a checked task whose work changed goes back to To check (`alm recheck`, the agent decides);
/// an approval through `alm confirm` shows its evidence and task and covers only the plan shown
/// (Task-A03-37).
/// 20: always work a task in a worktree of its own; the main checkout stays on the base branch
/// (Task-A03-38).
/// 21: at the handoff gate's quota check, the agent reports the figure with `alm quota`; the app
/// shows it (Task-A03-02).
/// 22: a line in the conversation starting with `alm ` and one of its commands is a command to run
/// as written, and a guide it prints is followed; `alm guide github-release` (Task-A03-planet530).
/// 23: `alm claim`, `alm label`, `alm release`, `alm done` and `alm handoff` ask for a quota report
/// when the agent's last one is missing or 30 minutes old; the agent reads its usage then and
/// reports it (Task-A03-flour054).
/// 24: approvals are asked in the conversation; `alm confirm` only on a job the person gave from
/// the Allomos app (approver, 2026-10-22; Task-A03-bonsai517).
/// 25: a handoff is written into each task (`alm handoff <id> --text`, `--evidence`); no handoff
/// files, and old ones in `handoffs/` mean the taskbag needs updating (Task-A03-polar770).
/// 26: when the taskbag has project folders, the agent decides a new task's folders
/// (`--folder` / `--no-folder`), `alm folders`, and `alm next` inside a folder (Task-A01-dime494).
pub const SECTION_VERSION: u32 = 26;

/// The taskbag's format compared with this Allomos's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FormatState {
    /// The format this Allomos writes.
    Current,
    /// Written by an older Allomos, in any older format: not read; an AI agent moves its tasks
    /// into a new taskbag (`taskbag_to_move_in`, Task-A01-45).
    Older { found: String },
    /// Written by a newer Allomos: the person chooses (update Allomos, or let the agent go on).
    Newer { found: String },
    /// No `project.json`, or no `allomos/<n>` format in it: the usual checks report it.
    Unknown,
}

/// A taskbag from an older Allomos whose tasks are to be moved into a new one (Task-A01-45).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MoveIn {
    /// The old taskbag's folder: where it was renamed aside (`prepared`), or, when it still is the
    /// taskbag, the name it is to be renamed to.
    pub old: PathBuf,
    /// Its format, for example `allomos/3`.
    pub found: String,
    /// The old folder is already renamed aside and the new taskbag created (by the app); otherwise
    /// the agent does both, after the person's Go ahead.
    pub prepared: bool,
}

/// Allomos's section in one instruction file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SectionState {
    /// `AGENTS.md` or `CLAUDE.md`.
    pub file: String,
    /// The version in the section's heading; `None` when the heading has none (an older section,
    /// or one an agent merged in its own words).
    pub version: Option<u32>,
}

impl SectionState {
    /// Older than this Allomos's section, or without a version.
    pub fn outdated(&self) -> bool {
        self.version.is_none_or(|v| v < SECTION_VERSION)
    }
}

/// Everything the check found. Made once an agent is connected with the project (the section is
/// in an instruction file and the project folder is known; owner, 2026-10-12), and also, whatever
/// is connected, while an older taskbag's tasks are still to be moved in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionCheck {
    /// What this Allomos writes, for messages: `allomos/5`.
    pub format_current: String,
    pub section_current: u32,
    pub format: FormatState,
    /// An older taskbag whose tasks are still to be moved in: the taskbag itself (not prepared),
    /// or one renamed aside beside a new taskbag that has no tasks yet (prepared).
    pub move_in: Option<MoveIn>,
    /// The instruction files that hold Allomos's section (a `CLAUDE.md` that only imports
    /// `@AGENTS.md` is not listed: it follows `AGENTS.md`).
    pub sections: Vec<SectionState>,
    /// A taskbag this Allomos still reads as it is but could upgrade (`allomos/5` → `allomos/6`,
    /// Task-A05-fish286): the format it is in. Upgraded only when the person asks, through
    /// Update the Taskbag… (approver, 2026-10-25).
    pub upgrade: Option<String>,
}

impl VersionCheck {
    /// The instruction files whose section is out of date.
    pub fn outdated_files(&self) -> Vec<String> {
        self.sections
            .iter()
            .filter(|s| s.outdated())
            .map(|s| s.file.clone())
            .collect()
    }

    /// Whether the running agent has something to do: tasks to move in, or the section to renew.
    pub fn needs_update(&self) -> bool {
        self.move_in.is_some() || self.upgrade.is_some() || !self.outdated_files().is_empty()
    }

    /// The warning `alm` adds when the section is out of date.
    pub fn instructions_warning(&self) -> Option<Problem> {
        let files = self.outdated_files();
        (!files.is_empty()).then(|| Problem::InstructionsOutdated {
            files: files.join(" and "),
            current: self.section_current.to_string(),
        })
    }
}

/// The taskbag's format at `root`, read from `project.json` without the usual checks (a newer
/// taskbag may have fields this Allomos does not know, an older one fields it no longer reads).
pub fn format_state(root: &Path) -> FormatState {
    let Some(found) = read_format(root) else {
        return FormatState::Unknown;
    };
    match format_version(&found) {
        // `allomos/5` is read and written as it is until a person upgrades it (Task-A01-dime494).
        Some(n) if (allomos_core::READ_FROM..=FORMAT_VERSION).contains(&n) => FormatState::Current,
        Some(n) if n > FORMAT_VERSION => FormatState::Newer { found },
        Some(_) => FormatState::Older { found },
        None => FormatState::Unknown,
    }
}

fn read_project(root: &Path) -> Option<Value> {
    let text = fs::read_to_string(root.join(PROJECT_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

/// The main project folder as `project.json` records it: the first of `project_folders`
/// (`allomos/6`), else `project_folder`.
fn recorded_folder(value: &Value) -> Option<String> {
    value
        .get("project_folders")
        .and_then(|l| l.get(0)?.get("path"))
        .or_else(|| value.get("project_folder"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

fn read_format(root: &Path) -> Option<String> {
    read_project(root)?
        .get("format")?
        .as_str()
        .map(String::from)
}

/// May the taskbag at `root` be changed? Refused for a newer taskbag (`taskbag_newer`) and for an
/// older one (`taskbag_to_move_in`: its tasks are moved into a new taskbag, never changed in
/// place). Used by `alm` and by the app's changes.
pub fn write_allowed(root: &Path) -> Result<(), Problem> {
    match format_state(root) {
        FormatState::Newer { found } => Err(Problem::TaskbagNewer {
            found,
            supported: FORMAT.to_string(),
        }),
        FormatState::Older { found } => Err(Problem::TaskbagToMoveIn {
            found,
            current: FORMAT.to_string(),
        }),
        FormatState::Current | FormatState::Unknown => Ok(()),
    }
}

// ---------------------------------------------------------------------------------------------
// Moving an older taskbag in (Task-A01-45)
// ---------------------------------------------------------------------------------------------

/// The name the old taskbag at `root` (format version `version`) is renamed to: beside it, its
/// own name followed by the old format (`.allomos/taskbag-allomos-3/`), then `-2`, `-3` … when
/// that name is taken.
pub fn aside_name(root: &Path, version: u32) -> PathBuf {
    let parent = root.parent().unwrap_or(root);
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "taskbag".to_string());
    let base = format!("{name}-allomos-{version}");
    let mut candidate = parent.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = parent.join(format!("{base}-{n}"));
        n += 1;
    }
    candidate
}

/// An older taskbag renamed aside beside the taskbag at `root`: a folder named
/// `<root's name>-allomos-…` holding a `project.json` in a format below this Allomos's. The first
/// by name when there are several.
pub fn old_beside(root: &Path) -> Option<(PathBuf, String)> {
    let parent = root.parent()?;
    let prefix = format!("{}-allomos-", root.file_name()?.to_string_lossy());
    let mut found: Vec<(PathBuf, String)> = fs::read_dir(parent)
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with(&prefix))
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .filter_map(|p| match format_state(&p) {
            FormatState::Older { found } => Some((p, found)),
            _ => None,
        })
        .collect();
    found.sort();
    found.into_iter().next()
}

/// Whether there are tasks to move in for the taskbag at `root`: it is itself older (not
/// prepared), or an older one sits renamed aside beside it while it has no tasks yet (prepared).
pub fn move_in(root: &Path) -> Option<MoveIn> {
    match format_state(root) {
        FormatState::Older { found } => {
            let version = format_version(&found)?;
            Some(MoveIn {
                old: aside_name(root, version),
                found,
                prepared: false,
            })
        }
        FormatState::Current => {
            let (old, found) = old_beside(root)?;
            let empty = crate::load(root)
                .taskbag
                .is_some_and(|bag| bag.tasks.is_empty());
            empty.then_some(MoveIn {
                old,
                found,
                prepared: true,
            })
        }
        FormatState::Newer { .. } | FormatState::Unknown => None,
    }
}

/// The app's **Update the taskbag…** on an older taskbag (Task-A01-45): rename the older taskbag at `root` aside
/// ([`aside_name`]) and create a new, empty taskbag in its place with the old project's ID and name
/// (and its recorded project folder), so an agent can move the tasks across through `alm`. Only
/// the rename and a fresh taskbag: nothing in the old folder is read beyond `project.json`'s ID,
/// name and project folder, or changed. When the new taskbag cannot be created, the old folder is
/// renamed back. On a taskbag whose tasks are already waiting to be moved in, nothing changes and
/// that is returned; on any other, `nothing_to_move_in`.
pub fn move_aside(root: &Path) -> Result<MoveIn, Problem> {
    let found = match format_state(root) {
        FormatState::Older { found } => found,
        FormatState::Current => {
            return move_in(root).ok_or_else(|| Problem::NothingToMoveIn {
                path: root.display().to_string(),
            });
        }
        FormatState::Newer { found } => {
            return Err(Problem::TaskbagNewer {
                found,
                supported: FORMAT.to_string(),
            });
        }
        FormatState::Unknown => {
            return Err(Problem::NothingToMoveIn {
                path: root.display().to_string(),
            });
        }
    };
    let version = format_version(&found).unwrap_or_default();
    let old_project = read_project(root).unwrap_or(Value::Null);
    let text = |key: &str| {
        old_project
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let folder_name = crate::allomos_folder(root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Project".to_string());
    let name = text("name").unwrap_or(folder_name);
    let id = text("id")
        .filter(|id| {
            id.len() <= 64
                && id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        .unwrap_or_else(|| crate::id_from_name(&name));
    let project_folder = read_project(root)
        .as_ref()
        .and_then(recorded_folder)
        .map(|r| crate::setup::join_recorded(&crate::allomos_folder(root), &r));
    let old = aside_name(root, version);
    fs::rename(root, &old).map_err(|e| Problem::WriteFailed {
        path: root.display().to_string(),
        detail: e.to_string(),
    })?;
    if let Err(problem) = crate::init(root, &id, &name) {
        // Leave things as they were: the old folder back in its place.
        if !root.exists() || fs::remove_dir_all(root).is_ok() {
            let _ = fs::rename(&old, root);
        }
        return Err(problem);
    }
    if let Some(folder) = project_folder.filter(|f| f.is_dir()) {
        // Not needed for the move: the open-folder flow asks again when it cannot be recorded.
        let _ = crate::record_project_folder(root, &normalize(&folder));
    }
    Ok(MoveIn {
        old,
        found,
        prepared: true,
    })
}

// ---------------------------------------------------------------------------------------------
// Checking the section, and connecting
// ---------------------------------------------------------------------------------------------

/// The version in Allomos's section heading in `text`: `Some(Some(n))` for
/// `## Tasks: Allomos (v<n>)`, `Some(None)` when the text tells agents about Allomos without a
/// versioned heading, `None` when it does not tell them at all.
pub fn section_version(text: &str) -> Option<Option<u32>> {
    let versioned = text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix(AGENT_SECTION_HEADING)?;
        let n = rest.trim().strip_prefix("(v")?.strip_suffix(')')?;
        n.parse::<u32>().ok()
    });
    match versioned {
        Some(n) => Some(Some(n)),
        None => tells_agents(text).then_some(None),
    }
}

/// Check the taskbag at `root` and the instruction files in `project` (its project folder).
pub fn check(root: &Path, project: &Path) -> VersionCheck {
    let mut sections = Vec::new();
    for file in ["AGENTS.md", "CLAUDE.md"] {
        let text = fs::read_to_string(project.join(file)).unwrap_or_default();
        // A file that imports `@AGENTS.md` follows it; it is checked only when it holds its own
        // copy of the section (a heading such as "# Allomos — Claude entry point" is not one).
        if crate::imports_agents_md(&text) && !text.contains(AGENT_SECTION_HEADING) {
            continue;
        }
        if let Some(version) = section_version(&text) {
            sections.push(SectionState {
                file: file.to_string(),
                version,
            });
        }
    }
    VersionCheck {
        format_current: FORMAT.to_string(),
        section_current: SECTION_VERSION,
        format: format_state(root),
        move_in: move_in(root),
        sections,
        upgrade: upgradable(root),
    }
}

/// The format of a taskbag this Allomos reads as it is but writes newer (`allomos/5` today), so
/// Update the Taskbag… offers the upgrade (Task-A05-fish286); `None` otherwise.
pub fn upgradable(root: &Path) -> Option<String> {
    let found = read_format(root)?;
    format_version(&found)
        .filter(|n| (allomos_core::READ_FROM..FORMAT_VERSION).contains(n))
        .map(|_| found)
}

/// The project folder of the taskbag at `root`: the one recorded in `project.json` (read without
/// the usual checks, so an older taskbag's counts too), else the folder holding `.allomos/`.
pub fn project_folder(root: &Path) -> PathBuf {
    let recorded = read_project(root).as_ref().and_then(recorded_folder);
    match recorded {
        Some(r) => normalize(&crate::setup::join_recorded(
            picked_folder(root).unwrap_or(root),
            &r,
        )),
        None => picked_folder(root).unwrap_or(root).to_path_buf(),
    }
}

/// The project folder of the taskbag at `root` for the check: the one recorded in
/// `project.json`, else the folder holding `.allomos/` when its instruction files mention
/// Allomos. `None` when no agent is connected yet, so nothing is checked.
pub fn connected_project(root: &Path) -> Option<std::path::PathBuf> {
    let value = read_project(root)?;
    let recorded = recorded_folder(&value);
    let folder = match recorded.as_deref() {
        Some(r) => {
            let base = picked_folder(root).unwrap_or(root);
            normalize(&crate::setup::join_recorded(base, r))
        }
        None => picked_folder(root)?.to_path_buf(),
    };
    let told = ["AGENTS.md", "CLAUDE.md"].iter().any(|f| {
        fs::read_to_string(folder.join(f))
            .map(|t| tells_agents(&t))
            .unwrap_or(false)
    });
    told.then_some(folder)
}

pub(crate) fn normalize(path: &Path) -> std::path::PathBuf {
    let mut clean = std::path::PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                clean.pop();
            }
            other => clean.push(other),
        }
    }
    clean
}

/// [`check`] for a taskbag, when an agent is connected ([`connected_project`]) or an older
/// taskbag's tasks are still to be moved in (then the project folder is [`project_folder`]).
pub fn check_connected(root: &Path) -> Option<VersionCheck> {
    match connected_project(root) {
        Some(project) => Some(check(root, &project)),
        // An older taskbag to move in, or one to upgrade, is offered even before an agent is
        // connected.
        None => (move_in(root).is_some() || upgradable(root).is_some())
            .then(|| check(root, &project_folder(root))),
    }
}

// ---------------------------------------------------------------------------------------------
// The update request (an out-of-date section)
// ---------------------------------------------------------------------------------------------

/// The fixed sentences of the update request, in one language.
struct Phrases {
    title: &'static str,
    project: &'static str,
    taskbag: &'static str,
    section: &'static str,
    report: &'static str,
    commit: &'static str,
}

const EN: Phrases = Phrases {
    title: "This project was set up by an older Allomos. Please update it to this version.",
    project: "Project folder",
    taskbag: "Taskbag (Allomos's task files)",
    section: "In the project folder, replace Allomos's section in {files} (headed \"{heading}…\", or any other text about Allomos or `alm`) with the output of `{alm} brief`, exactly as it is. Keep the rest of each file as it is.",
    report: "Show the user what you changed and the commits you made.",
    commit: "If the project is under version control, save only these changes in your version control tool (in Git: commit them): the taskbag's files and {files}. Leave every other change as it is; do not share (push) unless the person asks.",
};

const ZH_CN: Phrases = Phrases {
    title: "这个项目是用旧版的 Allomos 设置的，请把它更新到当前版本。",
    project: "项目文件夹",
    taskbag: "任务包（Allomos 的任务文件）",
    section: "在项目文件夹中，把 {files} 里 Allomos 的一节（标题以“{heading}”开头，或其他关于 Allomos 或 `alm` 的说明）替换为 `{alm} brief` 的输出，原样（英文）使用。文件的其他内容保持不变。",
    report: "告诉用户你改了什么，以及你做的提交。",
    commit: "如果项目在版本控制下，用你的版本控制工具只保存这些改动（用 Git 时：提交）：任务包的文件和 {files}。其他改动保持原样；除非用户要求，不要共享（推送）。",
};

const JA: Phrases = Phrases {
    title: "このプロジェクトは古いバージョンの Allomos で設定されました。今のバージョンに更新してください。",
    project: "プロジェクトのフォルダー",
    taskbag: "タスクバッグ（Allomos のタスクファイル）",
    section: "プロジェクトのフォルダーで、{files} の Allomos の節（見出しが「{heading}」で始まるもの、または Allomos や `alm` についてのほかの説明）を `{alm} brief` の出力で置き換え、そのまま（英語のまま）使います。ファイルのほかの部分は変えません。",
    report: "何を変えたか、どんなコミットをしたかをユーザーに見せてください。",
    commit: "プロジェクトがバージョン管理されていれば、これらの変更だけをバージョン管理ツールで保存します（Git ならコミット）：タスクバッグのファイルと {files}。ほかの変更はそのままにし、頼まれない限り共有（プッシュ）しません。",
};

fn phrases(language: &str) -> &'static Phrases {
    match language {
        "zh-CN" => &ZH_CN,
        "ja" => &JA,
        _ => &EN,
    }
}

/// What File → Connect with the Agents… asks the chosen agent while Allomos's section is out of
/// date, in an interface language (`en`, `zh-CN`, `ja`; English otherwise): replace it with
/// `alm brief`'s output in each such file. The agent does the work; the person sends the request
/// (owner, 2026-10-12). Tasks to move in from an older taskbag have their own request
/// (`agent_move_in_request_in`), which renews the section too.
pub fn agent_update_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    check: &VersionCheck,
) -> String {
    let w = phrases(language);
    let alm = format!("alm --taskbag \"{}\"", shown(taskbag));
    let files = check.outdated_files().join(", ");
    let steps = [
        w.section
            .replace("{files}", &files)
            .replace("{heading}", AGENT_SECTION_HEADING)
            .replace("{alm}", &alm),
        w.commit.replace("{files}", &files),
        w.report.to_string(),
    ];
    let mut out = format!(
        "{}\n\n{}: {}\n{}: {}\n\n",
        w.title,
        w.project,
        shown(project),
        w.taskbag,
        shown(taskbag)
    );
    for (i, s) in steps.iter().enumerate() {
        out.push_str(&format!("{}. {s}\n", i + 1));
    }
    out
}

/// The part of Update the Taskbag…'s request that upgrades an `allomos/5` taskbag to
/// `allomos/6` (Task-A05-fish286), in the request's language. The agent shows the plan first
/// (`alm confirm`), saying what changes and that older Allomos versions will then refuse the
/// taskbag; it upgrades only after the person chooses Go ahead (approver, 2026-10-25: option B).
pub fn agent_upgrade_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    found: &str,
) -> String {
    let alm = format!("alm --taskbag \"{}\"", shown(taskbag));
    let (title, steps): (String, Vec<String>) = match language {
        "zh-CN" => (
            format!(
                "这个任务包是 {found}，请把它升级到 {FORMAT}（项目文件夹和任务所属的文件夹需要它）。"
            ),
            vec![
                format!(
                    "先运行 `alm --version`：它必须是安装好的最新版「换肩」（能识别 `{alm} upgrade`）。否则停下，请用户先安装最新的「换肩」。"
                ),
                format!(
                    "`{alm} progress --title \"更新任务包\" --step \"升级到 {FORMAT}\"`，让用户看到正在做什么。"
                ),
                format!(
                    "运行 `{alm} upgrade`（不加 --confirm），它只说明会做什么。把这些写进计划，并写明：升级后，较旧版本的「换肩」（应用、alm，以及使用它们的每个智能体会话）都会拒绝这个任务包，直到装上最新版。用 `{alm} confirm` 给用户看这份计划。"
                ),
                format!(
                    "用户选择“继续”后：`{alm} upgrade --confirm`，然后按 alm 列出的文件保存并共享任务包的改动。"
                ),
                "汇报做了什么，以及 project.json 现在的格式。".to_string(),
            ],
        ),
        "ja" => (
            format!(
                "このタスクバッグは {found} です。{FORMAT} にアップグレードしてください（プロジェクトフォルダとタスクのフォルダに必要です）。"
            ),
            vec![
                format!(
                    "まず `alm --version` を実行します。入っているのは最新の「肩代」（`{alm} upgrade` を知っているもの）でなければなりません。そうでなければ止まり、先に最新の「肩代」を入れるようユーザーに頼んでください。"
                ),
                format!(
                    "`{alm} progress --title \"タスクバッグを更新\" --step \"{FORMAT} にアップグレード\"` で、何をしているかをユーザーに見せます。"
                ),
                format!(
                    "`{alm} upgrade`（--confirm なし）を実行すると、何をするかだけを説明します。それを計画に書き、アップグレード後は古い「肩代」（アプリ、alm、それらを使うすべてのエージェントのセッション）が最新版を入れるまでこのタスクバッグを開けなくなることも書きます。`{alm} confirm` でこの計画をユーザーに見せます。"
                ),
                format!(
                    "ユーザーが「進める」を選んだら：`{alm} upgrade --confirm`。そのあと alm が挙げるファイルでタスクバッグの変更を保存・共有します。"
                ),
                "何をしたかと、project.json の今の形式を報告してください。".to_string(),
            ],
        ),
        _ => (
            format!(
                "This taskbag is in {found}. Please upgrade it to {FORMAT} (project folders and task folders need it)."
            ),
            vec![
                format!(
                    "First run `alm --version`: it must be the newest Allomos installed (one that knows `{alm} upgrade`). If not, stop and ask the person to install the newest Allomos first."
                ),
                format!(
                    "`{alm} progress --title \"Update the taskbag\" --step \"Upgrading to {FORMAT}\"`, so the person sees what is happening."
                ),
                format!(
                    "Run `{alm} upgrade` (without --confirm): it only says what it will do. Put that in your plan, and say that after the upgrade older Allomos versions (the app, alm, and every agent session using them) refuse the taskbag until the newest Allomos is installed. Show the plan with `{alm} confirm`."
                ),
                format!(
                    "Once the person chooses Go ahead: `{alm} upgrade --confirm`, then save and share the taskbag's changes with the files alm lists."
                ),
                "Report what you did and the format project.json is in now.".to_string(),
            ],
        ),
    };
    let w = phrases(language);
    let mut out = format!(
        "{title}\n\n{}: {}\n{}: {}\n\n",
        w.project,
        shown(project),
        w.taskbag,
        shown(taskbag)
    );
    for (i, s) in steps.iter().enumerate() {
        out.push_str(&format!("{}. {s}\n", i + 1));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, Lock, agent_section, init, load};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let k = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("allomos-test-versions-{name}-{n}-{k}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// A project folder with a taskbag holding one task.
    fn project(name: &str) -> (PathBuf, PathBuf) {
        let dir = tmp(name);
        let root = dir.join(DEFAULT_DIR);
        init(&root, "demo", "Demo").unwrap();
        let lock = Lock::acquire(&root, Duration::from_secs(5)).unwrap();
        lock.create_area(&serde_json::from_str(r#"{"id":"A1","name":"Area"}"#).unwrap())
            .unwrap();
        lock.create_task(
            &serde_json::from_str(r#"{"id":"A1-01","area":"A1","title":"Bake","state":"ready"}"#)
                .unwrap(),
        )
        .unwrap();
        (dir, root)
    }

    fn set_project_format(root: &Path, format: &str) {
        let path = root.join(PROJECT_FILE);
        let text = fs::read_to_string(&path).unwrap();
        let old = read_format(root).unwrap();
        let text = text.replace(&format!("\"{old}\""), &format!("\"{format}\""));
        fs::write(path, text).unwrap();
    }

    /// Every file under `dir`, with its contents, to show that nothing changed.
    fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        let mut out = Vec::new();
        let mut todo = vec![dir.to_path_buf()];
        while let Some(d) = todo.pop() {
            for e in fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    todo.push(p);
                } else {
                    out.push((p.clone(), fs::read(&p).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn section_versions_are_read_from_the_heading() {
        assert_eq!(
            section_version(&agent_section()),
            Some(Some(SECTION_VERSION))
        );
        assert_eq!(
            section_version("# Ours\n\n## Tasks: Allomos (v7)\n\nUse `alm next`.\n"),
            Some(Some(7))
        );
        // A section from before versions, or merged by an agent in its own words.
        assert_eq!(
            section_version("## Tasks: Allomos\n\nRun `alm next`.\n"),
            Some(None)
        );
        assert_eq!(section_version("Start with alm next.\n"), Some(None));
        // Nothing about Allomos: not connected, nothing to check.
        assert_eq!(section_version("# Rules\n\nBe kind.\n"), None);
        assert_eq!(section_version("@AGENTS.md\n"), None);
    }

    #[test]
    fn nothing_is_checked_before_an_agent_is_connected() {
        let (dir, root) = project("not-connected");
        assert_eq!(check_connected(&root), None);
        fs::write(dir.join("AGENTS.md"), "# Rules\n").unwrap();
        assert_eq!(check_connected(&root), None);
    }

    #[test]
    fn an_old_or_unversioned_section_is_out_of_date() {
        let (dir, root) = project("sections");
        fs::write(
            dir.join("AGENTS.md"),
            "# Rules\n\n## Tasks: Allomos\n\nUse `alm next`.\n",
        )
        .unwrap();
        fs::write(dir.join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.format, FormatState::Current);
        assert_eq!(c.move_in, None);
        assert_eq!(c.outdated_files(), vec!["AGENTS.md".to_string()]);
        assert!(c.needs_update());
        assert_eq!(
            c.instructions_warning().unwrap().code(),
            "instructions_outdated"
        );

        // Renewed with the current section: nothing to do.
        fs::write(
            dir.join("AGENTS.md"),
            format!("# Rules\n\n{}", agent_section()),
        )
        .unwrap();
        let c = check_connected(&root).unwrap();
        assert!(!c.needs_update());
        assert_eq!(c.instructions_warning(), None);

        // A CLAUDE.md that imports AGENTS.md under a title naming Allomos follows AGENTS.md.
        fs::write(
            dir.join("CLAUDE.md"),
            "# Allomos — Claude entry point\n\n@AGENTS.md\n",
        )
        .unwrap();
        assert!(!check_connected(&root).unwrap().needs_update());

        // A CLAUDE.md with its own old copy is listed too.
        fs::write(
            dir.join("CLAUDE.md"),
            "## Tasks: Allomos (v0)\n\nalm next\n",
        )
        .unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.outdated_files(), vec!["CLAUDE.md".to_string()]);
    }

    #[test]
    fn the_recorded_project_folder_is_the_one_checked() {
        let base = tmp("recorded");
        let root = base.join("tasks-repo").join(DEFAULT_DIR);
        init(&root, "demo", "Demo").unwrap();
        let app = base.join("app");
        fs::create_dir_all(&app).unwrap();
        fs::write(app.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        crate::record_project_folder(&root, &app).unwrap();
        let c = check_connected(&root).unwrap();
        assert_eq!(c.outdated_files(), vec!["AGENTS.md".to_string()]);
    }

    #[test]
    fn format_states() {
        let (_, root) = project("formats");
        assert_eq!(format_state(&root), FormatState::Current);
        set_project_format(&root, &format!("allomos/{}", FORMAT_VERSION + 1));
        assert_eq!(
            format_state(&root),
            FormatState::Newer {
                found: format!("allomos/{}", FORMAT_VERSION + 1)
            }
        );
        // Every format below the readable ones is older: none is "too old" (Task-A01-45);
        // `allomos/5` is still read and written as it is (Task-A01-dime494).
        set_project_format(&root, "allomos/5");
        assert_eq!(format_state(&root), FormatState::Current);
        for n in 1..allomos_core::READ_FROM {
            set_project_format(&root, &format!("allomos/{n}"));
            assert_eq!(
                format_state(&root),
                FormatState::Older {
                    found: format!("allomos/{n}")
                }
            );
        }
        set_project_format(&root, "fliphopper/1");
        assert_eq!(format_state(&root), FormatState::Unknown);
    }

    /// Owner, 2026-10-15: a taskbag from any older Allomos is not read or changed; every change
    /// is refused with `taskbag_to_move_in`, and the check offers the move.
    #[test]
    fn an_older_taskbag_is_never_read_or_changed() {
        for n in 1..allomos_core::READ_FROM {
            let (dir, root) = project(&format!("older-{n}"));
            fs::write(dir.join("AGENTS.md"), agent_section()).unwrap();
            set_project_format(&root, &format!("allomos/{n}"));
            let before = snapshot(&dir);
            let loaded = load(&root);
            assert!(loaded.taskbag.is_none());
            assert_eq!(loaded.issues[0].problem.code(), "taskbag_to_move_in");
            let err = write_allowed(&root).unwrap_err();
            assert_eq!(err.code(), "taskbag_to_move_in");
            assert!(err.english().contains("`alm move-in`"), "{}", err.english());
            assert_eq!(
                crate::add_area(
                    &root,
                    crate::Actor::APP,
                    Duration::from_millis(100),
                    Some("A2"),
                    "More",
                )
                .unwrap_err()
                .code(),
                "taskbag_to_move_in"
            );
            let c = check_connected(&root).unwrap();
            assert_eq!(
                c.move_in,
                Some(MoveIn {
                    old: dir.join(".allomos").join(format!("taskbag-allomos-{n}")),
                    found: format!("allomos/{n}"),
                    prepared: false,
                })
            );
            assert!(c.needs_update());
            assert_eq!(snapshot(&dir), before, "allomos/{n}: nothing changed");
        }
    }

    /// Approver, 2026-10-16 (Task-A01-45): one Update the taskbag… and one request. An older
    /// taskbag, before and after it is renamed aside, gets the move-in request (which moves its
    /// card files too); a current one gets the move-cards request.
    #[test]
    fn move_the_tasks_in_asks_for_whichever_applies() {
        use crate::{agent_move_tasks_in_request_in, cards};
        let (dir, root) = project("one-request");
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_move_tasks_in_request_in(language, &dir, &root);
            assert_eq!(
                r,
                cards::agent_move_cards_request_in(language, &dir, &root),
                "{language}"
            );
        }
        set_project_format(&root, "allomos/3");
        let older = move_in(&root).unwrap();
        assert!(!older.prepared);
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_move_tasks_in_request_in(language, &dir, &root);
            assert_eq!(
                r,
                crate::agent_move_in_request_in(language, &dir, &root, &older),
                "{language}"
            );
            assert!(r.contains("alm card <id> --from"), "{language}");
            assert!(r.contains("guide move-cards"), "{language}");
        }
        let prepared = move_aside(&root).unwrap();
        let r = agent_move_tasks_in_request_in("en", &dir, &root);
        assert!(r.contains(&format!("to {}", shown(&prepared.old))), "{r}");
        assert!(r.contains("other card files of these tasks"), "{r}");
    }

    /// The app's Update the taskbag… on an older taskbag: the old folder is renamed aside, keeping every file, and a
    /// new, empty taskbag takes its place with the old project's ID, name and project folder.
    #[test]
    fn moving_aside_keeps_the_old_folder_and_makes_a_new_taskbag() {
        let (dir, root) = project("aside");
        let app = dir.join("app");
        fs::create_dir_all(&app).unwrap();
        crate::record_project_folder(&root, &app).unwrap();
        set_project_format(&root, "allomos/3");
        let old_files: Vec<(PathBuf, Vec<u8>)> = snapshot(&root)
            .into_iter()
            .map(|(p, b)| (p.strip_prefix(&root).unwrap().to_path_buf(), b))
            .collect();

        let moved = move_aside(&root).unwrap();
        let aside = dir.join(".allomos").join("taskbag-allomos-3");
        assert_eq!(
            moved,
            MoveIn {
                old: aside.clone(),
                found: "allomos/3".into(),
                prepared: true,
            }
        );
        let kept: Vec<(PathBuf, Vec<u8>)> = snapshot(&aside)
            .into_iter()
            .map(|(p, b)| (p.strip_prefix(&aside).unwrap().to_path_buf(), b))
            .collect();
        assert_eq!(kept, old_files, "the old folder is kept as it was");

        let bag = load(&root).taskbag.unwrap();
        assert_eq!(bag.project.format, FORMAT);
        assert_eq!(bag.project.id, "demo");
        assert_eq!(bag.project.name, "Demo");
        assert!(bag.tasks.is_empty());
        assert_eq!(crate::project_folder_of(&root, &bag.project), Some(app));

        // Until the agent adds a task, the move is still to do (prepared); asking again changes
        // nothing.
        assert_eq!(move_in(&root), Some(moved.clone()));
        assert_eq!(move_aside(&root).unwrap(), moved);
        assert_eq!(check_connected(&root).unwrap().move_in, Some(moved));
        let lock = Lock::acquire(&root, Duration::from_secs(5)).unwrap();
        lock.create_area(&serde_json::from_str(r#"{"id":"A1","name":"Area"}"#).unwrap())
            .unwrap();
        lock.create_task(
            &serde_json::from_str(r#"{"id":"A1-01","area":"A1","title":"Bake","state":"ready"}"#)
                .unwrap(),
        )
        .unwrap();
        drop(lock);
        assert_eq!(move_in(&root), None);
        assert_eq!(move_aside(&root).unwrap_err().code(), "nothing_to_move_in");
    }

    #[test]
    fn an_aside_name_already_taken_gets_a_number() {
        let (dir, root) = project("taken");
        let taken = dir.join(".allomos").join("taskbag-allomos-1");
        fs::create_dir_all(&taken).unwrap();
        assert_eq!(
            aside_name(&root, 1),
            dir.join(".allomos").join("taskbag-allomos-1-2")
        );
        set_project_format(&root, "allomos/1");
        let moved = move_aside(&root).unwrap();
        assert_eq!(moved.old, dir.join(".allomos").join("taskbag-allomos-1-2"));
        assert_eq!(format_state(&root), FormatState::Current);
    }

    #[test]
    fn an_allomos_5_taskbag_is_offered_the_upgrade() {
        let dir = tmp("upgradable");
        let root = dir.join(DEFAULT_DIR);
        crate::init(&root, "p", "P").unwrap();
        // The current format: nothing to upgrade, and no check before an agent is connected.
        assert_eq!(upgradable(&root), None);
        assert_eq!(check_connected(&root), None);
        set_project_format(&root, "allomos/5");
        assert_eq!(upgradable(&root), Some("allomos/5".to_string()));
        // Offered even before an agent is connected, and it is a reason to update.
        let c = check_connected(&root).unwrap();
        assert_eq!(c.upgrade.as_deref(), Some("allomos/5"));
        assert!(c.needs_update());
        // Formats too old to read as they are are moved in, not upgraded.
        set_project_format(&root, "allomos/4");
        assert_eq!(upgradable(&root), None);
    }

    #[test]
    fn the_upgrade_request_confirms_first_and_warns_about_older_versions() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        for (language, warning) in [
            ("en", "older Allomos versions"),
            ("zh-CN", "较旧版本"),
            ("ja", "古い「肩代」"),
        ] {
            let r = agent_upgrade_request_in(language, &project, &taskbag, "allomos/5");
            assert!(
                r.contains("allomos/5") && r.contains(FORMAT),
                "{language}: {r}"
            );
            assert!(
                r.contains(&format!("{}\" confirm`", shown(&taskbag))),
                "{language}: {r}"
            );
            assert!(
                r.contains(&format!("{}\" upgrade --confirm`", shown(&taskbag))),
                "{language}: {r}"
            );
            assert!(r.contains(warning), "{language}: {r}");
            // The plan is shown before the upgrade.
            assert!(
                r.find(" confirm`").unwrap() < r.find("upgrade --confirm").unwrap(),
                "{language}"
            );
        }
    }

    #[test]
    fn the_update_request_renews_the_section() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        let check = VersionCheck {
            format_current: FORMAT.into(),
            section_current: SECTION_VERSION,
            format: FormatState::Current,
            move_in: None,
            upgrade: None,
            sections: vec![SectionState {
                file: "AGENTS.md".into(),
                version: None,
            }],
        };
        for language in ["en", "zh-CN", "ja"] {
            let r = agent_update_request_in(language, &project, &taskbag, &check);
            assert!(
                r.contains(&format!("{}\" brief`", shown(&taskbag))),
                "{language}"
            );
            assert!(r.contains("AGENTS.md"), "{language}");
            assert!(!r.contains("migrate"), "{language}");
            assert!(!r.contains("{"), "{language}: placeholders filled");
        }
        // The app's tests rely on its first line.
        let r = agent_update_request_in("en", &project, &taskbag, &check);
        assert!(r.starts_with("This project was set up by an older Allomos"));
    }
}
