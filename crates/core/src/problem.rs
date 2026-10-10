//! Problem codes: everything Allomos's Rust side can report to a person.
//!
//! Rust never sends sentences to the interface. It sends a code plus its details
//! (`params`), and the interface words the code in the chosen language (R94). Each code
//! also has one English wording here, for logs and for the `alm` command.
//!
//! The list is declared once, through `problems!`, so a new code is always part of
//! [`Problem::CODES`]; a test writes that list to `apps/web/src/i18n/codes.json`, where
//! the interface's translation check requires words for every code.

use serde::Serialize;

macro_rules! problems {
    ($(
        $(#[$doc:meta])*
        $variant:ident $({ $($param:ident),* $(,)? })? => $code:literal, $english:literal;
    )*) => {
        /// One thing that is wrong, as a code plus its details.
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        #[serde(tag = "code", content = "params")]
        pub enum Problem {
            $(
                $(#[$doc])*
                #[serde(rename = $code)]
                $variant $({ $($param: String),* })?,
            )*
        }

        impl Problem {
            /// Every code, with the names of its details, in declaration order.
            pub const CODES: &'static [(&'static str, &'static [&'static str])] =
                &[$(($code, &[$($(stringify!($param)),*)?])),*];

            /// The stable code, for example `field_missing`.
            pub fn code(&self) -> &'static str {
                match self {
                    $(Problem::$variant { .. } => $code,)*
                }
            }

            /// The English wording, with details filled in.
            pub fn english(&self) -> String {
                match self {
                    $(
                        #[allow(unused_variables)]
                        Problem::$variant $({ $($param),* })? => format!($english),
                    )*
                }
            }
        }
    };
}

