//! Find the AI agents on this computer and ask them to add Allomos's instructions to a
//! project (task A03-03). Allomos only **opens** each agent with the request typed in;
//! the person reads it and sends it, so nothing runs or uses their plan without them.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use serde::Serialize;

// Starting terminal agents moved to the monitor crate, shared with `alm monitor` (Task-A03-cow112).
pub use allomos_monitor::terminal::{
    applescript_string, command, new_console, powershell_quote, shell_quote,
};
use allomos_monitor::terminal::{
    ask_failed, find_tool, home, name, powershell_script, session_command, terminal_script, tool_dirs,
};

/// One agent the person can ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Agent {
    /// `claude_cli`, `claude_app`, `codex_cli`, `codex_app` or `antigravity_cli` (Google
    /// Antigravity's `agy`, Task-A03-queen969).
    pub id: String,
    /// What was found: the command's path, or the app's name.
    pub found: String,
    /// How it will be asked: `link` (the request arrives typed in) or `paste` (a terminal opens
    /// in the project folder and the request is on the clipboard).
    pub how: String,
}

/// The URL schemes an app bundle says it opens (from its `Info.plist`, read with `plutil`).
fn app_schemes(app: &Path) -> Vec<String> {
    let Ok(out) = command("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(app.join("Contents/Info.plist"))
        .output()
    else {
        return Vec::new();
    };
    let Ok(info) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else {
        return Vec::new();
    };
    info["CFBundleURLTypes"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|t| {
            t["CFBundleURLSchemes"]
                .as_array()
                .cloned()
                .unwrap_or_default()
        })
        .filter_map(|s| s.as_str().map(str::to_lowercase))
        .collect()
}

/// Apps known to open each agent's links, checked first: reading every app's `Info.plist` takes
/// about two seconds on a full Applications folder.
const KNOWN_APPS: &[(&str, &[&str])] = &[
    ("claude", &["Claude.app"]),
    ("claude-cli", &["Claude Code URL Handler.app"]),
    ("codex", &["Codex.app", "ChatGPT.app"]),
];

/// The first app in `dirs` that opens `scheme://` links: a known app first, else any app.
fn find_app(scheme: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    let known = KNOWN_APPS
        .iter()
        .filter(|(s, _)| *s == scheme)
        .flat_map(|(_, names)| names.iter())
        .flat_map(|name| dirs.iter().map(move |d| d.join(name)))
        .find(|app| app_schemes(app).iter().any(|s| s == scheme));
    known.or_else(|| scan_apps(scheme, dirs))
}

fn scan_apps(scheme: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "app"))
        .find(|app| app_schemes(app).iter().any(|s| s == scheme))
}

