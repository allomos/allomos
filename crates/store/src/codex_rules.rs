//! Allomos's rules for Codex (Task-A03-pier032, approver 2026-11-09): a Codex rules file
//! (`prefix_rule`, read from `$CODEX_HOME/rules/`) that lets an unattended Codex session run Git
//! and `alm` outside its sandbox, every other command staying inside it. It ships with Allomos,
//! `alm guide codex-rules` prints it, and the app's Monitor ready tasks… dialog adds it to Codex
//! (`allomos.rules`, never the person's own `default.rules`).
//!
//! Every version Allomos ever shipped is kept in [`SHIPPED`], so the app can tell a file it wrote
//! (of any version) from one somebody changed, and list what an update adds and removes.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// The version of the rules this Allomos ships. Raise it, and add the new text to [`SHIPPED`],
/// whenever a rule changes; the release notes say what changed and why.
///
/// 1: Git pull, fetch, add, commit, push (not forced right after `push`), merge, switch,
/// branch -d, worktree, and `alm` (Task-A03-pier032).
pub const VERSION: u32 = 1;

/// The file's name in Codex's rules folder.
pub const FILE_NAME: &str = "allomos.rules";

const V1: &str = r#"# Allomos rules for Codex, version 1. Written by Allomos: changes here are replaced on update.
# Your own rules go in default.rules in this folder; Allomos never changes it.
prefix_rule(pattern = ["git", "pull"], decision = "allow")
prefix_rule(pattern = ["git", "fetch"], decision = "allow")
prefix_rule(pattern = ["git", "add"], decision = "allow")
prefix_rule(pattern = ["git", "commit"], decision = "allow")
prefix_rule(pattern = ["git", "push"], decision = "allow")
prefix_rule(pattern = ["git", "push", ["--force", "-f", "--force-with-lease"]], decision = "forbidden")
prefix_rule(pattern = ["git", "merge"], decision = "allow")
prefix_rule(pattern = ["git", "switch"], decision = "allow")
prefix_rule(pattern = ["git", "branch", "-d"], decision = "allow")
prefix_rule(pattern = ["git", "worktree"], decision = "allow")
prefix_rule(pattern = ["alm"], decision = "allow")
"#;

/// Every version Allomos shipped, oldest first, each with its exact text.
pub const SHIPPED: &[(u32, &str)] = &[(1, V1)];

/// The text of the rules this Allomos ships.
pub fn text() -> &'static str {
    SHIPPED
        .iter()
        .find(|(v, _)| *v == VERSION)
        .map(|(_, t)| *t)
        .expect("the current version is shipped")
}

/// One rule as a person reads it: `git pull` (allowed), or `git push --force` (never). A choice of
/// words is written `--force | -f`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Rule {
    pub command: String,
    /// `allow` or `forbidden`, as the file says.
    pub decision: String,
}

/// The rules of a rules file, in order; lines that are not a `prefix_rule` are left out.
pub fn rules(text: &str) -> Vec<Rule> {
    text.lines().filter_map(rule).collect()
}

fn rule(line: &str) -> Option<Rule> {
    let line = line.trim();
    let inner = line.strip_prefix("prefix_rule(")?;
    let start = inner.find('[')?;
    // The pattern ends at the bracket that closes its first one.
    let mut depth = 0;
    let mut end = None;
    for (i, c) in inner[start..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(start + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let pattern = &inner[start + 1..end?];
    let mut words = Vec::new();
    let mut rest = pattern;
    while let Some(i) = rest.find(['"', '[']) {
        if rest[i..].starts_with('[') {
            let close = rest[i..].find(']')? + i;
            let choices: Vec<&str> = rest[i + 1..close]
                .split(',')
                .map(|w| w.trim().trim_matches('"'))
                .filter(|w| !w.is_empty())
                .collect();
            words.push(choices.join(" | "));
            rest = &rest[close + 1..];
        } else {
            let after = &rest[i + 1..];
            let close = after.find('"')?;
            words.push(after[..close].to_string());
            rest = &after[close + 1..];
        }
    }
    let decision = inner[end? + 1..]
        .split("decision")
        .nth(1)
        .and_then(|d| d.split('"').nth(1))
        .unwrap_or("allow")
        .to_string();
    Some(Rule {
        command: words.join(" "),
        decision,
    })
}

/// The version a rules file says it is, from its first line (`… for Codex, version 2.`).
pub fn version_of(text: &str) -> Option<u32> {
    let first = text.lines().next()?;
    let after = first.strip_prefix("# Allomos rules for Codex, version ")?;
    after
        .split(|c: char| !c.is_ascii_digit())
        .next()?
        .parse()
        .ok()
}

/// What changes from one set of rules to another: the rules added and the rules removed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Changes {
    pub added: Vec<Rule>,
    pub removed: Vec<Rule>,
}

pub fn changes(from: &str, to: &str) -> Changes {
    let (old, new) = (rules(from), rules(to));
    Changes {
        added: new.iter().filter(|r| !old.contains(r)).cloned().collect(),
        removed: old.iter().filter(|r| !new.contains(r)).cloned().collect(),
    }
}

/// Codex's settings folder: `CODEX_HOME`, or `.codex` in the home folder.
pub fn codex_home() -> Option<PathBuf> {
    if let Some(h) = std::env::var_os("CODEX_HOME").filter(|h| !h.is_empty()) {
        return Some(PathBuf::from(h));
    }
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".codex"))
}

