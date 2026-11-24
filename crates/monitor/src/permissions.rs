//! What the terminal agents Allomos starts may do without asking, for that session only: the
//! folders they add, Claude Code's permissions (Task-A03-emu042, Task-A03-lava982), and quiet
//! Antigravity's session-only permission rules (Task-A03-candle790, A03-cobalt687).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// `--add-dir '<dir>' ` for each of `dirs`, quoted with `quote` (the shell's or PowerShell's).
pub(crate) fn add_dirs(dirs: &[PathBuf], quote: fn(&str) -> String) -> String {
    let mut out = String::new();
    for dir in dirs {
        out.push_str("--add-dir ");
        out.push_str(&quote(&dir.display().to_string()));
        out.push(' ');
    }
    out
}

/// The commands a Claude Code session Allomos starts may run without asking (Task-A03-emu042;
/// approver, 2026-11-20: `alm` and common test commands only; everything else still asks).
pub const CLAUDE_ALLOWED_TOOLS: &str = "Bash(alm:*),Bash(cargo test:*),Bash(cargo check:*),\
Bash(npm test:*),Bash(npm run test:*),Bash(pnpm test:*),Bash(yarn test:*),Bash(pytest:*),\
Bash(go test:*)";

/// The permission mode Claude Code already defaults to for this user/account, read (never
/// modified) from its `settings.json` and `.claude.json` (Task-A03-lava982).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ClaudeDefaultMode {
    /// `permissions.defaultMode == "bypassPermissions"` in `settings.json`: neither quiet nor
    /// normal launches downgrade it.
    BypassPermissions,
    /// Claude already defaults to `auto` (`permissions.defaultMode == "auto"` in `settings.json`,
    /// or unset while `.claude.json` records `hasSeenAutoDefaultNotice: true`): normal/desktop
    /// launches leave it alone instead of downgrading to `acceptEdits`.
    Auto,
    /// Any other default (`manual`, `default`, `acceptEdits`, `plan`, `dontAsk`, or unknown):
    /// normal/desktop launches keep `A03-emu042`'s `--permission-mode acceptEdits`, while quiet
    /// monitoring starts in `--permission-mode auto` for that session only.
    #[default]
    Other,
}

/// Read-only snapshot of Claude Code's local settings and trust state for one account
/// (Task-A03-lava982). Passed into pure command builders so tests never depend on the machine's
/// real `~/.claude`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClaudeEnv {
    pub default_mode: ClaudeDefaultMode,
    pub state_json: Option<String>,
}

/// Work out Claude Code's effective default permission mode from its `settings.json` and
/// `.claude.json` contents. An explicit `permissions.defaultMode` in `settings.json` always wins
/// over `.claude.json`'s built-in auto-default flag.
pub fn claude_default_mode(
    settings_json: Option<&str>,
    state_json: Option<&str>,
) -> ClaudeDefaultMode {
    if let Some(mode) = settings_json
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|root| {
            root.get("permissions")?
                .get("defaultMode")?
                .as_str()
                .map(str::to_string)
        })
    {
        return match mode.as_str() {
            "bypassPermissions" => ClaudeDefaultMode::BypassPermissions,
            "auto" => ClaudeDefaultMode::Auto,
            _ => ClaudeDefaultMode::Other,
        };
    }
    let built_in_auto = state_json
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|root| root.get("hasSeenAutoDefaultNotice")?.as_bool())
        .unwrap_or(false);
    if built_in_auto {
        ClaudeDefaultMode::Auto
    } else {
        ClaudeDefaultMode::Other
    }
}

/// Normalize a path string for comparing against `.claude.json`'s `projects` keys.
fn normalize_trust_key(path: &Path) -> String {
    let tidied = allomos_store::tidy(path);
    let text = tidied.display().to_string();
    if cfg!(windows) {
        let trimmed = text.trim_end_matches(['\\', '/']);
        if trimmed.is_empty() {
            text.to_lowercase()
        } else {
            trimmed.replace('/', "\\").to_lowercase()
        }
    } else if text.len() > 1 {
        text.trim_end_matches('/').to_string()
    } else {
        text
    }
}

/// Directories from `dirs` that Claude Code has permanently trusted (`hasTrustDialogAccepted:
/// true` in `.claude.json`'s `projects` map on the directory itself or any ancestor). Passing an
/// untrusted directory to interactive `claude --add-dir` stops startup with a session-only
/// "Adding a new directory" confirmation prompt (Task-A03-lava982).
pub fn claude_trusted_dirs(dirs: &[PathBuf], state_json: Option<&str>) -> Vec<PathBuf> {
    let Some(projects) = state_json
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
        .and_then(|root| root.get("projects")?.as_object().cloned())
    else {
        return Vec::new();
    };
    let mut trusted: HashSet<String> = HashSet::new();
    for (key, value) in &projects {
        if value
            .get("hasTrustDialogAccepted")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            continue;
        }
        let path = Path::new(key);
        trusted.insert(normalize_trust_key(path));
        if let Ok(canon) = std::fs::canonicalize(path) {
            trusted.insert(normalize_trust_key(&canon));
        }
    }
    if trusted.is_empty() {
        return Vec::new();
    }
    dirs.iter()
        .filter(|dir| is_trusted_dir(dir, &trusted))
        .cloned()
        .collect()
}

