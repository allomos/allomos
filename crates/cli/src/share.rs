//! The Git commands `alm` prints after a change, for exactly the files it wrote (decision 0017): it
//! runs none of them.

use std::path::{Path, PathBuf};

use allomos_core::{State, Task};
use allomos_store::{events, load, plain_path, quota_is_codex};
use serde_json::json;

use crate::{Cli, Command, Done, locate};

/// What `alm` knows before a change, for the Git commands it prints after it.
pub(crate) struct Before {
    /// Changes made in the app that no agent has been told to share yet.
    app: Option<events::AppChanges>,
}

impl Before {
    pub(crate) fn look(cli: &Cli, cwd: &Path) -> Before {
        let telling = (cli.command.changes() && !matches!(cli.command, Command::Init { .. }))
            || matches!(cli.command, Command::Next { .. });
        let app = telling
            .then(|| locate(cli, cwd).ok())
            .flatten()
            .map(|r| events::unshared_app_changes(&r))
            .filter(|a| a.count > 0);
        Before { app }
    }
}

/// One argument of a printed command: as it is when it is plain, otherwise in double quotes.
pub(crate) fn quoted(arg: &str) -> String {
    let plain = !arg.is_empty()
        && arg
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-/@+=,:".contains(c));
    if plain {
        arg.to_string()
    } else {
        format!("\"{}\"", arg.replace('"', "\\\""))
    }
}

/// Said after every change's Git commands (Task-A01-flour637): what is not in the list stays
/// where it is, whoever made it.
pub(crate) const SAVE_AS_PRINTED: &str = "Run the commands as printed, never `git add -A`, `git add .` or `git commit -a` in the taskbag: those also take changes others have not shared yet (the person's changes in the Allomos app, other sessions' changes).\n";

/// Whether the session running this command is Codex for certain (Task-A01-flame635): then the
/// Git commands alm prints are plain (`git add …`, run in the folder it names), because
/// `git -C <folder> …` matches none of Allomos's rules for Codex. The agent is the session agent
/// of the task the command acts on (unless another session holds that task), else that of a task
/// this session holds, else what the session name says for certain. Unsure is not Codex.
pub(crate) fn for_codex(cli: &Cli, cwd: &Path) -> bool {
    let me = cli.holder.as_deref();
    let tasks = locate(cli, cwd)
        .ok()
        .and_then(|root| load(&root).taskbag)
        .map(|bag| bag.tasks)
        .unwrap_or_default();
    let agent_of = |t: &Task| {
        t.session
            .as_ref()
            .and_then(|s| s.agent.clone())
            .filter(|a| !a.trim().is_empty())
    };
    let others = |t: &&Task| t.holder.is_some() && me.is_some() && t.holder.as_deref() != me;
    let target = cli
        .command
        .target()
        .and_then(|id| tasks.iter().find(|t| t.id == id))
        .filter(|t| !others(t))
        .and_then(agent_of);
    let held = || {
        let me = me?;
        tasks
            .iter()
            .filter(|t| t.state == State::InProgress && t.holder.as_deref() == Some(me))
            .find_map(agent_of)
    };
    quota_is_codex(target.or_else(held).as_deref(), me)
}

/// Said to Codex after the commands of a change (Task-A01-flame635, the unattended trial): it
/// wrote `git status --short && git add … && git commit … && git push`, and a line with one
/// command the rules do not name runs in the sandbox as a whole, where Git cannot write.
pub(crate) const CODEX_ON_ITS_OWN: &str = "Codex: run every Git command on a line of its own, your own Git commands too (commit, push, merge, worktree): no `&&`, `;` or pipes, and nothing else on the line. Joined with a command that Allomos's rules for Codex do not name (`git status`, a test, `chmod`), the whole line runs in the sandbox, where Git cannot write.\n";

/// The line above printed Git commands: `For Git:`, or for Codex (`plain`) the folder to run
/// them in, since they do not name it themselves.
pub(crate) fn git_heading(folder: &Path, plain: bool) -> String {
    if plain {
        format!(
            "For Git, in {} (run each command on its own, with that folder as its working folder):\n",
            quoted(&plain_path(folder).display().to_string())
        )
    } else {
        "For Git:\n".to_string()
    }
}

/// `git add`, `git commit` (with the two message lines) and, unless `push` is false, `git push`,
/// run in `taskbag` and naming exactly `files` (relative to it). `plain` (for Codex,
/// [`for_codex`]): without `-C <taskbag>`; [`git_heading`] names the folder instead.
pub(crate) fn git_commands(
    taskbag: &Path,
    files: &[String],
    subject: &str,
    who: &str,
    push: bool,
    plain: bool,
) -> Vec<String> {
    let at = if plain {
        "git".to_string()
    } else {
        format!(
            "git -C {}",
            quoted(&plain_path(taskbag).display().to_string())
        )
    };
    let list: Vec<String> = files.iter().map(|f| quoted(f)).collect();
    let list = list.join(" ");
    let mut out = vec![
        format!("{at} add -- {list}"),
        format!(
            "{at} commit -m {} -m {} -- {list}",
            quoted(subject),
            quoted(who)
        ),
    ];
    if push {
        out.push(format!("{at} push"));
    }
    out
}

