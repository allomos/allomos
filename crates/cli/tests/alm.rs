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

/// The test run's own settings folder, so no test reads or writes the person's real settings (their
/// quota reports, their pseudonyms: Task-A01-black004).
fn test_settings() -> PathBuf {
    std::env::temp_dir().join(format!("alm-test-settings-{}", std::process::id()))
}

/// `alm`, with the test run's own settings folder.
fn alm_command() -> Command {
    let mut c = Command::new(ALM);
    c.env("ALLOMOS_SETTINGS_DIR", test_settings());
    c
}

/// `alm` in `dir`, with no Allomos settings inherited from the test's own environment. Its
/// settings folder is an empty one of the test run's own, so the person's real settings and quota
/// reports never change a result (Task-A03-flour054).
fn alm(dir: &Path, args: &[&str]) -> Output {
    alm_command()
        .current_dir(dir)
        .args(args)
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env_remove("ALLOMOS_LOCK_WAIT_MS")
        .env_remove("CLAUDECODE")
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        .env_remove("CODEX_THREAD_ID")
        .env_remove("CODEX_SANDBOX")
        .env_remove("CODEX_SANDBOX_NETWORK_DISABLED")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("ALLOMOS_MODEL")
        .env_remove("ALLOMOS_FOR")
        .env_remove("ALLOMOS_APPROVAL")
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
        !text.contains("Save this change") && !text.contains("Save only these files"),
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

    let v = ok(
        &sub,
        &[
            "--as",
            "agent-a",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
    );
    assert_eq!(v["task"]["labels"], json!(["locally verified by agent"]));
    let v = ok(
        &sub,
        &["--as", "agent-a", "wait", "A01-01", "--on", "owner review"],
    );
    assert_eq!(v["task"]["state"], "waiting");
    assert_eq!(v["task"]["waiting_on"], "owner review");

    // Finished without the human check (decision 0020: no "by human" label yet).
    let v = ok(
        &sub,
        &[
            "--as",
            "agent-a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
    assert_eq!(v["task"]["state"], "done");
    // Without steps, `alm` asks for them (Task-A03-18).
    assert_eq!(v["warnings"][0]["code"], "human_check_steps_missing");

    // The files say what the command said.
    let file = task_file(&repo, "A01-01");
    assert_eq!(file["state"], "done");
    assert_eq!(file["labels"], json!(["locally verified by agent"]));
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
    assert!(
        text.contains(
            "A01  Tracking\n  A01-01  done  locally verified by agent  agent-a  Open a taskbag"
        ),
        "{text}"
    );
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
    // A handed-off task comes with its handoff note (Task-A03-polar770).
    ok(
        &repo,
        &[
            "handoff",
            "A01-02",
            "--as",
            "claude/run",
            "--text",
            "stopped at tests",
        ],
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
    assert_eq!(v["tasks"][1]["handoff_note"], "stopped at tests");
    assert_eq!(v["tasks"][0]["handoff_note"], Value::Null);
    assert!(v["tasks"][1].get("handoff").is_none(), "{v}");
    let rule = v["instruction"].as_str().unwrap();
    assert!(
        rule.starts_with("Take the task the person named, if any."),
        "{rule}"
    );
    assert!(
        rule.contains("Get the latest changes first, then claim it with `alm claim <id> --as <your session name> --branch <the branch you are on>`")
    );
    assert!(rule.contains("until your claim is shared"), "{rule}");
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
        &["--as", "b", "label", "A01-01", "--remove", "shipped"],
        1,
        "label_not_on_task",
    );
    refused(
        &repo,
        &["--as", "b", "wait", "A01-01", "--on", " "],
        1,
        "waiting_on_missing",
    );
    ok(
        &repo,
        &[
            "--as",
            "b",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
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
    let out = alm_command()
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
    let out = alm_command()
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
                    alm_command()
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
    // One event per change: init, add area, add task, claim (Task-A01-17), and the first signed
    // change's `add key` (Task-A01-gusto404).
    assert_eq!(events.len(), 5, "{events:?}");
    assert!(events.iter().all(|f| f.ends_with(".json")));
    // This computer's public key (its name is the key's ID).
    let (keys, files): (Vec<String>, Vec<String>) = files
        .into_iter()
        .partition(|f| f.starts_with(".allomos/taskbag/keys/"));
    assert_eq!(keys.len(), 1, "{keys:?}");
    assert_eq!(
        files,
        [
            ".allomos/taskbag/.gitattributes",
            ".allomos/taskbag/.gitignore",
            ".allomos/taskbag/areas/.keep",
            ".allomos/taskbag/areas/A01.json",
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
    // The five built-in labels first, then the project's own (decision 0020).
    let names: Vec<&str> = project["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 7);
    assert_eq!(names[0], "locally verified by agent");
    assert_eq!(&names[5..], ["drafted", "owner verified"]);
}

/// Task-A01-50: `alm reopen` turns a done or dropped task into a ready one held by nobody, with
/// the reason in its history; any other state, and a reopen without a reason, are refused.
#[test]
fn reopen_a_done_or_dropped_task() {
    let repo = filled();
    // Only done or dropped tasks; a reason is required.
    refused(
        &repo,
        &["reopen", "A01-01", "--note", "x"],
        1,
        "cannot_reopen",
    );
    assert_eq!(
        alm(&repo, &["reopen", "A01-01"]).status.code(),
        Some(2),
        "--note is required"
    );
    // Done without the human check, with a label: reopened by someone else (finished tasks are
    // held by nobody); label and human-check record cleared, handoff note and description kept.
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(
        &repo,
        &["--as", "a", "label", "A01-01", "locally verified by agent"],
    );
    ok(&repo, &["--as", "a", "note", "A01-01", "see the log"]);
    ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "loop run",
        ],
    );
    refused(
        &repo,
        &["--as", "b", "reopen", "A01-01", "--note", " "],
        1,
        "reopen_note_missing",
    );
    let v = ok(
        &repo,
        &[
            "--as",
            "b",
            "reopen",
            "A01-01",
            "--note",
            "The check failed: no total",
        ],
    );
    let t = &v["task"];
    assert_eq!(t["state"], "ready");
    for field in [
        "holder",
        "label",
        "without_human_check",
        "human_check_steps",
    ] {
        assert_eq!(t[field], Value::Null, "{field}");
    }
    assert_eq!(t["handoff_note"], "see the log");
    let file = task_file(&repo, "A01-01");
    assert_eq!(file["state"], "ready");
    assert!(
        file.get("labels")
            .is_none_or(|l| l.as_array().is_none_or(Vec::is_empty))
    );
    // The history: who, the command, the cleared fields and the reason.
    let h = ok(&repo, &["history", "A01-01"]);
    let e = &h["events"][0];
    assert_eq!((&e["command"], &e["by"]), (&json!("reopen"), &json!("b")));
    let changes = e["changes"].as_array().unwrap();
    let change = |f: &str| changes.iter().find(|c| c["field"] == f).cloned();
    assert_eq!(
        change("state"),
        Some(json!({ "field": "state", "from": "done", "to": "ready" }))
    );
    let labels = change("labels").expect("the labels change");
    assert_eq!(labels["from"], json!(["locally verified by agent"]));
    assert_eq!(
        change("reopen_note"),
        Some(json!({ "field": "reopen_note", "to": "The check failed: no total" }))
    );
    assert!(change("without_human_check").is_some());
    // It is ready for anyone: next lists it, and another session claims it.
    let next = ok(&repo, &["next"]);
    assert!(next["tasks"].to_string().contains("A01-01"), "{next}");
    ok(&repo, &["--as", "c", "claim", "A01-01"]);
    refused(
        &repo,
        &["reopen", "A01-01", "--note", "x"],
        1,
        "cannot_reopen",
    );
    // A dropped task too.
    ok(&repo, &["drop", "A01-02"]);
    let v = ok(&repo, &["reopen", "A01-02", "--note", "Wanted after all"]);
    assert_eq!(v["task"]["state"], "ready");
    let out = alm(&repo, &["history", "A01-02"]);
    assert!(String::from_utf8_lossy(&out.stdout).contains("reopen_note: — → Wanted after all"));
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

/// The command line `c` as a person pasting it would run it: `sh -c` here, `cmd /C` on Windows.
/// On Windows the line goes to `cmd` as it is (`raw_arg`): `args` would wrap it in quotes and
/// escape its own quotes as `\"`, which `cmd` does not undo, so Git got `"C:\…"` with the quotes.
fn shell(dir: &Path, c: &str) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("cmd");
        cmd.current_dir(dir).arg("/C").raw_arg(c);
        cmd
    }
    #[cfg(not(windows))]
    {
        let mut cmd = Command::new("sh");
        cmd.current_dir(dir).args(["-c", c]);
        cmd
    }
}

/// Run the Git commands `alm` printed (its `git_commands`) in `dir`, as an agent would.
fn run_printed(dir: &Path, v: &Value) {
    let commands = v["git_commands"]
        .as_array()
        .unwrap_or_else(|| panic!("{v}"));
    assert!(!commands.is_empty(), "{v}");
    for c in commands {
        let c = c.as_str().unwrap();
        let out = shell(dir, c).output().unwrap();
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
    let v = ok(
        &repo,
        &["add", "task", "A01-01", "--area", "A01", "--title", "T"],
    );
    assert_eq!(
        v["git_instruction"],
        "Save this change in your version control tool and share it. Changed files:"
    );
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
        "Save this change in your version control tool and share it now, before you start the work. Until it is shared, other sessions do not see your claim and may take this task too. Changed files:"
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
        "Nothing changed, so there is nothing to save."
    );
    // For people: the same commands, then the rule.
    let out = alm(
        &repo,
        &[
            "--as",
            "agent-a",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains(
            "\nSave this change in your version control tool and share it. Changed files:\n  "
        ) && text.contains("\nFor Git:\n  git -C "),
        "{text}"
    );
    assert!(
        text.contains("commit -m \"alm: label A01-01\" -m \"Agent: agent-a\" -- events/"),
        "{text}"
    );
    assert!(
        text.contains("Save only these files; leave other changes as they are.\n"),
        "{text}"
    );
}

/// A command that moves a task on asks for a quota report when the agent's is missing or 30
/// minutes old (Task-A03-flour054); a fresh report silences it; other commands never ask.
#[test]
fn moving_a_task_on_asks_for_a_missing_or_old_quota_report() {
    let repo = filled();
    let settings = tmp("quota-ask-settings");
    let run = |args: &[&str], now: Option<&str>| {
        let mut c = alm_command();
        c.current_dir(&repo)
            .args(args)
            .env("ALLOMOS_SETTINGS_DIR", &settings)
            .env_remove("ALLOMOS_TASKBAG")
            .env_remove("ALLOMOS_HOLDER");
        if let Some(now) = now {
            c.env("ALLOMOS_NOW", now);
        }
        let out = c.output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    let text = run(
        &[
            "--as",
            "claude/a03",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
        None,
    );
    assert!(
        text.contains(
            "Quota: Your agent (claude-code) has not reported its quota on this computer."
        ) && text.ends_with(
            "\n  alm quota --agent claude-code --left <percent> [--resets <local date and time>]\n"
        ),
        "{text}"
    );
    run(
        &["quota", "--agent", "claude-code", "--left", "80"],
        Some("2026-10-21T10:00:00Z"),
    );
    let text = run(
        &[
            "--as",
            "claude/a03",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
        Some("2026-10-21T10:29:00Z"),
    );
    assert!(!text.contains("Quota:"), "{text}");
    let v: Value = serde_json::from_str(&run(
        &[
            "--json",
            "--as",
            "claude/a03",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
        Some("2026-10-21T10:45:00Z"),
    ))
    .unwrap();
    assert_eq!(v["quota_ask"]["agent"], json!("claude-code"));
    assert_eq!(v["quota_ask"]["minutes"], json!(45));
    // Reading commands never ask.
    assert!(!run(&["list"], Some("2026-10-22T10:00:00Z")).contains("Quota:"));
    assert!(
        !run(
            &["--as", "claude/a03", "show", "A01-01"],
            Some("2026-10-22T10:00:00Z")
        )
        .contains("Quota:")
    );
}

/// Task-A03-possum352: a loop session claims under a name that names no agent (`loop-hourly`).
/// The quota ask names the agent the claim recorded (`--session-agent`), or else shows a
/// placeholder; it never prints the session name as the agent's id.
#[test]
fn the_quota_ask_never_names_a_plain_session_name_as_the_agent() {
    let repo = filled();
    let settings = tmp("quota-ask-session-settings");
    let run = |args: &[&str]| {
        let out = alm_command()
            .current_dir(&repo)
            .args(args)
            .env("ALLOMOS_SETTINGS_DIR", &settings)
            .env_remove("ALLOMOS_TASKBAG")
            .env_remove("ALLOMOS_HOLDER")
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{args:?}: {out:?}");
        String::from_utf8(out.stdout).unwrap()
    };
    let text = run(&["--as", "loop-hourly", "claim", "A01-01", "--branch", "main"]);
    assert!(
        text.contains("\n  alm quota --agent <your agent's id, e.g. claude-code or codex> --left")
            && !text.contains("--agent loop-hourly"),
        "{text}"
    );
    let text = run(&[
        "--as",
        "loop-hourly",
        "session",
        "A01-01",
        "--session-agent",
        "claude-code",
    ]);
    assert!(!text.contains("Quota:"), "{text}");
    let text = run(&[
        "--as",
        "loop-hourly",
        "label",
        "A01-01",
        "locally verified by agent",
    ]);
    assert!(
        text.contains("Quota: Your agent (claude-code) has not reported its quota")
            && text.contains("\n  alm quota --agent claude-code --left <percent>"),
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
        let out = alm_command()
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
            "add", "task", "A01-01", "--area", "A01", "--title", "T", "--branch", "main",
        ],
        &["--as", "a", "claim", "A01-01", "--branch", "main"],
        &["--as", "a", "label", "A01-01", "locally verified by human"],
        &[
            "--as",
            "a",
            "handoff",
            "A01-01",
            "--text",
            "done; nothing left",
        ],
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
        let (code, v) = run(&[
            "--as",
            "a",
            "label",
            "A01-01",
            "locally verified by agent",
            flag,
        ]);
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
    let steps = steps.to_str().unwrap();
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
        vec!["--as", "a", "label", "A01-01", "locally verified by agent"],
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
        vec!["labels", "add", "drafted", "--meaning", "Written down"],
        vec!["rename", "label", "drafted", "sketched"],
        vec!["labels", "remove", "sketched"],
        vec!["project", "--name", "Demo 2"],
        vec!["task-language", "ja"],
        vec!["handoff-rules", "--quota", "5"],
        vec![
            "--as",
            "b",
            "handoff",
            "A01-02",
            "--text",
            "all done",
            "--evidence",
            "deployed",
        ],
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
    assert!(
        text.contains("Save this change in your version control tool and share it. Changed files:"),
        "{text}"
    );
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
    // The person's pseudonym, never the name the app was given (Task-A01-frolic587). This test
    // process may have no settings folder for one (then nobody is named).
    assert!(!v.to_string().contains("Ann"), "{v}");
    let by: Vec<&str> = app_changes["by"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b.as_str().unwrap())
        .collect();
    assert!(
        by.len() <= 1 && by.iter().all(|b| b.starts_with("p-")),
        "{v}"
    );
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
        commit.contains("commit -m \"app: changes made in the Allomos app\" -m \"By: "),
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
    let v = ok(
        &repo,
        &["--as", "a", "label", "A01-01", "locally verified by agent"],
    );
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
    ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
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
        text.starts_with("1  I01  v0.1  (v0.1, until 2026-10-07)  [active]\n  A01-01  done"),
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
    let child = alm_command()
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
        allomos_store::answer_confirm(project, &asked.id, yes, None).unwrap();
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

/// `alm confirm --task --evidence` (Task-A03-37): the popup carries the evidence and the task; a
/// Go ahead counts only for the plan shown and is recorded in the task's history; a yes given for
/// another version of the plan counts as no.
#[test]
fn a_go_ahead_counts_only_for_the_plan_shown_and_is_recorded() {
    let repo = filled();
    fs::write(repo.join("plan.md"), "- Rename the field\n").unwrap();
    fs::write(repo.join("evidence.md"), "- 58 tests pass\n").unwrap();
    let ask = |shown: Option<&str>| {
        let child = alm_command()
            .current_dir(&repo)
            .args([
                "--json",
                "--as",
                "claude/test",
                "confirm",
                "--title",
                "Rename",
            ])
            .args(["--plan", "plan.md", "--evidence", "evidence.md"])
            .args(["--task", "A01-01", "--timeout", "20"])
            .env_remove("ALLOMOS_TASKBAG")
            .env_remove("ALLOMOS_HOLDER")
            .env("ALLOMOS_NO_LAUNCH", "1")
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let asked = (0..100)
            .find_map(|_| {
                std::thread::sleep(Duration::from_millis(100));
                allomos_store::pending_confirms(&repo).pop()
            })
            .expect("the question appears");
        assert_eq!(asked.evidence.as_deref(), Some("- 58 tests pass\n"));
        assert_eq!(asked.task.as_deref(), Some("A01-01"));
        let shown = shown.unwrap_or(&asked.fingerprint).to_string();
        allomos_store::answer_confirm(&repo, &asked.id, true, Some(&shown)).unwrap();
        let out = child.wait_with_output().unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code().unwrap(), v, asked.fingerprint)
    };
    let (code, v, fingerprint) = ask(None);
    assert_eq!((code, &v["answer"]), (0, &json!("yes")), "{v}");
    assert_eq!(v["fingerprint"], json!(fingerprint));
    // Since Task-A01-coach782 the Go ahead is an approval record, channel confirm, fixing the
    // exact plan text shown by its SHA-256.
    let approval = &v["approval"];
    assert_eq!(approval["channel"], "confirm", "{v}");
    assert_eq!(approval["words"], "Go ahead: Rename");
    assert_eq!(approval["scope"]["plan"], "plan.md");
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["approvals"][0]["id"], approval["id"], "{shown}");
    assert_eq!(shown["approvals"][0]["status"], "applies");
    // Shown another version of the plan: not a Go ahead for this one.
    let (code, v, _) = ask(Some("0123456789abcdef"));
    assert_eq!((code, &v["error"]["code"]), (1, &json!("confirm_declined")));
    // An unknown task is refused before the person is asked.
    refused(
        &repo,
        &[
            "confirm", "--title", "x", "--plan", "plan.md", "--task", "A09-09",
        ],
        1,
        "task_not_found",
    );
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
    // Decision 0020: the project's label table; built-in labels stay, others are added and
    // removed, and a label in use is not removed.
    let repo = filled();
    let v = ok(
        &repo,
        &[
            "labels",
            "add",
            "connected",
            "--meaning",
            "Wired end to end",
        ],
    );
    let names: Vec<&str> = v["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"locally verified by agent") && names.contains(&"connected"));
    // Adding it again changes nothing.
    let v = ok(&repo, &["labels", "add", "connected"]);
    assert_eq!(v["changed_files"], json!([]));
    ok(&repo, &["label", "A01-01", "connected"]);
    refused(
        &repo,
        &["labels", "remove", "locally verified by agent"],
        1,
        "label_built_in",
    );
    refused(&repo, &["labels", "remove", "connected"], 1, "label_in_use");
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    let entry = project["labels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["name"] == "connected")
        .unwrap();
    assert_eq!(entry["meaning"], "Wired end to end");
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
    // A topic in separate words, and the words after it (Task-A03-planet530).
    let v = ok(&repo, &["guide", "github", "release", "minor"]);
    assert_eq!(v["topic"], json!("github-release"));
    assert_eq!(v["words"], json!(["minor"]));
    let text = v["guide"].as_str().unwrap();
    assert!(text.starts_with("Words after the topic, for you to read as this guide says: minor"));
    assert!(text.contains(
        "# GitHub release: publish a new release, and install it on this computer when asked"
    ));
    let v = ok(&repo, &["guide", "github-release", "--language", "ja"]);
    assert_eq!(v["written_in_language"], json!(false));
    assert!(
        v["guide"]
            .as_str()
            .unwrap()
            .starts_with("This text is not written in ja yet: follow")
    );
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
    // A word ID in its area, never the next number (Task-A01-54).
    let id = v["task"]["id"].as_str().unwrap().to_string();
    assert!(
        id.starts_with("A01-") && !allomos_core::ids::numbered(&id),
        "{id}"
    );
    assert_eq!(v["task"]["state"], "backlog");
    let v = ok(
        &repo,
        &["add", "task", "--area", "A02", "--title", "Charts"],
    );
    assert!(v["task"]["id"].as_str().unwrap().starts_with("A02-"));
    // `alm list` shows the numbered tasks first, then the new ones in the order they were made.
    let list = ok(&repo, &["list", "--area", "A01"]);
    let ids: Vec<&str> = list["areas"][0]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.last(), Some(&id.as_str()), "{list}");
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
    // An unused `I-<labor word>` (Task-A00-rope803).
    let first = v["iteration"]["id"].as_str().unwrap().to_string();
    assert!(first.starts_with("I-"), "{first}");
    assert_eq!(v["iteration"]["release"], "v0.1");
    let v = ok(
        &repo,
        &["add", "iteration", "Second", "--start", "2026-10-15"],
    );
    let second = v["iteration"]["id"].as_str().unwrap();
    assert!(second.starts_with("I-") && second != first, "{second}");
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
        &["add", "iteration", &first, "Again"],
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
    let files = fs::read_dir(repo.join(".allomos/taskbag/iterations")).unwrap();
    assert_eq!(files.count(), 2);
}

/// `alm rename iteration` (Task-A00-rope803): a new ID, with every task planned in it moved
/// along; one event for the iteration and one for each task.
#[test]
fn an_iteration_is_renamed_with_its_tasks() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    let events = || fs::read_dir(bag.join("events")).unwrap().count();
    ok(
        &repo,
        &["add", "iteration", "I01", "One", "--release", "v0.1"],
    );
    ok(&repo, &["add", "iteration", "I02", "Two"]);
    ok(&repo, &["change", "iteration", "I01", "--state", "active"]);
    let tasks: Vec<String> = ok(&repo, &["list"])["areas"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|a| a["tasks"].as_array().unwrap().clone())
        .map(|t| t["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(tasks.len(), 3);
    ok(&repo, &["plan", &tasks[0], "I01"]);
    ok(&repo, &["plan", &tasks[1], "I01"]);
    ok(&repo, &["plan", &tasks[2], "I02"]);
    ok(&repo, &["drop", &tasks[1]]);
    let before = events();

    let v = ok(&repo, &["rename", "iteration", "I01", "I-knit"]);
    assert_eq!(v["command"], "rename iteration");
    assert_eq!(
        (
            &v["iteration"]["id"],
            &v["iteration"]["release"],
            &v["iteration"]["state"]
        ),
        (&json!("I-knit"), &json!("v0.1"), &json!("active"))
    );
    assert_eq!(v["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(events(), before + 3);
    assert!(!bag.join("iterations/I01.json").exists());
    assert!(bag.join("iterations/I-knit.json").exists());
    for (t, it) in [
        (&tasks[0], "I-knit"),
        (&tasks[1], "I-knit"),
        (&tasks[2], "I02"),
    ] {
        assert_eq!(ok(&repo, &["show", t])["task"]["iteration"], it, "{t}");
    }
    let v = ok(&repo, &["list", "--json"]);
    assert_eq!(v["issues"], json!([]));

    // The same ID: nothing written. Refusals: nothing written.
    let before = events();
    ok(&repo, &["rename", "iteration", "I-knit", "I-knit"]);
    refused(
        &repo,
        &["rename", "iteration", "I09", "I-sow"],
        1,
        "unknown_iteration",
    );
    refused(
        &repo,
        &["rename", "iteration", "I02", "I-knit"],
        1,
        "duplicate_iteration_id",
    );
    refused(
        &repo,
        &["rename", "iteration", "I02", "I.sow"],
        1,
        "invalid_id",
    );
    assert_eq!(events(), before);
}

#[test]
fn an_iteration_is_moved_before_or_after_another() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    let events = || fs::read_dir(bag.join("events")).unwrap().count();
    for (id, name) in [
        ("I01", "One"),
        ("I02", "Two"),
        ("I03", "Three"),
        ("I04", "Four"),
    ] {
        ok(&repo, &["add", "iteration", id, name]);
    }
    ok(&repo, &["change", "iteration", "I03", "--release", "v0.3"]);
    let ids = |v: &Value| -> Vec<String> {
        v["iterations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_str().unwrap().to_string())
            .collect()
    };
    let before = events();

    // Task-A00-skate868: one event per iteration whose order changed; nothing else changes.
    let v = ok(&repo, &["move", "iteration", "I03", "--before", "I02"]);
    assert_eq!(v["command"], "move iteration");
    assert_eq!(ids(&v), ["I01", "I03", "I02", "I04"]);
    assert_eq!(v["moved"].as_array().unwrap().len(), 2);
    assert_eq!(events(), before + 2);
    let v = ok(&repo, &["move", "iteration", "I04", "--before", "I03"]);
    assert_eq!(ids(&v), ["I01", "I04", "I03", "I02"]);
    let v = ok(&repo, &["move", "iteration", "I01", "--after", "I02"]);
    assert_eq!(ids(&v), ["I04", "I03", "I02", "I01"]);
    let v = ok(&repo, &["list", "--by", "iteration"]);
    assert_eq!(ids(&v), ["I04", "I03", "I02", "I01"]);
    assert_eq!(v["iterations"][1]["release"], "v0.3");
    assert_eq!(v["issues"], json!([]));

    // Already there: nothing written.
    let before = events();
    let v = ok(&repo, &["move", "iteration", "I03", "--after", "I04"]);
    assert_eq!(v["moved"], json!([]));
    ok(&repo, &["move", "iteration", "I03", "--before", "I03"]);
    assert_eq!(events(), before);

    // Refusals.
    refused(
        &repo,
        &["move", "iteration", "I09", "--before", "I01"],
        1,
        "unknown_iteration",
    );
    refused(
        &repo,
        &["move", "iteration", "I01", "--after", "I09"],
        1,
        "unknown_iteration",
    );
    for args in [
        &["move", "iteration", "I01"][..],
        &[
            "move",
            "iteration",
            "I01",
            "--before",
            "I02",
            "--after",
            "I03",
        ][..],
    ] {
        assert!(!alm(&repo, args).status.success(), "{args:?}");
    }
    assert_eq!(events(), before);
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
    ok(
        &repo,
        &[
            "--as",
            "w",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
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
    let out = alm_command()
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
        assert!(text.contains("(goal prompt v4)"), "{language}");
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
    assert!(brief.starts_with("## Tasks: Allomos (v36)"), "{brief}");
    fs::write(repo.join("AGENTS.md"), format!("# Rules\n\n{brief}")).unwrap();
    let v = ok(&repo, &["next"]);
    assert_eq!(v["warnings"], json!([]));

    // A v16 section (before personal details were kept out like credentials, Task-A03-29) is out of date too.
    fs::write(
        repo.join("AGENTS.md"),
        format!(
            "# Rules\n\n{}",
            brief.replace("## Tasks: Allomos (v36)", "## Tasks: Allomos (v22)")
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
        "hosted resource or a remote repository",
        "learn a pitfall",
        "a credential moves",
        "Change lines in place",
        "save and share that file with your version control tool yourself",
        "Personal details are kept out like credentials",
        "email address, account or user name",
        "Write a placeholder such as `<CLOUDFLARE_ACCOUNT_EMAIL>` and how to look it up",
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
    let v = ok(
        &repo,
        &["handoff", "A01-01", "--as", "s/one", "--text", "n"],
    );
    assert_eq!(v["project_notes"]["changed"], "2026-10-14");
    let out = alm(
        &repo,
        &["handoff", "A01-01", "--as", "s/one", "--text", "n"],
    );
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
    let v = ok(
        &repo,
        &["handoff", "A01-01", "--as", "s/one", "--text", "n"],
    );
    assert_eq!(v["project_notes"]["found"], false);
    let out = alm(
        &repo,
        &["handoff", "A01-01", "--as", "s/one", "--text", "n"],
    );
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("update `project-notes.md` too (this taskbag has none yet)")
    );
}

/// The project notes' path is printed full and tidy (Task-A03-32): from a worktree with a
/// relative `--taskbag`, no `.` or `..` is left in it.
#[test]
fn project_notes_path_is_tidy_from_a_worktree() {
    let base = tmp("notes-path");
    let bag = base.join("bag");
    fs::create_dir_all(&bag).unwrap();
    ok(&bag, &["init"]);
    let worktree = base.join("code/.claude/worktrees/one");
    fs::create_dir_all(&worktree).unwrap();
    let rel = "./../../../../bag/.allomos/taskbag";
    // The worktree as alm sees it from its working folder: resolved (`/private/var` on macOS),
    // but on Windows as given, since `canonicalize` there adds `\\?\` and expands short names
    // (`RUNNER~1`), which alm rightly does not print.
    let bag_seen = if cfg!(windows) {
        bag.clone()
    } else {
        fs::canonicalize(&bag).unwrap()
    };
    let want = bag_seen
        .join(".allomos")
        .join("taskbag")
        .join("project-notes.md");
    let v = ok(&worktree, &["--taskbag", rel, "next"]);
    assert_eq!(v["project_notes"]["path"], want.display().to_string());
    let out = alm(&worktree, &["--taskbag", rel, "next"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains(&want.display().to_string()), "{text}");
    assert!(!text.contains("/../") && !text.contains("/./"), "{text}");
}

/// After `alm add task`, alm tells the agent what to do next (Task-A03-33): plan it in an
/// iteration, from the ones listed, then make it ready unless something is the person's to
/// decide; after the Git commands, in the task language. alm itself plans and readies nothing.
#[test]
fn add_task_tells_the_next_steps() {
    let dir = tmp("add-next-steps");
    ok(&dir, &["init"]);
    // English, whatever language this computer's Allomos app shows.
    ok(&dir, &["task-language", "en"]);
    ok(&dir, &["add", "area", "A01", "Core"]);

    // No iteration yet: said, with how to add one.
    let out = alm(
        &dir,
        &["add", "task", "A01-01", "--area", "A01", "--title", "One"],
    );
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    let after = text.split("Save only these files").nth(1).unwrap_or("");
    assert!(
        after.contains("Next, for the agent who added it:"),
        "{text}"
    );
    assert!(after.contains("`alm plan A01-01 <iteration>`"), "{text}");
    assert!(after.contains("There is no iteration yet"), "{text}");
    assert!(
        after.contains("Nothing: make it ready, `alm ready A01-01`"),
        "{text}"
    );
    assert!(
        after.contains("name each open question with your proposal in the description's Notes"),
        "{text}"
    );

    // With an iteration, the first planned one is active (Task-A01-root343): the task goes into
    // it, in backlog; in JSON too.
    ok(
        &dir,
        &["add", "iteration", "I01", "First steps", "--release", "0.1"],
    );
    let v = ok(
        &dir,
        &["add", "task", "A01-02", "--area", "A01", "--title", "Two"],
    );
    let steps = v["next_steps"].as_str().unwrap();
    assert!(steps.contains("It went into iteration I01"), "{steps}");
    assert!(steps.contains("`alm ready A01-02`"), "{steps}");
    assert_eq!(v["task"]["state"], "backlog");
    assert_eq!(v["task"]["iteration"], "I01");

    // In the task language.
    ok(&dir, &["task-language", "zh-CN"]);
    let out = alm(
        &dir,
        &["add", "task", "A01-03", "--area", "A01", "--title", "Three"],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("接下来（给添加它的智能体）"), "{text}");
    assert!(text.contains("`alm plan A01-03 <迭代>`"), "{text}");
    ok(&dir, &["task-language", "ja"]);
    let out = alm(
        &dir,
        &["add", "task", "A01-04", "--area", "A01", "--title", "Four"],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("次に（追加したエージェントへ）"), "{text}");
}

/// A save from an out-of-date copy is refused (Task-A01-57): `alm show` gives the version; a
/// second writer that passes the old one with `--expected-version` changes nothing and gets the
/// task as it is now; without the option, `describe` and `title` work as before.
#[test]
fn describe_and_title_refuse_an_out_of_date_version() {
    let repo = filled();
    let read = ok(&repo, &["show", "A01-01"])["task"]["version"]
        .as_str()
        .unwrap()
        .to_string();
    let text = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(text.contains(&format!("version: {read}\n")), "{text}");
    // The first writer saves with the version it read.
    ok(
        &repo,
        &[
            "--as",
            "s/one",
            "describe",
            "A01-01",
            "--text",
            "First",
            "--expected-version",
            &read,
        ],
    );
    let before = snapshot(&repo);
    // The second writer still holds the old version: refused, nothing written.
    let v = refused(
        &repo,
        &[
            "--as",
            "s/two",
            "describe",
            "A01-01",
            "--text",
            "Second",
            "--expected-version",
            &read,
        ],
        1,
        "task_changed",
    );
    assert_eq!(v["current"]["description"], "First\n");
    let now = v["current"]["version"].as_str().unwrap().to_string();
    assert_ne!(now, read);
    refused(
        &repo,
        &[
            "--as",
            "s/two",
            "title",
            "A01-01",
            "New",
            "--expected-version",
            &read,
        ],
        1,
        "task_changed",
    );
    assert_eq!(snapshot(&repo), before, "a refused save writes nothing");
    let out = alm(
        &repo,
        &[
            "--as",
            "s/two",
            "describe",
            "A01-01",
            "--text",
            "Second",
            "--expected-version",
            &read,
        ],
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("error [task_changed]")
            && err.contains("Now:\n  title:")
            && err.contains("    First"),
        "{err}"
    );
    // Read again, the save goes through; without a version, as before.
    ok(
        &repo,
        &[
            "--as",
            "s/two",
            "describe",
            "A01-01",
            "--text",
            "Second",
            "--expected-version",
            &now,
        ],
    );
    ok(&repo, &["--as", "s/two", "title", "A01-01", "Newer"]);
    assert_eq!(ok(&repo, &["show", "A01-01"])["task"]["title"], "Newer");
}

/// A one-time number makes a repeated command safe (Task-A01-58): the same command with the
/// same `--request-id` returns the first result and changes nothing; the same number with a
/// different command is refused; without a number, commands work as before.
#[test]
fn a_repeated_command_with_the_same_number_runs_once() {
    let repo = filled();
    let add = [
        "--as",
        "s/one",
        "--request-id",
        "n-1",
        "add",
        "task",
        "--area",
        "A01",
        "--title",
        "Sign-in page",
        "--branch",
        "main",
    ];
    let first = ok(&repo, &add);
    let id = first["task"]["id"].as_str().unwrap().to_string();
    let before = snapshot(&repo);
    // The crash: the agent runs it again. Nothing new; the first result comes back.
    let again = ok(&repo, &add);
    assert_eq!(again["repeated"], true);
    assert_eq!(again["first"][0]["id"], id.as_str());
    assert_eq!(again["first"][0]["command"], "add task");
    assert_eq!(snapshot(&repo), before, "a repeat writes nothing");
    let text = String::from_utf8(alm(&repo, &add).stdout).unwrap();
    assert!(
        text.contains("Done already under the one-time number n-1")
            && text.contains(&format!("add task {id}")),
        "{text}"
    );
    // --json or not is the same input.
    let plain: Vec<&str> = add.to_vec();
    assert_eq!(alm(&repo, &plain).status.code(), Some(0));
    assert_eq!(snapshot(&repo), before);
    // The same number for something else: refused, nothing done.
    refused(
        &repo,
        &[
            "--as",
            "s/one",
            "--request-id",
            "n-1",
            "note",
            "A01-01",
            "Different",
        ],
        1,
        "request_reused",
    );
    assert_eq!(snapshot(&repo), before);
    // A note under a new number, repeated: one note event.
    let note = [
        "--as",
        "s/one",
        "--request-id",
        "n-2",
        "note",
        &id,
        "Read the spec",
    ];
    ok(&repo, &note);
    let after_note = snapshot(&repo);
    assert_eq!(ok(&repo, &note)["repeated"], true);
    assert_eq!(snapshot(&repo), after_note);
    // Without a number, the same command adds a second task, as always.
    let plain_add = [
        "--as",
        "s/one",
        "add",
        "task",
        "--area",
        "A01",
        "--title",
        "Sign-in page",
        "--branch",
        "main",
    ];
    let second = ok(&repo, &plain_add);
    assert_ne!(second["task"]["id"], id.as_str());
    // The number is kept in the history only, and is not shown as a change of the task.
    let h = ok(&repo, &["history", &id]);
    let shown = h.to_string();
    assert!(!shown.contains("request_id"), "{shown}");
}

/// `alm` at a given time (`ALLOMOS_NOW`), for the packing of old history (Task-A01-59).
fn alm_at(dir: &Path, now: &str, args: &[&str]) -> Output {
    alm_command()
        .current_dir(dir)
        .args(args)
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env_remove("ALLOMOS_LOCK_WAIT_MS")
        .env("ALLOMOS_NOW", now)
        .output()
        .unwrap()
}

/// From the 28th, any command packs the history of earlier months (Task-A01-59): it says so and
/// prints the Git commands for the packed file and the removed single files; the history reads
/// the same afterwards.
#[test]
fn any_command_packs_earlier_months_from_the_28th() {
    let dir = tmp("pack");
    let sept = "2026-09-10T09:00:00Z";
    for args in [
        vec!["init"],
        vec!["add", "area", "A01", "Core"],
        vec![
            "add", "task", "A01-01", "--area", "A01", "--title", "One", "--branch", "main",
        ],
        vec!["--as", "s/one", "note", "A01-01", "Next"],
    ] {
        assert_eq!(alm_at(&dir, sept, &args).status.code(), Some(0), "{args:?}");
    }
    let history = |now: &str| -> Value {
        let out = alm_at(&dir, now, &["--json", "history", "A01-01"]);
        let v: Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
        v["events"].clone()
    };
    let before = history("2026-10-27T09:00:00Z");
    assert!(
        !dir.join(".allomos/taskbag/events/2026-09.events").exists(),
        "not before the 28th"
    );

    // October 28th: a reading command packs September and says how to share it.
    let out = alm_at(&dir, "2026-10-28T09:00:00Z", &["list"]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("Packed the history of 2026-09 into events/2026-09.events"),
        "{text}"
    );
    assert!(text.contains("alm: pack history"), "{text}");
    assert!(text.contains("events/2026-09.events"), "{text}");
    assert!(
        text.contains("events/2026-09-10-"),
        "the removed files are shared too: {text}"
    );
    assert!(dir.join(".allomos/taskbag/events/2026-09.events").exists());
    assert_eq!(history("2026-10-28T09:00:00Z"), before);

    // Nothing more to pack: nothing said, nothing to share.
    let out = alm_at(&dir, "2026-10-29T09:00:00Z", &["--json", "list"]);
    let v: Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
    assert_eq!(v["packed"], Value::Null);
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

/// Approver, 2026-10-16 (Task-A01-45): one Update the taskbag…. On a current taskbag its request
/// is the move-cards one (card files kept outside the taskbag), and it changes nothing.
#[test]
fn move_in_on_a_current_taskbag_asks_for_the_card_files() {
    let repo = filled();
    let before = snapshot(&repo);
    let v = ok(&repo, &["move-in", "--language", "en"]);
    assert_eq!(v["move_in"], json!({ "needed": false }));
    assert_eq!(v["cards_outside"], json!([]));
    let request = v["request"].as_str().unwrap();
    assert!(
        request.starts_with("Please move this project's card files into Allomos's taskbag"),
        "{request}"
    );
    assert!(request.contains("guide move-cards"), "{request}");
    assert_eq!(snapshot(&repo), before);
    // Card files an agent reported are listed with it.
    fs::create_dir_all(repo.join("docs/tasks")).unwrap();
    fs::write(repo.join("docs/tasks/A01-01.md"), "# A01-01\n").unwrap();
    ok(&repo, &["cards", "found", "A01-01=docs/tasks/A01-01.md"]);
    let v = ok(&repo, &["move-in", "--language", "ja"]);
    assert_eq!(v["cards_outside"][0]["task"], "A01-01");
    assert_eq!(v["cards_outside"][0]["path"], "docs/tasks/A01-01.md");
    assert!(
        v["request"]
            .as_str()
            .unwrap()
            .contains("カードの移し入れの依頼")
    );
}

/// Owner, 2026-10-15 (Task-A01-45): a taskbag from any older Allomos is moved in by an agent,
/// like another task system. `alm` reads and changes nothing in it, says so on every command, and
/// `alm move-in` prints the request.
#[test]
fn every_older_taskbag_is_to_be_moved_in() {
    for n in 1..=4 {
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
            &["labels"],
            &["rename", "label", "locally verified by agent", "built"],
            &["handoff", "A01-01", "--as", "a", "--text", "n"],
            &["handoff-rules"],
        ] {
            let v = refused(&repo, args, 1, "taskbag_to_move_in");
            assert_eq!(v["error"]["params"]["found"], format!("allomos/{n}"));
            assert_eq!(v["error"]["params"]["current"], "allomos/8");
            let message = v["error"]["message"].as_str().unwrap();
            assert!(message.contains("Update the taskbag…"), "{message}");
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

/// After the app's Update the taskbag… on an older taskbag: the new taskbag works, and `alm move-in` prints the
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
    let v = ok(&repo, &["move-in", "--language", "en"]);
    assert_eq!(v["move_in"], json!({ "needed": false }));
    assert!(
        v["request"]
            .as_str()
            .unwrap()
            .starts_with("Please move this project's card files")
    );
}

#[test]
fn rename_label_renames_it_on_the_ladder_and_the_tasks() {
    // Decision 0020: a project's own label is renamed in the table and on its tasks; a built-in
    // label is not renamed.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(&repo, &["--as", "a", "label", "A01-01", "connected"]);
    let v = ok(
        &repo,
        &["--as", "a", "rename", "label", "connected", "wired"],
    );
    let names: Vec<&str> = v["labels"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l.as_str().or(l["name"].as_str()).unwrap())
        .collect();
    assert!(
        names.contains(&"wired") && !names.contains(&"connected"),
        "{v}"
    );
    assert_eq!(v["tasks"][0]["id"], "A01-01");
    assert_eq!(v["tasks"][0]["labels"], json!(["wired"]));
    assert_eq!(v["issues"], json!([]));
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["task"]["labels"], json!(["wired"]));
    assert_eq!(
        ok(&repo, &["history", "A01-01"])["events"][0]["command"],
        "rename label"
    );
    refused(
        &repo,
        &["rename", "label", "shipped", "x"],
        1,
        "label_not_in_table",
    );
    refused(
        &repo,
        &["rename", "label", "wired", "locally verified by agent"],
        1,
        "label_exists",
    );
    refused(
        &repo,
        &["rename", "label", "locally verified by agent", "built"],
        1,
        "label_built_in",
    );
    refused(
        &repo,
        &["rename", "label", "wired", " "],
        1,
        "label_name_missing",
    );
    ok(&repo, &["labels", "add", "spare"]);
    let out = alm(&repo, &["rename", "label", "spare", "extra"]);
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).contains("No task carried the label."));
}

/// Task-A03-18: `alm done --without-human-check` with its steps, `alm human` (To check, To
/// answer), `alm checked`, and the old `--without-owner` flag kept as an alias.
#[test]
fn a_task_finished_without_the_human_check_is_listed_checked_and_cleared() {
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(
        &repo,
        &["--as", "a", "label", "A01-01", "locally verified by agent"],
    );
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
    assert_eq!(
        file["labels"],
        json!(["locally verified by agent"]),
        "the label stays"
    );

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
    ok(
        &repo,
        &["--as", "b", "label", "A01-02", "locally verified by human"],
    );
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
    // The agent's label stays; the by-human label is added (decision 0020).
    assert_eq!(
        v["task"]["labels"],
        json!(["locally verified by agent", "locally verified by human"])
    );
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
    // Beside the label: who checked, against which steps, the result and the words
    // (Task-A01-table778, decision 0021 § 1.1).
    let check = |history: &Value| {
        history["events"][0]["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["field"] == "human_check")
            .unwrap()["to"]
            .clone()
    };
    let hc = check(&history);
    assert_eq!(hc["result"], "pass");
    assert_eq!(hc["where"], "local");
    assert!(hc["words"].is_null());
    let steps_were = note["from"].clone();
    assert!(steps_were.is_string());
    ok(
        &repo,
        &[
            "--as",
            "will",
            "checked",
            "A01-01",
            "--result",
            "partly",
            "--words",
            "Works; the button is small",
            "--checker",
            "p-abc123",
            "--deployed",
        ],
    );
    let hc = check(&ok(&repo, &["history", "A01-01"]));
    assert_eq!(hc["checker"], "p-abc123");
    assert_eq!(hc["result"], "partly");
    assert_eq!(hc["words"], "Works; the button is small");
    assert_eq!(hc["where"], "deployed");
    let out = alm(&repo, &["checked", "A01-01", "--result", "fail"]);
    assert!(!out.status.success());
    assert_eq!(ids(&ok(&repo, &["human"])["to_check"]), ["A02-01"]);
    // The top label set on a finished task is the human check too.
    ok(
        &repo,
        &["--as", "c", "label", "A02-01", "locally verified by human"],
    );
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
    // The first one adds the label "AI decisions to review" to the task (decision 0020).
    assert_eq!(v["changed_files"].as_array().unwrap().len(), 2, "{v}");
    assert!(
        v["changed_files"][0]
            .as_str()
            .unwrap()
            .starts_with("events/")
    );
    assert_eq!(v["changed_files"][1], "tasks/A01-01/task.json");
    let before = task_file(&repo, "A01-01");
    assert_eq!(before["labels"], json!(["AI decisions to review"]));
    // Anyone may record one, on any task: once the label is there, the task stays as it is.
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
    ok(
        &repo,
        &["--as", "a", "label", "A01-01", "locally verified by human"],
    );
    ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
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
    // Both decisions, and the label "AI decisions to review" taken off (decision 0020).
    let changes = last["changes"].as_array().unwrap();
    assert_eq!(changes.len(), 3, "{last}");
    let decisions: Vec<&Value> = changes
        .iter()
        .filter(|c| c["field"] == "ai_decision")
        .collect();
    assert_eq!(decisions.len(), 2);
    assert!(decisions[0]["to"].is_null());
    assert_eq!(decisions[1]["from"], "Layout: one file per command");
    assert!(changes.iter().any(|c| c["field"] == "labels"));
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

/// Whoever takes a task over is told, where its handoff note is shown, to check the real state
/// first and to repeat nothing already done (Task-A03-23).
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

    ok(
        &repo,
        &[
            "handoff",
            "A01-01",
            "--as",
            "claude/run",
            "--text",
            "stopped at tests",
        ],
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

/// Task-A03-polar770: `alm handoff` writes the handoff into the task: its note, its evidence and
/// its history; no handoff file is made. The next session finds it with `alm show` and `alm next`.
#[test]
fn a_handoff_is_written_into_the_task_that_the_next_one_finds() {
    let repo = filled();
    ok(&repo, &["--as", "claude/run", "claim", "A01-01"]);
    refused(
        &repo,
        &["handoff", "A01-01", "--text", "n"],
        1,
        "holder_missing",
    );
    refused(
        &repo,
        &["handoff", "A01-01", "--as", "claude/run", "--text", " "],
        1,
        "handoff_empty",
    );
    refused(
        &repo,
        &["handoff", "Z9-99", "--as", "claude/run", "--text", "n"],
        1,
        "task_not_found",
    );
    refused(
        &repo,
        &["handoff", "A01-01", "--as", "codex/other", "--text", "n"],
        1,
        "task_held",
    );

    let v = ok(
        &repo,
        &[
            "handoff",
            "A01-01",
            "--as",
            "claude/run",
            "--text",
            "Stopped at docs. Next: the guide.",
            "--evidence",
            "Deployed v1 to staging.",
        ],
    );
    assert_eq!(
        v["task"]["handoff_note"],
        "Stopped at docs. Next: the guide."
    );
    let named: Vec<&str> = v["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(
        named.iter().all(|f| !f.starts_with("handoffs/")),
        "{named:?}"
    );
    assert!(!repo.join(".allomos/taskbag/handoffs").exists());
    let v = ok(&repo, &["show", "A01-01"]);
    let description = v["task"]["description"].as_str().unwrap();
    assert!(description.contains("## Evidence\n\n- "), "{description}");
    assert!(
        description.ends_with(", claude/run: Deployed v1 to staging.\n"),
        "{description}"
    );
    let text = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(
        text.contains("handoff note: Stopped at docs. Next: the guide."),
        "{text}"
    );

    // From a file, and again: the note is replaced, not kept twice; the history says who.
    fs::write(repo.join("draft.md"), "Stopped at the guide.\n").unwrap();
    ok(
        &repo,
        &[
            "handoff",
            "A01-01",
            "--as",
            "claude/run",
            "--from",
            "draft.md",
        ],
    );
    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["task"]["handoff_note"], "Stopped at the guide.");
    let events = ok(&repo, &["history", "A01-01"])["events"].clone();
    let handoffs: Vec<&Value> = events
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["command"] == "handoff")
        .collect();
    assert_eq!(handoffs.len(), 2, "{events}");
    assert!(
        handoffs.iter().all(|e| e["by"] == "claude/run"
            && e["changes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["field"] == "handed_off_by" && c["to"] == "claude/run")),
        "{events}"
    );

    // The old commands are gone.
    for gone in [&["handoffs"][..], &["handoff-template"]] {
        let out = alm(&repo, gone);
        assert_ne!(out.status.code(), Some(0), "{gone:?}");
    }
}

/// Task-A03-polar770: an older taskbag's `handoffs/` folder makes `alm next` say the taskbag needs
/// updating, naming the files; alm reads nothing in them.
#[test]
fn old_handoff_files_make_next_ask_for_an_update() {
    let repo = filled();
    assert!(ok(&repo, &["next"])["handoffs"].is_null());
    let dir = repo.join(".allomos/taskbag/handoffs");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(".keep"), "").unwrap();
    assert!(ok(&repo, &["next"])["handoffs"].is_null());
    fs::write(
        dir.join("2026-10-12-1130-claude-run.md"),
        "---\nwriter: claude/run\n---\nold\n",
    )
    .unwrap();
    let v = ok(&repo, &["next"]);
    assert_eq!(
        v["old_handoffs"],
        json!(["handoffs/2026-10-12-1130-claude-run.md"])
    );
    let line = v["handoffs"].as_str().unwrap();
    assert!(
        line.starts_with(
            "This taskbag needs updating: its `handoffs/` folder still holds 1 old handoff file(s)"
        ),
        "{line}"
    );
    assert!(
        line.contains("Update the taskbag…") && line.contains("`alm move-in`"),
        "{line}"
    );
    let text = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(text.contains(line), "{text}");
}

/// Task-A00-storm072: `alm list --by iteration` lists every iteration, empty ones too, each with
/// its place in the plan, as the app shows it; the place follows `alm move iteration` and stays
/// the same with `--iteration`.
#[test]
fn list_by_iteration_shows_every_iteration_with_its_place() {
    let repo = filled();
    for (id, name) in [("I01", "One"), ("I02", "Two"), ("I03", "Three")] {
        ok(&repo, &["add", "iteration", id, name]);
    }
    ok(&repo, &["plan", "A01-01", "I02"]);
    let places = |v: &Value| -> Vec<(String, u64, usize)> {
        v["iterations"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| {
                (
                    g["id"].as_str().unwrap().to_string(),
                    g["place"].as_u64().unwrap(),
                    g["tasks"].as_array().unwrap().len(),
                )
            })
            .collect()
    };
    let v = ok(&repo, &["list", "--by", "iteration"]);
    assert_eq!(
        places(&v),
        [
            ("I01".into(), 1, 0),
            ("I02".into(), 2, 1),
            ("I03".into(), 3, 0)
        ]
    );
    let text = String::from_utf8(alm(&repo, &["list", "--by", "iteration"]).stdout).unwrap();
    assert!(
        text.contains("1  I01  One  [active]\n  (none)\n2  I02  Two\n  A01-01  "),
        "{text}"
    );
    assert!(text.contains("3  I03  Three\n  (none)\n"), "{text}");
    // A filter that leaves an iteration with nothing still shows its heading.
    let v = ok(&repo, &["list", "--by", "iteration", "--state", "done"]);
    assert_eq!(v["iterations"].as_array().unwrap().len(), 3);

    // Moved: the places follow the new order.
    ok(&repo, &["move", "iteration", "I03", "--before", "I01"]);
    let v = ok(&repo, &["list", "--by", "iteration"]);
    assert_eq!(
        places(&v),
        [
            ("I03".into(), 1, 0),
            ("I01".into(), 2, 0),
            ("I02".into(), 3, 1)
        ]
    );
    // One iteration: still its place in the whole plan.
    let v = ok(&repo, &["list", "--by", "iteration", "--iteration", "I02"]);
    assert_eq!(places(&v), [("I02".into(), 3, 1)]);
    let text =
        String::from_utf8(alm(&repo, &["list", "--by", "iteration", "--iteration", "I02"]).stdout)
            .unwrap();
    assert!(text.starts_with("3  I02  Two\n"), "{text}");
}

/// Task-A01-lotus143: `alm backlog` sends an unheld ready, waiting or blocked task back to backlog;
/// a held or finished one is refused, saying what to do instead.
#[test]
fn backlog_sends_an_unheld_task_back() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    let v = ok(&repo, &["backlog", "A01-01"]);
    assert_eq!(v["task"]["state"], "backlog");
    let events = ok(&repo, &["history", "A01-01"])["events"].clone();
    assert!(
        events
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["command"] == "backlog"),
        "{events}"
    );
    // Already backlog, held, or done: refused.
    refused(&repo, &["backlog", "A01-01"], 1, "cannot_back_to_backlog");
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let v = refused(
        &repo,
        &["--as", "a", "backlog", "A01-01"],
        1,
        "cannot_back_to_backlog",
    );
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("alm release")
    );
    ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
    let v = refused(&repo, &["backlog", "A01-01"], 1, "cannot_back_to_backlog");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("alm reopen")
    );
}

/// Task-A03-totem655: `alm move-in --translate` asks to translate the taskbag's contents into the
/// request's language; without it the request says not to.
#[test]
fn move_in_asks_to_translate_only_when_told() {
    let repo = filled();
    let no = ok(&repo, &["move-in", "--language", "ja"]);
    assert!(
        no["request"]
            .as_str()
            .unwrap()
            .contains("翻訳しないでください"),
        "{no}"
    );
    let yes = ok(&repo, &["move-in", "--language", "ja", "--translate"]);
    let r = yes["request"].as_str().unwrap();
    assert!(
        r.contains("日本語 (`ja`)に翻訳してください") && r.contains("task-language ja"),
        "{r}"
    );
    // Any language (Task-A05-cape996), and the task language becomes it.
    let vi = ok(
        &repo,
        &[
            "move-in",
            "--language",
            "en",
            "--translate",
            "vi",
            "--name",
            "Tiếng Việt",
        ],
    );
    let r = vi["request"].as_str().unwrap();
    assert!(
        r.contains("into Tiếng Việt (`vi`)") && r.contains("task-language vi"),
        "{r}"
    );
    refused(
        &repo,
        &["move-in", "--translate", "not a code"],
        1,
        "invalid_language",
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
    // add key, init, two areas and three tasks so far.
    assert_eq!(event_count(&repo), 7);
    let changes: &[&[&str]] = &[
        &["add", "iteration", "I01", "First"],
        &["ready", "A01-01"],
        &["--as", "a", "claim", "A01-01"],
        &["--as", "a", "label", "A01-01", "locally verified by agent"],
        &["--as", "a", "needs", "A01-01", "a check"],
        &["plan", "A01-01", "I01"],
        &["--as", "a", "title", "A01-01", "Open a folder"],
        &["--as", "a", "wait", "A01-01", "--on", "the owner"],
        &["--as", "a", "release", "A01-01", "--note", "over to you"],
        &["--as", "b", "claim", "A01-01"],
        &["--as", "b", "label", "A01-01", "locally verified by human"],
        &["--as", "b", "done", "A01-01"],
        &["drop", "A01-02"],
        &["labels", "add", "shipped"],
        &["project", "--name", "Demo 2"],
        &["task-language", "ja"],
        &["handoff-rules", "--quota", "5"],
    ];
    for (i, args) in changes.iter().enumerate() {
        ok(&repo, args);
        assert_eq!(event_count(&repo), 8 + i, "after {args:?}");
    }
    let n = event_count(&repo);
    // Refused changes write nothing.
    refused(&repo, &["--as", "c", "claim", "A01-01"], 1, "task_closed");
    refused(
        &repo,
        &["--as", "b", "label", "A02-01", "--remove", "nope"],
        1,
        "label_not_on_task",
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
        &[
            "--as",
            "claude/x",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
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
        &[
            "--as",
            "claude/a",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
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

/// Task-A03-polar770: `alm done` no longer asks to check handoff files.
#[test]
fn done_asks_nothing_about_handoff_files() {
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
    assert!(v.get("handoff_check").is_none(), "{v}");
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
fn block_shows_blockers_and_next_unblocks_left_overs() {
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

    // The first blocker done: still blocked, and the output names nothing unblocked.
    let out = alm(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains("Unblocked"), "{text}");
    assert_eq!(task_file(&repo, "A01-02")["state"], "blocked");

    // The last blocker dropped: a dropped blocker counts as finished, so the same change moves
    // it on by itself (Task-A01-tuna668): ready, blockers cleared, a note, an `unblock` event.
    let v = ok(&repo, &["drop", "A02-01"]);
    assert_eq!(v["unblocked"][0]["id"], "A01-02");
    assert_eq!(v["unblocked"][0]["state"], "ready");
    assert!(
        v["changed_files"]
            .as_array()
            .unwrap()
            .contains(&json!("tasks/A01-02/task.json")),
        "{v}"
    );
    let file = task_file(&repo, "A01-02");
    assert_eq!(file["state"], "ready");
    assert!(file.get("blocked_by").is_none());
    assert!(
        file["handoff_note"]
            .as_str()
            .unwrap()
            .starts_with("Unblocked automatically after A01-01, A02-01 finished;"),
        "{file}"
    );
    let v = ok(&repo, &["history", "A01-02"]);
    let last = &v["events"][0];
    assert_eq!(last["command"], "unblock");
    assert!(
        last["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["field"] == "reason" && c["to"].as_str().unwrap().starts_with("automatic:")),
        "{last}"
    );
    let v = ok(&repo, &["next"]);
    assert_eq!(v["blocked"], json!([]), "nothing left over to tell");

    // Blocking on a dropped task is allowed, with a warning: it holds nothing back.
    let v = ok(&repo, &["block", "A01-02", "--by", "A02-01"]);
    assert_eq!(v["warnings"][0]["code"], "blocker_dropped");

    // A task left blocked with every blocker finished (as from before the rule): `alm next` lets
    // it go on first, lists it, says so and names the files to share (Task-A01-steam936).
    ok(&repo, &["block", "A01-02", "--by", "A01-01"]);
    let v = ok(&repo, &["--as", "s", "next"]);
    assert_eq!(v["unblocked"][0]["id"], "A01-02");
    assert_eq!(v["unblocked"][0]["state"], "ready");
    assert_eq!(v["unblocked"][0]["finished_blockers"], json!(["A01-01"]));
    assert_eq!(v["blocked"], json!([]));
    assert!(
        v["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == "A01-02"),
        "{v}"
    );
    assert!(
        v["changed_files"]
            .as_array()
            .unwrap()
            .contains(&json!("tasks/A01-02/task.json")),
        "{v}"
    );
    assert!(
        v["git_commands"][1]
            .as_str()
            .unwrap()
            .contains("alm: unblock A01-02"),
        "{v}"
    );
    let file = task_file(&repo, "A01-02");
    assert_eq!(file["state"], "ready");
    assert!(file.get("blocked_by").is_none());
    assert!(
        file["handoff_note"]
            .as_str()
            .unwrap()
            .starts_with("Unblocked automatically after A01-01 finished;"),
        "{file}"
    );
    let h = ok(&repo, &["history", "A01-02"]);
    assert_eq!(h["events"][0]["command"], "unblock");
    // Nothing more to let go on: `alm next` writes nothing and names no files.
    let before = fs::read_to_string(repo.join(".allomos/taskbag/tasks/A01-02/task.json")).unwrap();
    let events_before = fs::read_dir(repo.join(".allomos/taskbag/events"))
        .unwrap()
        .count();
    let next = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(!next.contains("Unblocked A01-02 ("), "{next}");
    assert!(!next.contains("Changed files"), "{next}");
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

    // Unblock moves it on by hand; only a blocked task.
    ok(&repo, &["block", "A01-02", "--by", "A01-01"]);
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
    assert_eq!(
        commands,
        [
            "unblock", "block", "unblock", "block", "block", "unblock", "block", "add task"
        ]
    );
}

/// Task-A01-tuna668: a held blocked task goes on in progress when its last blocker is done; a
/// task left blocked from before goes on at the next finishing change.
#[test]
fn the_last_blocker_finishing_unblocks_a_held_task_and_left_overs() {
    let repo = filled();
    ok(&repo, &["--as", "b", "claim", "A01-02"]);
    ok(&repo, &["--as", "b", "block", "A01-02", "--by", "A01-01"]);
    let v = ok(
        &repo,
        &["done", "A01-01", "--without-human-check", "test run"],
    );
    assert_eq!(v["unblocked"][0]["id"], "A01-02");
    assert_eq!(task_file(&repo, "A01-02")["state"], "in_progress");
    assert_eq!(task_file(&repo, "A01-02")["holder"], "b");
    let text =
        String::from_utf8(alm(&repo, &["--as", "b", "block", "A01-02", "--by", "A01-01"]).stdout)
            .unwrap();
    assert!(
        !text.contains("Unblocked"),
        "blocking moves nothing: {text}"
    );
    // Left over (its blocker finished before): the next done or drop lets it go on too.
    let out = alm(&repo, &["drop", "A02-01"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("Unblocked A01-02 (in progress): all its blockers have finished."),
        "{text}"
    );
    assert_eq!(task_file(&repo, "A01-02")["state"], "in_progress");
}

/// A taskbag in format 4 is newer than an allomos/3 build: the older build says so
/// (`taskbag_newer`). Shown with this build reading a format 5 taskbag, the same rule.
#[test]
fn a_newer_taskbag_is_refused_by_an_older_build() {
    let repo = filled();
    set_format(&repo, "allomos/9");
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
            "git worktree add <folder> -b claude/a01-01 storage-v2"
        ])
    );
    assert_eq!(v["branch_commands"]["merge"][2], "git merge claude/a01-01");
    assert_eq!(
        v["branch_commands"]["merge"][4],
        "git worktree remove <folder>"
    );
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

/// Card files beside their tasks (Task-A00-30): the agent reports the card files it found in the
/// project folder, `alm cards` and `alm next` show them, `alm card` moves one in and points the
/// description's link to it, and a finished task held by an old session can be changed too.
#[test]
fn card_files_are_reported_and_moved_in_beside_their_tasks() {
    let repo = filled();
    fs::create_dir_all(repo.join("docs/tasks")).unwrap();
    fs::write(repo.join("docs/tasks/A01-01.md"), "# A01-01\n\nCard.\n").unwrap();
    fs::write(repo.join("docs/tasks/A01-02.md"), "# A01-02\n\nCard.\n").unwrap();
    // Nothing linked or reported yet: no Card files line at all (Task-A03-34).
    let v = ok(&repo, &["next"]);
    assert_eq!(v["cards"], Value::Null);
    let text = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(!text.contains("Card files"), "{text}");
    assert_eq!(ok(&repo, &["cards"])["cards_outside"], json!([]));
    // A01-01 is finished by an old session (before the card files were reported: `alm claim`
    // refuses while some are outside the taskbag, Task-A03-27).
    ok(&repo, &["--as", "old/session", "claim", "A01-01"]);
    ok(
        &repo,
        &[
            "--as",
            "old/session",
            "done",
            "A01-01",
            "--without-human-check",
            "test run",
        ],
    );

    refused(&repo, &["cards", "found", "A01-01"], 2, "bad_usage");
    refused(
        &repo,
        &["cards", "found", "A01-01=docs/none.md"],
        1,
        "card_not_found",
    );
    refused(
        &repo,
        &["cards", "found", "A01-01=../x.md"],
        1,
        "card_path_invalid",
    );
    refused(
        &repo,
        &["cards", "found", "A09-09=docs/tasks/A01-01.md"],
        1,
        "task_not_found",
    );
    let v = ok(
        &repo,
        &[
            "--as",
            "claude/finder",
            "cards",
            "found",
            "A01-01=docs/tasks/A01-01.md",
            "A01-02=docs/tasks/A01-02.md",
        ],
    );
    assert_eq!(v["found"].as_array().unwrap().len(), 2);
    assert_eq!(v["cards_outside"].as_array().unwrap().len(), 2);
    assert!(
        v["changed_files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|f| f.as_str().unwrap().starts_with("events/"))
    );
    let v = ok(&repo, &["next"]);
    assert!(v["cards"].as_str().unwrap().contains("2 task(s)"));

    // A01-01 was finished by an old session before: still changeable.
    let v = ok(&repo, &["--as", "claude/mover", "card", "A01-01"]);
    assert_eq!(v["card"], "tasks/A01-01/card.md");
    let bag = repo.join(".allomos/taskbag");
    assert_eq!(
        fs::read_to_string(bag.join("tasks/A01-01/card.md")).unwrap(),
        "# A01-01\n\nCard.\n"
    );
    assert_eq!(
        fs::read_to_string(bag.join("tasks/A01-01/description.md")).unwrap(),
        "Card: [card.md](card.md)\n\nA person opens a folder.\n"
    );
    let files: Vec<&str> = v["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    assert!(files.contains(&"tasks/A01-01/card.md"), "{files:?}");
    assert!(files.contains(&"tasks/A01-01/description.md"), "{files:?}");
    // The original stays; alm says how to remove it.
    assert!(repo.join("docs/tasks/A01-01.md").is_file());
    let out = alm(
        &repo,
        &[
            "--as",
            "claude/mover",
            "card",
            "A01-02",
            "--from",
            "docs/tasks/A01-02.md",
        ],
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains("rm -- docs/tasks/A01-02.md"));
    assert_eq!(ok(&repo, &["cards"])["cards_outside"], json!([]));
    // A different card is not replaced by accident.
    fs::write(repo.join("new.md"), "New\n").unwrap();
    refused(
        &repo,
        &["card", "A01-02", "--from", "new.md"],
        1,
        "card_exists",
    );
    ok(&repo, &["card", "A01-02", "--from", "new.md", "--replace"]);
    refused(&repo, &["card", "A02-01"], 1, "card_not_reported");
    // An open task another session holds is still theirs.
    ok(&repo, &["--as", "other", "claim", "A02-01"]);
    refused(
        &repo,
        &["--as", "me", "card", "A02-01", "--from", "new.md"],
        1,
        "task_held",
    );
    // History names the moves; the taskbag has no problems.
    let h = ok(&repo, &["history", "A01-01"]);
    assert!(h.to_string().contains("card_outside"));
    assert_eq!(ok(&repo, &["list"])["issues"], json!([]));
    fs::remove_dir_all(repo).unwrap();
}

/// Tasks whose `Card:` link points outside their folder (Task-A01-51): `alm cards`, `alm next`
/// and `alm list --json` list them without any report, a report wins without counting the task
/// twice, `alm card` moves one in from its link, and reading the links changes nothing.
#[test]
fn card_links_outside_the_taskbag_are_listed_without_a_report() {
    let repo = filled();
    fs::create_dir_all(repo.join("docs/tasks")).unwrap();
    fs::write(repo.join("docs/tasks/A01-01.md"), "# A01-01\n\nCard.\n").unwrap();
    fs::write(repo.join("docs/tasks/A01-02.md"), "# A01-02\n\nCard.\n").unwrap();
    ok(
        &repo,
        &[
            "describe",
            "A01-01",
            "--text",
            "Card: [docs/tasks/A01-01.md](../../../../docs/tasks/A01-01.md)\n\nMore.",
        ],
    );
    ok(
        &repo,
        &[
            "describe",
            "A01-02",
            "--text",
            "**Notes.** Card: docs/tasks/A01-02.md.",
        ],
    );
    let bag = repo.join(".allomos/taskbag");
    let stamp = |dir: &Path| {
        let mut all = Vec::new();
        for e in fs::read_dir(dir.join("events")).unwrap() {
            all.push(e.unwrap().path());
        }
        all.len()
    };
    let events = stamp(&bag);
    let v = ok(&repo, &["cards"]);
    let listed: Vec<(&str, &str, &str)> = v["cards_outside"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["task"].as_str().unwrap(),
                c["path"].as_str().unwrap(),
                c["source"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        listed,
        [
            ("A01-01", "docs/tasks/A01-01.md", "link"),
            ("A01-02", "docs/tasks/A01-02.md", "link")
        ]
    );
    let text = String::from_utf8_lossy(&alm(&repo, &["cards"]).stdout).to_string();
    assert!(
        text.contains("2 task(s)") && text.contains("its Card: link"),
        "{text}"
    );
    assert_eq!(ok(&repo, &["list"])["cards_outside"], v["cards_outside"]);
    assert_eq!(
        ok(&repo, &["list", "--by", "iteration"])["cards_outside"],
        v["cards_outside"]
    );
    assert!(
        ok(&repo, &["next"])["cards"]
            .as_str()
            .unwrap()
            .contains("2 task(s)")
    );
    // Listing wrote nothing.
    assert_eq!(stamp(&bag), events);
    // A report wins and the task still counts once.
    ok(&repo, &["cards", "found", "A01-02=docs/tasks/A01-01.md"]);
    let v = ok(&repo, &["cards"]);
    assert_eq!(v["cards_outside"].as_array().unwrap().len(), 2);
    assert_eq!(v["cards_outside"][1]["source"], "report");
    // Moved in from the link alone.
    let out = alm(&repo, &["card", "A01-01"]);
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("rm -- "));
    assert_eq!(
        fs::read_to_string(bag.join("tasks/A01-01/card.md")).unwrap(),
        "# A01-01\n\nCard.\n"
    );
    let v = ok(&repo, &["cards"]);
    assert_eq!(v["cards_outside"].as_array().unwrap().len(), 1);
    assert_eq!(ok(&repo, &["list"])["issues"], json!([]));
    fs::remove_dir_all(repo).unwrap();
}

/// The newest event file's JSON.
fn newest_event(repo: &Path) -> Value {
    let dir = repo.join(".allomos/taskbag/events");
    let mut names: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    names.sort();
    let text = fs::read_to_string(dir.join(names.last().unwrap())).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// Decision 0021 § 1.1 (Task-A01-black004): every new event says who ran it (an agent's session
/// and model, or a person's pseudonym), whom an agent works for, and the approval it relies on;
/// what `alm` could not learn is listed, and events recorded before still read.
#[test]
fn events_record_who_ran_them_for_whom_and_under_which_approval() {
    let repo = filled();
    // An event recorded before, without the new fields, among them.
    fs::write(
        repo.join(".allomos/taskbag/events/2026-01-02-030405-000-old001.json"),
        r#"{"at":"2026-01-02T03:04:05Z","by":"old","via":"alm","command":"note","kind":"task","id":"A01-01","changes":[{"field":"handoff_note","to":"before"}]}"#,
    )
    .unwrap();
    let settings = repo.join("settings-of-this-computer");
    let run = |vars: &[(&str, &str)], args: &[&str]| {
        let mut c = alm_command();
        c.current_dir(&repo).arg("--json").args(args);
        for name in [
            "ALLOMOS_TASKBAG",
            "ALLOMOS_HOLDER",
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_SESSION_ID",
            "CODEX_THREAD_ID",
            "CODEX_SANDBOX",
            "CODEX_SANDBOX_NETWORK_DISABLED",
            "ALLOMOS_MODEL",
            "ALLOMOS_FOR",
            "ALLOMOS_APPROVAL",
            "CLAUDE_EFFORT",
            "ALLOMOS_EFFORT",
            "ALLOMOS_PERMISSION_MODE",
            "ALLOMOS_RULES",
        ] {
            c.env_remove(name);
        }
        c.env("ALLOMOS_SETTINGS_DIR", &settings);
        for (k, v) in vars {
            c.env(k, v);
        }
        let out = c.output().unwrap();
        assert!(out.status.success(), "{args:?}: {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (v, newest_event(&repo))
    };
    let claude = [
        ("CLAUDECODE", "1"),
        ("CLAUDE_CODE_ENTRYPOINT", "cli"),
        ("CLAUDE_CODE_SESSION_ID", "sess-1"),
    ];
    // An agent without --model: the session and whom it works for are recorded; the model is
    // listed as missing.
    let (v, e) = run(&claude, &["--as", "s1", "claim", "A01-01"]);
    assert_eq!(e["actor"]["session"], "sess-1", "{e}");
    assert_eq!(e["actor"]["model"], "unknown", "{e}");
    let person = e["for"].as_str().unwrap().to_string();
    assert!(person.starts_with("p-") && person.len() == 8, "{e}");
    let codes: Vec<&Value> = v["event_missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| &m["code"])
        .collect();
    assert_eq!(
        codes,
        [
            &json!("model"),
            &json!("effort"),
            &json!("permission_mode"),
            &json!("rules")
        ],
        "{v}"
    );
    // With --model and --approval: both recorded, the model no longer missing.
    let (v, e) = run(
        &claude,
        &[
            "--as",
            "s1",
            "--model",
            "claude-opus-5-5",
            "--approval",
            "ap-1",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
    );
    assert_eq!(
        (&e["actor"]["session"], &e["actor"]["model"]),
        (&json!("sess-1"), &json!("claude-opus-5-5")),
        "{e}"
    );
    assert_eq!(
        (&e["for"], &e["approval"]),
        (&json!(person), &json!("ap-1")),
        "{e}"
    );
    assert!(!v["event_missing"].to_string().contains("\"model\""), "{v}");
    // The same session again without --model: this computer remembers its model.
    let (_, e) = run(&claude, &["--as", "s1", "note", "A01-01", "half way"]);
    assert_eq!(e["actor"]["model"], "claude-opus-5-5", "{e}");
    assert!(
        e.get("approval").is_none(),
        "an approval is for one command: {e}"
    );
    // --for names another person.
    let (_, e) = run(
        &claude,
        &["--as", "s1", "--for", "p-other1", "note", "A01-01", "x"],
    );
    assert_eq!(e["for"], "p-other1", "{e}");
    // A person at this computer (no agent seen): their pseudonym, the same one, as the actor.
    let (v, e) = run(&[], &["note", "A01-02", "by hand"]);
    assert_eq!(e["actor"], json!({ "person": person }), "{e}");
    assert!(
        e.get("for").is_none() && v.get("event_missing").is_none(),
        "{e} {v}"
    );
    // Codex: its thread ID is the session.
    let (_, e) = run(
        &[("CODEX_THREAD_ID", "t-9")],
        &["--as", "s2", "claim", "A01-02"],
    );
    assert_eq!(e["actor"]["session"], "t-9", "{e}");
    // The text says what is missing and how to give it.
    let mut c = alm_command();
    c.current_dir(&repo).args(["--as", "s3", "claim", "A02-01"]);
    c.env("ALLOMOS_SETTINGS_DIR", &settings)
        .env("CLAUDECODE", "1")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("ALLOMOS_MODEL");
    let text = String::from_utf8(c.output().unwrap().stdout).unwrap();
    assert!(
        text.contains("which model ran it") && text.contains("alm --model"),
        "{text}"
    );
    assert!(text.contains("which agent session ran it"), "{text}");
    // Old and new events read together without issues.
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    let v = ok(&repo, &["history", "A01-01"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    assert!(
        v.to_string().contains("before"),
        "the old event still shows: {v}"
    );
}

/// Decision 0021 § 1.1, AI details (Task-A01-urchin867): an agent's event records its model,
/// effort, permission mode, the AGENTS.md commit it works under and alm's version; what the agent
/// did not say is "unknown", never guessed; a session names each once on this computer.
#[test]
fn events_record_the_ai_details() {
    let repo = filled();
    let settings = repo.join("settings-of-this-computer");
    let run = |vars: &[(&str, &str)], args: &[&str]| {
        let mut c = alm_command();
        c.current_dir(&repo).arg("--json").args(args);
        for name in [
            "ALLOMOS_TASKBAG",
            "ALLOMOS_HOLDER",
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "CLAUDE_CODE_SESSION_ID",
            "CLAUDE_EFFORT",
            "CODEX_THREAD_ID",
            "CODEX_SANDBOX",
            "CODEX_SANDBOX_NETWORK_DISABLED",
            "ALLOMOS_MODEL",
            "ALLOMOS_EFFORT",
            "ALLOMOS_PERMISSION_MODE",
            "ALLOMOS_RULES",
        ] {
            c.env_remove(name);
        }
        c.env("ALLOMOS_SETTINGS_DIR", &settings);
        for (k, v) in vars {
            c.env(k, v);
        }
        let out = c.output().unwrap();
        assert!(out.status.success(), "{args:?}: {out:?}");
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (v, newest_event(&repo))
    };
    let codes = |v: &Value| -> Vec<String> {
        v["event_missing"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|m| m["code"].as_str().unwrap().to_string())
                    .collect()
            })
            .unwrap_or_default()
    };
    let version = env!("CARGO_PKG_VERSION");
    // Codex, saying nothing: every AI detail is "unknown", and listed.
    let (v, e) = run(
        &[("CODEX_THREAD_ID", "t-1")],
        &["--as", "c1", "claim", "A01-01"],
    );
    assert_eq!(
        e["actor"],
        json!({
            "session": "t-1",
            "model": "unknown",
            "effort": "unknown",
            "permission_mode": "unknown",
            "rules": "unknown",
            "alm": version,
        }),
        "{e}"
    );
    assert_eq!(
        codes(&v),
        ["model", "effort", "permission_mode", "rules"],
        "{v}"
    );
    // Claude Code gives its effort by itself; the agent names the rest once.
    let claude = [
        ("CLAUDECODE", "1"),
        ("CLAUDE_CODE_ENTRYPOINT", "cli"),
        ("CLAUDE_CODE_SESSION_ID", "sess-9"),
        ("CLAUDE_EFFORT", "medium"),
    ];
    let (v, e) = run(
        &claude,
        &[
            "--as",
            "s9",
            "--model",
            "claude-opus-5-5",
            "--permission-mode",
            "auto",
            "--rules",
            "ea4c3b25",
            "claim",
            "A01-02",
        ],
    );
    assert_eq!(
        e["actor"],
        json!({
            "session": "sess-9",
            "model": "claude-opus-5-5",
            "effort": "medium",
            "permission_mode": "auto",
            "rules": "ea4c3b25",
            "alm": version,
        }),
        "{e}"
    );
    assert!(codes(&v).is_empty(), "{v}");
    // The same session later: remembered; a new effort from the environment wins.
    let mut later = claude;
    later[3] = ("CLAUDE_EFFORT", "high");
    let (v, e) = run(&later, &["--as", "s9", "note", "A01-02", "half way"]);
    assert_eq!(
        (
            &e["actor"]["model"],
            &e["actor"]["effort"],
            &e["actor"]["permission_mode"],
            &e["actor"]["rules"]
        ),
        (
            &json!("claude-opus-5-5"),
            &json!("high"),
            &json!("auto"),
            &json!("ea4c3b25")
        ),
        "{e}"
    );
    assert!(codes(&v).is_empty(), "{v}");
    // Naming one detail anew changes only that one.
    let (_, e) = run(
        &claude,
        &[
            "--as",
            "s9",
            "--rules",
            "1baa4f64",
            "note",
            "A01-02",
            "rules moved",
        ],
    );
    assert_eq!(
        (&e["actor"]["rules"], &e["actor"]["permission_mode"]),
        (&json!("1baa4f64"), &json!("auto")),
        "{e}"
    );
    // The text names what is unknown and how to give it.
    let mut c = alm_command();
    c.current_dir(&repo)
        .args(["--as", "c1", "note", "A01-01", "x"]);
    c.env("ALLOMOS_SETTINGS_DIR", &settings)
        .env("CODEX_THREAD_ID", "t-2")
        .env_remove("CLAUDECODE")
        .env_remove("ALLOMOS_PERMISSION_MODE")
        .env_remove("ALLOMOS_RULES");
    let text = String::from_utf8(c.output().unwrap().stdout).unwrap();
    assert!(
        text.contains("alm --permission-mode")
            && text.contains("git log -1 --format=%H -- AGENTS.md"),
        "{text}"
    );
    let v = ok(&repo, &["history", "A01-02"]);
    assert_eq!(v["issues"], json!([]), "{v}");
}

/// Approver, 2026-10-18 (Task-A03-27): every event `alm` writes names the agent it runs under,
/// judged from the environment; unsure, it names none.
#[test]
fn events_record_the_agent_alm_runs_under() {
    let repo = filled();
    // No agent's signs (the helper removes them): nothing recorded.
    let e = newest_event(&repo);
    assert!(e.get("agent").is_none() && e.get("tool").is_none(), "{e}");
    let run = |vars: &[(&str, &str)], args: &[&str]| {
        let mut c = alm_command();
        c.current_dir(&repo).args(args);
        for name in [
            "ALLOMOS_TASKBAG",
            "ALLOMOS_HOLDER",
            "CLAUDECODE",
            "CLAUDE_CODE_ENTRYPOINT",
            "CODEX_THREAD_ID",
            "CODEX_SANDBOX",
            "CODEX_SANDBOX_NETWORK_DISABLED",
        ] {
            c.env_remove(name);
        }
        for (k, v) in vars {
            c.env(k, v);
        }
        assert!(c.output().unwrap().status.success(), "{args:?}");
        newest_event(&repo)
    };
    let e = run(
        &[("CLAUDECODE", "1"), ("CLAUDE_CODE_ENTRYPOINT", "cli")],
        &["--as", "s1", "claim", "A01-01"],
    );
    assert_eq!(
        (&e["agent"], &e["tool"]),
        (&json!("claude"), &json!("claude_cli")),
        "{e}"
    );
    let e = run(
        &[
            ("CLAUDECODE", "1"),
            ("CLAUDE_CODE_ENTRYPOINT", "claude-desktop"),
        ],
        &["--as", "s1", "label", "A01-01", "locally verified by agent"],
    );
    assert_eq!(
        (&e["agent"], &e["tool"]),
        (&json!("claude"), &json!("claude_app")),
        "{e}"
    );
    let e = run(
        &[("CODEX_THREAD_ID", "t1")],
        &["--as", "s2", "claim", "A01-02"],
    );
    assert_eq!(e["agent"], "codex", "{e}");
    assert!(e.get("tool").is_none(), "{e}");
    // Signs of both: unsure.
    let e = run(
        &[("CLAUDECODE", "1"), ("CODEX_THREAD_ID", "t1")],
        &["--as", "s3", "claim", "A02-01"],
    );
    assert!(e.get("agent").is_none(), "{e}");
    // The events still read without issues.
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
}

/// Approver, 2026-10-18 (Task-A03-27): while card files are kept outside the taskbag, `alm claim`
/// refuses and says to move them in first (`alm guide move-cards`); nothing changes.
#[test]
fn claim_waits_until_card_files_are_moved_in() {
    let repo = filled();
    fs::create_dir_all(repo.join("docs/tasks")).unwrap();
    fs::write(repo.join("docs/tasks/A01-01.md"), "# A01-01\n").unwrap();
    ok(&repo, &["cards", "found", "A01-01=docs/tasks/A01-01.md"]);
    let before = snapshot(&repo);
    // Any task, not only the one whose card file is outside.
    let v = refused(
        &repo,
        &["--as", "a", "claim", "A01-02"],
        1,
        "claim_cards_outside",
    );
    assert_eq!(v["error"]["params"]["count"], "1", "{v}");
    assert_eq!(v["error"]["params"]["tasks"], "A01-01", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("alm guide move-cards"),
        "{v}"
    );
    assert_eq!(snapshot(&repo), before);
    // Once the card is moved in, claiming works.
    ok(&repo, &["card", "A01-01"]);
    ok(&repo, &["--as", "a", "claim", "A01-02"]);
}

/// Approver, 2026-10-19 (Task-A01-55): a history record with fields this Allomos does not know
/// (written by a newer one) is read, not reported and dropped. Other files are read too, with a
/// reminder for each unknown field (Task-A01-piano403).
#[test]
fn history_records_with_unknown_fields_still_show() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    fs::write(
        bag.join("events/2099-01-01-000000-000-zzzzzz.json"),
        r#"{"at":"2099-01-01T00:00:00Z","by":"newer","via":"alm","command":"label","kind":"task",
"id":"A01-01","from_a_newer_allomos":true,"changes":[{"field":"label","to":"locally verified by agent","extra":1}]}"#,
    )
    .unwrap();
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    let v = ok(&repo, &["history", "A01-01"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    assert!(v.to_string().contains("newer"), "{v}");
    // A task file with an unknown field is read, with a reminder.
    let task = bag.join("tasks/A01-02/task.json");
    let mut t: Value = serde_json::from_str(&fs::read_to_string(&task).unwrap()).unwrap();
    t["colour"] = json!("red");
    fs::write(&task, t.to_string()).unwrap();
    let v = ok(&repo, &["list"]);
    assert!(v["issues"].to_string().contains("field_unknown"), "{v}");
}

/// `alm quota` (Task-A03-02): an agent reports its quota; it is kept on this computer beside the
/// app's settings, one report per agent, account and window, the newest first; no taskbag needed
/// and nothing to share.
#[test]
fn quota_reports_are_kept_on_this_computer() {
    let dir = tmp("quota");
    let settings = dir.join("settings");
    let run = |args: &[&str]| {
        let out = alm_command()
            .current_dir(&dir)
            .arg("--json")
            .args(args)
            .env_remove("ALLOMOS_TASKBAG")
            .env("ALLOMOS_SETTINGS_DIR", &settings)
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        (out.status.code().unwrap(), v)
    };
    let (code, v) = run(&["quota"]);
    assert_eq!((code, &v["reports"]), (0, &json!([])));
    let (code, _) = run(&[
        "quota",
        "--agent",
        "claude-code",
        "--left",
        "52",
        "--resets",
        "2026-10-20T17:00",
    ]);
    assert_eq!(code, 0);
    run(&[
        "quota",
        "--agent",
        "codex",
        "--account",
        "work",
        "--left",
        "3",
    ]);
    let (_, v) = run(&["quota", "--agent", "claude-code", "--left", "40"]);
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2, "{v}");
    assert_eq!(reports[0]["agent"], "claude-code");
    assert_eq!(reports[0]["left"], 40);
    assert_eq!(reports[0]["window"], "5h");
    assert_eq!(reports[0]["resets"], Value::Null);
    assert_eq!(reports[1]["account"], "work");
    assert!(v.get("changed_files").is_none(), "{v}");
    assert!(settings.join("quota.json").is_file());
    let (code, v) = run(&[
        "quota", "--agent", "codex", "--left", "5", "--window", "day",
    ]);
    assert_eq!((code, &v["error"]["code"]), (2, &json!("bad_usage")));
}

/// Task-A01-54: two branches each add a task to the same area without pulling; the merge has no
/// conflict in the taskbag, both tasks are there with different word IDs, and the numbered task
/// made before keeps its ID.
#[test]
fn tasks_added_on_two_branches_merge_without_a_clash() {
    let (repo, _) = real_repo();
    ok(&repo, &["init"]);
    ok(&repo, &["add", "area", "A01", "Tracking"]);
    ok(
        &repo,
        &["add", "task", "A01-53", "--area", "A01", "--title", "Old"],
    );
    git(&repo, &["add", "--", ".allomos"]);
    git(&repo, &["commit", "--quiet", "-m", "taskbag"]);
    let mut made = Vec::new();
    for (branch, title) in [("one", "From one"), ("two", "From two")] {
        git(&repo, &["switch", "--quiet", "-c", branch, "main"]);
        let v = ok(&repo, &["add", "task", "--area", "A01", "--title", title]);
        made.push(v["task"]["id"].as_str().unwrap().to_string());
        git(&repo, &["add", "--", ".allomos"]);
        git(&repo, &["commit", "--quiet", "-m", title]);
    }
    assert_ne!(made[0], made[1]);
    git(&repo, &["merge", "--quiet", "--no-edit", "one"]);
    let v = ok(&repo, &["list", "--area", "A01"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    let ids: Vec<&str> = v["areas"][0]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 3, "{v}");
    assert_eq!(ids[0], "A01-53");
    assert!(ids.contains(&made[0].as_str()) && ids.contains(&made[1].as_str()));
}

/// With no iteration marked active, the first planned one is active (approver, 2026-10-22;
/// Task-A01-root343): new tasks go into it, `alm list` and `alm next` say so, nothing is written,
/// and marking another one active is not refused. With every iteration closed, none is active.
#[test]
fn the_first_planned_iteration_is_active_when_none_is_marked() {
    let repo = filled();
    ok(&repo, &["task-language", "en"]);
    ok(&repo, &["add", "iteration", "I01", "One"]);
    ok(&repo, &["add", "iteration", "I02", "Two"]);
    ok(&repo, &["add", "iteration", "I03", "Three"]);
    ok(&repo, &["change", "iteration", "I01", "--state", "closed"]);
    let bag = repo.join(".allomos/taskbag");
    let file = |id: &str| fs::read_to_string(bag.join(format!("iterations/{id}.json"))).unwrap();

    // I01 is closed, so I02 is active: a new task goes into it, and nothing is written.
    let v = ok(&repo, &["add", "task", "--area", "A01", "--title", "First"]);
    let first = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"]["iteration"], "I02");
    assert!(!file("I02").contains("state"), "{}", file("I02"));
    let listed = ok(&repo, &["list", "--by", "iteration"]);
    let actives: Vec<&Value> = listed["iterations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["active"] == true)
        .collect();
    assert_eq!(actives.len(), 1, "{listed}");
    assert_eq!(actives[0]["id"], "I02");
    let out = alm(&repo, &["list", "--by", "iteration"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("I02  Two  [active]"), "{text}");
    ok(&repo, &["ready", &first]);
    assert_eq!(ok(&repo, &["next"])["active_iteration"], "I02");

    // All done: closing it is enough, the next planned one is then active.
    ok(&repo, &["drop", &first]);
    let out = alm(&repo, &["next"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("I03, the next planned iteration, is then active"),
        "{text}"
    );
    ok(&repo, &["change", "iteration", "I02", "--state", "closed"]);
    assert_eq!(ok(&repo, &["next"])["active_iteration"], "I03");

    // Marking another one active is not refused by the first planned one, and wins over it.
    ok(&repo, &["add", "iteration", "I04", "Four"]);
    ok(&repo, &["change", "iteration", "I04", "--state", "active"]);
    let v = ok(
        &repo,
        &["add", "task", "--area", "A01", "--title", "Second"],
    );
    assert_eq!(v["task"]["iteration"], "I04");

    // Every iteration closed: none is active, a new task goes into none.
    for id in ["I03", "I04"] {
        ok(&repo, &["change", "iteration", id, "--state", "closed"]);
    }
    let v = ok(&repo, &["add", "task", "--area", "A01", "--title", "Loose"]);
    assert_eq!(v["task"]["iteration"], Value::Null);
    assert!(
        v["next_steps"]
            .as_str()
            .unwrap()
            .contains("none is planned either"),
        "{v}"
    );
    assert_eq!(ok(&repo, &["list"])["issues"], json!([]));
}

/// Iterations have a state (approver, 2026-10-22; Task-A01-root343): planned → active → closed,
/// at most one active, set only by a command. A new task goes into the active iteration; `alm
/// next` lists its tasks first and says when it is all done; closing one lists the unfinished
/// tasks and moves none.
#[test]
fn iterations_have_a_state_and_the_active_one_leads() {
    let repo = filled();
    ok(&repo, &["task-language", "en"]);
    ok(&repo, &["add", "iteration", "I01", "One"]);
    ok(&repo, &["add", "iteration", "I02", "Two"]);
    let bag = repo.join(".allomos/taskbag");
    let file = |id: &str| fs::read_to_string(bag.join(format!("iterations/{id}.json"))).unwrap();
    // Planned is the default and is not written.
    assert!(!file("I01").contains("state"), "{}", file("I01"));

    // None marked active: the first planned one is (approver, 2026-10-22), so a new task goes
    // into I01.
    let v = ok(&repo, &["add", "task", "--area", "A01", "--title", "Loose"]);
    let loose = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"]["iteration"], "I01");

    // Make I01 active; a second active iteration is refused, a bad state too.
    let v = ok(&repo, &["change", "iteration", "I01", "--state", "active"]);
    assert_eq!(v["iteration"]["state"], "active");
    assert!(
        file("I01").contains("\"state\": \"active\""),
        "{}",
        file("I01")
    );
    let v = refused(
        &repo,
        &["change", "iteration", "I02", "--state", "active"],
        1,
        "iteration_active_already",
    );
    assert_eq!(v["error"]["params"]["active"], "I01");
    refused(
        &repo,
        &["change", "iteration", "I02", "--state", "doing"],
        1,
        "invalid_iteration_state",
    );

    // A new task goes into the active iteration, unless another is named.
    let v = ok(
        &repo,
        &["add", "task", "--area", "A01", "--title", "In one"],
    );
    let in_one = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"]["iteration"], "I01");
    assert!(
        v["next_steps"]
            .as_str()
            .unwrap()
            .contains("It went into iteration I01"),
        "{v}"
    );
    let v = ok(
        &repo,
        &[
            "add",
            "task",
            "--area",
            "A01",
            "--title",
            "In two",
            "--iteration",
            "I02",
        ],
    );
    let in_two = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"]["iteration"], "I02");

    // `alm next`: the active iteration's tasks first, whatever the order of iterations.
    ok(&repo, &["change", "iteration", "I01", "--state", "planned"]);
    ok(&repo, &["change", "iteration", "I02", "--state", "active"]);
    ok(&repo, &["ready", &in_one]);
    ok(&repo, &["ready", &in_two]);
    let v = ok(&repo, &["next"]);
    assert_eq!(v["tasks"][0]["id"], in_two.as_str(), "{v}");
    assert_eq!(v["active_iteration"], "I02");
    assert!(
        v["instruction"]
            .as_str()
            .unwrap()
            .contains("from the active iteration")
    );
    let listed = ok(&repo, &["list", "--by", "iteration"]);
    assert!(
        listed.to_string().contains("\"state\":\"active\""),
        "{listed}"
    );

    // Closing I02 with an unfinished task lists it and moves nothing.
    let v = ok(&repo, &["change", "iteration", "I02", "--state", "closed"]);
    assert_eq!(v["unfinished"][0]["id"], in_two.as_str());
    assert_eq!(ok(&repo, &["show", &in_two])["task"]["iteration"], "I02");
    let out = alm(&repo, &["change", "iteration", "I02", "--state", "closed"]);
    assert!(
        String::from_utf8(out.stdout)
            .unwrap()
            .contains("none was moved")
    );

    // The active iteration all done: `alm next` suggests closing it and the next planned one;
    // it changes no state itself.
    ok(&repo, &["change", "iteration", "I01", "--state", "active"]);
    ok(&repo, &["add", "iteration", "I03", "Three"]);
    ok(&repo, &["drop", &in_one]);
    ok(&repo, &["drop", &loose]);
    let v = ok(&repo, &["next"]);
    assert_eq!(v["finished_iteration"]["iteration"], "I01", "{v}");
    assert_eq!(v["finished_iteration"]["next"], "I03", "{v}");
    assert_eq!(
        ok(&repo, &["list", "--by", "iteration"])
            .to_string()
            .matches("\"state\":\"active\"")
            .count(),
        1
    );
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]));
}

/// Task-A01-piano403 (approver, 2026-10-25): a taskbag a newer Allomos wrote, with fields and
/// values this one does not know, opens; each is listed as a reminder; changing those tasks keeps
/// them; and a state this Allomos does not know is never changed.
#[test]
fn future_fields_and_values_are_read_listed_and_kept() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    let edit = |file: &str, f: &dyn Fn(&mut Value)| {
        let path = bag.join(file);
        let mut v: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        f(&mut v);
        fs::write(&path, v.to_string()).unwrap();
    };
    edit("tasks/A01-01/task.json", &|t| t["colour"] = json!("red"));
    edit("tasks/A01-02/task.json", &|t| t["state"] = json!("parked"));
    edit("project.json", &|p| p["theme"] = json!({"dark": true}));
    let v = ok(&repo, &["list"]);
    let issues: Vec<(String, String)> = v["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            (
                i["field"].as_str().unwrap().to_string(),
                i["problem"]["code"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    for want in [
        ("theme", "field_unknown"),
        ("colour", "field_unknown"),
        ("state", "value_unknown"),
    ] {
        assert!(
            issues.contains(&(want.0.to_string(), want.1.to_string())),
            "{want:?} in {issues:?}"
        );
    }
    assert!(v.to_string().contains("A01-02"), "the task is listed: {v}");
    // Changing the tasks keeps what this Allomos does not know.
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    ok(&repo, &["note", "A01-02", "kept as it is"]);
    let read = |file: &str| -> Value {
        serde_json::from_str(&fs::read_to_string(bag.join(file)).unwrap()).unwrap()
    };
    let one = read("tasks/A01-01/task.json");
    assert_eq!(
        (&one["colour"], &one["state"]),
        (&json!("red"), &json!("in_progress"))
    );
    let two = read("tasks/A01-02/task.json");
    assert_eq!(two["state"], "parked");
    assert_eq!(two["handoff_note"], "kept as it is");
    // Its state is not changed.
    let before = snapshot(&repo);
    refused(&repo, &["--as", "a", "claim", "A01-02"], 1, "state_unknown");
    assert_eq!(snapshot(&repo), before);
    assert_eq!(read("project.json")["theme"], json!({"dark": true}));
}

/// Task-A01-dime494 (decision 0019; approver 2026-10-25: option B): an `allomos/5` taskbag is
/// read as it is until `alm upgrade --confirm`; then tasks name their project folders, chosen by
/// whoever adds them, and `alm next` inside a folder lists only that folder's tasks.
#[test]
fn tasks_name_their_project_folders_after_an_upgrade() {
    let repo = filled();
    let bag = repo.join(".allomos/taskbag");
    let project_file = bag.join("project.json");
    let edit = |f: &dyn Fn(&mut Value)| {
        let mut v: Value =
            serde_json::from_str(&fs::read_to_string(&project_file).unwrap()).unwrap();
        f(&mut v);
        fs::write(&project_file, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    };
    edit(&|v| {
        v["format"] = json!("allomos/5");
        v["project_folder"] = json!(".");
    });
    // Still read as allomos/5, but changes (folders too) need the upgrade.
    ok(&repo, &["list"]);
    let add = |extra: &[&str]| {
        let mut args = vec!["add", "task", "--area", "A01", "--title", "x"];
        args.extend_from_slice(extra);
        args.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    };
    fn strs(v: &[String]) -> Vec<&str> {
        v.iter().map(String::as_str).collect()
    }
    refused(&repo, &strs(&add(&[])), 1, "taskbag_to_upgrade");
    refused(
        &repo,
        &strs(&add(&["--folder", "main"])),
        1,
        "taskbag_to_upgrade",
    );
    // `alm upgrade` alone says what it will do and changes nothing.
    let before = snapshot(&repo);
    let v = ok(&repo, &["upgrade"]);
    assert_eq!(v["upgraded"], false);
    assert_eq!(snapshot(&repo), before);
    let v = ok(&repo, &["upgrade", "--confirm"]);
    assert_eq!(v["upgraded"], true, "{v}");
    let project: Value = serde_json::from_str(&fs::read_to_string(&project_file).unwrap()).unwrap();
    assert_eq!(project["format"], "allomos/8");
    assert_eq!(project.get("project_folder"), None);
    assert_eq!(project["project_folders"][0]["path"], ".");
    let main = project["project_folders"][0]["name"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(ok(&repo, &["upgrade", "--confirm"])["upgraded"], false);
    // A second folder (binding comes with alm folder add, A05-taco639).
    fs::create_dir(repo.join("site")).unwrap();
    edit(&|v| {
        v["project_folders"]
            .as_array_mut()
            .unwrap()
            .push(json!({ "name": "site", "path": "site" }))
    });
    // The agent decides: neither option is refused, listing the folders.
    let v = refused(&repo, &strs(&add(&[])), 1, "folder_choice_missing");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("site (site)"),
        "{v}"
    );
    refused(
        &repo,
        &strs(&add(&["--folder", "nope"])),
        1,
        "folder_unknown",
    );
    let v = ok(
        &repo,
        &strs(&add(&["--folder", "site", "--folder", "site"])),
    );
    let site_task = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"]["folders"], json!(["site"]));
    let v = ok(&repo, &strs(&add(&["--no-folder"])));
    let no_folder = v["task"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["task"].get("folders"), None);
    // Changed later; recorded in the history.
    ok(&repo, &["folders", "A01-01", &main]);
    assert_eq!(
        ok(&repo, &["show", "A01-01"])["task"]["folders"],
        json!([main])
    );
    refused(&repo, &["folders", "A01-01", "nope"], 1, "folder_unknown");
    let h = ok(&repo, &["history", "A01-01"]);
    assert!(h.to_string().contains("\"field\":\"folders\""), "{h}");
    // `alm list --folder`.
    let v = ok(&repo, &["list", "--folder", "site"]);
    let listed = v.to_string();
    assert!(
        listed.contains(&site_task) && !listed.contains("A01-01\""),
        "{listed}"
    );
    // `alm next` inside a folder: only its tasks, not those with no folder.
    for id in [&site_task, &no_folder, &"A01-01".to_string()] {
        ok(&repo, &["ready", id]);
    }
    let ids = |v: &Value| -> Vec<String> {
        v["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap().to_string())
            .collect()
    };
    let v = ok(&repo.join("site"), &["next"]);
    assert_eq!(v["folder"], "site");
    assert_eq!(ids(&v), [site_task.clone()]);
    let v = ok(&repo, &["next"]);
    assert_eq!(v["folder"], json!(main));
    assert!(ids(&v).contains(&"A01-01".to_string()) && !ids(&v).contains(&no_folder));
    // In the taskbag, or with --all-folders: every ready task.
    let v = ok(&repo.join("site"), &["next", "--all-folders"]);
    assert_eq!(v["folder"], Value::Null);
    assert!(ids(&v).contains(&no_folder) && ids(&v).contains(&site_task));
    let v = ok(&bag, &["next"]);
    assert!(ids(&v).contains(&no_folder));
    // Clearing a task's folders.
    ok(&repo, &["folders", "A01-01", "--none"]);
    assert_eq!(ok(&repo, &["show", "A01-01"])["task"].get("folders"), None);
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
}

/// Task-A05-taco639 (decision 0019): `alm folder` binds, renames, locates and unbinds a taskbag's
/// project folders; binding writes Allomos's section into the folder's AGENTS.md, pointing back
/// to the taskbag, and unbinding removes only that section and keeps the tasks.
#[test]
fn project_folders_are_bound_renamed_located_and_unbound() {
    let repo = filled();
    // Folders need allomos/6 (a new taskbag is).
    let site = repo.join("site");
    fs::create_dir(&site).unwrap();
    fs::write(site.join("AGENTS.md"), "# Site\n\nBuild with make.\n").unwrap();
    refused(
        &repo,
        &["folder", "add", "missing"],
        1,
        "startup_not_a_folder",
    );
    let v = ok(&repo, &["folder", "add", "site"]);
    assert_eq!(v["folder"]["name"], "site");
    assert_eq!(v["folder"]["path"], "site");
    assert_eq!(v["folder_files"], json!(["AGENTS.md", "CLAUDE.md"]));
    let agents = fs::read_to_string(site.join("AGENTS.md")).unwrap();
    assert!(
        agents.starts_with("# Site\n\nBuild with make.\n"),
        "{agents}"
    );
    assert!(agents.contains("## Tasks: Allomos"), "{agents}");
    assert!(
        agents.contains("`../.allomos/taskbag`"),
        "points back: {agents}"
    );
    assert_eq!(
        fs::read_to_string(site.join("CLAUDE.md")).unwrap(),
        "@AGENTS.md\n"
    );
    // An agent started in the folder finds the taskbag from what AGENTS.md says.
    let v = ok(&site, &["--taskbag", "../.allomos/taskbag", "list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
    // Refusals: a name taken, an invalid name, an unknown folder.
    fs::create_dir(repo.join("other")).unwrap();
    refused(
        &repo,
        &["folder", "add", "other", "--name", "site"],
        1,
        "folder_name_taken",
    );
    refused(
        &repo,
        &["folder", "add", "other", "--name", "Bad Name"],
        1,
        "folder_name_invalid",
    );
    refused(
        &repo,
        &["folder", "rename", "nope", "x"],
        1,
        "folder_unknown",
    );
    // A task in it, then rename: the task follows.
    ok(
        &repo,
        &[
            "add",
            "task",
            "--area",
            "A01",
            "--title",
            "Site work",
            "--folder",
            "site",
        ],
    );
    let v = ok(&repo, &["folder", "rename", "site", "web"]);
    assert_eq!(v["folder"]["name"], "web");
    assert_eq!(v["tasks"].as_array().unwrap().len(), 1);
    let id = v["tasks"][0].as_str().unwrap().to_string();
    assert_eq!(ok(&repo, &["show", &id])["task"]["folders"], json!(["web"]));
    // Locate it after a move: no files move; the section is written where it now is.
    let moved = repo.join("moved-site");
    fs::rename(&site, &moved).unwrap();
    let v = ok(&repo, &["folder", "list"]);
    assert_eq!(v["folders"][0]["exists"], false);
    let v = ok(&repo, &["folder", "locate", "web", "moved-site"]);
    assert_eq!(v["folder"]["path"], "moved-site");
    assert_eq!(v["folder"]["exists"], true);
    assert_eq!(v["folder"]["section"], true);
    // Remove: first only what it will do, then with --confirm.
    let before = snapshot(&repo);
    let v = ok(&repo, &["folder", "remove", "web"]);
    assert_eq!(v["removed"], false);
    assert_eq!(v["folder"]["tasks"], 1);
    assert_eq!(v["folder"]["open"], 1);
    assert_eq!(snapshot(&repo), before);
    let v = ok(&repo, &["folder", "remove", "web", "--confirm"]);
    assert_eq!(v["removed"], true);
    let agents = fs::read_to_string(moved.join("AGENTS.md")).unwrap();
    assert_eq!(agents, "# Site\n\nBuild with make.\n");
    assert!(moved.join("CLAUDE.md").is_file(), "files stay");
    // The task stays, with no folder now.
    let t = ok(&repo, &["show", &id]);
    assert_eq!(t["task"].get("folders"), None);
    assert_eq!(ok(&repo, &["folder", "list"])["folders"], json!([]));
    let v = ok(&repo, &["list"]);
    assert_eq!(v["issues"], json!([]), "{v}");
}

/// Task-A03-hive049: `alm claim` stores the session the agent passes, as given; the task keeps it
/// after release, and a new claim replaces it (nothing passed clears it, with a reminder).
#[test]
fn claim_records_the_session_the_agent_passes() {
    let repo = filled();
    let wt = repo.join("wt");
    fs::create_dir(&wt).unwrap();
    let v = ok(
        &wt,
        &[
            "--as",
            "claude/x",
            "claim",
            "A01-01",
            "--session-agent",
            "claude-code",
            "--session-id",
            " 9ad41e08 ",
            "--session-kind",
            "desktop",
            "--app-session-id",
            "local_ec51",
        ],
    );
    assert_eq!(
        v["task"]["session"],
        json!({
            "by": "claude/x",
            "agent": "claude-code",
            "id": "9ad41e08",
            "kind": "desktop",
            "app_session_id": "local_ec51",
            "folder": "wt",
        })
    );
    ok(
        &repo,
        &["--as", "claude/x", "release", "A01-01", "--note", "n"],
    );
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["task"]["session"]["id"], "9ad41e08", "{shown}");
    // A claim with nothing passed replaces it with nothing, and says what to pass.
    let out = alm(&repo, &["--as", "codex/y", "claim", "A01-01"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("--session-agent <your agent's id>"), "{text}");
    let shown = ok(&repo, &["show", "A01-01"]);
    assert!(shown["task"].get("session").is_none(), "{shown}");
    // The history keeps the earlier one.
    let h = ok(&repo, &["history", "A01-01"]);
    assert!(h.to_string().contains("local_ec51"), "{h}");
}

/// Task-A05-board831: `alm translate <code>` prints the app's Translate the taskbag… request for
/// any language code, and changes nothing.
#[test]
fn translate_prints_the_request_for_any_language() {
    let repo = filled();
    let v = ok(
        &repo,
        &[
            "translate",
            "vi",
            "--name",
            "Tiếng Việt",
            "--language",
            "en",
        ],
    );
    let request = v["request"].as_str().unwrap();
    assert!(request.contains("into Tiếng Việt (`vi`)"), "{request}");
    assert!(request.contains("task-language vi"), "{request}");
    // Nothing changed: the task language is still the default.
    assert_eq!(
        ok(&repo, &["task-language"])["task_language"]["setting"],
        "ui"
    );
    refused(&repo, &["translate", "not a code"], 1, "invalid_language");
}

/// Task-A03-ham538: after a command that changes a task, alm lists what the task is missing (session
/// details, working branch, handoff note) with the fix, refusing nothing; `alm session` fills the
/// session in afterwards.
#[test]
fn task_commands_list_what_the_task_is_missing() {
    let repo = filled();
    // Claimed without session details: named, with the fix; the claim itself goes through.
    let out = alm(
        &repo,
        &["--as", "claude/x", "claim", "A01-01", "--branch", "main"],
    );
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("Fact: Task A01-01: no session is recorded")
            && text.contains("Fix: `alm session A01-01 --session-agent"),
        "{text}"
    );
    let v = ok(
        &repo,
        &[
            "--as",
            "claude/x",
            "label",
            "A01-01",
            "locally verified by agent",
        ],
    );
    let codes: Vec<&str> = v["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["session"], "{v}");
    // Filled in afterwards; what was given replaces, the rest stays.
    let v = ok(
        &repo,
        &[
            "--as",
            "claude/x",
            "session",
            "A01-01",
            "--session-agent",
            "claude-code",
            "--session-id",
            "s1",
        ],
    );
    assert_eq!(v["task"]["session"]["id"], "s1");
    // A Claude Code session that does not say where it runs (Task-A03-sugar117): told to add the
    // desktop details; `--session-kind terminal` says it runs in a terminal.
    assert_eq!(v["missing"][0]["code"], "session_kind", "{v}");
    assert_eq!(
        v["missing"][0]["fix"],
        "alm session A01-01 --session-kind desktop --app-session-id <its local_… ID>"
    );
    let v = ok(
        &repo,
        &[
            "--as",
            "claude/x",
            "session",
            "A01-01",
            "--session-id",
            "s2",
            "--session-kind",
            "terminal",
        ],
    );
    assert_eq!(v["missing"], json!([]), "{v}");
    assert_eq!(v["task"]["session"]["agent"], "claude-code", "{v}");
    assert_eq!(v["task"]["session"]["id"], "s2");
    // Someone else's task, or nothing to record: refused.
    refused(
        &repo,
        &["--as", "codex/y", "session", "A01-01", "--session-id", "s3"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &["--as", "claude/x", "session", "A01-01"],
        1,
        "session_missing",
    );
    // Done to check without a handoff note: named.
    let v = ok(
        &repo,
        &[
            "--as",
            "claude/x",
            "done",
            "A01-01",
            "--without-human-check",
            "only the hands-on check is left",
        ],
    );
    assert_eq!(v["missing"][0]["code"], "handoff_note", "{v}");
    // After done, anyone may correct the session.
    ok(
        &repo,
        &[
            "--as",
            "codex/y",
            "session",
            "A01-01",
            "--session-kind",
            "cli",
        ],
    );
}

/// Decision 0021 § 1.1 "Reasons" (Task-A01-air289): state changes, labels taken off and plan
/// changes record why as a `reason` change; without one `alm` asks (`event_missing`: `reason`)
/// and refuses nothing; the commands that already say why record that; older events still read.
#[test]
fn state_and_plan_changes_record_why() {
    let repo = filled();
    fs::write(
        repo.join(".allomos/taskbag/events/2026-01-02-030405-000-old002.json"),
        r#"{"at":"2026-01-02T03:04:05Z","via":"alm","command":"drop","kind":"task","id":"A01-01","changes":[{"field":"state","from":"ready","to":"dropped"}]}"#,
    )
    .unwrap();
    let reason = |e: &Value| -> Option<String> {
        e["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["field"] == "reason")
            .map(|c| c["to"].as_str().unwrap().to_string())
    };
    let asked = |v: &Value| {
        v["event_missing"]
            .as_array()
            .is_some_and(|a| a.iter().any(|m| m["code"] == "reason"))
    };
    // Made ready with --why: recorded, nothing asked.
    let v = ok(
        &repo,
        &["--why", "the approver approved it", "ready", "A01-01"],
    );
    let e = newest_event(&repo);
    assert_eq!(
        reason(&e).as_deref(),
        Some("the approver approved it"),
        "{e}"
    );
    assert!(!asked(&v), "{v}");
    // Claiming is a step forward: no reason asked for.
    let v = ok(&repo, &["--as", "s1", "claim", "A01-01"]);
    assert!(!asked(&v) && reason(&newest_event(&repo)).is_none(), "{v}");
    // Releasing without --why: made all the same, and alm asks for a reason.
    let v = ok(&repo, &["--as", "s1", "release", "A01-01"]);
    assert_eq!(task_file(&repo, "A01-01")["state"], "ready");
    assert!(asked(&v) && reason(&newest_event(&repo)).is_none(), "{v}");
    let text = String::from_utf8(alm(&repo, &["--as", "s1", "claim", "A01-01"]).stdout).unwrap();
    assert!(!text.contains("--why"), "{text}");
    let text = String::from_utf8(alm(&repo, &["--as", "s1", "release", "A01-01"]).stdout).unwrap();
    assert!(
        text.contains("Fact:") && text.contains("alm --why"),
        "{text}"
    );
    // wait --on is its own why.
    let v = ok(&repo, &["wait", "A01-02", "--on", "the approver's answer"]);
    assert_eq!(
        reason(&newest_event(&repo)).as_deref(),
        Some("the approver's answer")
    );
    assert!(!asked(&v), "{v}");
    // A label taken off asks; one added does not.
    let v = ok(&repo, &["label", "A02-01", "locally verified by agent"]);
    assert!(!asked(&v), "{v}");
    let v = ok(
        &repo,
        &["label", "A02-01", "locally verified by agent", "--remove"],
    );
    assert!(asked(&v), "{v}");
    // Dropping with --why.
    ok(&repo, &["drop", "A02-01", "--why", "a duplicate of A01-02"]);
    assert_eq!(
        reason(&newest_event(&repo)).as_deref(),
        Some("a duplicate of A01-02")
    );
    // A plan change: blocked by another task.
    let v = ok(&repo, &["block", "A01-01", "--by", "A01-02"]);
    assert!(asked(&v), "{v}");
    // Reopening says why with its --note.
    let v = ok(&repo, &["reopen", "A02-01", "--note", "more work on it"]);
    assert_eq!(
        reason(&newest_event(&repo)).as_deref(),
        Some("more work on it")
    );
    assert!(!asked(&v), "{v}");
    // A change that is neither a state nor a plan change asks for nothing.
    let v = ok(&repo, &["title", "A01-02", "List them"]);
    assert!(!asked(&v), "{v}");
    // The history still reads, the older event among the new ones.
    let v = ok(&repo, &["history", "A01-01"]);
    let text = v.to_string();
    assert!(
        text.contains("2026-01-02T03:04:05Z") && text.contains("the approver approved it"),
        "{v}"
    );
    assert!(v["issues"].as_array().is_none_or(|a| a.is_empty()), "{v}");
}

/// Task-A01-coach782 (decision 0021 § 1.1): an approval is a record in each task it covers;
/// withdrawing it, a changed description and an expiry each make it no longer apply; older
/// approval texts and older events still read.
#[test]
fn an_approval_is_recorded_withdrawn_and_ends_when_the_plan_changes() {
    let repo = filled();
    // An older `approved` event (Task-A03-37), written before records: still reads, not listed.
    fs::write(
        repo.join(".allomos/taskbag/events/2026-10-01-000000-000-old001.json"),
        r#"{"at":"2026-10-01T00:00:00Z","via":"alm","command":"approved","kind":"task","id":"A01-01","changes":[{"field":"approval","to":"Deploy (plan 0123abcd)"}]}"#,
    )
    .unwrap();
    refused(&repo, &["approve", "A01-01"], 1, "approval_words_missing");
    refused(
        &repo,
        &["approve", "A09-09", "--words", "ok"],
        1,
        "task_not_found",
    );
    refused(
        &repo,
        &["approve", "A01-01", "--words", "ok", "--expires", "soon"],
        1,
        "invalid_date",
    );
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "approve",
            "A01-01",
            "A01-02",
            "--words",
            "go ahead",
            "--approver",
            "p-abc123",
            "--expires",
            "2999-12-31",
        ],
    );
    let ap = &v["approval"];
    let id = ap["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("ap-"), "{v}");
    assert_eq!(ap["approver"], "p-abc123");
    assert_eq!(ap["channel"], "chat");
    assert_eq!(ap["words"], "go ahead");
    assert_eq!(ap["scope"]["tasks"], json!(["A01-01", "A01-02"]));
    assert_eq!(ap["scope"]["plan_sha256"].as_str().unwrap().len(), 64);

    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["approvals"].as_array().unwrap().len(), 1, "{shown}");
    assert_eq!(shown["approvals"][0]["status"], "applies");
    let text = String::from_utf8(alm(&repo, &["show", "A01-02"]).stdout).unwrap();
    assert!(
        text.contains(&format!("{id} by p-abc123")) && text.contains("applies"),
        "{text}"
    );

    // A changed plan: the approval no longer applies to it.
    ok(&repo, &["describe", "A01-01", "--text", "A new plan."]);
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["approvals"][0]["status"], "plan_changed");

    // A new approval of the new plan, for one task, from a file plan; then withdrawn.
    fs::write(repo.join("plan.md"), "Deploy it.").unwrap();
    let v = ok(
        &repo,
        &["approve", "A01-01", "--words", "yes", "--plan", "plan.md"],
    );
    let second = v["approval"]["id"].as_str().unwrap().to_string();
    assert_eq!(v["approval"]["scope"]["plan"], "plan.md");
    ok(
        &repo,
        &["--why", "the person took it back", "withdraw", &id],
    );
    refused(&repo, &["withdraw", &id], 1, "approval_already_withdrawn");
    refused(&repo, &["withdraw", "ap-none00"], 1, "approval_not_found");
    let shown = ok(&repo, &["show", "A01-01"]);
    let statuses: Vec<&str> = shown["approvals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, ["withdrawn", "applies"], "{shown}");
    assert_eq!(shown["approvals"][1]["id"], second.as_str());

    // An expired approval.
    ok(
        &repo,
        &[
            "approve",
            "A02-01",
            "--words",
            "ok",
            "--expires",
            "2000-01-01",
        ],
    );
    let shown = ok(&repo, &["show", "A02-01"]);
    assert_eq!(shown["approvals"][0]["status"], "expired");
    // Every event still reads: no issues.
    let list = ok(&repo, &["list"]);
    assert!(
        list["issues"].as_array().is_none_or(|a| a.is_empty()),
        "{list}"
    );
}

#[test]
fn an_approval_withdrawn_while_in_progress_is_pointed_out() {
    // Task-A01-baker800 (approver, 2026-11-08): withdrawing stops nothing; `alm show` and
    // `alm next` say so while the task is in progress, and the holder decides.
    let repo = filled();
    let v = ok(&repo, &["approve", "A01-01", "--words", "go"]);
    let id = v["approval"]["id"].as_str().unwrap().to_string();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let shown = ok(&repo, &["show", "A01-01"]);
    assert!(shown["approval_withdrawn"].is_null(), "{shown}");
    ok(&repo, &["--why", "wrong plan", "withdraw", &id]);
    let shown = ok(&repo, &["show", "A01-01"]);
    assert_eq!(shown["approval_withdrawn"], id.as_str(), "{shown}");
    assert_eq!(shown["task"]["state"], "in_progress");
    let text = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(
        text.contains("Approval withdrawn:") && text.contains("a decides"),
        "{text}"
    );
    let next = ok(&repo, &["next"]);
    assert_eq!(
        next["approval_withdrawn"],
        json!([{ "task": "A01-01", "approval": id }]),
        "{next}"
    );
    let text = String::from_utf8(alm(&repo, &["next"]).stdout).unwrap();
    assert!(text.contains("Approval withdrawn:"), "{text}");
    // A newer approval that applies: nothing to point out.
    ok(&repo, &["approve", "A01-01", "--words", "go again"]);
    let next = ok(&repo, &["next"]);
    assert_eq!(next["approval_withdrawn"], json!([]), "{next}");
}

#[test]
fn a_task_traces_to_requirements_and_decisions_and_back() {
    // Task-A01-fennel617 (decision 0021 § 1.1 "Traceability"): `alm trace` records the
    // requirements and decisions a task serves as a `traces` change in its history only;
    // `alm show` lists them; `alm traces <ref>` finds the tasks back.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &["--as", "a", "trace", "A01-01", "R77", "audit"],
        1,
        "trace_invalid",
    );
    refused(
        &repo,
        &["--as", "a", "trace", "A09-09", "R77"],
        1,
        "task_not_found",
    );
    refused(
        &repo,
        &["--as", "b", "trace", "A01-01", "R77"],
        1,
        "task_held",
    );
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "trace",
            "A01-01",
            "0021-audit-trail",
            "r77",
            "R5",
        ],
    );
    assert_eq!(v["traces"], json!(["R5", "R77", "0021"]));
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");
    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "trace");
    assert_eq!(last["changes"][0]["field"], "traces");
    assert_eq!(last["changes"][0]["to"], json!(["R5", "R77", "0021"]));

    // The same list again changes nothing and records nothing.
    let events = event_count(&repo);
    ok(&repo, &["--as", "a", "trace", "A01-01", "R77"]);
    assert_eq!(event_count(&repo), events);

    // A task nobody holds, done ones included, may be traced; --remove takes one off.
    ok(&repo, &["trace", "A01-02", "0021"]);
    let v = ok(&repo, &["--as", "a", "trace", "A01-01", "--remove", "R5"]);
    assert_eq!(v["traces"], json!(["R77", "0021"]));

    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["traces"], json!(["R77", "0021"]));
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("traces: R77, 0021"), "{out}");

    let v = ok(&repo, &["traces", "21"]);
    assert_eq!(v["reference"], "0021");
    assert_eq!(v["tasks"], json!(["A01-01", "A01-02"]));
    let v = ok(&repo, &["traces", "R5"]);
    assert_eq!(v["tasks"], json!([]));
    let v = ok(&repo, &["traces"]);
    assert_eq!(
        v["traces"],
        json!({ "R77": ["A01-01"], "0021": ["A01-01", "A01-02"] })
    );
    refused(&repo, &["traces", "nope"], 1, "trace_invalid");
}

#[test]
fn an_outside_effect_is_recorded_before_and_after_and_listed() {
    // Task-A01-job650 (decision 0021 § 1.1 "Outside effects"): `alm effect` records target,
    // action, rollback, approval and result as an `effect` change in the task's history only;
    // `alm show` and `alm effects` list them, for one task and for the project.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &["--as", "a", "effect", "A01-01", "--target", "production"],
        1,
        "effect_incomplete",
    );
    refused(
        &repo,
        &[
            "--as",
            "b",
            "effect",
            "A01-01",
            "--target",
            "t",
            "--action",
            "a",
            "--rollback",
            "r",
        ],
        1,
        "task_held",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "--approval",
            "ap-nope00",
            "effect",
            "A01-01",
            "--target",
            "t",
            "--action",
            "a",
            "--rollback",
            "r",
        ],
        1,
        "approval_not_found",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "effect",
            "A01-01",
            "--of",
            "ef-nope00",
            "--result",
            "ok",
        ],
        1,
        "effect_not_found",
    );
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    // Without an approval: recorded, and alm says so (it refuses nothing).
    let out = alm(
        &repo,
        &[
            "--as",
            "a",
            "effect",
            "A01-01",
            "--target",
            "staging",
            "--action",
            "apply 0003.sql",
            "--rollback",
            "apply 0003-down.sql",
        ],
    );
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("it names no approval"), "{text}");
    assert!(text.contains("Under way."), "{text}");

    let ap = ok(
        &repo,
        &["--as", "a", "approve", "A01-01", "--words", "deploy it"],
    );
    let ap_id = ap["approval"]["id"].as_str().unwrap().to_string();
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "--approval",
            &ap_id,
            "effect",
            "A01-01",
            "--target",
            "production",
            "--action",
            "deploy 0.1.43",
            "--rollback",
            "redeploy 0.1.42",
        ],
    );
    let ef_id = v["effect"]["id"].as_str().unwrap().to_string();
    assert!(ef_id.starts_with("ef-"), "{ef_id}");
    assert_eq!(v["effect"]["approval"], json!(ap_id));
    assert_eq!(v["effect"].get("result"), None);
    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "effect");
    assert_eq!(last["approval"], json!(ap_id));
    assert_eq!(last["changes"][0]["field"], "effect");
    assert_eq!(last["changes"][0]["to"]["target"], "production");

    // After it: the result; the same again changes nothing.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "effect",
            "A01-01",
            "--of",
            &ef_id,
            "--result",
            "deployed; health check green",
        ],
    );
    assert_eq!(v["changed"], json!(true));
    assert_eq!(
        v["effect"]["approval"],
        json!(ap_id),
        "the approval is kept"
    );
    let history = ok(&repo, &["history", "A01-01"]);
    assert_eq!(
        history["events"][0]["changes"][0]["from"]["id"],
        json!(ef_id)
    );
    let events = event_count(&repo);
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "effect",
            "A01-01",
            "--of",
            &ef_id,
            "--result",
            "deployed; health check green",
        ],
    );
    assert_eq!(v["changed"], json!(false));
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");

    // One recorded afterwards gives everything at once (a task nobody holds).
    ok(
        &repo,
        &[
            "effect",
            "A01-02",
            "--target",
            "DNS",
            "--action",
            "add a record",
            "--rollback",
            "remove it",
            "--result",
            "added",
        ],
    );

    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["effects"].as_array().unwrap().len(), 2);
    assert_eq!(v["effects"][1]["result"], "deployed; health check green");
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("Outside effects:"), "{out}");
    assert!(out.contains("rollback: redeploy 0.1.42"), "{out}");

    let v = ok(&repo, &["effects", "A01-01"]);
    assert_eq!(v["effects"].as_array().unwrap().len(), 2);
    let v = ok(&repo, &["effects"]);
    let all = v["effects"].as_array().unwrap();
    assert_eq!(all.len(), 3);
    assert_eq!(all[2]["task"], "A01-02");
    let out = String::from_utf8(alm(&repo, &["effects"]).stdout).unwrap();
    assert!(out.contains("1 without a result yet"), "{out}");
}

/// Task-A01-frolic587 (decision 0021 § 2, § 6): the identity level is shown and set with
/// `alm audit-settings`; a local-only project keeps the pseudonym, and the events of a person's
/// changes hold only their pseudonym.
#[test]
fn audit_settings_keep_a_local_project_on_the_pseudonym() {
    let repo = filled();
    let v = ok(&repo, &["audit-settings"]);
    assert_eq!(
        v["identity"],
        json!({ "level": "pseudonym", "reauthenticate": false, "sign_in_available": false })
    );
    assert_eq!(v["changed"], Value::Null);
    refused(
        &repo,
        &["audit-settings", "--identity", "account"],
        1,
        "identity_needs_cloud",
    );
    refused(
        &repo,
        &["audit-settings", "--identity", "sso"],
        1,
        "identity_needs_cloud",
    );
    refused(
        &repo,
        &["audit-settings", "--identity", "passport"],
        1,
        "identity_level_unknown",
    );
    refused(
        &repo,
        &["audit-settings", "--reauthenticate", "on"],
        1,
        "reauthenticate_needs_sign_in",
    );
    let events = || {
        fs::read_dir(repo.join(".allomos/taskbag/events"))
            .unwrap()
            .count()
    };
    let before = events();
    let v = ok(&repo, &["audit-settings", "--identity", "pseudonym"]);
    assert_eq!(v["changed"], false);
    assert_eq!(events(), before, "nothing changed, nothing recorded");
    let project = fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap();
    assert!(
        !project.contains("audit"),
        "the default is not written: {project}"
    );

    // A person's change at the default level: the pseudonym, nothing else about them.
    ok(&repo, &["note", "A01-01", "mine"]);
    let newest = fs::read_dir(repo.join(".allomos/taskbag/events"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .max()
        .unwrap();
    let e: Value = serde_json::from_str(&fs::read_to_string(newest).unwrap()).unwrap();
    let person = e["actor"]["person"].as_str().unwrap();
    assert!(person.starts_with("p-") && person.len() == 8, "{e}");
    assert_eq!(e["actor"].as_object().unwrap().len(), 1, "{e}");
}

/// Task-A01-grotto036 (decision 0021 § 1.2, Question 6): separation of duties is a switch, off by
/// default. Off, whoever did a task may check it; on, `alm` refuses that and says why, and the
/// switch is turned off only with a reason.
#[test]
fn separation_of_duties_refuses_checking_your_own_work_only_when_on() {
    let repo = filled();
    let v = ok(&repo, &["audit-settings"]);
    assert_eq!(v["separation_of_duties"], false);
    // Off: the person who did A01-01 checks it themselves, as before.
    ok(&repo, &["--as", "w", "claim", "A01-01"]);
    ok(
        &repo,
        &[
            "--as",
            "w",
            "done",
            "A01-01",
            "--without-human-check",
            "run",
        ],
    );
    ok(&repo, &["--as", "w", "checked", "A01-01"]);

    let v = ok(&repo, &["audit-settings", "--separation-of-duties", "on"]);
    assert_eq!(v["separation_of_duties"], true);
    assert_eq!(v["changed"], true);
    let project = fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap();
    assert!(
        project.contains("\"separation_of_duties\": true"),
        "{project}"
    );
    let out = alm(&repo, &["audit-settings"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Separation of duties: on."),
        "{out:?}"
    );

    ok(&repo, &["--as", "w", "claim", "A01-02"]);
    ok(
        &repo,
        &[
            "--as",
            "w",
            "done",
            "A01-02",
            "--without-human-check",
            "run",
        ],
    );
    let before = fs::read_dir(repo.join(".allomos/taskbag/events"))
        .unwrap()
        .count();
    // The same person (this computer's pseudonym) may not check what they did.
    let v = refused(&repo, &["--as", "w", "checked", "A01-02"], 1, "self_check");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(
        message.contains("A01-02") && message.contains("p-"),
        "{message}"
    );
    assert_eq!(
        fs::read_dir(repo.join(".allomos/taskbag/events"))
            .unwrap()
            .count(),
        before,
        "a refused check records nothing"
    );
    assert!(
        task_file(&repo, "A01-02")["labels"]
            .as_array()
            .is_none_or(|l| !l.iter().any(|l| l == "locally verified by human"))
    );
    // Someone else may; their check is recorded.
    ok(
        &repo,
        &["--as", "w", "checked", "A01-02", "--checker", "p-other1"],
    );
    // Nor may the doer approve it; another person may, and then may not work on it.
    refused(
        &repo,
        &["approve", "A01-02", "--words", "go"],
        1,
        "self_approve",
    );
    ok(
        &repo,
        &[
            "approve",
            "A02-01",
            "--words",
            "go",
            "--approver",
            "p-boss01",
        ],
    );
    ok(&repo, &["--as", "w", "claim", "A02-01"]);

    // Turning it off needs a reason, which the history keeps.
    refused(
        &repo,
        &["audit-settings", "--separation-of-duties", "off"],
        1,
        "audit_reason_missing",
    );
    let v = ok(
        &repo,
        &[
            "audit-settings",
            "--separation-of-duties",
            "off",
            "--reason",
            "one person for now",
        ],
    );
    assert_eq!(v["separation_of_duties"], false);
    let e = newest_event(&repo);
    assert_eq!(e["command"], "audit-settings");
    assert!(
        e["changes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["field"] == "reason" && c["to"] == "one person for now"),
        "{e}"
    );
    let project = fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap();
    assert!(!project.contains("audit"), "{project}");
}

#[test]
fn a_task_links_to_its_code_and_releases_and_back() {
    // Task-A01-mint242 (decision 0021 § 1.1 "Links to code and releases"): `alm link` records a
    // commit, branch, merge, PR, CI run or release as a `link` change in the task's history only;
    // `alm show` lists them and `alm links <ref>` finds the task of a commit.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &["--as", "a", "link", "A01-01", "tag", "v1"],
        1,
        "link_kind_unknown",
    );
    refused(
        &repo,
        &["--as", "a", "link", "A01-01", "commit", "main"],
        1,
        "link_ref_invalid",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "link",
            "A01-01",
            "pr",
            "7",
            "--url",
            "example.test/7",
        ],
        1,
        "link_url_invalid",
    );
    refused(
        &repo,
        &["--as", "b", "link", "A01-01", "branch", "x"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &["--as", "a", "link", "A01-01", "branch", "x", "--remove"],
        1,
        "link_not_recorded",
    );
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    let out = alm(
        &repo,
        &["--as", "a", "link", "A01-01", "commit", "ABCDEF1234567890"],
    );
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("commit abcdef1234567890 recorded."), "{text}");
    assert!(text.contains("it has no web address"), "{text}");
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "link",
            "A01-01",
            "pr",
            "#7",
            "--url",
            "https://example.test/pull/7",
        ],
    );
    assert_eq!(v["link"]["ref"], "7");
    assert_eq!(v["links"].as_array().unwrap().len(), 2);
    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "link");
    assert_eq!(last["changes"][0]["field"], "link");
    assert_eq!(last["changes"][0]["to"]["kind"], "pr");

    // The same commit again with its address: changed; again: nothing.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "link",
            "A01-01",
            "commit",
            "abcdef1234567890",
            "--url",
            "https://example.test/commit/abcdef1",
        ],
    );
    assert_eq!(v["changed"], json!(true));
    let events = event_count(&repo);
    let v = ok(
        &repo,
        &["--as", "a", "link", "A01-01", "commit", "abcdef1234567890"],
    );
    assert_eq!(v["changed"], json!(false));
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");

    // A release on a task nobody holds, afterwards.
    ok(&repo, &["link", "A01-02", "release", "v0.1.43"]);

    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["links"].as_array().unwrap().len(), 2);
    assert_eq!(v["links"][0]["kind"], "commit");
    assert_eq!(v["links"][0]["url"], "https://example.test/commit/abcdef1");
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("Links to code and releases:"), "{out}");
    assert!(
        out.contains("- pr: 7  https://example.test/pull/7"),
        "{out}"
    );

    // Back from a commit (a short hash), a PR address, a tag; and every task's links.
    let v = ok(&repo, &["links", "abcdef1"]);
    assert_eq!(v["tasks"], json!(["A01-01"]));
    let v = ok(&repo, &["links", "https://example.test/pull/7"]);
    assert_eq!(v["tasks"], json!(["A01-01"]));
    let v = ok(&repo, &["links", "v0.1.43"]);
    assert_eq!(v["tasks"], json!(["A01-02"]));
    let v = ok(&repo, &["links", "fffffff"]);
    assert_eq!(v["tasks"], json!([]));
    let v = ok(&repo, &["links"]);
    assert_eq!(v["tasks"], json!(["A01-01", "A01-02"]));

    // Taken off.
    let v = ok(
        &repo,
        &["--as", "a", "link", "A01-01", "pr", "7", "--remove"],
    );
    assert_eq!(v["removed"], json!(true));
    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["links"].as_array().unwrap().len(), 1);
}

#[test]
fn a_task_has_a_change_type_and_risk_and_emergencies_are_listed_until_approved_afterwards() {
    // Task-A01-palm011 (decision 0021 § 1.1 "Risk class and change type"): `alm classify`
    // records the change type, risk and what a task touches as changes in its history only;
    // `alm show` lists them; an emergency without an approval recorded afterwards is listed by
    // `alm human` (with its deadline) until `alm approve` records one.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &["--as", "a", "classify", "A01-01", "--type", "urgent"],
        1,
        "classify_invalid",
    );
    refused(
        &repo,
        &["--as", "a", "classify", "A01-01", "--risk", "critical"],
        1,
        "classify_invalid",
    );
    refused(
        &repo,
        &["--as", "a", "classify", "A01-01", "--touches", "prod"],
        1,
        "classify_invalid",
    );
    refused(
        &repo,
        &["--as", "b", "classify", "A01-01", "--risk", "low"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "classify",
            "A01-01",
            "--type",
            "normal",
            "--due",
            "2026-11-11",
        ],
        2,
        "bad_usage",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "classify",
            "A01-01",
            "--type",
            "emergency",
            "--due",
            "soon",
        ],
        1,
        "invalid_date",
    );
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    // Nothing recorded yet: alm says how.
    let v = ok(&repo, &["classify", "A01-01"]);
    assert_eq!(v["classification"], json!({}));
    assert_eq!(event_count(&repo), events, "showing records nothing");

    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "classify",
            "A01-01",
            "--type",
            "Normal",
            "--risk",
            "medium",
            "--touches",
            "others-data,production",
        ],
    );
    assert_eq!(
        v["classification"],
        json!({ "change_type": "normal", "risk": "medium", "touches": ["production", "others_data"] })
    );
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");
    let history = ok(&repo, &["history", "A01-01"]);
    let last = &history["events"][0];
    assert_eq!(last["command"], "classify");
    let fields: Vec<&str> = last["changes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["field"].as_str().unwrap())
        .collect();
    assert_eq!(fields, vec!["change_type", "risk", "touches"]);

    // The same again records nothing.
    let events = event_count(&repo);
    ok(
        &repo,
        &["--as", "a", "classify", "A01-01", "--risk", "medium"],
    );
    assert_eq!(event_count(&repo), events);
    let v = ok(&repo, &["human"]);
    assert_eq!(v["to_approve_afterwards"], json!([]));

    // An emergency, past its deadline: listed by alm human, shown by alm show.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "classify",
            "A01-01",
            "--type",
            "emergency",
            "--due",
            "2000-01-01",
            "--touches",
            "none",
        ],
    );
    assert_eq!(v["classification"]["approval_due"], "2000-01-01");
    assert_eq!(
        v["classification"]["touches"],
        Value::Null,
        "touches neither: an empty list"
    );
    let v = ok(&repo, &["human"]);
    let list = v["to_approve_afterwards"].as_array().unwrap();
    assert_eq!(list.len(), 1, "{v}");
    assert_eq!(list[0]["task"], "A01-01");
    assert_eq!(list[0]["overdue"], true);
    let out = String::from_utf8(alm(&repo, &["human"]).stdout).unwrap();
    assert!(
        out.contains("To approve afterwards: emergency changes (1)"),
        "{out}"
    );
    assert!(out.contains("(past due)"), "{out}");
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("change type: emergency"), "{out}");
    assert!(out.contains("touches: neither"), "{out}");
    assert!(out.contains("not approved afterwards yet"), "{out}");

    // Approved afterwards: no longer listed.
    ok(&repo, &["approve", "A01-01", "--words", "fine, keep it"]);
    let v = ok(&repo, &["human"]);
    assert_eq!(v["to_approve_afterwards"], json!([]), "{v}");
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("emergency: approved afterwards"), "{out}");

    // Leaving emergency clears the deadline; a new emergency without --due gets three days.
    let v = ok(
        &repo,
        &["--as", "a", "classify", "A01-01", "--type", "standard"],
    );
    assert_eq!(v["classification"]["approval_due"], Value::Null);
    let out = alm_at(
        &repo,
        "2026-12-30T10:00:00Z",
        &[
            "--json",
            "--as",
            "a",
            "classify",
            "A01-02",
            "--type",
            "emergency",
        ],
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["classification"]["approval_due"], "2027-01-02", "{v}");
}

