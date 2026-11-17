//! Working with agents from the app: the `alm` command, finding and connecting agents, the
//! requests the app types or shows (connect, build an app, update the taskbag, translate, work
//! on a task), and what agents report back (progress, confirm requests, the project's setup).

use std::path::{Path, PathBuf};

use allomos_core::Problem;
use allomos_store::progress::{Job, with_progress_in};
use allomos_store::{
    AgentSetup, ConfirmRequest, GitStep, SetupState, add_agent_instructions,
    agent_connect_request_with, agent_setup, agent_setup_request_in, answer_confirm,
    pending_confirms, picked_folder, record_project_folder, setup_state, versions,
};
use serde::Serialize;

use crate::{agents, alm_command};

/// Whether to offer **Install the alm command** (Task-A07-12, A07-14): in a built app whose own
/// `alm` is not the one Terminal finds (a Mac: through `/usr/local/bin`; Windows: first on PATH),
/// unless the person chose "Don't ask again" for this same mismatch.
#[tauri::command]
pub(crate) async fn alm_command_status() -> alm_command::AlmStatus {
    let dismissed = allomos_store::alm_offer_dismissed_for();
    if cfg!(windows) {
        let Some(own) = alm_command::windows_own() else {
            return alm_command::AlmStatus::default();
        };
        let (terminal, system) = alm_command::windows_terminal();
        return alm_command::decide(
            Some(&own),
            &alm_command::Place::Missing,
            &terminal,
            alm_command::Fix::UserPath { system },
            dismissed.as_deref(),
        );
    }
    let Some(bundled) = alm_command::bundled() else {
        return alm_command::AlmStatus::default();
    };
    alm_command::decide(
        Some(&bundled),
        &alm_command::place(),
        &alm_command::terminal(),
        alm_command::Fix::Link,
        dismissed.as_deref(),
    )
}

/// Make the app's own `alm` the one Terminal finds, only when the person clicks: on a Mac, link
/// `/usr/local/bin/alm` to it (asking for the Mac password); on Windows, put the app's folder
/// first in the person's own PATH. Returns the status afterwards; `cancelled` when the password
/// dialog was closed.
#[derive(Serialize)]
pub(crate) struct AlmInstalled {
    cancelled: bool,
    status: alm_command::AlmStatus,
}

#[tauri::command]
pub(crate) async fn install_alm_command() -> Result<AlmInstalled, Problem> {
    if cfg!(windows) {
        let own = alm_command::windows_own().ok_or_else(|| Problem::WriteFailed {
            path: "PATH".to_string(),
            detail: "this Allomos carries no alm".to_string(),
        })?;
        alm_command::install_windows(&own).map_err(|detail| Problem::WriteFailed {
            path: "PATH".to_string(),
            detail,
        })?;
        return Ok(AlmInstalled {
            cancelled: false,
            status: alm_command_status().await,
        });
    }
    let bundled = alm_command::bundled().ok_or_else(|| Problem::WriteFailed {
        path: alm_command::LINK.to_string(),
        detail: "this Allomos carries no alm".to_string(),
    })?;
    let done = alm_command::install(&bundled).map_err(|detail| Problem::WriteFailed {
        path: alm_command::LINK.to_string(),
        detail,
    })?;
    Ok(AlmInstalled {
        cancelled: !done,
        status: alm_command_status().await,
    })
}

/// "Don't ask again" for **Install the alm command**, for this mismatch only (the status's
/// `mismatch`): when Terminal's `alm` changes, the app asks again (Task-A07-14).
#[tauri::command]
pub(crate) fn dismiss_alm_offer(mismatch: String) {
    let _ = allomos_store::dismiss_alm_offer(&mismatch);
}

/// The AI agents on this computer that can be asked to add Allomos's instructions.
#[tauri::command]
pub(crate) async fn find_agents() -> Vec<agents::Agent> {
    agents::find_agents()
}

/// Which request each chosen agent gets from **Connect with the agents** (Task-A05-03), in the
/// order chosen: every agent gets the instructions request only (Task-A03-radish933). Finding the
/// project's tasks and out-of-date files are **Update the Taskbag…**'s job. The first agent's
/// request is the connect request; with several agents, each request starts with a note naming
/// the others.
fn connect_plan(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
) -> Vec<(String, String)> {
    connect_plan_with(agents, language, project, taskbag, true, GitStep::Ask)
}

