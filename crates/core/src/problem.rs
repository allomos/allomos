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
    /// The taskbag was written by a newer Allomos (Task-A01-25). The person chooses what happens.
    TaskbagNewer { found, supported } => "taskbag_newer", "This taskbag is written in the format \"{found}\", newer than this Allomos understands (\"{supported}\"). Stop and let the person choose: update Allomos now, or let you go on working with the taskbag's files directly, from your own understanding of the newer format.";
    /// The taskbag was written by an older Allomos (any format below this one's). It is not read
    /// or changed: an AI agent moves its tasks into a new taskbag, like another task system (owner,
    /// 2026-10-15; Task-A01-45). The app renames the old folder aside and asks the agent;
    /// `alm move-in` prints the same request.
    TaskbagToMoveIn { found, current } => "taskbag_to_move_in", "This taskbag was written by an older Allomos (format \"{found}\"); this Allomos writes \"{current}\" and does not read or change older taskbags, so nothing was read or changed. Its tasks are moved into a new taskbag by an AI agent, like another task system: the person opens the project in the Allomos app and chooses Move the tasks in…, or you run `alm move-in` and follow the request it prints. Stop and tell the person.";
    /// **Move the tasks in…** (or `alm move-in`) on a taskbag that is not from an older Allomos and
    /// has no older taskbag waiting beside it (Task-A01-45).
    NothingToMoveIn { path } => "nothing_to_move_in", "There is nothing to move in: the taskbag at {path} is not from an older Allomos, and no older taskbag is waiting beside it to be moved in.";
    /// A warning: Allomos's section in AGENTS.md / CLAUDE.md is out of date (Task-A01-25).
    InstructionsOutdated { files, current } => "instructions_outdated", "Allomos's section in {files} is out of date (this Allomos writes version {current}). Update it now: replace that section with the output of `alm brief`, keep the rest of the file, and commit only those files.";
    /// A handoff gate is not a percentage (Task-A03-06).
    InvalidPercent { value } => "invalid_percent", "\"{value}\" is not a percentage. Use a whole number from 0 to 100.";
    /// The handoff template may not be empty (Task-A03-06).
    TemplateEmpty => "template_empty", "The handoff template cannot be empty.";
    /// `alm handoff --remove` named no handoff file (Task-A03-13).
    HandoffNotFound { file } => "handoff_not_found", "There is no handoff \"{file}\". `alm handoffs` lists them.";
    ProjectNameMissing => "project_name_missing", "The project needs a name.";
    LadderEmpty => "ladder_empty", "The label ladder needs at least one step.";
    /// `alm rename label` with an empty new name (Task-A01-32).
    LabelNameMissing => "label_name_missing", "The label needs a name.";
    /// `alm rename label` to a name the ladder already has (Task-A01-32).
    LabelExists { label } => "label_exists", "\"{label}\" is already a step of this project's ladder.";
    /// `task_language` is neither `ui` nor a language code (Task-A01-21).
    InvalidLanguage { value } => "invalid_language", "\"{value}\" is not a language. Use a language code such as en, zh-CN or ja, or ui to follow the Allomos app's language.";
    AreaNameMissing => "area_name_missing", "The area needs a name.";
    TaskTitleMissing => "task_title_missing", "The task needs a title.";
    DuplicateAreaId { id } => "duplicate_area_id", "Another area already uses the ID \"{id}\".";
    DuplicateTaskId { id } => "duplicate_task_id", "Another task already uses the ID \"{id}\".";
    UnknownArea { area } => "unknown_area", "The area \"{area}\" does not exist. Add areas/{area}.json or use an existing area ID.";
    LabelNotOnLadder { label, ladder } => "label_not_on_ladder", "\"{label}\" is not a step of this project's ladder ({ladder}).";
    WaitingOnMissing => "waiting_on_missing", "A waiting task must say what it is waiting on.";
    /// A `blocked` task names no blocking task (decision 0016, Task-A01-31).
    BlockedByMissing => "blocked_by_missing", "A blocked task must name the task(s) that must finish first: `alm block <id> --by <task-id>`.";
    /// `blocked_by` names a task that is not in this taskbag.
    BlockerUnknown { id } => "blocker_unknown", "There is no task \"{id}\" in this taskbag, so it cannot block another task.";
    /// `blocked_by` names the task itself.
    BlockedBySelf { id } => "blocked_by_self", "Task {id} cannot be blocked by itself.";
    /// `alm unblock` on a task that is not blocked.
    NotBlocked { id, state } => "not_blocked", "Task {id} is {state}, not blocked, so there is nothing to unblock.";
    /// A warning: a blocking task is dropped, so it will never be done (owner, 2026-10-14).
    BlockerDropped { id, blocker } => "blocker_dropped", "{blocker}, which blocks {id}, is dropped: it will never be done, and it does not count as done. Is {id} still blocked?";
    /// `alm done --without-human-check` with no text, or the field empty in `task.json`
    /// (Task-A03-15; renamed from `without_owner_missing` in Task-A03-18).
    WithoutHumanCheckMissing => "without_human_check_missing", "Say why the task is finished without the human check, and in which run.";
    /// A warning: `alm done --without-human-check` without the steps for the person who checks it
    /// (Task-A03-18). `alm` asks for them and shows how to write them; it does not refuse.
    HumanCheckStepsMissing { id } => "human_check_steps_missing", "No human check steps for {id}: a person must be able to pick it up and test it. Write them in Markdown (the test folder, what to open, what to click, what they should see) to a file, then run `alm done {id} --without-human-check \"<why, which run>\" --steps <file.md>`.";
    /// `--steps` names an empty file (Task-A03-18).
    HumanCheckStepsEmpty => "human_check_steps_empty", "The human check steps are empty. Write what a person opens, clicks and should see, or leave out --steps.";
    /// `alm checked` on a task that is not done (Task-A03-18).
    NotDone { id, state } => "not_done", "Task {id} is {state}, not done: only a finished task is human-checked.";
    /// `alm reopen` (or the app's Reopen…) on a task that is not done or dropped (Task-A01-50).
    CannotReopen { id, state } => "cannot_reopen", "Task {id} is {state}: only a done or dropped task can be reopened. It is still open, so carry on with it as it is.";
    /// `alm reopen` (or the app's Reopen…) without saying why (Task-A01-50).
    ReopenNoteMissing => "reopen_note_missing", "Say why the task is reopened: `alm reopen <id> --note \"<why>\"`.";
    /// `alm ai-decided` with no text (Task-A03-19).
    AiDecisionMissing => "ai_decision_missing", "Say what you decided and why, in one line: `alm ai-decided <id> \"<what and why>\"`.";
    /// `alm reviewed` (or the app's Reviewed button) on a task with no AI decisions to review (Task-A03-19).
    NothingToReview { id } => "nothing_to_review", "Task {id} has no AI decisions waiting for review.";
    /// A branch name that is not one: empty, with a space, or starting with `-` (Task-A03-22).
    BranchInvalid { name } => "branch_invalid", "\"{name}\" is not a branch name. Give the branch's real name, as Git shows it (for example main).";
    /// A warning: `alm add task` or `alm claim` was not told which branch the agent is on, so the
    /// task's branches were not recorded (Task-A03-22). alm reads no Git.
    BranchMissing { id } => "branch_missing", "alm was not told which branch you are on, so task {id}'s branches were not recorded. Pass --branch <name>: read it with `git branch --show-current` (alm reads no Git).";
    /// A warning: the session adding a task holds another task on a different base (Task-A03-22).
    BaseDiffers { held, held_base, base } => "base_differs", "You hold {held}, which starts from and merges into {held_base}, but the new task got the base {base}. Decide which is right: keep it, or change the new task's base with `alm base <its id> {held_base}`.";
    IterationNameMissing => "iteration_name_missing", "The iteration needs a name.";
    DuplicateIterationId { id } => "duplicate_iteration_id", "Another iteration already uses the ID \"{id}\".";
    UnknownIteration { iteration } => "unknown_iteration", "The iteration \"{iteration}\" does not exist. Add it with `alm add iteration`, or use an existing iteration ID.";
    /// `alm delete area` while a task, even a done or dropped one, is in the area (owner, 2026-10-14; Task-A01-29).
    AreaNotEmpty { id, count } => "area_not_empty", "The area \"{id}\" cannot be deleted: {count} task(s) are in it, counting done and dropped ones. An area can be deleted only when no task is in it.";
    /// `alm delete iteration` while a task, even a done or dropped one, is planned in it (owner, 2026-10-14; Task-A01-29).
    IterationNotEmpty { id, count } => "iteration_not_empty", "The iteration \"{id}\" cannot be deleted: {count} task(s) are planned in it, counting done and dropped ones. Plan them in another iteration (`alm plan <task> <iteration>`) or take them out (`alm plan <task>`) first.";
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
    /// Only an unheld backlog, waiting or blocked task can be made ready.
    CannotMakeReady { id, state } => "cannot_make_ready", "Task {id} is {state}; only a backlog, waiting or blocked task that no one holds can be made ready.";
    /// The app was asked to set a state that only another step sets (Task-A01-18).
    CannotSetState { id, state } => "cannot_set_state", "Task {id} cannot be set to {state} here: a task becomes in progress only when an agent claims it (`alm claim`), and nothing moves a task back to backlog.";
    /// `alm cards found`: a card file's place must be a relative path inside the project folder
    /// (Task-A00-30).
    CardPathInvalid { path } => "card_path_invalid", "\"{path}\" is not a card file's place: give its path inside the project folder, relative to that folder, such as docs/tasks/A01-01.md.";
    /// `alm cards found` / `alm card`: the card file named is not there.
    CardNotFound { path } => "card_not_found", "There is no file {path}.";
    /// `alm card`: the task's folder already holds another card file, which is never replaced.
    CardExists { id } => "card_exists", "Task {id} already has a different card file (tasks/{id}/card.md), so it was not replaced. Compare the two; to replace it on purpose, add --replace.";
    /// `alm card`: the card file given is empty.
    CardEmpty { path } => "card_empty", "The card file {path} is empty, so nothing was moved in.";
    /// `alm card` without `--from`, for a task no agent reported a card file for.
    CardNotReported { id } => "card_not_reported", "No card file was reported for task {id}, and its description links to none outside its folder. Name it: `alm card {id} --from <file>`.";
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
        // Git for Windows may check the file out with \r\n line endings.
        let have = std::fs::read_to_string(path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            have == want,
            "apps/web/src/i18n/codes.json is out of date. Run \
             `ALLOMOS_WRITE_CODES=1 cargo test -p allomos-core`, then add words for any new \
             code to every language file."
        );
    }
}