#[test]
fn exceptions_record_why_and_who_accepted_the_risk_and_are_listed() {
    // Task-A01-seed343 (decision 0021 § 1.1 "Exceptions"): a check not run and a task finished
    // without the human check are `exception` changes in the task's history, with why and who
    // accepted the risk; `alm show` and `alm exceptions` list them, for one task and the project.
    let repo = filled();
    ok(&repo, &["--as", "a", "claim", "A01-01"]);
    let before = task_file(&repo, "A01-01");
    let events = event_count(&repo);
    refused(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--kind",
            "skipped",
            "--why",
            "x",
        ],
        1,
        "exception_kind_unknown",
    );
    refused(
        &repo,
        &["--as", "a", "exception", "A01-01", "--kind", "other"],
        1,
        "exception_incomplete",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--kind",
            "check_not_run",
            "--why",
            "x",
        ],
        1,
        "exception_incomplete",
    );
    refused(
        &repo,
        &["--as", "b", "exception", "A01-01", "--why", "x"],
        1,
        "task_held",
    );
    refused(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--of",
            "ex-nope00",
            "--accepted-by",
            "approver",
        ],
        1,
        "exception_not_found",
    );
    assert_eq!(event_count(&repo), events, "a refusal records nothing");

    // A check not run, nobody named yet: recorded, and alm says so.
    let out = alm(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--kind",
            "check-not-run",
            "--check",
            "cargo test -p allomos-desktop",
            "--why",
            "no display on this computer",
        ],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(out.status.success(), "{text}");
    assert!(
        text.contains("nobody is named as accepting this risk"),
        "{text}"
    );
    assert_eq!(task_file(&repo, "A01-01"), before, "task.json is untouched");
    let v = ok(&repo, &["exceptions", "A01-01"]);
    let ex = v["exceptions"][0].clone();
    assert_eq!(ex["kind"], "check_not_run");
    assert_eq!(ex["check"], "cargo test -p allomos-desktop");
    assert_eq!(ex["accepted_by"], Value::Null);
    let id = ex["id"].as_str().unwrap().to_string();
    assert!(id.starts_with("ex-"));

    // Completed later with who accepted it; the same again changes nothing.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--of",
            &id,
            "--accepted-by",
            "approver",
        ],
    );
    assert_eq!(v["changed"], true);
    assert_eq!(v["exception"]["accepted_by"], "approver");
    let events = event_count(&repo);
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--of",
            &id,
            "--accepted-by",
            "approver",
        ],
    );
    assert_eq!(v["changed"], false);
    assert_eq!(event_count(&repo), events);

    // An email address is told about, not refused.
    let out = alm(
        &repo,
        &[
            "--as",
            "a",
            "exception",
            "A01-01",
            "--why",
            "waived",
            "--accepted-by",
            "x@example.com",
        ],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("looks like an email address"), "{text}");

    // `alm done --without-human-check --accepted-by` records its own exception.
    let v = ok(
        &repo,
        &[
            "--as",
            "a",
            "done",
            "A01-01",
            "--without-human-check",
            "only the hands-on check is left",
            "--accepted-by",
            "approver",
        ],
    );
    assert_eq!(v["exception"]["kind"], "without_human_check");
    assert_eq!(v["exception"]["accepted_by"], "approver");
    assert_eq!(v["exception"]["why"], "only the hands-on check is left");

    let v = ok(&repo, &["show", "A01-01"]);
    assert_eq!(v["exceptions"].as_array().unwrap().len(), 3);
    let out = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(out.contains("Exceptions:"), "{out}");
    assert!(out.contains("finished without the human check"), "{out}");

    // The project's list.
    let v = ok(&repo, &["exceptions"]);
    assert_eq!(v["exceptions"].as_array().unwrap().len(), 3);
    assert!(
        v["exceptions"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["task"] == "A01-01")
    );
    refused(&repo, &["exceptions", "nope"], 1, "task_not_found");
}