/// [`connect_plan`]; `progress`: the first agent's request asks it to report its progress to the
/// app (Task-A05-08), unless a larger job's request does (Build a new app…); `git`: whether the
/// first agent asks before `git init` or, in a quiet goal run, runs it itself (Task-A03-35).
fn connect_plan_with(
    agents: &[String],
    language: &str,
    project: &Path,
    taskbag: &Path,
    progress: bool,
    git: GitStep,
) -> Vec<(String, String)> {
    let job = |job: Job, request: String| {
        if progress {
            with_progress_in(language, job, &request)
        } else {
            request
        }
    };
    let note = agents::shared_note(agents, language);
    agents
        .iter()
        .enumerate()
        .map(|(i, id)| {
            // The first agent does the job and reports its progress to the app (Task-A05-08).
            let request = if i == 0 {
                job(
                    Job::Connect,
                    agent_connect_request_with(language, project, taskbag, git),
                )
            } else {
                agent_setup_request_in(language, project, taskbag)
            };
            let request = match &note {
                Some(note) => format!("{note}\n\n{request}"),
                None => request,
            };
            (id.clone(), request)
        })
        .collect()
}

/// Open each chosen agent in `project` with its request typed in ([`connect_plan`]). Nothing is
/// sent: the person sends it. Returns the agents that opened.
#[tauri::command]
pub(crate) async fn connect_agents(
    agents: Vec<String>,
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
) -> Result<Vec<String>, Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let mut opened = Vec::new();
    for (id, request) in connect_plan(&agents, &language, &project, &taskbag) {
        let Some(agent) = found.iter().find(|a| a.id == id) else {
            continue;
        };
        agents::ask(agent, &project, &taskbag, &request)?;
        opened.push(id);
    }
    Ok(opened)
}

/// What the **Manual** choice of Connect with the agents shows (Task-A05-06): exactly the request
/// the first chosen agent would get ([`connect_plan`]: connect, or update when something is out
/// of date), as an agent opened through a link gets it ([`agents::link_request`]).
fn manual_plan(language: &str, project: &Path, taskbag: &Path) -> String {
    connect_plan(&[MANUAL.to_string()], language, project, taskbag)
        .remove(0)
        .1
}

/// The id of the Manual choice: no agent is opened; the person copies the request.
const MANUAL: &str = "manual";

#[tauri::command]
pub(crate) async fn manual_request(
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
) -> Result<String, Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let request = manual_plan(&language, &project, &taskbag);
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// The one request **Build a new app…** types in (Task-A03-25-02): the connect request the setup
/// wizard's first agent gets ([`connect_plan`]), then the goal request with the person's
/// description and run mode ([`allomos_store::goal_request_in`]). Manual shows the same text.
/// With `discuss` (the Start menu's **Discuss**, Task-A03-tundra474) the discuss request takes the
/// goal request's place ([`allomos_store::discuss_request_in`]): the agent writes the goal task
/// only, and the person talks it over before sending the line that starts the build.
fn build_app_plan(
    agent: &str,
    language: &str,
    project: &Path,
    taskbag: &Path,
    description: &str,
    quiet: bool,
    discuss: bool,
) -> String {
    // A quiet run creates the Git repository itself, without stopping to ask (Task-A03-35,
    // approver 2026-10-19); a normal run asks, as connecting does.
    let git = if quiet { GitStep::Create } else { GitStep::Ask };
    let connect = connect_plan_with(&[agent.to_string()], language, project, taskbag, false, git)
        .remove(0)
        .1;
    let goal = if discuss {
        allomos_store::discuss_request_in(language, description, quiet)
    } else {
        allomos_store::goal_request_in(language, description, quiet)
    };
    // The whole job is building the app: one progress request, at the end.
    with_progress_in(
        language,
        Job::BuildApp,
        &format!("{}\n\n{goal}", connect.trim_end()),
    )
}

/// Check what **Build a new app…** was given: a project folder and a description.
fn build_app_checked(project: &Path, description: &str) -> Result<(), Problem> {
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    if description.trim().is_empty() {
        return Err(Problem::BadUsage {
            detail: "Describe the app first.".to_string(),
        });
    }
    Ok(())
}