/// After a change (decision 0017): `alm` runs no Git; it tells the agent the exact commands that
/// share this change, naming only the files it wrote (`changed_files`, `git_commands`), and
/// any changes made in the app that are not shared yet (`app_changes`), to commit as the
/// person's. `alm init` prints none: no agent is connected yet.
pub(crate) fn share(cli: &Cli, cwd: &Path, before: &Before, written: &[PathBuf], done: &mut Done) {
    let Some(root) = done.fields["taskbag"].as_str().map(PathBuf::from) else {
        return;
    };
    // A command that packed old history (Task-A01-59) shares that too, even when it only reads.
    let packed = done.fields["packed"]
        .as_array()
        .is_some_and(|p| !p.is_empty());
    let changes = (cli.command.changes() || packed) && !matches!(cli.command, Command::Init { .. });
    let mut text = String::new();
    let mut files: Vec<String> = written
        .iter()
        .filter(|p| p.file_name().is_some_and(|n| n != allomos_store::LOCK_FILE))
        .map(|p| match p.strip_prefix(&root) {
            Ok(rel) => allomos_store::portable_path(rel),
            Err(_) => p.display().to_string(),
        })
        .collect();
    files.sort();
    files.dedup();
    // Read after the change, so `alm claim --session-agent codex` already prints the plain form.
    let plain = (changes || before.app.is_some()) && for_codex(cli, cwd);
    if changes || before.app.is_some() {
        done.fields["git_folder"] = json!(plain_path(&root).display().to_string());
    }
    if changes {
        let command = cli.command.name();
        // Added without an ID: name the one it got.
        let made = ["task", "area", "iteration"]
            .iter()
            .find_map(|k| done.fields[*k]["id"].as_str())
            .map(String::from);
        let subject = match cli.command.target().map(String::from).or(made) {
            Some(id) => format!("alm: {command} {id}"),
            None => format!("alm: {command}"),
        };
        let subject = if packed && !cli.command.changes() {
            "alm: pack history".to_string()
        } else {
            subject
        };
        let who = format!(
            "Agent: {}",
            cli.holder.as_deref().unwrap_or("<your session name>")
        );
        // In general words, with the Git commands as the example (Task-A01-60, approver
        // 2026-10-20): Allomos does not assume Git.
        let (instruction, commands) = if files.is_empty() {
            (
                "Nothing changed, so there is nothing to save.".to_string(),
                Vec::new(),
            )
        } else if matches!(cli.command, Command::Claim { .. }) {
            (
                "Save this change in your version control tool and share it now, before you start the work. Until it is shared, other sessions do not see your claim and may take this task too. Changed files:".to_string(),
                git_commands(&root, &files, &subject, &who, true, plain),
            )
        } else {
            (
                "Save this change in your version control tool and share it. Changed files:"
                    .to_string(),
                git_commands(&root, &files, &subject, &who, true, plain),
            )
        };
        text.push('\n');
        text.push_str(&instruction);
        text.push('\n');
        if !commands.is_empty() {
            for f in &files {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str(&git_heading(&root, plain));
        }
        for c in &commands {
            text.push_str(&format!("  {c}\n"));
        }
        if !commands.is_empty() {
            text.push_str("Save only these files; leave other changes as they are.\n");
            // Task-A01-flour637: a beat saved with `git commit -a` took along changes the
            // person had made in the app, without their events.
            text.push_str(SAVE_AS_PRINTED);
            if plain {
                text.push_str(CODEX_ON_ITS_OWN);
            }
        }
        done.fields["changed_files"] = json!(files);
        done.fields["git_commands"] = json!(commands);
        done.fields["git_instruction"] = json!(instruction);
    }
    if let Some(app) = &before.app {
        // Files this change also wrote are already in its own commit.
        let rest: Vec<String> = app
            .files
            .iter()
            .filter(|f| !(changes && files.contains(f)))
            .cloned()
            .collect();
        if !rest.is_empty() {
            let by = if app.by.is_empty() {
                "the person".to_string()
            } else {
                app.by.join(", ")
            };
            let commands = git_commands(
                &root,
                &rest,
                "app: changes made in the Allomos app",
                &format!("By: {by} (Allomos app)"),
                true,
                plain,
            );
            text.push_str(&format!(
                "\n{} change(s) made in the Allomos app by {by} are not shared yet. Save and share them in your version control tool as the person's, on their own (if someone already did, there is nothing to save; that is fine). Changed files:\n",
                app.count
            ));
            for f in &rest {
                text.push_str(&format!("  {f}\n"));
            }
            text.push_str(&git_heading(&root, plain));
            for c in &commands {
                text.push_str(&format!("  {c}\n"));
            }
            done.fields["app_changes"] = json!({
                "count": app.count,
                "by": app.by,
                "files": rest,
                "git_commands": commands,
            });
        }
    }
    done.text.push_str(&text);
    if !done.after.is_empty() {
        let after = std::mem::take(&mut done.after);
        done.text.push('\n');
        done.text.push_str(&after);
    }
}