/// Task-A01-stem986 (decision 0021 § 1.2, § 6 item 5): a retention period and a legal hold are
/// set with `alm audit-settings`, each recorded with its reason; while the hold is on, `alm
/// redact` is refused; a redaction leaves an event naming the field and who approved it, without
/// the old value.
#[test]
fn retention_legal_hold_and_redaction() {
    let repo = filled();
    let v = ok(&repo, &["audit-settings"]);
    assert_eq!(v["retention_years"], Value::Null);
    assert_eq!(v["legal_hold"], false);
    // A shorter period and the hold need a reason; a bad period is refused.
    refused(
        &repo,
        &["audit-settings", "--retention", "7"],
        1,
        "audit_reason_missing",
    );
    refused(
        &repo,
        &["audit-settings", "--retention", "6m"],
        1,
        "retention_invalid",
    );
    let v = ok(
        &repo,
        &[
            "audit-settings",
            "--retention",
            "7",
            "--reason",
            "company policy",
        ],
    );
    assert_eq!(v["retention_years"], 7);
    assert_eq!(v["past_retention"]["events"], 0);
    let out = alm(&repo, &["audit-settings"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("Keep the history: 7 years."), "{text}");
    assert!(text.contains("Cybersecurity Law"), "{text}");
    refused(
        &repo,
        &["audit-settings", "--legal-hold", "on"],
        1,
        "audit_reason_missing",
    );
    ok(
        &repo,
        &[
            "audit-settings",
            "--legal-hold",
            "on",
            "--reason",
            "litigation",
        ],
    );
    let project = fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap();
    assert!(project.contains("\"retention_years\": 7"), "{project}");
    assert!(project.contains("\"legal_hold\": true"), "{project}");

    // A personal detail written by mistake, then corrected.
    ok(
        &repo,
        &["describe", "A01-01", "--text", "Ask <PHONE> about it."],
    );
    ok(
        &repo,
        &["describe", "A01-01", "--text", "Ask the approver about it."],
    );
    let redact = [
        "redact",
        "A01-01",
        "--field",
        "description",
        "--reason",
        "a phone number written by mistake",
        "--approved-by",
        "approver",
    ];
    refused(&repo, &redact, 1, "legal_hold_on");
    ok(
        &repo,
        &[
            "audit-settings",
            "--legal-hold",
            "off",
            "--reason",
            "case closed",
        ],
    );
    refused(
        &repo,
        &["redact", "A01-01", "--field", "description"],
        1,
        "redact_reason_missing",
    );
    refused(
        &repo,
        &["redact", "A01-01", "--field", "nickname", "--reason", "x"],
        1,
        "redact_field_unknown",
    );
    refused(
        &repo,
        &[
            "redact",
            "A01-01",
            "--field",
            "description",
            "--event",
            "nope.json",
            "--reason",
            "x",
        ],
        1,
        "redact_event_unknown",
    );
    let v = ok(&repo, &redact);
    assert_eq!(v["changed"], true);
    assert_eq!(v["redaction"]["fields"], json!(["description"]));
    assert_eq!(v["redaction"]["approved_by"], "approver");
    let events: String = fs::read_dir(repo.join(".allomos/taskbag/events"))
        .unwrap()
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert!(!events.contains("PHONE"), "the old value is gone");
    assert!(events.contains("Ask the approver about it."));
    assert!(events.contains("\"command\": \"redact\""));
    assert!(task_file(&repo, "A01-01").is_object());
    // Again: nothing left to remove, nothing recorded.
    let v = ok(&repo, &redact);
    assert_eq!(v["changed"], false);
}

/// The outside witness (Task-A01-wood982): off by default; on only with the person's yes
/// (--confirm), with FreeTSA unless another service is named; off only with a reason.
#[test]
fn the_outside_witness_is_turned_on_with_a_yes_and_off_with_a_reason() {
    let repo = filled();
    let v = ok(&repo, &["audit-settings"]);
    assert_eq!(v["witness"], Value::Null);
    refused(
        &repo,
        &["audit-settings", "--witness", "on"],
        1,
        "witness_not_confirmed",
    );
    refused(
        &repo,
        &["audit-settings", "--witness-service", "freetsa"],
        1,
        "witness_off",
    );
    let v = ok(&repo, &["audit-settings", "--witness", "on", "--confirm"]);
    assert_eq!(v["witness"]["service"], "https://freetsa.org/tsr");
    refused(
        &repo,
        &["audit-settings", "--witness-second", "tsa.example.com"],
        1,
        "witness_service_invalid",
    );
    let v = ok(
        &repo,
        &[
            "audit-settings",
            "--witness-second",
            "https://tsa.example.com/tsr",
        ],
    );
    assert_eq!(v["witness"]["second"], "https://tsa.example.com/tsr");
    refused(
        &repo,
        &["audit-settings", "--witness", "off"],
        1,
        "audit_reason_missing",
    );
    let v = ok(
        &repo,
        &[
            "audit-settings",
            "--witness",
            "off",
            "--reason",
            "trial over",
        ],
    );
    assert_eq!(v["witness"], Value::Null);
    let out = alm(&repo, &["audit-settings"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Outside witness: off."),
        "{out:?}"
    );
}

/// `alm --json` at a given time with the time-stamp service offline (Task-A01-bake791): nothing
/// leaves the test.
fn witness_at(dir: &Path, now: &str, args: &[&str]) -> Value {
    let out = alm_command()
        .current_dir(dir)
        .arg("--json")
        .args(args)
        .env_remove("ALLOMOS_TASKBAG")
        .env_remove("ALLOMOS_HOLDER")
        .env_remove("ALLOMOS_LOCK_WAIT_MS")
        .env("ALLOMOS_NOW", now)
        .env("ALLOMOS_TSA_REPLY", "offline")
        .output()
        .unwrap();
    let v: Value = serde_json::from_str(String::from_utf8(out.stdout).unwrap().trim()).unwrap();
    assert_eq!(v["ok"], true, "{args:?}: {v}");
    v
}

fn checkpoint_count(repo: &Path) -> usize {
    fs::read_dir(repo.join(".allomos/taskbag/checkpoints"))
        .map(|d| d.count())
        .unwrap_or(0)
}

/// The outside witness's "how often" (Task-A01-bake791): daily is chosen when it is turned on; a
/// change a day after the last checkpoint makes one (listed to share, its fingerprint waiting
/// while offline, the command ending as usual), the next change that day none; at approvals, an
/// `alm checked` makes one; only when asked, nothing by itself.
#[test]
fn the_outside_witness_makes_checkpoints_by_itself_as_often_as_set() {
    let repo = filled();
    let t0 = "2026-11-01T09:00:00Z";
    let v = witness_at(
        &repo,
        t0,
        &["audit-settings", "--witness", "on", "--confirm"],
    );
    assert_eq!(v["witness"]["when"], "daily");
    assert_eq!(checkpoint_count(&repo), 0, "turning it on makes none");
    witness_at(&repo, t0, &["checkpoint"]);
    assert_eq!(checkpoint_count(&repo), 1);

    // The same day: no more.
    let v = witness_at(&repo, "2026-11-01T18:00:00Z", &["note", "A01-01", "later"]);
    assert_eq!(checkpoint_count(&repo), 1);
    assert!(v["auto_checkpoint"]["made"].is_null(), "{v}");

    // A day after the last one: a checkpoint, listed with the change's files; offline, it waits.
    let v = witness_at(
        &repo,
        "2026-11-02T09:00:00Z",
        &["note", "A01-01", "next day"],
    );
    assert_eq!(checkpoint_count(&repo), 2);
    let made = v["auto_checkpoint"]["made"]["file"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(v["auto_checkpoint"]["when"], "daily");
    let files: Vec<&str> = v["changed_files"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(files.contains(&made.as_str()), "{files:?}");
    assert!(
        !v["auto_checkpoint"]["witnessed"]["waiting"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    // Run again: no more.
    witness_at(&repo, "2026-11-02T09:05:00Z", &["note", "A01-01", "again"]);
    assert_eq!(checkpoint_count(&repo), 2);

    // At each approval and human check: a change makes none, `alm checked` one.
    let v = witness_at(
        &repo,
        "2026-11-02T10:00:00Z",
        &["audit-settings", "--witness-when", "approvals"],
    );
    assert_eq!(v["witness"]["when"], "approvals");
    witness_at(
        &repo,
        "2026-11-04T10:00:00Z",
        &["--as", "b", "claim", "A01-01"],
    );
    witness_at(
        &repo,
        "2026-11-04T10:01:00Z",
        &["--as", "b", "done", "A01-01", "--without-owner", "test"],
    );
    assert_eq!(checkpoint_count(&repo), 2);
    let v = witness_at(
        &repo,
        "2026-11-04T10:02:00Z",
        &["--as", "will", "checked", "A01-01"],
    );
    assert_eq!(checkpoint_count(&repo), 3);
    assert_eq!(v["auto_checkpoint"]["when"], "approvals");

    // Only when asked: nothing by itself, even days later.
    witness_at(
        &repo,
        "2026-11-04T11:00:00Z",
        &["audit-settings", "--witness-when", "asked"],
    );
    witness_at(
        &repo,
        "2026-11-09T11:00:00Z",
        &["note", "A01-02", "much later"],
    );
    assert_eq!(checkpoint_count(&repo), 3);

    // An unknown value is refused; the audit still reads every checkpoint.
    refused(
        &repo,
        &["audit-settings", "--witness-when", "hourly"],
        2,
        "bad_usage",
    );
    let out = alm(&repo, &["audit-settings"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("How often: only when asked"),
        "{out:?}"
    );
    let v = ok(&repo, &["audit"]);
    assert_eq!(v["audit"]["checkpoints"].as_array().unwrap().len(), 3);
}

/// `alm --json …` at a given time (`ALLOMOS_NOW`): the exit code and the JSON object.
fn fj_at(dir: &Path, now: &str, args: &[&str]) -> (i32, Value) {
    let mut all = vec!["--json"];
    all.extend_from_slice(args);
    let out = alm_at(dir, now, &all);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    (out.status.code().unwrap(), v)
}

fn ok_at(dir: &Path, now: &str, args: &[&str]) -> Value {
    let (code, v) = fj_at(dir, now, args);
    assert_eq!((code, &v["ok"]), (0, &Value::Bool(true)), "{args:?}: {v}");
    v
}

/// What a session is doing, and pushing work in progress (Task-A01-butter895): `--doing` goes in the
/// heartbeat and no event; `alm next`, `alm show` and a takeover say it with its time; `alm beat`
/// prints the commands to push work in progress while the project's setting is on, and not after
/// it is turned off; the claim says only what was pushed counts.
#[test]
fn beats_say_what_a_session_is_doing_and_remind_to_push_work() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    let t0 = "2026-11-09T14:00:00Z";
    ok_at(
        &repo,
        t0,
        &[
            "--as",
            "s1",
            "claim",
            "A01-01",
            "--branch",
            "main",
            "--working",
            "claude/x",
        ],
    );
    let events = event_count(&repo);

    // New words are written at once, even under half the interval, and no event records them.
    let t1 = "2026-11-09T14:01:00Z";
    let v = ok_at(
        &repo,
        t1,
        &["--as", "s1", "beat", "--doing", "write the dialog tests"],
    );
    assert_eq!(v["beats"][0]["written"], true, "{v}");
    assert_eq!(event_count(&repo), events);
    assert_eq!(
        task_file(&repo, "A01-01")["heartbeat"],
        json!({ "at": t1, "by": "s1", "doing": "write the dialog tests", "doing_at": t1 })
    );

    // The reminder: the commands on the working branch.
    assert_eq!(v["push_work"][0]["id"], "A01-01", "{v}");
    assert_eq!(v["push_work"][0]["working_branch"], "claude/x");
    assert_eq!(
        v["push_work"][0]["git_commands"],
        json!([
            "git add -A",
            "git commit -m \"WIP A01-01: <what you did>\" -m \"Agent: s1\"",
            "git push origin claude/x",
        ])
    );
    // Also on a beat too soon to write: an automatic heartbeat may have written the last one.
    let v = ok_at(&repo, t1, &["--as", "s1", "beat"]);
    assert_eq!(v["beats"][0]["written"], false, "{v}");
    assert_eq!(v["push_work"][0]["id"], "A01-01", "{v}");

    // alm next and alm show say it, with its time.
    let v = ok_at(&repo, "2026-11-09T14:05:00Z", &["next"]);
    assert_eq!(
        v["doing"],
        json!([{ "id": "A01-01", "holder": "s1", "doing": "write the dialog tests", "doing_at": t1 }]),
        "{v}"
    );
    assert_eq!(v["heartbeat_settings"]["push_work"], true);
    let out = alm_at(&repo, "2026-11-09T14:05:00Z", &["next"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Doing: A01-01 is held by s1: write the dialog tests (said 4 min ago"),
        "{text}"
    );
    let text = String::from_utf8_lossy(&alm(&repo, &["show", "A01-01"]).stdout).into_owned();
    assert!(
        text.contains("doing: write the dialog tests (said"),
        "{text}"
    );
    assert_eq!(
        ok(&repo, &["show", "A01-01"])["task"]["heartbeat"]["doing_at"],
        t1
    );

    // Lost: alm next says what it was doing; the takeover repeats it and says only pushes count.
    let later = "2026-11-09T16:00:00Z";
    let v = ok_at(&repo, later, &["next"]);
    assert_eq!(v["lost"][0]["doing"], "write the dialog tests", "{v}");
    assert_eq!(v["lost"][0]["doing_at"], t1);
    let v = ok_at(
        &repo,
        later,
        &["--as", "s2", "claim", "A01-01", "--branch", "main"],
    );
    assert_eq!(v["took_over"]["doing"], "write the dialog tests", "{v}");
    assert_eq!(v["took_over"]["doing_at"], t1);
    // The words were the former holder's: the new holder starts without them.
    assert_eq!(
        task_file(&repo, "A01-01")["heartbeat"],
        json!({ "at": later, "by": "s2" })
    );

    // Turned off: written in project.json, and alm beat no longer reminds.
    let v = ok(&repo, &["heartbeat-settings", "--wip", "off"]);
    assert_eq!(v["heartbeat"], json!({ "push_work": false }), "{v}");
    assert_eq!(v["applied"]["push_work"], false);
    let v = ok_at(
        &repo,
        "2026-11-09T16:10:00Z",
        &["--as", "s2", "beat", "--doing", "go on"],
    );
    assert_eq!(v["beats"][0]["written"], true, "{v}");
    assert_eq!(v["push_work"], json!([]), "{v}");
    let text = String::from_utf8_lossy(&alm(&repo, &["heartbeat-settings"]).stdout).into_owned();
    assert!(
        text.contains("Push work in progress at each heartbeat: off"),
        "{text}"
    );

    // On again: the default, so nothing is written.
    let v = ok(&repo, &["heartbeat-settings", "--wip", "on"]);
    assert_eq!(v["heartbeat"], Value::Null, "{v}");
    let text = String::from_utf8_lossy(&alm(&repo, &["heartbeat-settings"]).stdout).into_owned();
    assert!(
        text.contains("Push work in progress at each heartbeat: on (default)"),
        "{text}"
    );
}

/// Heartbeats (decision 0022, Task-A01-nurse164): `alm beat` writes the holder's last heartbeat
/// into the task's file and no event; under half the interval it writes nothing; `--busy` records a
/// busy window; with no ID it beats for every task the session holds; the holder's changes beat
/// along with them, reading does not; and a beat leaves the task's version as it is.
#[test]
fn holders_beat_and_their_changes_beat_along() {
    let repo = filled();
    for id in ["A01-01", "A01-02", "A02-01"] {
        ok(&repo, &["ready", id]);
    }
    let t0 = "2026-11-09T14:00:00Z";
    ok_at(&repo, t0, &["--as", "s1", "claim", "A01-01"]);
    ok_at(&repo, t0, &["--as", "s1", "claim", "A01-02"]);
    ok_at(&repo, t0, &["--as", "s2", "claim", "A02-01"]);
    // A claim is the first heartbeat.
    assert_eq!(
        task_file(&repo, "A01-01")["heartbeat"],
        json!({ "at": t0, "by": "s1" })
    );
    let events = event_count(&repo);
    let version = ok(&repo, &["show", "A01-01"])["task"]["version"].clone();

    // Under half the interval (5 of 10 minutes): nothing written, and it says when the next is due.
    let v = ok_at(
        &repo,
        "2026-11-09T14:04:59Z",
        &["--as", "s1", "beat", "A01-01"],
    );
    assert_eq!(v["beats"][0]["written"], false);
    assert_eq!(v["beats"][0]["ago_seconds"], 299);
    assert_eq!(v["beats"][0]["due"], "2026-11-09T14:10:00Z");
    assert_eq!(v["changed_files"], json!([]), "{v}");
    assert_eq!(task_file(&repo, "A01-01")["heartbeat"]["at"], t0);

    // From half the interval on, it is written: no event, and the version stays.
    let t1 = "2026-11-09T14:05:00Z";
    let v = ok_at(&repo, t1, &["--as", "s1", "beat", "A01-01"]);
    assert_eq!(v["beats"][0]["written"], true);
    assert_eq!(v["beats"][0]["due"], "2026-11-09T14:15:00Z");
    assert_eq!(task_file(&repo, "A01-01")["heartbeat"]["at"], t1);
    assert_eq!(event_count(&repo), events);
    assert_eq!(ok(&repo, &["show", "A01-01"])["task"]["version"], version);

    // A busy window is written even right after a beat, and shows.
    let v = ok_at(
        &repo,
        "2026-11-09T14:06:00Z",
        &["--as", "s1", "beat", "A01-01", "--busy", "2h"],
    );
    assert_eq!(
        v["beats"][0]["heartbeat"]["busy_until"],
        "2026-11-09T16:06:00Z"
    );
    let out = alm_at(&repo, "2026-11-09T14:07:00Z", &["show", "A01-01"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains(
            "heartbeat: 1 min ago (2026-11-09T14:06:00Z, by s1); busy until 2026-11-09T16:06:00Z"
        ),
        "{text}"
    );
    assert_eq!(event_count(&repo), events);

    // With no ID: every task the session holds, in one change; another session's are left alone.
    let t2 = "2026-11-09T14:20:00Z";
    let v = ok_at(&repo, t2, &["--as", "s1", "beat"]);
    let ids: Vec<&str> = v["beats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A01-01", "A01-02"]);
    assert_eq!(v["changed_files"].as_array().unwrap().len(), 2, "{v}");
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t2);
    // The busy window still running is kept.
    assert_eq!(
        task_file(&repo, "A01-01")["heartbeat"]["busy_until"],
        "2026-11-09T16:06:00Z"
    );
    assert_eq!(task_file(&repo, "A02-01")["heartbeat"]["at"], t0);
    // `0m` ends it.
    ok_at(&repo, t2, &["--as", "s1", "beat", "A01-01", "--busy", "0m"]);
    assert!(
        task_file(&repo, "A01-01")["heartbeat"]
            .get("busy_until")
            .is_none()
    );

    // Reading writes nothing.
    let t3 = "2026-11-09T14:40:00Z";
    for args in [
        &["--as", "s1", "show", "A01-02"][..],
        &["--as", "s1", "list"],
        &["--as", "s1", "next"],
    ] {
        ok_at(&repo, t3, args);
    }
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t2);
    // The holder's changes beat along, with the event they make anyway.
    ok_at(
        &repo,
        t3,
        &["--as", "s1", "label", "A01-02", "locally verified by agent"],
    );
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t3);
    let t4 = "2026-11-09T14:41:00Z";
    ok_at(&repo, t4, &["--as", "s1", "note", "A01-02", "next: tests"]);
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t4);
    let t5 = "2026-11-09T14:42:00Z";
    ok_at(
        &repo,
        t5,
        &[
            "--as",
            "s1",
            "handoff",
            "A01-02",
            "--text",
            "stopped at tests",
        ],
    );
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t5);
    let t6 = "2026-11-09T14:43:00Z";
    ok_at(
        &repo,
        t6,
        &["--as", "s1", "ai-decided", "A01-02", "named it x"],
    );
    assert_eq!(task_file(&repo, "A01-02")["heartbeat"]["at"], t6);
    // …and no event records a heartbeat.
    let v = ok(&repo, &["history", "A01-02"]);
    for e in v["events"].as_array().unwrap() {
        for c in e["changes"].as_array().unwrap() {
            assert_ne!(c["field"], "heartbeat", "{e}");
        }
    }
    // A released task has no heartbeat.
    ok_at(
        &repo,
        t6,
        &["--as", "s1", "release", "A01-02", "--note", "n"],
    );
    assert!(task_file(&repo, "A01-02").get("heartbeat").is_none());

    // Refused: no session name, a task the session does not hold, a task nobody holds, a bad length.
    refused(&repo, &["beat", "A01-01"], 1, "holder_missing");
    refused(&repo, &["--as", "s1", "beat", "A02-01"], 1, "task_held");
    refused(&repo, &["--as", "s1", "beat", "A01-02"], 1, "not_held");
    refused(
        &repo,
        &["--as", "s1", "beat", "A01-01", "--busy", "soon"],
        1,
        "busy_invalid",
    );
    // A session holding nothing: nothing written.
    let v = ok(&repo, &["--as", "s9", "beat"]);
    assert_eq!(v["beats"], json!([]));
    let (_, v) = fj(&repo, &["list"]);
    assert!(v.get("issues").is_none_or(|i| i == &json!([])), "{v}");
}

/// Lost tasks (decision 0022 § 3 and § 4, Task-A01-zero387): `alm next` lists a held task with no
/// heartbeat for longer than the limit and no busy window running under "Lost, can be taken
/// over", with who holds it, since when, the limit and the command to take it over (also in
/// `--json`); it shows busy windows; waiting, blocked and released tasks are never lost; the two
/// numbers are project settings; and `alm` takes nothing over itself.
#[test]
fn next_lists_lost_tasks_and_busy_windows() {
    let repo = filled();
    for id in ["A01-01", "A01-02", "A02-01"] {
        ok(&repo, &["ready", id]);
    }
    let t0 = "2026-11-09T14:00:00Z";
    ok_at(
        &repo,
        t0,
        &["--as", "s1", "claim", "A01-01", "--branch", "claude/one"],
    );
    ok_at(&repo, t0, &["--as", "s2", "claim", "A01-02"]);
    ok_at(&repo, t0, &["--as", "s3", "claim", "A02-01"]);
    ok_at(&repo, t0, &["--as", "s2", "beat", "A01-02", "--busy", "2h"]);
    // A02-01 waits: no holder, never lost.
    ok_at(
        &repo,
        t0,
        &["--as", "s3", "wait", "A02-01", "--on", "the approver"],
    );

    // At the limit (30 minutes): not lost yet; the busy window shows.
    let v = ok_at(&repo, "2026-11-09T14:30:00Z", &["next"]);
    assert_eq!(v["lost"], json!([]), "{v}");
    assert_eq!(v["busy"][0]["id"], "A01-02");
    assert_eq!(v["busy"][0]["busy_until"], "2026-11-09T16:00:00Z");
    assert_eq!(v["heartbeat_settings"]["lost_after_secs"], 1800);

    // Past it: A01-01 is lost; A01-02 is busy; nothing changes.
    let events = event_count(&repo);
    let now = "2026-11-09T14:45:00Z";
    let v = ok_at(&repo, now, &["next"]);
    let lost = v["lost"].as_array().unwrap();
    assert_eq!(lost.len(), 1, "{v}");
    assert_eq!(lost[0]["id"], "A01-01");
    assert_eq!(lost[0]["holder"], "s1");
    assert_eq!(lost[0]["heartbeat_at"], t0);
    assert_eq!(lost[0]["silent_seconds"], 2700);
    assert_eq!(lost[0]["lost_after_seconds"], 1800);
    assert_eq!(lost[0]["working_branch"], "claude/one");
    assert_eq!(
        lost[0]["take_over"],
        "alm claim A01-01 --as <your session name> --branch claude/one"
    );
    assert!(
        v.get("changed_files")
            .is_none_or(|f| f.as_array().is_some_and(Vec::is_empty)),
        "{v}"
    );
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01")["holder"], "s1");
    let text = String::from_utf8(alm_at(&repo, now, &["next"]).stdout).unwrap();
    assert!(text.contains("Lost, can be taken over"), "{text}");
    assert!(
        text.contains(
            "held by s1; last heartbeat 45 min ago (2026-11-09T14:00:00Z); lost after 30 min"
        ),
        "{text}"
    );
    assert!(
        text.contains("Busy: A01-02 is held by s2, busy until 2026-11-09T16:00:00Z"),
        "{text}"
    );

    // After the busy window, the limit counts from its end.
    let v = ok_at(&repo, "2026-11-09T16:30:00Z", &["next"]);
    let ids: Vec<&str> = v["lost"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["A01-01"]);
    let v = ok_at(&repo, "2026-11-09T16:31:00Z", &["next"]);
    assert_eq!(v["lost"][1]["id"], "A01-02");
    assert_eq!(v["lost"][1]["since"], "2026-11-09T16:00:00Z");
    assert_eq!(v["busy"], json!([]));

    // A released task is never lost.
    ok_at(
        &repo,
        "2026-11-09T16:31:00Z",
        &["--as", "s2", "release", "A01-02", "--note", "n"],
    );
    let v = ok_at(&repo, "2026-11-09T18:00:00Z", &["next"]);
    assert_eq!(v["lost"].as_array().unwrap().len(), 1, "{v}");

    // The two numbers are project settings.
    let v = ok(&repo, &["heartbeat-settings"]);
    assert_eq!(
        v["applied"],
        json!({ "interval_secs": 600, "lost_after_secs": 1800, "push_work": true })
    );
    let v = ok(
        &repo,
        &[
            "heartbeat-settings",
            "--interval",
            "20m",
            "--lost-after",
            "4h",
        ],
    );
    assert_eq!(
        v["heartbeat"],
        json!({ "interval_minutes": 20, "lost_after_minutes": 240 })
    );
    assert!(
        v["changed_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "project.json"),
        "{v}"
    );
    let v = ok_at(&repo, "2026-11-09T18:00:00Z", &["next"]);
    assert_eq!(v["lost"], json!([]), "{v}");
    let v = ok_at(&repo, "2026-11-09T18:01:00Z", &["next"]);
    assert_eq!(v["lost"][0]["lost_after_seconds"], 14_400);
    // The interval sets when `alm beat` writes again and when the next one is due.
    let v = ok_at(
        &repo,
        "2026-11-09T18:01:00Z",
        &["--as", "s1", "beat", "A01-01"],
    );
    assert_eq!(v["beats"][0]["due"], "2026-11-09T18:21:00Z");
    let v = ok_at(
        &repo,
        "2026-11-09T18:10:59Z",
        &["--as", "s1", "beat", "A01-01"],
    );
    assert_eq!(v["beats"][0]["written"], false);
    refused(
        &repo,
        &["heartbeat-settings", "--lost-after", "0m"],
        1,
        "heartbeat_minutes_invalid",
    );
    refused(
        &repo,
        &["heartbeat-settings", "--interval", "90s"],
        1,
        "heartbeat_minutes_invalid",
    );
    let v = ok(&repo, &["heartbeat-settings", "--clear"]);
    assert_eq!(v["heartbeat"], Value::Null);
    let project: Value = serde_json::from_str(
        &fs::read_to_string(repo.join(".allomos/taskbag/project.json")).unwrap(),
    )
    .unwrap();
    assert!(project.get("heartbeat").is_none());
    let (_, v) = fj(&repo, &["list"]);
    assert!(v.get("issues").is_none_or(|i| i == &json!([])), "{v}");
}

/// Takeover (decision 0022 § 6 and § 7, Task-A01-sign613): a plain `alm claim` takes over a lost
/// task, records the former holder in the claim's event, keeps the working branch and prints the
/// commands to continue on it and where the former holder worked; a task that is not lost is
/// still refused; the new holder beats and can be taken over in turn.
#[test]
fn claim_takes_over_a_lost_task() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    let wt = repo.join("wt-s1");
    std::fs::create_dir_all(&wt).unwrap();
    let t0 = "2026-11-09T10:00:00Z";
    ok_at(
        &wt,
        t0,
        &[
            "--as",
            "s1",
            "claim",
            "A01-01",
            "--branch",
            "main",
            "--working",
            "claude/one",
            "--session-agent",
            "claude-code",
            "--session-id",
            "x",
        ],
    );

    // Not lost yet: refused as before, and nothing changes.
    let events = event_count(&repo);
    let (code, v) = fj_at(
        &repo,
        "2026-11-09T10:30:00Z",
        &["--as", "s2", "claim", "A01-01", "--branch", "main"],
    );
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (1, Some("task_held")),
        "{v}"
    );
    assert_eq!(event_count(&repo), events);

    // Lost: the claim takes it over.
    let t1 = "2026-11-09T11:00:00Z";
    let v = ok_at(
        &repo,
        t1,
        &["--as", "s2", "claim", "A01-01", "--branch", "main"],
    );
    assert_eq!(v["warnings"][0]["code"], "took_over_lost", "{v}");
    assert_eq!(v["warnings"][0]["params"]["previous"], "s1");
    assert_eq!(v["warnings"][0]["params"]["since"], t0);
    assert_eq!(v["took_over"]["from"], "s1");
    assert_eq!(v["took_over"]["lost"]["silent_seconds"], 3600);
    assert_eq!(v["took_over"]["folder"], "wt-s1");
    assert!(
        v["took_over"]["folder_path"]
            .as_str()
            .unwrap()
            .ends_with("wt-s1"),
        "{v}"
    );
    // Work goes on on the original working branch, from the branch that exists.
    assert_eq!(v["branches"]["working"], "claude/one");
    assert_eq!(
        v["branch_commands"]["start"],
        json!([
            "git switch main",
            "git pull --ff-only",
            "git worktree add <folder> claude/one"
        ])
    );
    let task = task_file(&repo, "A01-01");
    assert_eq!(task["holder"], "s2");
    assert_eq!(task["heartbeat"]["by"], "s2");
    assert_eq!(task["heartbeat"]["at"], t1);
    // The claim's event records whom it was taken from, and when.
    let h = ok(&repo, &["history", "A01-01"]);
    // Found by its holder, not its place: `ready` ran on the real clock, after this fixed date.
    let e = h["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["command"] == "claim" && e["by"] == "s2")
        .unwrap();
    assert_eq!(e["at"].as_str(), Some(t1));
    assert_eq!(
        e["changes"][0],
        json!({ "field": "holder", "from": "s1", "to": "s2" })
    );
    let text = String::from_utf8(
        alm_at(
            &repo,
            t1,
            &[
                "--as",
                "s2",
                "claim",
                "A01-01",
                "--branch",
                "main",
                "--working",
                "claude/one",
            ],
        )
        .stdout,
    )
    .unwrap();
    assert!(
        !text.contains("taken over from"),
        "same holder again: {text}"
    );

    // The new holder is an ordinary holder: its heartbeat keeps the task, its silence loses it.
    ok_at(
        &repo,
        "2026-11-09T11:20:00Z",
        &["--as", "s2", "beat", "A01-01"],
    );
    refused_at(
        &repo,
        "2026-11-09T11:40:00Z",
        &["--as", "s3", "claim", "A01-01", "--branch", "main"],
        "task_held",
    );
    let v = ok_at(
        &repo,
        "2026-11-09T12:00:00Z",
        &["--as", "s3", "claim", "A01-01", "--branch", "main"],
    );
    assert_eq!(v["warnings"][0]["code"], "took_over_lost", "{v}");
    assert_eq!(v["took_over"]["from"], "s2");
    assert_eq!(v["branches"]["working"], "claude/one");
    let h = ok(&repo, &["history", "A01-01"]);
    let holders: Vec<&str> = h["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["command"] == "claim")
        .filter_map(|e| {
            e["changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["field"] == "holder")
        })
        .map(|c| c["to"].as_str().unwrap())
        .collect();
    assert_eq!(holders, ["s3", "s2", "s1"]);
    // The text tells about the former holder's worktree.
    ok_at(
        &repo,
        "2026-11-09T12:00:00Z",
        &["--as", "s1", "claim", "A01-02", "--branch", "main"],
    );
    let out = alm_at(
        &repo,
        "2026-11-09T13:00:00Z",
        &["--as", "s4", "claim", "A01-02", "--branch", "main"],
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("taken over from: s1 (lost)"), "{text}");
    assert!(
        text.contains("do not change, commit or remove what is there"),
        "{text}"
    );
}

/// A session coming back (decision 0022 § 8, Task-A01-castle658): with `--as`, every command
/// first names the tasks taken over from it (by whom, when), read-only ones too; its changes to
/// them are refused (`task_held`), claiming back included; other tasks and other sessions are
/// not affected, and the notice ends once the task is no longer held by someone else.
#[test]
fn a_session_coming_back_is_told_of_the_takeover_and_refused() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    ok(&repo, &["ready", "A01-02"]);
    let claim = |who: &str, id: &str, at: &str| {
        ok_at(
            &repo,
            at,
            &[
                "--as",
                who,
                "claim",
                id,
                "--branch",
                "main",
                "--working",
                "claude/one",
            ],
        )
    };
    claim("s1", "A01-01", "2026-11-09T10:00:00Z");
    let t1 = "2026-11-09T11:00:00Z";
    claim("s2", "A01-01", t1);
    let later = "2026-11-09T11:05:00Z";

    // Read-only commands: the notice comes first, in the text and in --json.
    for args in [
        vec!["--as", "s1", "show", "A01-01"],
        vec!["--as", "s1", "list"],
        vec!["--as", "s1", "next"],
    ] {
        let v = ok_at(&repo, later, &args);
        assert_eq!(v["warnings"][0]["code"], "taken_over", "{args:?}: {v}");
        assert_eq!(
            v["warnings"][0]["params"],
            json!({ "id": "A01-01", "by": "s2", "at": t1 })
        );
        assert_eq!(
            v["taken_over"],
            json!([{ "id": "A01-01", "by": "s2", "at": t1, "holder": "s2" }])
        );
        let out = alm_at(&repo, later, &args);
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(
            text.starts_with(&format!(
                "notice [taken_over]: Task A01-01 was taken over by s2 at {t1}: you no longer hold it. Stop work on it"
            )),
            "{args:?}: {text}"
        );
    }

    // Changes to the task are refused, the takeover named; nothing is written.
    let events = event_count(&repo);
    for args in [
        vec!["--as", "s1", "label", "A01-01", "locally verified"],
        vec!["--as", "s1", "note", "A01-01", "still here"],
        vec!["--as", "s1", "beat", "A01-01"],
        vec!["--as", "s1", "release", "A01-01", "--note", "n"],
        vec![
            "--as", "s1", "claim", "A01-01", "--branch", "main", "--take",
        ],
    ] {
        let (code, v) = fj_at(&repo, later, &args);
        assert_eq!(
            (
                code,
                v["error"]["code"].as_str(),
                v["error"]["params"]["holder"].as_str()
            ),
            (1, Some("task_held"), Some("s2")),
            "{args:?}: {v}"
        );
        assert_eq!(v["taken_over"][0]["by"], "s2", "{args:?}: {v}");
        let out = alm_at(&repo, later, &args);
        let err = String::from_utf8(out.stderr).unwrap();
        assert!(
            err.starts_with("notice [taken_over]: Task A01-01 was taken over by s2")
                && err.contains("error [task_held]: Task A01-01 is held by s2."),
            "{args:?}: {err}"
        );
    }
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01")["holder"], "s2");

    // Its other work goes on, still told first; the new holder and other sessions are told nothing.
    let v = claim("s1", "A01-02", later);
    assert_eq!(v["warnings"][0]["code"], "taken_over", "{v}");
    let v = ok_at(
        &repo,
        later,
        &["--as", "s2", "label", "A01-01", "locally verified"],
    );
    assert!(v.get("taken_over").is_none(), "{v}");
    let v = ok_at(&repo, later, &["list"]);
    assert!(v.get("taken_over").is_none(), "{v}");

    // A chain: s3 takes it from s2; s1 is still told, and who holds it now.
    claim("s3", "A01-01", "2026-11-09T12:00:00Z");
    let v = ok_at(&repo, "2026-11-09T12:05:00Z", &["--as", "s1", "list"]);
    assert_eq!(
        v["taken_over"],
        json!([{ "id": "A01-01", "by": "s2", "at": t1, "holder": "s3" }])
    );
    let text =
        String::from_utf8(alm_at(&repo, "2026-11-09T12:05:00Z", &["--as", "s1", "list"]).stdout)
            .unwrap();
    assert!(text.contains("It is held by s3 now."), "{text}");

    // Released by its holder: nobody holds it, so no notice, and s1 may claim it like anyone.
    ok_at(
        &repo,
        "2026-11-09T12:10:00Z",
        &[
            "--as", "s3", "release", "A01-01", "--note", "n", "--why", "w",
        ],
    );
    let v = ok_at(&repo, "2026-11-09T12:15:00Z", &["--as", "s1", "list"]);
    assert!(v.get("taken_over").is_none(), "{v}");
    claim("s1", "A01-01", "2026-11-09T12:20:00Z");
    let v = ok_at(&repo, "2026-11-09T12:25:00Z", &["--as", "s1", "list"]);
    assert!(v.get("taken_over").is_none(), "{v}");
}

/// Task-A01-heart358: the takeover notice and refusal last after the taker has finished the task
/// (done or dropped); and `alm release` of a finished task is refused for every session
/// (`cannot_release`, pointing to `alm reopen`), so it can no longer erase who finished it.
#[test]
fn a_session_coming_back_is_refused_after_the_taker_finished() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    ok(&repo, &["ready", "A01-02"]);
    let claim = |who: &str, id: &str, at: &str| {
        ok_at(
            &repo,
            at,
            &[
                "--as",
                who,
                "claim",
                id,
                "--branch",
                "main",
                "--working",
                "claude/one",
            ],
        )
    };
    let t1 = "2026-11-09T11:00:00Z";
    for id in ["A01-01", "A01-02"] {
        claim("s1", id, "2026-11-09T10:00:00Z");
        claim("s2", id, t1);
    }
    let later = "2026-11-09T11:30:00Z";
    ok_at(
        &repo,
        "2026-11-09T11:10:00Z",
        &["--as", "s2", "label", "A01-01", "locally verified by human"],
    );
    ok_at(
        &repo,
        "2026-11-09T11:15:00Z",
        &["--as", "s2", "done", "A01-01"],
    );
    ok_at(
        &repo,
        "2026-11-09T11:20:00Z",
        &["--as", "s2", "drop", "A01-02"],
    );

    // Both are named first, finished, by whom.
    let v = ok_at(&repo, later, &["--as", "s1", "list"]);
    assert_eq!(
        v["taken_over"],
        json!([
            { "id": "A01-01", "by": "s2", "at": t1, "holder": "s2", "finished": "done" },
            { "id": "A01-02", "by": "s2", "at": t1, "holder": "s2", "finished": "dropped" },
        ]),
        "{v}"
    );
    let text = String::from_utf8(alm_at(&repo, later, &["--as", "s1", "list"]).stdout).unwrap();
    assert!(
        text.starts_with("notice [taken_over]: Task A01-01 was taken over by s2")
            && text.contains("It is done now, finished by s2.")
            && text.contains("It is dropped now, finished by s2."),
        "{text}"
    );

    // The old holder's changes are refused; nothing is written.
    let events = event_count(&repo);
    for id in ["A01-01", "A01-02"] {
        for args in [
            vec!["--as", "s1", "note", id, "still here"],
            vec!["--as", "s1", "label", id, "locally verified by agent"],
            vec!["--as", "s1", "release", id, "--note", "n"],
        ] {
            let (code, v) = fj_at(&repo, later, &args);
            assert_eq!(
                (code, v["error"]["code"].as_str()),
                (1, Some("task_held")),
                "{args:?}: {v}"
            );
            assert_eq!(v["taken_over"][0]["by"], "s2", "{args:?}: {v}");
        }
    }
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01")["holder"], "s2");

    // Releasing a finished task is refused for everyone else too: the taker and anyone.
    for args in [
        vec!["--as", "s2", "release", "A01-01", "--note", "n"],
        vec!["--as", "s9", "release", "A01-02"],
        vec!["release", "A01-01"],
    ] {
        let (code, v) = fj_at(&repo, later, &args);
        assert_eq!(
            (code, v["error"]["code"].as_str()),
            (1, Some("cannot_release")),
            "{args:?}: {v}"
        );
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("alm reopen"),
            "{v}"
        );
    }
    assert_eq!(event_count(&repo), events);
    assert_eq!(task_file(&repo, "A01-01")["holder"], "s2");

    // Other sessions are told nothing; reopened, nobody holds it and the notice ends.
    let v = ok_at(&repo, later, &["--as", "s2", "list"]);
    assert!(v.get("taken_over").is_none(), "{v}");
    ok_at(&repo, later, &["reopen", "A01-02", "--note", "more to do"]);
    let v = ok_at(&repo, later, &["--as", "s1", "list"]);
    assert_eq!(v["taken_over"].as_array().unwrap().len(), 1, "{v}");
    assert_eq!(v["taken_over"][0]["id"], "A01-01", "{v}");
}

fn refused_at(dir: &Path, now: &str, args: &[&str], problem: &str) {
    let (code, v) = fj_at(dir, now, args);
    assert_eq!(
        (code, v["error"]["code"].as_str()),
        (1, Some(problem)),
        "{args:?}: {v}"
    );
}

/// Two sessions claim the same ready task at once, each in its own clone of the taskbag's
/// repository (Task-A01-lawn320). The first push wins; the second is rejected. After
/// `git pull --rebase` the loser's copy holds both claims in conflict markers, and every `alm`
/// command first says the task is the winner's, with the next ready task. Once the loser drops
/// its claim (`git rebase --skip`), the task shows the winner, and claiming it again is refused
/// with the same pointer.
#[test]
fn two_sessions_claiming_one_task_at_once() {
    let base = tmp("race");
    let git = |dir: &Path, args: &[&str]| {
        Command::new("git")
            .current_dir(dir)
            .args(["-c", "user.name=t", "-c", "user.email=t@example.invalid"])
            .args([
                "-c",
                "init.defaultBranch=main",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .output()
    };
    match git(&base, &["init", "-q", "--bare", "remote.git"]) {
        Ok(o) if o.status.success() => {}
        _ => return, // No Git on this machine: nothing to check.
    }
    let run = |dir: &Path, args: &[&str]| {
        let o = git(dir, args).unwrap();
        assert!(o.status.success(), "git {args:?}: {o:?}");
    };
    let a = base.join("a");
    let b = base.join("b");
    run(&base, &["clone", "-q", "remote.git", "a"]);
    ok(&a, &["init", "--name", "Demo"]);
    ok(&a, &["add", "area", "A01", "One"]);
    for (id, title) in [("A01-01", "First"), ("A01-02", "Second")] {
        ok(&a, &["add", "task", id, "--area", "A01", "--title", title]);
        ok(&a, &["ready", id]);
    }
    run(&a, &["add", "-A"]);
    run(&a, &["commit", "-qm", "start"]);
    run(&a, &["push", "-q", "origin", "HEAD:main"]);
    run(&base, &["clone", "-q", "-b", "main", "remote.git", "b"]);

    let claim = |dir: &Path, who: &str| {
        ok(dir, &["--as", who, "claim", "A01-01", "--branch", "main"]);
        run(dir, &["add", "-A"]);
        run(dir, &["commit", "-qm", "claim"]);
    };
    claim(&a, "s1");
    claim(&b, "s2");
    run(&a, &["push", "-q", "origin", "HEAD:main"]);
    // The second push is rejected.
    assert!(
        !git(&b, &["push", "-q", "origin", "HEAD:main"])
            .unwrap()
            .status
            .success()
    );
    // Getting the latest changes: the task's file holds both claims.
    assert!(
        !git(&b, &["pull", "-q", "--rebase", "origin", "main"])
            .unwrap()
            .status
            .success()
    );

    // Every command says so first: as the loser, the task is s1's; the next ready task follows.
    let out = alm(&b, &["--as", "s2", "next"]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.starts_with("notice [claim_lost]: Task A01-01 was claimed by two sessions at once, and the claim of s1"),
        "{text}"
    );
    assert!(text.contains("the task is s1's, not yours"), "{text}");
    assert!(text.contains("`git rebase --skip`"), "{text}");
    assert!(
        text.contains(
            "Next ready task (alm next's order; you choose, see `alm next`): A01-02  Second"
        ),
        "{text}"
    );
    let v = ok(&b, &["--as", "s2", "list"]);
    assert_eq!(v["warnings"][0]["code"], "claim_lost", "{v}");
    assert_eq!(v["warnings"][0]["params"]["holder"], "s1", "{v}");
    assert_eq!(v["claim_races"][0]["held_by"], "s1", "{v}");
    assert_eq!(v["next_ready"]["id"], "A01-02", "{v}");
    assert_eq!(v["issues"][0]["problem"]["code"], "merge_conflict", "{v}");
    // Without --as: both claims, nobody chosen.
    let v = ok(&b, &["list"]);
    assert_eq!(v["warnings"][0]["code"], "claim_race", "{v}");

    // The loser drops its claim: the task is s1's, and claiming it is refused with the pointer.
    run(&b, &["-c", "core.editor=true", "rebase", "--skip"]);
    let v = ok(&b, &["--as", "s2", "show", "A01-01"]);
    assert_eq!(v["task"]["holder"], "s1", "{v}");
    assert!(v.get("claim_races").is_none(), "{v}");
    let v = refused(
        &b,
        &["--as", "s2", "claim", "A01-01", "--branch", "main"],
        1,
        "task_held",
    );
    assert_eq!(v["next_ready"]["id"], "A01-02", "{v}");
    let out = alm(&b, &["--as", "s2", "claim", "A01-01", "--branch", "main"]);
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("error [task_held]: Task A01-01 is held by s1.\nNext ready task (alm next's order; you choose, see `alm next`): A01-02  Second"),
        "{err}"
    );
}

/// Task-A01-actor813: a task's difficulty, 1 to 5; none counts as 3 and is not written.
#[test]
fn a_task_has_a_difficulty_from_one_to_five() {
    let repo = filled();
    // Added without one: alm says so with the command, refuses nothing, writes nothing.
    let added = ok(
        &repo,
        &["add", "task", "A01-03", "--area", "A01", "--title", "Plain"],
    );
    assert_eq!(added["missing"][0]["code"], "difficulty");
    assert_eq!(added["missing"][0]["fix"], "alm difficulty A01-03 <1-5>");
    assert!(task_file(&repo, "A01-03").get("difficulty").is_none());
    assert_eq!(added["task"]["difficulty"], 3);
    assert_eq!(added["task"]["difficulty_default"], true);
    // Given when added: written, and nothing missing.
    let hard = ok(
        &repo,
        &[
            "add",
            "task",
            "A01-04",
            "--area",
            "A01",
            "--title",
            "Hard",
            "--difficulty",
            "5",
        ],
    );
    assert!(hard.get("missing").is_none_or(Value::is_null), "{hard}");
    assert_eq!(task_file(&repo, "A01-04")["difficulty"], 5);
    // Outside 1–5: wrong usage, nothing added.
    let out = alm(
        &repo,
        &[
            "add",
            "task",
            "A01-05",
            "--area",
            "A01",
            "--title",
            "X",
            "--difficulty",
            "6",
        ],
    );
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("not a difficulty"));
    assert!(!repo.join(".allomos/taskbag/tasks/A01-05").exists());
    let out = alm(&repo, &["difficulty", "A01-01", "0"]);
    assert_eq!(out.status.code(), Some(2));
    // Changed, shown, recorded in the history; --none takes it off.
    ok(&repo, &["difficulty", "A01-01", "1"]);
    assert_eq!(task_file(&repo, "A01-01")["difficulty"], 1);
    let shown = String::from_utf8(alm(&repo, &["show", "A01-01"]).stdout).unwrap();
    assert!(shown.contains("difficulty: 1\n"), "{shown}");
    let shown = String::from_utf8(alm(&repo, &["show", "A01-02"]).stdout).unwrap();
    assert!(shown.contains("difficulty: 3 (default)\n"), "{shown}");
    let history = ok(&repo, &["history", "A01-01"]);
    assert!(
        history["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["command"] == "difficulty"),
        "{history}"
    );
    ok(&repo, &["difficulty", "A01-01", "--none"]);
    assert!(task_file(&repo, "A01-01").get("difficulty").is_none());
    // Filters: list and next, a task with none counting as 3.
    ok(&repo, &["difficulty", "A01-02", "2"]);
    let ids = |v: &Value| -> Vec<String> {
        v["areas"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|a| a["tasks"].as_array().unwrap().clone())
            .map(|t| t["id"].as_str().unwrap().to_string())
            .collect()
    };
    let low = ok(&repo, &["list", "--difficulty", "1-2"]);
    assert_eq!(ids(&low), ["A01-02"]);
    let three = ok(&repo, &["list", "--difficulty", "3"]);
    assert_eq!(ids(&three), ["A01-01", "A01-03", "A02-01"]);
    let out = alm(&repo, &["list", "--difficulty", "4-2"]);
    assert_eq!(out.status.code(), Some(2));
    for id in ["A01-02", "A01-04"] {
        ok(&repo, &["ready", id]);
    }
    let next = ok(&repo, &["next", "--difficulty", "4-5"]);
    let listed: Vec<&str> = next["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_str().unwrap())
        .collect();
    assert_eq!(listed, ["A01-04"]);
    // A value outside 1–5 written by hand is kept, counts as 3 and is listed as a problem.
    let path = repo.join(".allomos/taskbag/tasks/A01-03/task.json");
    let mut file = task_file(&repo, "A01-03");
    file["difficulty"] = serde_json::json!(9);
    fs::write(&path, serde_json::to_string_pretty(&file).unwrap()).unwrap();
    let all = ok(&repo, &["list"]);
    assert!(
        all["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["problem"]["code"] == "difficulty_invalid"),
        "{all}"
    );
}

/// Task-A03-cow112: `alm monitor` without the app. With nothing running, status says so and
/// check and stop are refused; an app agent and an unknown frequency are refused with the reason.
#[test]
fn monitor_commands_say_what_runs_and_refuse_what_cannot() {
    let dir = tmp("monitor");
    let settings = tmp("monitor-settings");
    ok(&dir, &["init", "--name", "Mon"]);
    let run = |args: &[&str]| -> (i32, Value) {
        let mut all = vec!["--json"];
        all.extend_from_slice(args);
        let out = Command::new(ALM)
            .current_dir(&dir)
            .args(&all)
            .env("ALLOMOS_SETTINGS_DIR", &settings)
            .env_remove("ALLOMOS_TASKBAG")
            .output()
            .unwrap();
        let v: Value = serde_json::from_str(&String::from_utf8(out.stdout).unwrap()).unwrap();
        (out.status.code().unwrap(), v)
    };
    let (code, v) = run(&["monitor", "status"]);
    assert_eq!((code, &v["monitors"]), (0, &json!([])), "{v}");
    for words in [["monitor", "check"], ["monitor", "stop"]] {
        let (code, v) = run(&words);
        assert_eq!(code, 2, "{v}");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("no monitor is running"),
            "{v}"
        );
    }
    let (code, v) = run(&[
        "monitor",
        "start",
        "--mode",
        "quiet",
        "--agent",
        "claude-app",
    ]);
    assert_eq!(code, 2);
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("only agents that run in a terminal"),
        "{v}"
    );
    let (code, v) = run(&["monitor", "list"]);
    assert_eq!(code, 0, "{v}");
    // Without --mode: the person must choose quiet or normal.
    let out = Command::new(ALM)
        .current_dir(&dir)
        .args(["monitor", "start"])
        .env("ALLOMOS_SETTINGS_DIR", &settings)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("--mode"));
}

/// What `alm` prints for people.
fn plain(dir: &Path, args: &[&str]) -> String {
    String::from_utf8(alm(dir, args).stdout).unwrap()
}

/// Task-A01-minnow200: writes a request to hand `id` over into its `task.json`, as an older app did.
fn ask_handover(dir: &Path, id: &str, holder: &str) {
    let path = dir.join(format!(".allomos/taskbag/tasks/{id}/task.json"));
    let mut task: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    task["handover_request"] = json!({
        "holder": holder, "by": "will", "at": "2026-11-11T06:10:02Z", "to": "codex_cli"
    });
    fs::write(&path, serde_json::to_string_pretty(&task).unwrap()).unwrap();
}

/// Task-A01-kind858: an old request still reads, tells nobody, and a release or claim clears it.
#[test]
fn an_old_handover_request_tells_nobody_and_ends_with_release_or_claim() {
    let repo = filled();
    ok(&repo, &["ready", "A01-01"]);
    ok(
        &repo,
        &["--as", "s1", "claim", "A01-01", "--branch", "main"],
    );
    ask_handover(&repo, "A01-01", "s1");

    let (code, v) = fj(&repo, &["--as", "s1", "show", "A01-01"]);
    assert_eq!(code, 0, "{v}");
    assert!(v.get("handover_requested").is_none(), "{v}");
    assert!(!v["warnings"].to_string().contains("handover"), "{v}");
    assert_eq!(v["task"]["handover_request"]["to"], "codex_cli", "{v}");
    let out = plain(&repo, &["--as", "s1", "beat"]);
    assert!(!out.contains("handover"), "{out}");
    let out = plain(&repo, &["--as", "s1", "show", "A01-01"]);
    assert!(!out.contains("handover"), "{out}");

    ok(
        &repo,
        &["--as", "s1", "release", "A01-01", "--note", "go on on w"],
    );
    let (_, v) = fj(&repo, &["--as", "s1", "show", "A01-01"]);
    assert!(v["task"].get("handover_request").is_none(), "{v}");

    ok(
        &repo,
        &["--as", "s2", "claim", "A01-01", "--branch", "main"],
    );
    ask_handover(&repo, "A01-01", "s2");
    ok(
        &repo,
        &[
            "--as", "s3", "claim", "A01-01", "--take", "--branch", "main",
        ],
    );
    let (_, v) = fj(&repo, &["--as", "s3", "show", "A01-01"]);
    assert!(v["task"].get("handover_request").is_none(), "{v}");
}