/// **Build a new app…** with a found agent (Task-A03-25-02): open it in `project` with
/// [`build_app_plan`] typed in. Nothing is sent: the person presses Enter.
#[tauri::command]
pub(crate) async fn build_app(
    agent: String,
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
    description: String,
    quiet: bool,
    discuss: bool,
) -> Result<(), Problem> {
    build_app_checked(&project, &description)?;
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = build_app_plan(
        &agent,
        &language,
        &project,
        &taskbag,
        &description,
        quiet,
        discuss,
    );
    agents::ask(chosen, &project, &taskbag, &request)
}

/// **Build a new app…** with Manual: the same request, as an agent opened through a link gets it
/// ([`manual_request`]), for the person to copy.
#[tauri::command]
pub(crate) async fn build_app_request(
    project: PathBuf,
    taskbag: PathBuf,
    language: String,
    description: String,
    quiet: bool,
    discuss: bool,
) -> Result<String, Problem> {
    build_app_checked(&project, &description)?;
    let request = build_app_plan(
        MANUAL,
        &language,
        &project,
        &taskbag,
        &description,
        quiet,
        discuss,
    );
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// The explanation of quiet and normal mode (Task-A03-21's `alm guide run-modes`, the person's
/// part from its first mode on), for the **Build a new app…** dialog.
#[tauri::command]
pub(crate) fn run_modes_text(language: String) -> String {
    run_modes_person(&language)
}

fn run_modes_person(language: &str) -> String {
    let text = allomos_store::run_modes(language)
        .or_else(|| allomos_store::run_modes("en"))
        .unwrap_or_default();
    match text.find("\n## ") {
        Some(at) => text[at + 1..].trim_end().to_string(),
        None => text.to_string(),
    }
}

/// Whether `project` has an `AGENTS.md` and a `CLAUDE.md`. The window notes which were missing
/// before agents are connected, and says when one appears that running agent sessions have not
/// read it (Task-A05-03).
#[derive(Serialize)]
pub(crate) struct InstructionFiles {
    agents_md: bool,
    claude_md: bool,
}

#[tauri::command(async)]
pub(crate) fn instruction_files(project: PathBuf) -> InstructionFiles {
    InstructionFiles {
        agents_md: project.join("AGENTS.md").is_file(),
        claude_md: project.join("CLAUDE.md").is_file(),
    }
}

/// The entries under `## Agents set up` in the taskbag's project notes (Task-A03-24): whether the
/// agent used last is registered decides which agents **Update the taskbag…** offers
/// (Task-A03-27). Only that section is read; nothing else is checked.
#[tauri::command(async)]
pub(crate) fn agents_set_up(taskbag: PathBuf) -> Vec<String> {
    allomos_store::handoff::agents_set_up(&taskbag)
}

/// Steps 2 to 4 of the open-folder flow for the taskbag at `root`: its project folder (recorded
/// here when AGENTS.md or CLAUDE.md sits beside `.allomos/`), whether that folder's agent
/// instructions mention Allomos, and whether the taskbag has tasks.
#[tauri::command(async)]
pub(crate) fn project_setup(root: PathBuf) -> Result<SetupState, Problem> {
    setup_state(&root)
}

/// Record `folder` as the project folder of the taskbag at `root`; then the flow's new state.
#[tauri::command(async)]
pub(crate) fn set_project_folder(root: PathBuf, folder: PathBuf) -> Result<SetupState, Problem> {
    record_project_folder(&root, &folder)?;
    setup_state(&root)
}

/// The job **Update the taskbag…** asks an agent to do, for its progress popup (Task-A05-08):
/// moving an older Allomos's tasks in, renewing an out-of-date section (Task-A03-27), or card
/// files kept outside the taskbag.
fn tasks_in_job(taskbag: &Path) -> Job {
    if versions::move_in(taskbag).is_some() {
        Job::MoveIn
    } else if versions::check_connected(taskbag).is_some_and(|c| !c.outdated_files().is_empty()) {
        Job::Update
    } else {
        Job::Cards
    }
}

/// **Update the taskbag…** (Task-A01-45, approver 2026-10-16: one button for an older taskbag and
/// for card files kept outside the taskbag, Task-A00-30): open the chosen agent in the project
/// folder of the taskbag at `taskbag`, with the request for whichever applies typed in
/// ([`allomos_store::agent_update_taskbag_request_in`]). Nothing is sent: the person sends it;
/// the agent shows its plan in the app (`alm confirm`) before anything changes.
#[tauri::command]
pub(crate) async fn move_tasks_in(
    agent: String,
    taskbag: PathBuf,
    language: String,
    translate: Option<String>,
    name: Option<String>,
) -> Result<(), Problem> {
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = tasks_in_plan(&taskbag, &language, translate.as_deref(), name.as_deref())?;
    agents::ask(chosen, &project, &taskbag, &request)
}

/// **Update the taskbag…** with Manual: the same request, as an agent opened through a link gets
/// it, for the person to copy.
#[tauri::command(async)]
pub(crate) fn move_tasks_in_request(
    taskbag: PathBuf,
    language: String,
    translate: Option<String>,
    name: Option<String>,
) -> Result<String, Problem> {
    let request = tasks_in_plan(&taskbag, &language, translate.as_deref(), name.as_deref())?;
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// Update the taskbag…'s request with its progress line; `translate`, the language code the
/// person chose to translate into (Task-A05-cape996), none for Don't translate.
fn tasks_in_plan(
    taskbag: &Path,
    language: &str,
    translate: Option<&str>,
    name: Option<&str>,
) -> Result<String, Problem> {
    let translate = translate.map(str::trim).filter(|c| !c.is_empty());
    if let Some(code) = translate.filter(|c| !allomos_core::valid_language(c)) {
        return Err(Problem::InvalidLanguage {
            value: code.to_string(),
        });
    }
    let project = allomos_store::versions::project_folder(taskbag);
    Ok(with_progress_in(
        language,
        tasks_in_job(taskbag),
        &allomos_store::agent_update_taskbag_request_with(
            language,
            &project,
            taskbag,
            translate.map(|code| allomos_store::TranslateTarget { code, name }),
        ),
    ))
}

/// **Translate the taskbag…** (Task-A05-board831): the request for translating the taskbag at
/// `taskbag` into `target` (a language code; `name` as the menu showed it), with the progress
/// line, in the interface language `language`.
fn translate_plan(taskbag: &Path, language: &str, target: &str, name: Option<&str>) -> String {
    let project = allomos_store::versions::project_folder(taskbag);
    with_progress_in(
        language,
        Job::Translate,
        &allomos_store::agent_translate_request_in(language, &project, taskbag, target, name),
    )
}

/// Open the chosen agent in the project folder with [`translate_plan`] typed in. Nothing is sent:
/// the person sends it; the agent shows its plan (`alm confirm`) before anything changes.
#[tauri::command]
pub(crate) async fn translate_taskbag(
    agent: String,
    taskbag: PathBuf,
    language: String,
    target: String,
    name: Option<String>,
) -> Result<(), Problem> {
    if !allomos_core::valid_language(target.trim()) {
        return Err(Problem::InvalidLanguage { value: target });
    }
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    let request = translate_plan(&taskbag, &language, target.trim(), name.as_deref());
    agents::ask(chosen, &project, &taskbag, &request)
}

/// **Translate the taskbag…** with Manual: the same request, for the person to copy.
#[tauri::command(async)]
pub(crate) fn translate_request(
    taskbag: PathBuf,
    language: String,
    target: String,
    name: Option<String>,
) -> Result<String, Problem> {
    if !allomos_core::valid_language(target.trim()) {
        return Err(Problem::InvalidLanguage { value: target });
    }
    let request = translate_plan(&taskbag, &language, target.trim(), name.as_deref());
    Ok(agents::link_request(
        &request,
        alm_command::own().as_deref(),
    ))
}

/// **Work on it…** (Task-A03-galley051): the taskbag at `taskbag` as an agent working in its project
/// folder passes it to `--taskbag`: relative to that folder, written with `/` (absolute when there
/// is no relative way).
fn taskbag_arg(project: &Path, taskbag: &Path) -> String {
    let (project, taskbag) = (allomos_store::tidy(project), allomos_store::tidy(taskbag));
    allomos_store::relative_path(&project, &taskbag)
        .map(|p| allomos_store::portable_path(&p))
        .unwrap_or_else(|| taskbag.display().to_string())
}

/// **Work on it…**: the taskbag's path for the request the interface writes ([`taskbag_arg`]).
#[tauri::command]
pub(crate) fn proceed_taskbag(taskbag: PathBuf) -> String {
    taskbag_arg(&allomos_store::versions::project_folder(&taskbag), &taskbag)
}

/// **Work on it…**: open the chosen agent in the project folder with `request` (written by the
/// interface, perhaps edited by the person) typed in. Nothing is sent: the person sends it.
#[tauri::command]
pub(crate) async fn proceed_with_agent(
    agent: String,
    taskbag: PathBuf,
    request: String,
) -> Result<(), Problem> {
    let project = allomos_store::versions::project_folder(&taskbag);
    if !project.is_dir() {
        return Err(Problem::StartupNotAFolder {
            path: project.display().to_string(),
        });
    }
    let found = agents::find_agents();
    let Some(chosen) = found.iter().find(|a| a.id == agent) else {
        return Err(Problem::AgentNotFound { agent });
    };
    agents::ask(chosen, &project, &taskbag, &request)
}

/// **Work on it…** with Manual: `request` as the person copies it (with this Allomos's own `alm`, as
/// the other Manual requests are).
#[tauri::command]
pub(crate) fn proceed_manual_request(request: String) -> String {
    agents::link_request(&request, alm_command::own().as_deref())
}

/// The agent `alm` last ran under for a task in the taskbag at `root`, from its events
/// (Task-A03-27); none when no event records one.
#[tauri::command(async)]
pub(crate) fn last_agent(root: PathBuf) -> Option<allomos_store::events::LastAgent> {
    allomos_store::events::last_agent(&root)
}

/// What agents report about the jobs they do in `project` (`alm progress`, Task-A05-08), the
/// earliest started first.
#[tauri::command(async)]
pub(crate) fn progress_reports(project: PathBuf) -> Vec<allomos_store::progress::Progress> {
    allomos_store::progress::reports(&project)
}

/// The person closed a finished (or silent) report: remove it.
#[tauri::command(async)]
pub(crate) fn dismiss_progress(project: PathBuf, id: String) {
    allomos_store::progress::dismiss(&project, &id)
}

/// Questions an agent is asking about `project` with `alm confirm`, oldest first. Listing them
/// marks them seen, so `alm` knows this window shows them.
#[tauri::command(async)]
pub(crate) fn confirm_requests(project: PathBuf) -> Vec<ConfirmRequest> {
    pending_confirms(&project)
}

/// The person's answer to question `id`: go ahead (`yes`) or cancel.
#[tauri::command(async)]
pub(crate) fn answer_confirm_request(
    project: PathBuf,
    id: String,
    yes: bool,
    fingerprint: Option<String>,
) -> Result<(), Problem> {
    answer_confirm(&project, &id, yes, fingerprint.as_deref())
}

/// Do the AI agents know about this project? The project folder and which agents' instruction
/// files tell them about Allomos, for the taskbag settings' Agents section (Task-A05-bag963). The
/// window's agent line that showed it is gone (Task-A03-root435).
#[derive(Serialize)]
pub(crate) struct AgentStatus {
    /// The project folder the status is about.
    project: PathBuf,
    /// Which agents' instruction files tell them about Allomos.
    setup: AgentSetup,
}

/// Reads `project.json` and the instruction files; it does not load the tasks.
#[tauri::command]
pub(crate) async fn agent_status(root: PathBuf) -> AgentStatus {
    // The recorded project folder; before one is recorded, the folder holding `.allomos/`.
    let project = allomos_store::recorded_project_folder(&root)
        .unwrap_or_else(|| picked_folder(&root).unwrap_or(&root).to_path_buf());
    AgentStatus {
        setup: agent_setup(&project),
        project,
    }
}

/// Which agents a folder's instruction files tell about Allomos, for the Agents section of the
/// taskbag's settings (Task-A05-bag963): asked once for each project folder.
#[tauri::command(async)]
pub(crate) fn agent_setup_of(project: PathBuf) -> allomos_store::AgentSetup {
    agent_setup(&project)
}

/// Write Allomos's section into `project`'s AGENTS.md and CLAUDE.md directly, for when no
/// agent is available. Returns the files changed.
#[tauri::command(async)]
pub(crate) fn write_agent_files(project: PathBuf) -> Result<Vec<String>, Problem> {
    add_agent_instructions(&project).map_err(|e| Problem::WriteFailed {
        path: project.join("AGENTS.md").display().to_string(),
        detail: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskbag_commands::{create_taskbag, move_taskbag_aside, version_check};

    /// Task-A03-galley051: Work on it…'s request names the taskbag as the agent in the project folder
    /// passes it, beside the project or inside it.
    #[test]
    fn proceed_names_the_taskbag_from_the_project_folder() {
        assert_eq!(
            taskbag_arg(
                Path::new("/work/app"),
                Path::new("/work/app-taskbag/.allomos/taskbag")
            ),
            "../app-taskbag/.allomos/taskbag"
        );
        assert_eq!(
            taskbag_arg(
                Path::new("/work/app"),
                Path::new("/work/app/.allomos/taskbag")
            ),
            ".allomos/taskbag"
        );
    }

    use allomos_store::agent_connect_request_in;

    #[test]
    fn an_out_of_date_project_still_gets_the_connect_request() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-update-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        // A section from before versions: connected, but out of date.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        // Connect only tells the agent about Allomos (Task-A03-radish933); the out-of-date
        // notice is Update the Taskbag…'s, so the connect request is the same.
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert!(!plan[0].1.contains("older Allomos"));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());

        // Renewed: still the usual connect request.
        std::fs::write(dir.join("AGENTS.md"), allomos_store::agent_section()).unwrap();
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert!(!version_check(taskbag).unwrap().needs_update());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Task-A01-45: an older Allomos's taskbag gets the move-in request, before and after the app
    /// renames it aside; once the first task is moved in, the usual requests again.
    #[test]
    fn an_older_taskbag_gets_the_move_in_request() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-move-in-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        std::fs::write(dir.join("AGENTS.md"), allomos_store::agent_section()).unwrap();
        let project = taskbag.join("project.json");
        let text = std::fs::read_to_string(&project).unwrap();
        std::fs::write(&project, text.replace(allomos_core::FORMAT, "allomos/2")).unwrap();

        // Not renamed yet. Connect asks for the instructions only, not for the move-in.
        let check = version_check(taskbag.clone()).unwrap();
        assert!(!check.move_in.as_ref().unwrap().prepared);
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(!plan[0].1.starts_with("Please move the tasks of a taskbag"));
        assert!(!plan[0].1.contains(" init --id "));
        assert_eq!(manual_plan("en", &dir, &taskbag), plan[0].1);
        // The wizard can reach the Connect step: the project folder is known without reading.
        let setup = project_setup(taskbag.clone()).unwrap();
        assert_eq!(setup.project_folder.as_deref(), Some(dir.as_path()));

        // Move the tasks in…: the old folder aside, a new empty taskbag in its place.
        let opened = move_taskbag_aside(taskbag.clone()).unwrap();
        let bag = opened.loaded.taskbag.unwrap();
        assert!(bag.tasks.is_empty());
        let aside = dir.join(".allomos").join("taskbag-allomos-2");
        assert_eq!(
            std::fs::read_to_string(aside.join("project.json")).unwrap(),
            text.replace(allomos_core::FORMAT, "allomos/2")
        );
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert!(!plan[0].1.contains("Allomos renamed the old taskbag"));
        assert!(!plan[0].1.contains(" init --id "));
        assert!(version_check(taskbag.clone()).unwrap().needs_update());
        // Update the taskbag… asks for the move-in, reported as that job (Task-A01-45); once the
        // tasks are in, it is about card files (below).
        assert_eq!(tasks_in_job(&taskbag), Job::MoveIn);
        let request = move_tasks_in_request(taskbag.clone(), "en".into(), None, None).unwrap();
        assert!(request.contains("Allomos renamed the old taskbag (format allomos/2;"));
        // No agent setup in it (Task-A03-27): each agent registers itself.
        assert!(!request.contains("agent setup request"));
        // Asking twice changes nothing.
        assert!(move_taskbag_aside(taskbag.clone()).is_ok());
        assert!(!dir.join(".allomos").join("taskbag-allomos-4").exists());

        // The agent moved a task in: the usual request again.
        allomos_store::add_area(
            &taskbag,
            allomos_store::Actor::APP,
            std::time::Duration::from_secs(1),
            Some("A1"),
            "Area",
        )
        .unwrap();
        allomos_store::add_task(
            &taskbag,
            allomos_store::Actor::APP,
            std::time::Duration::from_secs(1),
            &serde_json::from_str(r#"{"area":"A1","title":"Bake"}"#).unwrap(),
        )
        .unwrap();
        let plan = connect_plan(&["claude_app".into()], "en", &dir, &taskbag);
        assert_eq!(
            plan[0].1,
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        assert_eq!(tasks_in_job(&taskbag), Job::Cards);
        assert!(!version_check(taskbag).unwrap().needs_update());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn manual_shows_the_request_an_agent_would_get() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-manual-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        // Not connected yet: the connect request, in every language, as automatic sending.
        for language in ["en", "zh-CN", "ja"] {
            let auto = connect_plan(&["claude_app".into()], language, &dir, &taskbag);
            assert_eq!(manual_plan(language, &dir, &taskbag), auto[0].1);
        }
        assert_eq!(
            manual_plan("en", &dir, &taskbag),
            with_progress_in(
                "en",
                Job::Connect,
                &agent_connect_request_in("en", &dir, &taskbag)
            )
        );
        // Out of date: still the connect request, as automatic sending.
        std::fs::write(dir.join("AGENTS.md"), "## Tasks: Allomos\n\nalm next\n").unwrap();
        let auto = connect_plan(&["codex_cli".into()], "en", &dir, &taskbag);
        let manual = manual_plan("en", &dir, &taskbag);
        assert_eq!(manual, auto[0].1);
        assert!(!manual.starts_with("This project was set up by an older Allomos"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn build_a_new_app_types_the_connect_request_then_the_goal() {
        let dir =
            std::env::temp_dir().join(format!("allomos-desktop-build-app-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let taskbag = dir.join(allomos_store::DEFAULT_DIR);
        allomos_store::init(&taskbag, "p", "P").unwrap();
        let description = "A recipe box for two people.\nIt runs in the browser.";
        for language in ["en", "zh-CN", "ja"] {
            for quiet in [true, false] {
                let auto = build_app_plan(
                    "claude_app",
                    language,
                    &dir,
                    &taskbag,
                    description,
                    quiet,
                    false,
                );
                // One request: the wizard's connect request first, then the goal request. A quiet
                // run creates the Git repository itself; a normal run asks (Task-A03-35).
                let git = if quiet { GitStep::Create } else { GitStep::Ask };
                let connect = agent_connect_request_with(language, &dir, &taskbag, git);
                assert!(auto.starts_with(connect.trim_end()), "{language}");
                assert_eq!(
                    connect.contains("alm ai-decided <"),
                    quiet,
                    "{language}: only a quiet run records git init as an AI decision"
                );
                assert_eq!(
                    connect == agent_connect_request_in(language, &dir, &taskbag),
                    !quiet,
                    "{language}: a normal run asks, as connecting does"
                );
                let goal = allomos_store::goal_request_in(language, description, quiet);
                assert!(auto.contains(goal.trim_end()), "{language}");
                // One progress request, for the whole job (Task-A05-08).
                assert_eq!(
                    auto,
                    with_progress_in(
                        language,
                        Job::BuildApp,
                        &format!("{}\n\n{goal}", connect.trim_end())
                    )
                );
                assert_eq!(auto.matches("alm progress --finished").count(), 1);
                // The goal prompt in the task language, the description and the run mode.
                assert!(
                    auto.contains(allomos_store::goal(language).unwrap()),
                    "{language}"
                );
                assert!(
                    auto.contains("> A recipe box for two people.\n> It runs in the browser."),
                    "{language}"
                );
                let mode = if quiet {
                    "Run mode: quiet"
                } else {
                    "Run mode: normal"
                };
                assert!(auto.contains(mode), "{language}");
                // Manual shows the same request, for every agent.
                assert_eq!(
                    build_app_plan(MANUAL, language, &dir, &taskbag, description, quiet, false),
                    auto
                );
                assert_eq!(
                    build_app_plan(
                        "codex_cli",
                        language,
                        &dir,
                        &taskbag,
                        description,
                        quiet,
                        false
                    ),
                    auto
                );
                // Discuss (Task-A03-tundra474): the same connect request, then the discuss
                // request in place of the goal request, and one progress request.
                let talk = build_app_plan(
                    "claude_app",
                    language,
                    &dir,
                    &taskbag,
                    description,
                    quiet,
                    true,
                );
                let discuss = allomos_store::discuss_request_in(language, description, quiet);
                assert_eq!(
                    talk,
                    with_progress_in(
                        language,
                        Job::BuildApp,
                        &format!("{}\n\n{discuss}", connect.trim_end())
                    )
                );
                assert!(
                    !talk.contains(allomos_store::goal(language).unwrap()),
                    "{language}"
                );
                assert!(talk.contains(mode), "{language}");
            }
        }
        // Nothing to build without a description or a folder.
        assert!(matches!(
            build_app_checked(&dir, "  \n"),
            Err(Problem::BadUsage { .. })
        ));
        assert!(matches!(
            build_app_checked(&dir.join("missing"), description),
            Err(Problem::StartupNotAFolder { .. })
        ));
        assert_eq!(build_app_checked(&dir, description), Ok(()));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn the_dialog_explains_both_run_modes() {
        for language in ["en", "zh-CN", "ja"] {
            let text = run_modes_person(language);
            assert!(text.starts_with("## "), "{language}");
            assert_eq!(text.matches("\n## ").count(), 1, "{language}");
            assert!(!text.contains("For the agent"), "{language}");
            assert!(text.contains("`alm human`"), "{language}");
        }
    }

    #[test]
    fn connect_asks_only_for_the_instructions() {
        let project = PathBuf::from("/work/bakery");
        let taskbag = project.join(allomos_store::DEFAULT_DIR);
        // Only the first agent does the job, so only it reports progress (Task-A05-08).
        let connect = with_progress_in(
            "en",
            Job::Connect,
            &agent_connect_request_in("en", &project, &taskbag),
        );
        let setup = agent_setup_request_in("en", &project, &taskbag);

        // One agent: the connect request, with no note.
        let one = connect_plan(&["claude_app".into()], "en", &project, &taskbag);
        assert_eq!(one, [("claude_app".to_string(), connect.clone())]);

        // Two: the first chosen gets the connect request, the other the same instructions; both
        // are told about each other, and neither is asked to look for tasks (Task-A03-radish933).
        let two = connect_plan(
            &["claude_app".into(), "codex_cli".into()],
            "en",
            &project,
            &taskbag,
        );
        assert_eq!(two.len(), 2);
        assert_eq!(two[0].0, "claude_app");
        assert!(two[0].1.ends_with(&connect) && two[0].1.starts_with("Note: "));
        assert!(!two[0].1.contains("alm confirm"));
        assert_eq!(two[1].0, "codex_cli");
        assert!(two[1].1.ends_with(&setup) && two[1].1.starts_with("Note: "));
        assert!(!two[1].1.contains("alm confirm"));
        assert!(connect_plan(&[], "en", &project, &taskbag).is_empty());
    }

    /// Decision 0017: the wizard's steps 2 and 3 run no Git. A folder outside Git is set up
    /// without errors and stays outside Git; the agent connected in step 4 is asked to set Git up
    /// with the person and commit. The changes not shared yet come from the change history.
    #[test]
    fn the_wizard_sets_up_a_folder_outside_git_without_running_git() {
        let base = std::env::temp_dir().join(format!(
            "allomos-no-git-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let root = plain.join(allomos_store::DEFAULT_DIR);
        // Step 2 (Create) and step 3 (the project folder).
        let opened = create_taskbag(root.clone(), "Plain".into(), "en".into()).unwrap();
        assert!(opened.readme_added);
        let setup = set_project_folder(root.clone(), plain.clone()).unwrap();
        assert!(setup.project_folder.is_some());
        assert!(!plain.join(".git").exists() && !root.join(".git").exists());
        // Step 4's request asks the agent to set up Git with the person, then commit.
        let (project, taskbag) = (plain.as_path(), root.as_path());
        for language in ["en", "zh-CN", "ja"] {
            let request = agent_connect_request_in(language, project, taskbag);
            assert!(request.contains("`git init`"), "{language}");
            assert!(
                request.contains("commit -m \"Add Allomos taskbag\""),
                "{language}"
            );
        }
        // The app's own changes (create, project folder) are not shared yet.
        assert_eq!(allomos_store::events::unshared_app_changes(&root).count, 2);
        std::fs::remove_dir_all(&base).unwrap();
    }

    /// Task-A03-root435: `agent_status` names the recorded project folder (the Agents section
    /// asks it); it no longer counts the changes not shared yet.
    #[test]
    fn agent_status_names_the_project_folder() {
        let base = std::env::temp_dir().join(format!(
            "allomos-agent-status-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let plain = base.join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        let root = plain.join(allomos_store::DEFAULT_DIR);
        create_taskbag(root.clone(), "Plain".into(), "en".into()).unwrap();
        set_project_folder(root.clone(), plain.clone()).unwrap();
        let status = tauri::async_runtime::block_on(agent_status(root.clone()));
        assert_eq!(
            status.project.canonicalize().unwrap(),
            plain.canonicalize().unwrap()
        );
        std::fs::remove_dir_all(&base).unwrap();
    }
}
