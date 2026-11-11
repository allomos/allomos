//! **Hand over to another agent…** (Task-A01-minnow200): the app's two changes to a task's
//! `handover_request`, made as the person's saved name (or as the app, when none is saved yet).

use std::path::PathBuf;

use allomos_core::Problem;

use crate::{LOCK_WAIT, app_actor};

/// Ask the session holding task `id` to hand it over to `to` (the agent picker's choice).
#[tauri::command]
pub(crate) fn request_handover(
    root: PathBuf,
    id: String,
    to: String,
) -> Result<allomos_core::Task, Problem> {
    let name = allomos_store::saved_owner_name();
    let actor = app_actor(name.as_deref()).unwrap_or(allomos_store::Actor::APP);
    allomos_store::request_handover(&root, actor, LOCK_WAIT, &id, &to).map(|c| c.task)
}

/// Take the request back: the dialog's Cancel the request.
#[tauri::command]
pub(crate) fn cancel_handover(root: PathBuf, id: String) -> Result<allomos_core::Task, Problem> {
    let name = allomos_store::saved_owner_name();
    let actor = app_actor(name.as_deref()).unwrap_or(allomos_store::Actor::APP);
    allomos_store::cancel_handover(&root, actor, LOCK_WAIT, &id).map(|c| c.task)
}
