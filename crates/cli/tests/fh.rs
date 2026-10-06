//! `fh` run as a real program, the way agents run it: every command's success, its
//! refusals, its `--json` output and its exit codes, plus the race between two agents.
//! Tests use temporary folders only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

const FH: &str = env!("CARGO_BIN_EXE_fh");

fn tmp(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "fh-test-{name}-{n}-{}",
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

/// `fh` in `dir`, with no FlipHopper settings inherited from the test's own environment.
fn fh(dir: &Path, args: &[&str]) -> Output {
    Command::new(FH)
        .current_dir(dir)
        .args(args)
        .env_remove("FLIPHOPPER_SEABAG")
        .env_remove("FLIPHOPPER_HOLDER")
        .env_remove("FLIPHOPPER_LOCK_WAIT_MS")
        .output()
        .unwrap()
}

/// `fh --json ...`: the exit code and the one JSON object on standard output.
fn fj(dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = fh(dir, &all);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "one line of JSON: {stdout:?}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["schema"], "fh/1");
    assert!(out.stderr.is_empty(), "nothing for people with --json");
    (out.status.code().unwrap(), v)
}

fn ok(dir: &Path, args: &[&str]) -> Value {
    let (code, v) = fj(dir, args);
    assert_eq!((code, &v["ok"]), (0, &Value::Bool(true)), "{args:?}: {v}");
    v
}

fn refused(dir: &Path, args: &[&str], want_code: i32, problem: &str) -> Value {
    let (code, v) = fj(dir, args);
    assert_eq!(v["ok"], false, "{args:?}: {v}");
    assert_eq!(v["error"]["code"], problem, "{args:?}: {v}");
    assert!(v["error"]["message"].as_str().unwrap().len() > 5);
    assert_eq!(code, want_code, "{args:?}: {v}");
    v
}

/// A fresh Git-like repository (a `.git` folder is enough for `fh init`) with a seabag
/// holding two areas and three tasks.
fn filled() -> PathBuf {
    let repo = tmp("repo");
    fs::create_dir(repo.join(".git")).unwrap();
    ok(&repo, &["init", "--name", "Demo"]);
    ok(&repo, &["add", "area", "A01", "Tracking"]);
    ok(&repo, &["add", "area", "A02", "Charts"]);
    ok(
        &repo,
        &[
            "add",
            "task",
            "A01-01",
            "--area",
            "A01",
            "--title",
            "Open a seabag",
            "--description",
            "A person opens a folder.",
        ],
    );
    ok(
        &repo,
        &["add", "task", "A01-02", "--area", "A01", "--title", "List"],
    );
    ok(
        &repo,
        &["add", "task", "A02-01", "--area", "A02", "--title", "Burn"],
    );
    repo
}

fn task_file(repo: &Path, id: &str) -> Value {
    let text =
        fs::read_to_string(repo.join(format!(".fliphopper/seabag/tasks/{id}/task.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn both_names_run() {
    let fliphopper =
        Path::new(FH).with_file_name(format!("fliphopper{}", std::env::consts::EXE_SUFFIX));
    let out = Command::new(fliphopper).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("0.1.0"));
}

#[test]
fn init_refuses_twice_and_needs_no_git() {
    let dir = tmp("plain");
    let v = ok(&dir, &["init"]);
    let root = dir.join(".fliphopper/seabag");
    // The command reports the real path (on macOS, /var is /private/var).
    let reported = PathBuf::from(v["seabag"].as_str().unwrap());
    assert_eq!(reported, root.canonicalize().unwrap());
    assert!(!root.join(".git").exists() && !dir.join(".git").exists());
    refused(&dir, &["init"], 1, "seabag_exists");
    // From a subfolder of a repository, the seabag goes at the repository's root.
    let repo = tmp("sub");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(repo.join("src/deep")).unwrap();
    ok(&repo.join("src/deep"), &["init"]);
    assert!(repo.join(".fliphopper/seabag/project.json").is_file());
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".fliphopper/seabag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["name"], repo.file_name().unwrap().to_str().unwrap());
}

#[test]
fn work_one_task_from_claim_to_done() {
    let repo = filled();
    let sub = repo.join("src");
    fs::create_dir_all(&sub).unwrap();

    let v = ok(&sub, &["--as", "agent-a", "claim", "A01-01"]);
    assert_eq!(v["command"], "claim");
    assert_eq!(v["task"]["state"], "in_progress");
    assert_eq!(v["task"]["holder"], "agent-a");
    assert_eq!(v["warnings"], serde_json::json!([]));

    let v = ok(&sub, &["--as", "agent-a", "label", "A01-01", "implemented"]);
    assert_eq!(v["task"]["label"], "implemented");
    let v = ok(
        &sub,
        &["--as", "agent-a", "wait", "A01-01", "--on", "owner review"],
    );
    assert_eq!(v["task"]["state"], "waiting");
    assert_eq!(v["task"]["waiting_on"], "owner review");

    let v = ok(&sub, &["--as", "agent-a", "done", "A01-01"]);
    assert_eq!(v["task"]["state"], "done");
    assert_eq!(v["warnings"][0]["code"], "label_below_top");

    // The files say what the command said.
    let file = task_file(&repo, "A01-01");
    assert_eq!(file["state"], "done");
    assert_eq!(file["label"], "implemented");
    assert_eq!(file["holder"], "agent-a");
    assert!(file.get("waiting_on").is_none() && file.get("description").is_none());
    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["task"]["description"], "A person opens a folder.\n");
    assert_eq!(v["task"]["state"], file["state"]);

    // list: grouped by area, filtered.
    let v = ok(&repo, &["list"]);
    let areas = v["areas"].as_array().unwrap();
    assert_eq!(areas.len(), 2);
    assert_eq!(areas[0]["tasks"].as_array().unwrap().len(), 2);
    let v = ok(&repo, &["list", "--state", "done"]);
    assert_eq!(v["areas"][0]["tasks"][0]["id"], "A01-01");
    assert_eq!(v["areas"][1]["tasks"], serde_json::json!([]));
    let v = ok(&repo, &["--as", "agent-a", "list", "--mine"]);
    assert_eq!(v["areas"][0]["tasks"].as_array().unwrap().len(), 1);
    let v = ok(&repo, &["list", "--area", "A02"]);
    assert_eq!(v["areas"].as_array().unwrap().len(), 1);

    // Text output for people.
    let out = fh(&repo, &["list"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("A01  Tracking\n  A01-01  done  implemented  agent-a  Open a seabag"));
}

#[test]
fn release_leaves_a_note_that_next_shows() {
    let repo = filled();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["task"], Value::Null);
    ok(&repo, &["--as", "a", "claim", "A01-02"]);
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "release",
            "A01-02",
            "--note",
            "tests pass; docs left",
        ],
    );
    assert_eq!(v["task"]["state"], "ready");
    assert_eq!(v["task"].get("holder"), Some(&Value::Null));
    let v = ok(&repo, &["next"]);
    assert_eq!(v["task"]["id"], "A01-02");
    assert_eq!(v["task"]["handoff_note"], "tests pass; docs left");
    assert_eq!(
        task_file(&repo, "A01-02")["handoff_note"],
        "tests pass; docs left"
    );
    let text = String::from_utf8(fh(&repo, &["show", "A01-02"]).stdout).unwrap();
    assert!(text.contains("handoff note: tests pass; docs left"));
    // A new agent resumes in one step.
    let v = ok(&repo, &["--as", "b", "claim", "A01-02"]);
    assert_eq!(v["task"]["handoff_note"], "tests pass; docs left");
}

