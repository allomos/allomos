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
/// path, so these are searched besides `PATH`.
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
    dirs
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
    dirs.iter()
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .filter(|p| p.is_file())
        // Asked twice: a tool updating itself can fail its first answer (seen once with `claude`).
        .find(|p| runs(p) || runs(p))
}

fn runs(program: &Path) -> bool {
    let Ok(mut child) = command(program)
        .arg("--version")
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
    match agent {
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
pub fn powershell_script(project: &Path, found: &str, alm_dir: Option<&Path>) -> String {
    let start = format!(
        "Set-Location -LiteralPath {}; & {}",
        powershell_quote(&project.display().to_string()),
        powershell_quote(found)
    );
    match alm_dir {
        Some(dir) => format!(
            "$env:PATH = {} + $env:PATH; {start}",
            powershell_quote(&format!("{};", dir.display()))
        ),
        None => start,
    }
}

/// The command line that starts a terminal agent with the request read from `file` (and the file
/// removed once read), for Monitor ready tasks (Task-A03-moth285): `claude --session-id <id>`
/// takes the session ID Allomos made, so the app knows the session before it claims; Codex picks
/// its own, and so does Antigravity's `agy`, which takes an initial prompt only after `-i`
/// (`--prompt-interactive`; Task-A03-queen969). The request goes through a file because a long, many-line request typed into a
/// terminal window breaks at its first line end.
pub fn session_command(id: &str, file: &Path, session_id: Option<&str>) -> Option<String> {
    let file = shell_quote(&file.display().to_string());
    let request = format!("\"$(cat {file}; rm -f {file})\"");
    match (id, session_id) {
        ("claude_cli", Some(s)) => {
            Some(format!("claude --session-id {} {request}", shell_quote(s)))
        }
        ("claude_cli", None) => Some(format!("claude {request}")),
        ("codex_cli", _) => Some(format!("codex {request}")),
        ("antigravity_cli", _) => Some(format!("agy -i {request}")),
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
) -> String {
    let file = powershell_quote(&file.display().to_string());
    let session = match (id, session_id) {
        ("claude_cli", Some(s)) => format!("--session-id {} ", powershell_quote(s)),
        ("antigravity_cli", _) => "-i ".to_string(),
        _ => String::new(),
    };
    let start = format!(
        "Set-Location -LiteralPath {}; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath {file}; Remove-Item -LiteralPath {file}; & {} {session}$request",
        powershell_quote(&project.display().to_string()),
        powershell_quote(found)
    );
    match alm_dir {
        Some(dir) => format!(
            "$env:PATH = {} + $env:PATH; {start}",
            powershell_quote(&format!("{};", dir.display()))
        ),
        None => start,
    }
}

/// A session ID only ever has letters, digits and `-` (a UUID): anything else is refused.
pub fn plain_session_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// **Monitor ready tasks** (Task-A03-moth285): start a new session of a terminal agent in
/// `project` with `request` already sent, in a terminal window of its own, so nobody has to press
/// Return. The agent's own permission settings stay as they are (Allomos never changes them).
/// `id` is the agent (`claude_cli`, `codex_cli`, `antigravity_cli`), `found` its command's path;
/// `alm_dir` the folder of the `alm` the session should use, put first in its `PATH`.
pub fn start_session(
    id: &str,
    found: &str,
    project: &Path,
    request: &str,
    session_id: Option<&str>,
    alm_dir: Option<&Path>,
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
    let Some(program) = session_command(id, &file, session_id) else {
        return Err(ask_failed(id, "no way to start it"));
    };
    std::fs::write(&file, request).map_err(|e| ask_failed(id, e))?;
    if cfg!(windows) {
        let script = powershell_session(id, project, found, &file, session_id, alm_dir);
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
            session_command("claude_cli", file, Some("0b1c-2d")).as_deref(),
            Some(
                "claude --session-id '0b1c-2d' \"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\""
            )
        );
        assert_eq!(
            session_command("codex_cli", file, Some("ignored")).as_deref(),
            Some(
                "codex \"$(cat '/tmp/allomos-request-1.txt'; rm -f '/tmp/allomos-request-1.txt')\""
            )
        );
        assert_eq!(
            session_command("antigravity_cli", file, Some("ignored")).as_deref(),
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
                None
            ),
            r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & 'C:\agy\bin\agy.exe' -i $request"
        );
        assert_eq!(session_command("claude_app", file, None), None);
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
                None
            ),
            format!(
                r"Set-Location -LiteralPath 'C:\p'; $request = Get-Content -Raw -Encoding UTF8 -LiteralPath 'C:\t\r.txt'; Remove-Item -LiteralPath 'C:\t\r.txt'; & '{found}' --session-id 'ab-12' $request"
            )
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
                Some(Path::new(r"C:\Apps\Allomos"))
            ),
            format!(
                r"$env:PATH = 'C:\Apps\Allomos;' + $env:PATH; Set-Location -LiteralPath 'C:\p'; & '{found}'"
            )
        );
        assert_eq!(
            powershell_script(Path::new(r"C:\p"), found, None),
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
}
