//! Is this project up to date with this Allomos (Task-A01-25)? Two things carry a version: the
//! taskbag's format (`"format": "allomos/<n>"` in `project.json`) and Allomos's section in the
//! project's `AGENTS.md` / `CLAUDE.md` (its heading, `## Tasks: Allomos (v<n>)`).
//!
//! Allomos only *checks*. It never rewrites the section by itself, and it upgrades a taskbag
//! only when the running agent asks for it with `alm upgrade` (owner, 2026-10-12).

use std::fs;
use std::io;
use std::path::Path;

use allomos_core::{FORMAT, FORMAT_VERSION, Problem, format_version};
use serde::Serialize;
use serde_json::Value;

use crate::{AGENT_SECTION_HEADING, Lock, PROJECT_FILE, picked_folder, tells_agents};

/// The version of Allomos's section that this Allomos writes. Raise it whenever the section's
/// wording changes, so projects with the older wording are asked to renew it.
///
/// 2: the handoff lines (Task-A03-06).
pub const SECTION_VERSION: u32 = 2;

/// How long `alm upgrade` waits for another writer.
const LOCK_WAIT: std::time::Duration = std::time::Duration::from_secs(5);

/// The taskbag's format compared with this Allomos's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum FormatState {
    /// The format this Allomos writes.
    Current,
    /// Written by an older Allomos: the agent runs `alm upgrade`.
    Older { found: String },
    /// Written by a newer Allomos: the person chooses (update Allomos, or let the agent go on).
    Newer { found: String },
    /// No `project.json`, or no `allomos/<n>` format in it: the usual checks report it.
    Unknown,
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