fn is_trusted_dir(dir: &Path, trusted: &HashSet<String>) -> bool {
    let has_trusted_ancestor = |p: &Path| {
        p.ancestors()
            .any(|anc| trusted.contains(&normalize_trust_key(anc)))
    };
    if has_trusted_ancestor(dir) {
        return true;
    }
    std::fs::canonicalize(dir)
        .ok()
        .is_some_and(|canon| has_trusted_ancestor(&canon))
}

/// Where Claude Code keeps `settings.json` and `.claude.json` for a session:
/// - Extra account (`config_dir = Some(dir)`): `dir/settings.json` and `dir/.claude.json`.
/// - Default account (`config_dir = None`): `home/.claude/settings.json` and `home/.claude.json`.
pub fn claude_config_files(home: &Path, config_dir: Option<&Path>) -> (PathBuf, PathBuf) {
    match config_dir {
        Some(dir) => (dir.join("settings.json"), dir.join(".claude.json")),
        None => (
            home.join(".claude").join("settings.json"),
            home.join(".claude.json"),
        ),
    }
}

/// Read Claude Code's local settings and trust state without modifying any file.
pub fn read_claude_env(home: &Path, config_dir: Option<&Path>) -> ClaudeEnv {
    let (settings_path, state_path) = claude_config_files(home, config_dir);
    let settings_json = std::fs::read_to_string(settings_path).ok();
    let state_json = std::fs::read_to_string(state_path).ok();
    let default_mode = claude_default_mode(settings_json.as_deref(), state_json.as_deref());
    ClaudeEnv {
        default_mode,
        state_json,
    }
}

/// Extract the `CLAUDE_CONFIG_DIR` folder from an optional `(env_var, path)` account override.
pub(crate) fn claude_config_dir<'a>(config: Option<(&str, &'a Path)>) -> Option<&'a Path> {
    config.and_then(|(var, dir)| (var == "CLAUDE_CONFIG_DIR").then_some(dir))
}

/// Resolve [`ClaudeEnv`] (either an explicit test snapshot or read from `home` / `config`) and
/// build Claude Code's session arguments via [`claude_permission_args_for`].
pub(crate) fn claude_permission_args(
    quiet: bool,
    dirs: &[PathBuf],
    config: Option<(&str, &Path)>,
    claude: Option<&ClaudeEnv>,
    has_prompt: bool,
    quote: fn(&str) -> String,
    end_of_options: &str,
) -> String {
    let owned;
    let env = match claude {
        Some(env) => env,
        None => {
            owned = read_claude_env(&crate::terminal::home(), claude_config_dir(config));
            &owned
        }
    };
    claude_permission_args_for(quiet, dirs, env, has_prompt, quote, end_of_options)
}

/// Arguments a Claude Code (terminal) session gets ahead of its request (Task-A03-emu042,
/// Task-A03-lava982), separated by launch mode:
///
/// - **Quiet monitoring (`quiet = true`)**: runs unattended in `auto` mode (`--permission-mode
///   auto`, or omits `--permission-mode` if the user already configured `bypassPermissions`),
///   pre-allows [`CLAUDE_ALLOWED_TOOLS`], and omits `--add-dir` altogether: in `auto` mode Claude
///   reads, edits and commits in external taskbags smoothly without `--add-dir`, whereas passing
///   an untrusted `--add-dir` blocks interactive startup on a session-only trust dialog.
/// - **Normal monitoring and desktop launches (`quiet = false`)**: keeps `A03-emu042`'s
///   [`CLAUDE_ALLOWED_TOOLS`] and adds `--add-dir` only for folders Claude Code already trusts
///   permanently in `.claude.json`. Uses `--permission-mode acceptEdits` unless Claude Code
///   already defaults to `auto` or `bypassPermissions`, so users on `auto` mode are never
///   downgraded.
///
/// When `--permission-mode` is omitted and a positional prompt follows (`has_prompt = true`),
/// appends `end_of_options` (`--` in POSIX shells, `'--'` in PowerShell) so variadic
/// `--allowedTools` / `--add-dir` never swallow the prompt. Never edits `~/.claude/settings.json`.
pub fn claude_permission_args_for(
    quiet: bool,
    dirs: &[PathBuf],
    env: &ClaudeEnv,
    has_prompt: bool,
    quote: fn(&str) -> String,
    end_of_options: &str,
) -> String {
    let mut out = format!("--allowedTools {} ", quote(CLAUDE_ALLOWED_TOOLS));
    if quiet {
        match env.default_mode {
            ClaudeDefaultMode::BypassPermissions => {
                if has_prompt {
                    out.push_str(end_of_options);
                    out.push(' ');
                }
            }
            ClaudeDefaultMode::Auto | ClaudeDefaultMode::Other => {
                out.push_str("--permission-mode auto ");
            }
        }
        return out;
    }
    let trusted = claude_trusted_dirs(dirs, env.state_json.as_deref());
    out.push_str(&add_dirs(&trusted, quote));
    match env.default_mode {
        ClaudeDefaultMode::Auto | ClaudeDefaultMode::BypassPermissions => {
            if has_prompt {
                out.push_str(end_of_options);
                out.push(' ');
            }
        }
        ClaudeDefaultMode::Other => {
            out.push_str("--permission-mode acceptEdits ");
        }
    }
    out
}

