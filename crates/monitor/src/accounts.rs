//! Extra accounts of a terminal agent (Task-A03-oven548, approved by the approver, ap-zomndh,
//! 2026-11-09). One agent on this computer may have several logins (for example two Claude
//! accounts). Each extra account is a folder the person picked, with a name they gave it; Allomos
//! stores only that path and name, never reads the login files and never shows an email. A
//! session of an account is started with the agent's own folder setting: `CLAUDE_CONFIG_DIR` for
//! Claude Code, `CODEX_HOME` for Codex.
//!
//! The default account of an agent (its usual folder, `~/.claude` or `~/.codex`) needs no entry:
//! its agent ID is the plain one (`claude_cli`), as it was before accounts existed. An extra
//! account's agent ID is `<agent>:<account ID>` (`claude_cli:a19f2c3d-1f0a`), the ID the monitor
//! and the app use for it everywhere.
//!
//! The list lives in one file in Allomos's settings folder, `agent-accounts.json`, never in the
//! taskbag or a project.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's format.
pub const FORMAT: u32 = 1;
/// The longest name an account may have, in characters.
pub const NAME_MAX: usize = 40;

/// One extra account of a terminal agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    /// The account's ID: letters, digits and `-` (it is part of the agent ID).
    pub id: String,
    /// The agent: `claude_cli` or `codex_cli`.
    pub agent: String,
    /// The name the person gave it (shown in the monitor's dialog and list).
    pub name: String,
    /// The folder the agent keeps this account's settings and login in.
    pub dir: PathBuf,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct File {
    format: u32,
    #[serde(default)]
    accounts: Vec<Account>,
}

/// The file that lists the accounts: `agent-accounts.json` in Allomos's settings folder.
pub fn file_path() -> Option<PathBuf> {
    allomos_store::settings_dir().map(|d| d.join("agent-accounts.json"))
}

/// The accounts in `path`: none when the file is missing or does not read (a damaged list must
/// not stop a run; the default accounts still work).
pub fn load_at(path: &Path) -> Vec<Account> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str::<File>(&text)
        .map(|f| f.accounts)
        .unwrap_or_default()
}

/// The accounts in Allomos's settings folder (see [`load_at`]).
pub fn load() -> Vec<Account> {
    file_path().map(|p| load_at(&p)).unwrap_or_default()
}

/// Write the whole list to `path`, to a file beside it first, then renamed over it.
pub fn save_at(path: &Path, accounts: &[Account]) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let file = File {
        format: FORMAT,
        accounts: accounts.to_vec(),
    };
    let text = serde_json::to_string_pretty(&file).map_err(io::Error::other)?;
    crate::registry::write_whole(path, &text)
}

/// The agent ID of an account: `<agent>:<account ID>`.
pub fn key(agent: &str, id: &str) -> String {
    format!("{agent}:{id}")
}

/// The agent a monitor agent ID belongs to: `claude_cli` for both `claude_cli` and
/// `claude_cli:<account>`.
pub fn base_agent(id: &str) -> &str {
    id.split_once(':').map_or(id, |(agent, _)| agent)
}

/// The folder an agent reads its settings from by default, when the person has not picked
/// another (Claude Code: `~/.claude`; Codex: `$CODEX_HOME` or `~/.codex`).
pub fn default_dir(agent: &str, home: &Path) -> Option<PathBuf> {
    match agent {
        "claude_cli" => Some(home.join(".claude")),
        "codex_cli" => Some(
            std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex")),
        ),
        _ => None,
    }
}

/// The environment variable an agent reads its settings folder from.
pub fn config_var(agent: &str) -> Option<&'static str> {
    match agent {
        "claude_cli" => Some("CLAUDE_CONFIG_DIR"),
        "codex_cli" => Some("CODEX_HOME"),
        _ => None,
    }
}

/// The folder and variable a session of the monitor agent `id` starts with: none for a default
/// account (the agent's own setting stays as it is), the account's folder for an extra one. An
/// account that is no longer listed, or whose folder is gone, is an error: the session is not
/// started with another account by mistake.
pub fn config_of(
    id: &str,
    accounts: &[Account],
) -> Result<Option<(&'static str, PathBuf)>, String> {
    let Some((_, account_id)) = id.split_once(':') else {
        return Ok(None);
    };
    let base = base_agent(id);
    let var = config_var(base).ok_or_else(|| format!("{base} has no account folder"))?;
    let account = accounts
        .iter()
        .find(|a| a.agent == base && a.id == account_id)
        .ok_or_else(|| format!("the account {account_id} is not listed any more"))?;
    if !account.dir.is_dir() {
        return Err(format!(
            "the folder of account {} ({}) is not there any more",
            account.name,
            account.dir.display()
        ));
    }
    Ok(Some((var, account.dir.clone())))
}

/// Whether the monitor agent `id` may be used now: a default agent, or an extra account that is
/// listed and whose folder is there.
pub fn usable(id: &str, accounts: &[Account]) -> bool {
    config_of(id, accounts).is_ok()
}

