//! Allomos's per-user settings on this computer, outside any taskbag (Task-A01-21). For now it
//! holds one thing: the interface language the app last showed, so `alm language` can tell an
//! agent which language to write tasks in when the taskbag says "follow the app" (`ui`).

use std::fs;
use std::io;
use std::path::PathBuf;

use allomos_core::{Project, TASK_LANGUAGE_UI};
use serde::{Deserialize, Serialize};

/// The settings file, in the folder [`settings_dir`] names.
const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct Settings {
    /// The language the app's interface shows (`en`, `zh-CN`, `ja`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ui_language: Option<String>,
    /// The person chose "Don't ask again" for **Install the alm command** (Task-A07-12).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    alm_offer_dismissed: bool,
}

/// Allomos's settings folder for this user: `ALLOMOS_SETTINGS_DIR` when set (tests), else
/// the system's place for app settings (`~/Library/Application Support/io.allomos` on macOS,
/// `%APPDATA%\io.allomos` on Windows, `$XDG_CONFIG_HOME/allomos` or
/// `~/.config/allomos` elsewhere).
pub fn settings_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("ALLOMOS_SETTINGS_DIR") {
        return Some(PathBuf::from(dir));
    }
    let env = |name: &str| std::env::var_os(name).map(PathBuf::from);
    if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support/io.allomos"))
    } else if cfg!(windows) {
        env("APPDATA").map(|a| a.join("io.allomos"))
    } else {
        env("XDG_CONFIG_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".config")))
            .map(|c| c.join("allomos"))
    }
}

fn read_settings() -> Settings {
    settings_dir()
        .and_then(|d| {
            fs::read_to_string(d.join(SETTINGS_FILE)).ok().or_else(|| {
                // Before the rename the app was FlipHopper (`io.fliphopper`); read its settings
                // until Allomos saves its own. Not when a test chose the folder.
                if std::env::var_os("ALLOMOS_SETTINGS_DIR").is_some() {
                    return None;
                }
                let old = d
                    .parent()?
                    .join(if cfg!(any(target_os = "macos", windows)) {
                        "io.fliphopper"
                    } else {
                        "fliphopper"
                    });
                fs::read_to_string(old.join(SETTINGS_FILE)).ok()
            })
        })
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Save the app's interface language. Called by the app whenever its language is chosen.
pub fn save_ui_language(language: &str) -> io::Result<()> {
    let mut settings = read_settings();
    if settings.ui_language.as_deref() == Some(language) {
        return Ok(());
    }
    settings.ui_language = Some(language.to_string());
    write_settings(&settings)
}

/// Whether the person chose "Don't ask again" for **Install the alm command** (Task-A07-12).
pub fn alm_offer_dismissed() -> bool {
    read_settings().alm_offer_dismissed
}

/// Remember "Don't ask again" for **Install the alm command**.
pub fn dismiss_alm_offer() -> io::Result<()> {
    let mut settings = read_settings();
    settings.alm_offer_dismissed = true;
    write_settings(&settings)
}

fn write_settings(settings: &Settings) -> io::Result<()> {
    let dir = settings_dir().ok_or_else(|| io::Error::other("no settings folder"))?;
    fs::create_dir_all(&dir)?;
    let text = serde_json::to_string_pretty(settings).map_err(io::Error::other)?;
    let tmp = dir.join(format!("{SETTINGS_FILE}.tmp-{}", std::process::id()));
    fs::write(&tmp, text + "\n")?;
    fs::rename(tmp, dir.join(SETTINGS_FILE))
}

/// The interface language the app last saved, if it has run on this computer.
pub fn saved_ui_language() -> Option<String> {
    read_settings().ui_language
}

/// The interface language the app picks from the computer's preferred languages: the first it
/// speaks (English, Japanese, Chinese as Simplified Chinese), English otherwise. The same rule
/// as `pickLanguage` in `apps/web/src/i18n`.
pub fn pick_ui_language<S: AsRef<str>>(preferred: &[S]) -> &'static str {
    for tag in preferred {
        let t = tag.as_ref().to_lowercase().replace('_', "-");
        if t == "en" || t.starts_with("en-") {
            return "en";
        }
        if t == "ja" || t.starts_with("ja-") {
            return "ja";
        }
        if t == "zh" || t.starts_with("zh-") {
            return "zh-CN";
        }
    }
    "en"
}

