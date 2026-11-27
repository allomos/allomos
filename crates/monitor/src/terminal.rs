//! The terminal agents Monitor ready tasks starts (Task-A03-cow112, moved from the desktop app's
//! `agents.rs`): finding Claude Code, Codex and Antigravity's command, and starting a new session
//! of one in a terminal window of its own with the request already sent. Shared by `alm monitor`
//! and the app, so both start sessions the same way.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::flow::Choice;
use crate::permissions::{
    ClaudeEnv, add_dirs, antigravity_quiet_settings_arg, claude_permission_args,
};
use allomos_core::Problem;

/// A program started without a window. On Windows a windowed app's command-line child (`claude
/// --version`) otherwise gets a console window that flashes on screen (task A07-05).
pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// The person's home folder: `HOME`, or `USERPROFILE` on Windows.
pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Folders where command-line tools usually live. A macOS app does not get the person's shell
/// `PATH`; npm-installed commands use `#!/usr/bin/env node`, so their Node folders are searched
/// after `PATH` and the existing common tool folders.
pub fn tool_dirs(home: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    for d in [
        ".local/bin",
        ".claude/local",
        ".npm-global/bin",
        ".bun/bin",
        ".cargo/bin",
    ] {
        dirs.push(
            d.split('/')
                .fold(home.to_path_buf(), |p, part| p.join(part)),
        );
    }
    if cfg!(windows) {
        // Where npm, pnpm and WinGet put the commands they install.
        for (var, sub) in [
            ("APPDATA", &["npm"][..]),
            ("LOCALAPPDATA", &["pnpm"][..]),
            ("LOCALAPPDATA", &["Microsoft", "WinGet", "Links"][..]),
            // Where Antigravity's installer puts `agy` (Task-A03-queen969).
            ("LOCALAPPDATA", &["agy", "bin"][..]),
        ] {
            if let Some(base) = std::env::var_os(var) {
                dirs.push(sub.iter().fold(PathBuf::from(base), |p, part| p.join(part)));
            }
        }
    } else {
        dirs.push("/opt/homebrew/bin".into());
        dirs.push("/usr/local/bin".into());
    }
    let prefixes = [PathBuf::from("/opt/homebrew"), PathBuf::from("/usr/local")];
    for dir in node_dirs(home, &prefixes) {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

/// Node folders to add when checking npm-installed terminal agents. A macOS app does not inherit
/// the shell's `PATH`, and those commands rely on `#!/usr/bin/env node`; only folders with an
/// executable `node` are included. `prefixes` keeps Homebrew discovery testable without reading the
/// computer's real installations. Windows leaves this to `PATH`, where npm's `.cmd` and Node live.
#[cfg(windows)]
pub fn node_dirs(_home: &Path, _prefixes: &[PathBuf]) -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(not(windows))]
pub fn node_dirs(home: &Path, prefixes: &[PathBuf]) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    for prefix in prefixes {
        let opt = prefix.join("opt");
        push_node_dir(&mut dirs, opt.join("node").join("bin"));

        let mut versioned = std::fs::read_dir(&opt)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let version = name
                    .to_str()?
                    .strip_prefix("node@")
                    .and_then(version_parts)?;
                let bin = entry.path().join("bin");
                Some((version, bin))
            })
            .collect::<Vec<_>>();
        versioned.sort_by(|(a_version, a_path), (b_version, b_path)| {
            b_version.cmp(a_version).then_with(|| a_path.cmp(b_path))
        });
        for (_, bin) in versioned {
            push_node_dir(&mut dirs, bin);
        }
    }

    push_node_dir(&mut dirs, home.join(".volta/bin"));

    let nvm = home.join(".nvm");
    let versions = nvm_versions(&nvm);
    let default = std::fs::read_to_string(nvm.join("alias/default")).ok();
    let selected_nvm = default
        .as_deref()
        .and_then(|alias| matching_nvm_version(alias, &nvm, &versions))
        .or_else(|| versions.first().map(|(_, bin)| bin.clone()));
    if let Some(bin) = selected_nvm {
        push_node_dir(&mut dirs, bin);
    }

    for bin in [
        home.join(".local/share/fnm/aliases/default/bin"),
        home.join("Library/Application Support/fnm/aliases/default/bin"),
        home.join(".asdf/shims"),
        home.join(".local/share/mise/shims"),
    ] {
        push_node_dir(&mut dirs, bin);
    }

    dirs
}

#[cfg(not(windows))]
fn nvm_versions(nvm: &Path) -> Vec<(Vec<u64>, PathBuf)> {
    let mut versions = std::fs::read_dir(nvm.join("versions/node"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let version = name.to_str()?.strip_prefix('v').and_then(version_parts)?;
            let bin = entry.path().join("bin");
            Some((version, bin))
        })
        .filter(|(_, bin)| has_executable_node(bin))
        .collect::<Vec<_>>();
    versions.sort_by(|(a_version, a_path), (b_version, b_path)| {
        b_version.cmp(a_version).then_with(|| a_path.cmp(b_path))
    });
    versions
}

#[cfg(not(windows))]
fn matching_nvm_version(
    alias: &str,
    nvm: &Path,
    versions: &[(Vec<u64>, PathBuf)],
) -> Option<PathBuf> {
    let alias = alias.trim();
    if alias == "lts/*" {
        return std::fs::read_dir(nvm.join("alias/lts"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
            .filter_map(|alias| matching_nvm_version_value(&alias, versions))
            .max_by_key(|bin| {
                versions
                    .iter()
                    .find(|(_, version_bin)| version_bin == bin)
                    .map(|(version, _)| version.clone())
            });
    }

    if let Some(name) = alias.strip_prefix("lts/") {
        if name.is_empty() || name.contains('/') {
            return None;
        }
        let target = std::fs::read_to_string(nvm.join("alias/lts").join(name)).ok()?;
        return matching_nvm_version_value(&target, versions);
    }

    matching_nvm_version_value(alias, versions)
}

#[cfg(not(windows))]
fn matching_nvm_version_value(alias: &str, versions: &[(Vec<u64>, PathBuf)]) -> Option<PathBuf> {
    let alias = alias.trim();
    let requested = alias.strip_prefix('v').unwrap_or(alias);
    let requested = version_parts(requested)?;
    versions
        .iter()
        .find(|(version, _)| {
            requested.len() <= version.len()
                && requested
                    .iter()
                    .zip(version)
                    .all(|(wanted, installed)| wanted == installed)
        })
        .map(|(_, bin)| bin.clone())
}

#[cfg(not(windows))]
fn version_parts(version: &str) -> Option<Vec<u64>> {
    let parts = version.split('.').collect::<Vec<_>>();
    if parts.is_empty() || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    parts
        .into_iter()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()
}

#[cfg(not(windows))]
fn push_node_dir(dirs: &mut Vec<PathBuf>, bin: PathBuf) {
    if has_executable_node(&bin) && !dirs.contains(&bin) {
        dirs.push(bin);
    }
}

#[cfg(not(windows))]
fn has_executable_node(bin: &Path) -> bool {
    let node = bin.join("node");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        node.metadata()
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        node.is_file()
    }
}

/// The file names a command called `name` may have: on Windows a program (`claude.exe`) or the
/// script npm writes for it (`codex.cmd`); elsewhere just the name.
pub fn tool_names(name: &str, windows: bool) -> Vec<String> {
    if windows {
        ["exe", "cmd", "bat"]
            .iter()
            .map(|ext| format!("{name}.{ext}"))
            .collect()
    } else {
        vec![name.to_string()]
    }
}

/// The first `name` executable in `dirs` that answers `--version` (so a broken install is not offered).
pub fn find_tool(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let names = tool_names(name, cfg!(windows));
    let path = std::env::join_paths(dirs).ok()?;
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .filter(|p| p.is_file())
        // Asked twice: a tool updating itself can fail its first answer (seen once with `claude`).
        .find(|p| runs(p, &path) || runs(p, &path))
}

fn runs(program: &Path, path: &OsStr) -> bool {
    let Ok(mut child) = command(program)
        .arg("--version")
        .env("PATH", path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    else {
        return false;
    };
    // A tool that hangs is treated as not working.
    for _ in 0..50 {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return false,
        }
    }
    let _ = child.kill();
    false
}

/// An agent's name, as messages say it.
pub fn name(agent: &str) -> &'static str {
    match crate::accounts::base_agent(agent) {
        "claude_cli" => "Claude Code (terminal)",
        "claude_app" => "Claude (app)",
        "codex_cli" => "Codex (terminal)",
        "codex_app" => "Codex (app)",
        "antigravity_cli" => "Antigravity (terminal)",
        _ => "another agent",
    }
}

pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

pub fn applescript_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Opening or starting `agent` failed.
pub fn ask_failed(agent: &str, detail: impl ToString) -> Problem {
    Problem::OpenFailed {
        path: name(agent).to_string(),
        detail: detail.to_string(),
    }
}

/// PowerShell's quoting: single quotes, with each quote inside doubled (PowerShell also takes
/// the typographic single quotes as quotes).
pub fn powershell_quote(text: &str) -> String {
    let mut out = String::from("'");
    for c in text.chars() {
        if matches!(c, '\'' | '\u{2018}' | '\u{2019}' | '\u{201a}' | '\u{201b}') {
            out.push(c);
        }
        out.push(c);
    }
    out.push('\'');
    out
}

/// What a new Terminal window types (macOS): the folder of the app's own `alm` first in `PATH`,
/// for that window only, then the agent in the project. With no `alm` beside the app, `PATH` is
/// left as it is (task A03-16).
pub fn terminal_script(project: &Path, program: &str, alm_dir: Option<&Path>) -> String {
    let start = format!(
        "cd {} && {program}",
        shell_quote(&project.display().to_string())
    );
    match alm_dir {
        Some(dir) => format!(
            "export PATH={}:\"$PATH\"; {start}",
            shell_quote(&dir.display().to_string())
        ),
        None => start,
    }
}

/// The start script a new Terminal window runs (macOS, Task-A03-golf005): it removes itself, then
/// does what [`terminal_script`] says. The command is not typed into the window: Terminal types
/// `do script` text into the terminal device, and a line longer than the device's input limit
/// (1024 bytes on macOS) is cut off before the shell reads it, so a long command (Antigravity's
/// `--settings` with the person's permission rules) reached zsh with an open quote and stopped at
/// its continuation prompt. zsh reads the script from the file, whatever its length.
///
/// The request is read here from `request` (Task-A03-marina834): into [`REQUEST_ARG`]'s variable,
/// exactly as written, its last line ends too (`$(…)` alone drops them, so a `.` is read after it
/// and taken off again), and the file is removed. A request that cannot be read starts nothing:
/// the window says so, and both files are removed all the same.
pub fn start_script(
    script: &Path,
    request: &Path,
    project: &Path,
    program: &str,
    alm_dir: Option<&Path>,
) -> String {
    let file = shell_quote(&request.display().to_string());
    format!(
        "rm -f {script}\n\
if {REQUEST_VAR}=\"$(cat {file} && printf .)\"; then\n\
rm -f {file}\n\
{REQUEST_VAR}=\"${{{REQUEST_VAR}%.}}\"\n\
{start}\n\
unset {REQUEST_VAR}\n\
else\n\
rm -f {file}\n\
unset {REQUEST_VAR}\n\
printf '%s\\n' {failed} >&2\n\
fi\n",
        script = shell_quote(&script.display().to_string()),
        start = terminal_script(project, program, alm_dir),
        failed = shell_quote(START_FAILED),
    )
}

/// The shell variable the start script reads the request into.
const REQUEST_VAR: &str = "allomos_request";
/// How a terminal agent's command takes the request on macOS: the start script's variable
/// ([`start_script`]), quoted, so it reaches the agent as one argument, whatever it holds.
const REQUEST_ARG: &str = "\"$allomos_request\"";
/// What the window says when the request cannot be read (the agent is not started).
const START_FAILED: &str = "Allomos could not read the request, so the agent was not started. \
Nothing was sent; open it again from Allomos.";

/// A new file in the temporary folder holding `contents`, readable and writable only by the person
/// (0600), under a name no other file has: `<prefix>-<process>-<time>-<count>.<ext>`, made new
/// every time (an existing file is never reused or overwritten; Task-A03-marina834).
pub fn private_temp_file(prefix: &str, ext: &str, contents: &[u8]) -> std::io::Result<PathBuf> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNT: AtomicU64 = AtomicU64::new(0);
    loop {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        let n = COUNT.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{prefix}-{}-{stamp}-{n}.{ext}", std::process::id()));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut f) => {
                return match f.write_all(contents) {
                    Ok(()) => Ok(path),
                    Err(e) => {
                        let _ = std::fs::remove_file(&path);
                        Err(e)
                    }
                };
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
}

