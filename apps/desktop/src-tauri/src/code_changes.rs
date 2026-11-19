//! The Evidence screen's **What changed** (Task-A01-hyena938, decision 0029 point 3): for the
//! commits and merges a task recorded, each one's first line, and together which files they
//! changed with the lines added and removed, read from the project's own git.
//!
//! Only reading, like **Take over…** reads the former worktree (`former_worktree.rs`): no fetch,
//! nothing written, nothing kept in the taskbag. A commit this computer's repository does not have
//! is listed as missing, and the screen shows its link only.
//!
//! The lines are counted once: a merge counts as its first parent to the merge (what it brought
//! into its base); a recorded commit that a recorded merge brought in is not counted again.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// One recorded commit or merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommitLine {
    /// As the task recorded it.
    pub hash: String,
    /// The commit message's first line; `None`: not on this computer.
    pub subject: Option<String>,
    /// Whether it has more than one parent.
    pub merge: bool,
}

/// A changed file, with the lines added and removed (`None` for both: a binary file).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileLines {
    /// Relative to the repository, with `/`.
    pub path: String,
    pub added: Option<u64>,
    pub removed: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodeChanges {
    pub commits: Vec<CommitLine>,
    pub files: Vec<FileLines>,
}

fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

fn text(out: Vec<u8>) -> String {
    String::from_utf8_lossy(&out).trim_end().to_string()
}

