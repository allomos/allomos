//! Find the AI agents on this computer and ask them to add Allomos's instructions to a
//! project ([`Task-A03-03`]). Allomos only **opens** each agent with the request typed in;
//! the person reads it and sends it, so nothing runs or uses their plan without them.
//!
//! [`Task-A03-03`]: ../../../../docs/tasks/Task-A03-03-app-asks-agents-to-set-up.md

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use allomos_core::Problem;
use serde::Serialize;

/// One agent the person can ask.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Agent {
    /// `claude_cli`, `claude_app`, `codex_cli` or `codex_app`.
    pub id: String,
    /// What was found: the command's path, or the app's name.
    pub found: String,
    /// How it will be asked: `link` (the request arrives typed in) or `paste` (a terminal opens
    /// in the project folder and the request is on the clipboard).
    pub how: String,
}

/// Folders where command-line tools usually live. A macOS app does not get the person's shell
/// path, so these are searched besides `PATH`.
fn tool_dirs(home: &Path) -> Vec<PathBuf> {
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
        dirs.push(home.join(d));
    }
    dirs.push("/opt/homebrew/bin".into());
    dirs.push("/usr/local/bin".into());
    dirs
}

/// The first `name` executable in `dirs` that answers `--version` (so a broken install is not offered).
fn find_tool(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    dirs.iter()
        .map(|d| d.join(name))
        .filter(|p| p.is_file())
        // Asked twice: a tool updating itself can fail its first answer (seen once with `claude`).
        .find(|p| runs(p) || runs(p))
}

fn runs(program: &Path) -> bool {
    let Ok(mut child) = Command::new(program)
        .arg("--version")
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

/// The URL schemes an app bundle says it opens (from its `Info.plist`, read with `plutil`).
fn app_schemes(app: &Path) -> Vec<String> {
    let Ok(out) = Command::new("/usr/bin/plutil")
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

/// The agents found on this computer. macOS only for now (Windows arrives in Iteration 2).
pub fn find_agents() -> Vec<Agent> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let tools = tool_dirs(&home);
    let apps = [PathBuf::from("/Applications"), home.join("Applications")];
    let mut agents = Vec::new();
    if let Some(claude) = find_tool("claude", &tools) {
        let how = if find_app("claude-cli", &apps).is_some() {
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
    if let Some(app) = find_app("claude", &apps) {
        agents.push(Agent {
            id: "claude_app".into(),
            found: app_name(&app),
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
    if let Some(app) = find_app("codex", &apps) {
        agents.push(Agent {
            id: "codex_app".into(),
            found: app_name(&app),
            how: "link".into(),
        });
    }
    agents
}

/// The newest change to a taskbag made by an agent through `alm --commit` / `--push`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastUse {
    /// The agent's name from the commit's `Agent:` line.
    pub who: String,
    /// When, in seconds since 1970 (the commit's time).
    pub when: i64,
}

/// Read the taskbag's Git history (read-only) for the newest commit with an `Agent:` line.
pub fn last_agent_use(taskbag: &Path) -> Option<LastUse> {
    let out = Command::new("git")
        .arg("-C")
        .arg(taskbag)
        .args(["log", "-n", "200", "--format=%ct%x1f%B%x1e", "--", "."])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_last_use(&String::from_utf8_lossy(&out.stdout))
}

/// The first commit in `git log --format=%ct%x1f%B%x1e` output whose message names an agent.
fn parse_last_use(log: &str) -> Option<LastUse> {
    log.split('\u{1e}').find_map(|commit| {
        let (when, body) = commit.trim_start().split_once('\u{1f}')?;
        let who = body
            .lines()
            .find_map(|l| l.trim().strip_prefix("Agent:"))
            .map(str::trim)
            .filter(|w| !w.is_empty() && !w.starts_with("unknown"))?;
        Some(LastUse {
            who: who.to_string(),
            when: when.trim().parse().ok()?,
        })
    })
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

fn name(agent: &str) -> &'static str {
    match agent {
        "claude_cli" => "Claude Code (terminal)",
        "claude_app" => "Claude (app)",
        "codex_cli" => "Codex (terminal)",
        "codex_app" => "Codex (app)",
        _ => "another agent",
    }
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

fn applescript_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn ask_failed(agent: &str, detail: impl ToString) -> Problem {
    Problem::OpenFailed {
        path: name(agent).to_string(),
        detail: detail.to_string(),
    }
}

/// Open one agent in `project` with `request`: through its link, or in a new Terminal window
/// with the request on the clipboard.
pub fn ask(agent: &str, project: &Path, request: &str, how: &str) -> Result<(), Problem> {
    if !cfg!(target_os = "macos") {
        return Err(ask_failed(agent, "not available on this system yet"));
    }
    if how == "link" {
        if let Some(url) = link(agent, project, request) {
            return Command::new("open")
                .arg(url)
                .status()
                .map_err(|e| ask_failed(agent, e))
                .and_then(|s| {
                    s.success()
                        .then_some(())
                        .ok_or_else(|| ask_failed(agent, "the link did not open"))
                });
        }
    }
    let program = match agent {
        "claude_cli" => "claude",
        "codex_cli" => "codex",
        _ => return Err(ask_failed(agent, "no way to open it")),
    };
    copy_to_clipboard(request).map_err(|e| ask_failed(agent, e))?;
    let script = format!(
        "tell application \"Terminal\"\nactivate\ndo script {}\nend tell",
        applescript_string(&format!(
            "cd {} && {program}",
            shell_quote(&project.display().to_string())
        ))
    );
    Command::new("/usr/bin/osascript")
        .args(["-e", &script])
        .status()
        .map_err(|e| ask_failed(agent, e))
        .and_then(|s| {
            s.success()
                .then_some(())
                .ok_or_else(|| ask_failed(agent, "Terminal did not open"))
        })
}

fn copy_to_clipboard(text: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut child = Command::new("/usr/bin/pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("piped stdin")
        .write_all(text.as_bytes())?;
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
    fn last_use_is_the_newest_commit_naming_an_agent() {
        let log = "1700000300\u{1f}fix typo\n\u{1e}\n1700000200\u{1f}alm: claim A01-02\n\nAgent: unknown (alm)\n\u{1e}\n1700000100\u{1f}alm: label A01-01\n\nAgent: claude/a01-run\n\u{1e}\n";
        assert_eq!(
            parse_last_use(log),
            Some(LastUse {
                who: "claude/a01-run".into(),
                when: 1_700_000_100
            })
        );
        assert_eq!(parse_last_use("1\u{1f}no agent\n\u{1e}"), None);
    }

    #[test]
    fn last_use_comes_from_a_real_repository() {
        let dir = std::env::temp_dir().join(format!("allomos-lastuse-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let taskbag = dir.join(".allomos/taskbag");
        std::fs::create_dir_all(&taskbag).unwrap();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&dir)
                .args(args)
                .output()
                .unwrap();
            assert!(
                ok.status.success(),
                "{}",
                String::from_utf8_lossy(&ok.stderr)
            );
        };
        git(&["init", "-q"]);
        git(&["config", "user.name", "T"]);
        git(&["config", "user.email", "t@example.com"]);
        assert_eq!(last_agent_use(&taskbag), None);
        std::fs::write(taskbag.join("project.json"), "{}").unwrap();
        git(&["add", "."]);
        git(&["commit", "-q", "-m", "alm: init", "-m", "Agent: codex/s1"]);
        assert_eq!(last_agent_use(&taskbag).unwrap().who, "codex/s1");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn quoting_survives_quotes() {
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(applescript_string(r#"a "b" \c"#), r#""a \"b\" \\c""#);
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