/// Where Allomos keeps its rules for Codex: `<Codex's settings folder>/rules/allomos.rules`.
pub fn path_in(codex_home: &Path) -> PathBuf {
    codex_home.join("rules").join(FILE_NAME)
}

/// How the rules file on this computer stands against the rules this Allomos ships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum State {
    /// No file: Allomos's rules are not in Codex.
    Missing,
    /// The file is this version, as Allomos wrote it.
    Current,
    /// An older version Allomos wrote: an update adds and removes these rules.
    Older { version: u32, changes: Changes },
    /// A newer version, from a newer Allomos: this one leaves it alone.
    Newer { version: u32 },
    /// Somebody changed the file (or it is not one Allomos wrote): what replacing it with this
    /// version adds and removes.
    Changed { changes: Changes },
}

/// The state of the file at `path`, given its text (`None`: no file).
pub fn state_of(installed: Option<&str>) -> State {
    let Some(text) = installed else {
        return State::Missing;
    };
    let ours = self::text();
    match version_of(text) {
        Some(v) if v > VERSION => State::Newer { version: v },
        Some(v) if v == VERSION && text == ours => State::Current,
        Some(v) if SHIPPED.iter().any(|(sv, st)| *sv == v && *st == text) => State::Older {
            version: v,
            changes: changes(text, ours),
        },
        _ => State::Changed {
            changes: changes(text, ours),
        },
    }
}

/// The state of the rules file at `path`.
pub fn state(path: &Path) -> State {
    state_of(std::fs::read_to_string(path).ok().as_deref())
}

/// Write this version's rules at `path` (making its folder when needed).
pub fn install(path: &Path) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, text())
}

/// Remove the rules file at `path`; nothing to do when it is not there.
pub fn remove(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_file_names_its_version_and_reads_as_commands() {
        assert_eq!(version_of(text()), Some(VERSION));
        assert!(text().contains("Written by Allomos: changes here are replaced on update."));
        let rules = rules(text());
        let commands: Vec<&str> = rules.iter().map(|r| r.command.as_str()).collect();
        assert_eq!(
            commands,
            [
                "git pull",
                "git fetch",
                "git add",
                "git commit",
                "git push",
                "git push --force | -f | --force-with-lease",
                "git merge",
                "git switch",
                "git branch -d",
                "git worktree",
                "alm",
            ]
        );
        assert_eq!(rules[5].decision, "forbidden");
        assert!(rules.iter().filter(|r| r.decision == "allow").count() == 10);
        // Generic: no path, person or task in it.
        assert!(!text().contains('/') && !text().contains("target") && !text().contains('@'));
    }

    #[test]
    fn the_state_tells_missing_current_older_newer_and_changed_apart() {
        assert_eq!(state_of(None), State::Missing);
        assert_eq!(state_of(Some(text())), State::Current);
        let newer = text().replace(&format!("version {VERSION}."), "version 99.");
        assert_eq!(state_of(Some(&newer)), State::Newer { version: 99 });
        let edited = format!(
            "{}prefix_rule(pattern = [\"rm\"], decision = \"allow\")\n",
            text()
        );
        match state_of(Some(&edited)) {
            State::Changed { changes } => {
                assert!(changes.added.is_empty());
                assert_eq!(changes.removed[0].command, "rm");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(state_of(Some("anything")), State::Changed { .. }));
    }

    #[test]
    fn changes_list_both_the_added_and_the_removed_rules() {
        let old = "# Allomos rules for Codex, version 1.\nprefix_rule(pattern = [\"git\", \"pull\"], decision = \"allow\")\nprefix_rule(pattern = [\"git\", \"gc\"], decision = \"allow\")\n";
        let new = "# Allomos rules for Codex, version 2.\nprefix_rule(pattern = [\"git\", \"pull\"], decision = \"allow\")\nprefix_rule(pattern = [\"alm\"], decision = \"allow\")\n";
        let c = changes(old, new);
        assert_eq!(c.added[0].command, "alm");
        assert_eq!(c.removed[0].command, "git gc");
    }

    #[test]
    fn install_and_remove_write_only_allomos_own_file() {
        let dir = std::env::temp_dir().join(format!("allomos-codex-rules-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let own = dir.join("rules").join("default.rules");
        std::fs::create_dir_all(own.parent().unwrap()).unwrap();
        std::fs::write(&own, "mine").unwrap();
        let path = path_in(&dir);
        install(&path).unwrap();
        assert_eq!(state(&path), State::Current);
        remove(&path).unwrap();
        remove(&path).unwrap();
        assert_eq!(state(&path), State::Missing);
        assert_eq!(std::fs::read_to_string(&own).unwrap(), "mine");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
