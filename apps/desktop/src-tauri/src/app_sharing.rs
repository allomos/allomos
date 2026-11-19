//! Open the existing agent launcher with the current unshared app batch.
use crate::agents;
use allomos_core::Problem;
use std::path::PathBuf;

#[tauri::command(async)]
pub(crate) fn share_app_changes_request(
    taskbag: PathBuf,
    language: String,
) -> Result<String, Problem> {
    let request = allomos_store::app_share_request::request(&taskbag, &language)?;
    Ok(agents::link_request(
        &request,
        crate::alm_command::own().as_deref(),
    ))
}

#[tauri::command(async)]
pub(crate) fn share_app_changes(
    agent: String,
    taskbag: PathBuf,
    language: String,
) -> Result<(), Problem> {
    let project = allomos_store::versions::project_folder(&taskbag);
    let found = agents::find_agents();
    let chosen = found
        .iter()
        .find(|a| a.id == agent)
        .ok_or(Problem::AgentNotFound { agent })?;
    let request = allomos_store::app_share_request::request(&taskbag, &language)?;
    agents::ask(chosen, &project, &taskbag, &request)
}

#[tauri::command(async)]
pub(crate) fn app_changes(taskbag: PathBuf) -> allomos_store::events::AppChanges {
    allomos_store::events::unshared_app_changes(&taskbag)
}