/// Only quiet, monitor-started Antigravity sessions may auto-allow an ordinary push to an
/// explicitly named origin branch. The fully qualified source keeps tags out; the deny rules below
/// reject options, forced/colon refspecs, and extra arguments (Antigravity splits each
/// `command(regex:...)` rule on whitespace into per-token anchored regexes matched as a prefix of
/// each parsed subcommand, so a 4-token allow rule would otherwise prefix-match a 5th token).
/// Command matching cannot inspect the repository's actual origin URL.
pub(crate) const ANTIGRAVITY_PUSH_RULE: &str =
    "command(regex:^git push origin refs/heads/[A-Za-z0-9][A-Za-z0-9._/-]*$)";

/// Forbidden `git push` flags, `+src` forced refspecs, and `src:dst` / `:dst` refspecs.
/// Contains no whitespace so Antigravity keeps it as a single token regex.
macro_rules! push_bad {
    () => {
        r"(?:(?:--force|--force-with-lease|--force-if-includes|--delete|--mirror|--prune|--tags|--all|--follow-tags|--set-upstream|--atomic|--no-verify|--porcelain|--repo)(?:=.*)?|-[A-Za-z]*[fdDu][A-Za-z]*|\+.*|.*:.*)"
    };
}

macro_rules! clean_f {
    () => {
        r"(?:-[A-Za-z]*f[A-Za-z]*|--force)"
    };
}

macro_rules! branch_d {
    () => {
        r"(?:-[A-Za-z]*D[A-Za-z]*|--force|-f)"
    };
}

macro_rules! rm_rf {
    () => {
        r"(?:-[A-Za-z]*[rR][A-Za-z]*f[A-Za-z]*|-[A-Za-z]*f[A-Za-z]*[rR][A-Za-z]*)"
    };
}

macro_rules! rm_r {
    () => {
        r"(?:-[A-Za-z]*[rR][A-Za-z]*|--recursive)"
    };
}

macro_rules! rm_f {
    () => {
        r"(?:-[A-Za-z]*f[A-Za-z]*|--force)"
    };
}

