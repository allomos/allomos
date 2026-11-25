//! The online model list (A03-alto709, approver 2026-11-17): the models and reasoning efforts
//! the **Monitor ready tasks** dialog offers for each agent. It is one JSON file on the web (by
//! default `https://www.allomos.com/agents/models.json`, made by A03-coral174; the person may set
//! another address in **Settings… › General › Model list address**). The app reads it with one
//! plain GET, carrying no personal details, each time the dialog opens and when its Refresh is
//! clicked; a list read whole is kept on this computer, and used when the web cannot be read.
//! Asked with `curl`, as the update check does, so the app needs no web library.

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_monitor::flow::plain_choice;
use serde::Serialize;
use serde_json::Value;

/// Allomos's own list.
pub const DEFAULT_URL: &str = "https://www.allomos.com/agents/models.json";
/// The list's formats this app reads (its `version`). In format 1 an agent has one list of
/// reasoning efforts for all its models; in format 2 (A03-pony586) a model may carry its own,
/// because agents such as Antigravity offer different levels per model, and a model that takes no
/// reasoning effort says so with an empty list. A model that names none uses its agent's.
const FORMATS: [u64; 2] = [1, 2];
/// The copy kept on this computer, in the settings folder.
const CACHE_FILE: &str = "agent-models.json";
const TIMEOUT_SECS: &str = "10";

/// One model as the dialog offers it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Model {
    pub value: String,
    pub label: String,
    /// The reasoning efforts this model offers: its own when the list gives it one, else its
    /// agent's. Empty: the model takes no reasoning effort, so the dialog offers Default only.
    pub efforts: Vec<String>,
}

/// One agent of the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ListedAgent {
    pub id: String,
    pub name: String,
    pub models: Vec<Model>,
    pub efforts: Vec<String>,
}

/// What the dialog gets: the list (none when neither the web nor the copy could be read), where
/// it came from, and why the web could not be read, if it could not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelsRead {
    pub agents: Option<Vec<ListedAgent>>,
    /// `online`, `cache` or `none`.
    pub from: &'static str,
    /// `fetch` (no answer: offline, or the address is wrong) or `format` (not a list this app
    /// reads: not JSON, or another format version). None: read from the web.
    pub failed: Option<&'static str>,
    pub url: String,
}

/// The list in `text`, or `format` when it is not one this app reads. Models and efforts that are
/// not one plain word are left out (they go into the command that starts the agent), and so are
/// agents with no ID; fields this app does not know are ignored.
pub fn parse(text: &str) -> Result<Vec<ListedAgent>, &'static str> {
    let v: Value = serde_json::from_str(text).map_err(|_| "format")?;
    if !FORMATS.contains(&v["version"].as_u64().unwrap_or_default()) {
        return Err("format");
    }
    let agents = v["agents"].as_array().ok_or("format")?;
    let words = |x: &Value| -> Vec<String> {
        x.as_array()
            .into_iter()
            .flatten()
            .filter_map(|e| e.as_str())
            .map(str::trim)
            .filter(|e| plain_choice(e))
            .map(str::to_string)
            .collect()
    };
    Ok(agents
        .iter()
        .filter_map(|a| {
            let id = a["id"].as_str()?.trim().to_string();
            if id.is_empty() {
                return None;
            }
            let efforts = words(&a["efforts"]);
            let models = a["models"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| {
                    let value = m["value"].as_str()?.trim();
                    plain_choice(value).then(|| Model {
                        efforts: if m["efforts"].is_array() { words(&m["efforts"]) } else { efforts.clone() },
                        value: value.to_string(),
                        label: m["label"]
                            .as_str()
                            .map(str::trim)
                            .filter(|l| !l.is_empty())
                            .unwrap_or(value)
                            .to_string(),
                    })
                })
                .collect();
            Some(ListedAgent {
                name: a["name"].as_str().unwrap_or(&id).to_string(),
                id,
                models,
                efforts,
            })
        })
        .collect())
}

/// A web address the app may read the list from: `https://` or `http://`, then a host, and no
/// spaces.
pub fn plain_url(url: &str) -> bool {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    rest.is_some_and(|r| {
        !r.is_empty()
            && !r.starts_with('/')
            && url.len() <= 2048
            && !url.chars().any(|c| c.is_whitespace() || c.is_control())
    })
}

/// The address in use: the one set in Settings, or Allomos's own.
pub fn url() -> String {
    allomos_store::models_url().unwrap_or_else(|| DEFAULT_URL.to_string())
}

