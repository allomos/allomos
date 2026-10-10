//! Reads and creates seabags on disk.
//!
//! A *seabag* is a folder holding `project.json`, `areas/<id>.json`, optionally
//! `iterations/<id>.json`, and `tasks/<id>/task.json` (+ `description.md`). See `docs/product/FILE-FORMAT.md`.
//! This crate never runs `git`: a seabag inside a repository is versioned by that
//! repository, and nothing here creates a nested one.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use fs4::{FileExt, TryLockError};

use fliphopper_core::{
    Area, FORMAT, Issue, Iteration, Loaded, Problem, Project, Seabag, Task, parse_area,
    parse_iteration, parse_project, parse_task, validate_seabag,
};
use serde::Serialize;

mod confirm;
mod convert;
mod guide;
mod setup;
pub use confirm::{
    ConfirmRequest, MAX_PLAN_BYTES, REQUESTS_DIR, answer_confirm, pending_confirms, post_confirm,
    read_answer, requests_dir, was_seen, withdraw_confirm,
};
pub use convert::{CONVERT_REQUEST_VERSION, agent_convert_request_in};
pub use guide::{GUIDE_TOPICS, guide};
pub use setup::{
    SetupState, fliphopper_folder, project_folder_of, record_project_folder, relative_path,
    setup_state,
};

/// Where a project's seabag lives inside the folder the user picked.
/// Always used for a new seabag, also in an empty folder, so every tool looks in one place.
pub const DEFAULT_DIR: &str = ".fliphopper/seabag";
const PROJECT_FILE: &str = "project.json";
/// The file every writer locks for the length of one change. Never committed.
pub const LOCK_FILE: &str = ".lock";
/// Prefix of the temporary files and folders a writer renames into place.
const TMP_PREFIX: &str = ".tmp-";
/// Written into every new seabag, so Git never sees FlipHopper's working files.
const GITIGNORE: &str = "# FlipHopper's own working files. Never commit them.\n.lock\n.tmp-*\n";

/// What a folder the user picked turns out to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Inspection {
    /// The folder is a seabag, or contains one in `.fliphopper/seabag/`.
    Found { root: PathBuf },
    /// The folder looks empty (nothing, or only hidden entries such as `.DS_Store` or `.git`):
    /// a starter seabag can be created in its `.fliphopper/seabag/` without asking.
    Empty {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The folder has other content but no seabag: one can be created in `.fliphopper/seabag/`.
    NoSeabag {
        folder: PathBuf,
        suggested_root: PathBuf,
    },
    /// The path does not exist or is not a folder.
    NotAFolder { path: PathBuf },
}

/// Name of the short introduction FlipHopper leaves in a folder it fills from empty.
/// Never `README.md`, so it cannot clash with the user's own README (owner, 2026-10-06).
pub const README_FILE: &str = "README-fliphopper.md";

/// A folder looks empty when it has no entries, or only hidden ones (names starting with
/// `.`, such as `.DS_Store`, `.git` or `.fliphopper`). A folder that cannot be read does not.
pub fn looks_empty(folder: &Path) -> bool {
    fs::read_dir(folder)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .all(|e| e.file_name().to_string_lossy().starts_with('.'))
        })
        .unwrap_or(false)
}

/// The README's text in an interface language (`en`, `zh-CN`, `ja`); English otherwise.
pub fn readme_text(language: &str) -> &'static str {
    match language {
        "zh-CN" => README_ZH_CN,
        "ja" => README_JA,
        _ => README_EN,
    }
}

const README_EN: &str = "# FlipHopper project

This folder is a FlipHopper project. Its tasks, areas and progress are in
`.fliphopper/seabag/`, a hidden folder.

- Open this folder in the FlipHopper app to see the tasks by area.
- AI agents and people can also work the tasks from a terminal with the `fh` command
  (`fh list`, `fh next`, `fh claim <task>`).
- The files are plain JSON and Markdown, so Git can track every change.

FlipHopper wrote this file because the folder was empty. You may edit or delete it.
";

const README_ZH_CN: &str = "# FlipHopper 项目

此文件夹是一个 FlipHopper 项目。它的任务、领域和进度保存在隐藏文件夹
`.fliphopper/seabag/` 中。

- 在 FlipHopper 应用中打开此文件夹，即可按领域查看任务。
- AI 智能体和人也可以在终端中用 `fh` 命令处理任务
  （`fh list`、`fh next`、`fh claim <任务>`）。
- 这些文件是普通的 JSON 和 Markdown，Git 可以记录每一处更改。

此文件夹原本是空的，所以 FlipHopper 写了这个文件。你可以修改或删除它。
";

const README_JA: &str = "# FlipHopper プロジェクト

このフォルダーは FlipHopper のプロジェクトです。タスク、エリア、進捗は隠しフォルダー
`.fliphopper/seabag/` に入っています。

- FlipHopper アプリでこのフォルダーを開くと、タスクをエリアごとに見られます。
- AI エージェントや人は、ターミナルの `fh` コマンドでもタスクを扱えます
  （`fh list`、`fh next`、`fh claim <タスク>`）。
- ファイルはただの JSON と Markdown なので、Git ですべての変更を記録できます。

このフォルダーが空だったので、FlipHopper がこのファイルを書きました。編集や削除は自由です。
";