/// Refused at once in quiet monitored Antigravity sessions, without a prompt (Task-A03-candle790,
/// fixed for agy's per-token prefix matcher in A03-cobalt687, approver 2026-11-20): the session is
/// told and goes on, using the allowed push or parking the task with `alm wait`.
///
/// Antigravity splits compound commands (`&&`, `||`, `;`, `|`) into subcommands, splits each
/// `command(regex:...)` rule on whitespace into per-token anchored regexes `^(?:token)$`, and
/// matches them as a prefix of a subcommand's tokens. Therefore:
/// - No token regex here may contain a space (use `.+` to match any single token, including one
///   that held spaces when quoted in the shell).
/// - Bare `git push` and `git push origin` cannot be deny rules because a 2- or 3-token prefix
///   would also deny 4-token `git push origin refs/heads/<branch>`; they are not matched by
///   [`ANTIGRAVITY_PUSH_RULE`] and stay under normal CLI confirmation instead.
/// - Deny wins over allow, so none of these may match [`ANTIGRAVITY_PUSH_RULE`] or ordinary
///   taskbag `git -C <taskbag> add/commit/push`.
pub(crate) const ANTIGRAVITY_DENY_RULES: &[&str] = &[
    // Short-form `git push origin <target>` (`main`, `HEAD`, tags, `refs`, `refs/heads`, `+ref`, `:old`).
    "command(regex:^git push origin (?:r|re|ref|refs|refs/|refs/h|refs/he|refs/hea|refs/head|refs/heads|(?:[^r-]|r[^e]|re[^f]|ref[^s]|refs[^/]|refs/[^h]|refs/h[^e]|refs/he[^a]|refs/hea[^d]|refs/head[^s]|refs/heads[^/]).*)$)",
    // Multiple refspecs or any extra argument after `git push origin refs/heads/<branch>`.
    "command(regex:^git push origin refs/heads/.+ .+$)",
    // Force, delete, mirror, prune, tags, other push options, `+src`, and `src:dst` / `:dst`.
    concat!("command(regex:^git push ", push_bad!(), "$)"),
    concat!("command(regex:^git push .+ ", push_bad!(), "$)"),
    concat!("command(regex:^git push .+ .+ ", push_bad!(), "$)"),
    concat!("command(regex:^git push .+ .+ .+ ", push_bad!(), "$)"),
    // Same after a 1-token global option (`git --no-pager push …`).
    concat!(r"command(regex:^git -\S+ push ", push_bad!(), "$)"),
    concat!(r"command(regex:^git -\S+ push .+ ", push_bad!(), "$)"),
    concat!(r"command(regex:^git -\S+ push .+ .+ ", push_bad!(), "$)"),
    // Same after a 2-token global option (`git -C <folder> push …`, `git -c k=v push …`).
    concat!(r"command(regex:^git -\S+ .+ push ", push_bad!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ push .+ ", push_bad!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ push .+ .+ ", push_bad!(), "$)"),
    concat!(
        r"command(regex:^git -\S+ .+ push .+ .+ .+ ",
        push_bad!(),
        "$)"
    ),
    // Throwing work away (`git reset --hard`, `git clean -f`, `git branch -D`, `rm -rf`).
    "command(regex:^git reset --hard$)",
    "command(regex:^git reset .+ --hard$)",
    "command(regex:^git reset .+ .+ --hard$)",
    r"command(regex:^git -\S+ reset --hard$)",
    r"command(regex:^git -\S+ reset .+ --hard$)",
    r"command(regex:^git -\S+ .+ reset --hard$)",
    r"command(regex:^git -\S+ .+ reset .+ --hard$)",
    concat!("command(regex:^git clean ", clean_f!(), "$)"),
    concat!("command(regex:^git clean .+ ", clean_f!(), "$)"),
    concat!("command(regex:^git clean .+ .+ ", clean_f!(), "$)"),
    concat!(r"command(regex:^git -\S+ clean ", clean_f!(), "$)"),
    concat!(r"command(regex:^git -\S+ clean .+ ", clean_f!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ clean ", clean_f!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ clean .+ ", clean_f!(), "$)"),
    concat!("command(regex:^git branch ", branch_d!(), "$)"),
    concat!("command(regex:^git branch .+ ", branch_d!(), "$)"),
    concat!("command(regex:^git branch .+ .+ ", branch_d!(), "$)"),
    concat!(r"command(regex:^git -\S+ branch ", branch_d!(), "$)"),
    concat!(r"command(regex:^git -\S+ branch .+ ", branch_d!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ branch ", branch_d!(), "$)"),
    concat!(r"command(regex:^git -\S+ .+ branch .+ ", branch_d!(), "$)"),
    concat!("command(regex:^rm ", rm_rf!(), "$)"),
    concat!("command(regex:^rm .+ ", rm_rf!(), "$)"),
    concat!("command(regex:^rm ", rm_r!(), " ", rm_f!(), "$)"),
    concat!("command(regex:^rm ", rm_r!(), " .+ ", rm_f!(), "$)"),
    concat!("command(regex:^rm ", rm_f!(), " ", rm_r!(), "$)"),
    concat!("command(regex:^rm ", rm_f!(), " .+ ", rm_r!(), "$)"),
    concat!("command(regex:^rm .+ ", rm_r!(), " ", rm_f!(), "$)"),
    concat!("command(regex:^rm .+ ", rm_f!(), " ", rm_r!(), "$)"),
];

/// Escapes a literal path for use as one whitespace-free token inside `command(regex:...)`.
fn escape_regex_token(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            ' ' => out.push_str(r"\x20"),
            '\t' => out.push_str(r"\t"),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' => {
                out.push('\\');
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
}

/// Session-only allow rules for sharing this project's taskbag (`git -C <taskbag> add/commit/push`)
/// in a quiet Antigravity session (Task-A03-cobalt687, approver 2026-11-20). Global settings are
/// untouched. Paths containing whitespace or `)` use a whitespace-free token regex so Antigravity
/// keeps the path as a single `-C` argument token.
pub(crate) fn antigravity_taskbag_allow_rules(taskbag: &Path) -> Vec<String> {
    let abs = allomos_store::plain_path(&allomos_store::tidy(
        &std::path::absolute(taskbag).unwrap_or_else(|_| taskbag.to_path_buf()),
    ));
    let path = abs.display().to_string();
    let plain = !path.is_empty() && !path.chars().any(|c| c.is_whitespace() || c == ')');
    ["add", "commit", "push"]
        .into_iter()
        .map(|sub| {
            if plain {
                format!("command(git -C {path} {sub})")
            } else {
                format!(
                    "command(regex:^git -C {} {sub}$)",
                    escape_regex_token(&path)
                )
            }
        })
        .collect()
}

