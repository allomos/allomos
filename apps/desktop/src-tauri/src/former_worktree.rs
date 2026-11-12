//! **Take over…** (Task-A01-flag101): what the former holder's worktree still holds that is not
//! pushed, when that worktree is on this computer. The dialog lists it so the person can let the new
//! session commit and push it to the working branch before going on.
//!
//! Only reading: `git status` and `git log` against the remote-tracking branches as they are (no
//! fetch, nothing written). The worktree is the one on the task's working branch, found from the
//! folder the former holder's claim ran in (a worktree itself, or the main checkout beside it), or
//! from the project folder; none on that branch means it is not on this computer.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

/// A file the worktree has not committed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Uncommitted {
    /// `modified`, `added`, `untracked` or `deleted`.
    pub kind: &'static str,
    /// Relative to the worktree, with `/`.
    pub path: String,
}

/// A commit on the working branch that no remote-tracking branch has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Unpushed {
    /// The short hash.
    pub hash: String,
    /// The commit message's first line.
    pub subject: String,
}

/// The former holder's worktree and what it has not pushed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FormerWorktree {
    /// Where it is, in full.
    pub path: String,
    pub files: Vec<Uncommitted>,
    pub commits: Vec<Unpushed>,
}

/// How many unpushed commits are listed at most (a branch never pushed may have many).
const MOST_COMMITS: usize = 200;