/// Write `README-fliphopper.md` into `folder` when the folder looks empty (see
/// [`looks_empty`]). Never overwrites a file. Returns whether the README was written.
pub fn add_readme_if_empty(folder: &Path, language: &str) -> io::Result<bool> {
    if !looks_empty(folder) {
        return Ok(false);
    }
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join(README_FILE))
    {
        Ok(mut file) => {
            file.write_all(readme_text(language).as_bytes())?;
            Ok(true)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// The heading of FlipHopper's section in a project's `AGENTS.md` / `CLAUDE.md`.
pub const AGENT_SECTION_HEADING: &str = "## Tasks: FlipHopper";

/// The section that tells an AI agent this project's tasks are in FlipHopper. English: it is
/// read by agents, not shown in the app.
pub fn agent_section() -> String {
    format!(
        "{AGENT_SECTION_HEADING}

This project's tasks, progress and handoff notes live in FlipHopper's seabag
(`.fliphopper/seabag/`). Work them with the `fh` command; never edit those files by hand.

- Start a session with `fh next` (the next ready task and its handoff note), or `fh list`.
- Take a task: `fh claim <id> --as <your session name> --push`.
- Report progress: `fh label <id> \"<label>\" --push`; when blocked: `fh wait <id> --on \"<what>\" --push`.
- Stopping before it is done: `fh release <id> --note \"<what the next one needs>\" --push`.
- Finished: `fh done <id> --push`.
- Adding a task: read `fh guide tasks` first, then `fh add task --area <area> --title \"<outcome>\" --push`.

`--push` commits only the seabag's files and pushes them, so others see the change.

FlipHopper relies on these terms: **area** (what part of the product a task belongs to),
**iteration** (when it is planned), **label** (the step of this project's progress ladder a task
has proven) and **state** (backlog, ready, in progress, waiting, done, dropped). When you need
more, ask `fh` rather than guessing: `fh guide terms` (what they mean), `fh guide customize`
(what this project may change and what is fixed), `fh guide commands`, `fh guide workflow`.
"
    )
}

/// The line before the section's copy in a request: the agent takes the section from
/// `fh brief`, so it always matches the installed `fh`; the copy is for when `fh` cannot run
/// where the agent works.
pub(crate) fn section_source(language: &str, seabag: &Path) -> String {
    let brief = format!("fh --seabag \"{}\" brief", seabag.display());
    match language {
        "zh-CN" => format!(
            "这一节的内容：运行 `{brief}`，把它的输出原样（英文）作为这一节。只有在那里无法运行 `fh` 时，才使用下面的副本："
        ),
        "ja" => format!(
            "節の内容：`{brief}` を実行し、その出力をそのまま（英語のまま）この節にします。そこで `fh` を実行できない場合だけ、下の写しを使います："
        ),
        _ => format!(
            "The section's text: run `{brief}` and use its output exactly as it is. Only if `fh` \
cannot run there, use this copy:"
        ),
    }
}

/// [`agent_section`], plus where the seabag is when it is not inside `project` (written relative
/// to `project`, so the file works on every computer that has both folders side by side).
pub fn agent_section_for(project: &Path, seabag: &Path) -> String {
    let section = agent_section();
    if seabag.starts_with(project) {
        return section;
    }
    let at = relative_path(project, seabag)
        .unwrap_or_else(|| seabag.to_path_buf())
        .display()
        .to_string();
    format!(
        "{section}
This project's seabag is not inside this folder: it is at `{at}`. Run every `fh` command with
`--seabag \"{at}\"` (or set `FLIPHOPPER_SEABAG` to that path).
"
    )
}

/// What `fh init` (and later the app) asks an AI agent to do: add [`agent_section`] to the
/// project's instruction files, merged with what is already there. Plain numbered steps, so any
/// agent can follow them.
pub fn agent_setup_request(project: &Path, seabag: &Path) -> String {
    agent_setup_request_in("en", project, seabag)
}

/// [`agent_setup_request`] in an interface language (`en`, `zh-CN`, `ja`; English otherwise).
/// The steps follow the language; the section itself stays English, so every agent and teammate
/// reads the same rules.
pub fn agent_setup_request_in(language: &str, project_path: &Path, seabag_path: &Path) -> String {
    let (project, seabag, heading) = (
        project_path.display(),
        seabag_path.display(),
        AGENT_SECTION_HEADING,
    );
    let source = section_source(language, seabag_path);
    let steps = match language {
        "zh-CN" => format!(
            "FlipHopper 已为这个项目建立了水手袋。现在请把它告诉以后的 AI 智能体。

项目文件夹：{project}
水手袋（FlipHopper 的任务文件）：{seabag}

在项目文件夹中：

1. 打开 AGENTS.md（如果没有就新建）。
2. 如果 AGENTS.md 已经有关于 FlipHopper 的说明（例如标题为“{heading}”的一节，或其他关于 FlipHopper 或 `fh` 的文字），用下面这一节替换它们；否则添加下面这一节。
3. 如果 AGENTS.md 中有其他关于跟踪任务或状态的规则（例如 Markdown 任务看板），把它们改为指向 FlipHopper，避免规则冲突。不要改动无关内容，也不要重复添加相同的说明。
4. 确保 CLAUDE.md 包含一行 @AGENTS.md，让 Claude Code 读取 AGENTS.md（如果没有 CLAUDE.md，就新建一个只含这一行的文件）。不要把这一节复制到 CLAUDE.md。
5. 告诉用户你改了什么。

{source}"
        ),
        "ja" => format!(
            "FlipHopper がこのプロジェクトのシーバッグを用意しました。これからの AI エージェントにそれを伝えてください。

プロジェクトのフォルダー：{project}
シーバッグ（FlipHopper のタスクファイル）：{seabag}

プロジェクトのフォルダーで：

1. AGENTS.md を開きます（なければ作成します）。
2. AGENTS.md にすでに FlipHopper の説明（たとえば「{heading}」という見出しの節や、FlipHopper または `fh` についてのほかの文）があれば、下の節で置き換えます。なければ下の節を追加します。
3. AGENTS.md にタスクや状況の管理についてのほかのルール（たとえば Markdown のタスクボード）があれば、ルールが食い違わないよう FlipHopper を指すように書き換えます。関係のない部分は変えず、同じ説明を二重に追加しないでください。
4. CLAUDE.md に @AGENTS.md という行があることを確かめます（Claude Code が AGENTS.md を読むためです。CLAUDE.md がなければその一行だけで作成します）。この節を CLAUDE.md にコピーしないでください。
5. 何を変えたかをユーザーに見せてください。

{source}"
        ),
        _ => format!(
            "FlipHopper has set up this project's seabag. Please now tell future AI agents about it.

Project folder: {project}
Seabag (FlipHopper's task files): {seabag}

In the project folder:

1. Open AGENTS.md (create it if it is missing).
2. If AGENTS.md already has FlipHopper instructions (for example a section headed \
\"{heading}\", or any other text about FlipHopper or `fh`), replace them with the \
section below. Otherwise add the section below.
3. If AGENTS.md has other rules about tracking tasks or status (for example a Markdown task \
board), change them to point to FlipHopper, so the rules do not conflict. Do not change \
unrelated text, and do not add the same instructions twice.
4. Make sure CLAUDE.md contains the line @AGENTS.md, so Claude Code reads AGENTS.md (create \
CLAUDE.md with just that line if it is missing). Do not copy the section into CLAUDE.md.
5. Show the user what you changed.

{source}"
        ),
    };
    format!(
        "{steps}\n\n{}",
        agent_section_for(project_path, seabag_path)
    )
}

/// Add FlipHopper's section for AI agents to `folder`'s `AGENTS.md` and `CLAUDE.md`, never
/// changing what is already there. `AGENTS.md`: created with the section, or the section is
/// appended unless it is there. `CLAUDE.md`: created as `@AGENTS.md` (Claude Code then reads
/// AGENTS.md); an existing one is left alone if it imports `@AGENTS.md` or holds the section,
/// otherwise the section is appended. Returns the names of the files it changed.
pub fn add_agent_instructions(folder: &Path) -> io::Result<Vec<String>> {
    let mut changed = Vec::new();
    let section = agent_section();
    let agents = folder.join("AGENTS.md");
    match fs::read_to_string(&agents) {
        Ok(text) if tells_agents(&text) => {}
        Ok(text) => {
            append(&agents, &text, &section)?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&agents, format!("# Agent instructions\n\n{section}"))?;
            changed.push("AGENTS.md".to_string());
        }
        Err(e) => return Err(e),
    }
    let claude = folder.join("CLAUDE.md");
    match fs::read_to_string(&claude) {
        Ok(text) if tells_agents(&text) || imports_agents_md(&text) => {}
        Ok(text) => {
            append(&claude, &text, &section)?;
            changed.push("CLAUDE.md".to_string());
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            fs::write(&claude, "@AGENTS.md\n")?;
            changed.push("CLAUDE.md".to_string());
        }
        Err(e) => return Err(e),
    }
    Ok(changed)
}

/// Whether an instruction file's text tells AI agents about FlipHopper: a Markdown heading names
/// it, or the text mentions `fh next` or `fh claim`, or `.fliphopper/seabag`. Agents may merge the
/// section in their own words, so any of these counts.
pub fn tells_agents(text: &str) -> bool {
    text.contains("fh next")
        || text.contains("fh claim")
        || text.contains(".fliphopper/seabag")
        || text.lines().any(|l| {
            let l = l.trim_start();
            l.starts_with('#') && l.to_lowercase().contains("fliphopper")
        })
}

fn imports_agents_md(text: &str) -> bool {
    text.lines().any(|l| l.trim() == "@AGENTS.md")
}

/// Which AI agents' instruction files in `project` tell them about FlipHopper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AgentSetup {
    /// Codex reads `AGENTS.md`.
    pub codex: bool,
    /// Claude Code reads `CLAUDE.md`, and through `@AGENTS.md` also `AGENTS.md`.
    pub claude: bool,
}

/// Read `project`'s `AGENTS.md` and `CLAUDE.md` and say which agents are told about FlipHopper.
pub fn agent_setup(project: &Path) -> AgentSetup {
    let agents = fs::read_to_string(project.join("AGENTS.md")).unwrap_or_default();
    let claude = fs::read_to_string(project.join("CLAUDE.md")).unwrap_or_default();
    let codex = tells_agents(&agents);
    AgentSetup {
        codex,
        claude: tells_agents(&claude) || (imports_agents_md(&claude) && codex),
    }
}

/// Append `section` to a file whose current text is `text`, after one blank line.
fn append(path: &Path, text: &str, section: &str) -> io::Result<()> {
    let gap = if text.is_empty() || text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    let mut file = fs::OpenOptions::new().append(true).open(path)?;
    file.write_all(format!("{gap}{section}").as_bytes())
}

/// Where a link in a task's description leads, once FlipHopper has agreed to open it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkTarget {
    /// An `http` or `https` address, for the web browser.
    Web(String),
    /// A text file inside the project, for the user's editor.
    File(PathBuf),
}

/// File types a description link may open: text the user reads or edits, never a program.
const LINKABLE: &[&str] = &["md", "markdown", "txt"];

/// Check a link from task `task`'s `description.md` in the seabag at `root`. Web addresses
/// (`http`, `https`) are allowed. A relative path is read from the task's folder, the way a
/// Markdown viewer would, and must lead to an existing `.md`, `.markdown` or `.txt` file inside
/// the project folder (the folder holding `.fliphopper/`, or the seabag itself when it stands
/// alone). Everything else is refused, so a description cannot be used to start a program.
pub fn resolve_task_link(root: &Path, task: &str, link: &str) -> Result<LinkTarget, Problem> {
    let refused = || Problem::LinkRefused {
        link: link.to_string(),
    };
    let lower = link.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Ok(LinkTarget::Web(link.to_string()));
    }
    // Any other scheme (`file:`, `mailto:`, `javascript:` …) and absolute paths are refused.
    if link.is_empty() || link.contains(':') || link.starts_with('/') || link.starts_with('\\') {
        return Err(refused());
    }
    if !fliphopper_core::ops::valid_id(task) {
        return Err(refused());
    }
    // A link may point into a file (`card.md#approvals`); the part after `#` is not a file name.
    let path = link.split('#').next().unwrap_or_default();
    let project = picked_folder(root).unwrap_or(root);
    let target = root.join("tasks").join(task).join(path);
    let (Ok(project), Ok(target)) = (project.canonicalize(), target.canonicalize()) else {
        return Err(Problem::LinkNotFound {
            link: link.to_string(),
        });
    };
    let text = target
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| LINKABLE.contains(&e.to_ascii_lowercase().as_str()));
    if !target.starts_with(&project) || !target.is_file() || !text {
        return Err(refused());
    }
    Ok(LinkTarget::File(target))
}

