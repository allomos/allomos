//! Who is behind a change, kept on this computer (Task-A01-black004, decision 0021 § 2).
//!
//! The history only ever holds a pseudonym ID (`p-7k3q2f`), one per person per project (the
//! decision's option A), so two projects cannot be linked to the same person from their files
//! alone. Allomos makes it the first time this computer writes to a project and keeps it in the
//! settings folder's `identities.json`, outside the taskbag and outside Git; who is behind it is
//! never written anywhere by Allomos. An audit joins the two.
//!
//! The same folder's `session-models.json` remembers the model an agent session named with
//! `alm --model`, and its effort, permission mode and rules (`--effort`, `--permission-mode`,
//! `--rules`, Task-A01-urchin867), so the session need not repeat them on every command.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const IDENTITIES_FILE: &str = "identities.json";
const SESSION_MODELS_FILE: &str = "session-models.json";
/// How many sessions' models are kept; the oldest go first.
const SESSION_MODELS_KEPT: usize = 200;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Identities {
    /// Project ID → this computer's person's pseudonym ID in it.
    #[serde(default)]
    pseudonyms: BTreeMap<String, String>,
    #[serde(flatten)]
    extra: allomos_core::Extra,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SessionModels {
    /// Session ID → `{model, at}`.
    #[serde(default)]
    sessions: BTreeMap<String, SessionModel>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct SessionModel {
    /// Empty when the session named only other details (files written before
    /// Task-A01-urchin867 always have it).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    model: String,
    #[serde(flatten)]
    details: SessionDetails,
    /// When it was last named, in UTC, so the oldest can be dropped.
    at: String,
}

/// What an agent session last named about itself, besides its model (Task-A01-urchin867).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rules: Option<String>,
}

fn file(name: &str) -> Option<PathBuf> {
    // Unit tests never touch the person's real settings folder: only one a test chose.
    if cfg!(any(test, feature = "test-settings"))
        && std::env::var_os("ALLOMOS_SETTINGS_DIR").is_none()
    {
        return None;
    }
    crate::settings::settings_dir().map(|d| d.join(name))
}

fn read<T: Default + for<'de> Deserialize<'de>>(name: &str) -> T {
    file(name)
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Saved like the app's settings, not through the taskbag's writer: these files are on this
/// computer only, so they are never among a change's files to commit.
fn save<T: Serialize>(name: &str, value: &T) -> Option<()> {
    let path = file(name)?;
    let dir = path.parent()?;
    fs::create_dir_all(dir).ok()?;
    let tmp = dir.join(format!("{name}.tmp-{}", std::process::id()));
    fs::write(&tmp, crate::json_text(value)).ok()?;
    crate::replace_file(&tmp, &path).ok()
}

/// This computer's person's pseudonym ID in `project`, made and saved the first time it is
/// asked for. `None` when there is no settings folder to keep it in (then nothing is recorded,
/// rather than a pseudonym that would change on the next command).
pub fn pseudonym(project: &str) -> Option<String> {
    let project = project.trim();
    if project.is_empty() {
        return None;
    }
    let mut ids: Identities = read(IDENTITIES_FILE);
    if let Some(p) = ids.pseudonyms.get(project) {
        return Some(p.clone());
    }
    let made = format!("p-{}", crate::events::random_suffix());
    ids.pseudonyms.insert(project.to_string(), made.clone());
    save(IDENTITIES_FILE, &ids)?;
    // Read it back: another writer may have saved one first.
    read::<Identities>(IDENTITIES_FILE)
        .pseudonyms
        .get(project)
        .cloned()
        .or(Some(made))
}

/// This computer's person's pseudonym ID in `project`, if one was made; never makes one. The
/// app shows the person's own name beside it: the one place the two are joined (decision 0021
/// § 2), on this computer only.
pub fn known_pseudonym(project: &str) -> Option<String> {
    read::<Identities>(IDENTITIES_FILE)
        .pseudonyms
        .get(project.trim())
        .cloned()
}

/// Where this computer keeps who is behind its pseudonyms (`identities.json`), for the app's
/// **Show in Finder**; `None` without a settings folder.
pub fn identities_file() -> Option<PathBuf> {
    file(IDENTITIES_FILE)
}

/// The model `session` last named with `alm --model`, on this computer.
pub fn session_model(session: &str) -> Option<String> {
    read::<SessionModels>(SESSION_MODELS_FILE)
        .sessions
        .get(session)
        .map(|s| s.model.clone())
        .filter(|m| !m.is_empty())
}

/// The effort, permission mode and rules `session` last named, on this computer.
pub fn session_details(session: &str) -> SessionDetails {
    read::<SessionModels>(SESSION_MODELS_FILE)
        .sessions
        .get(session)
        .map(|s| s.details.clone())
        .unwrap_or_default()
}

/// Remember what `session` named now (each detail that is `Some`), keeping what it named before
/// for the rest. Keeps the most recent sessions only.
pub fn remember(session: &str, model: Option<&str>, details: &SessionDetails) {
    let mut all: SessionModels = read(SESSION_MODELS_FILE);
    let mut entry = all.sessions.get(session).cloned().unwrap_or_default();
    let before = (entry.model.clone(), entry.details.clone());
    if let Some(model) = model {
        entry.model = model.to_string();
    }
    let d = &mut entry.details;
    for (slot, named) in [
        (&mut d.effort, &details.effort),
        (&mut d.permission_mode, &details.permission_mode),
        (&mut d.rules, &details.rules),
    ] {
        if named.is_some() {
            *slot = named.clone();
        }
    }
    if all.sessions.contains_key(session) && before == (entry.model.clone(), entry.details.clone())
    {
        return;
    }
    entry.at = crate::events::now_utc();
    all.sessions.insert(session.to_string(), entry);
    while all.sessions.len() > SESSION_MODELS_KEPT {
        let oldest = all
            .sessions
            .iter()
            .min_by(|a, b| a.1.at.cmp(&b.1.at))
            .map(|(k, _)| k.clone());
        match oldest {
            Some(k) => all.sessions.remove(&k),
            None => break,
        };
    }
    let _ = save(SESSION_MODELS_FILE, &all);
}