/// Session-only Antigravity settings for quiet monitoring. Preserve the user's current fine-grained
/// rules because `--settings` overrides them for this session; if they cannot be read safely, omit
/// the override so the CLI keeps its usual permission behavior. `title`: the command that tells
/// the run when a confirmation still shows ([`crate::confirm`]), when there is one. `taskbag`:
/// this project's taskbag folder, allowed only for `git -C <taskbag> add/commit/push`.
pub(crate) fn antigravity_quiet_settings(
    existing: Option<&str>,
    title: Option<&str>,
    taskbag: Option<&Path>,
) -> Option<String> {
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
    let mut needed_allow = vec![ANTIGRAVITY_PUSH_RULE.to_string()];
    if let Some(tb) = taskbag {
        needed_allow.extend(antigravity_taskbag_allow_rules(tb));
    }
    for rule in needed_allow {
        if !allow
            .iter()
            .any(|existing| existing.as_str() == Some(&rule))
        {
            allow.push(serde_json::json!(rule));
        }
    }
    let deny = permissions
        .entry("deny")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()?;
    for rule in ANTIGRAVITY_DENY_RULES {
        if !deny.iter().any(|existing| existing.as_str() == Some(rule)) {
            deny.push(serde_json::json!(rule));
        }
    }
    let mut out = serde_json::json!({ "permissions": permissions });
    if let Some(command) = title {
        out["title"] = serde_json::json!({ "type": "command", "command": command });
    }
    Some(out.to_string())
}

pub(crate) fn antigravity_quiet_settings_arg(
    taskbag: Option<&Path>,
    quote: fn(&str) -> String,
) -> String {
    antigravity_quiet_settings_from_read(
        std::fs::read_to_string(
            crate::terminal::home().join(".gemini/antigravity-cli/settings.json"),
        ),
        crate::confirm::title_command().as_deref(),
        taskbag,
    )
    .map(|settings| format!("--settings {} ", quote(&settings)))
    .unwrap_or_default()
}