/// The folder the user picked, for a seabag at `<folder>/.fliphopper/seabag`; `None` for a
/// seabag anywhere else.
pub fn picked_folder(root: &Path) -> Option<&Path> {
    if root.ends_with(DEFAULT_DIR) {
        root.parent()?.parent()
    } else {
        None
    }
}

/// Work out what `folder` is, without changing anything.
pub fn inspect(folder: &Path) -> Inspection {
    if !folder.is_dir() {
        return Inspection::NotAFolder {
            path: folder.to_path_buf(),
        };
    }
    if folder.join(PROJECT_FILE).is_file() {
        return Inspection::Found {
            root: folder.to_path_buf(),
        };
    }
    let nested = folder.join(DEFAULT_DIR);
    if nested.join(PROJECT_FILE).is_file() {
        return Inspection::Found { root: nested };
    }
    if looks_empty(folder) {
        Inspection::Empty {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    } else {
        Inspection::NoSeabag {
            folder: folder.to_path_buf(),
            suggested_root: nested,
        }
    }
}

/// The nearest folder at or above `path` that contains `.git`, if any (shown to the user;
/// never used to run Git here).
pub fn git_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|p| p.join(".git").exists())
        .map(Path::to_path_buf)
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn read(root: &Path, path: &Path, issues: &mut Vec<Issue>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(e) => {
            issues.push(Issue::new(
                &rel(root, path),
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            ));
            None
        }
    }
}

