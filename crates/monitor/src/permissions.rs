//! What the terminal agents Allomos starts may do without asking, for that session only: the
//! folders they add and Claude Code's permissions (Task-A03-emu042, Task-A03-lava982).

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
}
