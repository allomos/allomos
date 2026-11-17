//! The terminal agents Monitor ready tasks starts (Task-A03-cow112, moved from the desktop app's
//! `agents.rs`): finding Claude Code, Codex and Antigravity's command, and starting a new session
//! of one in a terminal window of its own with the request already sent. Shared by `alm monitor`
//! and the app, so both start sessions the same way.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

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

/// What a new PowerShell window runs (Windows): the same, with `$env:PATH`. The tool is started
/// by its full path: a new window does not always get the same `PATH`.
/// `dirs`: folders an Antigravity session adds to its workspace ([`workspace_dirs`]; empty for the
/// other agents).
pub fn powershell_script(
    project: &Path,
    found: &str,
    alm_dir: Option<&Path>,
    dirs: &[PathBuf],
) -> String {
    let args = add_dirs(dirs, powershell_quote);
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

/// `--add-dir '<dir>' ` for each of `dirs`, quoted with `quote` (the shell's or PowerShell's).
fn add_dirs(dirs: &[PathBuf], quote: fn(&str) -> String) -> String {
    let mut out = String::new();
    for dir in dirs {
        out.push_str("--add-dir ");
        out.push_str(&quote(&dir.display().to_string()));
        out.push(' ');
    }
    out
}

/// Only quiet, monitor-started Antigravity sessions may auto-allow an ordinary push to an
/// explicitly named origin branch. The fully qualified source keeps tags out; anchoring the
/// regex rejects options, extra arguments, and compound shell commands. Command matching cannot
/// inspect the repository's actual origin URL.
const ANTIGRAVITY_PUSH_RULE: &str =
    "command(regex:^git push origin refs/heads/[A-Za-z0-9][A-Za-z0-9._/-]*$)";
const ANTIGRAVITY_ASK_RULES: &[&str] = &[
    "command(regex:^git push$)",
    "command(regex:^git push origin$)",
    "command(regex:^git push .* (--force|--force-with-lease|--force-if-includes|--delete|-f|-d|-D|--mirror|--prune|--tags|--all|--follow-tags|--set-upstream|-u|--atomic|--no-verify)(=| |$).*)",
    r"command(regex:^git push .* \+[^ ]+( |$).*)",
    r"command(regex:^git push .* :[^ ]+( |$).*)",
];

/// Session-only Antigravity settings for quiet monitoring. Preserve the user's current fine-grained
/// rules because `--settings` overrides them for this session; if they cannot be read safely, omit
/// the override so the CLI keeps its usual permission behavior.
fn antigravity_quiet_settings(existing: Option<&str>) -> Option<String> {
    let mut settings = match existing {
        Some(text) => serde_json::from_str::<serde_json::Value>(text).ok()?,
        None => serde_json::json!({}),
    };
    let root = settings.as_object_mut()?;
    let permissions = root
        .entry("permissions")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()?;
    let allow = permissions
        .entry("allow")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()?;
    if !allow
        .iter()
        .any(|rule| rule.as_str() == Some(ANTIGRAVITY_PUSH_RULE))
    {
        allow.push(serde_json::json!(ANTIGRAVITY_PUSH_RULE));
    }
    let ask = permissions
        .entry("ask")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()?;
    for rule in ANTIGRAVITY_ASK_RULES {
        if !ask.iter().any(|existing| existing.as_str() == Some(rule)) {
            ask.push(serde_json::json!(rule));
        }
    }
    Some(serde_json::json!({"permissions": permissions}).to_string())
}

fn antigravity_quiet_settings_arg(quote: fn(&str) -> String) -> String {
    antigravity_quiet_settings_from_read(std::fs::read_to_string(
        home().join(".gemini/antigravity-cli/settings.json"),
    ))
    .map(|settings| format!("--settings {} ", quote(&settings)))
    .unwrap_or_default()
}

fn antigravity_quiet_settings_from_read(existing: std::io::Result<String>) -> Option<String> {
    match existing {
        Ok(settings) => antigravity_quiet_settings(Some(&settings)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            antigravity_quiet_settings(None)
        }
        Err(_) => None,
    }
}

/// The command line that starts a terminal agent with the request read from `file` (and the file
/// removed once read), for Monitor ready tasks (Task-A03-moth285): `claude --session-id <id>`
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
/// silently; Claude ignores them.
pub fn session_command(
    id: &str,
    file: &Path,
    session_id: Option<&str>,
    dirs: &[PathBuf],
) -> Option<String> {
    session_command_with_quiet(id, file, session_id, false, dirs)
}

fn session_command_with_quiet(
    id: &str,
    file: &Path,
    session_id: Option<&str>,
    quiet: bool,
    dirs: &[PathBuf],
) -> Option<String> {
    let file = shell_quote(&file.display().to_string());
    let request = format!("\"$(cat {file}; rm -f {file})\"");
    match (id, session_id) {
        ("claude_cli", Some(s)) => {
            Some(format!("claude --session-id {} {request}", shell_quote(s)))
        }
        ("claude_cli", None) => Some(format!("claude {request}")),
        ("codex_cli", _) => Some(format!(
            "codex {CODEX_SANDBOX}{}{}{request}",
            add_dirs(dirs, shell_quote),
            if quiet {
                codex_quiet_policy()
            } else {
                String::new()
            }
        )),
        ("antigravity_cli", _) => Some(format!(
            "agy {}{}-i {request}",
            add_dirs(dirs, shell_quote),
            if quiet {
                antigravity_quiet_settings_arg(shell_quote)
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
        id, project, found, file, session_id, alm_dir, config, false, dirs,
    )
}

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
) -> String {
    let file = powershell_quote(&file.display().to_string());
    let session = match (id, session_id) {
        ("claude_cli", Some(s)) => format!("--session-id {} ", powershell_quote(s)),
        ("codex_cli", _) => format!(
            "{CODEX_SANDBOX}{}{}",
            add_dirs(dirs, powershell_quote),
            if quiet {
                codex_quiet_policy()
            } else {
                String::new()
            }
        ),
        ("antigravity_cli", _) => format!(
            "{}{}-i ",
            add_dirs(dirs, powershell_quote),
            if quiet {
                antigravity_quiet_settings_arg(powershell_quote)
            } else {
                String::new()
            }
        ),
        _ => String::new(),
    };
    let start = format!(
        "Set-Location -LiteralPath {}; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath {file}; Remove-Item -LiteralPath {file}; & {} {session}$request",
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
) -> Result<Option<crate::session_tab::SessionTab>, Problem> {
    if !cfg!(any(target_os = "macos", windows)) {
        return Err(ask_failed(id, "not available on this system yet"));
    }
    if session_id.is_some_and(|s| !plain_session_id(s)) {
        return Err(ask_failed(id, "the session ID is not a plain ID"));
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let file = std::env::temp_dir().join(format!("allomos-request-{stamp}.txt"));
    let Some(program) = session_command_with_quiet(id, &file, session_id, quiet, dirs) else {
        return Err(ask_failed(id, "no way to start it"));
    };
    // An extra account's folder variable goes in front of the command, for that command only.
    let program = with_config(&program, config);
    std::fs::write(&file, request).map_err(|e| ask_failed(id, e))?;
    if cfg!(windows) {
        let script = powershell_session_with_quiet(
            id, project, found, &file, session_id, alm_dir, config, quiet, dirs,
        );
        return new_console("powershell.exe")
            .args(["-NoExit", "-NoLogo", "-Command", &script])
            .spawn()
            .map(|_| None)
            .map_err(|e| ask_failed(id, e));
    }
    // The new tab's window and terminal device are kept, to close that tab once its task is done
    // (Task-A03-sierra014).
    let script = format!(
        "tell application \"Terminal\"\nset t to do script {}\nset w to (first window whose tabs contains t)\nreturn (id of w as text) & \", \" & (tty of t)\nend tell",
        applescript_string(&terminal_script(project, &program, alm_dir))
    );
    let out = command("/usr/bin/osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| ask_failed(id, e))?;
    if !out.status.success() {
        return Err(ask_failed(id, "Terminal did not open"));
    }
    Ok(crate::session_tab::parse_opened(&String::from_utf8_lossy(
        &out.stdout,
    )))
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
        let file = Path::new("/tmp/allomos-request-1.txt");
        assert_eq!(
            session_command("claude_cli", file, Some("0b1c-2d"), &[]).as_deref(),
            Some(
                "claude --session-id '0b1c-2d' \"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\""
            )
        );
        assert_eq!(
            session_command("codex_cli", file, Some("ignored"), &[]).as_deref(),
            Some(
                "codex --sandbox workspace-write \"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\""
            )
        );
        assert_eq!(
            session_command("antigravity_cli", file, Some("ignored"), &[]).as_deref(),
            Some(
                "agy -i \"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\""
            )
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
            r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\agy\bin\agy.exe' -i $request"
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
            r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\npm\codex.cmd' --sandbox workspace-write $request"
        );
        assert_eq!(session_command("claude_app", file, None, &[]), None);
        assert!(plain_session_id("8f0e5a52-1c2d-4e3f-9a0b-1c2d3e4f5a6b"));
        assert!(!plain_session_id("x; rm -rf ~"));
        assert!(!plain_session_id(""));
        let found = r"C:\npm\claude.cmd";
        assert_eq!(
            powershell_session(
                "claude_cli",
                Path::new(r"C:\p"),
                found,
                Path::new(r"C:\t\r.txt"),
                Some("ab-12"),
                None,
                None,
                &[]
            ),
            format!(
                r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-12' $request"
            )
        );
    }

    #[test]
    fn antigravity_quiet_settings_preserve_existing_permission_rules() {
        let global = r#"{"permissions":{"allow":["command(git status)"],"ask":["command(*)"],"deny":["command(rm -rf)"]}}"#;
        let settings: serde_json::Value =
            serde_json::from_str(&antigravity_quiet_settings(Some(global)).unwrap()).unwrap();
        assert_eq!(
            settings["permissions"]["allow"],
            serde_json::json!(["command(git status)", ANTIGRAVITY_PUSH_RULE])
        );
        let mut expected_ask = vec![serde_json::json!("command(*)")];
        expected_ask.extend(
            ANTIGRAVITY_ASK_RULES
                .iter()
                .map(|rule| serde_json::json!(rule)),
        );
        assert_eq!(
            settings["permissions"]["ask"],
            serde_json::Value::Array(expected_ask)
        );
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(["command(rm -rf)"])
        );

        let already_allowed =
            format!(r#"{{"permissions":{{"allow":["{ANTIGRAVITY_PUSH_RULE}"]}}}}"#);
        let settings: serde_json::Value =
            serde_json::from_str(&antigravity_quiet_settings(Some(&already_allowed)).unwrap())
                .unwrap();
        assert_eq!(
            settings["permissions"]["allow"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            settings["permissions"]["ask"],
            serde_json::json!(ANTIGRAVITY_ASK_RULES)
        );
        assert!(antigravity_quiet_settings(Some("not json")).is_none());
        assert!(
            antigravity_quiet_settings_from_read(Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "not readable",
            )))
            .is_none()
        );
        assert!(
            antigravity_quiet_settings_from_read(Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "not present",
            )))
            .is_some()
        );
    }

    /// Task-A03-latch882: an Antigravity session adds the taskbag and the taskbag's other project
    /// folders to its workspace, one `--add-dir` each, quoted (spaces too); Claude takes none.
    #[test]
    fn antigravity_adds_the_taskbag_and_its_project_folders_to_its_workspace() {
        let file = Path::new("/tmp/allomos-request-1.txt");
        let request = "\"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\"";
        let one = [PathBuf::from("/w/my tb/.allomos/taskbag")];
        let quiet = session_command_with_quiet("antigravity_cli", file, None, true, &one)
            .expect("quiet Antigravity command");
        assert!(quiet.starts_with("agy --add-dir '/w/my tb/.allomos/taskbag' --settings '"));
        assert!(quiet.contains(ANTIGRAVITY_PUSH_RULE));
        assert!(quiet.ends_with(&format!(" -i {request}")));
        let several = [
            PathBuf::from("/w/tb/.allomos/taskbag"),
            PathBuf::from("/w/site"),
            PathBuf::from("/w/O'Neil app"),
        ];
        assert_eq!(
            session_command("antigravity_cli", file, None, &several).as_deref(),
            Some(
                format!(
                    "agy --add-dir '/w/tb/.allomos/taskbag' --add-dir '/w/site' --add-dir '/w/O'\\''Neil app' -i {request}"
                )
                .as_str()
            )
        );
        assert_eq!(
            session_command("claude_cli", file, None, &several).as_deref(),
            Some(format!("claude {request}").as_str())
        );
        let windows_one = [PathBuf::from(r"C:\my tb\.allomos\taskbag")];
        let windows_several = [
            PathBuf::from(r"C:\tb\.allomos\taskbag"),
            PathBuf::from(r"C:\site"),
        ];
        let start = r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\agy\bin\agy.exe'";
        let settings = antigravity_quiet_settings_arg(powershell_quote);
        for (dirs, args) in [
            (&windows_one[..], r"--add-dir 'C:\my tb\.allomos\taskbag' "),
            (
                &windows_several[..],
                r"--add-dir 'C:\tb\.allomos\taskbag' --add-dir 'C:\site' ",
            ),
        ] {
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
                    dirs
                ),
                format!("{start} {args}{settings}-i $request")
            );
        }
        // Opened from the app on Windows: the request is pasted, the folders are still added.
        assert_eq!(
            powershell_script(
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
        let file = Path::new("/tmp/allomos-request-1.txt");
        let request = "\"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\"";
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
            session_command_with_quiet("codex_cli", file, None, true, &one).as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/my tb/.allomos/taskbag' {quiet}{request}"
                )
                .as_str()
            )
        );
        assert_eq!(
            session_command_with_quiet("codex_cli", file, None, true, &several).as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/tb/.allomos/taskbag' --add-dir '/w/site' --add-dir '/w/O'\\''Neil app' {quiet}{request}"
                )
                .as_str()
            )
        );
        assert_eq!(
            session_command("codex_cli", file, None, &one).as_deref(),
            Some(
                format!(
                    "codex --sandbox workspace-write --add-dir '/w/my tb/.allomos/taskbag' {request}"
                )
                .as_str()
            )
        );
        let start = r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\npm\codex.cmd' --sandbox workspace-write";
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
                    dirs
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
        let file = Path::new("/tmp/allomos-request-1.txt");
        let request = "\"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\"";
        let app_access = if cfg!(target_os = "macos") {
            "--config 'computer_use.macos.bundle_ids={\"io.allomos\"=\"allow\"}' "
        } else {
            ""
        };
        assert_eq!(
            session_command_with_quiet("codex_cli", file, None, true, &[]).as_deref(),
            Some(format!(
                "codex --sandbox workspace-write --ask-for-approval never --config sandbox_workspace_write.network_access=true {app_access}{request}"
            ).as_str())
        );
        assert_eq!(
            session_command("codex_cli", file, None, &[]).as_deref(),
            Some(format!("codex --sandbox workspace-write {request}").as_str())
        );
        assert_eq!(
            session_command_with_quiet("claude_cli", file, Some("ab-12"), true, &[]).as_deref(),
            Some(format!("claude --session-id 'ab-12' {request}").as_str())
        );
        assert_eq!(
            session_command_with_quiet("antigravity_cli", file, None, true, &[]).as_deref(),
            Some(
                format!(
                    "agy {}-i {request}",
                    antigravity_quiet_settings_arg(shell_quote)
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
                &[]
            ),
            format!(
                "Set-Location -LiteralPath 'C:\\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\\t\\r.txt'; Remove-Item -LiteralPath 'C:\\t\\r.txt'; & 'C:\\npm\\codex.cmd' --sandbox workspace-write --ask-for-approval never --config sandbox_workspace_write.network_access=true {app_access}$request"
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
            .starts_with(r"${env:CODEX_HOME} = 'C:\acct'; Set-Location -LiteralPath 'C:\p';"),
            true
        );
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
            powershell_script(Path::new(r"C:\p"), found, None, &[]),
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