fn hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Sorted entries of a folder, without hidden ones (the lock file and a writer's
/// temporary files); a missing folder is simply empty.
fn sorted_entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(dir)
        .map(|it| {
            it.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| !hidden(p))
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Load a seabag. Valid parts are returned even when other files have problems,
/// so the user sees their tasks plus a list of what to fix.
pub fn load(root: &Path) -> Loaded {
    let mut issues = Vec::new();
    let project_path = root.join(PROJECT_FILE);
    let project = match read(root, &project_path, &mut issues) {
        Some(text) => match parse_project(PROJECT_FILE, &text) {
            Ok(p) => Some(p),
            Err(mut e) => {
                issues.append(&mut e);
                None
            }
        },
        None => None,
    };
    let Some(project) = project else {
        return Loaded {
            seabag: None,
            issues,
        };
    };

    let mut areas: Vec<(String, Area)> = Vec::new();
    for path in sorted_entries(&root.join("areas")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_area(&file, &text) {
                Ok(a) => areas.push((file, a)),
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    // Optional: a seabag without an `iterations/` folder simply has none.
    let mut iterations: Vec<(String, Iteration)> = Vec::new();
    for path in sorted_entries(&root.join("iterations")) {
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let file = rel(root, &path);
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_iteration(&file, &text) {
                Ok(i) => iterations.push((file, i)),
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    let mut tasks: Vec<(String, Task)> = Vec::new();
    for dir in sorted_entries(&root.join("tasks")) {
        if !dir.is_dir() {
            continue;
        }
        let path = dir.join("task.json");
        let file = rel(root, &path);
        if !path.is_file() {
            issues.push(Issue::new(&file, None, Problem::TaskFileMissing));
            continue;
        }
        if let Some(text) = read(root, &path, &mut issues) {
            match parse_task(&file, &text) {
                Ok(mut t) => {
                    t.description =
                        fs::read_to_string(dir.join("description.md")).unwrap_or_default();
                    tasks.push((file, t));
                }
                Err(mut e) => issues.append(&mut e),
            }
        }
    }

    issues.extend(validate_seabag(&project, &areas, &iterations, &tasks));
    let mut areas: Vec<Area> = areas.into_iter().map(|(_, a)| a).collect();
    areas.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let mut iterations: Vec<Iteration> = iterations.into_iter().map(|(_, i)| i).collect();
    iterations.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));
    let tasks = tasks.into_iter().map(|(_, t)| t).collect();
    Loaded {
        seabag: Some(Seabag {
            project,
            areas,
            iterations,
            tasks,
        }),
        issues,
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
    text.push('\n');
    fs::write(path, text)
}

/// The JSON stored in `task.json`: the task without its description (that lives in
/// `description.md`) and without empty optional fields.
fn task_file_json(task: &Task) -> serde_json::Value {
    let mut v = serde_json::to_value(task).expect("a task always serializes");
    if let Some(map) = v.as_object_mut() {
        map.remove("description");
        map.retain(|_, value| !value.is_null());
    }
    v
}

/// Create a small starter project at `root`. `root` must not exist yet, or be an empty folder.
/// Returns `root`.
pub fn create_starter(root: &Path, name: &str) -> io::Result<PathBuf> {
    if root.exists() && fs::read_dir(root)?.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is not empty", root.display()),
        ));
    }
    fs::create_dir_all(root)?;
    fs::write(root.join(".gitignore"), GITIGNORE)?;
    let project = Project {
        format: FORMAT.to_string(),
        id: "project".to_string(),
        name: name.to_string(),
        labels: fliphopper_core::default_labels(),
        project_folder: None,
    };
    write_json(&root.join(PROJECT_FILE), &project)?;
    let areas = [("A01", "Getting started", 1), ("A02", "First feature", 2)];
    for (id, name, order) in areas {
        write_json(
            &root.join("areas").join(format!("{id}.json")),
            &Area {
                id: id.into(),
                name: name.into(),
                order,
            },
        )?;
    }
    let tasks = [
        (
            "A01-01",
            "Open this seabag in FlipHopper",
            "A01",
            fliphopper_core::State::Done,
            Some("owner verified"),
            "You picked a folder and FlipHopper created this starter project in it.\n",
        ),
        (
            "A01-02",
            "Rename the areas and tasks to your project's",
            "A01",
            fliphopper_core::State::Ready,
            None,
            "Edit the files in this folder, or use the `fh` command once it is available.\n",
        ),
        (
            "A02-01",
            "Describe the first outcome you want",
            "A02",
            fliphopper_core::State::Backlog,
            None,
            "One sentence about what a person can do when this task is done.\n",
        ),
    ];
    for (id, title, area, state, label, description) in tasks {
        let dir = root.join("tasks").join(id);
        let task = Task {
            id: id.into(),
            title: title.into(),
            area: area.into(),
            state,
            label: label.map(String::from),
            holder: None,
            waiting_on: None,
            next_dependency: None,
            iteration: None,
            handoff_note: None,
            description: String::new(),
        };
        write_json(&dir.join("task.json"), &task_file_json(&task))?;
        fs::write(dir.join("description.md"), description)?;
    }
    Ok(root.to_path_buf())
}

/// Find the seabag for `start`: the nearest `.fliphopper/seabag/` at or above it,
/// the way Git finds its repository.
pub fn find_seabag(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|p| p.join(DEFAULT_DIR))
        .find(|p| p.join(PROJECT_FILE).is_file())
}

/// A project ID made from a folder name: letters, digits, `-` and `_` kept, anything else
/// turned into `-`, at most 64 characters; `project` when nothing usable is left.
pub fn id_from_name(name: &str) -> String {
    let id: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .take(64)
        .collect();
    let id = id.trim_matches('-').to_string();
    if id.is_empty() {
        "project".to_string()
    } else {
        id
    }
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// A name for a temporary file or folder next to `dir`'s contents, unique to this process.
fn tmp_path(dir: &Path, what: &str) -> PathBuf {
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!("{TMP_PREFIX}{}-{n}-{what}", std::process::id()))
}

fn write_failed(path: &Path, e: impl std::fmt::Display) -> Problem {
    Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    }
}

fn json_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).expect("seabag values always serialize");
    text.push('\n');
    text
}

/// Write `path` whole: the text goes to a temporary file in the same folder, is flushed to
/// disk, and is then renamed over `path`. A reader sees either the old file or the new one,
/// never a part-written one.
fn write_atomic(path: &Path, text: &str) -> Result<(), Problem> {
    let dir = path.parent().expect("seabag files are inside a folder");
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp = tmp_path(dir, name);
    let result = (|| {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(text.as_bytes())?;
        f.sync_all()?;
        fs::rename(&tmp, path)
    })();
    result.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        write_failed(path, e)
    })
}