#[test]
fn refusals_have_codes_and_exit_codes() {
    let repo = filled();
    refused(&repo, &["claim", "A01-01"], 1, "holder_missing");
    refused(&repo, &["list", "--mine"], 1, "holder_missing");
    refused(&repo, &["--as", "a", "claim", "nope"], 1, "task_not_found");
    refused(&repo, &["show", "nope"], 1, "task_not_found");
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let v = refused(&repo, &["--as", "b", "claim", "A01-01"], 1, "task_held");
    assert_eq!(v["error"]["params"]["holder"], "a");
    refused(&repo, &["--as", "b", "release", "A01-01"], 1, "task_held");
    refused(&repo, &["done", "A01-01"], 1, "task_held");
    let v = ok(&repo, &["--as", "b", "claim", "--take", "A01-01"]);
    assert_eq!(v["warnings"][0]["code"], "took_over");
    refused(
        &repo,
        &["--as", "b", "label", "A01-01", "shipped"],
        1,
        "label_not_on_ladder",
    );
    refused(
        &repo,
        &["--as", "b", "wait", "A01-01", "--on", " "],
        1,
        "waiting_on_missing",
    );
    ok(&repo, &["--as", "b", "done", "A01-01"]);
    refused(&repo, &["--as", "b", "claim", "A01-01"], 1, "task_closed");
    refused(
        &repo,
        &["add", "area", "A01", "Again"],
        1,
        "duplicate_area_id",
    );
    refused(&repo, &["add", "area", "bad id", "X"], 1, "invalid_id");
    refused(
        &repo,
        &["add", "task", "A01-01", "--area", "A01", "--title", "x"],
        1,
        "duplicate_task_id",
    );
    refused(
        &repo,
        &["add", "task", "Z-1", "--area", "A09", "--title", "x"],
        1,
        "unknown_area",
    );
    refused(&repo, &["list", "--area", "A09"], 1, "unknown_area");
    refused(&repo, &["claim"], 2, "bad_usage");
    refused(&repo, &["list", "--state", "finished"], 2, "bad_usage");
    refused(&repo, &["frobnicate"], 2, "bad_usage");
    refused(&tmp("none"), &["list"], 1, "seabag_not_found");
}