/// The language tasks are written in, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TaskLanguage {
    /// The language code to write in.
    pub language: String,
    /// What the taskbag says: `ui` or a language code.
    pub setting: String,
    /// Where `language` came from: `taskbag` (a code set there), `app` (the app's saved interface
    /// language) or `system` (the computer's language, before the app has run).
    pub source: &'static str,
}

/// The task language for `project`: its own code, or with `ui` (or nothing set) the app's saved
/// interface language, else the one the app would pick from `system` (the computer's preferred
/// languages).
pub fn task_language<S: AsRef<str>>(project: &Project, system: &[S]) -> TaskLanguage {
    let setting = project
        .task_language
        .clone()
        .unwrap_or_else(|| TASK_LANGUAGE_UI.to_string());
    if setting != TASK_LANGUAGE_UI {
        return TaskLanguage {
            language: setting.clone(),
            setting,
            source: "taskbag",
        };
    }
    match saved_ui_language() {
        Some(language) => TaskLanguage {
            language,
            setting,
            source: "app",
        },
        None => TaskLanguage {
            language: pick_ui_language(system).to_string(),
            setting,
            source: "system",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use allomos_core::{FORMAT, default_labels};
    use std::sync::Mutex;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Tests that set ALLOMOS_SETTINGS_DIR run one at a time.
    static ENV: Mutex<()> = Mutex::new(());

    fn project(language: Option<&str>) -> Project {
        Project {
            format: FORMAT.into(),
            id: "p".into(),
            name: "P".into(),
            labels: default_labels(),
            project_folder: None,
            task_language: language.map(String::from),
            handoff: None,
        }
    }

    #[test]
    fn the_app_language_wins_over_the_system_and_a_code_wins_over_both() {
        let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("allomos-settings-{n}"));
        // SAFETY: the variable is set and read only by these tests, one at a time (the lock).
        unsafe { std::env::set_var("ALLOMOS_SETTINGS_DIR", &dir) };

        // Before the app has run: the computer's language, picked as the app picks.
        let l = task_language(&project(None), &["ja-JP", "en-US"]);
        assert_eq!(
            (l.language.as_str(), l.setting.as_str(), l.source),
            ("ja", "ui", "system")
        );
        assert_eq!(
            task_language(&project(Some("ui")), &["fr-FR"]).language,
            "en"
        );

        // After the app saved Chinese.
        save_ui_language("zh-CN").unwrap();
        let l = task_language(&project(Some("ui")), &["ja-JP"]);
        assert_eq!((l.language.as_str(), l.source), ("zh-CN", "app"));

        // "Don't ask again" for the alm command is kept beside the language (Task-A07-12).
        assert!(!alm_offer_dismissed());
        dismiss_alm_offer().unwrap();
        assert!(alm_offer_dismissed());
        assert_eq!(saved_ui_language().as_deref(), Some("zh-CN"));

        // A code in the taskbag wins.
        let l = task_language(&project(Some("fr")), &["ja-JP"]);
        assert_eq!(
            (l.language.as_str(), l.setting.as_str(), l.source),
            ("fr", "fr", "taskbag")
        );

        // SAFETY: as above.
        unsafe { std::env::remove_var("ALLOMOS_SETTINGS_DIR") };
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn picking_matches_the_app() {
        assert_eq!(pick_ui_language(&["zh_TW"]), "zh-CN");
        assert_eq!(pick_ui_language(&["de-DE", "ja"]), "ja");
        assert_eq!(pick_ui_language::<&str>(&[]), "en");
    }
}