/// Fill a new folder with files, then rename it to `target` in one step, so a reader never
/// sees it half-made. Refused when `target` already exists.
fn create_dir_atomic(target: &Path, files: &[(&str, String)]) -> Result<(), Problem> {
    let parent = target.parent().expect("a target has a parent");
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("dir");
    let tmp = tmp_path(parent, name);
    let result = (|| {
        fs::create_dir_all(&tmp)?;
        for (rel_path, text) in files {
            let path = tmp.join(rel_path);
            if let Some(dir) = path.parent() {
                fs::create_dir_all(dir)?;
            }
            let mut f = fs::File::create(&path)?;
            f.write_all(text.as_bytes())?;
            f.sync_all()?;
        }
        if target.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "already exists",
            ));
        }
        fs::rename(&tmp, target)
    })();
    result.map_err(|e| {
        let _ = fs::remove_dir_all(&tmp);
        write_failed(target, e)
    })
}

/// Create an empty seabag at `root` (normally `<repository>/.fliphopper/seabag/`): `project.json`,
/// empty `areas/` and `tasks/`, and a `.gitignore` for the lock. Refused when `root` exists.
/// Never creates a Git repository.
pub fn init(root: &Path, id: &str, name: &str) -> Result<PathBuf, Problem> {
    init_with_labels(root, id, name, fliphopper_core::default_labels())
}

/// [`init`] with the project's own label ladder, lowest step first. An empty ladder is
/// refused (`ladder_empty`), like an invalid ID.
pub fn init_with_labels(
    root: &Path,
    id: &str,
    name: &str,
    labels: Vec<String>,
) -> Result<PathBuf, Problem> {
    if root.exists() {
        return Err(Problem::SeabagExists {
            path: root.display().to_string(),
        });
    }
    let project = Project {
        format: FORMAT.to_string(),
        id: id.to_string(),
        name: name.to_string(),
        labels,
        project_folder: None,
    };
    // Checked like any project file, so `init` never writes one that `load` refuses.
    let text = json_text(&project);
    if let Err(issues) = parse_project(PROJECT_FILE, &text) {
        return Err(issues[0].problem.clone());
    }
    let parent = root.parent().expect("a seabag root has a parent");
    fs::create_dir_all(parent).map_err(|e| write_failed(parent, e))?;
    create_dir_atomic(
        root,
        &[
            (PROJECT_FILE, text),
            (".gitignore", GITIGNORE.to_string()),
            ("areas/.keep", String::new()),
            ("tasks/.keep", String::new()),
        ],
    )
    .map_err(|p| match p {
        Problem::WriteFailed { .. } if root.exists() => Problem::SeabagExists {
            path: root.display().to_string(),
        },
        p => p,
    })?;
    Ok(root.to_path_buf())
}

/// The seabag's lock, held for one change: load, check the rules, write, then drop it.
/// Every write goes through a `Lock`, so two FlipHopper writers never change the seabag at
/// the same moment. The operating system releases the lock if the process dies.
pub struct Lock {
    root: PathBuf,
    _file: fs::File,
}

