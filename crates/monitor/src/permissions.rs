//! What the terminal agents Allomos starts may do without asking, for that session only: the
//! folders they add, Claude Code's permissions (Task-A03-emu042), and quiet Antigravity's
//! session-only permission rules (Task-A03-candle790, A03-cobalt687).

use std::path::PathBuf;

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
pub(crate) const CLAUDE_ALLOWED_TOOLS: &str = "Bash(alm:*),Bash(cargo test:*),Bash(cargo check:*),\
Bash(npm test:*),Bash(npm run test:*),Bash(pnpm test:*),Bash(yarn test:*),Bash(pytest:*),\
Bash(go test:*)";

/// What a Claude Code session Allomos starts gets ahead of its request (Task-A03-emu042), the
/// same reach as Codex's `workspace-write`: it edits files in its project folder and in `dirs`
/// (the taskbag and its other project folders, [`crate::terminal::workspace_dirs`]) without
/// asking, and may run [`CLAUDE_ALLOWED_TOOLS`]; for this session only, the person's settings are
/// untouched.
/// `--permission-mode` comes last: `--allowedTools` and `--add-dir` take several values and
/// would swallow the request that follows.
pub(crate) fn claude_permission_args(dirs: &[PathBuf], quote: fn(&str) -> String) -> String {
    format!(
        "--allowedTools {} {}--permission-mode acceptEdits ",
        quote(CLAUDE_ALLOWED_TOOLS),
        add_dirs(dirs, quote)
    )
}

/// Only quiet, monitor-started Antigravity sessions may auto-allow an ordinary push to an
/// explicitly named origin branch. The fully qualified source keeps tags out; anchoring the
/// regex rejects options, extra arguments, and compound shell commands. Command matching cannot
/// inspect the repository's actual origin URL.
pub(crate) const ANTIGRAVITY_PUSH_RULE: &str =
    "command(regex:^git push origin refs/heads/[A-Za-z0-9][A-Za-z0-9._/-]*$)";

/// The start of a `git` command in a deny rule: alone or after other commands, with or without
/// Git's own options before the subcommand (`git -C <folder> …`, `git -c <key>=<value> …`), so
/// those cannot slip a forced push or a reset past the rules (Task-A03-candle790).
macro_rules! git_start {
    () => {
        r#"^(.*(&&|\|\||;|\|) *)?git( +-[^ ]+( +("[^"]*"|'[^']*'|[^- ][^ ]*))?)* +"#
    };
}

/// Refused at once in quiet monitored Antigravity sessions, without a prompt (Task-A03-candle790,
/// approver 2026-11-19; they asked for a confirmation before, A03-fork706): the session is told
/// and goes on, using the allowed push or parking the task with `alm wait`. Antigravity matches a
/// rule against the whole command line, so a command joined to others (`&&`, `||`, `;`, `|`) is
/// caught too. Deny wins over allow, so none of these may match [`ANTIGRAVITY_PUSH_RULE`].
pub(crate) const ANTIGRAVITY_DENY_RULES: &[&str] = &[
    // A push without a remote, or without a branch.
    "command(regex:^git push$)",
    "command(regex:^git push origin$)",
    // A short form (`git push origin main`, `HEAD`, a tag): a source not starting `refs/heads/`.
    "command(regex:^git push origin ([^r-]|r[^e]|re[^f]|ref[^s]|refs[^/]|refs/[^h]|refs/h[^e]|refs/he[^a]|refs/hea[^d]|refs/head[^s]|refs/heads[^/]).*$)",
    // Force, delete, mirror, prune, tags and other options; also after `git -C <folder>`.
    concat!(
        "command(regex:",
        git_start!(),
        "push( .*)? (--force|--force-with-lease|--force-if-includes|--delete|-f|-d|-D|--mirror|--prune|--tags|--all|--follow-tags|--set-upstream|-u|--atomic|--no-verify|--porcelain|--repo)(=| |$).*$)"
    ),
    // A forced (`+src`) or deleting / renaming (`src:dst`, `:dst`) refspec.
    concat!(
        "command(regex:",
        git_start!(),
        r"push( .*)? \+[^ ]*( .*)?$)"
    ),
    concat!(
        "command(regex:",
        git_start!(),
        "push( .*)? [^ ]*:[^ ]*( .*)?$)"
    ),
    // More than one refspec.
    "command(regex:^git push origin refs/heads/[^ ]+ .+$)",
    // A push written together with other commands.
    r"command(regex:^.*(&&|\|\||;|\|).*git push.*$)",
    r"command(regex:^.*git push.*(&&|\|\||;|\|).*$)",
    // Throwing work away, alone or within other commands.
    concat!("command(regex:", git_start!(), "reset( .*)? --hard( .*)?$)"),
    concat!(
        "command(regex:",
        git_start!(),
        "clean( .*)? (-[A-Za-z]*f[A-Za-z]*|--force)( .*)?$)"
    ),
    concat!(
        "command(regex:",
        git_start!(),
        "branch( .*)? (-[A-Za-z]*D[A-Za-z]*|--force|-f)( .*)?$)"
    ),
    r"command(regex:^(.*(&&|\|\||;|\|) *)?rm( .*)? (-[A-Za-z]*[rR][A-Za-z]*f[A-Za-z]*|-[A-Za-z]*f[A-Za-z]*[rR][A-Za-z]*)( .*)?$)",
    r"command(regex:^(.*(&&|\|\||;|\|) *)?rm( .*)? (-[rR]|--recursive)( .*)? (-f|--force)( .*)?$)",
    r"command(regex:^(.*(&&|\|\||;|\|) *)?rm( .*)? (-f|--force)( .*)? (-[rR]|--recursive)( .*)?$)",
];