#[test]
fn a_busy_seabag_exits_3() {
    let repo = filled();
    let root = repo.join(".fliphopper/seabag");
    let lock = fliphopper_store::Lock::acquire(&root, Duration::ZERO).unwrap();
    let out = Command::new(FH)
        .current_dir(&repo)
        .args(["--json", "--as", "a", "claim", "A01-01"])
        .env("FLIPHOPPER_LOCK_WAIT_MS", "50")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["code"], "seabag_busy");
    assert_eq!(out.status.code(), Some(3));
    // Reading does not need the lock.
    ok(&repo, &["show", "A01-01"]);
    drop(lock);
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
}

#[test]
fn seabag_found_by_option_variable_or_search() {
    let repo = filled();
    let elsewhere = tmp("elsewhere");
    // --seabag: the seabag itself, or the folder holding it.
    ok(
        &elsewhere,
        &["--seabag", repo.to_str().unwrap(), "show", "A01-01"],
    );
    let root = repo.join(".fliphopper/seabag");
    ok(
        &elsewhere,
        &["--seabag", root.to_str().unwrap(), "show", "A01-01"],
    );
    let out = Command::new(FH)
        .current_dir(&elsewhere)
        .args(["--json", "claim", "A01-01"])
        .env("FLIPHOPPER_SEABAG", &repo)
        .env("FLIPHOPPER_HOLDER", "from-env")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["task"]["holder"], "from-env");
    refused(
        &elsewhere,
        &["--seabag", elsewhere.to_str().unwrap(), "list"],
        1,
        "seabag_not_found",
    );
}

/// Two agents claim the same task at the same moment, many times over: exactly one wins
/// each time, and a reader never sees a half-written or invalid file.
#[test]
fn racing_claims_have_exactly_one_winner() {
    const ROUNDS: usize = 30;
    const AGENTS: usize = 4;
    let repo = filled();
    for i in 0..ROUNDS {
        let id = format!("R-{i:02}");
        ok(
            &repo,
            &["add", "task", &id, "--area", "A02", "--title", "Race"],
        );
    }
    let root = repo.join(".fliphopper/seabag");
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        // A reader that keeps loading while the agents write.
        let reader = s.spawn(|| {
            let mut loads = 0;
            while !stop.load(Ordering::Relaxed) {
                let loaded = fliphopper_store::load(&root);
                assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
                assert_eq!(loaded.seabag.unwrap().tasks.len(), 3 + ROUNDS);
                loads += 1;
            }
            loads
        });
        for i in 0..ROUNDS {
            let id = format!("R-{i:02}");
            let children: Vec<_> = (0..AGENTS)
                .map(|a| {
                    Command::new(FH)
                        .current_dir(&repo)
                        .args(["--json", "--as", &format!("agent-{a}"), "claim", &id])
                        .env_remove("FLIPHOPPER_LOCK_WAIT_MS")
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                        .unwrap()
                })
                .collect();
            let results: Vec<(i32, Value)> = children
                .into_iter()
                .map(|c| {
                    let out = c.wait_with_output().unwrap();
                    (
                        out.status.code().unwrap(),
                        serde_json::from_slice(&out.stdout).unwrap(),
                    )
                })
                .collect();
            let winners: Vec<_> = results.iter().filter(|(c, _)| *c == 0).collect();
            assert_eq!(winners.len(), 1, "round {i}: {results:?}");
            for (code, v) in &results {
                if *code != 0 {
                    assert_eq!(*code, 1);
                    assert_eq!(v["error"]["code"], "task_held");
                }
            }
            let holder = winners[0].1["task"]["holder"].clone();
            assert_eq!(task_file(&repo, &id)["holder"], holder);
        }
        stop.store(true, Ordering::Relaxed);
        assert!(reader.join().unwrap() > 0);
    });
    let leftovers: Vec<_> = walk(&root)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".tmp-")
        })
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut all = Vec::new();
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            all.extend(walk(&p));
        }
        all.push(p);
    }
    all
}

/// In a real Git repository, `git status` shows only the seabag's text files: never the lock.
#[test]
fn git_sees_only_text_files() {
    let repo = tmp("git");
    let git = |args: &[&str]| Command::new("git").current_dir(&repo).args(args).output();
    match git(&["init", "-q"]) {
        Ok(o) if o.status.success() => {}
        _ => return, // No Git on this machine: nothing to check.
    }
    ok(&repo, &["init"]);
    ok(&repo, &["add", "area", "A01", "One"]);
    ok(
        &repo,
        &["add", "task", "A01-01", "--area", "A01", "--title", "T"],
    );
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    assert!(repo.join(".fliphopper/seabag/.lock").exists());
    let out = git(&["status", "--porcelain", "-uall"]).unwrap();
    let files: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| l[3..].to_string())
        .collect();
    assert_eq!(
        files,
        [
            ".fliphopper/seabag/.gitignore",
            ".fliphopper/seabag/areas/.keep",
            ".fliphopper/seabag/areas/A01.json",
            ".fliphopper/seabag/project.json",
            ".fliphopper/seabag/tasks/.keep",
            ".fliphopper/seabag/tasks/A01-01/task.json",
        ]
    );
}