fn app_name(app: &Path) -> String {
    app.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The app Windows opens `scheme://` links with, as the person would name it (`Claude`), read
/// from the registry with `reg` (which looks at the person's own and the computer's settings).
fn registered_app(scheme: &str) -> Option<String> {
    let out = command("reg")
        .args([
            "query",
            &format!(r"HKCR\{scheme}\shell\open\command"),
            "/ve",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value = reg_default_value(&String::from_utf8_lossy(&out.stdout))?;
    let name = program_name(&value)?;
    let mut chars = name.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// The default value in `reg query … /ve` output: the text after its type on the value's line.
/// The value's own name (`(Default)`) is translated on some systems, so the type is looked for.
fn reg_default_value(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        ["REG_EXPAND_SZ", "REG_SZ"].iter().find_map(|kind| {
            let (_, rest) = line.split_once(&format!("    {kind}    "))?;
            let rest = rest.trim();
            (!rest.is_empty()).then(|| rest.to_string())
        })
    })
}

/// The program's file name without `.exe` in a Windows command line such as
/// `"C:\Program Files\Claude\claude.exe" --open-url "%1"`.
fn program_name(command_line: &str) -> Option<String> {
    let program = match command_line.strip_prefix('"') {
        Some(rest) => rest.split('"').next()?,
        None => {
            let lower = command_line.to_lowercase();
            match lower.find(".exe") {
                Some(i) => &command_line[..i + 4],
                None => command_line.split_whitespace().next()?,
            }
        }
    };
    let file = program.rsplit(['\\', '/']).next()?;
    let stem = file
        .strip_suffix(".exe")
        .or_else(|| file.strip_suffix(".EXE"))
        .unwrap_or(file);
    (!stem.is_empty()).then(|| stem.to_string())
}

/// The app that opens `scheme://` links, by name: on macOS from the Applications folders, on
/// Windows from the registry.
fn link_app(scheme: &str, home: &Path) -> Option<String> {
    if cfg!(windows) {
        registered_app(scheme)
    } else {
        let apps = [PathBuf::from("/Applications"), home.join("Applications")];
        find_app(scheme, &apps).map(|app| app_name(&app))
    }
}

/// The agents found on this computer (macOS and Windows).
pub fn find_agents() -> Vec<Agent> {
    if !cfg!(any(target_os = "macos", windows)) {
        return Vec::new();
    }
    let home = home();
    let tools = tool_dirs(&home);
    let mut agents = Vec::new();
    if let Some(claude) = find_tool("claude", &tools) {
        let how = if link_app("claude-cli", &home).is_some() {
            "link"
        } else {
            "paste"
        };
        agents.push(Agent {
            id: "claude_cli".into(),
            found: claude.display().to_string(),
            how: how.into(),
        });
    }
    if let Some(app) = link_app("claude", &home) {
        agents.push(Agent {
            id: "claude_app".into(),
            found: app,
            how: "link".into(),
        });
    }
    if let Some(codex) = find_tool("codex", &tools) {
        agents.push(Agent {
            id: "codex_cli".into(),
            found: codex.display().to_string(),
            how: "paste".into(),
        });
    }
    if let Some(app) = link_app("codex", &home) {
        agents.push(Agent {
            id: "codex_app".into(),
            found: app,
            how: "link".into(),
        });
    }
    // Google Antigravity's command-line agent (Task-A03-queen969): its installer puts `agy` in
    // `~/.local/bin`. The Antigravity app and editor plug-ins are not offered (approver,
    // 2026-11-07).
    if let Some(agy) = find_tool("agy", &tools) {
        agents.push(Agent {
            id: "antigravity_cli".into(),
            found: agy.display().to_string(),
            how: "paste".into(),
        });
    }
    agents
}

/// Percent-encode a link's query value (everything but unreserved characters).
pub fn encode(text: &str) -> String {
    let mut out = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The link that opens `agent` in `project` with `request` typed in, if it has one.
pub fn link(agent: &str, project: &Path, request: &str) -> Option<String> {
    let folder = encode(&project.display().to_string());
    let q = encode(request);
    match agent {
        "claude_cli" => Some(format!("claude-cli://open?cwd={folder}&q={q}")),
        "claude_app" => Some(format!("claude://code/new?folder={folder}&q={q}")),
        "codex_app" => Some(format!("codex://threads/new?path={folder}&prompt={q}")),
        _ => None,
    }
}

/// A line added to the request when several agents are asked at once, so they do not write twice.
pub fn shared_note(agents: &[String], language: &str) -> Option<String> {
    if agents.len() < 2 {
        return None;
    }
    let names: Vec<&str> = agents.iter().map(|a| name(a)).collect();
    Some(match language {
        "zh-CN" => format!(
            "注意：{} 同时收到了这个请求。请先阅读 AGENTS.md 和 CLAUDE.md；\
如果其中已经有 Allomos 的说明，只需检查，不要做其他改动。",
            names.join("和")
        ),
        "ja" => format!(
            "注意：{} にも同時に同じ依頼をしています。まず AGENTS.md と CLAUDE.md を読み、\
Allomos の説明がすでにあれば確認だけにして、ほかは変えないでください。",
            names.join("と")
        ),
        _ => format!(
            "Note: {} were asked to do this at the same time. Read AGENTS.md and CLAUDE.md first; \
if Allomos's instructions are already there, only check them and change nothing else.",
            names.join(" and ")
        ),
    })
}

/// For an agent opened through a link, which starts outside any window the app controls: the
/// line that names the app's own `alm` by its full path, only when the person's shell would find
/// a different one (task A03-16; the owner's wording, 2026-10-14). `None` with no `alm` beside
/// the app.
fn own_alm_line(own: Option<&Path>, shell: Option<&Path>) -> Option<String> {
    let own = own?;
    (shell != Some(own)).then(|| {
        format!(
            "In this task, run `alm` as `{}`, every time, never as plain `alm`.",
            own.display()
        )
    })
}

/// `request` with [`own_alm_line`] first, when there is one.
fn with_own_alm(request: &str, own: Option<&Path>, shell: Option<&Path>) -> String {
    match own_alm_line(own, shell) {
        Some(line) => format!("{line}\n\n{request}"),
        None => request.to_string(),
    }
}

/// The request as an agent opened through a link gets it: with [`own_alm_line`] first when the
/// person's shell would find another `alm`. The **Manual** choice (Task-A05-06) shows the same
/// text, since the AI it is pasted into also starts outside any window the app controls.
pub fn link_request(request: &str, own: Option<&Path>) -> String {
    match own {
        Some(own) => with_own_alm(
            request,
            Some(own),
            crate::alm_command::shell_alm().as_deref(),
        ),
        None => request.to_string(),
    }
}

/// Put `text` on the clipboard (the **Copy** button of the Manual choice, Task-A05-06).
pub fn copy_text(text: &str) -> Result<(), Problem> {
    copy_to_clipboard(text).map_err(|e| Problem::OpenFailed {
        path: "clipboard".to_string(),
        detail: e.to_string(),
    })
}

/// Open one agent in `project` with `request`: through its link, or in a new terminal window
/// with the request on the clipboard. Either way the agent is pointed at the app's own `alm`
/// (Task-A03-16).
pub fn ask(agent: &Agent, project: &Path, request: &str) -> Result<(), Problem> {
    let id = agent.id.as_str();
    if !cfg!(any(target_os = "macos", windows)) {
        return Err(ask_failed(id, "not available on this system yet"));
    }
    let own_alm = crate::alm_command::own();
    if agent.how == "link" {
        let request = link_request(request, own_alm.as_deref());
        if let Some(url) = link(id, project, &request) {
            return open_link(&url).map_err(|e| ask_failed(id, e));
        }
    }
    let alm_dir = own_alm.as_deref().and_then(Path::parent);
    let program = match id {
        "claude_cli" => "claude".to_string(),
        "codex_cli" => "codex".to_string(),
        "antigravity_cli" => "agy".to_string(),
        _ => return Err(ask_failed(id, "no way to open it")),
    };
    // Antigravity's `agy` takes the request on its command line and sends it at once (the person
    // chose this, 2026-11-09). On macOS it goes through a file, as the monitor does, so the
    // clipboard is not used. Other agents, and Windows, still put it on the clipboard to paste.
    let program = if id == "antigravity_cli" && !cfg!(windows) {
        let file = std::env::temp_dir().join(format!("allomos-request-{}.md", std::process::id()));
        std::fs::write(&file, request).map_err(|e| ask_failed(id, e))?;
        session_command(id, &file, None).ok_or_else(|| ask_failed(id, "no way to open it"))?
    } else {
        copy_to_clipboard(request).map_err(|e| ask_failed(id, e))?;
        program
    };
    if cfg!(windows) {
        let script = powershell_script(project, &agent.found, alm_dir);
        return new_console("powershell.exe")
            .args(["-NoExit", "-NoLogo", "-Command", &script])
            .spawn()
            .map(drop)
            .map_err(|e| ask_failed(id, e));
    }
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
        applescript_string(&terminal_script(project, &program, alm_dir))
    );
    command("/usr/bin/osascript")
        .args(["-e", &script])
        .status()
        .map_err(|e| ask_failed(id, e))
        .and_then(|s| {
            s.success()
                .then_some(())
                .ok_or_else(|| ask_failed(id, "Terminal did not open"))
        })
}

/// Open `url` with the app the system has for its scheme.
pub(crate) fn open_link(url: &str) -> Result<(), String> {
    if cfg!(windows) {
        // Windows' own handler for links; it reports no useful exit status, so starting it counts.
        return command("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", url])
            .spawn()
            .map(drop)
            .map_err(|e| e.to_string());
    }
    let status = command("open")
        .arg(url)
        .status()
        .map_err(|e| e.to_string())?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "the link did not open".to_string())
}
/// `text` as Windows' `clip` reads Unicode: UTF-16 with a byte-order mark. Without the mark it
/// reads the console's code page, which turns Chinese and Japanese into question marks.
fn utf16_with_bom(text: &str) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
    bytes
}

fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let (program, bytes) = if cfg!(windows) {
        ("clip.exe", utf16_with_bom(text))
    } else {
        ("/usr/bin/pbcopy", text.as_bytes().to_vec())
    };
    let mut child = command(program)
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("piped stdin").write_all(&bytes)?;
    child.wait().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_carry_the_folder_and_request_encoded() {
        let p = Path::new("/Users/a b/proj");
        assert_eq!(encode("a b/ü&"), "a%20b%2F%C3%BC%26");
        assert_eq!(
            link("claude_cli", p, "Hi & bye").unwrap(),
            "claude-cli://open?cwd=%2FUsers%2Fa%20b%2Fproj&q=Hi%20%26%20bye"
        );
        assert_eq!(
            link("claude_app", p, "x").unwrap(),
            "claude://code/new?folder=%2FUsers%2Fa%20b%2Fproj&q=x"
        );
        assert_eq!(
            link("codex_app", p, "x").unwrap(),
            "codex://threads/new?path=%2FUsers%2Fa%20b%2Fproj&prompt=x"
        );
        assert_eq!(link("codex_cli", p, "x"), None);
    }

    #[test]
    fn several_agents_are_told_about_each_other() {
        assert_eq!(shared_note(&["claude_cli".into()], "en"), None);
        let both = ["claude_cli".to_string(), "codex_app".to_string()];
        let note = shared_note(&both, "en").unwrap();
        assert!(note.contains("Claude Code (terminal) and Codex (app)"));
        assert!(shared_note(&both, "ja").unwrap().starts_with("注意："));
        assert!(
            shared_note(&both, "zh-CN")
                .unwrap()
                .contains("同时收到了这个请求")
        );
    }

    #[test]
    fn tools_that_do_not_run_are_not_offered() {
        let dir = std::env::temp_dir().join(format!("allomos-agents-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A file named like the tool that is not a working program.
        std::fs::write(dir.join("codex"), "not a program").unwrap();
        assert_eq!(find_tool("codex", std::slice::from_ref(&dir)), None);
        assert_eq!(find_app("codex", std::slice::from_ref(&dir)), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn link_requests_name_the_apps_alm_only_when_the_shell_finds_another() {
        let ours = Path::new("/w/tree/target/debug/alm");
        let installed = Path::new("/Applications/Allomos.app/Contents/MacOS/alm");
        assert_eq!(
            with_own_alm("Do it.", Some(ours), Some(installed)),
            "In this task, run `alm` as `/w/tree/target/debug/alm`, every time, never as plain `alm`.\n\nDo it."
        );
        // The shell finds no `alm` at all: named too.
        assert!(with_own_alm("Do it.", Some(ours), None).starts_with("In this task"));
        // The same one: nothing added.
        assert_eq!(with_own_alm("Do it.", Some(ours), Some(ours)), "Do it.");
        // No `alm` beside the app: nothing added.
        assert_eq!(with_own_alm("Do it.", None, Some(installed)), "Do it.");
    }

    #[test]
    fn link_registrations_name_their_app() {
        let out = "\r\nHKEY_CLASSES_ROOT\\claude\\shell\\open\\command\r\n    (Default)    REG_SZ    \"C:\\Users\\王\\AppData\\Local\\AnthropicClaude\\claude.exe\" --protocol-launcher \"%1\"\r\n\r\n";
        let value = reg_default_value(out).unwrap();
        assert!(value.starts_with("\"C:\\Users\\"));
        assert_eq!(program_name(&value).as_deref(), Some("claude"));
        // A translated value name and an unquoted path with spaces.
        let out =
            "    (默认)    REG_EXPAND_SZ    %LOCALAPPDATA%\\Programs\\Codex App\\Codex.exe %1\r\n";
        let value = reg_default_value(out).unwrap();
        assert_eq!(program_name(&value).as_deref(), Some("Codex"));
        assert_eq!(reg_default_value("ERROR: not found"), None);
        assert_eq!(reg_default_value("    (Default)    REG_SZ    \r\n"), None);
    }

    #[test]
    fn windows_clipboard_text_is_utf16_with_a_mark() {
        assert_eq!(utf16_with_bom("a中"), [0xFF, 0xFE, b'a', 0, 0x2D, 0x4E]);
    }
}

#[cfg(test)]
mod probe {
    #[test]
    #[ignore = "looks at this computer; run by hand"]
    fn print_found_agents() {
        let start = std::time::Instant::now();
        let found = super::find_agents();
        println!("find_agents took {} ms", start.elapsed().as_millis());
        for a in found {
            println!("{a:?}");
        }
    }
}