/// Session-only Antigravity settings for quiet monitoring. Preserve the user's current fine-grained
/// rules because `--settings` overrides them for this session; if they cannot be read safely, omit
/// the override so the CLI keeps its usual permission behavior. `title`: the command that tells
/// the run when a confirmation still shows ([`crate::confirm`]), when there is one.
pub(crate) fn antigravity_quiet_settings(
    existing: Option<&str>,
    title: Option<&str>,
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
    if !allow
        .iter()
        .any(|rule| rule.as_str() == Some(ANTIGRAVITY_PUSH_RULE))
    {
        allow.push(serde_json::json!(ANTIGRAVITY_PUSH_RULE));
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

pub(crate) fn antigravity_quiet_settings_arg(quote: fn(&str) -> String) -> String {
    antigravity_quiet_settings_from_read(
        std::fs::read_to_string(
            crate::terminal::home().join(".gemini/antigravity-cli/settings.json"),
        ),
        crate::confirm::title_command().as_deref(),
    )
    .map(|settings| format!("--settings {} ", quote(&settings)))
    .unwrap_or_default()
}

pub(crate) fn antigravity_quiet_settings_from_read(
    existing: std::io::Result<String>,
    title: Option<&str>,
) -> Option<String> {
    match existing {
        Ok(settings) => antigravity_quiet_settings(Some(&settings), title),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            antigravity_quiet_settings(None, title)
        }
        Err(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn antigravity_quiet_settings_preserve_existing_permission_rules() {
        let global = r#"{"permissions":{"allow":["command(git status)"],"ask":["command(*)"],"deny":["command(rm -rf)"]}}"#;
        let settings: serde_json::Value =
            serde_json::from_str(&antigravity_quiet_settings(Some(global), None).unwrap()).unwrap();
        assert_eq!(
            settings["permissions"]["allow"],
            serde_json::json!(["command(git status)", ANTIGRAVITY_PUSH_RULE])
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
            &antigravity_quiet_settings(Some(&already_allowed), Some("sh '/x/title.sh'")).unwrap(),
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
        assert!(antigravity_quiet_settings(Some("not json"), None).is_none());
        assert!(
            antigravity_quiet_settings_from_read(
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "not readable",
                )),
                None
            )
            .is_none()
        );
        assert!(
            antigravity_quiet_settings_from_read(
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "not present",
                )),
                None
            )
            .is_some()
        );
    }

    /// Task-A03-candle790: the approved deny list catches each risky form, alone or joined to
    /// other commands, and none of it catches the allowed push or ordinary work.
    #[test]
    fn the_deny_rules_refuse_the_listed_commands_only() {
        let pattern = |rule: &str| {
            let inner = rule
                .strip_prefix("command(regex:")
                .and_then(|r| r.strip_suffix(')'))
                .unwrap();
            regex::Regex::new(inner).unwrap()
        };
        let deny: Vec<regex::Regex> = ANTIGRAVITY_DENY_RULES.iter().map(|r| pattern(r)).collect();
        let allow = pattern(ANTIGRAVITY_PUSH_RULE);
        let denied = |c: &str| deny.iter().any(|r| r.is_match(c));
        for c in [
            "git push",
            "git push origin",
            "git push origin main",
            "git push origin HEAD",
            "git push origin v1.0",
            "git push origin refs/tags/v1.0",
            "git push --force origin refs/heads/main",
            "git push origin --force refs/heads/main",
            "git push -f origin refs/heads/main",
            "git push --force-with-lease origin refs/heads/x",
            "git push origin --delete old",
            "git push origin :old",
            "git push origin +refs/heads/x",
            "git push origin refs/heads/a:refs/heads/main",
            "git push --mirror origin",
            "git push --tags origin",
            "git push -u origin refs/heads/x",
            "git push origin refs/heads/a refs/heads/b",
            "git push upstream refs/heads/x && echo ok",
            "git push origin refs/heads/x && git branch -d x",
            "cd sub && git push origin refs/heads/x",
            "git status; git push origin refs/heads/x",
            "git reset --hard",
            "git reset HEAD~1 --hard",
            "git stash && git reset --hard origin/main",
            "git clean -fd",
            "git clean -xdf",
            "git clean --force",
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
            "git push origin refs/heads/main 2>&1",
        ] {
            assert!(denied(c), "not denied: {c}");
        }
        for c in [
            "git push origin refs/heads/main",
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
            // How alm shares a taskbag in another folder stays a normal confirmation.
            "git -C /x push",
            "git -C tb add -A && git -C tb commit -m m && git -C tb push",
            "git commit -m 'do not push --force'",
            "git -C /x reset --soft HEAD~1",
        ] {
            assert!(!denied(c), "denied: {c}");
        }
        assert!(allow.is_match("git push origin refs/heads/main"));
    }
}
