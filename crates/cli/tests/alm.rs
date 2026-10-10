//! `alm` run as a real program, the way agents run it: every command's success, its
//! refusals, its `--json` output and its exit codes, plus the race between two agents.
//! Tests use temporary folders only.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

const ALM: &str = env!("CARGO_BIN_EXE_alm");

fn tmp(name: &str) -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let p = std::env::temp_dir().join(format!(
        "alm-test-{name}-{n}-{}",
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&p).unwrap();
    p
}

/// `alm` in `dir`, with no Allomos settings inherited from the test's own environment.
fn alm(dir: &Path, args: &[&str]) -> Output {
    Command::new(ALM)
        .current_dir(dir)
        .args(args)
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env_remove("ALLOMOS_LOCK_WAIT_MS")
        .output()
        .unwrap()
}

/// `alm --json ...`: the exit code and the one JSON object on standard output.
fn fj(dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = alm(dir, &all);
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "one line of JSON: {stdout:?}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v["schema"], "alm/1");
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

/// A fresh Git-like repository (a `.git` folder is enough for `alm init`) with a taskbag
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
            "Open a taskbag",
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
        fs::read_to_string(repo.join(format!(".allomos/taskbag/tasks/{id}/task.json"))).unwrap();
    serde_json::from_str(&text).unwrap()
}

#[test]
fn both_names_run() {
    let allomos = Path::new(ALM).with_file_name(format!("allomos{}", std::env::consts::EXE_SUFFIX));
    let out = Command::new(allomos).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("0.1.0"));
}

#[test]
fn init_refuses_twice_and_needs_no_git() {
    let dir = tmp("plain");
    let v = ok(&dir, &["init"]);
    let root = dir.join(".allomos/taskbag");
    // The command reports the real path (on macOS, /var is /private/var).
    let reported = PathBuf::from(v["taskbag"].as_str().unwrap());
    assert_eq!(reported, root.canonicalize().unwrap());
    assert!(!root.join(".git").exists() && !dir.join(".git").exists());
    refused(&dir, &["init"], 1, "taskbag_exists");
    // From a subfolder of a repository, the taskbag goes at the repository's root.
    let repo = tmp("sub");
    fs::create_dir_all(repo.join(".git")).unwrap();
    fs::create_dir_all(repo.join("src/deep")).unwrap();
    ok(&repo.join("src/deep"), &["init"]);
    assert!(repo.join(".allomos/taskbag/project.json").is_file());
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
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
    let out = alm(&repo, &["list"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("A01  Tracking\n  A01-01  done  implemented  agent-a  Open a taskbag"));
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
    let text = String::from_utf8(alm(&repo, &["show", "A01-02"]).stdout).unwrap();
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
    refused(&tmp("none"), &["list"], 1, "taskbag_not_found");
}

#[test]
fn a_busy_taskbag_exits_3() {
    let repo = filled();
    let root = repo.join(".allomos/taskbag");
    let lock = allomos_store::Lock::acquire(&root, Duration::ZERO).unwrap();
    let out = Command::new(ALM)
        .current_dir(&repo)
        .args(["--json", "--as", "a", "claim", "A01-01"])
        .env("ALLOMOS_LOCK_WAIT_MS", "50")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["code"], "taskbag_busy");
    assert_eq!(out.status.code(), Some(3));
    // Reading does not need the lock.
    ok(&repo, &["show", "A01-01"]);
    drop(lock);
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
}