problems! {
    /// The file is not valid JSON, or a value has the wrong type. `detail` is the parser's own text.
    NotJson { detail } => "not_json", "This file could not be read as JSON for Allomos: {detail}.";
    FieldMissing => "field_missing", "This required field is missing.";
    FieldUnknown => "field_unknown", "Allomos does not know this field. Check its spelling, or remove it.";
    InvalidChoice { value, allowed } => "invalid_choice", "{value} is not a valid value here. Use one of: {allowed}.";
    InvalidId { id } => "invalid_id", "\"{id}\" is not a valid ID. Use 1–64 letters, digits, '-' or '_'.";
    WrongFormat { expected, found } => "wrong_format", "Expected \"{expected}\" but found \"{found}\". This version of Allomos reads only \"{expected}\".";
    ProjectNameMissing => "project_name_missing", "The project needs a name.";
    LadderEmpty => "ladder_empty", "The label ladder needs at least one step.";
    /// `task_language` is neither `ui` nor a language code (Task-A01-21).
    InvalidLanguage { value } => "invalid_language", "\"{value}\" is not a language. Use a language code such as en, zh-CN or ja, or ui to follow the Allomos app's language.";
    AreaNameMissing => "area_name_missing", "The area needs a name.";
    TaskTitleMissing => "task_title_missing", "The task needs a title.";
    DuplicateAreaId { id } => "duplicate_area_id", "Another area already uses the ID \"{id}\".";
    DuplicateTaskId { id } => "duplicate_task_id", "Another task already uses the ID \"{id}\".";
    UnknownArea { area } => "unknown_area", "The area \"{area}\" does not exist. Add areas/{area}.json or use an existing area ID.";
    LabelNotOnLadder { label, ladder } => "label_not_on_ladder", "\"{label}\" is not a step of this project's ladder ({ladder}).";
    WaitingOnMissing => "waiting_on_missing", "A waiting task must say what it is waiting on.";
    IterationNameMissing => "iteration_name_missing", "The iteration needs a name.";
    DuplicateIterationId { id } => "duplicate_iteration_id", "Another iteration already uses the ID \"{id}\".";
    UnknownIteration { iteration } => "unknown_iteration", "The iteration \"{iteration}\" does not exist. Add it with `alm add iteration`, or use an existing iteration ID.";
    InvalidDate { value } => "invalid_date", "\"{value}\" is not a date. Write it as YYYY-MM-DD, for example 2026-10-08.";
    /// The file exists but could not be read. `detail` is the system's own text.
    ReadFailed { detail } => "read_failed", "Could not read this file: {detail}.";
    TaskFileMissing => "task_file_missing", "This task folder has no task.json.";
    /// Creating a taskbag was refused because the folder already has files in it.
    FolderNotEmpty { path } => "folder_not_empty", "{path} is not empty, so no taskbag was created there.";
    CreateFailed { path, detail } => "create_failed", "Could not create the taskbag in {path}: {detail}.";
    TaskbagNotFound { path } => "taskbag_not_found", "No taskbag was found at or above {path}. Run `alm init` first, or pass --taskbag <path>.";
    TaskbagExists { path } => "taskbag_exists", "A taskbag already exists at {path}.";
    /// The taskbag's project file could not be read. `detail` says why.
    TaskbagUnreadable { path, detail } => "taskbag_unreadable", "The taskbag at {path} could not be read: {detail}";
    /// Another Allomos writer held the taskbag's lock for too long.
    TaskbagBusy { path } => "taskbag_busy", "Another Allomos writer is changing the taskbag at {path}. Try again in a moment.";
    WriteFailed { path, detail } => "write_failed", "Could not write {path}: {detail}.";
    TaskNotFound { id } => "task_not_found", "There is no task \"{id}\".";
    HolderMissing => "holder_missing", "Say who you are with --as <name> or the ALLOMOS_HOLDER environment variable.";
    TaskHeld { id, holder } => "task_held", "Task {id} is held by {holder}.";
    TaskClosed { id, state } => "task_closed", "Task {id} is {state}, so it cannot be changed this way.";
    /// A warning: the task was finished below the top step of the ladder.
    LabelBelowTop { label, top } => "label_below_top", "Finished with the label \"{label}\", below the top step \"{top}\".";
    /// A warning: the task was taken over from another holder.
    TookOver { id, previous } => "took_over", "Took task {id} over from {previous}.";
    NotATaskbag { path } => "not_a_taskbag", "{path} is not a taskbag folder.";
    /// The file manager could not be started. `detail` is the system's own text.
    OpenFailed { path, detail } => "open_failed", "Could not open {path} in the file manager: {detail}.";
    /// The chosen AI agent is no longer on this computer (Task-A05-02).
    AgentNotFound { agent } => "agent_not_found", "The AI agent {agent} was not found on this computer. Look for agents again.";
    /// `alm confirm`: the person chose Cancel in the app (Task-A05-02).
    ConfirmDeclined => "confirm_declined", "The person chose Cancel in Allomos, so nothing should change.";
    /// `alm confirm`: no answer came in time.
    ConfirmTimeout { seconds } => "confirm_timeout", "No answer came from the Allomos app within {seconds} seconds, so nothing should change. Open the project folder in the app and ask again.";
    /// The app was asked to answer a question that is no longer waiting.
    RequestNotFound { id } => "request_not_found", "The question {id} is no longer waiting for an answer.";
    /// A link in a task's description that Allomos will not open (Task-A01-16).
    LinkRefused { link } => "link_refused", "Allomos opens only web links and text files (.md, .txt) inside this project, so it did not open \"{link}\".";
    LinkNotFound { link } => "link_not_found", "The file \"{link}\" does not exist.";
    /// Allomos was started with a folder that does not exist or is not a folder.
    StartupNotAFolder { path } => "startup_not_a_folder", "Allomos was started with {path}, which is not a folder, so nothing was opened.";
    /// Only an unheld backlog or waiting task can be made ready.
    CannotMakeReady { id, state } => "cannot_make_ready", "Task {id} is {state}; only a backlog or waiting task that no one holds can be made ready.";
    /// A warning from `alm --commit` / `--push`: the change is saved but Git refused. `detail` is Git's own text.
    CommitFailed { detail } => "commit_failed", "The change is saved, but committing it failed: {detail}";
    PushFailed { detail } => "push_failed", "The change is saved and committed, but pushing it failed: {detail}";
    /// The command was used wrongly. `detail` is the parser's own text.
    BadUsage { detail } => "bad_usage", "{detail}";
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.english())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn codes_are_unique_snake_case() {
        let mut seen = HashSet::new();
        for (code, _) in Problem::CODES {
            assert!(seen.insert(code), "{code} is declared twice");
            assert!(
                code.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{code} must be snake_case"
            );
        }
    }

    #[test]
    fn serializes_as_code_and_params() {
        let p = Problem::InvalidId { id: "a b".into() };
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["code"], "invalid_id");
        assert_eq!(v["params"]["id"], "a b");
        assert_eq!(p.code(), "invalid_id");
        assert!(p.english().contains("\"a b\" is not a valid ID"));
        let unit = serde_json::to_value(Problem::FieldMissing).unwrap();
        assert_eq!(unit, serde_json::json!({ "code": "field_missing" }));
    }

    /// The interface's translation check reads this file. Rewrite it with
    /// `ALLOMOS_WRITE_CODES=1 cargo test -p allomos-core`.
    #[test]
    fn codes_file_is_up_to_date() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/web/src/i18n/codes.json"
        );
        let mut map = serde_json::Map::new();
        for (code, params) in Problem::CODES {
            map.insert((*code).into(), serde_json::json!(params));
        }
        let mut want = serde_json::to_string_pretty(&map).unwrap();
        want.push('\n');
        if std::env::var_os("ALLOMOS_WRITE_CODES").is_some() {
            std::fs::write(path, &want).unwrap();
        }
        let have = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            have == want,
            "apps/web/src/i18n/codes.json is out of date. Run \
             `ALLOMOS_WRITE_CODES=1 cargo test -p allomos-core`, then add words for any new \
             code to every language file."
        );
    }
}
