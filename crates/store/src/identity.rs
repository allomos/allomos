//! Who is behind a change, kept on this computer (Task-A01-black004, decision 0021 § 2).
//!
//! The history only ever holds a pseudonym ID (`p-7k3q2f`), one per person per project (the
//! decision's option A), so two projects cannot be linked to the same person from their files
//! alone. Allomos makes it the first time this computer writes to a project and keeps it in the
//! settings folder's `identities.json`, outside the taskbag and outside Git; who is behind it is
//! never written anywhere by Allomos. An audit joins the two.
//!
//! The same folder's `session-models.json` remembers the model an agent session named with
//! `alm --model`, so the session need not repeat it on every command.

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionModel {
    model: String,
    /// When it was last named, in UTC, so the oldest can be dropped.
    at: String,
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

/// The model `session` last named with `alm --model`, on this computer.
pub fn session_model(session: &str) -> Option<String> {
    read::<SessionModels>(SESSION_MODELS_FILE)
        .sessions
        .get(session)
        .map(|s| s.model.clone())
}

/// Remember that `session` runs on `model`. Keeps the most recent sessions only.
pub fn remember_session_model(session: &str, model: &str) {
    let mut all: SessionModels = read(SESSION_MODELS_FILE);
    if all.sessions.get(session).is_some_and(|s| s.model == model) {
        return;
    }
    all.sessions.insert(
        session.to_string(),
        SessionModel {
            model: model.to_string(),
            at: crate::events::now_utc(),
        },
    );
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