/// Everything the check found. Made only once an agent is connected with the project: the section
/// is in an instruction file and the project folder is known (owner, 2026-10-12). Before that,
/// connecting the agents writes the current section anyway.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VersionCheck {
    /// What this Allomos writes, for messages: `allomos/1`.
    pub format_current: String,
    pub section_current: u32,
    pub format: FormatState,
    /// The instruction files that hold Allomos's section (a `CLAUDE.md` that only imports
    /// `@AGENTS.md` is not listed: it follows `AGENTS.md`).
    pub sections: Vec<SectionState>,
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

    /// Whether the running agent has something to update (the taskbag or the section).
    pub fn needs_update(&self) -> bool {
        matches!(self.format, FormatState::Older { .. }) || !self.outdated_files().is_empty()
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

/// The taskbag's format, read from `project.json` without the usual checks (a newer taskbag may
/// have fields this Allomos does not know).
pub fn format_state(root: &Path) -> FormatState {
    format_state_against(root, FORMAT_VERSION)
}

fn format_state_against(root: &Path, current: u32) -> FormatState {
    let Some(found) = read_format(root) else {
        return FormatState::Unknown;
    };
    match format_version(&found) {
        Some(n) if n == current => FormatState::Current,
        Some(n) if n < current => FormatState::Older { found },
        Some(_) => FormatState::Newer { found },
        None => FormatState::Unknown,
    }
}

fn read_format(root: &Path) -> Option<String> {
    let text = fs::read_to_string(root.join(PROJECT_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    value.get("format")?.as_str().map(String::from)
}

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
        sections,
    }
}

/// The project folder of the taskbag at `root` for the check: the one recorded in
/// `project.json`, else the folder holding `.allomos/` when its instruction files mention
/// Allomos. `None` when no agent is connected yet, so nothing is checked.
pub fn connected_project(root: &Path) -> Option<std::path::PathBuf> {
    let text = fs::read_to_string(root.join(PROJECT_FILE)).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let recorded = value
        .get("project_folder")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let folder = match recorded {
        Some(r) => {
            let base = picked_folder(root).unwrap_or(root);
            normalize(&base.join(r))
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

fn normalize(path: &Path) -> std::path::PathBuf {
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

/// [`check`] for a taskbag, only when an agent is connected ([`connected_project`]).
pub fn check_connected(root: &Path) -> Option<VersionCheck> {
    connected_project(root).map(|project| check(root, &project))
}

/// One step of an upgrade: from format `from` to `from + 1`. `apply` adds what the newer format
/// has, with its defaults, and changes nothing else; it runs under the taskbag's lock and must be
/// safe to run on a taskbag that already has the new parts.
pub struct Step {
    pub from: u32,
    pub apply: fn(&Path) -> io::Result<()>,
}

/// The upgrade steps this Allomos knows, oldest first.
pub const STEPS: &[Step] = &[Step {
    // 1 → 2 (Task-A03-06): the handoff template, project notes and `handoffs/`. The handoff rules
    // in `project.json` are optional, so nothing is added there.
    from: 1,
    apply: crate::handoff::add_missing_files,
}];

/// What `alm upgrade` did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Upgraded {
    pub from: String,
    pub to: String,
    /// False when the taskbag was already in this Allomos's format.
    pub changed: bool,
}

/// Bring the taskbag at `root` up to this Allomos's format, one step at a time. A newer taskbag
/// is refused (`taskbag_newer`) and nothing is written.
pub fn upgrade(root: &Path) -> Result<Upgraded, Problem> {
    upgrade_with(root, STEPS, FORMAT_VERSION)
}

fn upgrade_with(root: &Path, steps: &[Step], current: u32) -> Result<Upgraded, Problem> {
    let lock = Lock::acquire(root, LOCK_WAIT)?;
    let to = format!("allomos/{current}");
    let found = read_format(root).unwrap_or_default();
    let mut version = match format_version(&found) {
        Some(n) => n,
        None => {
            return Err(Problem::WrongFormat {
                expected: to,
                found,
            });
        }
    };
    if version > current {
        return Err(Problem::TaskbagNewer {
            found,
            supported: to,
        });
    }
    // Check the whole way first, so a missing step changes nothing.
    if (version..current).any(|v| !steps.iter().any(|s| s.from == v)) {
        return Err(Problem::UpgradeMissing { found, current: to });
    }
    let changed = version < current;
    let path = lock.root().join(PROJECT_FILE);
    let write_failed = |e: io::Error| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    };
    while version < current {
        let step = steps
            .iter()
            .find(|s| s.from == version)
            .expect("checked above");
        (step.apply)(lock.root()).map_err(write_failed)?;
        version += 1;
        set_format(&path, version)?;
    }
    Ok(Upgraded {
        from: found,
        to,
        changed,
    })
}

/// Write `allomos/<version>` as `project.json`'s format, keeping every other byte as it is (the
/// fields' order too), so the change is one line in a diff.
fn set_format(path: &Path, version: u32) -> Result<(), Problem> {
    let failed = |detail: &str| Problem::WriteFailed {
        path: path.display().to_string(),
        detail: detail.to_string(),
    };
    let text = fs::read_to_string(path).map_err(|e| failed(&e.to_string()))?;
    let key = text
        .find("\"format\"")
        .ok_or_else(|| failed("no format field"))?;
    let colon = key
        + text[key..]
            .find(':')
            .ok_or_else(|| failed("no format value"))?;
    let open = colon
        + text[colon..]
            .find('"')
            .ok_or_else(|| failed("no format value"))?;
    let close = open
        + 1
        + text[open + 1..]
            .find('"')
            .ok_or_else(|| failed("no format value"))?;
    let out = format!(
        "{}\"allomos/{version}\"{}",
        &text[..open],
        &text[close + 1..]
    );
    crate::write_atomic(path, &out)
}

/// What **Update the taskbag…** (and File → Connect with the Agents… while something is out of
/// date) asks the chosen agent, in an interface language (`en`, `zh-CN`, `ja`; English
/// otherwise): run `alm upgrade` when the taskbag is older, and replace Allomos's section with
/// `alm brief`'s output in each out-of-date file. The agent does the work; the person sends the
/// request (owner, 2026-10-12: Allomos never updates these files itself).
pub fn agent_update_request_in(
    language: &str,
    project: &Path,
    taskbag: &Path,
    check: &VersionCheck,
) -> String {
    let alm = format!("alm --taskbag \"{}\"", taskbag.display());
    let (p, t) = (project.display(), taskbag.display());
    let older = matches!(check.format, FormatState::Older { .. });
    let files = check.outdated_files();
    let list = files.join(", ");
    let mut steps: Vec<String> = Vec::new();
    match language {
        "zh-CN" => {
            if older {
                steps.push(format!("运行 `{alm} upgrade`，把任务包升级到这个版本的 Allomos 的格式。它只添加新格式需要的部分；不要手动编辑任务包的文件。"));
            }
            if !files.is_empty() {
                steps.push(format!("在项目文件夹中，把 {list} 里 Allomos 的一节（标题以“{AGENT_SECTION_HEADING}”开头，或其他关于 Allomos 或 `alm` 的说明）替换为 `{alm} brief` 的输出，原样（英文）使用。文件的其他内容保持不变。"));
            }
            steps.push(format!("如果项目是 Git 仓库，只提交这些改动：任务包的文件和 {}。其他改动保持原样；除非用户要求，不要推送。", if files.is_empty() { "（无）".to_string() } else { list.clone() }));
            steps.push("告诉用户你改了什么，以及你做的提交。".to_string());
            let numbered: Vec<String> = steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {s}", i + 1))
                .collect();
            format!(
                "这个项目是用旧版的 Allomos 设置的，请把它更新到当前版本。\n\n项目文件夹：{p}\n任务包（Allomos 的任务文件）：{t}\n\n{}\n",
                numbered.join("\n")
            )
        }
        "ja" => {
            if older {
                steps.push(format!("`{alm} upgrade` を実行して、タスクバッグをこのバージョンの Allomos の形式に更新します。新しい形式に必要な部分だけが追加されます。タスクバッグのファイルを手で編集しないでください。"));
            }
            if !files.is_empty() {
                steps.push(format!("プロジェクトのフォルダーで、{list} の Allomos の節（見出しが「{AGENT_SECTION_HEADING}」で始まるもの、または Allomos や `alm` についてのほかの説明）を `{alm} brief` の出力で置き換え、そのまま（英語のまま）使います。ファイルのほかの部分は変えません。"));
            }
            steps.push(format!("プロジェクトが Git リポジトリなら、これらの変更だけをコミットします：タスクバッグのファイルと {}。ほかの変更はそのままにし、頼まれない限りプッシュしません。", if files.is_empty() { "（なし）".to_string() } else { list.clone() }));
            steps.push(
                "何を変えたか、どんなコミットをしたかをユーザーに見せてください。".to_string(),
            );
            let numbered: Vec<String> = steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {s}", i + 1))
                .collect();
            format!(
                "このプロジェクトは古いバージョンの Allomos で設定されました。今のバージョンに更新してください。\n\nプロジェクトのフォルダー：{p}\nタスクバッグ（Allomos のタスクファイル）：{t}\n\n{}\n",
                numbered.join("\n")
            )
        }
        _ => {
            if older {
                steps.push(format!("Run `{alm} upgrade` to bring the taskbag up to this Allomos's format. It adds only what the newer format has; never edit the taskbag's files by hand."));
            }
            if !files.is_empty() {
                steps.push(format!("In the project folder, replace Allomos's section in {list} (headed \"{AGENT_SECTION_HEADING}…\", or any other text about Allomos or `alm`) with the output of `{alm} brief`, exactly as it is. Keep the rest of each file as it is."));
            }
            steps.push(format!("If the project is a Git repository, commit only these changes: the taskbag's files{}. Leave every other change as it is; do not push unless the person asks.", if files.is_empty() { String::new() } else { format!(" and {list}") }));
            steps.push("Show the user what you changed and the commits you made.".to_string());
            let numbered: Vec<String> = steps
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {s}", i + 1))
                .collect();
            format!(
                "This project was set up by an older Allomos. Please update it to this version.\n\nProject folder: {p}\nTaskbag (Allomos's task files): {t}\n\n{}\n",
                numbered.join("\n")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DEFAULT_DIR, agent_section, init, load};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("allomos-test-versions-{name}-{n}"));
        fs::create_dir_all(&p).unwrap();
        p
    }

    /// A project folder with a taskbag holding one task.
    fn project(name: &str) -> (PathBuf, PathBuf) {
        let dir = tmp(name);
        let root = dir.join(DEFAULT_DIR);
        init(&root, "demo", "Demo").unwrap();
        let lock = Lock::acquire(&root, LOCK_WAIT).unwrap();
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
        set_project_format(&root, "allomos/3");
        assert_eq!(
            format_state(&root),
            FormatState::Newer {
                found: "allomos/3".into()
            }
        );
        assert_eq!(
            format_state_against(&root, 4),
            FormatState::Older {
                found: "allomos/3".into()
            }
        );
        set_project_format(&root, "fliphopper/1");
        assert_eq!(format_state(&root), FormatState::Unknown);
    }

    /// A made-up step from 2 to 3, standing in for a later one: it adds a file with a default,
    /// and leaves it alone when it is already there.
    fn add_template(root: &Path) -> io::Result<()> {
        let path = root.join("later.md");
        if !path.exists() {
            fs::write(path, "# Later\n")?;
        }
        Ok(())
    }
    const TEST_STEPS: &[Step] = &[Step {
        from: 2,
        apply: add_template,
    }];

    #[test]
    fn an_upgrade_adds_only_the_new_parts() {
        let (_, root) = project("upgrade");
        let before = load(&root).taskbag.unwrap();
        let project_before = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();

        let done = upgrade_with(&root, TEST_STEPS, 3).unwrap();
        assert_eq!(
            done,
            Upgraded {
                from: "allomos/2".into(),
                to: "allomos/3".into(),
                changed: true
            }
        );
        assert!(root.join("later.md").is_file());
        let project_after = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            project_after,
            project_before.replace("\"allomos/2\"", "\"allomos/3\""),
            "only the format line changes"
        );
        assert_eq!(format_state_against(&root, 3), FormatState::Current);

        // Upgrading again changes nothing.
        let again = upgrade_with(&root, TEST_STEPS, 3).unwrap();
        assert!(!again.changed);
        assert_eq!(
            fs::read_to_string(root.join(PROJECT_FILE)).unwrap(),
            project_after
        );

        // Every task is as it was (read back as allomos/2, the format this build knows).
        set_project_format(&root, "allomos/2");
        assert_eq!(load(&root).taskbag.unwrap().tasks, before.tasks);
    }

    #[test]
    fn a_newer_taskbag_or_a_missing_step_changes_nothing() {
        let (_, root) = project("refused");
        let text = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            upgrade_with(&root, &[], 3).unwrap_err().code(),
            "upgrade_missing"
        );
        set_project_format(&root, "allomos/4");
        let newer = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            upgrade_with(&root, TEST_STEPS, 3).unwrap_err().code(),
            "taskbag_newer"
        );
        assert_eq!(fs::read_to_string(root.join(PROJECT_FILE)).unwrap(), newer);
        assert!(!root.join("later.md").exists());
        assert_ne!(text, newer);
    }

    #[test]
    fn the_update_request_names_only_what_is_out_of_date() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(DEFAULT_DIR);
        let check = |format: FormatState, version: Option<u32>| VersionCheck {
            format_current: FORMAT.into(),
            section_current: SECTION_VERSION,
            format,
            sections: vec![SectionState {
                file: "AGENTS.md".into(),
                version,
            }],
        };
        for language in ["en", "zh-CN", "ja"] {
            let both = check(
                FormatState::Older {
                    found: "allomos/2".into(),
                },
                None,
            );
            let r = agent_update_request_in(language, &project, &taskbag, &both);
            assert!(
                r.contains("alm --taskbag \"/work/bakery/.allomos/taskbag\" upgrade"),
                "{language}"
            );
            assert!(
                r.contains("/work/bakery/.allomos/taskbag\" brief`"),
                "{language}"
            );
            assert!(r.contains("AGENTS.md"), "{language}");

            let section = check(FormatState::Current, None);
            let r = agent_update_request_in(language, &project, &taskbag, &section);
            assert!(
                !r.contains("upgrade`"),
                "{language}: no upgrade when the format is current"
            );
            assert!(r.contains("brief`"), "{language}");

            let format = check(
                FormatState::Older {
                    found: "allomos/2".into(),
                },
                Some(SECTION_VERSION),
            );
            let r = agent_update_request_in(language, &project, &taskbag, &format);
            assert!(
                r.contains("upgrade`") && !r.contains("brief`"),
                "{language}"
            );
        }
    }

    #[test]
    fn the_real_upgrade_from_1_adds_the_handoff_files() {
        let (_, root) = project("from-1");
        for f in ["handoff-template.md", "project-notes.md", "handoffs/.keep"] {
            fs::remove_file(root.join(f)).unwrap();
        }
        fs::remove_dir(root.join("handoffs")).unwrap();
        set_project_format(&root, "allomos/1");
        let before = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            format_state(&root),
            FormatState::Older {
                found: "allomos/1".into()
            }
        );
        let done = upgrade(&root).unwrap();
        assert_eq!(
            (done.from.as_str(), done.to.as_str(), done.changed),
            ("allomos/1", "allomos/2", true)
        );
        assert_eq!(
            crate::handoff::template(&root),
            crate::handoff::DEFAULT_TEMPLATE
        );
        assert!(root.join("project-notes.md").is_file() && root.join("handoffs/.keep").is_file());
        assert_eq!(
            fs::read_to_string(root.join(PROJECT_FILE)).unwrap(),
            before.replace("\"allomos/1\"", "\"allomos/2\"")
        );
        assert!(load(&root).issues.is_empty());
    }

    #[test]
    fn upgrading_a_current_taskbag_does_nothing() {
        let (_, root) = project("current");
        let done = upgrade(&root).unwrap();
        assert!(!done.changed);
        assert_eq!(done.to, FORMAT);
    }
}
