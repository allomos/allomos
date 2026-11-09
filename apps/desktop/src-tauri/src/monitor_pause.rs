//! The reminder before a restart or an update (Task-A03-bowl641): while **Monitor ready tasks**
//! runs on this computer, Restart to Update and quitting the app first say that the automatic
//! monitoring of ready tasks keeps running in the background, and that the sessions already open finish their tasks as
//! usual. The person can cancel. With no monitoring running, nothing is asked.
//!
//! The reminder only asks; it stops and saves nothing. The words come in the interface language
//! the app last saved (English, 中文 or 日本語; English otherwise), as the run-mode text does.

use allomos_monitor::registry::{self, Entry};
use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

/// What the person is about to do when the reminder comes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// **Restart to Update**: the installer runs and the app starts again.
    Restart,
    /// Quitting the app.
    Quit,
}

/// How many sessions are at work across the monitoring runs in `entries`, or `None` when no run
/// is going on (then no reminder). A run whose record is missing counts with no session it can name.
pub fn sessions_at_work(entries: &[Entry]) -> Option<usize> {
    if entries.is_empty() {
        return None;
    }
    Some(
        entries
            .iter()
            .map(|e| match e {
                Entry::Running(r) => r.run.working.len(),
                Entry::Missing { .. } => 0,
            })
            .sum(),
    )
}

/// The monitoring runs this computer has now, as [`sessions_at_work`] reads them: `None` when none.
pub fn running_sessions() -> Option<usize> {
    let dir = registry::dir()?;
    let entries = registry::list(&dir, Some(&registry::monitor_processes()));
    sessions_at_work(&entries)
}

/// The words of the reminder: a title, a body that names the sessions, and the two buttons.
#[derive(Debug, PartialEq, Eq)]
pub struct Words {
    pub title: String,
    pub body: String,
    pub confirm: &'static str,
    pub cancel: &'static str,
}

/// The reminder's words for `action` with `sessions` open, in `language` (a saved interface
/// language tag: `zh-CN`, `ja`, `en`…; English for any other).
pub fn words(language: &str, action: Action, sessions: usize) -> Words {
    let lang = language.to_lowercase().replace('_', "-");
    if lang == "zh" || lang.starts_with("zh-") {
        let (verb, confirm) = match action {
            Action::Restart => ("重新启动", "重新启动"),
            Action::Quit => ("退出", "退出"),
        };
        Words {
            title: "自动监测就绪任务会继续运行".into(),
            body: format!(
                "应用将{verb}。「自动监测就绪任务」会继续在后台运行；已开的 {sessions} 个会话照常做完各自的任务。要先停掉它，可以在状态栏里操作。"
            ),
            confirm,
            cancel: "取消",
        }
    } else if lang == "ja" || lang.starts_with("ja-") {
        let (verb, confirm) = match action {
            Action::Restart => ("再起動します", "再起動"),
            Action::Quit => ("終了します", "終了"),
        };
        Words {
            title: "準備完了のタスクの自動監視は続きます".into(),
            body: format!(
                "「肩代」を{verb}。「準備完了のタスクを自動で監視」はバックグラウンドで動き続け、開いている {sessions} 件のセッションは、それぞれのタスクをいつも通り最後まで進めます。止めてから行う場合は、ステータスバーから停止できます。"
            ),
            confirm,
            cancel: "キャンセル",
        }
    } else {
        let (verb, confirm) = match action {
            Action::Restart => ("restart", "Restart"),
            Action::Quit => ("quit", "Quit"),
        };
        let open = if sessions == 1 {
            "1 open session".to_string()
        } else {
            format!("{sessions} open sessions")
        };
        Words {
            title: "Automatic monitoring of ready tasks keeps running".into(),
            body: format!(
                "Allomos will {verb}. Monitor ready tasks keeps running in the background; the {open} will finish their tasks as usual. To stop it first, use the status bar."
            ),
            confirm,
            cancel: "Cancel",
        }
    }
}

/// The interface language the app last saved, for the reminder's words.
fn language() -> String {
    allomos_store::saved_ui_language().unwrap_or_else(|| "en".to_string())
}

/// Shows the reminder and waits for the answer: `true` when the person confirms. Call it off the
/// main thread (a blocking dialog waits for the window).
pub fn confirm_blocking(app: &AppHandle, action: Action, sessions: usize) -> bool {
    let w = words(&language(), action, sessions);
    app.dialog()
        .message(w.body)
        .title(w.title)
        .buttons(MessageDialogButtons::OkCancelCustom(
            w.confirm.to_string(),
            w.cancel.to_string(),
        ))
        .blocking_show()
}

/// Shows the reminder without waiting; when the person confirms, `on_confirm` runs (on the
/// dialog's thread). Used when quitting, where the app must not block the event loop.
pub fn confirm_then(
    app: &AppHandle,
    action: Action,
    sessions: usize,
    on_confirm: impl FnOnce() + Send + 'static,
) {
    let w = words(&language(), action, sessions);
    app.dialog()
        .message(w.body)
        .title(w.title)
        .buttons(MessageDialogButtons::OkCancelCustom(
            w.confirm.to_string(),
            w.cancel.to_string(),
        ))
        .show(move |confirmed| {
            if confirmed {
                on_confirm();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_run_means_no_reminder() {
        assert_eq!(sessions_at_work(&[]), None);
    }

    #[test]
    fn a_run_with_no_record_counts_as_running_with_no_named_session() {
        let missing = Entry::Missing {
            pid: 42,
            id: None,
            taskbag: None,
        };
        assert_eq!(sessions_at_work(&[missing]), Some(0));
    }

    #[test]
    fn the_reminder_names_the_running_monitor_and_the_sessions_in_english() {
        let w = words("en", Action::Restart, 2);
        assert_eq!(w.title, "Automatic monitoring of ready tasks keeps running");
        assert!(w.body.contains("Monitor ready tasks keeps running"));
        assert!(!w.body.contains("will pause"));
        assert!(w.body.contains("2 open sessions"));
        assert_eq!((w.confirm, w.cancel), ("Restart", "Cancel"));
    }

    #[test]
    fn one_open_session_is_singular_in_english() {
        assert!(
            words("en-GB", Action::Quit, 1)
                .body
                .contains("1 open session ")
        );
        assert_eq!(words("en", Action::Quit, 1).confirm, "Quit");
    }

    #[test]
    fn the_reminder_is_in_chinese_and_japanese() {
        let zh = words("zh-CN", Action::Restart, 3);
        assert!(zh.body.contains("「自动监测就绪任务」会继续在后台运行"));
        assert!(zh.body.contains("已开的 3 个会话"));
        assert_eq!((zh.confirm, zh.cancel), ("重新启动", "取消"));
        let ja = words("ja", Action::Quit, 3);
        assert!(
            ja.body
                .contains("「準備完了のタスクを自動で監視」はバックグラウンドで動き続け")
        );
        assert!(ja.body.contains("3 件のセッション"));
        assert_eq!((ja.confirm, ja.cancel), ("終了", "キャンセル"));
    }

    #[test]
    fn an_unknown_language_gets_english() {
        assert_eq!(
            words("fr", Action::Restart, 0),
            words("en", Action::Restart, 0)
        );
    }
}
