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
    assert!(String::from_utf8_lossy(&out.stdout).contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn init_refuses_twice_and_needs_no_git() {
    let dir = tmp("plain");
    let v = ok(&dir, &["init"]);
    let root = dir.join(".allomos/taskbag");
    // The command reports the taskbag's folder. Compared as folders, not as text: on macOS /var
    // is /private/var, and on Windows the temporary folder may come in its short 8.3 form
    // (RUNNER~1) while canonicalize gives the long one with a `\\?\` prefix.
    let reported = v["taskbag"].as_str().unwrap();
    assert!(!reported.starts_with(r"\\?\"), "a plain path: {reported}");
    assert_eq!(
        PathBuf::from(reported).canonicalize().unwrap(),
        root.canonicalize().unwrap()
    );
    assert!(!root.join(".git").exists() && !dir.join(".git").exists());
    refused(&dir, &["init"], 1, "taskbag_exists");
    // `alm` reads no Git (decision 0017): in a subfolder of a repository the taskbag goes in that
    // subfolder, not at the repository's root, and `init` prints no Git commands.
    let repo = tmp("sub");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let deep = repo.join("src/deep");
    fs::create_dir_all(&deep).unwrap();
    let v = ok(&deep, &["init"]);
    assert!(deep.join(".allomos/taskbag/project.json").is_file());
    assert!(!repo.join(".allomos").exists());
    assert!(
        v.get("git_commands").is_none() && v.get("changed_files").is_none(),
        "{v}"
    );
    let project: Value = serde_json::from_str(
        &fs::read_to_string(deep.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["name"], "deep");
    let text = String::from_utf8(alm(&tmp("init-text"), &["init"]).stdout).unwrap();
    assert!(
        !text.contains("Commit and push") && !text.contains("Commit only these files"),
        "{text}"
    );
}

#[test]
fn work_one_task_from_claim_to_done() {
    let repo = filled();
    let sub = repo.join("src");
    fs::create_dir_all(&sub).unwrap();

    let v = ok(
        &sub,
        &["--as", "agent-a", "claim", "A01-01", "--branch", "main"],
    );
    assert_eq!(v["command"], "claim");
    assert_eq!(v["branches"], json!({ "base": "main", "working": "main" }));
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

    // Below the top label: finished with no warning; `alm` does not judge (Task-A03-15).
    let v = ok(&sub, &["--as", "agent-a", "done", "A01-01"]);
    assert_eq!(v["task"]["state"], "done");
    assert_eq!(v["warnings"], json!([]));

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
    assert_eq!(v["tasks"], json!([]));
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
    assert_eq!(v["tasks"][0]["id"], "A01-02");
    assert_eq!(v["tasks"][0]["handoff_note"], "tests pass; docs left");
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
fn next_lists_every_free_ready_task_with_the_rule_for_choosing() {
    let repo = filled();
    ok(
        &repo,
        &["add", "task", "A01-03", "--area", "A01", "--title", "Sort"],
    );
    ok(&repo, &["add", "iteration", "I01", "v0.1"]);
    ok(&repo, &["add", "iteration", "I02", "v0.2"]);
    for id in ["A01-01", "A01-02", "A01-03", "A02-01"] {
        ok(&repo, &["ready", id]);
    }
    ok(&repo, &["plan", "A01-02", "I02"]);
    ok(&repo, &["plan", "A02-01", "I01"]);
    // Held and non-ready tasks are not listed.
    ok(&repo, &["--as", "a", "claim", "A01-03"]);
    ok(
        &repo,
        &[
            "add",
            "task",
            "A01-04",
            "--area",
            "A01",
            "--title",
            "Not ready",
        ],
    );
    // A handoff covering a listed task comes with it.
    ok(
        &repo,
        &["handoff", "--as", "claude/run", "--tasks", "A01-02"],
    );

    let v = ok(&repo, &["next"]);
    let ids: Vec<&str> = v["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    // Earliest iteration first, unplanned last.
    assert_eq!(ids, ["A02-01", "A01-02", "A01-01"]);
    assert_eq!(v["tasks"][1]["handoff"]["writer"], "claude/run");
    assert_eq!(v["tasks"][0]["handoff"], Value::Null);
    let rule = v["instruction"].as_str().unwrap();
    assert!(
        rule.starts_with("Take the task the person named, if any."),
        "{rule}"
    );
    assert!(
        rule.contains("Pull first, then claim it with `alm claim <id> --as <your session name> --branch <the branch you are on>`")
    );
    assert!(rule.contains("until your claim is pushed"), "{rule}");
    assert!(!rule.contains("--push"), "{rule}");
    assert!(v["handoff_rules"]["quota_gate"].is_number());

    // `--iteration` narrows the list; an unknown iteration is refused.
    let v = ok(&repo, &["next", "--iteration", "I02"]);
    assert_eq!(v["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(v["tasks"][0]["id"], "A01-02");
    refused(
        &repo,
        &["next", "--iteration", "I09"],
        1,
        "unknown_iteration",
    );

    // Text for people: every task, then the rule and the handoff rules.
    let text = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(text.contains("A02-01  ready") && text.contains("  iteration: I01"));
    assert!(text.contains("Take the task the person named") && text.contains("Handoff rules"));

    // Nothing to take in an iteration with no free ready task.
    ok(&repo, &["add", "iteration", "I03", "v0.3"]);
    let v = ok(&repo, &["next", "--iteration", "I03"]);
    assert_eq!(v["tasks"], json!([]));
    assert!(
        v["instruction"]
            .as_str()
            .unwrap()
            .starts_with("Nothing to take")
    );
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
    let (events, files): (Vec<String>, Vec<String>) = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| l[3..].to_string())
        .partition(|f| f.starts_with(".allomos/taskbag/events/"));
    // One event per change: init, add area, add task, claim (Task-A01-17).
    assert_eq!(events.len(), 4, "{events:?}");
    assert!(events.iter().all(|f| f.ends_with(".json")));
    assert_eq!(
        files,
        [
            ".allomos/taskbag/.gitattributes",
            ".allomos/taskbag/.gitignore",
            ".allomos/taskbag/areas/.keep",
            ".allomos/taskbag/areas/A01.json",
            ".allomos/taskbag/handoff-template.md",
            ".allomos/taskbag/handoffs/.keep",
            ".allomos/taskbag/project-notes.md",
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

/// Run the Git commands `alm` printed (its `git_commands`) in `dir`, as an agent would.
fn run_printed(dir: &Path, v: &Value) {
    let commands = v["git_commands"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"));
    assert!(!commands.is_empty(), "{v}");
    for c in commands {
        let c = c.as_str().unwrap();
        let out = if cfg!(windows) {
            Command::new("cmd")
                .current_dir(dir)
                .args(["/C", c])
                .output()
        } else {
            Command::new("sh").current_dir(dir).args(["-c", c]).output()
        }
        .unwrap();
        assert!(
            out.status.success(),
            "{c}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Decision 0017: `alm` runs no Git. After a change it prints the commands that share exactly
/// that change's files; the agent runs them, and other changes stay as they are.
#[test]
fn printed_git_commands_share_only_that_change() {
    let (repo, remote) = real_repo();
    let v = ok(&repo, &["init", "--write-agent-files"]);
    assert_eq!(v["agent_files"], json!(["AGENTS.md", "CLAUDE.md"]));
    // `init` prints no Git commands (row 3): no agent is connected yet. Nothing was committed.
    assert!(v.get("git_commands").is_none(), "{v}");
    assert_eq!(git(&repo, &["log", "-1", "--format=%s"]), "first");
    git(&repo, &["add", "--", ".allomos", "AGENTS.md", "CLAUDE.md"]);
    git(&repo, &["commit", "--quiet", "-m", "Add Allomos taskbag"]);
    let v = ok(&repo, &["add", "area", "A01", "Tracking"]);
    run_printed(&repo, &v);
    assert_eq!(
        git(&repo, &["log", "-1", "--format=%s%n%b"]),
        "alm: add area A01\nAgent: <your session name>"
    );
    let v = ok(&repo, &["add", "task", "--area", "A01", "--title", "T"]);
    assert_eq!(v["git_instruction"], "Commit and push this change:");
    run_printed(&repo, &v);
    assert_eq!(
        git(&repo, &["log", "-1", "--format=%s"]),
        "alm: add task A01-01"
    );
    // Unrelated changes: a changed file of the project, and another task added but not committed.
    fs::write(repo.join("README.md"), "changed\n").unwrap();
    let other = ok(&repo, &["add", "task", "--area", "A01", "--title", "Other"]);
    let v = ok(&repo, &["--as", "agent-a", "claim", "A01-01"]);
    // Only the task and the event that records its change (Task-A01-17).
    let files: Vec<&str> = v["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 2, "{v}");
    assert!(
        files[0].starts_with("events/") && files[0].ends_with(".json"),
        "{v}"
    );
    assert_eq!(files[1], "tasks/A01-01/task.json");
    // The claim says to push at once, before the work starts (row 2).
    assert_eq!(
        v["git_instruction"],
        "Commit and push this change now, before you start the work. Until it is pushed, other sessions do not see your claim and may take this task too:"
    );
    let commands: Vec<&str> = v["git_commands"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert_eq!(commands.len(), 3, "{v}");
    assert!(
        commands[0].ends_with(&format!(" add -- {} {}", files[0], files[1])),
        "{v}"
    );
    assert!(
        commands[1].ends_with(&format!(
            " commit -m \"alm: claim A01-01\" -m \"Agent: agent-a\" -- {} {}",
            files[0], files[1]
        )),
        "{v}"
    );
    assert!(commands[2].ends_with(" push"), "{v}");
    // alm ran no Git: the claim is not committed until the agent runs the commands.
    assert_eq!(
        git(&repo, &["log", "-1", "--format=%s"]),
        "alm: add task A01-01"
    );
    run_printed(&repo, &v);
    assert_eq!(
        git(&repo, &["log", "-1", "--format=%s%n%b"]),
        "alm: claim A01-01\nAgent: agent-a"
    );
    let shown = git(&repo, &["show", "--name-only", "--format=", "HEAD"]);
    assert_eq!(
        shown.lines().collect::<Vec<_>>(),
        [
            format!(".allomos/taskbag/{}", files[0]),
            ".allomos/taskbag/tasks/A01-01/task.json".to_string()
        ]
    );
    // The other changes are left as they are.
    let status = git(&repo, &["status", "--porcelain"]);
    assert!(
        status.lines().any(|l| l.trim() == "M README.md"),
        "{status}"
    );
    for f in other["changed_files"].as_array().unwrap() {
        let f = f.as_str().unwrap();
        let top = f.split('/').next().unwrap();
        assert!(
            status.contains(&format!(".allomos/taskbag/{top}/")),
            "{f}: {status}"
        );
    }
    // The push landed on the remote.
    assert_eq!(
        git(&remote, &["log", "-1", "--format=%s", "main"]),
        "alm: claim A01-01"
    );
    // A change that changes nothing names no files and no commands.
    let v = ok(&repo, &["--as", "agent-a", "claim", "A01-01"]);
    assert_eq!(v["changed_files"], json!([]), "{v}");
    assert_eq!(v["git_commands"], json!([]), "{v}");
    assert_eq!(
        v["git_instruction"],
        "Nothing changed, so there is nothing to commit."
    );
    // For people: the same commands, then the rule.
    let out = alm(
        &repo,
        &["--as", "agent-a", "label", "A01-01", "implemented"],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("\nCommit and push this change:\n  git -C "),
        "{text}"
    );
    assert!(
        text.contains("commit -m \"alm: label A01-01\" -m \"Agent: agent-a\" -- events/"),
        "{text}"
    );
    assert!(
        text.ends_with("Commit only these files; leave other changes as they are.\n"),
        "{text}"
    );
}

/// `alm` never runs Git: every command works with no `git` program at all, in a folder outside
/// any repository; and the removed options are refused as unknown.
#[test]
fn alm_needs_no_git_and_refuses_the_removed_options() {
    let dir = tmp("no-git");
    let empty = tmp("no-git-path");
    let run = |args: &[&str]| {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        let out = Command::new(ALM)
            .current_dir(&dir)
            .args(&all)
            .env("PATH", &empty)
            .env_remove("ALLOMOS_TASKBAG")
            .env_remove("ALLOMOS_HOLDER")
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code().unwrap(), v)
    };
    for args in [
        &["init"][..],
        &["add", "area", "A01", "Tracking"],
        &[
            "add", "task", "--area", "A01", "--title", "T", "--branch", "main",
        ],
        &["--as", "a", "claim", "A01-01", "--branch", "main"],
        &["--as", "a", "label", "A01-01", "human verified"],
        &["--as", "a", "handoff", "--tasks", "A01-01"],
        &["--as", "a", "done", "A01-01"],
        &["next"],
    ] {
        let (code, v) = run(args);
        assert_eq!((code, &v["ok"]), (0, &json!(true)), "{args:?}: {v}");
        assert!(
            v["warnings"].as_array().unwrap().is_empty(),
            "{args:?}: {v}"
        );
    }
    assert!(!dir.join(".git").exists());
    for flag in ["--push", "--commit"] {
        let (code, v) = run(&["--as", "a", "label", "A01-01", "implemented", flag]);
        assert_eq!(code, 2, "{flag}: {v}");
        assert_eq!(v["error"]["code"], "bad_usage", "{flag}: {v}");
        assert!(
            v["error"]["message"].as_str().unwrap().contains(flag),
            "{v}"
        );
    }
}

/// Every file in the taskbag and its contents, except the lock.
fn contents(root: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
    walk(root)
        .into_iter()
        .filter(|p| p.is_file() && p.file_name().unwrap() != ".lock")
        .map(|p| {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            (rel, fs::read(&p).unwrap())
        })
        .collect()
}

/// Each changing command names exactly the files it wrote, added or removed: no more (other
/// uncommitted changes are present all along) and no fewer.
#[test]
fn each_change_names_exactly_the_files_it_wrote() {
    let repo = filled();
    let root = repo.join(".allomos/taskbag");
    let steps = tmp("steps").join("steps.md");
    fs::write(&steps, "Open it.\n").unwrap();
    let template = tmp("template").join("t.md");
    fs::write(&template, "# Handoff\n\nWhat next.\n").unwrap();
    let (steps, template) = (steps.to_str().unwrap(), template.to_str().unwrap());
    let commands: Vec<Vec<&str>> = vec![
        vec!["add", "iteration", "I01", "One"],
        vec!["add", "iteration", "I02", "Two"],
        vec![
            "add",
            "task",
            "--area",
            "A02",
            "--title",
            "With text",
            "--description",
            "Why.",
        ],
        vec!["--as", "a", "claim", "A01-01"],
        vec!["--as", "a", "label", "A01-01", "implemented"],
        vec!["--as", "a", "needs", "A01-01", "a person"],
        vec!["--as", "a", "note", "A01-01", "half way"],
        vec!["--as", "a", "describe", "A01-01", "--text", "New words."],
        vec!["--as", "a", "describe", "A01-01", "--from", steps],
        vec!["--as", "a", "describe", "A01-01", "--text", ""],
        vec![
            "--as", "a", "wait", "A01-01", "--on", "approve", "--who", "Ann",
        ],
        vec!["--as", "a", "ready", "A02-01"],
        vec!["--as", "a", "claim", "A01-02"],
        vec!["--as", "a", "block", "A01-02", "--by", "A02-01"],
        vec!["--as", "a", "unblock", "A01-02"],
        vec!["plan", "A01-02", "I01"],
        vec!["--as", "a", "title", "A01-02", "Lists"],
        vec!["--as", "a", "release", "A01-02", "--note", "half done"],
        vec!["--as", "a", "drop", "A02-01"],
        vec!["--as", "b", "claim", "A01-02"],
        vec![
            "--as",
            "b",
            "done",
            "A01-02",
            "--without-human-check",
            "loop run",
            "--steps",
            steps,
        ],
        vec!["checked", "A01-02"],
        vec!["rename", "area", "A02", "Graphs"],
        vec!["change", "iteration", "I01", "--release", "v0.1"],
        vec!["delete", "iteration", "I02"],
        vec![
            "labels",
            "drafted,implemented,locally verified,deployed and verified,human verified",
        ],
        vec!["rename", "label", "drafted", "sketched"],
        vec!["project", "--name", "Demo 2"],
        vec!["task-language", "ja"],
        vec!["handoff-rules", "--quota", "5"],
        vec!["handoff-template", "--set", template],
        vec!["--as", "b", "handoff", "--tasks", "A01-02"],
    ];
    // An unrelated uncommitted change sits in the taskbag the whole time.
    fs::write(root.join("project-notes.md"), "Changed by hand.\n").unwrap();
    for args in &commands {
        let before = contents(&root);
        let v = ok(&repo, args);
        let after = contents(&root);
        let mut changed: Vec<&String> = after
            .iter()
            .filter(|(f, text)| before.get(*f) != Some(*text))
            .map(|(f, _)| f)
            .chain(before.keys().filter(|f| !after.contains_key(*f)))
            .collect();
        changed.sort();
        let named: Vec<&str> = v["changed_files"]
            .as_array()
            .unwrap_or_else(|| panic!("{args:?}: {v}"))
            .iter()
            .map(|f| f.as_str().unwrap())
            .collect();
        assert_eq!(named, changed, "{args:?}");
        assert!(!named.is_empty(), "{args:?}");
        let commands = v["git_commands"].as_array().unwrap();
        assert_eq!(commands.len(), 3, "{args:?}: {v}");
        for c in commands.iter().take(2) {
            assert!(
                c.as_str().unwrap().ends_with(&format!(
                    " -- {}",
                    named
                        .iter()
                        .map(|f| if f.contains(' ') {
                            format!("\"{f}\"")
                        } else {
                            f.to_string()
                        })
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                "{args:?}: {c}"
            );
        }
    }
    // Removing a finished handoff names the removed file.
    let file = ok(&repo, &["handoffs"])["handoffs"][0]["file"]
        .as_str()
        .unwrap()
        .to_string();
    let v = ok(&repo, &["handoff", "--remove", &file]);
    assert_eq!(v["changed_files"], json!([file]));
}

/// `alm describe` and `alm note` change a task's description and handoff note through the same
/// rules as the app's task pane (Task-A01-37): the holder rule, one history event each named as
/// the app names it, `description.md` removed when emptied, and Git commands naming only the
/// files written.
#[test]
fn describe_and_note_change_a_task_like_the_app() {
    let repo = filled();
    let root = repo.join(".allomos/taskbag");
    let desc = root.join("tasks/A01-01/description.md");
    let task_json = "tasks/A01-01/task.json";
    ok(&repo, &["--as", "a", "claim", "A01-01"]);

    // A new description from text: description.md rewritten; only it and one event are named.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "describe",
            "A01-01",
            "--text",
            "# Goal\n\nOpen it.  \n\n",
        ],
    );
    assert_eq!(fs::read_to_string(&desc).unwrap(), "# Goal\n\nOpen it.\n");
    assert_eq!(v["task"]["description"], "# Goal\n\nOpen it.\n");
    let files = v["changed_files"].as_array().unwrap();
    assert_eq!(files.len(), 2, "{v}");
    assert!(files[0].as_str().unwrap().starts_with("events/"), "{v}");
    assert_eq!(files[1], "tasks/A01-01/description.md", "{v}");
    let commit = v["git_commands"][1].as_str().unwrap();
    assert!(commit.contains("alm: describe A01-01"), "{commit}");
    assert!(commit.contains("Agent: a"), "{commit}");

    // From a Markdown file.
    let file = tmp("describe").join("d.md");
    fs::write(&file, "From a file.\n").unwrap();
    ok(
        &repo,
        &[
            "--as",
            "a",
            "describe",
            "A01-01",
            "--from",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(fs::read_to_string(&desc).unwrap(), "From a file.\n");
    assert_eq!(
        ok(&repo, &["show", "A01-01"])["task"]["description"],
        "From a file.\n"
    );

    // Emptied: description.md removed, and named among the changed files.
    let v = ok(&repo, &["--as", "a", "describe", "A01-01", "--text", ""]);
    assert!(!desc.exists());
    assert!(
        v["changed_files"]
            .as_array()
            .unwrap()
            .contains(&json!("tasks/A01-01/description.md")),
        "{v}"
    );

    // The handoff note: set, then cleared with no text.
    let v = ok(
        &repo,
        &["--as", "a", "note", "A01-01", "Tests pass; docs left."],
    );
    assert_eq!(
        task_file(&repo, "A01-01")["handoff_note"],
        "Tests pass; docs left."
    );
    let files: Vec<&str> = v["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 2, "{v}");
    assert!(
        files[0].starts_with("events/") && files[1] == task_json,
        "{v}"
    );
    assert!(
        v["git_commands"][1]
            .as_str()
            .unwrap()
            .contains("alm: note A01-01")
    );
    ok(&repo, &["--as", "a", "note", "A01-01"]);
    assert!(task_file(&repo, "A01-01").get("handoff_note").is_none());

    // Each change is one event in the history, named as the app names it.
    let events = ok(&repo, &["history", "A01-01"])["events"].clone();
    let commands: Vec<&str> = events
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["command"].as_str().unwrap())
        .collect();
    assert_eq!(
        &commands[..5],
        ["note", "note", "describe", "describe", "describe"],
        "{events}"
    );
    assert_eq!(events[0]["by"], "a", "{events}");
    let note = events[1]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["field"] == "handoff_note")
        .unwrap_or_else(|| panic!("{events}"));
    assert_eq!(note["to"], "Tests pass; docs left.");

    // Someone else holds it: refused like the app, and no file changes.
    let before = contents(&root);
    refused(
        &repo,
        &["--as", "b", "describe", "A01-01", "--text", "Mine"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &["--as", "b", "note", "A01-01", "Mine"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &["describe", "A01-01", "--text", "Mine"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &["--as", "b", "describe", "A09-09", "--text", "x"],
        1,
        "task_not_found",
    );
    // --text or --from: exactly one.
    refused(&repo, &["--as", "a", "describe", "A01-01"], 2, "bad_usage");
    refused(
        &repo,
        &[
            "--as", "a", "describe", "A01-01", "--text", "x", "--from", "d.md",
        ],
        2,
        "bad_usage",
    );
    refused(
        &repo,
        &["--as", "a", "describe", "A01-01", "--from", "missing.md"],
        3,
        "read_failed",
    );
    assert_eq!(contents(&root), before, "a refused change writes nothing");

    // People see the same commands, naming only the changed files.
    let out = alm(&repo, &["--as", "a", "note", "A01-01", "Back soon."]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("Commit and push this change:"), "{text}");
    assert!(text.contains(task_json), "{text}");
    assert!(!text.contains("description.md"), "{text}");
}

/// Changes made in the app are listed by the next `alm` change, with commands to commit them as
/// the person's (decision 0017, row 9), and only once.
#[test]
fn app_changes_not_shared_yet_come_with_the_next_change() {
    let repo = filled();
    let root = repo.join(".allomos/taskbag");
    let app = allomos_store::Actor {
        by: Some("Ann"),
        via: "app",
    };
    allomos_store::change_task(&root, app, Duration::from_secs(5), |bag| {
        allomos_core::ops::make_ready(bag, "A01-02", None).map(|o| (o, "ready"))
    })
    .unwrap();
    // `alm next` lists them as well.
    let v = ok(&repo, &["next"]);
    assert_eq!(v["app_changes"]["count"], 1, "{v}");
    let v = ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let app_changes = &v["app_changes"];
    assert_eq!(app_changes["count"], 1, "{v}");
    assert_eq!(app_changes["by"], json!(["Ann"]));
    let files: Vec<&str> = app_changes["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert_eq!(files.len(), 2, "{v}");
    assert!(files[0].starts_with("events/"));
    assert_eq!(files[1], "tasks/A01-02/task.json");
    let commit = app_changes["git_commands"][1].as_str().unwrap();
    assert!(
        commit.contains(
            "commit -m \"app: changes made in the Allomos app\" -m \"By: Ann (Allomos app)\""
        ),
        "{commit}"
    );
    // The claim's own commands name only the claim's files.
    assert!(
        !v["changed_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "tasks/A01-02/task.json")
    );
    // Handed over: the next change no longer lists them.
    let v = ok(&repo, &["--as", "a", "label", "A01-01", "implemented"]);
    assert!(v.get("app_changes").is_none(), "{v}");
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
            .contains("2. Open AGENTS.md")
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
    assert!(v["guide"].as_str().unwrap().contains("The seven states"));
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

/// `alm add area` and `alm add iteration` take the ID as optional, like `alm add task`
/// (Task-A01-19): with only a name, the next free one; each writes one event.
#[test]
fn add_area_and_iteration_without_an_id_take_the_next_free_one() {
    let repo = filled();
    let events = || {
        fs::read_dir(repo.join(".allomos/taskbag/events"))
            .map(|d| d.count())
            .unwrap_or(0)
    };
    // filled() has A01 and A02.
    let before = events();
    let v = ok(&repo, &["add", "area", "Planning"]);
    assert_eq!(
        (&v["area"]["id"], &v["area"]["name"]),
        (&json!("A03"), &json!("Planning"))
    );
    assert_eq!(events(), before + 1);
    // An ID given is still used as given.
    let v = ok(&repo, &["add", "area", "A07", "Later"]);
    assert_eq!(v["area"]["id"], "A07");
    assert_eq!(ok(&repo, &["add", "area", "Next"])["area"]["id"], "A08");

    let v = ok(&repo, &["add", "iteration", "First", "--release", "v0.1"]);
    assert_eq!(
        (&v["iteration"]["id"], &v["iteration"]["release"]),
        (&json!("I01"), &json!("v0.1"))
    );
    let v = ok(
        &repo,
        &["add", "iteration", "Second", "--start", "2026-10-15"],
    );
    assert_eq!(v["iteration"]["id"], "I02");
    assert_eq!(events(), before + 5);

    // Refused with the same codes as before, and nothing written.
    refused(
        &repo,
        &["add", "area", "A01", "Again"],
        1,
        "duplicate_area_id",
    );
    refused(
        &repo,
        &["add", "iteration", "I01", "Again"],
        1,
        "duplicate_iteration_id",
    );
    refused(
        &repo,
        &["add", "iteration", "Third", "--end", "soon"],
        1,
        "invalid_date",
    );
    assert_eq!(events(), before + 5);
    assert!(!repo.join(".allomos/taskbag/iterations/I03.json").exists());
}

/// `alm rename area`, `alm change iteration`, `alm delete area|iteration` (Task-A01-29): each
/// writes one event; a delete is refused while any task is in it, done and dropped ones too; a bad
/// date has `alm add iteration`'s code.
#[test]
fn areas_and_iterations_are_renamed_changed_and_deleted() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    let events = || {
        fs::read_dir(bag.join("events"))
            .map(|d| d.count())
            .unwrap_or(0)
    };
    ok(&repo, &["add", "area", "A03", "Spare"]);
    ok(
        &repo,
        &["add", "iteration", "I01", "One", "--end", "2026-10-10"],
    );
    ok(&repo, &["add", "iteration", "I02", "Two"]);
    let before = events();

    let v = ok(&repo, &["rename", "area", "A01", "Tracking work"]);
    assert_eq!(
        (&v["command"], &v["area"]["id"], &v["area"]["name"]),
        (
            &json!("rename area"),
            &json!("A01"),
            &json!("Tracking work")
        )
    );
    assert_eq!(events(), before + 1);

    let v = ok(
        &repo,
        &[
            "change",
            "iteration",
            "I01",
            "--end",
            "2026-10-20",
            "--release",
            "v0.1",
        ],
    );
    assert_eq!(
        (&v["iteration"]["end"], &v["iteration"]["release"]),
        (&json!("2026-10-20"), &json!("v0.1"))
    );
    assert_eq!(events(), before + 2);
    // An empty value clears it.
    let v = ok(&repo, &["change", "iteration", "I01", "--release", ""]);
    assert!(v["iteration"].get("release").is_none());
    assert_eq!(events(), before + 3);

    // Refusals: nothing written.
    refused(
        &repo,
        &["change", "iteration", "I01", "--start", "soon"],
        1,
        "invalid_date",
    );
    refused(
        &repo,
        &["change", "iteration", "I09", "--name", "x"],
        1,
        "unknown_iteration",
    );
    refused(&repo, &["rename", "area", "A09", "x"], 1, "unknown_area");
    refused(
        &repo,
        &["rename", "area", "A01", " "],
        1,
        "area_name_missing",
    );
    // A02 holds only a dropped task; I01 only a done one: both still count.
    ok(&repo, &["drop", "A02-01"]);
    ok(&repo, &["plan", "A01-01", "I01"]);
    ok(&repo, &["--as", "w", "claim", "A01-01"]);
    ok(&repo, &["--as", "w", "done", "A01-01"]);
    let before = events();
    let v = refused(&repo, &["delete", "area", "A02"], 1, "area_not_empty");
    assert_eq!(v["error"]["params"]["count"], "1");
    refused(
        &repo,
        &["delete", "iteration", "I01"],
        1,
        "iteration_not_empty",
    );
    refused(&repo, &["delete", "area", "A09"], 1, "unknown_area");
    assert_eq!(events(), before);
    assert!(bag.join("areas/A02.json").exists() && bag.join("iterations/I01.json").exists());

    let v = ok(&repo, &["delete", "iteration", "I02"]);
    assert_eq!(v["deleted"]["iteration"]["id"], "I02");
    let v = ok(&repo, &["delete", "area", "A03"]);
    assert_eq!(v["deleted"]["area"]["id"], "A03");
    assert_eq!(events(), before + 2);
    assert!(!bag.join("areas/A03.json").exists() && !bag.join("iterations/I02.json").exists());

    // `alm list` agrees.
    let list = ok(&repo, &["list", "--by", "iteration"]);
    let ids: Vec<&str> = list["iterations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["I01"]);
    let text = String::from_utf8(alm(&repo, &["list"]).stdout).unwrap();
    assert!(
        text.contains("Tracking work") && !text.contains("Spare"),
        "{text}"
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

/// `alm guide run-modes` prints the explanation the person reads before an automatic or loop run,
/// in the task language (Task-A03-21): `--language` wins, then the taskbag's task language; a
/// language it is not written in gets the English text with a line asking for a translation.
#[test]
fn run_modes_follow_the_task_language() {
    let repo = filled();
    let settings = tmp("settings");
    for (language, words) in [
        ("en", "## Quiet mode"),
        ("ja", "## 静かモード"),
        ("zh-CN", "## 安静模式"),
    ] {
        let (code, v) = fj_settings(
            &repo,
            &settings,
            &["guide", "run-modes", "--language", language],
        );
        assert_eq!(code, 0, "{v}");
        assert_eq!(v["written_in_language"], true, "{v}");
        let text = v["guide"].as_str().unwrap();
        assert!(text.contains(words), "{language}: {text}");
        for command in [
            "`alm human`",
            "`alm ready <id>`",
            "`alm reviewed <id>`",
            "`alm checked <id>`",
        ] {
            assert!(text.contains(command), "{language}: {command}");
        }
    }
    // Without --language: the taskbag's task language.
    fj_settings(&repo, &settings, &["task-language", "ja"]);
    let (_, v) = fj_settings(&repo, &settings, &["guide", "run-modes"]);
    assert_eq!(v["language"], "ja");
    assert!(v["guide"].as_str().unwrap().contains("静かモード"), "{v}");
    // Not written in French: English, with a line asking for a faithful translation.
    fj_settings(&repo, &settings, &["task-language", "fr"]);
    let (code, v) = fj_settings(&repo, &settings, &["guide", "run-modes"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["written_in_language"], false, "{v}");
    let text = v["guide"].as_str().unwrap();
    assert!(
        text.starts_with("This text is not written in fr yet"),
        "{text}"
    );
    assert!(text.contains("## Quiet mode"), "{text}");
}

/// `alm guide agent-files` (Task-A03-24): the one-time procedure for shared rules in AGENTS.md,
/// in the task language like `run-modes`; `alm guide` (all topics) includes the English one.
#[test]
fn agent_files_follow_the_task_language() {
    let repo = filled();
    let settings = tmp("settings");
    for (language, words) in [
        ("en", "# Agent files"),
        ("ja", "# エージェントのファイル"),
        ("zh-CN", "# 智能体文件"),
    ] {
        let (code, v) = fj_settings(
            &repo,
            &settings,
            &["guide", "agent-files", "--language", language],
        );
        assert_eq!(code, 0, "{v}");
        assert_eq!(v["topic"], "agent-files", "{v}");
        assert_eq!(v["written_in_language"], true, "{v}");
        let text = v["guide"].as_str().unwrap();
        assert!(text.starts_with(words), "{language}: {text}");
        assert!(text.contains("`## Agents set up`"), "{language}");
    }
    fj_settings(&repo, &settings, &["task-language", "zh-CN"]);
    let (_, v) = fj_settings(&repo, &settings, &["guide", "agent-files"]);
    assert_eq!(v["language"], "zh-CN");
    assert!(
        v["guide"].as_str().unwrap().starts_with("# 智能体文件"),
        "{v}"
    );
    fj_settings(&repo, &settings, &["task-language", "fr"]);
    let (_, v) = fj_settings(&repo, &settings, &["guide", "agent-files"]);
    assert_eq!(v["written_in_language"], false, "{v}");
    let text = v["guide"].as_str().unwrap();
    assert!(
        text.starts_with("This text is not written in fr yet"),
        "{text}"
    );
    assert!(text.contains("# Agent files"), "{text}");
    let all = ok(&repo, &["guide"])["guide"].as_str().unwrap().to_string();
    assert!(all.contains("# Agent files") && all.contains("# Run modes"));
}

/// `alm guide goal` (Task-A03-25-01): the goal prompt, in the task language like `run-modes`;
/// in another language, the English text with a first line asking the agent to follow a faithful
/// translation; `alm guide` (all topics) includes the English one.
#[test]
fn goal_follows_the_task_language() {
    let repo = filled();
    let settings = tmp("settings");
    for (language, words) in [
        ("en", "# Goal run"),
        ("ja", "# ゴール実行"),
        ("zh-CN", "# 目标运行"),
    ] {
        let (code, v) = fj_settings(&repo, &settings, &["guide", "goal", "--language", language]);
        assert_eq!(code, 0, "{v}");
        assert_eq!(v["topic"], "goal", "{v}");
        assert_eq!(v["written_in_language"], true, "{v}");
        let text = v["guide"].as_str().unwrap();
        assert!(text.starts_with(words), "{language}: {text}");
        assert!(text.contains("(goal prompt v1)"), "{language}");
        assert!(text.contains("## 4. "), "{language}");
        assert!(text.contains("`/goal`"), "{language}");
    }
    fj_settings(&repo, &settings, &["task-language", "ja"]);
    let (_, v) = fj_settings(&repo, &settings, &["guide", "goal"]);
    assert_eq!(v["language"], "ja");
    assert!(
        v["guide"].as_str().unwrap().starts_with("# ゴール実行"),
        "{v}"
    );
    fj_settings(&repo, &settings, &["task-language", "fr"]);
    let (_, v) = fj_settings(&repo, &settings, &["guide", "goal"]);
    assert_eq!(v["written_in_language"], false, "{v}");
    let text = v["guide"].as_str().unwrap();
    assert!(
        text.starts_with(
            "This text is not written in fr yet: follow a faithful translation of it into fr"
        ),
        "{text}"
    );
    assert!(text.contains("# Goal run"), "{text}");
    let all = ok(&repo, &["guide"])["guide"].as_str().unwrap().to_string();
    assert!(all.contains("# Goal run") && all.contains("# Agent files"));
    // The brief names the topic and the goal-run exception.
    let brief = ok(&repo, &["brief"])["brief"].as_str().unwrap().to_string();
    assert!(brief.contains("`alm guide goal`"), "{brief}");
    assert!(brief.contains("Goal run:"), "{brief}");
}

#[test]
fn title_and_project_rename() {
    let repo = filled();
    let v = ok(&repo, &["title", "A01-01", "A clearer outcome"]);
    assert_eq!(v["task"]["title"], "A clearer outcome");
    refused(&repo, &["title", "A01-01", " "], 1, "task_title_missing");
    refused(&repo, &["title", "Z99-99", "x"], 1, "task_not_found");
    let v = ok(&repo, &["project"]);
    assert_eq!(v["project"]["name"], "Demo");
    let v = ok(&repo, &["project", "--name", "Allomos", "--id", "allomos"]);
    assert_eq!(
        (&v["project"]["name"], &v["project"]["id"]),
        (&json!("Allomos"), &json!("allomos"))
    );
    refused(&repo, &["project", "--id", "bad id"], 1, "invalid_id");
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(project["name"], "Allomos");
}

/// Set `project.json`'s format, as a newer (or older) Allomos would have written it.
fn set_format(repo: &Path, format: &str) {
    let path = repo.join(".allomos/taskbag/project.json");
    let mut v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    v["format"] = json!(format);
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&v).unwrap()),
    )
    .unwrap();
}

#[test]
fn a_newer_taskbag_stops_alm_with_the_persons_choice() {
    let repo = filled();
    set_format(&repo, "allomos/9");
    // A field this Allomos does not know, as a newer one might add.
    let path = repo.join(".allomos/taskbag/project.json");
    let mut v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    v["handoff"] = json!({ "context": 90 });
    fs::write(&path, v.to_string()).unwrap();
    let before = fs::read_to_string(&path).unwrap();

    for args in [
        &["next"][..],
        &["list"],
        &["claim", "A01-01", "--as", "a"],
        &["move-in"],
    ] {
        let v = refused(&repo, args, 4, "taskbag_newer");
        let message = v["error"]["message"].as_str().unwrap();
        assert!(message.contains("update Allomos now"), "{message}");
        assert!(message.contains("your own understanding"), "{message}");
        assert_eq!(v["error"]["params"]["found"], "allomos/9");
    }
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        before,
        "nothing written"
    );
    // The brief and the guide still work: they do not read the taskbag's tasks.
    ok(&repo, &["brief"]);
    ok(&repo, &["guide", "terms"]);

    // People see the same message, and the exit code.
    let out = alm(&repo, &["next"]);
    assert_eq!(out.status.code(), Some(4));
    assert!(String::from_utf8_lossy(&out.stderr).contains("error [taskbag_newer]"));
}

#[test]
fn an_out_of_date_section_warns_the_agent_on_every_call() {
    let repo = filled();
    // Not connected yet (no section anywhere): no warning.
    let v = ok(&repo, &["next"]);
    assert_eq!(v["warnings"], json!([]));

    // A section from before versions: every command works, and warns.
    fs::write(
        repo.join("AGENTS.md"),
        "# Rules\n\n## Tasks: Allomos\n\nStart with `alm next`.\n",
    )
    .unwrap();
    fs::write(repo.join("CLAUDE.md"), "@AGENTS.md\n").unwrap();
    for args in [&["next"][..], &["list"], &["show", "A01-01"], &["human"]] {
        let v = ok(&repo, args);
        assert_eq!(
            v["warnings"][0]["code"], "instructions_outdated",
            "{args:?}"
        );
        assert_eq!(v["warnings"][0]["params"]["files"], "AGENTS.md");
        assert!(
            v["warnings"][0]["message"]
                .as_str()
                .unwrap()
                .contains("`alm brief`")
        );
    }
    let out = alm(&repo, &["next"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stderr).contains("warning [instructions_outdated]"));

    // The agent renews it from `alm brief`: no more warning.
    let brief = ok(&repo, &["brief"])["brief"].as_str().unwrap().to_string();
    assert!(brief.starts_with("## Tasks: Allomos (v13)"), "{brief}");
    fs::write(repo.join("AGENTS.md"), format!("# Rules\n\n{brief}")).unwrap();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["warnings"], json!([]));

    // A v12 section (before the goal run, Task-A03-25-01) is out of date too.
    fs::write(
        repo.join("AGENTS.md"),
        format!(
            "# Rules\n\n{}",
            brief.replace("## Tasks: Allomos (v13)", "## Tasks: Allomos (v12)")
        ),
    )
    .unwrap();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["warnings"][0]["code"], "instructions_outdated");
}

/// The project notes are a standard part of Allomos (Task-A03-20): the brief names them and when
/// to update them, `alm next` shows their Running and Pitfalls and the day they changed, and
/// `alm handoff` reminds to update them.
#[test]
fn project_notes_are_in_the_brief_next_and_handoff() {
    let repo = filled();
    let brief = ok(&repo, &["brief"])["brief"].as_str().unwrap().to_string();
    // Compared as one line: the brief wraps at 100 columns.
    let brief = brief.split_whitespace().collect::<Vec<_>>().join(" ");
    for words in [
        "`project-notes.md`",
        "Read it at the start",
        "you edit it directly",
        "start or stop a server or process",
        "hosted resource or a Git remote",
        "learn a pitfall",
        "a credential moves",
        "Change lines in place",
        "commit and push that file with Git yourself",
        "every new piece of work (a new card, a new outcome) gets its own task",
        "`alm guide tasks`",
    ] {
        assert!(brief.contains(words), "{words}: {brief}");
    }
    assert!(
        brief.contains("never edit those files by hand, except the project notes"),
        "{brief}"
    );

    // `alm next`: the default notes, with the day they last changed.
    let notes_file = repo.join(".allomos/taskbag/project-notes.md");
    fs::write(
        &notes_file,
        "# Project notes\n\n## Running\n\n- dev server on :5173 (`npm run dev`)\n\n## Pitfalls\n\n- zsh expands `=word`\n- run checks in the worktree\n\n## Where credentials live\n\nNever here.\n",
    )
    .unwrap();
    fs::File::options()
        .write(true)
        .open(&notes_file)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(20_740 * 86_400 + 3_600))
        .unwrap();
    let v = ok(&repo, &["next"]);
    let notes = &v["project_notes"];
    assert_eq!(notes["file"], "project-notes.md");
    assert_eq!(notes["found"], true);
    assert_eq!(notes["changed"], "2026-10-14");
    assert_eq!(notes["running"], "- dev server on :5173 (`npm run dev`)");
    assert_eq!(
        notes["pitfalls"],
        "- zsh expands `=word`\n- run checks in the worktree"
    );
    assert!(
        notes["path"]
            .as_str()
            .unwrap()
            .ends_with("project-notes.md")
    );
    let out = alm(&repo, &["next"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("project-notes.md, last changed 2026-10-14):"),
        "{text}"
    );
    assert!(text.contains("Running:\n  - dev server on :5173"), "{text}");
    assert!(
        text.contains("Pitfalls:\n  - zsh expands `=word`\n  - run checks in the worktree\n"),
        "{text}"
    );
    assert!(
        !text.contains("Never here."),
        "only Running and Pitfalls: {text}"
    );

    // `alm handoff` reminds, with the day.
    let v = ok(&repo, &["handoff", "--as", "s/one", "--tasks", "A01-01"]);
    assert_eq!(v["project_notes"]["changed"], "2026-10-14");
    let out = alm(&repo, &["handoff", "--as", "s/one"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains(
            "If what is running or a pitfall changed, update `project-notes.md` too (last changed 2026-10-14)"
        ),
        "{text}"
    );

    // No notes file: said plainly, not an error.
    fs::remove_file(&notes_file).unwrap();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["project_notes"]["found"], false);
    assert_eq!(v["project_notes"]["running"], Value::Null);
    let out = alm(&repo, &["next"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("Project notes: this taskbag has no project-notes.md yet"),
        "{text}"
    );
    let v = ok(&repo, &["handoff", "--as", "s/one"]);
    assert_eq!(v["project_notes"]["found"], false);
    let out = alm(&repo, &["handoff", "--as", "s/one"]);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("update `project-notes.md` too (this taskbag has none yet)")
    );
}

/// Every file under `dir`, with its contents, to show that nothing changed.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out: Vec<(PathBuf, Vec<u8>)> = walk(dir)
        .into_iter()
        .filter(|p| p.is_file())
        .map(|p| {
            let bytes = fs::read(&p).unwrap();
            (p, bytes)
        })
        .collect();
    out.sort();
    out
}

#[test]
fn move_in_on_a_current_taskbag_changes_nothing() {
    let repo = filled();
    let before = snapshot(&repo);
    let v = ok(&repo, &["move-in"]);
    assert_eq!(v["move_in"], json!({ "needed": false }));
    assert!(v.get("request").is_none());
    assert_eq!(snapshot(&repo), before);
}

/// Owner, 2026-10-15 (Task-A01-45): a taskbag from any older Allomos is moved in by an agent,
/// like another task system. `alm` reads and changes nothing in it, says so on every command, and
/// `alm move-in` prints the request.
#[test]
fn every_older_taskbag_is_to_be_moved_in() {
    for n in 1..=3 {
        let repo = filled();
        fs::write(
            repo.join("AGENTS.md"),
            "## Tasks: Allomos (v2)\n\nalm next\n",
        )
        .unwrap();
        set_format(&repo, &format!("allomos/{n}"));
        let before = snapshot(&repo);
        for args in [
            &["list"][..],
            &["next"],
            &["show", "A01-01"],
            &["claim", "A01-01", "--as", "a"],
            &["add", "task", "--area", "A01", "--title", "x"],
            &["labels", "a,b"],
            &["rename", "label", "implemented", "built"],
            &["handoff", "--as", "a"],
            &["handoff-rules"],
        ] {
            let v = refused(&repo, args, 1, "taskbag_to_move_in");
            assert_eq!(v["error"]["params"]["found"], format!("allomos/{n}"));
            assert_eq!(v["error"]["params"]["current"], "allomos/4");
            let message = v["error"]["message"].as_str().unwrap();
            assert!(message.contains("Move the tasks in"), "{message}");
            assert!(message.contains("`alm move-in`"), "{message}");
        }
        // The request, naming the folder it will be renamed to; still nothing changed.
        let v = ok(&repo, &["move-in", "--language", "en"]);
        let old = format!("taskbag-allomos-{n}");
        assert_eq!(v["move_in"]["needed"], true);
        assert_eq!(v["move_in"]["prepared"], false);
        assert_eq!(v["move_in"]["from"], format!("allomos/{n}"));
        let named = Path::new(v["move_in"]["old"].as_str().unwrap()).to_path_buf();
        assert!(named.ends_with(format!(".allomos/{old}")), "{named:?}");
        let request = v["request"].as_str().unwrap();
        assert!(
            request.starts_with("Please move the tasks of a taskbag written by an older Allomos")
        );
        assert!(request.contains(&named.display().to_string()), "{request}");
        assert!(request.contains(" init --id "), "{request}");
        assert!(request.contains("alm confirm --title"), "{request}");
        for language in ["zh-CN", "ja"] {
            let v = ok(&repo, &["move-in", "--language", language]);
            assert_eq!(v["language"], language);
        }
        assert_eq!(snapshot(&repo), before, "allomos/{n}: nothing written");
        // People see the code too.
        let out = alm(&repo, &["list"]);
        assert_eq!(out.status.code(), Some(1));
        assert!(String::from_utf8_lossy(&out.stderr).contains("error [taskbag_to_move_in]"));
    }
}

/// After the app's Move the tasks in…: the new taskbag works, and `alm move-in` prints the
/// request for the old folder beside it until the first task is moved in.
#[test]
fn move_in_names_the_old_folder_beside_a_new_taskbag() {
    let repo = filled();
    set_format(&repo, "allomos/3");
    let root = repo.join(".allomos/taskbag");
    let old = repo.join(".allomos/taskbag-allomos-3");
    fs::rename(&root, &old).unwrap();
    ok(&repo, &["init", "--name", "Demo"]);
    let v = ok(&repo, &["move-in", "--language", "en"]);
    assert_eq!(v["move_in"]["prepared"], true);
    let named = Path::new(v["move_in"]["old"].as_str().unwrap()).to_path_buf();
    assert!(named.ends_with(".allomos/taskbag-allomos-3"), "{named:?}");
    let request = v["request"].as_str().unwrap();
    assert!(request.contains("Allomos renamed the old taskbag (format allomos/3;"));
    assert!(!request.contains(" init --id "), "{request}");
    // The agent moves the tasks with `alm`; the new taskbag takes them.
    ok(&repo, &["add", "area", "A01", "Tracking"]);
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
        ],
    );
    let v = ok(&repo, &["move-in"]);
    assert_eq!(v["move_in"], json!({ "needed": false }));
}

#[test]
fn rename_label_renames_it_on_the_ladder_and_the_tasks() {
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(&repo, &["--as", "a", "label", "A01-01", "human verified"]);
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "rename",
            "label",
            "human verified",
            "checked by a person",
        ],
    );
    assert_eq!(
        v["labels"],
        json!([
            "implemented",
            "locally verified",
            "deployed and verified",
            "checked by a person"
        ])
    );
    assert_eq!(v["tasks"][0]["id"], "A01-01");
    assert_eq!(v["tasks"][0]["label"], "checked by a person");
    assert_eq!(v["issues"], json!([]));
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["task"]["label"], "checked by a person");
    assert_eq!(
        ok(&repo, &["history", "A01-01"])["events"][0]["command"],
        "rename label"
    );
    refused(
        &repo,
        &["rename", "label", "shipped", "x"],
        1,
        "label_not_on_ladder",
    );
    refused(
        &repo,
        &["rename", "label", "checked by a person", "implemented"],
        1,
        "label_exists",
    );
    refused(
        &repo,
        &["rename", "label", "implemented", " "],
        1,
        "label_name_missing",
    );
    let out = alm(&repo, &["rename", "label", "implemented", "built"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("No task carried the label."));
}

/// Task-A03-18: `alm done --without-human-check` with its steps, `alm human` (To check, To
/// answer), `alm checked`, and the old `--without-owner` flag kept as an alias.
#[test]
fn a_task_finished_without_the_human_check_is_listed_checked_and_cleared() {
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(&repo, &["--as", "a", "label", "A01-01", "locally verified"]);
    // Empty text is refused, and nothing changes; so are steps without a note.
    refused(
        &repo,
        &["--as", "a", "done", "A01-01", "--without-human-check", " "],
        1,
        "without_human_check_missing",
    );
    let (code, _) = fj(&repo, &["--as", "a", "done", "A01-01", "--steps", "x.md"]);
    assert_eq!(code, 2, "--steps needs --without-human-check");
    assert_eq!(task_file(&repo, "A01-01")["state"], "in_progress");

    // Without steps: finished, and alm asks for them, saying how to write them.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "card run, approver away",
        ],
    );
    assert_eq!(v["warnings"][0]["code"], "human_check_steps_missing");
    assert!(
        v["warnings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("--steps <file.md>")
    );
    assert_eq!(v["task"]["without_human_check"], "card run, approver away");
    let file = task_file(&repo, "A01-01");
    assert_eq!(file["without_human_check"], "card run, approver away");
    assert_eq!(file["label"], "locally verified", "the label stays");

    // The steps, from a file, on the task already done; an empty file is refused.
    fs::write(repo.join("empty.md"), " \n").unwrap();
    refused(
        &repo,
        &[
            "--as",
            "b",
            "done",
            "A01-01",
            "--without-human-check",
            "card run",
            "--steps",
            "empty.md",
        ],
        1,
        "human_check_steps_empty",
    );
    let steps = "1. Open `~/allomos-tests/demo` in Allomos.\n2. Click **A01-01**.\n3. You should see the badge.\n";
    fs::write(repo.join("steps.md"), steps).unwrap();
    // Already done: the holder rule does not apply, only the human check is recorded.
    let v = ok(
        &repo,
        &[
            "--as",
            "b",
            "done",
            "A01-01",
            "--without-human-check",
            "card run, approver away",
            "--steps",
            "steps.md",
        ],
    );
    assert_eq!(v["warnings"], json!([]));
    assert_eq!(task_file(&repo, "A01-01")["human_check_steps"], steps);
    let out = alm(&repo, &["show", "A01-01"]);
    let shown = String::from_utf8_lossy(&out.stdout);
    assert!(
        shown.contains("not human-checked: card run, approver away"),
        "{shown}"
    );
    assert!(shown.contains("human check steps:\n    1. Open"), "{shown}");

    // The old flag still works (alias).
    ok(&repo, &["--as", "c", "claim", "A02-01"]);
    ok(
        &repo,
        &["--as", "c", "done", "A02-01", "--without-owner", "loop run"],
    );
    assert_eq!(
        task_file(&repo, "A02-01")["without_human_check"],
        "loop run"
    );

    // A task finished the usual way is not listed; a waiting one is To answer; backlog is not.
    ok(&repo, &["--as", "b", "claim", "A01-02"]);
    ok(&repo, &["--as", "b", "done", "A01-02"]);
    ok(
        &repo,
        &["add", "task", "A01-03", "--area", "A01", "--title", "Wait"],
    );
    ok(
        &repo,
        &["add", "task", "A01-04", "--area", "A01", "--title", "Idea"],
    );
    ok(
        &repo,
        &["wait", "A01-03", "--on", "approve the card", "--who", "Ann"],
    );
    for flag in ["--without-human-check", "--without-owner"] {
        let v = ok(&repo, &["list", flag]);
        let found: Vec<&Value> = v["areas"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|a| a["tasks"].as_array().unwrap())
            .map(|t| &t["id"])
            .collect();
        assert_eq!(found, [&json!("A01-01"), &json!("A02-01")], "{flag}");
    }
    let v = ok(&repo, &["human"]);
    let ids = |list: &Value| -> Vec<String> {
        list.as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(ids(&v["to_check"]), ["A01-01", "A02-01"]);
    assert_eq!(ids(&v["to_answer"]), ["A01-03"]);
    assert_eq!(
        v["to_check"][0]["human_check_steps"], steps,
        "--json has the steps"
    );
    assert_eq!(
        ids(&ok(&repo, &["human", "--area", "A02"])["to_check"]),
        ["A02-01"]
    );
    refused(&repo, &["human", "--area", "A09"], 1, "unknown_area");
    refused(
        &repo,
        &["human", "--iteration", "I09"],
        1,
        "unknown_iteration",
    );
    let out = alm(&repo, &["human"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.starts_with("To check (2)\n"), "{text}");
    assert!(
        text.contains("not human-checked: card run, approver away"),
        "{text}"
    );
    assert!(text.contains("To answer (1)"), "{text}");
    assert!(text.contains("waiting for Ann: approve the card"), "{text}");
    assert!(!text.contains("A01-04"), "backlog is not listed: {text}");
    assert!(!text.contains("1. Open"), "steps only with --steps: {text}");
    let out = alm(&repo, &["human", "--steps"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("steps:\n          1. Open"), "{text}");
    assert!(text.contains("steps: none written"), "{text}");

    // A person checked it: top label, off To check; anyone may say so; history keeps the note.
    refused(&repo, &["checked", "A01-03"], 1, "not_done");
    let v = ok(&repo, &["--as", "will", "checked", "A01-01"]);
    assert_eq!(v["task"]["label"], "human verified");
    let file = task_file(&repo, "A01-01");
    assert!(file.get("without_human_check").is_none());
    assert!(file.get("human_check_steps").is_none());
    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "checked");
    let note = last["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["field"] == "without_human_check")
        .unwrap();
    assert_eq!(note["from"], "card run, approver away");
    assert_eq!(ids(&ok(&repo, &["human"])["to_check"]), ["A02-01"]);
    // The top label set on a finished task is the human check too.
    ok(&repo, &["--as", "c", "label", "A02-01", "human verified"]);
    assert!(
        task_file(&repo, "A02-01")
            .get("without_human_check")
            .is_none()
    );
    let out = alm(&repo, &["human"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Nothing to check."));
}

#[test]
fn an_agents_own_decisions_wait_for_review_and_leave_when_reviewed() {
    // Task-A03-19: `alm ai-decided` records a decision in the history only; `alm human` lists it
    // under To review; `alm reviewed` takes it off, and the history keeps both.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &["--as", "a", "ai-decided", "A01-01", " "],
        1,
        "ai_decision_missing",
    );
    refused(
        &repo,
        &["--as", "a", "ai-decided", "A09-09", "x"],
        1,
        "task_not_found",
    );
    refused(&repo, &["reviewed", "A01-01"], 1, "nothing_to_review");
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "ai-decided",
            "A01-01",
            "Names: alm describe, alm note — match alm title",
        ],
    );
    assert_eq!(
        v["decision"]["text"],
        "Names: alm describe, alm note — match alm title"
    );
    assert_eq!(v["decision"]["by"], "a");
    assert_eq!(v["changed_files"].as_array().unwrap().len(), 1, "{v}");
    assert!(
        v["changed_files"][0]
            .as_str()
            .unwrap()
            .starts_with("events/")
    );
    // Anyone may record one, on any task: recording changes nothing in the task.
    ok(
        &repo,
        &[
            "--as",
            "b",
            "ai-decided",
            "A01-01",
            "Layout: one file per command",
        ],
    );
    ok(
        &repo,
        &["--as", "b", "ai-decided", "A01-02", "Default: newest first"],
    );
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");

    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "ai-decided");
    assert_eq!(last["changes"][0]["field"], "ai_decision");
    assert_eq!(last["changes"][0]["to"], "Layout: one file per command");

    let v = ok(&repo, &["human"]);
    let review = v["to_review"].as_array().unwrap();
    let ids: Vec<&str> = review.iter().map(|t| t["id"].as_str().unwrap()).collect();
    assert_eq!(
        ids,
        ["A01-01", "A01-02"],
        "backlog tasks are listed too: {v}"
    );
    let texts: Vec<&str> = review[0]["ai_decisions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        texts,
        [
            "Names: alm describe, alm note — match alm title",
            "Layout: one file per command"
        ]
    );
    assert_eq!(
        ok(&repo, &["human", "--area", "A02"])["to_review"],
        json!([])
    );
    let out = alm(&repo, &["human"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("To review: AI decisions (2)"), "{text}");
    assert!(
        text.contains("- Layout: one file per command (b, "),
        "{text}"
    );
    assert!(text.contains("`alm reviewed <id>`"), "{text}");
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["ai_decisions"].as_array().unwrap().len(), 2);

    // Marking a done task checked does not review its decisions: they are separate questions.
    ok(&repo, &["--as", "a", "label", "A01-01", "human verified"]);
    ok(&repo, &["--as", "a", "done", "A01-01"]);
    assert_eq!(
        ok(&repo, &["human"])["to_review"].as_array().unwrap().len(),
        2
    );

    // The person reviewed them: off To review; the history keeps each decision.
    let v = ok(&repo, &["--as", "will", "reviewed", "A01-01"]);
    assert_eq!(v["reviewed"].as_array().unwrap().len(), 2);
    let v = ok(&repo, &["human"]);
    let ids: Vec<&str> = v["to_review"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A01-02"]);
    let last = &ok(&repo, &["history", "A01-01"])["events"][0];
    assert_eq!(last["command"], "reviewed");
    assert_eq!(last["by"], "will");
    assert_eq!(last["changes"].as_array().unwrap().len(), 2);
    assert!(last["changes"][0]["to"].is_null());
    assert_eq!(last["changes"][1]["from"], "Layout: one file per command");
    refused(&repo, &["reviewed", "A01-01"], 1, "nothing_to_review");
    assert_eq!(ok(&repo, &["show", "A01-01"])["ai_decisions"], json!([]));
}

#[test]
fn handoff_rules_have_defaults_overrides_and_refusals() {
    let repo = filled();
    let v = ok(&repo, &["handoff-rules"]);
    assert_eq!(v["applied"]["quota_gate"], 3);
    assert_eq!(v["applied"]["context_gate"], 90);

    ok(&repo, &["handoff-rules", "--quota", "5", "--context", "80"]);
    ok(
        &repo,
        &["handoff-rules", "--agent", "codex", "--context", "70"],
    );
    let v = ok(&repo, &["handoff-rules", "--as", "codex/a01-01"]);
    assert_eq!(v["applied"]["quota_gate"], 5);
    assert_eq!(v["applied"]["context_gate"], 70);
    assert_eq!(v["applied"]["override_for"], "codex");
    let v = ok(&repo, &["handoff-rules", "--as", "claude/x"]);
    assert_eq!(v["applied"]["context_gate"], 80);

    // `alm next` tells the agent its own gates.
    let v = ok(&repo, &["next", "--as", "codex/a01-01"]);
    assert_eq!(v["handoff_rules"]["context_gate"], 70);
    let out = alm(&repo, &["next", "--as", "codex/a01-01"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("at 70% context used"));

    refused(
        &repo,
        &["handoff-rules", "--quota", "150"],
        1,
        "invalid_percent",
    );
    refused(
        &repo,
        &["handoff-rules", "--context", "lots"],
        1,
        "invalid_percent",
    );
    ok(&repo, &["handoff-rules", "--clear"]);
    let v = ok(&repo, &["handoff-rules", "--as", "codex/a01-01"]);
    assert_eq!(v["applied"]["quota_gate"], 3);
    assert_eq!(v["applied"]["context_gate"], 70, "the override stays");
    ok(&repo, &["handoff-rules", "--agent", "codex", "--clear"]);
    let project = fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap();
    assert!(!project.contains("handoff"), "{project}");
}

/// Whoever takes a task over is told, where its handoff is shown, to check the real state first
/// and to repeat nothing already done; the template asks for what was done (Task-A03-23).
#[test]
fn show_and_next_give_the_taking_over_rules_with_a_handoff() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    let text = |args: &[&str]| String::from_utf8(alm(&repo, args).stdout).unwrap();
    // No handoff yet: no rules.
    assert!(ok(&repo, &["show", "A01-01"])["taking_over"].is_null());
    assert!(ok(&repo, &["next"])["taking_over"].is_null());
    assert!(!text(&["show", "A01-01"]).contains("Taking over"));
    assert!(!text(&["next"]).contains("Taking over"));

    let v = ok(
        &repo,
        &["handoff", "--as", "claude/run", "--tasks", "A01-01"],
    );
    let written = fs::read_to_string(v["path"].as_str().unwrap()).unwrap();
    assert!(
        written.contains("- **Already done, do not repeat:**"),
        "{written}"
    );

    for args in [&["show", "A01-01"][..], &["next"][..]] {
        let v = ok(&repo, args);
        let rules = v["taking_over"].as_str().unwrap();
        assert!(rules.contains("Check the real state"), "{rules}");
        let out = text(args);
        let flat = out.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            out.matches("Taking over from a handoff:").count(),
            1,
            "{out}"
        );
        assert!(flat.contains("git worktree list"), "{out}");
        assert!(flat.contains("is not yours to change or remove"), "{out}");
        assert!(flat.contains("if unsure, ask the person"), "{out}");
        assert!(flat.contains("already done is not done again"), "{out}");
        assert!(flat.contains("it does not authorize repeating it"), "{out}");
    }
}

#[test]
fn a_session_writes_one_handoff_that_the_next_one_finds() {
    let repo = filled();
    let ids: Vec<String> = ok(&repo, &["list"])["areas"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|a| a["tasks"].as_array().unwrap().clone())
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    let (one, two) = (&ids[0], &ids[1]);
    refused(&repo, &["handoff"], 1, "holder_missing");

    let tasks = format!("{one},{two}");
    let v = ok(&repo, &["handoff", "--as", "claude/run", "--tasks", &tasks]);
    assert_eq!(v["created"], true);
    let path = PathBuf::from(v["path"].as_str().unwrap());
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("## For anyone, in plain words"), "{text}");

    // The agent fills it in; a second call is the same file.
    let draft = repo.join("draft.md");
    fs::write(&draft, "# Handoff\n\n- **What was being done:** baking\n").unwrap();
    let v = ok(
        &repo,
        &["handoff", "--as", "claude/run", "--from", "draft.md"],
    );
    assert_eq!(v["created"], false);
    assert_eq!(v["path"].as_str().unwrap(), path.to_str().unwrap());
    assert_eq!(v["handoff"]["tasks"], json!([one, two]));

    let v = ok(&repo, &["show", two]);
    assert_eq!(v["handoff"]["writer"], "claude/run");
    assert_eq!(v["handoff"]["summary"], "What was being done: baking");
    let v = ok(&repo, &["handoffs", "--task", one]);
    assert_eq!(v["handoffs"].as_array().unwrap().len(), 1);
    refused(
        &repo,
        &["handoff", "--as", "x", "--tasks", "Z9-99"],
        1,
        "task_not_found",
    );
}

#[test]
fn the_template_can_be_shown_and_replaced() {
    let repo = filled();
    let v = ok(&repo, &["handoff-template"]);
    assert!(v["template"].as_str().unwrap().starts_with("# Handoff"));
    fs::write(repo.join("t.md"), "# Ours\n").unwrap();
    let v = ok(&repo, &["handoff-template", "--set", "t.md"]);
    assert_eq!(v["template"], "# Ours\n");
    fs::write(repo.join("empty.md"), "\n").unwrap();
    refused(
        &repo,
        &["handoff-template", "--set", "empty.md"],
        1,
        "template_empty",
    );
}

/// The event files in a taskbag (Task-A01-17).
fn event_count(repo: &Path) -> usize {
    fs::read_dir(repo.join(".allomos/taskbag/events"))
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .count()
        })
        .unwrap_or(0)
}

#[test]
fn every_change_writes_exactly_one_event_and_a_refusal_none() {
    let repo = filled();
    // init, two areas and three tasks so far.
    assert_eq!(event_count(&repo), 6);
    let changes: &[&[&str]] = &[
        &["add", "iteration", "I01", "First"],
        &["ready", "A01-01"],
        &["--as", "a", "claim", "A01-01"],
        &["--as", "a", "label", "A01-01", "implemented"],
        &["--as", "a", "needs", "A01-01", "a check"],
        &["plan", "A01-01", "I01"],
        &["--as", "a", "title", "A01-01", "Open a folder"],
        &["--as", "a", "wait", "A01-01", "--on", "the owner"],
        &["--as", "a", "release", "A01-01", "--note", "over to you"],
        &["--as", "b", "claim", "A01-01"],
        &["--as", "b", "done", "A01-01"],
        &["drop", "A01-02"],
        &["labels", "implemented,locally verified,owner verified"],
        &["project", "--name", "Demo 2"],
        &["task-language", "ja"],
        &["handoff-rules", "--quota", "5"],
    ];
    for (i, args) in changes.iter().enumerate() {
        ok(&repo, args);
        assert_eq!(event_count(&repo), 7 + i, "after {args:?}");
    }
    let n = event_count(&repo);
    // Refused changes write nothing.
    refused(&repo, &["--as", "c", "claim", "A01-01"], 1, "task_closed");
    refused(
        &repo,
        &["--as", "b", "label", "A02-01", "nope"],
        1,
        "label_not_on_ladder",
    );
    refused(
        &repo,
        &["add", "task", "A01-01", "--area", "A01", "--title", "T"],
        1,
        "duplicate_task_id",
    );
    // A change that changes nothing records nothing.
    ok(&repo, &["task-language", "ja"]);
    // Reading never writes.
    ok(&repo, &["history", "A01-01"]);
    ok(&repo, &["list"]);
    assert_eq!(event_count(&repo), n);
}

#[test]
fn history_lists_a_tasks_changes_newest_first() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    ok(&repo, &["--as", "claude/x", "claim", "A01-01"]);
    ok(
        &repo,
        &["--as", "claude/x", "label", "A01-01", "implemented"],
    );
    let v = ok(&repo, &["history", "A01-01"]);
    assert!(v["since"].as_str().unwrap().ends_with('Z'));
    let events = v["events"].as_array().unwrap();
    let commands: Vec<&str> = events
        .iter()
        .map(|e| e["command"].as_str().unwrap())
        .collect();
    assert_eq!(commands, ["label", "claim", "ready", "add task"]);
    assert_eq!(events[0]["by"], "claude/x");
    assert_eq!(events[0]["via"], "alm");
    assert_eq!(
        events[1]["changes"],
        json!([
            { "field": "state", "from": "ready", "to": "in_progress" },
            { "field": "holder", "to": "claude/x" }
        ])
    );
    // Nobody named: no `by`.
    assert!(events[2].get("by").is_none());
    // Another task's events are not listed; an unknown task is refused.
    assert_eq!(
        ok(&repo, &["history", "A01-02"])["events"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    refused(&repo, &["history", "A09-09"], 1, "task_not_found");
    // For people: newest first, one line per change.
    let out = alm(&repo, &["history", "A01-01"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("History of A01-01 (recorded since "),
        "{text}"
    );
    assert!(
        text.contains(
            "claude/x (alm)  claim\n  state: ready → in_progress\n  holder: — → claude/x\n"
        ),
        "{text}"
    );
    assert!(
        text.find("  label").unwrap() < text.find("  claim").unwrap(),
        "{text}"
    );
}

#[test]
fn two_copies_with_different_events_merge_without_conflict() {
    let (repo, remote) = real_repo();
    ok(&repo, &["init"]);
    git(&repo, &["add", "--", ".allomos"]);
    git(&repo, &["commit", "--quiet", "-m", "Add Allomos taskbag"]);
    run_printed(&repo, &ok(&repo, &["add", "area", "A01", "One"]));
    run_printed(
        &repo,
        &ok(
            &repo,
            &["add", "task", "A01-01", "--area", "A01", "--title", "T1"],
        ),
    );
    run_printed(
        &repo,
        &ok(
            &repo,
            &["add", "task", "A01-02", "--area", "A01", "--title", "T2"],
        ),
    );
    let other = repo.with_file_name("other");
    git(
        repo.parent().unwrap(),
        &[
            "clone",
            "--quiet",
            "-b",
            "main",
            remote.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    git(&other, &["config", "user.name", "Other"]);
    git(&other, &["config", "user.email", "other@example.com"]);
    // Each copy changes a different task, at the same time, and records its own events.
    run_printed(&other, &ok(&other, &["--as", "codex/b", "claim", "A01-02"]));
    // This copy commits without pushing: the first of the printed commands only.
    for args in [
        &["--as", "claude/a", "claim", "A01-01"][..],
        &["--as", "claude/a", "label", "A01-01", "implemented"],
    ] {
        let mut v = ok(&repo, args);
        v["git_commands"].as_array_mut().unwrap().truncate(2);
        run_printed(&repo, &v);
    }
    let before = event_count(&repo);
    git(
        &repo,
        &[
            "pull",
            "--quiet",
            "--no-rebase",
            "--no-edit",
            "origin",
            "main",
        ],
    );
    assert_eq!(event_count(&repo), before + 1);
    assert_eq!(git(&repo, &["status", "--porcelain"]), "");
    let mine = ok(&repo, &["history", "A01-01"]);
    assert_eq!(mine["events"][0]["command"], "label");
    let theirs = ok(&repo, &["history", "A01-02"]);
    assert_eq!(theirs["events"][0]["by"], "codex/b");
}

#[test]
fn done_and_handoffs_check_give_the_agent_the_rule_to_check_itself() {
    let repo = filled();
    // alm done always ends with the rule for the finished task; alm looks at no handoff.
    ok(&repo, &["--as", "a", "claim", "A02-01"]);
    let v = ok(&repo, &["--as", "a", "done", "A02-01"]);
    let rule = v["handoff_check"]["instruction"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(
        rule.starts_with("Now check the handoffs that name A02-01 yourself"),
        "{rule}"
    );
    assert!(rule.contains("`alm handoffs --task A02-01`"));
    assert!(rule.contains("`alm handoff --remove <file>`."));
    ok(&repo, &["--as", "b", "claim", "A01-01"]);
    let text = String::from_utf8(alm(&repo, &["--as", "b", "done", "A01-01"]).stdout).unwrap();
    assert!(text.starts_with("A01-01  done"), "{text}");
    assert!(text.contains("Now check the handoffs that name A01-01 yourself, one by one:\n1. Get the list: `alm handoffs --task A01-01`."), "{text}");

    // alm handoffs --check: only the rule, with or without a task.
    let v = ok(&repo, &["handoffs", "--check"]);
    assert!(v.get("handoffs").is_none(), "{v}");
    assert!(
        v["instruction"]
            .as_str()
            .unwrap()
            .contains("1. Get the list: `alm handoffs`.")
    );
    let v = ok(&repo, &["handoffs", "--check", "--task", "A01-01"]);
    // The done's text: the task, the rule, then the Git commands for the done itself.
    let rule = text.split_once('\n').unwrap().1;
    let rule = &rule[..rule.find("\nCommit and push this change:").unwrap()];
    assert_eq!(v["instruction"], json!(rule));

    // The agent removes a handoff by the name alm handoffs lists; then it is gone.
    let file = ok(&repo, &["handoff", "--as", "s/one", "--tasks", "A01-01"])["handoff"]["file"]
        .as_str()
        .unwrap()
        .to_string();
    let v = ok(&repo, &["handoff", "--remove", &file]);
    assert_eq!(v["removed"]["writer"], "s/one");
    assert_eq!(ok(&repo, &["handoffs"])["handoffs"], json!([]));
    assert_eq!(ok(&repo, &["list"])["issues"], json!([]));
    refused(
        &repo,
        &["handoff", "--remove", &file],
        1,
        "handoff_not_found",
    );
    refused(
        &repo,
        &["handoff", "--remove", "../project.json"],
        1,
        "handoff_not_found",
    );
    assert!(repo.join(".allomos/taskbag/project.json").is_file());
}

/// Decision 0016 (Task-A01-31): waiting on a person, with an optional who.
#[test]
fn wait_says_what_and_optionally_who() {
    let repo = filled();
    refused(
        &repo,
        &["wait", "A01-01", "--on", " "],
        1,
        "waiting_on_missing",
    );
    let v = ok(
        &repo,
        &["wait", "A01-01", "--on", "reply on licence", "--who", "Ann"],
    );
    assert_eq!(v["task"]["state"], "waiting");
    assert_eq!(v["task"]["who"], "Ann");
    assert_eq!(task_file(&repo, "A01-01")["who"], "Ann");
    let out = alm(&repo, &["list"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("waiting for Ann: reply on licence"), "{text}");
    // Without --who, the owner is meant; a who left over is cleared.
    ok(&repo, &["wait", "A01-01", "--on", "approve the card"]);
    let file = task_file(&repo, "A01-01");
    assert!(file.get("who").is_none(), "{file}");
    let text = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(text.contains("waiting on: approve the card"), "{text}");
}

/// Decision 0016: `alm block` and its refusals; each blocker's state in list, show and next; a
/// dropped blocker flagged; all done told but not acted on; one event per change.
#[test]
fn block_shows_blockers_and_next_only_tells() {
    let repo = filled();
    refused(
        &repo,
        &["block", "A01-02", "--by", "A01-02"],
        1,
        "blocked_by_self",
    );
    refused(
        &repo,
        &["block", "A01-02", "--by", "A09-09"],
        1,
        "blocker_unknown",
    );
    refused(
        &repo,
        &["block", "A01-02", "--by", " "],
        1,
        "blocked_by_missing",
    );
    let (code, _) = fj(&repo, &["block", "A01-02"]);
    assert_eq!(code, 2, "--by is required");
    assert_eq!(
        task_file(&repo, "A01-02")["state"],
        "backlog",
        "nothing written"
    );

    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let v = ok(&repo, &["block", "A01-02", "--by", "A01-01", "A02-01"]);
    assert_eq!(v["task"]["state"], "blocked");
    assert_eq!(v["task"]["blocked_by"], json!(["A01-01", "A02-01"]));
    assert_eq!(
        v["task"]["blockers"],
        json!([{ "id": "A01-01", "state": "in_progress" }, { "id": "A02-01", "state": "backlog" }])
    );
    assert_eq!(
        task_file(&repo, "A01-02")["blocked_by"],
        json!(["A01-01", "A02-01"])
    );
    // People read each blocker's current state.
    let list = String::from_utf8(alm(&repo, &["list"]).stdout).unwrap();
    assert!(
        list.contains("blocked by A01-01 (in progress), A02-01 (backlog)"),
        "{list}"
    );
    let v = ok(&repo, &["list", "--state", "blocked"]);
    assert_eq!(v["areas"][0]["tasks"][0]["id"], "A01-02");
    assert_eq!(v["areas"][0]["tasks"][0]["all_blockers_done"], false);
    let show = String::from_utf8(alm(&repo, &["show", "A01-02"]).stdout).unwrap();
    assert!(
        show.contains("blocked by: A01-01 (in progress), A02-01 (backlog)"),
        "{show}"
    );
    // Not ready, so never in next; nothing to tell yet.
    let v = ok(&repo, &["next"]);
    assert_eq!(v["blocked"], json!([]));

    // A dropped blocker is flagged, never counted as done.
    ok(&repo, &["--as", "a", "done", "A01-01"]);
    ok(&repo, &["drop", "A02-01"]);
    let v = ok(&repo, &["next"]);
    assert_eq!(v["blocked"][0]["id"], "A01-02");
    assert_eq!(v["blocked"][0]["all_blockers_done"], false);
    let next = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(next.contains("A02-01 (dropped; still blocked?)"), "{next}");
    // Blocking on a dropped task is allowed, with a warning.
    let v = ok(&repo, &["block", "A01-02", "--by", "A02-01"]);
    assert_eq!(v["warnings"][0]["code"], "blocker_dropped");

    // Every blocker done: next tells, and moves nothing.
    ok(&repo, &["block", "A01-02", "--by", "A01-01"]);
    let before = fs::read_to_string(repo.join(".allomos/taskbag/tasks/A01-02/task.json")).unwrap();
    let events_before = fs::read_dir(repo.join(".allomos/taskbag/events"))
        .unwrap()
        .count();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["blocked"][0]["id"], "A01-02");
    assert_eq!(v["blocked"][0]["all_blockers_done"], true);
    let next = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(
        next.contains("A01-02: all blockers are done (A01-01 (done)) — unblock it?"),
        "{next}"
    );
    assert_eq!(
        fs::read_to_string(repo.join(".allomos/taskbag/tasks/A01-02/task.json")).unwrap(),
        before,
        "next changes nothing"
    );
    assert_eq!(
        fs::read_dir(repo.join(".allomos/taskbag/events"))
            .unwrap()
            .count(),
        events_before
    );

    // Unblock moves it on; only a blocked task.
    let v = ok(&repo, &["unblock", "A01-02"]);
    assert_eq!(v["task"]["state"], "ready");
    assert!(task_file(&repo, "A01-02").get("blocked_by").is_none());
    refused(&repo, &["unblock", "A01-02"], 1, "not_blocked");
    // One event per change, named after the command.
    let v = ok(&repo, &["history", "A01-02"]);
    let commands: Vec<&str> = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["command"].as_str().unwrap())
        .collect();
    assert_eq!(commands, ["unblock", "block", "block", "block", "add task"]);
}

/// A taskbag in format 4 is newer than an allomos/3 build: the older build says so
/// (`taskbag_newer`). Shown with this build reading a format 5 taskbag, the same rule.
#[test]
fn a_newer_taskbag_is_refused_by_an_older_build() {
    let repo = filled();
    set_format(&repo, "allomos/5");
    refused(&repo, &["list"], 4, "taskbag_newer");
}

/// Task-A03-22: each task records its base branch by its real name; `alm add task --branch`
/// chooses it from the branch the agent is on; `alm claim --branch` records the working branch and
/// prints the base to start from and merge into. Only events change: task.json keeps its shape.
#[test]
fn tasks_record_their_base_branch_and_claims_say_where_to_merge() {
    let repo = filled();
    // Tasks made before (here: without --branch) have no base.
    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["branches"], json!({}));

    // A task without a base gets the branch the agent is on at claim.
    let v = ok(
        &repo,
        &[
            "--as",
            "s1",
            "claim",
            "A01-01",
            "--branch",
            "storage-v2",
            "--working",
            "claude/a01-01",
        ],
    );
    assert_eq!(
        v["branches"],
        json!({ "base": "storage-v2", "working": "claude/a01-01" })
    );
    assert_eq!(v["base_why"]["why"], "none_before");
    assert_eq!(
        v["branch_commands"]["start"],
        json!([
            "git switch storage-v2",
            "git pull --ff-only",
            "git switch -c claude/a01-01"
        ])
    );
    assert_eq!(v["branch_commands"]["merge"][2], "git merge claude/a01-01");
    assert_eq!(v["branch_facts"], json!([]));
    // task.json has no new field: the branches are in the claim's event.
    let file = task_file(&repo, "A01-01");
    assert!(file.get("base_branch").is_none() && file.get("working_branch").is_none());
    let h = ok(&repo, &["history", "A01-01"]);
    let fields: Vec<&str> = h["events"][0]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    assert!(
        fields.contains(&"base_branch") && fields.contains(&"working_branch"),
        "{h}"
    );

    // On a task's working branch, a new task shares that task's base.
    let v = ok(
        &repo,
        &[
            "--as",
            "s1",
            "add",
            "task",
            "--area",
            "A01",
            "--title",
            "Next",
            "--branch",
            "claude/a01-01",
        ],
    );
    assert_eq!(v["base"]["base"], "storage-v2");
    assert_eq!(v["base"]["why"], "working_branch_of");
    assert_eq!(v["base"]["task"], "A01-01");
    assert_eq!(v["warnings"], json!([]));
    let next = v["task"]["id"].as_str().unwrap().to_string();

    // On another branch, the base is that branch itself; the session holding a task on another
    // base is told, and may change it with `alm base`.
    let v = ok(
        &repo,
        &[
            "--as", "s1", "add", "task", "--area", "A02", "--title", "Other", "--branch", "master",
        ],
    );
    assert_eq!(v["base"]["base"], "master");
    assert_eq!(v["base"]["why"], "current_branch");
    assert_eq!(v["warnings"][0]["code"], "base_differs");
    assert_eq!(v["warnings"][0]["params"]["held_base"], "storage-v2");
    let other = v["task"]["id"].as_str().unwrap().to_string();
    let v = ok(&repo, &["--as", "s1", "base", &other, "storage-v2"]);
    assert_eq!(v["branches"]["base"], "storage-v2");
    // --base overrides the choice.
    let v = ok(
        &repo,
        &[
            "add", "task", "--area", "A02", "--title", "Given", "--branch", "master", "--base",
            "main",
        ],
    );
    assert_eq!(
        (&v["base"]["base"], &v["base"]["why"]),
        (&json!("main"), &json!("given"))
    );
    refused(
        &repo,
        &[
            "add",
            "task",
            "--area",
            "A02",
            "--title",
            "Bad",
            "--base",
            "my branch",
        ],
        1,
        "branch_invalid",
    );
    // Without --branch: added, no base, and a warning.
    let v = ok(
        &repo,
        &["add", "task", "--area", "A02", "--title", "Unsaid"],
    );
    assert_eq!(v["base"], Value::Null);
    assert_eq!(v["warnings"][0]["code"], "branch_missing");

    // A claim on another branch than the base lists it as a fact; the base is kept.
    let v = ok(
        &repo,
        &["--as", "s2", "claim", &next, "--branch", "claude/a01-01"],
    );
    assert_eq!(
        v["branches"],
        json!({ "base": "storage-v2", "working": "claude/a01-01" })
    );
    assert_eq!(v["base_why"]["why"], "recorded");
    assert_eq!(v["branch_commands"]["start"], json!([]));
    assert_eq!(
        v["branch_facts"],
        json!(["You are on claude/a01-01, not on storage-v2, the task's base."])
    );
    // Working on the base itself: nothing to merge.
    let v = ok(
        &repo,
        &["--as", "s3", "claim", &other, "--branch", "storage-v2"],
    );
    assert_eq!(v["branch_commands"]["start"], json!(["git pull --ff-only"]));
    assert_eq!(v["branch_commands"]["merge"], json!([]));
    let text = String::from_utf8(alm(&repo, &["--as", "s3", "show", &other]).stdout).unwrap();
    assert!(
        text.contains("base branch: storage-v2\nworking branch: storage-v2\n"),
        "{text}"
    );
    // Someone else's task: its base is not yours to change.
    refused(
        &repo,
        &["--as", "s1", "base", &other, "main"],
        1,
        "task_held",
    );
    let (_, list) = fj(&repo, &["list"]);
    assert_eq!(list["issues"], json!([]));
}