#[test]
fn taskbag_found_by_option_variable_or_search() {
    let repo = filled();
    let elsewhere = tmp("elsewhere");
    // --taskbag: the taskbag itself, or the folder holding it.
    ok(
        &elsewhere,
        &["--taskbag", repo.to_str().unwrap(), "show", "A01-01"],
    );
    let root = repo.join(".allomos/taskbag");
    ok(
        &elsewhere,
        &["--taskbag", root.to_str().unwrap(), "show", "A01-01"],
    );
    let out = Command::new(ALM)
        .current_dir(&elsewhere)
        .args(["--json", "claim", "A01-01"])
        .env("ALLOMOS_TASKBAG", &repo)
        .env("ALLOMOS_HOLDER", "from-env")
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["task"]["holder"], "from-env");
    refused(
        &elsewhere,
        &["--taskbag", elsewhere.to_str().unwrap(), "list"],
        1,
        "taskbag_not_found",
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
    let root = repo.join(".allomos/taskbag");
    let stop = AtomicBool::new(false);
    std::thread::scope(|s| {
        // A reader that keeps loading while the agents write.
        let reader = s.spawn(|| {
            let mut loads = 0;
            while !stop.load(Ordering::Relaxed) {
                let loaded = allomos_store::load(&root);
                assert!(loaded.issues.is_empty(), "{:?}", loaded.issues);
                assert_eq!(loaded.taskbag.unwrap().tasks.len(), 3 + ROUNDS);
                loads += 1;
            }
            loads
        });
        for i in 0..ROUNDS {
            let id = format!("R-{i:02}");
            let children: Vec<_> = (0..AGENTS)
                .map(|a| {
                    Command::new(ALM)
                        .current_dir(&repo)
                        .args(["--json", "--as", &format!("agent-{a}"), "claim", &id])
                        .env_remove("ALLOMOS_LOCK_WAIT_MS")
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

/// In a real Git repository, `git status` shows only the taskbag's text files: never the lock.
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
    assert!(repo.join(".allomos/taskbag/.lock").exists());
    let out = git(&["status", "--porcelain", "-uall"]).unwrap();
    let files: Vec<String> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| l[3..].to_string())
        .collect();
    assert_eq!(
        files,
        [
            ".allomos/taskbag/.gitignore",
            ".allomos/taskbag/areas/.keep",
            ".allomos/taskbag/areas/A01.json",
            ".allomos/taskbag/project.json",
            ".allomos/taskbag/tasks/.keep",
            ".allomos/taskbag/tasks/A01-01/task.json",
        ]
    );
}

#[test]
fn needs_ready_drop_and_init_options() {
    let repo = filled();
    // Next dependency: set, shown, cleared.
    let v = ok(&repo, &["needs", "A01-01", "owner review"]);
    assert_eq!(v["task"]["next_dependency"], "owner review");
    assert_eq!(
        task_file(&repo, "A01-01")["next_dependency"],
        "owner review"
    );
    let out = alm(&repo, &["show", "A01-01"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("next dependency: owner review"));
    ok(&repo, &["needs", "A01-01"]);
    assert!(task_file(&repo, "A01-01").get("next_dependency").is_none());
    // Ready from backlog; refused when already ready.
    assert_eq!(ok(&repo, &["ready", "A01-01"])["task"]["state"], "ready");
    refused(&repo, &["ready", "A01-01"], 1, "cannot_make_ready");
    // Drop: refused for another holder's task, then closed for good.
    ok(&repo, &["--as", "a", "claim", "A01-02"]);
    refused(&repo, &["--as", "b", "drop", "A01-02"], 1, "task_held");
    let d = ok(&repo, &["--as", "a", "drop", "A01-02"]);
    assert_eq!(
        (&d["task"]["state"], &d["task"]["holder"]),
        (&json!("dropped"), &Value::Null)
    );
    refused(&repo, &["drop", "A01-02"], 1, "task_closed");

    // init with an ID and a ladder; an empty ladder is refused and creates nothing.
    let dir = tmp("init-options");
    ok(
        &dir,
        &[
            "init",
            "--id",
            "allomos",
            "--labels",
            "drafted, owner verified",
        ],
    );
    let project: Value = serde_json::from_str(
        &fs::read_to_string(dir.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["id"], "allomos");
    assert_eq!(project["labels"], json!(["drafted", "owner verified"]));
    let empty = tmp("init-empty-ladder");
    refused(&empty, &["init", "--labels", " , "], 1, "ladder_empty");
    assert!(!empty.join(".allomos/taskbag").exists());
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A real Git repository with a first commit and a bare remote it can push to.
fn real_repo() -> (PathBuf, PathBuf) {
    let base = tmp("git");
    let remote = base.join("remote.git");
    let repo = base.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git(
        &base,
        &["init", "--quiet", "--bare", remote.to_str().unwrap()],
    );
    git(&repo, &["init", "--quiet", "-b", "main"]);
    git(&repo, &["config", "user.name", "Test"]);
    git(&repo, &["config", "user.email", "test@example.com"]);
    fs::write(repo.join("README.md"), "x\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "--quiet", "-m", "first"]);
    git(
        &repo,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    git(&repo, &["push", "--quiet", "-u", "origin", "main"]);
    (repo, remote)
}

#[test]
fn commit_and_push_share_only_the_taskbag() {
    let (repo, remote) = real_repo();
    let v = ok(&repo, &["init", "--write-agent-files", "--commit"]);
    assert_eq!(v["warnings"], json!([]));
    assert_eq!(v["agent_files"], json!(["AGENTS.md", "CLAUDE.md"]));
    assert_eq!(git(&repo, &["log", "-1", "--format=%s"]), "alm: init");
    // The agent files went into the same commit as the new taskbag.
    let files = git(&repo, &["show", "--name-only", "--format=", "HEAD"]);
    assert!(
        files.contains("AGENTS.md") && files.contains("CLAUDE.md"),
        "{files}"
    );
    ok(&repo, &["add", "area", "A01", "Tracking", "--commit"]);
    ok(
        &repo,
        &[
            "add", "task", "A01-01", "--area", "A01", "--title", "T", "--commit",
        ],
    );
    // Another changed file stays out of alm's commit.
    fs::write(repo.join("README.md"), "changed\n").unwrap();
    let v = ok(&repo, &["--as", "agent-a", "claim", "A01-01", "--push"]);
    assert_eq!(v["warnings"], json!([]), "{v}");
    assert_eq!(
        git(&repo, &["log", "-1", "--format=%s%n%b"]),
        "alm: claim A01-01\nAgent: agent-a"
    );
    assert_eq!(
        git(&repo, &["show", "--name-only", "--format=", "HEAD"]),
        ".allomos/taskbag/tasks/A01-01/task.json"
    );
    assert_eq!(git(&repo, &["status", "--porcelain"]), "M README.md");
    // The push landed on the remote.
    assert_eq!(
        git(&remote, &["log", "-1", "--format=%s", "main"]),
        "alm: claim A01-01"
    );
    // A change that changes nothing makes no commit.
    let before = git(&repo, &["rev-parse", "HEAD"]);
    ok(&repo, &["--as", "agent-a", "claim", "A01-01", "--commit"]);
    assert_eq!(git(&repo, &["rev-parse", "HEAD"]), before);
}

#[test]
fn commit_outside_git_warns_and_keeps_the_change() {
    let dir = tmp("no-git");
    ok(&dir, &["init"]);
    ok(&dir, &["add", "area", "A01", "Tracking"]);
    let v = ok(
        &dir,
        &[
            "add", "task", "A01-01", "--area", "A01", "--title", "T", "--commit",
        ],
    );
    assert_eq!(v["warnings"][0]["code"], "commit_failed", "{v}");
    assert!(
        dir.join(".allomos/taskbag/tasks/A01-01/task.json")
            .is_file()
    );
}

#[test]
fn init_asks_the_agent_or_writes_when_told() {
    // By default: no files written; the request is printed and in `agent_setup`.
    let dir = tmp("agent-ask");
    let v = ok(&dir, &["init"]);
    assert_eq!(v["agent_files"], json!([]));
    assert!(
        v["agent_setup"]
            .as_str()
            .unwrap()
            .contains("1. Open AGENTS.md")
    );
    assert!(!dir.join("AGENTS.md").exists() && !dir.join("CLAUDE.md").exists());
    let text = tmp("agent-ask-text");
    let out = alm(&text, &["init"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Next step for the AI agent"));

    // --write-agent-files: written directly, existing text kept.
    let dir = tmp("agent-files");
    fs::write(dir.join("AGENTS.md"), "# Ours\n").unwrap();
    let v = ok(&dir, &["init", "--write-agent-files"]);
    assert_eq!(v["agent_files"], json!(["AGENTS.md", "CLAUDE.md"]));
    assert_eq!(v["agent_setup"], Value::Null);
    let agents = fs::read_to_string(dir.join("AGENTS.md")).unwrap();
    assert!(agents.starts_with("# Ours\n\n## Tasks: Allomos") && agents.contains("alm next"));
    assert_eq!(
        fs::read_to_string(dir.join("CLAUDE.md")).unwrap(),
        "@AGENTS.md\n"
    );
}

#[test]
fn plan_tasks_in_iterations_and_list_by_iteration() {
    let repo = filled();
    // No iterations yet: every task is unplanned.
    let v = ok(&repo, &["list", "--by", "iteration"]);
    assert_eq!(v["iterations"], json!([]));
    assert_eq!(v["unplanned"].as_array().unwrap().len(), 3);

    let v = ok(
        &repo,
        &[
            "add",
            "iteration",
            "I01",
            "v0.1",
            "--release",
            "v0.1",
            "--end",
            "2026-10-07",
        ],
    );
    assert_eq!(v["iteration"]["order"], 1);
    ok(&repo, &["add", "iteration", "I02", "v0.2"]);
    refused(
        &repo,
        &["add", "iteration", "I02", "again"],
        1,
        "duplicate_iteration_id",
    );
    refused(
        &repo,
        &["add", "iteration", "I03", "x", "--start", "soon"],
        1,
        "invalid_date",
    );
    refused(
        &repo,
        &["add", "iteration", "I03", " "],
        1,
        "iteration_name_missing",
    );
    let file: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/iterations/I01.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        file,
        json!({"id": "I01", "name": "v0.1", "order": 1, "end": "2026-10-07", "release": "v0.1"})
    );

    // Planning ignores who holds the task, and works on finished tasks.
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(&repo, &["--as", "a", "done", "A01-01"]);
    let v = ok(&repo, &["--as", "b", "plan", "A01-01", "I01"]);
    assert_eq!(v["task"]["iteration"], "I01");
    ok(&repo, &["plan", "A02-01", "I02"]);
    refused(&repo, &["plan", "A01-02", "I09"], 1, "unknown_iteration");
    refused(&repo, &["plan", "Z", "I01"], 1, "task_not_found");
    assert_eq!(task_file(&repo, "A01-01")["iteration"], "I01");

    let v = ok(&repo, &["list", "--by", "iteration"]);
    let groups = v["iterations"].as_array().unwrap();
    let ids: Vec<_> = groups.iter().map(|g| g["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["I01", "I02"]);
    assert_eq!(groups[0]["tasks"][0]["id"], "A01-01");
    assert_eq!(groups[1]["tasks"][0]["id"], "A02-01");
    assert_eq!(v["unplanned"][0]["id"], "A01-02");
    let v = ok(&repo, &["list", "--iteration", "I02"]);
    assert_eq!(v["areas"][1]["tasks"][0]["id"], "A02-01");
    assert_eq!(v["areas"][0]["tasks"], json!([]));
    refused(
        &repo,
        &["list", "--iteration", "I09"],
        1,
        "unknown_iteration",
    );
    refused(&repo, &["list", "--by", "week"], 2, "bad_usage");

    let text = String::from_utf8(alm(&repo, &["list", "--by", "iteration"]).stdout).unwrap();
    assert!(
        text.starts_with("I01  v0.1  (v0.1, until 2026-10-07)\n  A01-01  done"),
        "{text}"
    );
    assert!(
        text.ends_with("Unplanned\n  A01-02  backlog  -  -  List\n"),
        "{text}"
    );
    let text = String::from_utf8(alm(&repo, &["show", "A02-01"]).stdout).unwrap();
    assert!(text.contains("iteration: I02\n"), "{text}");

    // No iteration given: the task is taken out and becomes unplanned; its file loses the field.
    let v = ok(&repo, &["plan", "A02-01"]);
    assert_eq!(v["task"]["iteration"], Value::Null);
    assert!(task_file(&repo, "A02-01").get("iteration").is_none());
}

/// `alm confirm` in `project` (which has no taskbag yet), answered by a stand-in for the app.
fn confirm_answered(project: &Path, answer: Option<bool>, timeout: &str) -> (i32, Value) {
    let plan = project.join("plan.md");
    fs::write(&plan, "## Plan\n\n- 3 areas\n- 5 tasks\n").unwrap();
    let taskbag = project.join(".allomos/taskbag");
    let child = Command::new(ALM)
        .current_dir(project)
        .args(["--json", "--as", "claude/test", "--taskbag"])
        .arg(&taskbag)
        .args([
            "confirm",
            "--title",
            "Move tasks into Allomos",
            "--plan",
            "plan.md",
        ])
        .args(["--timeout", timeout])
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env("ALLOMOS_NO_LAUNCH", "1")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(yes) = answer {
        // The app: wait for the question, check it, answer it.
        let asked = (0..100)
            .find_map(|_| {
                std::thread::sleep(Duration::from_millis(100));
                allomos_store::pending_confirms(project).pop()
            })
            .expect("the question appears");
        assert_eq!(asked.title, "Move tasks into Allomos");
        assert!(asked.plan.contains("- 5 tasks"));
        assert_eq!(asked.from.as_deref(), Some("claude/test"));
        allomos_store::answer_confirm(project, &asked.id, yes).unwrap();
    }
    let out = child.wait_with_output().unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    // Settled questions are tidied away; only the folder's .gitignore stays.
    let left: Vec<_> = fs::read_dir(project.join(".allomos/requests"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, [".gitignore"]);
    assert!(!taskbag.exists(), "asking never creates the taskbag");
    (out.status.code().unwrap(), v)
}

#[test]
fn confirm_waits_for_the_answer_in_the_app() {
    let project = tmp("confirm-yes");
    let (code, v) = confirm_answered(&project, Some(true), "20");
    assert_eq!(
        (code, &v["ok"], &v["answer"]),
        (0, &json!(true), &json!("yes"))
    );

    let project = tmp("confirm-no");
    let (code, v) = confirm_answered(&project, Some(false), "20");
    assert_eq!((code, &v["error"]["code"]), (1, &json!("confirm_declined")));

    let project = tmp("confirm-none");
    let (code, v) = confirm_answered(&project, None, "1");
    assert_eq!((code, &v["error"]["code"]), (1, &json!("confirm_timeout")));

    // A plan that cannot be read is refused before anything is asked.
    let project = tmp("confirm-missing");
    refused(
        &project,
        &["confirm", "--title", "x", "--plan", "nope.md"],
        3,
        "read_failed",
    );
    assert!(!project.join(".allomos").exists());
}

#[test]
fn labels_change_the_ladder_without_stranding_a_task() {
    let repo = filled();
    let v = ok(&repo, &["labels", "implemented, connected,owner verified"]);
    assert_eq!(
        v["labels"],
        json!(["implemented", "connected", "owner verified"])
    );
    ok(&repo, &["label", "A01-01", "connected"]);
    refused(&repo, &["labels", "implemented"], 1, "label_not_on_ladder");
    refused(&repo, &["labels", " , "], 1, "ladder_empty");
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["labels"][1], "connected");
}

#[test]
fn brief_and_guide_tell_agents_about_allomos() {
    let repo = filled();
    let v = ok(&repo, &["brief"]);
    let brief = v["brief"].as_str().unwrap();
    assert!(brief.starts_with("## Tasks: Allomos"));
    assert!(brief.contains("alm guide customize"));
    // Without a taskbag the brief is still printed, as plain text.
    let out = alm(&tmp("no-taskbag"), &["brief"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("alm guide terms")
    );

    let v = ok(&repo, &["guide", "customize"]);
    assert!(v["guide"].as_str().unwrap().contains("The six states"));
    let all = ok(&repo, &["guide"]);
    assert!(all["guide"].as_str().unwrap().contains("# Allomos terms"));
    let (code, v) = fj(&repo, &["guide", "nope"]);
    assert_eq!((code, &v["error"]["code"]), (2, &json!("bad_usage")));
}

#[test]
fn add_task_without_an_id_takes_the_next_in_its_area() {
    let repo = filled();
    // filled() has A01-01, A01-02 and A02-01.
    let v = ok(
        &repo,
        &[
            "add",
            "task",
            "--area",
            "A01",
            "--title",
            "A customer orders a cake",
        ],
    );
    assert_eq!(v["task"]["id"], "A01-03");
    assert_eq!(v["task"]["state"], "backlog");
    let v = ok(
        &repo,
        &["add", "task", "--area", "A02", "--title", "Charts"],
    );
    assert_eq!(v["task"]["id"], "A02-02");
    refused(
        &repo,
        &["add", "task", "--area", "A09", "--title", "x"],
        1,
        "unknown_area",
    );
    let (_, v) = fj(&repo, &["guide", "tasks"]);
    assert!(v["guide"].as_str().unwrap().contains("# How to add a task"));
    assert!(
        ok(&repo, &["brief"])["brief"]
            .as_str()
            .unwrap()
            .contains("alm guide tasks")
    );
}

/// `alm --json …` with its own settings folder, so the person's real app settings are never read.
fn fj_settings(dir: &Path, settings: &Path, args: &[&str]) -> (i32, Value) {
    let out = Command::new(ALM)
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env("ALLOMOS_SETTINGS_DIR", settings)
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    (out.status.code().unwrap(), v)
}

#[test]
fn task_language_follows_the_app_until_set() {
    let repo = filled();
    let settings = tmp("settings");
    // A new taskbag says "ui".
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["task_language"], "ui");
    // The app saved Chinese: that is the language.
    fs::write(
        settings.join("settings.json"),
        r#"{ "ui_language": "zh-CN" }"#,
    )
    .unwrap();
    let (code, v) = fj_settings(&repo, &settings, &["task-language"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["task_language"],
        json!({ "language": "zh-CN", "setting": "ui", "source": "app" })
    );
    // Set a code (the underscore spelling works too), then back to ui.
    let (_, v) = fj_settings(&repo, &settings, &["task_language", "fr"]);
    assert_eq!(v["task_language"]["language"], "fr");
    assert_eq!(v["task_language"]["source"], "taskbag");
    let (code, v) = fj_settings(&repo, &settings, &["task-language", "French!"]);
    assert_eq!((code, &v["error"]["code"]), (1, &json!("invalid_language")));
    let (_, v) = fj_settings(&repo, &settings, &["task-language", "ui"]);
    assert_eq!(v["task_language"]["language"], "zh-CN");
}

#[test]
fn a_fliphopper_project_still_opens_and_migrate_moves_it() {
    // A project made by FlipHopper: `.fliphopper/seabag/` with format `fliphopper/1`.
    let repo = tmp("legacy");
    fs::create_dir(repo.join(".git")).unwrap();
    ok(&repo, &["init", "--name", "Old"]);
    ok(&repo, &["add", "area", "A01", "Area"]);
    ok(&repo, &["add", "task", "--area", "A01", "--title", "Kept"]);
    let new = repo.join(".allomos/taskbag");
    let old = repo.join(".fliphopper/seabag");
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    fs::rename(&new, &old).unwrap();
    fs::remove_dir(repo.join(".allomos")).unwrap();
    let project = old.join("project.json");
    let text = fs::read_to_string(&project)
        .unwrap()
        .replace("allomos/1", "fliphopper/1");
    fs::write(&project, text).unwrap();

    // Found from the folder, by the old option name and by the old environment setting.
    assert_eq!(
        ok(&repo, &["list"])["areas"][0]["tasks"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    ok(
        &repo,
        &["--seabag", repo.to_str().unwrap(), "show", "A01-01"],
    );
    let out = Command::new(ALM)
        .current_dir(std::env::temp_dir())
        .args(["--json", "show", "A01-01"])
        .env_remove("ALLOMOS_TASKBAG")
        .env("FLIPHOPPER_SEABAG", &repo)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );

    // `alm migrate` moves it, rewrites the format, and removes the empty old folder.
    let v = ok(&repo, &["migrate"]);
    assert_eq!(v["moved"], true);
    assert!(new.join("project.json").is_file() && !repo.join(".fliphopper").exists());
    assert!(
        fs::read_to_string(new.join("project.json"))
            .unwrap()
            .contains("\"allomos/1\"")
    );
    assert_eq!(ok(&repo, &["show", "A01-01"])["task"]["title"], "Kept");
    assert_eq!(ok(&repo, &["migrate"])["moved"], false);
}
