//! What the terminal agents Allomos starts may do without asking, for that session only: the
//! folders they add and Claude Code's permissions (Task-A03-emu042).

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