/// Only a hash is looked up (never a name or an option git would read differently).
fn is_hash(s: &str) -> bool {
    (4..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The full hash and parents of `hash`, when this repository has it.
fn find(dir: &Path, hash: &str) -> Option<(String, Vec<String>)> {
    if !is_hash(hash) {
        return None;
    }
    let spec = format!("{hash}^{{commit}}");
    let full = text(git(dir, &["rev-parse", "--verify", "--quiet", &spec])?);
    let line = text(git(dir, &["rev-list", "--parents", "-n", "1", &full])?);
    let parents = line
        .split_whitespace()
        .skip(1)
        .map(str::to_string)
        .collect();
    Some((full, parents))
}

fn is_ancestor(dir: &Path, a: &str, b: &str) -> bool {
    git(dir, &["merge-base", "--is-ancestor", a, b]).is_some()
}

/// `--numstat -z` output (renames off), added to `into` per path.
fn add_numstat(out: &[u8], into: &mut BTreeMap<String, (Option<u64>, Option<u64>)>) {
    for entry in out.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let s = String::from_utf8_lossy(entry);
        let s = s.trim_start_matches('\n');
        let mut parts = s.splitn(3, '\t');
        let (Some(a), Some(r), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        let (a, r) = (a.parse::<u64>().ok(), r.parse::<u64>().ok());
        let slot = into.entry(path.to_string()).or_insert((Some(0), Some(0)));
        slot.0 = slot.0.zip(a).map(|(x, y)| x + y);
        slot.1 = slot.1.zip(r).map(|(x, y)| x + y);
    }
}

/// The recorded `hashes`, read from the repository at `dir`.
pub fn read(dir: &Path, hashes: &[String]) -> CodeChanges {
    let found: Vec<Option<(String, Vec<String>)>> = hashes.iter().map(|h| find(dir, h)).collect();
    let mut commits = Vec::new();
    for (hash, f) in hashes.iter().zip(&found) {
        let subject = f
            .as_ref()
            .and_then(|(full, _)| git(dir, &["log", "-1", "--format=%s", full]))
            .map(text);
        let merge = f.as_ref().is_some_and(|(_, p)| p.len() > 1);
        commits.push(CommitLine {
            hash: hash.clone(),
            subject,
            merge,
        });
    }
    let mut seen = std::collections::HashSet::new();
    let known: Vec<&(String, Vec<String>)> = found
        .iter()
        .flatten()
        .filter(|(full, _)| seen.insert(full.clone()))
        .collect();
    let merges: Vec<&(String, Vec<String>)> =
        known.iter().copied().filter(|(_, p)| p.len() > 1).collect();
    let mut lines = BTreeMap::new();
    for (full, parents) in &merges {
        let args = ["diff", "--numstat", "-z", "--no-renames", &parents[0], full];
        if let Some(out) = git(dir, &args) {
            add_numstat(&out, &mut lines);
        }
    }
    for (full, _) in known.iter().filter(|(_, p)| p.len() <= 1) {
        let brought = merges
            .iter()
            .any(|(m, p)| is_ancestor(dir, full, m) && !is_ancestor(dir, full, &p[0]));
        if brought {
            continue;
        }
        let args = [
            "diff-tree",
            "--no-commit-id",
            "--numstat",
            "-z",
            "--no-renames",
            "-r",
            "--root",
            full,
        ];
        if let Some(out) = git(dir, &args) {
            add_numstat(&out, &mut lines);
        }
    }
    let files = lines
        .into_iter()
        .map(|(path, (added, removed))| FileLines {
            path,
            added,
            removed,
        })
        .collect();
    CodeChanges { commits, files }
}

/// **What changed** on the Evidence screen: the `hashes` the task recorded, read from the project
/// folder of the taskbag at `taskbag`. Only reads.
#[tauri::command(async)]
pub fn code_changes(taskbag: PathBuf, hashes: Vec<String>) -> CodeChanges {
    read(&allomos_store::versions::project_folder(&taskbag), &hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numstat_entries_add_up_per_path() {
        let mut m = BTreeMap::new();
        add_numstat(b"3\t1\ta.rs\x002\t0\tb/c.ts\x00-\t-\tlogo.png\x00", &mut m);
        add_numstat(b"\n1\t4\ta.rs\x00", &mut m);
        assert_eq!(m["a.rs"], (Some(4), Some(5)));
        assert_eq!(m["b/c.ts"], (Some(2), Some(0)));
        assert_eq!(m["logo.png"], (None, None));
    }

    #[test]
    fn only_hashes_are_looked_up() {
        assert!(is_hash("989bed1a"));
        assert!(!is_hash("--all") && !is_hash("main") && !is_hash("abc"));
    }

    /// A real repository: two commits on a branch and their merge are found, with their first
    /// lines, and the lines are counted once; a commit not here is listed as missing.
    #[test]
    fn reads_found_and_missing_commits() {
        let tmp = std::env::temp_dir().join(format!("alm-changes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let run = |args: &[&str]| -> String {
            let out = Command::new("git")
                .arg("-C")
                .arg(&tmp)
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "init.defaultBranch=main",
                ])
                .args(args)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {args:?}");
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        run(&["init", "-q"]);
        std::fs::write(tmp.join("a.txt"), "1\n2\n").unwrap();
        run(&["add", "a.txt"]);
        run(&["commit", "-q", "-m", "first"]);
        run(&["switch", "-q", "-c", "claude/x"]);
        std::fs::write(tmp.join("a.txt"), "1\n2\n3\n").unwrap();
        run(&["commit", "-q", "-am", "WIP: three\n\nmore words"]);
        std::fs::write(tmp.join("b.txt"), "b\n").unwrap();
        std::fs::write(tmp.join("a.txt"), "1\n3\n").unwrap();
        run(&["add", "b.txt", "a.txt"]);
        run(&["commit", "-q", "-m", "b and tidy"]);
        let (c1, c2) = (run(&["rev-parse", "HEAD~1"]), run(&["rev-parse", "HEAD"]));
        run(&["switch", "-q", "main"]);
        std::fs::write(tmp.join("other.txt"), "o\n").unwrap();
        run(&["add", "other.txt"]);
        run(&["commit", "-q", "-m", "elsewhere"]);
        run(&["merge", "-q", "--no-ff", "-m", "Merge claude/x", "claude/x"]);
        let m = run(&["rev-parse", "HEAD"]);
        let missing = "0123456789abcdef0123456789abcdef01234567".to_string();

        let got = read(
            &tmp,
            &[c1[..8].to_string(), c2.clone(), m.clone(), missing.clone()],
        );
        let subjects: Vec<_> = got
            .commits
            .iter()
            .map(|c| (c.subject.as_deref(), c.merge))
            .collect();
        assert_eq!(
            subjects,
            vec![
                (Some("WIP: three"), false),
                (Some("b and tidy"), false),
                (Some("Merge claude/x"), true),
                (None, false),
            ]
        );
        assert_eq!(got.commits[0].hash, c1[..8]);
        // The merge brought a.txt (2 lines → "1\n3\n": +1 −1) and b.txt (+1); the commits it
        // brought in are not counted again, and other.txt is not part of it.
        assert_eq!(
            got.files,
            vec![
                FileLines {
                    path: "a.txt".into(),
                    added: Some(1),
                    removed: Some(1)
                },
                FileLines {
                    path: "b.txt".into(),
                    added: Some(1),
                    removed: Some(0)
                },
            ]
        );

        // Without the merge, each commit counts.
        let alone = read(&tmp, &[c1, c2]);
        assert_eq!(
            alone.files[0],
            FileLines {
                path: "a.txt".into(),
                added: Some(1),
                removed: Some(1)
            }
        );
        // Nothing found: nothing counted.
        let none = read(&tmp, &[missing, "--all".into()]);
        assert!(none.commits.iter().all(|c| c.subject.is_none()) && none.files.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