pub(crate) fn antigravity_quiet_settings_from_read(
    existing: std::io::Result<String>,
    title: Option<&str>,
    taskbag: Option<&Path>,
) -> Option<String> {
    match existing {
        Ok(settings) => antigravity_quiet_settings(Some(&settings), title, taskbag),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            antigravity_quiet_settings(None, title, taskbag)
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::{powershell_quote, shell_quote};

    #[test]
    fn default_mode_respects_explicit_settings_and_builtin_auto_notice() {
        assert_eq!(claude_default_mode(None, None), ClaudeDefaultMode::Other);
        assert_eq!(
            claude_default_mode(Some(r#"{"permissions":{"defaultMode":"auto"}}"#), None),
            ClaudeDefaultMode::Auto
        );
        assert_eq!(
            claude_default_mode(
                Some(r#"{"permissions":{"defaultMode":"bypassPermissions"}}"#),
                None
            ),
            ClaudeDefaultMode::BypassPermissions
        );
        // Explicit non-auto mode in settings.json overrides built-in auto notice.
        assert_eq!(
            claude_default_mode(
                Some(r#"{"permissions":{"defaultMode":"manual"}}"#),
                Some(r#"{"hasSeenAutoDefaultNotice":true}"#)
            ),
            ClaudeDefaultMode::Other
        );
        // Unset defaultMode + hasSeenAutoDefaultNotice: true => Claude defaults to auto.
        assert_eq!(
            claude_default_mode(
                Some(r#"{"model":"opus"}"#),
                Some(r#"{"hasSeenAutoDefaultNotice":true}"#)
            ),
            ClaudeDefaultMode::Auto
        );
        assert_eq!(
            claude_default_mode(
                Some("not json"),
                Some(r#"{"hasSeenAutoDefaultNotice":false}"#)
            ),
            ClaudeDefaultMode::Other
        );
    }

    #[test]
    fn trusted_dirs_keep_exact_and_descendant_trusted_paths_only() {
        let state = r#"{
            "projects": {
                "/w/trusted": {"hasTrustDialogAccepted": true},
                "/w/declined": {"hasTrustDialogAccepted": false},
                "/w/missing-flag": {}
            }
        }"#;
        let dirs = vec![
            PathBuf::from("/w/trusted"),
            PathBuf::from("/w/trusted/.allomos/taskbag"),
            PathBuf::from("/w/trusted-sibling"),
            PathBuf::from("/w/declined"),
            PathBuf::from("/w/missing-flag"),
            PathBuf::from("/w/untrusted/.allomos/taskbag"),
        ];
        assert_eq!(
            claude_trusted_dirs(&dirs, Some(state)),
            vec![
                PathBuf::from("/w/trusted"),
                PathBuf::from("/w/trusted/.allomos/taskbag"),
            ]
        );
        assert!(claude_trusted_dirs(&dirs, None).is_empty());
        assert!(claude_trusted_dirs(&dirs, Some("not json")).is_empty());
    }

    #[test]
    fn config_files_follow_default_home_or_extra_account_dir() {
        let home = Path::new("/Users/me");
        assert_eq!(
            claude_config_files(home, None),
            (
                PathBuf::from("/Users/me/.claude/settings.json"),
                PathBuf::from("/Users/me/.claude.json")
            )
        );
        assert_eq!(
            claude_config_files(home, Some(Path::new("/Users/me/.claude-work"))),
            (
                PathBuf::from("/Users/me/.claude-work/settings.json"),
                PathBuf::from("/Users/me/.claude-work/.claude.json")
            )
        );
    }

    #[test]
    fn quiet_mode_uses_auto_without_add_dir_and_keeps_bypass_permissions() {
        let dirs = [PathBuf::from("/w/trusted"), PathBuf::from("/w/untrusted")];
        let state = Some(r#"{"projects":{"/w/trusted":{"hasTrustDialogAccepted":true}}}"#.into());
        let tools = format!("--allowedTools '{}' ", CLAUDE_ALLOWED_TOOLS);

        for mode in [ClaudeDefaultMode::Other, ClaudeDefaultMode::Auto] {
            let env = ClaudeEnv {
                default_mode: mode,
                state_json: state.clone(),
            };
            assert_eq!(
                claude_permission_args_for(true, &dirs, &env, true, shell_quote, "--"),
                format!("{tools}--permission-mode auto ")
            );
            assert_eq!(
                claude_permission_args_for(true, &dirs, &env, true, powershell_quote, "'--'"),
                format!("{tools}--permission-mode auto ")
            );
        }

        let bypass = ClaudeEnv {
            default_mode: ClaudeDefaultMode::BypassPermissions,
            state_json: state,
        };
        assert_eq!(
            claude_permission_args_for(true, &dirs, &bypass, true, shell_quote, "--"),
            format!("{tools}-- ")
        );
        assert_eq!(
            claude_permission_args_for(true, &dirs, &bypass, true, powershell_quote, "'--'"),
            format!("{tools}'--' ")
        );
    }

    #[test]
    fn normal_mode_adds_only_trusted_dirs_and_never_downgrades_auto() {
        let dirs = [
            PathBuf::from("/w/trusted/.allomos/taskbag"),
            PathBuf::from("/w/untrusted/.allomos/taskbag"),
        ];
        let state = Some(r#"{"projects":{"/w/trusted":{"hasTrustDialogAccepted":true}}}"#.into());
        let tools = format!("--allowedTools '{}' ", CLAUDE_ALLOWED_TOOLS);

        let other = ClaudeEnv {
            default_mode: ClaudeDefaultMode::Other,
            state_json: state.clone(),
        };
        assert_eq!(
            claude_permission_args_for(false, &dirs, &other, true, shell_quote, "--"),
            format!(
                "{tools}--add-dir '/w/trusted/.allomos/taskbag' --permission-mode acceptEdits "
            )
        );

        let auto = ClaudeEnv {
            default_mode: ClaudeDefaultMode::Auto,
            state_json: state,
        };
        // With prompt: `--` (POSIX) or `'--'` (PowerShell) stops variadic flags eating the prompt.
        assert_eq!(
            claude_permission_args_for(false, &dirs, &auto, true, shell_quote, "--"),
            format!("{tools}--add-dir '/w/trusted/.allomos/taskbag' -- ")
        );
        assert_eq!(
            claude_permission_args_for(false, &dirs, &auto, true, powershell_quote, "'--'"),
            format!("{tools}--add-dir '/w/trusted/.allomos/taskbag' '--' ")
        );
        // Without positional prompt (Windows clipboard paste): no `--` terminator needed.
        assert_eq!(
            claude_permission_args_for(false, &dirs, &auto, false, powershell_quote, "'--'"),
            format!("{tools}--add-dir '/w/trusted/.allomos/taskbag' ")
        );
    }

    /// Split a shell command into subcommands on `&&`, `||`, `;`, `|` (outside quotes) and each
    /// subcommand into unquoted tokens, mirroring how `agy 1.5.13` evaluates `command(...)` rules.
    fn shell_subcommands(cmd: &str) -> Vec<Vec<String>> {
        let mut subs = Vec::new();
        let mut cur = Vec::new();
        let mut tok = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut chars = cmd.chars().peekable();
        let flush_tok = |tok: &mut String, cur: &mut Vec<String>| {
            if !tok.is_empty() {
                cur.push(std::mem::take(tok));
            }
        };
        let flush_sub = |cur: &mut Vec<String>, subs: &mut Vec<Vec<String>>| {
            if !cur.is_empty() {
                subs.push(std::mem::take(cur));
            }
        };
        while let Some(c) = chars.next() {
            if in_single {
                if c == '\'' {
                    in_single = false;
                } else {
                    tok.push(c);
                }
                continue;
            }
            if in_double {
                if c == '"' {
                    in_double = false;
                } else if c == '\\' {
                    if let Some(n) = chars.next() {
                        tok.push(n);
                    }
                } else {
                    tok.push(c);
                }
                continue;
            }
            match c {
                '\'' => in_single = true,
                '"' => in_double = true,
                '\\' => {
                    if let Some(n) = chars.next() {
                        tok.push(n);
                    }
                }
                c if c.is_whitespace() => flush_tok(&mut tok, &mut cur),
                ';' => {
                    flush_tok(&mut tok, &mut cur);
                    flush_sub(&mut cur, &mut subs);
                }
                '&' if chars.peek() == Some(&'&') => {
                    chars.next();
                    flush_tok(&mut tok, &mut cur);
                    flush_sub(&mut cur, &mut subs);
                }
                '|' => {
                    if chars.peek() == Some(&'|') {
                        chars.next();
                    }
                    flush_tok(&mut tok, &mut cur);
                    flush_sub(&mut cur, &mut subs);
                }
                _ => tok.push(c),
            }
        }
        flush_tok(&mut tok, &mut cur);
        flush_sub(&mut cur, &mut subs);
        subs
    }

    /// Match one `command(regex:...)` or literal `command(...)` rule against one parsed subcommand
    /// the way `agy 1.5.13` does: split on whitespace into per-token matchers, matched as a prefix.
    fn agy_rule_matches_subcommand(rule: &str, tokens: &[String]) -> bool {
        let inner = rule
            .strip_prefix("command(")
            .and_then(|r| r.strip_suffix(')'))
            .expect("command(...) rule");
        if let Some(pattern) = inner.strip_prefix("regex:") {
            let parts: Vec<&str> = pattern.split_whitespace().collect();
            if tokens.len() < parts.len() {
                return false;
            }
            parts.iter().zip(tokens).all(|(part, tok)| {
                let re = regex::Regex::new(&format!("^(?:{part})$"))
                    .unwrap_or_else(|e| panic!("invalid token regex `{part}` in `{rule}`: {e}"));
                re.is_match(tok)
            })
        } else {
            let parts: Vec<&str> = inner.split_whitespace().collect();
            tokens.len() >= parts.len() && parts.iter().zip(tokens).all(|(p, t)| p == t)
        }
    }

    fn agy_matches_any_subcommand(rules: &[&str], cmd: &str) -> bool {
        shell_subcommands(cmd).iter().any(|sub| {
            rules
                .iter()
                .any(|rule| agy_rule_matches_subcommand(rule, sub))
        })
    }

    #[test]
    fn antigravity_quiet_settings_preserve_existing_permission_rules() {
        let global = r#"{"permissions":{"allow":["command(git status)"],"ask":["command(*)"],"deny":["command(rm -rf)"]}}"#;
        let tb = Path::new("/w/tb/.allomos/taskbag");
        let settings: serde_json::Value = serde_json::from_str(
            &antigravity_quiet_settings(Some(global), None, Some(tb)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            settings["permissions"]["allow"],
            serde_json::json!([
                "command(git status)",
                ANTIGRAVITY_PUSH_RULE,
                "command(git -C /w/tb/.allomos/taskbag add)",
                "command(git -C /w/tb/.allomos/taskbag commit)",
                "command(git -C /w/tb/.allomos/taskbag push)",
            ])
        );
        assert_eq!(
            settings["permissions"]["ask"],
            serde_json::json!(["command(*)"])
        );
        let mut expected_deny = vec![serde_json::json!("command(rm -rf)")];
        expected_deny.extend(
            ANTIGRAVITY_DENY_RULES
                .iter()
                .map(|rule| serde_json::json!(rule)),
        );
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::Value::Array(expected_deny)
        );
        assert!(settings.get("title").is_none());

        let already_allowed =
            format!(r#"{{"permissions":{{"allow":["{ANTIGRAVITY_PUSH_RULE}"]}}}}"#);
        let settings: serde_json::Value = serde_json::from_str(
            &antigravity_quiet_settings(Some(&already_allowed), Some("sh '/x/title.sh'"), None)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            settings["permissions"]["allow"].as_array().unwrap().len(),
            1
        );
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(ANTIGRAVITY_DENY_RULES)
        );
        assert_eq!(
            settings["title"],
            serde_json::json!({"type": "command", "command": "sh '/x/title.sh'"})
        );
        assert!(antigravity_quiet_settings(Some("not json"), None, None).is_none());
        assert!(
            antigravity_quiet_settings_from_read(
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "not readable",
                )),
                None,
                None,
            )
            .is_none()
        );
        assert!(
            antigravity_quiet_settings_from_read(
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not present",
                )),
                None,
                None,
            )
            .is_some()
        );
    }

    /// Task-A03-cobalt687: taskbag allow rules match exact absolute-path `git -C <taskbag>`
    /// add/commit/push (including paths with spaces) without allowing other repos or force-pushes.
    #[test]
    fn taskbag_allow_rules_match_plain_and_spaced_paths_only() {
        let plain_tb = Path::new("/w/allomos-taskbag/.allomos/taskbag");
        let plain_rules = antigravity_taskbag_allow_rules(plain_tb);
        let plain_refs: Vec<&str> = plain_rules.iter().map(String::as_str).collect();
        for c in [
            "git -C /w/allomos-taskbag/.allomos/taskbag add -- tasks/A03/task.json",
            "git -C /w/allomos-taskbag/.allomos/taskbag commit -m \"alm: beat\" -- tasks/A03/task.json",
            "git -C /w/allomos-taskbag/.allomos/taskbag push",
        ] {
            assert!(
                agy_matches_any_subcommand(&plain_refs, c),
                "not allowed: {c}"
            );
            assert!(
                !agy_matches_any_subcommand(ANTIGRAVITY_DENY_RULES, c),
                "denied: {c}"
            );
        }
        assert!(!agy_matches_any_subcommand(
            &plain_refs,
            "git -C /w/other-repo push"
        ));
        assert!(agy_matches_any_subcommand(
            ANTIGRAVITY_DENY_RULES,
            "git -C /w/allomos-taskbag/.allomos/taskbag push --force"
        ));

        let spaced_tb = Path::new("/w/my tb (1)/.allomos/taskbag");
        let spaced_rules = antigravity_taskbag_allow_rules(spaced_tb);
        let spaced_refs: Vec<&str> = spaced_rules.iter().map(String::as_str).collect();
        for c in [
            "git -C \"/w/my tb (1)/.allomos/taskbag\" add -- tasks/A03/task.json",
            "git -C \"/w/my tb (1)/.allomos/taskbag\" commit -m \"alm: beat\" -- tasks/A03/task.json",
            "git -C \"/w/my tb (1)/.allomos/taskbag\" push",
        ] {
            assert!(
                agy_matches_any_subcommand(&spaced_refs, c),
                "not allowed: {c}"
            );
            assert!(
                !agy_matches_any_subcommand(ANTIGRAVITY_DENY_RULES, c),
                "denied: {c}"
            );
        }
        assert!(!agy_matches_any_subcommand(
            &spaced_refs,
            "git -C \"/w/other tb/.allomos/taskbag\" push"
        ));
    }

    /// Task-A03-candle790 & A03-cobalt687: using agy's real per-token prefix matcher, every deny
    /// rule compiles per token with no internal spaces, catches every risky form, and never blocks
    /// the allowed full-refspec pushes or ordinary work.
    #[test]
    fn the_deny_rules_refuse_the_listed_commands_only() {
        let denied = |c: &str| agy_matches_any_subcommand(ANTIGRAVITY_DENY_RULES, c);
        let allowed_push = |c: &str| {
            let subs = shell_subcommands(c);
            subs.len() == 1
                && agy_rule_matches_subcommand(ANTIGRAVITY_PUSH_RULE, &subs[0])
                && !denied(c)
        };
        for c in [
            "git push origin main",
            "git push origin HEAD",
            "git push origin v1.0",
            "git push origin refs",
            "git push origin refs/heads",
            "git push origin refs/tags/v1.0",
            "git push --force origin refs/heads/main",
            "git push origin --force refs/heads/main",
            "git push origin refs/heads/main --force",
            "git push -f origin refs/heads/main",
            "git push --force-with-lease origin refs/heads/x",
            "git push --force-with-lease=main origin refs/heads/x",
            "git push origin --delete old",
            "git push origin :old",
            "git push origin +refs/heads/x",
            "git push origin refs/heads/a:refs/heads/main",
            "git push --mirror origin",
            "git push --tags origin",
            "git push -u origin refs/heads/x",
            "git push origin refs/heads/a refs/heads/b",
            "git push origin refs/heads/main 2>&1",
            "git reset --hard",
            "git reset HEAD~1 --hard",
            "git stash && git reset --hard origin/main",
            "git clean -fd",
            "git clean -xdf",
            "git clean --force",
            "git clean -d -f",
            "git branch -D old",
            "git branch --delete --force old",
            "rm -rf target",
            "rm -fr target",
            "rm -Rf target",
            "rm -r -f target",
            "rm --recursive --force target",
            "rm -f -r target",
            "cd x && rm -rf build",
            "git -C /x push --force origin refs/heads/main",
            "git -C \"my tb\" push -f origin refs/heads/main",
            "git -c a=b push origin +refs/heads/x",
            "git --no-pager push origin --delete x",
            "git -C /x push origin refs/heads/a:refs/heads/main",
            "echo && git -C y push --mirror origin",
            "git -C /x reset --hard",
            "git -C /x clean -fdx",
            "git -C /x branch -D y",
        ] {
            assert!(denied(c), "not denied: {c}");
        }
        for c in [
            "git push origin refs/heads/main",
            "git push origin refs/heads/antigravity/a03-cobalt687",
            "git push origin refs/heads/claude/a03-candle790",
            "git status",
            "git add -A",
            "git commit -m 'x && y'",
            "git switch -c claude/x",
            "git branch -d merged",
            "git reset --soft HEAD~1",
            "git clean -n",
            "rm -f file.txt",
            "rm -r emptydir",
            "cargo test -p allomos-monitor",
            // How alm shares a taskbag in another folder is never denied.
            "git -C /x push",
            "git -C tb add -A && git -C tb commit -m m && git -C tb push",
            "git commit -m 'do not push --force'",
            "git -C /x reset --soft HEAD~1",
        ] {
            assert!(!denied(c), "denied: {c}");
        }
        assert!(allowed_push("git push origin refs/heads/main"));
        assert!(allowed_push(
            "git push origin refs/heads/antigravity/a03-cobalt687"
        ));
        // Bare `git push` and `git push origin` cannot be deny prefixes without blocking valid
        // pushes, and they do not match the 4-token allow rule either (so they still prompt).
        assert!(!allowed_push("git push"));
        assert!(!allowed_push("git push origin"));
        assert!(!allowed_push("git push origin refs/heads/a refs/heads/b"));
        assert!(!allowed_push("git push origin refs/heads/main --force"));
    }
}