/// A new account ID: `a` and the time in hex, with a few random digits so two added in one
/// millisecond differ.
pub fn new_id(now_ms: i64) -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_i64(now_ms);
    format!("a{:x}-{:04x}", now_ms, h.finish() & 0xffff)
}

/// Why a folder cannot become an account, or the account it would be (not yet saved). Checks:
/// the agent takes accounts, the name reads, the folder exists and is a folder, and it is not the
/// default folder of the agent or of another account already listed. `home` is the person's home
/// folder (the default folders are under it).
pub fn check_new(
    accounts: &[Account],
    agent: &str,
    dir: &Path,
    name: &str,
    home: &Path,
    now_ms: i64,
) -> Result<Account, String> {
    if config_var(agent).is_none() {
        return Err(format!("{agent} does not take extra accounts"));
    }
    let name = name.trim();
    if name.is_empty() {
        return Err("the account needs a name".to_string());
    }
    if name.chars().count() > NAME_MAX || name.contains(['\n', '\r']) {
        return Err(format!(
            "the name is longer than {NAME_MAX} characters or has a line break"
        ));
    }
    if !dir.is_dir() {
        return Err(format!("{} is not a folder", dir.display()));
    }
    let picked = fs::canonicalize(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let same = |other: &Path| fs::canonicalize(other).is_ok_and(|o| o == picked);
    if default_dir(agent, home).is_some_and(|d| same(&d)) {
        return Err(
            "that is the agent's default folder, which is already the first account".to_string(),
        );
    }
    if accounts.iter().any(|a| a.agent == agent && same(&a.dir)) {
        return Err("that folder is already an account".to_string());
    }
    Ok(Account {
        id: new_id(now_ms),
        agent: agent.to_string(),
        name: name.to_string(),
        dir: picked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("allomos-accounts-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn account(id: &str, agent: &str, dir: &Path) -> Account {
        Account {
            id: id.into(),
            agent: agent.into(),
            name: "Account 2".into(),
            dir: dir.to_path_buf(),
        }
    }

    #[test]
    fn keys_and_their_agent() {
        assert_eq!(key("claude_cli", "a1-2"), "claude_cli:a1-2");
        assert_eq!(base_agent("claude_cli:a1-2"), "claude_cli");
        assert_eq!(base_agent("codex_cli"), "codex_cli");
    }

    #[test]
    fn a_default_agent_has_no_folder_to_set() {
        assert_eq!(config_of("claude_cli", &[]), Ok(None));
    }

    #[test]
    fn an_account_sets_its_agents_folder_variable() {
        let home = tmp("env");
        let accounts = vec![account("x1", "claude_cli", &home)];
        let got = config_of("claude_cli:x1", &accounts).unwrap().unwrap();
        assert_eq!(got.0, "CLAUDE_CONFIG_DIR");
        assert_eq!(got.1, home);
        let codex = vec![account("y1", "codex_cli", &home)];
        assert_eq!(
            config_of("codex_cli:y1", &codex).unwrap().unwrap().0,
            "CODEX_HOME"
        );
    }

    #[test]
    fn an_unlisted_or_missing_account_is_refused_not_replaced_by_the_default() {
        let home = tmp("missing");
        assert!(config_of("claude_cli:nope", &[]).is_err());
        let gone = home.join("gone");
        let accounts = vec![account("x1", "claude_cli", &gone)];
        assert!(config_of("claude_cli:x1", &accounts).is_err());
        assert!(!usable("claude_cli:x1", &accounts));
    }

    #[test]
    fn the_list_round_trips_through_its_file() {
        let dir = tmp("file");
        let path = dir.join("agent-accounts.json");
        assert!(load_at(&path).is_empty());
        let list = vec![account("x1", "claude_cli", &dir)];
        save_at(&path, &list).unwrap();
        assert_eq!(load_at(&path), list);
        fs::write(&path, "not json").unwrap();
        assert!(load_at(&path).is_empty());
    }

    #[test]
    fn a_new_account_needs_a_folder_a_name_and_no_duplicate() {
        let home = tmp("home");
        fs::create_dir_all(home.join(".claude")).unwrap();
        let second = home.join("claude-2");
        fs::create_dir_all(&second).unwrap();
        let ok = check_new(&[], "claude_cli", &second, "  Account 2 ", &home, 1_000).unwrap();
        assert_eq!(ok.name, "Account 2");
        assert!(ok.id.starts_with('a'));
        // The default folder is already the first account.
        let err = check_new(&[], "claude_cli", &home.join(".claude"), "B", &home, 1_000);
        assert!(err.unwrap_err().contains("default"));
        // The same folder twice.
        let listed = vec![ok.clone()];
        assert!(check_new(&listed, "claude_cli", &second, "C", &home, 1_000).is_err());
        // No name, a file, an agent without accounts.
        assert!(check_new(&[], "claude_cli", &second, "  ", &home, 1_000).is_err());
        assert!(check_new(&[], "claude_cli", &home.join("nothing"), "D", &home, 1_000).is_err());
        assert!(check_new(&[], "antigravity_cli", &second, "E", &home, 1_000).is_err());
    }
}
