//! Problem codes: everything FlipHopper's Rust side can report to a person.
//!
//! Rust never sends sentences to the interface. It sends a code plus its details
//! (`params`), and the interface words the code in the chosen language (R94). Each code
//! also has one English wording here, for logs and for the `fh` command.
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
    NotJson { detail } => "not_json", "This file could not be read as JSON for FlipHopper: {detail}.";
    FieldMissing => "field_missing", "This required field is missing.";
    FieldUnknown => "field_unknown", "FlipHopper does not know this field. Check its spelling, or remove it.";
    InvalidChoice { value, allowed } => "invalid_choice", "{value} is not a valid value here. Use one of: {allowed}.";
    InvalidId { id } => "invalid_id", "\"{id}\" is not a valid ID. Use 1–64 letters, digits, '-' or '_'.";
    WrongFormat { expected, found } => "wrong_format", "Expected \"{expected}\" but found \"{found}\". This version of FlipHopper reads only \"{expected}\".";
    ProjectNameMissing => "project_name_missing", "The project needs a name.";
    LadderEmpty => "ladder_empty", "The label ladder needs at least one step.";
    AreaNameMissing => "area_name_missing", "The area needs a name.";
    TaskTitleMissing => "task_title_missing", "The task needs a title.";
    DuplicateAreaId { id } => "duplicate_area_id", "Another area already uses the ID \"{id}\".";
    DuplicateTaskId { id } => "duplicate_task_id", "Another task already uses the ID \"{id}\".";
    UnknownArea { area } => "unknown_area", "The area \"{area}\" does not exist. Add areas/{area}.json or use an existing area ID.";
    LabelNotOnLadder { label, ladder } => "label_not_on_ladder", "\"{label}\" is not a step of this project's ladder ({ladder}).";
    WaitingOnMissing => "waiting_on_missing", "A waiting task must say what it is waiting on.";
    /// The file exists but could not be read. `detail` is the system's own text.
    ReadFailed { detail } => "read_failed", "Could not read this file: {detail}.";
    TaskFileMissing => "task_file_missing", "This task folder has no task.json.";
    /// Creating a seabag was refused because the folder already has files in it.
    FolderNotEmpty { path } => "folder_not_empty", "{path} is not empty, so no seabag was created there.";
    CreateFailed { path, detail } => "create_failed", "Could not create the seabag in {path}: {detail}.";
    SeabagNotFound { path } => "seabag_not_found", "No seabag was found at or above {path}. Run `fh init` first, or pass --seabag <path>.";
    SeabagExists { path } => "seabag_exists", "A seabag already exists at {path}.";
    /// The seabag's project file could not be read. `detail` says why.
    SeabagUnreadable { path, detail } => "seabag_unreadable", "The seabag at {path} could not be read: {detail}";
    /// Another FlipHopper writer held the seabag's lock for too long.
    SeabagBusy { path } => "seabag_busy", "Another FlipHopper writer is changing the seabag at {path}. Try again in a moment.";
    WriteFailed { path, detail } => "write_failed", "Could not write {path}: {detail}.";
    TaskNotFound { id } => "task_not_found", "There is no task \"{id}\".";
    HolderMissing => "holder_missing", "Say who you are with --as <name> or the FLIPHOPPER_HOLDER environment variable.";
    TaskHeld { id, holder } => "task_held", "Task {id} is held by {holder}.";
    TaskClosed { id, state } => "task_closed", "Task {id} is {state}, so it cannot be changed this way.";
    /// A warning: the task was finished below the top step of the ladder.
    LabelBelowTop { label, top } => "label_below_top", "Finished with the label \"{label}\", below the top step \"{top}\".";
    /// A warning: the task was taken over from another holder.
    TookOver { id, previous } => "took_over", "Took task {id} over from {previous}.";
    NotASeabag { path } => "not_a_seabag", "{path} is not a seabag folder.";
    /// The file manager could not be started. `detail` is the system's own text.
    OpenFailed { path, detail } => "open_failed", "Could not open {path} in the file manager: {detail}.";
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
    /// `FLIPHOPPER_WRITE_CODES=1 cargo test -p fliphopper-core`.
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
        if std::env::var_os("FLIPHOPPER_WRITE_CODES").is_some() {
            std::fs::write(path, &want).unwrap();
        }
        let have = std::fs::read_to_string(path).unwrap_or_default();
        assert!(
            have == want,
            "apps/web/src/i18n/codes.json is out of date. Run \
             `FLIPHOPPER_WRITE_CODES=1 cargo test -p fliphopper-core`, then add words for any new \
             code to every language file."
        );
    }
}