fn git(dir: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// The worktree on branch `working`, from any of `places` that is in the repository.
fn find(places: &[PathBuf], working: &str) -> Option<PathBuf> {
    let want = format!("branch refs/heads/{working}");
    for place in places.iter().filter(|p| p.is_dir()) {
        let Some(out) = git(place, &["worktree", "list", "--porcelain"]) else {
            continue;
        };
        let text = String::from_utf8_lossy(&out);
        let mut path: Option<&str> = None;
        for line in text.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = Some(p);
            } else if line == want {
                if let Some(p) = path.map(PathBuf::from).filter(|p| p.is_dir()) {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// `git status --porcelain -z` as files: renames and copies count as added (their new path).
fn files(porcelain: &[u8]) -> Vec<Uncommitted> {
    let mut out = Vec::new();
    let mut entries = porcelain.split(|b| *b == 0).filter(|e| !e.is_empty());
    while let Some(e) = entries.next() {
        if e.len() < 4 {
            continue;
        }
        let (x, y) = (e[0], e[1]);
        let path = String::from_utf8_lossy(&e[3..]).into_owned();
        if matches!(x, b'R' | b'C') {
            entries.next(); // the original path
        }
        let kind = match (x, y) {
            (b'?', b'?') => "untracked",
            (b'D', _) | (_, b'D') => "deleted",
            (b'A' | b'R' | b'C', _) => "added",
            _ => "modified",
        };
        out.push(Uncommitted { kind, path });
    }
    out
}

fn commits(log: &[u8]) -> Vec<Unpushed> {
    String::from_utf8_lossy(log)
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .map(|(h, s)| Unpushed {
            hash: h.to_string(),
            subject: s.to_string(),
        })
        .collect()
}

/// The former holder's worktree on `working`, looked for from `places` in order; `None` when it is
/// not on this computer (or git cannot read it).
pub fn look(places: &[PathBuf], working: &str) -> Option<FormerWorktree> {
    let wt = find(places, working)?;
    let status = git(
        &wt,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?;
    let max = format!("--max-count={MOST_COMMITS}");
    let log = git(
        &wt,
        &[
            "log",
            &max,
            "--format=%h%x09%s",
            "HEAD",
            "--not",
            "--remotes",
        ],
    )
    .unwrap_or_default();
    Some(FormerWorktree {
        path: wt.display().to_string(),
        files: files(&status),
        commits: commits(&log),
    })
}

/// **Take over…**: the former holder's worktree for the task in the taskbag at `taskbag`, on its
/// working branch `working`; `folder` is where its claim ran, relative to the folder holding
/// `.allomos/` (the task's session). `None`: not on this computer.
#[tauri::command]
pub fn former_worktree(
    taskbag: PathBuf,
    folder: Option<String>,
    working: String,
) -> Option<FormerWorktree> {
    let mut places = Vec::new();
    if let Some(f) = folder.as_deref().filter(|f| !f.is_empty()) {
        places.push(allomos_store::allomos_folder(&taskbag).join(f));
    }
    places.push(allomos_store::versions::project_folder(&taskbag));
    look(&places, &working)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_entries_become_kinds() {
        let raw =
            b" M src/a.rs\0A  b.rs\0?? new/c.txt\0 D gone.rs\0R  to.rs\0from.rs\0MM both.rs\0";
        let found = files(raw);
        let got: Vec<(&str, &str)> = found.iter().map(|f| (f.kind, f.path.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("modified", "src/a.rs"),
                ("added", "b.rs"),
                ("untracked", "new/c.txt"),
                ("deleted", "gone.rs"),
                ("added", "to.rs"),
                ("modified", "both.rs"),
            ]
        );
    }

    #[test]
    fn log_lines_become_commits() {
        assert_eq!(
            commits(b"abc1234\tWIP A01-1: half\ndef5678\tfix: tabs\tand more\n"),
            vec![
                Unpushed {
                    hash: "abc1234".into(),
                    subject: "WIP A01-1: half".into()
                },
                Unpushed {
                    hash: "def5678".into(),
                    subject: "fix: tabs\tand more".into()
                },
            ]
        );
    }

    /// A real repository: a worktree on the working branch with a file not committed and a commit
    /// not pushed is found from the main checkout; another branch is not on this computer.
    #[test]
    fn finds_the_worktree_on_the_working_branch() {
        let tmp = std::env::temp_dir().join(format!("alm-former-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let (origin, main, wt) = (tmp.join("origin.git"), tmp.join("main"), tmp.join("wt"));
        std::fs::create_dir_all(&tmp).unwrap();
        let run = |dir: &Path, args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(dir)
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
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?}");
        };
        run(&tmp, &["init", "--bare", "-q", origin.to_str().unwrap()]);
        run(
            &tmp,
            &[
                "clone",
                "-q",
                origin.to_str().unwrap(),
                main.to_str().unwrap(),
            ],
        );
        std::fs::write(main.join("a.txt"), "a").unwrap();
        run(&main, &["add", "a.txt"]);
        run(&main, &["commit", "-q", "-m", "first"]);
        run(&main, &["push", "-q", "origin", "HEAD:main"]);
        run(&main, &["fetch", "-q"]);
        run(
            &main,
            &[
                "worktree",
                "add",
                "-q",
                wt.to_str().unwrap(),
                "-b",
                "claude/x",
            ],
        );
        std::fs::write(wt.join("b.txt"), "b").unwrap();
        run(&wt, &["add", "b.txt"]);
        run(&wt, &["commit", "-q", "-m", "WIP: b"]);
        std::fs::write(wt.join("a.txt"), "changed").unwrap();
        std::fs::write(wt.join("c.txt"), "c").unwrap();

        let found = look(&[tmp.join("missing"), main.clone()], "claude/x").expect("found");
        assert_eq!(
            std::fs::canonicalize(&found.path).unwrap(),
            std::fs::canonicalize(&wt).unwrap()
        );
        let kinds: Vec<_> = found
            .files
            .iter()
            .map(|f| (f.kind, f.path.clone()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("modified", "a.txt".to_string()),
                ("untracked", "c.txt".to_string())
            ]
        );
        assert_eq!(found.commits.len(), 1);
        assert_eq!(found.commits[0].subject, "WIP: b");
        assert!(look(&[main.clone()], "claude/other").is_none());

        run(&wt, &["push", "-q", "origin", "claude/x"]);
        run(&wt, &["checkout", "-q", "--", "a.txt"]);
        std::fs::remove_file(wt.join("c.txt")).unwrap();
        let clean = look(&[wt.clone()], "claude/x").expect("found");
        assert!(clean.files.is_empty() && clean.commits.is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
