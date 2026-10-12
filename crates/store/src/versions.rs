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
pub const SECTION_VERSION: u32 = 1;

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

/// The upgrade steps this Allomos knows, oldest first. Empty while `allomos/1` is the only format;
/// the first card that changes the taskbag's shape adds the step from 1 to 2.
pub const STEPS: &[Step] = &[];

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
        set_project_format(&root, "allomos/2");
        assert_eq!(
            format_state(&root),
            FormatState::Newer {
                found: "allomos/2".into()
            }
        );
        assert_eq!(
            format_state_against(&root, 3),
            FormatState::Older {
                found: "allomos/2".into()
            }
        );
        set_project_format(&root, "fliphopper/1");
        assert_eq!(format_state(&root), FormatState::Unknown);
    }

    /// A made-up step from 1 to 2, standing in for the first real one (A03-06): it adds a file
    /// with a default, and leaves it alone when it is already there.
    fn add_template(root: &Path) -> io::Result<()> {
        let path = root.join("handoff-template.md");
        if !path.exists() {
            fs::write(path, "# Handoff\n")?;
        }
        Ok(())
    }
    const TEST_STEPS: &[Step] = &[Step {
        from: 1,
        apply: add_template,
    }];

    #[test]
    fn an_upgrade_adds_only_the_new_parts() {
        let (_, root) = project("upgrade");
        let before = load(&root).taskbag.unwrap();
        let project_before = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();

        let done = upgrade_with(&root, TEST_STEPS, 2).unwrap();
        assert_eq!(
            done,
            Upgraded {
                from: "allomos/1".into(),
                to: "allomos/2".into(),
                changed: true
            }
        );
        assert!(root.join("handoff-template.md").is_file());
        let project_after = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            project_after,
            project_before.replace("\"allomos/1\"", "\"allomos/2\""),
            "only the format line changes"
        );
        assert_eq!(format_state_against(&root, 2), FormatState::Current);

        // Upgrading again changes nothing.
        let again = upgrade_with(&root, TEST_STEPS, 2).unwrap();
        assert!(!again.changed);
        assert_eq!(
            fs::read_to_string(root.join(PROJECT_FILE)).unwrap(),
            project_after
        );

        // Every task is as it was (read back as allomos/1, the format this build knows).
        set_project_format(&root, "allomos/1");
        assert_eq!(load(&root).taskbag.unwrap().tasks, before.tasks);
    }

    #[test]
    fn a_newer_taskbag_or_a_missing_step_changes_nothing() {
        let (_, root) = project("refused");
        let text = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            upgrade_with(&root, &[], 2).unwrap_err().code(),
            "upgrade_missing"
        );
        set_project_format(&root, "allomos/3");
        let newer = fs::read_to_string(root.join(PROJECT_FILE)).unwrap();
        assert_eq!(
            upgrade_with(&root, TEST_STEPS, 2).unwrap_err().code(),
            "taskbag_newer"
        );
        assert_eq!(fs::read_to_string(root.join(PROJECT_FILE)).unwrap(), newer);
        assert!(!root.join("handoff-template.md").exists());
        assert_ne!(text, newer);
    }

    #[test]
    fn upgrading_a_current_taskbag_does_nothing() {
        let (_, root) = project("current");
        let done = upgrade(&root).unwrap();
        assert!(!done.changed);
        assert_eq!(done.to, FORMAT);
    }
}