/// The list, read from `url` with `fetch`, else from the copy in `dir` kept for that address.
/// A list read whole from the web replaces the copy.
pub fn read_with(
    url: &str,
    dir: Option<&Path>,
    fetch: impl FnOnce(&str) -> Option<String>,
) -> ModelsRead {
    let failed = match fetch(url)
        .ok_or("fetch")
        .and_then(|t| parse(&t).map(|a| (t, a)))
    {
        Ok((text, agents)) => {
            if let Some(dir) = dir {
                save_copy(dir, url, &text);
            }
            return ModelsRead {
                agents: Some(agents),
                from: "online",
                failed: None,
                url: url.to_string(),
            };
        }
        Err(why) => why,
    };
    let copy = dir.and_then(|d| read_copy(d, url));
    ModelsRead {
        from: if copy.is_some() { "cache" } else { "none" },
        agents: copy,
        failed: Some(failed),
        url: url.to_string(),
    }
}

fn cache_path(dir: &Path) -> PathBuf {
    dir.join(CACHE_FILE)
}

/// The copy holds the address it was read from and the list as it came.
fn save_copy(dir: &Path, url: &str, text: &str) {
    let body = serde_json::json!({ "url": url, "list": text }).to_string();
    let path = cache_path(dir);
    if std::fs::read_to_string(&path).ok().as_deref() == Some(body.as_str()) {
        return;
    }
    let _ = std::fs::create_dir_all(dir);
    let tmp = dir.join(format!("{CACHE_FILE}.tmp-{}", std::process::id()));
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, &path);
    }
}

fn read_copy(dir: &Path, url: &str) -> Option<Vec<ListedAgent>> {
    let v: Value = serde_json::from_str(&std::fs::read_to_string(cache_path(dir)).ok()?).ok()?;
    if v["url"].as_str() != Some(url) {
        return None;
    }
    parse(v["list"].as_str()?).ok()
}