impl Lock {
    /// Take the lock, waiting up to `wait` for another writer to finish.
    pub fn acquire(root: &Path, wait: Duration) -> Result<Lock, Problem> {
        if !root.join(PROJECT_FILE).is_file() {
            return Err(Problem::SeabagNotFound {
                path: root.display().to_string(),
            });
        }
        let path = root.join(LOCK_FILE);
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .map_err(|e| write_failed(&path, e))?;
        let start = Instant::now();
        loop {
            match FileExt::try_lock(&file) {
                Ok(()) => {
                    return Ok(Lock {
                        root: root.to_path_buf(),
                        _file: file,
                    });
                }
                Err(TryLockError::WouldBlock) if start.elapsed() < wait => {
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(Problem::SeabagBusy {
                        path: root.display().to_string(),
                    });
                }
                Err(TryLockError::Error(e)) => return Err(write_failed(&path, e)),
            }
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Load the seabag while holding the lock, so what is read is what the change builds on.
    pub fn load(&self) -> Loaded {
        load(&self.root)
    }

    /// Write a new area file. Refused when the file already exists.
    pub fn create_area(&self, area: &Area) -> Result<(), Problem> {
        let dir = self.root.join("areas");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", area.id));
        if path.exists() {
            return Err(Problem::DuplicateAreaId {
                id: area.id.clone(),
            });
        }
        write_atomic(&path, &json_text(area))
    }

    /// Write a new iteration file. Refused when the file already exists.
    pub fn create_iteration(&self, iteration: &Iteration) -> Result<(), Problem> {
        let dir = self.root.join("iterations");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let path = dir.join(format!("{}.json", iteration.id));
        if path.exists() {
            return Err(Problem::DuplicateIterationId {
                id: iteration.id.clone(),
            });
        }
        write_atomic(&path, &json_text(iteration))
    }

    /// Write a new task folder: `task.json`, plus `description.md` when there is a description.
    pub fn create_task(&self, task: &Task) -> Result<(), Problem> {
        let dir = self.root.join("tasks");
        fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
        let target = dir.join(&task.id);
        if target.exists() {
            return Err(Problem::DuplicateTaskId {
                id: task.id.clone(),
            });
        }
        let mut files = vec![("task.json", json_text(&task_file_json(task)))];
        if !task.description.is_empty() {
            files.push(("description.md", task.description.clone()));
        }
        create_dir_atomic(&target, &files)
    }

    /// Rewrite `project.json` (`fh labels`).
    pub fn save_project(&self, project: &Project) -> Result<(), Problem> {
        write_atomic(&self.root.join(PROJECT_FILE), &json_text(project))
    }

    /// Rewrite an existing task's `task.json`. Its description is left as it is.
    pub fn save_task(&self, task: &Task) -> Result<(), Problem> {
        let path = self.root.join("tasks").join(&task.id).join("task.json");
        write_atomic(&path, &json_text(&task_file_json(task)))
    }
}

/// A fingerprint of the seabag's files (names, sizes and change times). It changes when any
/// file is written, so a window can reload only when something changed.
pub fn stamp(root: &Path) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    let mut add = |p: &Path| {
        if let Ok(m) = fs::metadata(p) {
            p.hash(&mut h);
            m.len().hash(&mut h);
            m.modified().ok().hash(&mut h);
        }
    };
    add(&root.join(PROJECT_FILE));
    for folder in ["areas", "iterations"] {
        for p in sorted_entries(&root.join(folder)) {
            add(&p);
        }
    }
    for dir in sorted_entries(&root.join("tasks")) {
        add(&dir);
        for p in sorted_entries(&dir) {
            add(&p);
        }
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("fliphopper-test-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn init_with_own_id_and_ladder() {
        let dir = tmp("init-ladder");
        let root = dir.join(DEFAULT_DIR);
        let ladder = vec!["drafted".to_string(), "owner verified".to_string()];
        init_with_labels(&root, "fliphopper", "FlipHopper", ladder.clone()).unwrap();
        let bag = load(&root).seabag.unwrap();
        assert_eq!(
            (bag.project.id.as_str(), bag.project.labels),
            ("fliphopper", ladder)
        );
        let other = dir.join("other");
        assert_eq!(
            init_with_labels(&other, "p", "P", vec![]),
            Err(Problem::LadderEmpty)
        );
        assert!(!other.exists());
        assert!(matches!(
            init_with_labels(&dir.join("x"), "bad id", "P", vec!["a".into()]),
            Err(Problem::InvalidId { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn agent_setup_request_follows_the_language_but_keeps_the_section_english() {
        let (p, b) = (Path::new("/p"), Path::new("/p/.fliphopper/seabag"));
        let zh = agent_setup_request_in("zh-CN", p, b);
        let ja = agent_setup_request_in("ja", p, b);
        assert!(zh.contains("1. 打开 AGENTS.md") && zh.contains("项目文件夹：/p\n"));
        assert!(
            ja.contains("1. AGENTS.md を開きます")
                && ja.contains("シーバッグ（FlipHopper のタスクファイル）：/p/.fliphopper/seabag")
        );
        for r in [&zh, &ja] {
            assert!(r.ends_with(&agent_section()), "the section stays English");
        }
        assert_eq!(
            agent_setup_request_in("fr", p, b),
            agent_setup_request(p, b)
        );
    }

    #[test]
    fn agent_setup_request_carries_the_section_and_steps() {
        let r = agent_setup_request(Path::new("/p"), Path::new("/p/.fliphopper/seabag"));
        assert!(r.contains("1. Open AGENTS.md") && r.contains("@AGENTS.md"));
        assert!(
            r.contains("Project folder: /p\n")
                && r.contains("task files): /p/.fliphopper/seabag\n")
        );
        assert!(r.contains("any other text about FlipHopper") && !r.contains("<!--"));
        assert!(r.ends_with(&agent_section()));
        assert!(r.contains("brief`"), "the section comes from `fh brief`");
        assert_eq!(r.matches(AGENT_SECTION_HEADING).count(), 2);
    }

    #[test]
    fn agent_setup_reads_the_instruction_files() {
        let dir = tmp("agent-setup");
        let none = AgentSetup {
            codex: false,
            claude: false,
        };
        assert_eq!(agent_setup(&dir), none);
        // Unrelated instructions do not count.
        fs::write(dir.join("AGENTS.md"), "# Rules\nRun the tests.\n").unwrap();
        assert_eq!(agent_setup(&dir), none);
        // Merged in other words: `fh next` counts; CLAUDE.md importing AGENTS.md follows it.
        fs::write(dir.join("AGENTS.md"), "## Workflow\nStart with fh next.\n").unwrap();
        fs::write(dir.join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
        assert_eq!(
            agent_setup(&dir),
            AgentSetup {
                codex: true,
                claude: true
            }
        );
        // CLAUDE.md alone, mentioning the seabag; AGENTS.md without it.
        fs::write(dir.join("AGENTS.md"), "# Rules\n").unwrap();
        fs::write(dir.join("CLAUDE.md"), "Tasks are in .fliphopper/seabag/.\n").unwrap();
        assert_eq!(
            agent_setup(&dir),
            AgentSetup {
                codex: false,
                claude: true
            }
        );
        // A heading naming FlipHopper.
        fs::write(dir.join("AGENTS.md"), "### Using FlipHopper\n").unwrap();
        assert!(agent_setup(&dir).codex);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn agent_instructions_are_added_once_and_keep_existing_text() {
        // No files: both created; CLAUDE.md only imports AGENTS.md.
        let dir = tmp("agents-new");
        assert_eq!(
            add_agent_instructions(&dir).unwrap(),
            ["AGENTS.md", "CLAUDE.md"]
        );
        let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents.contains(AGENT_SECTION_HEADING) && agents.contains("fh next"));
        assert!(!agents.contains("<!--"));
        assert_eq!(
            fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
            "@AGENTS.md\n"
        );
        // A second run changes nothing.
        assert!(add_agent_instructions(&dir).unwrap().is_empty());
        assert_eq!(fs::read_to_string(dir.join("AGENTS.md")).unwrap(), agents);
        fs::remove_dir_all(dir).unwrap();

        // Existing files without the section: it is appended once, earlier text kept.
        let dir = tmp("agents-old");
        fs::write(dir.join("AGENTS.md"), "# Ours\nKeep this.").unwrap();
        fs::write(dir.join("CLAUDE.md"), "Claude rules\n").unwrap();
        assert_eq!(
            add_agent_instructions(&dir).unwrap(),
            ["AGENTS.md", "CLAUDE.md"]
        );
        let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
        assert!(agents.starts_with("# Ours\nKeep this.\n\n## Tasks: FlipHopper"));
        let claude = fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
        assert!(claude.starts_with("Claude rules\n\n## Tasks: FlipHopper"));
        assert_eq!(agents.matches(AGENT_SECTION_HEADING).count(), 1);
        assert!(add_agent_instructions(&dir).unwrap().is_empty());
        fs::remove_dir_all(dir).unwrap();

        // A FlipHopper section under another heading (no markers) is recognised too.
        let dir = tmp("agents-own-heading");
        fs::write(
            dir.join("AGENTS.md"),
            "# Ours\n\n## Using FlipHopper\nRun fh next.\n",
        )
        .unwrap();
        assert_eq!(add_agent_instructions(&dir).unwrap(), ["CLAUDE.md"]);
        fs::remove_dir_all(dir).unwrap();

        // CLAUDE.md that already imports AGENTS.md is left alone.
        let dir = tmp("agents-import");
        fs::write(dir.join("CLAUDE.md"), "# Claude\n@AGENTS.md\n").unwrap();
        assert_eq!(add_agent_instructions(&dir).unwrap(), ["AGENTS.md"]);
        assert_eq!(
            fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
            "# Claude\n@AGENTS.md\n"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn hidden_only_folder_looks_empty() {
        let dir = tmp("hidden-only");
        assert!(looks_empty(&dir));
        fs::write(dir.join(".DS_Store"), "x").unwrap();
        fs::create_dir(dir.join(".git")).unwrap();
        assert!(looks_empty(&dir));
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: dir.join(DEFAULT_DIR)
            }
        );
        fs::write(dir.join("notes.txt"), "x").unwrap();
        assert!(!looks_empty(&dir));
        assert!(matches!(inspect(&dir), Inspection::NoSeabag { .. }));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_goes_only_into_a_folder_that_looks_empty() {
        // Empty, and only `.DS_Store`: the seabag and the README both arrive.
        for extra in [None, Some(".DS_Store")] {
            let dir = tmp("readme-empty");
            if let Some(name) = extra {
                fs::write(dir.join(name), "x").unwrap();
            }
            let root = dir.join(DEFAULT_DIR);
            create_starter(&root, "Demo").unwrap();
            assert_eq!(picked_folder(&root), Some(dir.as_path()));
            assert!(add_readme_if_empty(&dir, "en").unwrap());
            let text = fs::read_to_string(dir.join(README_FILE)).unwrap();
            assert!(text.contains(".fliphopper/seabag/"));
            assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
            // Now the folder has a visible file: a second call writes nothing.
            assert!(!add_readme_if_empty(&dir, "ja").unwrap());
            assert_eq!(fs::read_to_string(dir.join(README_FILE)).unwrap(), text);
            fs::remove_dir_all(dir).unwrap();
        }
        // A folder with a visible file gets no README.
        let dir = tmp("readme-code");
        fs::write(dir.join("main.rs"), "fn main() {}").unwrap();
        create_starter(&dir.join(DEFAULT_DIR), "Code").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert!(!dir.join(README_FILE).exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_never_overwrites() {
        let dir = tmp("readme-keep");
        fs::create_dir(dir.join(".git")).unwrap();
        let path = dir.join(README_FILE);
        // An existing README is a visible file, so the folder no longer looks empty.
        fs::write(&path, "mine").unwrap();
        assert!(!add_readme_if_empty(&dir, "en").unwrap());
        assert_eq!(fs::read_to_string(&path).unwrap(), "mine");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn readme_speaks_three_languages() {
        assert!(readme_text("en").starts_with("# FlipHopper project"));
        assert!(readme_text("zh-CN").contains("此文件夹是一个 FlipHopper 项目"));
        assert!(readme_text("ja").contains("このフォルダーは FlipHopper のプロジェクトです"));
        assert_eq!(readme_text("fr"), readme_text("en"));
        for lang in ["en", "zh-CN", "ja"] {
            assert!(readme_text(lang).contains("`.fliphopper/seabag/`"));
            assert!(readme_text(lang).contains("`fh"));
        }
        assert_eq!(picked_folder(Path::new("/x/seabag")), None);
    }

    #[test]
    fn empty_folder_gets_starter_seabag_in_default_dir() {
        let dir = tmp("empty");
        let root = dir.join(".fliphopper/seabag");
        assert_eq!(
            inspect(&dir),
            Inspection::Empty {
                folder: dir.clone(),
                suggested_root: root.clone()
            }
        );
        create_starter(&root, "Demo").unwrap();
        assert_eq!(inspect(&dir), Inspection::Found { root: root.clone() });
        let loaded = load(&root);
        assert_eq!(
            loaded.seabag.as_ref().unwrap().project.format,
            "fliphopper/1"
        );
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let tb = loaded.seabag.unwrap();
        assert_eq!(tb.project.name, "Demo");
        assert_eq!(tb.areas.len(), 2);
        assert_eq!(tb.tasks.len(), 3);
        assert!(tb.tasks[0].description.contains("starter"));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn repo_folder_gets_nested_seabag_without_nested_git() {
        let repo = tmp("repo");
        fs::create_dir(repo.join(".git")).unwrap();
        fs::write(repo.join("README.md"), "code").unwrap();
        let Inspection::NoSeabag { suggested_root, .. } = inspect(&repo) else {
            panic!("expected NoSeabag")
        };
        assert_eq!(suggested_root, repo.join(".fliphopper/seabag"));
        create_starter(&suggested_root, "Repo").unwrap();
        assert_eq!(
            inspect(&repo),
            Inspection::Found {
                root: repo.join(".fliphopper/seabag")
            }
        );
        assert!(!suggested_root.join(".git").exists());
        assert_eq!(git_root(&suggested_root), Some(repo.clone()));
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn refuses_to_overwrite_a_non_empty_folder() {
        let dir = tmp("full");
        fs::write(dir.join("notes.txt"), "keep me").unwrap();
        assert!(create_starter(&dir, "X").is_err());
        assert_eq!(
            fs::read_to_string(dir.join("notes.txt")).unwrap(),
            "keep me"
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_file_is_reported_and_valid_tasks_still_load() {
        let dir = tmp("broken");
        create_starter(&dir, "Demo").unwrap();
        fs::write(
            dir.join("tasks/A01-02/task.json"),
            r#"{"id":"A01-02","area":"A01","state":"ready"}"#,
        )
        .unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.issues.len(), 1);
        assert_eq!(loaded.issues[0].file, "tasks/A01-02/task.json");
        assert_eq!(loaded.issues[0].field.as_deref(), Some("title"));
        assert_eq!(loaded.seabag.unwrap().tasks.len(), 2);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_project_file_is_an_issue_not_a_crash() {
        let dir = tmp("noproject");
        let loaded = load(&dir);
        assert!(loaded.seabag.is_none());
        assert_eq!(loaded.issues[0].file, "project.json");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn not_a_folder() {
        let p = tmp("nf").join("missing");
        assert!(matches!(inspect(&p), Inspection::NotAFolder { .. }));
    }

    #[test]
    fn init_makes_an_empty_seabag_found_from_below() {
        let repo = tmp("init");
        let root = repo.join(DEFAULT_DIR);
        init(&root, &id_from_name("my repo!"), "My repo").unwrap();
        assert_eq!(
            init(&root, "x", "X"),
            Err(Problem::SeabagExists {
                path: root.display().to_string()
            })
        );
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.seabag.unwrap();
        assert_eq!(bag.project.id, "my-repo");
        assert!(bag.areas.is_empty() && bag.tasks.is_empty());
        assert!(
            fs::read_to_string(root.join(".gitignore"))
                .unwrap()
                .contains(".lock")
        );
        let deep = repo.join("src/a/b");
        fs::create_dir_all(&deep).unwrap();
        assert_eq!(find_seabag(&deep), Some(root.clone()));
        assert_eq!(find_seabag(&std::env::temp_dir()), None);
        // No temporary folder is left next to the seabag.
        let left: Vec<_> = fs::read_dir(repo.join(".fliphopper")).unwrap().collect();
        assert_eq!(left.len(), 1);
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn id_from_name_keeps_ids_valid() {
        assert_eq!(id_from_name("fliphopper"), "fliphopper");
        assert_eq!(id_from_name("我的"), "project");
        assert_eq!(id_from_name(&"a".repeat(80)).len(), 64);
    }

    #[test]
    fn writes_go_through_the_lock() {
        let dir = tmp("lock");
        let root = dir.join(DEFAULT_DIR);
        assert!(matches!(
            Lock::acquire(&root, Duration::ZERO),
            Err(Problem::SeabagNotFound { .. })
        ));
        init(&root, "p", "P").unwrap();
        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        // A second writer waits, then gives up while the first holds the lock.
        assert!(matches!(
            Lock::acquire(&root, Duration::from_millis(20)),
            Err(Problem::SeabagBusy { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let area = fliphopper_core::ops::new_area(&bag, "A01", "First").unwrap();
        lock.create_area(&area).unwrap();
        assert!(matches!(
            lock.create_area(&area),
            Err(Problem::DuplicateAreaId { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let task =
            fliphopper_core::ops::new_task(&bag, "A01-01", "A01", "Do it", Some("Why")).unwrap();
        lock.create_task(&task).unwrap();
        assert!(matches!(
            lock.create_task(&task),
            Err(Problem::DuplicateTaskId { .. })
        ));
        let bag = lock.load().seabag.unwrap();
        let claimed = fliphopper_core::ops::claim(&bag, "A01-01", Some("me"), false)
            .unwrap()
            .task;
        lock.save_task(&claimed).unwrap();
        drop(lock);
        assert!(Lock::acquire(&root, Duration::ZERO).is_ok());

        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let t = &loaded.seabag.unwrap().tasks[0];
        assert_eq!(t.holder.as_deref(), Some("me"));
        assert_eq!(t.description, "Why\n");
        let json = fs::read_to_string(root.join("tasks/A01-01/task.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("description"));
        assert_ne!(stamp(&root), before);
        // Only the lock and the seabag's own files: no temporary files left behind.
        let hidden: Vec<_> = fs::read_dir(&root)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.starts_with(TMP_PREFIX))
            .collect();
        assert!(hidden.is_empty(), "{hidden:?}");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn iterations_are_written_read_and_sorted() {
        use fliphopper_core::ops::{self, IterationDetails};
        let dir = tmp("iterations");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        // A seabag made before iterations existed has no `iterations/` folder: still valid.
        assert!(!root.join("iterations").exists());
        let loaded = load(&root);
        assert!(loaded.issues.is_empty() && loaded.seabag.unwrap().iterations.is_empty());

        let before = stamp(&root);
        let lock = Lock::acquire(&root, Duration::ZERO).unwrap();
        let bag = lock.load().seabag.unwrap();
        let two = ops::new_iteration(
            &bag,
            "I02",
            "v0.2",
            IterationDetails {
                release: Some("v0.2"),
                ..Default::default()
            },
        )
        .unwrap();
        lock.create_iteration(&two).unwrap();
        assert!(matches!(
            lock.create_iteration(&two),
            Err(Problem::DuplicateIterationId { .. })
        ));
        // Written by hand with a lower order: listed first.
        fs::write(
            root.join("iterations/I01.json"),
            r#"{"id":"I01","name":"v0.1","order":0}"#,
        )
        .unwrap();
        let bag = lock.load().seabag.unwrap();
        lock.create_area(&ops::new_area(&bag, "A01", "First").unwrap())
            .unwrap();
        let bag = lock.load().seabag.unwrap();
        lock.create_task(&ops::new_task(&bag, "A01-01", "A01", "Do it", None).unwrap())
            .unwrap();
        let bag = lock.load().seabag.unwrap();
        lock.save_task(&ops::plan(&bag, "A01-01", Some("I02")).unwrap().task)
            .unwrap();
        drop(lock);

        let json = fs::read_to_string(root.join("iterations/I02.json")).unwrap();
        assert!(!json.contains("null") && !json.contains("start"), "{json}");
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        let bag = loaded.seabag.unwrap();
        let ids: Vec<_> = bag.iterations.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["I01", "I02"]);
        assert_eq!(bag.tasks[0].iteration.as_deref(), Some("I02"));
        assert_ne!(stamp(&root), before);

        // A task planned in a missing iteration is reported, and the task still shows.
        fs::remove_file(root.join("iterations/I02.json")).unwrap();
        let loaded = load(&root);
        assert_eq!(loaded.issues[0].problem.code(), "unknown_iteration");
        assert_eq!(loaded.seabag.unwrap().tasks.len(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn task_links_open_only_text_files_in_the_project_and_web_pages() {
        let dir = tmp("links");
        let root = dir.join(DEFAULT_DIR);
        init(&root, "p", "P").unwrap();
        fs::create_dir_all(dir.join("docs/tasks")).unwrap();
        fs::write(dir.join("docs/tasks/Card.md"), "# Card\n").unwrap();
        fs::write(dir.join("docs/run.sh"), "echo\n").unwrap();
        fs::create_dir_all(root.join("tasks/T1")).unwrap();
        let outside = tmp("links-outside");
        fs::write(outside.join("other.md"), "x").unwrap();

        let card = dir.join("docs/tasks/Card.md").canonicalize().unwrap();
        let file = |link: &str| resolve_task_link(&root, "T1", link);
        assert_eq!(
            file("../../../../docs/tasks/Card.md"),
            Ok(LinkTarget::File(card.clone()))
        );
        assert_eq!(
            file("../../../../docs/tasks/Card.md#approvals"),
            Ok(LinkTarget::File(card))
        );
        assert_eq!(
            file("https://example.com/a"),
            Ok(LinkTarget::Web("https://example.com/a".into()))
        );
        let refused = |link: &str| Err(Problem::LinkRefused { link: link.into() });
        for link in [
            "../../../../docs/run.sh",
            "../../../../docs/tasks",
            "file:///etc/hosts",
            "mailto:a@b.c",
            "javascript:alert(1)",
            "/etc/hosts",
            "",
        ] {
            assert_eq!(file(link), refused(link), "{link}");
        }
        // Outside the project, even as a text file.
        let escape = format!(
            "../../../../../{}/other.md",
            outside.file_name().unwrap().to_str().unwrap()
        );
        assert_eq!(file(&escape), refused(&escape));
        assert_eq!(
            file("../../../../docs/tasks/Gone.md"),
            Err(Problem::LinkNotFound {
                link: "../../../../docs/tasks/Gone.md".into()
            })
        );
        assert!(matches!(
            resolve_task_link(&root, "../x", "a.md"),
            Err(Problem::LinkRefused { .. })
        ));
        fs::remove_dir_all(dir).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }

    #[test]
    fn hidden_entries_are_not_tasks() {
        let dir = tmp("hidden");
        init(&dir.join("s"), "p", "P").unwrap();
        let root = dir.join("s");
        fs::create_dir_all(root.join("tasks/.tmp-1-0-X")).unwrap();
        fs::write(root.join("areas/.tmp-1-1-A.json"), "{").unwrap();
        let loaded = load(&root);
        assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
        fs::remove_dir_all(dir).unwrap();
    }
}