/// macOS: open a new Terminal tab that starts `program` (a [`session_command`]) in `project` with
/// `request` sent, for the monitor and the app alike (Task-A03-marina834). The request and the
/// start script go into new private files ([`private_temp_file`]); the tab types only
/// [`start_line`], and the start script reads, checks and removes them. When the tab cannot be
/// opened, both files are removed here. Returns the tab's window and terminal device.
pub fn open_terminal(
    id: &str,
    project: &Path,
    request: &str,
    program: &str,
    alm_dir: Option<&Path>,
    activate: bool,
) -> Result<Option<crate::session_tab::SessionTab>, Problem> {
    let file = private_temp_file("allomos-request", "txt", request.as_bytes())
        .map_err(|e| ask_failed(id, e))?;
    let start = match private_temp_file("allomos-start", "sh", b"") {
        Ok(start) => start,
        Err(e) => {
            let _ = std::fs::remove_file(&file);
            return Err(ask_failed(id, e));
        }
    };
    let clean = |e: String| {
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(&start);
        ask_failed(id, e)
    };
    std::fs::write(
        &start,
        start_script(&start, &file, project, program, alm_dir),
    )
    .map_err(|e| clean(e.to_string()))?;
    let script = format!(
        "tell application \"Terminal\"\n{}set t to do script {}\nset w to (first window whose tabs contains t)\nreturn (id of w as text) & \", \" & (tty of t)\nend tell",
        if activate { "activate\n" } else { "" },
        applescript_string(&start_line(&start))
    );
    let out = command("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| clean(e.to_string()))?;
    if !out.status.success() {
        return Err(clean("Terminal did not open".to_string()));
    }
    Ok(crate::session_tab::parse_opened(&String::from_utf8_lossy(
        &out.stdout,
    )))
}

/// The short line a new Terminal window types to run the start script (macOS): `. '<script>'`, read
/// by the window's own shell, so the agent runs there as if typed.
pub fn start_line(script: &Path) -> String {
    format!(". {}", shell_quote(&script.display().to_string()))
}

/// What a new PowerShell window runs (Windows): the same, with `$env:PATH`. The tool is started
/// by its full path: a new window does not always get the same `PATH`.
/// `dirs`: folders an Antigravity or Claude Code session adds to its workspace
/// ([`workspace_dirs`]; empty for the other agents); `id` the agent, for Claude's permissions.
pub fn powershell_script(
    id: &str,
    project: &Path,
    found: &str,
    alm_dir: Option<&Path>,
    dirs: &[PathBuf],
) -> String {
    let args = if id == "claude_cli" {
        claude_permission_args(false, dirs, None, None, false, powershell_quote, "'--'")
    } else {
        add_dirs(dirs, powershell_quote)
    };
    let start = format!(
        "Set-Location -LiteralPath {}; & {}{}{}",
        powershell_quote(&project.display().to_string()),
        powershell_quote(found),
        if args.is_empty() { "" } else { " " },
        args.trim_end()
    );
    match alm_dir {
        Some(dir) => format!(
            "$env:PATH = {} + $env:PATH; {start}",
            powershell_quote(&format!("{};", dir.display()))
        ),
        None => start,
    }
}

/// What a monitored Codex session is started with ahead of its request (Task-A03-plaza436).
const CODEX_SANDBOX: &str = "--sandbox workspace-write ";
/// Quiet monitored sessions do not ask for approval and can use the network (Task-A03-trail745).
const CODEX_QUIET_POLICY: &str =
    "--ask-for-approval never --config sandbox_workspace_write.network_access=true ";
/// The app-access prompt is separate from command approvals. Allow only Allomos, only in the
/// quiet macOS process; this command-line override does not change the person's Codex settings.
const CODEX_QUIET_MACOS_APP_ACCESS: &str =
    "--config 'computer_use.macos.bundle_ids={\"io.allomos\"=\"allow\"}' ";

fn codex_quiet_policy() -> String {
    let app_access = if cfg!(target_os = "macos") {
        CODEX_QUIET_MACOS_APP_ACCESS
    } else {
        ""
    };
    format!("{CODEX_QUIET_POLICY}{app_access}")
}

/// The folders an Antigravity (`agy`) session started in `project` adds to its workspace with
/// `--add-dir` (Task-A03-latch882): the taskbag's folder, then each of the taskbag's project
/// folders (`project_folders`) other than `project` itself, as absolute paths. agy's auto mode lets
/// the session use files in its workspace without asking, so it no longer stops at "Allow access
/// to this file?" for the taskbag's `project-notes.md` or another project folder (approver,
/// 2026-11-17: every project folder of the taskbag, for sessions Allomos starts only; the person's
/// agy settings are untouched). A folder missing on this computer is left out, and so is a folder
/// that cannot be listed: the session then starts as it did before. A monitored Codex session gets
/// the same folders, made writable in its `workspace-write` sandbox (Task-A03-bag625; approver,
/// 2026-11-17: every project folder, for sessions Allomos starts only).
pub fn workspace_dirs(taskbag: &Path, project: &Path) -> Vec<PathBuf> {
    let whole = |p: &Path| allomos_store::tidy(&std::path::absolute(p).unwrap_or(p.to_path_buf()));
    let project = whole(project);
    let mut dirs = vec![whole(taskbag)];
    if let Ok(folders) = allomos_store::folders::list(taskbag) {
        dirs.extend(
            folders
                .into_iter()
                .filter(|f| f.exists)
                .map(|f| whole(&f.full)),
        );
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if dir != project && !out.contains(&dir) {
            out.push(dir);
        }
    }
    out
}

/// The command line that starts a terminal agent with the request the start script read
/// ([`start_script`], [`REQUEST_ARG`]), for Monitor ready tasks and the app (Task-A03-marina834) (Task-A03-moth285): `claude --session-id <id>`
/// takes the session ID Allomos made, so the app knows the session before it claims; Codex picks
/// its own, and so does Antigravity's `agy`, which takes an initial prompt only after `-i`
/// (`--prompt-interactive`; Task-A03-queen969). The request goes through a file because a long, many-line request typed into a
/// terminal window breaks at its first line end. Codex always starts in the `workspace-write`
/// sandbox, whatever the person's own Codex settings say, so it can change files in the project
/// folder without a person approving each edit (Task-A03-plaza436). Only quiet monitored sessions
/// set its approval policy to `never` and allow network access in the sandbox (Task-A03-trail745).
/// On macOS they also allow the Allomos app in this process only, so Computer Use does not ask for
/// app access. User settings are untouched. Antigravity adds `dirs` to its workspace
/// ([`workspace_dirs`], Task-A03-latch882), and Codex makes them writable in its sandbox
/// (`--add-dir`, Task-A03-bag625), so a quiet session's writes to the taskbag do not fail
/// silently; Claude Code separates quiet monitoring (`--permission-mode auto`, no `--add-dir`)
/// from normal/desktop launches (trusted `--add-dir` only, keeping `auto` when configured;
/// [`claude_permission_args_for`](crate::permissions::claude_permission_args_for),
/// Task-A03-emu042, Task-A03-lava982).
pub fn session_command(id: &str, session_id: Option<&str>, dirs: &[PathBuf]) -> Option<String> {
    session_command_with_quiet(
        id,
        session_id,
        false,
        dirs,
        None,
        &Choice::default(),
        None,
        None,
    )
}

/// The words a chosen model and reasoning effort add to agent `id`'s command (A03-alto709), each
/// ending in a space; nothing for the agent's own setting, so the command is as it was. Claude Code
/// and Antigravity take `--model` and `--effort`; Codex takes `-m` and its
/// `model_reasoning_effort` setting for this process only. Values are quoted with `quote`.
pub fn choice_args(id: &str, choice: &Choice, quote: fn(&str) -> String) -> String {
    let (model, effort): (&str, fn(&str) -> String) = match id {
        "claude_cli" | "antigravity_cli" => ("--model", |v| format!("--effort {v}")),
        "codex_cli" => ("-m", |v| format!("--config {v}")),
        _ => return String::new(),
    };
    let mut out = String::new();
    if let Some(m) = choice.model.as_deref() {
        out.push_str(&format!("{model} {} ", quote(m)));
    }
    if let Some(e) = choice.effort.as_deref() {
        let value = if id == "codex_cli" {
            quote(&format!("model_reasoning_effort={e}"))
        } else {
            quote(e)
        };
        out.push_str(&effort(&value));
        out.push(' ');
    }
    out
}

#[allow(clippy::too_many_arguments)]
pub fn session_command_with_quiet(
    id: &str,
    session_id: Option<&str>,
    quiet: bool,
    dirs: &[PathBuf],
    taskbag: Option<&Path>,
    choice: &Choice,
    config: Option<(&str, &Path)>,
    claude: Option<&ClaudeEnv>,
) -> Option<String> {
    let request = REQUEST_ARG;
    let chosen = choice_args(id, choice, shell_quote);
    match (id, session_id) {
        ("claude_cli", Some(s)) => Some(format!(
            "claude --session-id {} {chosen}{}{request}",
            shell_quote(s),
            claude_permission_args(quiet, dirs, config, claude, true, shell_quote, "--")
        )),
        ("claude_cli", None) => Some(format!(
            "claude {chosen}{}{request}",
            claude_permission_args(quiet, dirs, config, claude, true, shell_quote, "--")
        )),
        ("codex_cli", _) => Some(format!(
            "codex {CODEX_SANDBOX}{}{}{chosen}{request}",
            add_dirs(dirs, shell_quote),
            if quiet {
                codex_quiet_policy()
            } else {
                String::new()
            }
        )),
        ("antigravity_cli", _) => Some(format!(
            "agy {}{}{chosen}-i {request}",
            add_dirs(dirs, shell_quote),
            if quiet {
                antigravity_quiet_settings_arg(taskbag, shell_quote)
            } else {
                String::new()
            }
        )),
        _ => None,
    }
}

/// The same on Windows: PowerShell reads the file into a variable, removes it, and passes it.
pub fn powershell_session(
    id: &str,
    project: &Path,
    found: &str,
    file: &Path,
    session_id: Option<&str>,
    alm_dir: Option<&Path>,
    config: Option<(&str, &Path)>,
    dirs: &[PathBuf],
) -> String {
    powershell_session_with_quiet(
        id,
        project,
        found,
        file,
        session_id,
        alm_dir,
        config,
        false,
        dirs,
        None,
        &Choice::default(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn powershell_session_with_quiet(
    id: &str,
    project: &Path,
    found: &str,
    file: &Path,
    session_id: Option<&str>,
    alm_dir: Option<&Path>,
    config: Option<(&str, &Path)>,
    quiet: bool,
    dirs: &[PathBuf],
    taskbag: Option<&Path>,
    choice: &Choice,
    claude: Option<&ClaudeEnv>,
) -> String {
    let file = powershell_quote(&file.display().to_string());
    let chosen = choice_args(id, choice, powershell_quote);
    let session = match (id, session_id) {
        ("claude_cli", Some(s)) => format!(
            "--session-id {} {chosen}{}",
            powershell_quote(s),
            claude_permission_args(quiet, dirs, config, claude, true, powershell_quote, "'--'")
        ),
        ("claude_cli", None) => format!(
            "{chosen}{}",
            claude_permission_args(quiet, dirs, config, claude, true, powershell_quote, "'--'")
        ),
        ("codex_cli", _) => format!(
            "{CODEX_SANDBOX}{}{}{chosen}",
            add_dirs(dirs, powershell_quote),
            if quiet {
                codex_quiet_policy()
            } else {
                String::new()
            }
        ),
        ("antigravity_cli", _) => format!(
            "{}{}{chosen}-i ",
            add_dirs(dirs, powershell_quote),
            if quiet {
                antigravity_quiet_settings_arg(taskbag, powershell_quote)
            } else {
                String::new()
            }
        ),
        _ => String::new(),
    };
    let start = format!(
        "$env:{MONITORED_ENV} = '1'; Set-Location -LiteralPath {}; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath {file}; Remove-Item -LiteralPath {file}; & {} {session}$request",
        powershell_quote(&project.display().to_string()),
        powershell_quote(found)
    );
    // An extra account: its folder variable is set for this window only (Task-A03-oven548).
    let start = match config {
        Some((var, dir)) => format!(
            "${{env:{var}}} = {}; {start}",
            powershell_quote(&dir.display().to_string())
        ),
        None => start,
    };
    match alm_dir {
        Some(dir) => format!(
            "$env:PATH = {} + $env:PATH; {start}",
            powershell_quote(&format!("{};", dir.display()))
        ),
        None => start,
    }
}

/// The environment variable set on every terminal session started by automatic monitoring
/// (Task-A01-mural227): `alm claim` reads it so monitored sessions record `session.monitored: true`
/// and enable heartbeats.
pub const MONITORED_ENV: &str = "ALLOMOS_MONITORED";

/// The command with an extra account's folder variable in front of it: `CLAUDE_CONFIG_DIR='<dir>'
/// claude …` (zsh, bash), for that command only. None: the command as it is.
pub fn with_config(program: &str, config: Option<(&str, &Path)>) -> String {
    match config {
        Some((var, dir)) => format!(
            "{var}={} {program}",
            shell_quote(&dir.display().to_string())
        ),
        None => program.to_string(),
    }
}

/// The command with [`MONITORED_ENV`] set in front of it (zsh, bash), for monitored sessions only.
pub fn with_monitored(program: &str) -> String {
    format!("{MONITORED_ENV}=1 {program}")
}

/// A session ID only ever has letters, digits and `-` (a UUID): anything else is refused.
pub fn plain_session_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// **Monitor ready tasks** (Task-A03-moth285): start a new session of a terminal agent in
/// `project` with `request` already sent, in a terminal window of its own, so nobody has to press
/// Return. It never edits the user's Codex configuration; a quiet monitored Codex session gets
/// only the per-process approval and network settings approved for this task.
/// `id` is the agent (`claude_cli`, `codex_cli`, `antigravity_cli`), `found` its command's path;
/// `alm_dir` the folder of the `alm` the session should use, put first in its `PATH`; `dirs` the
/// folders an Antigravity session adds to its workspace and a Codex session may also write
/// ([`workspace_dirs`]).
#[allow(clippy::too_many_arguments)]
pub fn start_session(
    id: &str,
    found: &str,
    project: &Path,
    request: &str,
    session_id: Option<&str>,
    alm_dir: Option<&Path>,
    config: Option<(&str, &Path)>,
    quiet: bool,
    dirs: &[PathBuf],
    taskbag: Option<&Path>,
    choice: &Choice,
) -> Result<Option<crate::session_tab::SessionTab>, Problem> {
    if !cfg!(any(target_os = "macos", windows)) {
        return Err(ask_failed(id, "not available on this system yet"));
    }
    if session_id.is_some_and(|s| !plain_session_id(s)) {
        return Err(ask_failed(id, "the session ID is not a plain ID"));
    }
    let Some(program) =
        session_command_with_quiet(id, session_id, quiet, dirs, taskbag, choice, config, None)
    else {
        return Err(ask_failed(id, "no way to start it"));
    };
    // An extra account's folder variable and monitored marker go in front of the command, for
    // that command only.
    let program = with_monitored(&with_config(&program, config));
    if cfg!(windows) {
        let file = private_temp_file("allomos-request", "txt", request.as_bytes())
            .map_err(|e| ask_failed(id, e))?;
        let script = powershell_session_with_quiet(
            id, project, found, &file, session_id, alm_dir, config, quiet, dirs, taskbag, choice,
            None,
        );
        return new_console("powershell.exe")
            .args(["-NoExit", "-NoLogo", "-Command", &script])
            .spawn()
            .map(|_| None)
            .map_err(|e| {
                let _ = std::fs::remove_file(&file);
                ask_failed(id, e)
            });
    }
    // The new tab's window and terminal device are kept, to close that tab once its task is done
    // (Task-A03-sierra014). The command goes through a script file: typed into the window, a long
    // one is cut off (Task-A03-golf005, [`start_script`]).
    open_terminal(id, project, request, &program, alm_dir, false)
}

/// A program started in a console window of its own, for the person to use (Windows).
pub fn new_console(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        command.creation_flags(CREATE_NEW_CONSOLE);
    }
    command
}

/// The terminal agents found on this computer that a monitor can start: `(id, command path)`
/// for Claude Code (`claude_cli`), Codex (`codex_cli`) and Antigravity (`antigravity_cli`), in that
/// order (macOS and Windows; none elsewhere yet).
pub fn find_terminal_agents() -> Vec<(String, PathBuf)> {
    if !cfg!(any(target_os = "macos", windows)) {
        return Vec::new();
    }
    let dirs = tool_dirs(&home());
    [
        ("claude_cli", "claude"),
        ("codex_cli", "codex"),
        ("antigravity_cli", "agy"),
    ]
    .into_iter()
    .filter_map(|(id, tool)| find_tool(tool, &dirs).map(|p| (id.to_string(), p)))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permissions::{ANTIGRAVITY_PUSH_RULE, CLAUDE_ALLOWED_TOOLS, ClaudeDefaultMode};

    /// Task-A03-emu042, A03-lava982: Claude Code permission fragments for tests (explicit
    /// `ClaudeEnv`, never relying on the host computer's `~/.claude`).
    const CLAUDE_TOOLS: &str = "--allowedTools 'Bash(alm:*),Bash(cargo test:*),Bash(cargo check:*),Bash(npm test:*),Bash(npm run test:*),Bash(pnpm test:*),Bash(yarn test:*),Bash(pytest:*),Bash(go test:*)' ";
    const CLAUDE_EDITS: &str = "--allowedTools 'Bash(alm:*),Bash(cargo test:*),Bash(cargo check:*),Bash(npm test:*),Bash(npm run test:*),Bash(pnpm test:*),Bash(yarn test:*),Bash(pytest:*),Bash(go test:*)' --permission-mode acceptEdits ";
    const CLAUDE_AUTO: &str = "--allowedTools 'Bash(alm:*),Bash(cargo test:*),Bash(cargo check:*),Bash(npm test:*),Bash(npm run test:*),Bash(pnpm test:*),Bash(yarn test:*),Bash(pytest:*),Bash(go test:*)' --permission-mode auto ";

    fn claude_other() -> ClaudeEnv {
        ClaudeEnv::default()
    }

    fn claude_with_trusted(mode: ClaudeDefaultMode, trusted: &[&str]) -> ClaudeEnv {
        let mut projects = serde_json::Map::new();
        for dir in trusted {
            projects.insert(
                (*dir).to_string(),
                serde_json::json!({ "hasTrustDialogAccepted": true }),
            );
        }
        ClaudeEnv {
            default_mode: mode,
            state_json: Some(serde_json::json!({ "projects": projects }).to_string()),
        }
    }

    /// Task-A03-emu042, A03-lava982: Claude Code on Windows separates quiet monitoring (`auto`,
    /// no `--add-dir`) from normal/app launches (trusted `--add-dir` only, keeping `auto` when
    /// configured, falling back to `acceptEdits` otherwise).
    #[test]
    fn claude_on_windows_separates_quiet_and_normal_permissions() {
        let trusted_dir = PathBuf::from(r"C:\tb\.allomos\taskbag");
        let untrusted_dir = PathBuf::from(r"C:\other\.allomos\taskbag");
        let dirs = [trusted_dir.clone(), untrusted_dir];
        let found = r"C:\npm\claude.cmd";

        let other_trusted =
            claude_with_trusted(ClaudeDefaultMode::Other, &[r"C:\tb\.allomos\taskbag"]);
        let normal_other = format!(
            r"{CLAUDE_TOOLS}--add-dir 'C:\tb\.allomos\taskbag' --permission-mode acceptEdits "
        );
        assert_eq!(
            powershell_session_with_quiet(
                "claude_cli",
                Path::new(r"C:\p"),
                found,
                Path::new(r"C:\t\r.txt"),
                Some("ab-1"),
                None,
                None,
                false,
                &dirs,
                None,
                &Choice::default(),
                Some(&other_trusted),
            ),
            format!(
                r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-1' {normal_other}$request"
            )
        );

        // Normal mode with user already on auto: no downgrade to acceptEdits, `'--'` shields $request.
        let auto_trusted =
            claude_with_trusted(ClaudeDefaultMode::Auto, &[r"C:\tb\.allomos\taskbag"]);
        assert_eq!(
            powershell_session_with_quiet(
                "claude_cli",
                Path::new(r"C:\p"),
                found,
                Path::new(r"C:\t\r.txt"),
                Some("ab-1"),
                None,
                None,
                false,
                &dirs,
                None,
                &Choice::default(),
                Some(&auto_trusted),
            ),
            format!(
                r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-1' {CLAUDE_TOOLS}--add-dir 'C:\tb\.allomos\taskbag' '--' $request"
            )
        );

        // Quiet mode: `--permission-mode auto`, no `--add-dir` (even for trusted dirs).
        assert_eq!(
            powershell_session_with_quiet(
                "claude_cli",
                Path::new(r"C:\p"),
                found,
                Path::new(r"C:\t\r.txt"),
                Some("ab-1"),
                None,
                None,
                true,
                &dirs,
                None,
                &Choice::default(),
                Some(&other_trusted),
            ),
            format!(
                r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-1' {CLAUDE_AUTO}$request"
            )
        );

        // Manual app launch on Windows (`powershell_script`, pasted request): matches local env.
        let expected_script_args =
            claude_permission_args(false, &dirs, None, None, false, powershell_quote, "'--'");
        assert_eq!(
            powershell_script("claude_cli", Path::new(r"C:\p"), found, None, &dirs),
            format!(
                r"Set-Location -LiteralPath 'C:\p'; & '{found}' {}",
                expected_script_args.trim_end()
            )
        );
    }

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[cfg(unix)]
    struct TempDir(PathBuf);

    #[cfg(unix)]
    impl TempDir {
        fn new(name: &str) -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "allomos-terminal-{name}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    #[cfg(unix)]
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    fn executable(path: &Path, contents: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn a_chosen_model_and_effort_are_added_and_the_default_changes_nothing() {
        let request = REQUEST_ARG;
        let pick = |m: Option<&str>, e: Option<&str>| Choice {
            model: m.map(str::to_string),
            effort: e.map(str::to_string),
        };
        let other = claude_other();
        let cmd = |id: &str, c: &Choice| {
            session_command_with_quiet(id, Some("s-1"), false, &[], None, c, None, Some(&other))
                .unwrap()
        };
        // The default: exactly the command without a choice.
        assert_eq!(
            cmd("claude_cli", &Choice::default()),
            format!("claude --session-id 's-1' {CLAUDE_EDITS}{request}")
        );
        assert_eq!(
            cmd("claude_cli", &pick(Some("opus"), Some("high"))),
            format!(
                "claude --session-id 's-1' --model 'opus' --effort 'high' {CLAUDE_EDITS}{request}"
            )
        );
        assert_eq!(
            cmd("claude_cli", &pick(None, Some("max"))),
            format!("claude --session-id 's-1' --effort 'max' {CLAUDE_EDITS}{request}")
        );
        assert_eq!(
            cmd("codex_cli", &pick(Some("gpt-6.2-sol"), Some("xhigh"))),
            format!(
                "codex --sandbox workspace-write -m 'gpt-6.2-sol' --config 'model_reasoning_effort=xhigh' {request}"
            )
        );
        assert_eq!(
            cmd("codex_cli", &pick(Some("gpt-6-luna"), None)),
            format!("codex --sandbox workspace-write -m 'gpt-6-luna' {request}")
        );
        assert_eq!(
            cmd(
                "antigravity_cli",
                &pick(Some("gemini-4-argon"), Some("low"))
            ),
            format!("agy --model 'gemini-4-argon' --effort 'low' -i {request}")
        );
        let ps = |id: &str, c: &Choice| {
            powershell_session_with_quiet(
                id,
                Path::new(r"C:\p"),
                r"C:\bin\x.exe",
                Path::new(r"C:\t\r.txt"),
                None,
                None,
                None,
                false,
                &[],
                None,
                c,
                Some(&other),
            )
        };
        assert!(
            ps("claude_cli", &pick(Some("sonnet"), Some("low"))).contains(&format!(
                "& 'C:\\bin\\x.exe' --model 'sonnet' --effort 'low' {CLAUDE_EDITS}$request"
            ))
        );
        assert!(
            ps("codex_cli", &pick(None, Some("high")))
                .contains("--config 'model_reasoning_effort=high' $request")
        );
        assert_eq!(
            ps("claude_cli", &Choice::default()),
            ps("claude_cli", &pick(None, None))
        );
        assert!(
            ps("claude_cli", &Choice::default())
                .contains(&format!("& 'C:\\bin\\x.exe' {CLAUDE_EDITS}$request"))
        );
    }

    #[test]
    fn quoting_survives_quotes() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(applescript_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
        assert_eq!(
            powershell_quote(r"C:\Users\O'Neil\my proj"),
            r"'C:\Users\O''Neil\my proj'"
        );
        assert_eq!(powershell_quote("a\u{2019}b"), "'a\u{2019}\u{2019}b'");
    }

    /// Task-A03-moth285: a monitored session starts with the request read from its file, and
    /// Claude Code with the session ID Allomos made.
    #[test]
    fn monitored_sessions_start_with_the_request_sent() {
        let other = claude_other();
        assert_eq!(
            session_command_with_quiet(
                "claude_cli",
                Some("0b1c-2d"),
                false,
                &[],
                None,
                &Choice::default(),
                None,
                Some(&other),
            )
            .as_deref(),
            Some(
                format!("claude --session-id '0b1c-2d' {CLAUDE_EDITS}\"$allomos_request\"")
                    .as_str()
            )
        );
        assert_eq!(
            session_command("codex_cli", Some("ignored"), &[]).as_deref(),
            Some("codex --sandbox workspace-write \"$allomos_request\"")
        );
        assert_eq!(
            session_command("antigravity_cli", Some("ignored"), &[]).as_deref(),
            Some("agy -i \"$allomos_request\"")
        );
        assert_eq!(
            powershell_session(
                "antigravity_cli",
                Path::new(r"C:\p"),
                r"C:\agy\bin\agy.exe",
                Path::new(r"C:\t\r.txt"),
                None,
                None,
                None,
                &[]
            ),
            r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\agy\bin\agy.exe' -i $request"
        );
        assert_eq!(
            powershell_session(
                "codex_cli",
                Path::new(r"C:\p"),
                r"C:\npm\codex.cmd",
                Path::new(r"C:\t\r.txt"),
                Some("ignored"),
                None,
                None,
                &[]
            ),
            r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\npm\codex.cmd' --sandbox workspace-write $request"
        );
        assert_eq!(session_command("claude_app", None, &[]), None);
        assert!(plain_session_id("8f0e5a52-1c2d-4e3f-9a0b-1c2d3e4f5a6b"));
        assert!(!plain_session_id("x; rm -rf ~"));
        assert!(!plain_session_id(""));
        let found = r"C:\npm\claude.cmd";
        assert_eq!(
            powershell_session_with_quiet(
                "claude_cli",
                Path::new(r"C:\p"),
                found,
                Path::new(r"C:\t\r.txt"),
                Some("ab-12"),
                None,
                None,
                false,
                &[],
                None,
                &Choice::default(),
                Some(&other),
            ),
            format!(
                r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-12' {CLAUDE_EDITS}$request"
            )
        );
    }

    /// Task-A03-latch882: an Antigravity session adds the taskbag and the taskbag's other project
    /// folders to its workspace, one `--add-dir` each, quoted (spaces too); Claude takes none.
    #[test]
    fn antigravity_adds_the_taskbag_and_its_project_folders_to_its_workspace() {
        let request = REQUEST_ARG;
        let one = [PathBuf::from("/w/my tb/.allomos/taskbag")];
        let quiet = session_command_with_quiet(
            "antigravity_cli",
            None,
            true,
            &one,
            Some(&one[0]),
            &Choice::default(),
            None,
            None,
        )
        .expect("quiet Antigravity command");
        assert!(quiet.starts_with("agy --add-dir '/w/my tb/.allomos/taskbag' --settings '"));
        assert!(quiet.contains(ANTIGRAVITY_PUSH_RULE));
        assert!(quiet.contains(r"git -C /w/my\\x20tb/\\.allomos/taskbag push$"));
        assert!(quiet.ends_with(&format!(" -i {request}")));
        let several = [
            PathBuf::from("/w/tb/.allomos/taskbag"),
            PathBuf::from("/w/site"),
            PathBuf::from("/w/O'Neil app"),
        ];
        assert_eq!(
            session_command("antigravity_cli", None, &several).as_deref(),
            Some(
                format!(
                    "agy --add-dir '/w/tb/.allomos/taskbag' --add-dir '/w/site' --add-dir '/w/O'\\''Neil app' -i {request}"
                )
                .as_str()
            )
        );
        // Task-A03-lava982: Claude Code in normal mode adds only permanently trusted folders;
        // in quiet mode it omits --add-dir and uses --permission-mode auto.
        let trusted_all = claude_with_trusted(
            ClaudeDefaultMode::Other,
            &["/w/tb/.allomos/taskbag", "/w/site", "/w/O'Neil app"],
        );
        assert_eq!(
            session_command_with_quiet(
                "claude_cli",
                None,
                false,
                &several,
                None,
                &Choice::default(),
                None,
                Some(&trusted_all),
            )
            .as_deref(),
            Some(format!("claude {CLAUDE_TOOLS}--add-dir '/w/tb/.allomos/taskbag' --add-dir '/w/site' --add-dir '/w/O'\\''Neil app' --permission-mode acceptEdits {request}").as_str())
        );
        let trusted_one = claude_with_trusted(ClaudeDefaultMode::Auto, &["/w/site"]);
        assert_eq!(
            session_command_with_quiet(
                "claude_cli",
                None,
                false,
                &several,
                None,
                &Choice::default(),
                None,
                Some(&trusted_one),
            )
            .as_deref(),
            Some(format!("claude {CLAUDE_TOOLS}--add-dir '/w/site' -- {request}").as_str())
        );
        assert_eq!(
            session_command_with_quiet(
                "claude_cli",
                Some("ab-12"),
                true,
                &several,
                None,
                &Choice::default(),
                None,
                Some(&trusted_all),
            )
            .as_deref(),
            Some(format!("claude --session-id 'ab-12' {CLAUDE_AUTO}{request}").as_str())
        );
        let windows_one = [PathBuf::from(r"C:\my tb\.allomos\taskbag")];
        let windows_several = [
            PathBuf::from(r"C:\tb\.allomos\taskbag"),
            PathBuf::from(r"C:\site"),
        ];
        let start = r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\agy\bin\agy.exe'";
        for (dirs, args) in [
            (&windows_one[..], r"--add-dir 'C:\my tb\.allomos\taskbag' "),
            (
                &windows_several[..],
                r"--add-dir 'C:\tb\.allomos\taskbag' --add-dir 'C:\site' ",
            ),
        ] {
            let settings = antigravity_quiet_settings_arg(Some(&dirs[0]), powershell_quote);
            assert_eq!(
                powershell_session_with_quiet(
                    "antigravity_cli",
                    Path::new(r"C:\p"),
                    r"C:\agy\bin\agy.exe",
                    Path::new(r"C:\t\r.txt"),
                    None,
                    None,
                    None,
                    true,
                    dirs,
                    Some(&dirs[0]),
                    &Choice::default(),
                    None,
                ),
                format!("{start} {args}{settings}-i $request")
            );
        }
        // Opened from the app on Windows: the request is pasted, the folders are still added.
        assert_eq!(
            powershell_script(
                "antigravity_cli",
                Path::new(r"C:\p"),
                r"C:\agy\bin\agy.exe",
                None,
                &windows_several
            ),
            r"Set-Location -LiteralPath 'C:\p'; & 'C:\agy\bin\agy.exe' --add-dir 'C:\tb\.allomos\taskbag' --add-dir 'C:\site'"
        );
    }

    /// Task-A03-latch882: the folders come from the taskbag: the taskbag itself, then every
    /// project folder but the one the session starts in, absolute; a missing folder is left out.
    #[cfg(unix)]
    #[test]
    fn workspace_dirs_are_the_taskbag_and_its_other_project_folders() {
        let dir = TempDir::new("workspace");
        let w = dir.path();
        let taskbag = w.join("tb/.allomos/taskbag");
        std::fs::create_dir_all(&taskbag).unwrap();
        for f in ["app", "my site", "docs"] {
            std::fs::create_dir_all(w.join(f)).unwrap();
        }
        let write = |folders: &str| {
            std::fs::write(
                taskbag.join("project.json"),
                format!(
                    r#"{{"format":"allomos/9","id":"allomos-test-1","name":"P","labels":[],"project_folders":[{folders}]}}"#
                ),
            )
            .unwrap();
        };
        // One project folder: only the taskbag is added.
        write(r#"{"name":"app","path":"../app"}"#);
        let listed = allomos_store::folders::list(&taskbag);
        assert!(listed.is_ok(), "{listed:?}");
        assert_eq!(workspace_dirs(&taskbag, &w.join("app")), [taskbag.clone()]);
        // Several: the taskbag, then the others, in their order; a missing one is left out.
        write(
            r#"{"name":"app","path":"../app"},{"name":"site","path":"../my site"},{"name":"gone","path":"../gone"},{"name":"docs","path":"../docs"}"#,
        );
        assert_eq!(
            workspace_dirs(&taskbag, &w.join("app")),
            [taskbag.clone(), w.join("my site"), w.join("docs")]
        );
        // Started in another of them: that one is the workspace already.
        assert_eq!(
            workspace_dirs(&taskbag, &w.join("my site")),
            [taskbag.clone(), w.join("app"), w.join("docs")]
        );
    }

    /// Task-A03-bag625: a monitored Codex session makes the taskbag and the taskbag's other
    /// project folders writable in its sandbox, one `--add-dir` each after `--sandbox`, quoted,
    /// in quiet and normal mode, on macOS and on Windows.
    #[test]
    fn codex_makes_the_taskbag_and_its_project_folders_writable() {
        let request = REQUEST_ARG;
        let app_access = if cfg!(target_os = "macos") {
            "--config 'computer_use.macos.bundle_ids={\"io.allomos\"=\"allow\"}' "
        } else {
            ""
        };
        let quiet = format!(
            "--ask-for-approval never --config sandbox_workspace_write.network_access=true {app_access}"
        );
        let one = [PathBuf::from("/w/my tb/.allomos/taskbag")];
        let several = [
            PathBuf::from("/w/tb/.allomos/taskbag"),
            PathBuf::from("/w/site"),
            PathBuf::from("/w/O'Neil app"),
        ];
        assert_eq!(
            session_command_with_quiet(
                "codex_cli",
                None,
                true,
                &one,
                None,
                &Choice::default(),
                None,
                None,
            )
            .as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/my tb/.allomos/taskbag' {quiet}{request}"
                )
                .as_str()
            )
        );
        assert_eq!(
            session_command_with_quiet(
                "codex_cli",
                None,
                true,
                &several,
                None,
                &Choice::default(),
                None,
                None,
            )
            .as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/tb/.allomos/taskbag' --add-dir '/w/site' --add-dir '/w/O'\\''Neil app' {quiet}{request}"
                )
                .as_str()
            )
        );
        assert_eq!(
            session_command("codex_cli", None, &one).as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/my tb/.allomos/taskbag' {request}"
                )
                .as_str()
            )
        );
        let start = r"$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\npm\codex.cmd' --sandbox workspace-write";
        let windows_one = [PathBuf::from(r"C:\my tb\.allomos\taskbag")];
        let windows_several = [
            PathBuf::from(r"C:\tb\.allomos\taskbag"),
            PathBuf::from(r"C:\site"),
        ];
        for (dirs, args) in [
            (&windows_one[..], r"--add-dir 'C:\my tb\.allomos\taskbag' "),
            (
                &windows_several[..],
                r"--add-dir 'C:\tb\.allomos\taskbag' --add-dir 'C:\site' ",
            ),
        ] {
            assert_eq!(
                powershell_session_with_quiet(
                    "codex_cli",
                    Path::new(r"C:\p"),
                    r"C:\npm\codex.cmd",
                    Path::new(r"C:\t\r.txt"),
                    None,
                    None,
                    None,
                    true,
                    dirs,
                    None,
                    &Choice::default(),
                    None,
                ),
                format!("{start} {args}{quiet}$request")
            );
            assert_eq!(
                powershell_session(
                    "codex_cli",
                    Path::new(r"C:\p"),
                    r"C:\npm\codex.cmd",
                    Path::new(r"C:\t\r.txt"),
                    None,
                    None,
                    None,
                    dirs
                ),
                format!("{start} {args}$request")
            );
        }
    }

    #[test]
    fn quiet_monitor_codex_has_no_approval_prompts_and_its_sandbox_can_use_the_network() {
        let request = REQUEST_ARG;
        let app_access = if cfg!(target_os = "macos") {
            "--config 'computer_use.macos.bundle_ids={\"io.allomos\"=\"allow\"}' "
        } else {
            ""
        };
        let other = claude_other();
        assert_eq!(
            session_command_with_quiet(
                "codex_cli",
                None,
                true,
                &[],
                None,
                &Choice::default(),
                None,
                None,
            )
            .as_deref(),
            Some(format!(
                "codex --sandbox workspace-write --ask-for-approval never --config sandbox_workspace_write.network_access=true {app_access}{request}"
            ).as_str())
        );
        assert_eq!(
            session_command("codex_cli", None, &[]).as_deref(),
            Some(format!("codex --sandbox workspace-write {request}").as_str())
        );
        assert_eq!(
            session_command_with_quiet(
                "claude_cli",
                Some("ab-12"),
                true,
                &[PathBuf::from("/w/untrusted/.allomos/taskbag")],
                None,
                &Choice::default(),
                None,
                Some(&other),
            )
            .as_deref(),
            Some(format!("claude --session-id 'ab-12' {CLAUDE_AUTO}{request}").as_str())
        );
        assert_eq!(
            session_command_with_quiet(
                "antigravity_cli",
                None,
                true,
                &[],
                None,
                &Choice::default(),
                None,
                None,
            )
            .as_deref(),
            Some(
                format!(
                    "agy {}-i {request}",
                    antigravity_quiet_settings_arg(None, shell_quote)
                )
                .as_str()
            )
        );

        assert_eq!(
            powershell_session_with_quiet(
                "codex_cli",
                Path::new(r"C:\p"),
                r"C:\npm\codex.cmd",
                Path::new(r"C:\t\r.txt"),
                None,
                None,
                None,
                true,
                &[],
                None,
                &Choice::default(),
                None,
            ),
            format!(
                "$env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\\t\\r.txt'; Remove-Item -LiteralPath 'C:\\t\\r.txt'; & 'C:\\npm\\codex.cmd' --sandbox workspace-write --ask-for-approval never --config sandbox_workspace_write.network_access=true {app_access}$request"
            )
        );
    }

    #[test]
    fn an_extra_account_runs_the_command_in_its_own_folder() {
        let dir = Path::new("/Users/me/claude 2");
        assert_eq!(
            with_config("claude --session-id 'ab'", Some(("CLAUDE_CONFIG_DIR", dir))),
            "CLAUDE_CONFIG_DIR='/Users/me/claude 2' claude --session-id 'ab'"
        );
        assert_eq!(with_config("codex x", None), "codex x");
        assert_eq!(
            powershell_session(
                "codex_cli",
                Path::new(r"C:\p"),
                r"C:\npm\codex.cmd",
                Path::new(r"C:\t\r.txt"),
                None,
                None,
                Some(("CODEX_HOME", Path::new(r"C:\acct"))),
                &[]
            )
            .starts_with(r"${env:CODEX_HOME} = 'C:\acct'; $env:ALLOMOS_MONITORED = '1'; Set-Location -LiteralPath 'C:\p';"),
            true
        );
    }

    /// Task-A03-golf005, A03-marina834: a new Terminal window types only a short line; the start
    /// script, however long, parses in zsh and sh and hands Claude Code, Codex and Antigravity
    /// every argument and the request byte for byte (Chinese, quotes, backslashes, shell
    /// characters, line ends inside and at the end; more than the 1024-byte line limit), then
    /// both files are gone. Two requests opened one after the other do not mix.
    #[cfg(unix)]
    #[test]
    fn terminal_start_script_hands_the_agent_everything_whole() {
        let dir = TempDir::new("start");
        let project = dir.path().join("my \"proj\" it's $(x) `y`");
        let added = dir.path().join("task bag's \\ \"dir\"");
        let bin = dir.path().join("bin");
        for folder in [&project, &added, &bin] {
            std::fs::create_dir_all(folder).unwrap();
        }
        let fake = "#!/bin/sh\npwd -P > \"$ALLOMOS_TEST_OUT.pwd\"\nfor a in \"$@\"; do printf '%s\\0' \"$a\"; done > \"$ALLOMOS_TEST_OUT\"\n";
        for tool in ["claude", "codex", "agy"] {
            executable(&bin.join(tool), fake);
        }
        let request = |n: usize| {
            format!(
                "请求 {n}：任务 'A03' \"引号\" \\ back $HOME $(touch {}) `id` ; | & > <\n第二行  两个空格\n\n{}\n\n",
                shell_quote(&dir.path().join("pwned").display().to_string()),
                "接手这个任务，读完说明再动手。".repeat(40)
            )
        };
        assert!(request(1).len() > 1024);
        // As long as the person's permission rules make it (more than the 1024-byte line limit).
        let settings = format!(
            r#"{{"permissions":{{"allow":["command(regex:^git push origin it's \\+[^ ]+$)",{}]}}}}"#,
            vec![r#""command(regex:^echo x$)""#; 120].join(",")
        );
        let added_s = added.display().to_string();
        let claude_trusted = claude_with_trusted(ClaudeDefaultMode::Other, &[&added_s]);
        let claude_auto_env = claude_with_trusted(ClaudeDefaultMode::Auto, &[&added_s]);
        for shell in ["/bin/zsh", "/bin/sh"] {
            if !Path::new(shell).exists() {
                continue;
            }
            for (case, id) in [
                ("claude_normal_edits", "claude_cli"),
                ("claude_normal_auto", "claude_cli"),
                ("claude_quiet", "claude_cli"),
                ("codex", "codex_cli"),
                ("antigravity", "antigravity_cli"),
            ] {
                let mut program = match case {
                    "claude_normal_edits" => session_command_with_quiet(
                        id,
                        None,
                        false,
                        &[added.clone()],
                        None,
                        &Choice::default(),
                        None,
                        Some(&claude_trusted),
                    )
                    .unwrap(),
                    "claude_normal_auto" => session_command_with_quiet(
                        id,
                        None,
                        false,
                        &[added.clone()],
                        None,
                        &Choice::default(),
                        None,
                        Some(&claude_auto_env),
                    )
                    .unwrap(),
                    "claude_quiet" => session_command_with_quiet(
                        id,
                        None,
                        true,
                        &[added.clone()],
                        None,
                        &Choice::default(),
                        None,
                        Some(&claude_trusted),
                    )
                    .unwrap(),
                    _ => session_command(id, None, &[added.clone()]).unwrap(),
                };
                if id == "antigravity_cli" {
                    program = program.replacen(
                        " -i ",
                        &format!(" --settings {} -i ", shell_quote(&settings)),
                        1,
                    );
                }
                // Two opened before either window runs its script.
                let opened: Vec<(String, PathBuf, PathBuf)> = (1..=2)
                    .map(|n| {
                        let text = request(n);
                        let file =
                            private_temp_file("allomos-request", "txt", text.as_bytes()).unwrap();
                        let script = private_temp_file("allomos-start", "sh", b"").unwrap();
                        std::fs::write(
                            &script,
                            start_script(&script, &file, &project, &program, Some(&bin)),
                        )
                        .unwrap();
                        (text, file, script)
                    })
                    .collect();
                assert_ne!(opened[0].1, opened[1].1);
                assert_ne!(opened[0].2, opened[1].2);
                for (n, (text, file, script)) in opened.iter().enumerate() {
                    use std::os::unix::fs::PermissionsExt;
                    for f in [file, script] {
                        let mode = std::fs::metadata(f).unwrap().permissions().mode();
                        assert_eq!(mode & 0o777, 0o600, "{}", f.display());
                    }
                    let line = start_line(script);
                    assert!(line.len() < 200 && !line.contains('\n'), "{line}");
                    assert!(!line.contains("claude-cli://"));
                    let syntax = command(shell).arg("-n").arg(script).status().unwrap();
                    assert!(syntax.success(), "{shell} -n");
                    let out = dir.path().join(format!("args-{n}"));
                    let run = command(shell)
                        .arg("-c")
                        .arg(&line)
                        .env("ALLOMOS_TEST_OUT", &out)
                        .status()
                        .unwrap();
                    assert!(run.success(), "{shell} ran {line}");
                    let args = std::fs::read(&out).unwrap();
                    let args: Vec<&[u8]> = args
                        .strip_suffix(b"\0")
                        .unwrap()
                        .split(|b| *b == 0)
                        .collect();
                    let mut want: Vec<&[u8]> = match case {
                        // Task-A03-emu042, A03-lava982: each permission word reaches Claude whole.
                        "claude_normal_edits" => vec![
                            b"--allowedTools",
                            CLAUDE_ALLOWED_TOOLS.as_bytes(),
                            b"--add-dir",
                            added_s.as_bytes(),
                            b"--permission-mode",
                            b"acceptEdits",
                        ],
                        "claude_normal_auto" => vec![
                            b"--allowedTools",
                            CLAUDE_ALLOWED_TOOLS.as_bytes(),
                            b"--add-dir",
                            added_s.as_bytes(),
                            b"--",
                        ],
                        "claude_quiet" => vec![
                            b"--allowedTools",
                            CLAUDE_ALLOWED_TOOLS.as_bytes(),
                            b"--permission-mode",
                            b"auto",
                        ],
                        "codex" => vec![
                            b"--sandbox",
                            b"workspace-write",
                            b"--add-dir",
                            added_s.as_bytes(),
                        ],
                        _ => vec![
                            b"--add-dir",
                            added_s.as_bytes(),
                            b"--settings",
                            settings.as_bytes(),
                            b"-i",
                        ],
                    };
                    want.push(text.as_bytes());
                    assert_eq!(args, want, "{shell} {case} request {n}");
                    assert_eq!(
                        std::fs::read_to_string(format!("{}.pwd", out.display()))
                            .unwrap()
                            .trim_end(),
                        std::fs::canonicalize(&project)
                            .unwrap()
                            .display()
                            .to_string()
                    );
                    assert!(!script.exists() && !file.exists(), "both files removed");
                    std::fs::remove_file(&out).unwrap();
                }
            }
            assert!(
                !dir.path().join("pwned").exists(),
                "the request ran nothing"
            );
        }
    }

    /// Task-A03-marina834: a request file that cannot be read starts no agent; the window says
    /// why, and both files are removed.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_request_starts_nothing() {
        let dir = TempDir::new("unreadable");
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let out = dir.path().join("ran");
        executable(
            &bin.join("claude"),
            &format!(
                "#!/bin/sh\ntouch {}\n",
                shell_quote(&out.display().to_string())
            ),
        );
        for shell in ["/bin/zsh", "/bin/sh"] {
            if !Path::new(shell).exists() {
                continue;
            }
            let file = private_temp_file("allomos-request", "txt", b"x").unwrap();
            std::fs::remove_file(&file).unwrap();
            let script = private_temp_file("allomos-start", "sh", b"").unwrap();
            let program = session_command("claude_cli", None, &[]).unwrap();
            std::fs::write(
                &script,
                start_script(&script, &file, dir.path(), &program, Some(&bin)),
            )
            .unwrap();
            let run = command(shell)
                .arg("-c")
                .arg(start_line(&script))
                .output()
                .unwrap();
            assert!(!out.exists(), "{shell}: the agent was not started");
            assert!(
                String::from_utf8_lossy(&run.stderr).contains(START_FAILED),
                "{shell}: {}",
                String::from_utf8_lossy(&run.stderr)
            );
            assert!(!script.exists() && !file.exists());
        }
    }

    /// By hand (Task-A03-marina834): open a real Terminal tab the way the app and the monitor do.
    /// `ALLOMOS_PROBE_AGENT` (`claude_cli`, `codex_cli`, `antigravity_cli`), `ALLOMOS_PROBE_PROJECT`,
    /// `ALLOMOS_PROBE_REQUEST` (a file holding the request) and, optionally, `ALLOMOS_PROBE_BIN`
    /// (put first in the tab's `PATH`, for stand-in agents).
    #[test]
    #[ignore = "opens a Terminal window; run by hand"]
    fn probe_open_terminal() {
        let var = |k: &str| std::env::var(k).unwrap();
        let id = var("ALLOMOS_PROBE_AGENT");
        let request = std::fs::read_to_string(var("ALLOMOS_PROBE_REQUEST")).unwrap();
        let bin = std::env::var("ALLOMOS_PROBE_BIN").ok().map(PathBuf::from);
        let program = session_command(&id, None, &[]).unwrap();
        let tab = open_terminal(
            &id,
            Path::new(&var("ALLOMOS_PROBE_PROJECT")),
            &request,
            &program,
            bin.as_deref(),
            false,
        )
        .unwrap();
        println!("opened {tab:?}");
    }

    /// Task-A03-marina834: each temporary file is new, private and named apart from every other,
    /// even within one process at the same moment.
    #[test]
    fn temporary_files_are_new_private_and_unique() {
        let made: Vec<PathBuf> = (0..50)
            .map(|_| private_temp_file("allomos-test", "txt", "请求\n".as_bytes()).unwrap())
            .collect();
        let mut names: Vec<_> = made.iter().collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), made.len());
        for f in &made {
            assert_eq!(std::fs::read(f).unwrap(), "请求\n".as_bytes());
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(f).unwrap().permissions().mode();
                assert_eq!(mode & 0o777, 0o600);
            }
            std::fs::remove_file(f).unwrap();
        }
    }

    #[test]
    fn terminal_windows_put_the_apps_own_alm_first() {
        let project = Path::new("/Users/me/my proj");
        let dir = Path::new("/w/tree/target/debug");
        assert_eq!(
            terminal_script(project, "claude", Some(dir)),
            "export PATH='/w/tree/target/debug':\"$PATH\"; cd '/Users/me/my proj' && claude"
        );
        // No `alm` beside the app: `PATH` is left as it is.
        assert_eq!(
            terminal_script(project, "codex", None),
            "cd '/Users/me/my proj' && codex"
        );
        let found = r"C:\Users\me\AppData\Roaming\npm\claude.cmd";
        assert_eq!(
            powershell_script(
                "codex_cli",
                Path::new(r"C:\p"),
                found,
                Some(Path::new(r"C:\Apps\Allomos")),
                &[]
            ),
            format!(
                r"$env:PATH = 'C:\Apps\Allomos;' + $env:PATH; Set-Location -LiteralPath 'C:\p'; & '{found}'"
            )
        );
        assert_eq!(
            powershell_script("codex_cli", Path::new(r"C:\p"), found, None, &[]),
            format!(r"Set-Location -LiteralPath 'C:\p'; & '{found}'")
        );
    }

    #[test]
    fn windows_tools_are_programs_or_npm_scripts() {
        assert_eq!(
            tool_names("codex", true),
            ["codex.exe", "codex.cmd", "codex.bat"]
        );
        assert_eq!(tool_names("codex", false), ["codex"]);
    }

    #[cfg(unix)]
    #[test]
    fn node_dirs_sort_homebrew_versions_numerically_and_skip_missing_nodes() {
        let home = TempDir::new("brew-home");
        let prefix = TempDir::new("brew-prefix");
        let node20 = prefix.path().join("opt/node@20/bin");
        let node22 = prefix.path().join("opt/node@22/bin");
        let missing = prefix.path().join("opt/node@99/bin");
        executable(&node20.join("node"), "#!/bin/sh\nexit 0\n");
        executable(&node22.join("node"), "#!/bin/sh\nexit 0\n");
        std::fs::create_dir_all(&missing).unwrap();

        assert_eq!(
            node_dirs(home.path(), &[prefix.path().to_path_buf()]),
            [node22, node20]
        );
    }

    #[cfg(unix)]
    #[test]
    fn nvm_default_alias_selects_that_version_and_missing_alias_uses_the_highest() {
        let home = TempDir::new("nvm-home");
        let nvm = home.path().join(".nvm");
        let node20 = nvm.join("versions/node/v20.1.0/bin");
        let node22 = nvm.join("versions/node/v22.3.0/bin");
        let node23 = nvm.join("versions/node/v23.0.0/bin");
        executable(&node20.join("node"), "#!/bin/sh\nexit 0\n");
        executable(&node22.join("node"), "#!/bin/sh\nexit 0\n");
        executable(&node23.join("node"), "#!/bin/sh\nexit 0\n");

        let default = nvm.join("alias/default");
        std::fs::create_dir_all(default.parent().unwrap()).unwrap();
        std::fs::write(&default, "20\n").unwrap();
        assert_eq!(node_dirs(home.path(), &[]), [node20]);

        let lts = nvm.join("alias/lts");
        std::fs::create_dir_all(&lts).unwrap();
        std::fs::write(lts.join("iron"), "v20.1.0\n").unwrap();
        std::fs::write(lts.join("jod"), "v22.3.0\n").unwrap();
        std::fs::write(&default, "lts/*\n").unwrap();
        assert_eq!(node_dirs(home.path(), &[]), [node22]);

        std::fs::remove_file(default).unwrap();
        assert_eq!(node_dirs(home.path(), &[]), [node23]);
    }

    #[cfg(unix)]
    #[test]
    fn find_tool_gives_the_probe_the_candidate_path_for_env_node() {
        let dir = TempDir::new("probe-path");
        executable(
            &dir.path().join("allomos-node-probe"),
            "#!/bin/sh\nexit 0\n",
        );
        let tool = dir.path().join("allomos-tool-probe");
        executable(&tool, "#!/usr/bin/env allomos-node-probe\nexit 0\n");

        assert_eq!(
            find_tool("allomos-tool-probe", &[dir.path().to_path_buf()]),
            Some(tool)
        );
    }
}