/// One GET of `url`, with no cookies and nothing about the person but the app's version.
fn curl(url: &str) -> Option<String> {
    let out = crate::agents::command("curl")
        .args([
            "--silent",
            "--fail",
            "--location",
            "--proto",
            "=https,http",
            "--max-time",
            TIMEOUT_SECS,
        ])
        .args([
            "--user-agent",
            &format!("Allomos/{}", env!("CARGO_PKG_VERSION")),
        ])
        .arg("--")
        .arg(url)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The Monitor ready tasks dialog's list: read now from the web (when it opens, and at Refresh).
#[tauri::command]
pub async fn agent_models() -> ModelsRead {
    let url = url();
    let dir = allomos_store::settings_dir();
    tauri::async_runtime::spawn_blocking(move || read_with(&url, dir.as_deref(), curl))
        .await
        .unwrap_or(ModelsRead {
            agents: None,
            from: "none",
            failed: Some("fetch"),
            url: DEFAULT_URL.to_string(),
        })
}

/// Settings' **Model list address**: the address in use and Allomos's own.
#[derive(Serialize)]
pub struct ModelsUrl {
    url: String,
    default: &'static str,
}

#[tauri::command]
pub fn models_url() -> ModelsUrl {
    ModelsUrl {
        url: url(),
        default: DEFAULT_URL,
    }
}

/// Set the **Model list address**; none, empty or Allomos's own address goes back to the default.
#[tauri::command]
pub fn set_models_url(url: Option<String>) -> Result<(), Problem> {
    let url = url
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty() && u != DEFAULT_URL);
    if let Some(u) = url.as_deref().filter(|u| !plain_url(u)) {
        return Err(Problem::BadUsage {
            detail: format!("{u}: not a web address (it starts with https://)"),
        });
    }
    allomos_store::save_models_url(url.as_deref()).map_err(|e| Problem::WriteFailed {
        path: "settings.json".into(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = r#"{"version":2,"updated":"2026-11-25","agents":[
        {"id":"claude_cli","name":"Claude Code","models":[{"value":"opus","label":"Opus（最新）"},{"value":"bad value"}],"efforts":["low","high"],"new":1},
        {"id":"codex_cli","name":"Codex","models":[{"value":"gpt-6.2-sol"},{"value":"gpt-6-luna","efforts":[]}],"efforts":["x;y","xhigh"]},
        {"name":"no id"}]}"#;

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("allomos-models-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn the_list_is_read_and_odd_values_are_left_out() {
        let agents = parse(LIST).unwrap();
        assert_eq!(agents.len(), 2);
        assert_eq!(
            agents[0].models,
            vec![Model {
                value: "opus".into(),
                label: "Opus（最新）".into(),
                efforts: vec!["low".into(), "high".into()]
            }]
        );
        assert_eq!(agents[0].efforts, ["low", "high"]);
        // A model without a label is shown by its value.
        assert_eq!(agents[1].models[0].label, "gpt-6.2-sol");
        assert_eq!(agents[1].efforts, ["xhigh"]);
        // Format 2: a model without its own efforts uses its agent's; an empty list is its own.
        assert_eq!(agents[1].models[0].efforts, ["xhigh"]);
        assert_eq!(agents[1].models[1].efforts, Vec::<String>::new());
        assert_eq!(parse("not json"), Err("format"));
        assert_eq!(parse(r#"{"version":3,"agents":[]}"#), Err("format"));
        assert_eq!(parse(r#"{"version":1}"#), Err("format"));
    }

    #[test]
    fn format_1_is_still_read_and_gives_every_model_its_agents_efforts() {
        let agents = parse(
            r#"{"version":1,"updated":"2026-11-18","agents":[
                {"id":"antigravity_cli","name":"Antigravity","models":[{"value":"gemini-4-hydrogen","label":"Gemini 4 Hydrogen"}],"efforts":["low","high"]}]}"#,
        )
        .unwrap();
        assert_eq!(agents[0].models[0].efforts, ["low", "high"]);
    }

    #[test]
    fn a_model_keeps_only_the_efforts_that_are_one_plain_word() {
        let agents = parse(
            r#"{"version":2,"agents":[{"id":"a_cli","name":"A","models":[{"value":"m","efforts":["low","x y","low"]}],"efforts":["high"]}]}"#,
        )
        .unwrap();
        assert_eq!(agents[0].models[0].efforts, ["low"]);
        assert_eq!(agents[0].efforts, ["high"]);
    }

    #[test]
    fn a_list_read_is_kept_and_used_when_the_web_cannot_be_read() {
        let d = dir("cache");
        // Nothing read and no copy: no list, and why.
        let none = read_with(DEFAULT_URL, Some(&d), |_| None);
        assert_eq!(
            (none.from, none.failed, none.agents),
            ("none", Some("fetch"), None)
        );
        // Read from the web: the copy is kept.
        let asked = std::cell::RefCell::new(String::new());
        let online = read_with(DEFAULT_URL, Some(&d), |u| {
            *asked.borrow_mut() = u.to_string();
            Some(LIST.to_string())
        });
        assert_eq!(asked.borrow().as_str(), DEFAULT_URL);
        assert_eq!((online.from, online.failed), ("online", None));
        // Offline, or a list of an unknown format: the copy.
        let cached = read_with(DEFAULT_URL, Some(&d), |_| None);
        assert_eq!((cached.from, cached.failed), ("cache", Some("fetch")));
        assert_eq!(cached.agents, online.agents);
        let newer = read_with(DEFAULT_URL, Some(&d), |_| Some(r#"{"version":9}"#.into()));
        assert_eq!((newer.from, newer.failed), ("cache", Some("format")));
        // Refresh reads again: new content replaces the options and the copy.
        let changed = LIST.replace("Opus（最新）", "Opus 5.5");
        let refreshed = read_with(DEFAULT_URL, Some(&d), |_| Some(changed.clone()));
        assert_eq!(refreshed.agents.unwrap()[0].models[0].label, "Opus 5.5");
        let cached = read_with(DEFAULT_URL, Some(&d), |_| None);
        assert_eq!(cached.agents.unwrap()[0].models[0].label, "Opus 5.5");
        // Another address does not use that address's copy.
        let other = read_with("https://example.org/m.json", Some(&d), |_| None);
        assert_eq!((other.from, other.agents), ("none", None));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn only_web_addresses_are_taken() {
        assert!(plain_url(DEFAULT_URL));
        assert!(plain_url("http://localhost:8080/models.json"));
        assert!(!plain_url("www.allomos.com/agents/models.json"));
        assert!(!plain_url("https://"));
        assert!(!plain_url("https:///x"));
        assert!(!plain_url("file:///etc/passwd"));
        assert!(!plain_url("https://a b"));
        assert!(!plain_url("-K https://x"));
    }
}
