//! `alm init`, `alm confirm`, `alm progress`, `alm brief`, `alm handoff`, `alm move-in`, `alm translate` and `alm guide`: the taskbag as a whole, its handoffs and its guides.

use std::path::Path;

use allomos_core::ops::{self};
use allomos_core::{FORMAT, Problem};
use allomos_store::handoff;
use allomos_store::{
    Actor, GUIDE_TOPICS, LOCALIZED_TOPICS, add_agent_instructions, agent_section,
    agent_section_for, agent_setup_request, default_root, guide, guide_topic, id_from_name,
    init_with_labels, load, localized, pick_ui_language, picked_folder, project_folder_of,
    read_text, saved_ui_language, task_language, taskbag_of, versions,
};
use serde_json::json;

use crate::{
    Cli, Command, Done, confirm, confirm_project, locate, lock_wait, notes_reminder, task_json,
    task_line,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Init {
            name,
            id,
            labels,
            write_agent_files,
        } => {
            let root = match &cli.taskbag {
                Some(p) => cwd.join(p),
                // Here, never the repository's top folder: `alm` reads no Git (decision 0017).
                None => default_root(cwd),
            };
            let folder = root
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "project".to_string());
            let name = name.clone().unwrap_or_else(|| folder.clone());
            let id = id.clone().unwrap_or_else(|| id_from_name(&folder));
            let labels = match labels {
                Some(list) => list
                    .split(',')
                    .map(str::trim)
                    .filter(|l| !l.is_empty())
                    .map(String::from)
                    .collect(),
                None => Vec::new(),
            };
            let root = init_with_labels(&root, &id, &name, labels, Actor::alm(me))?;
            let mut text = format!("Created an empty taskbag at {}\n", root.display());
            // Tell AI agents about Allomos in the project's instruction files: by default ask
            // the agent running `alm init` to merge the section in; with --write-agent-files, write it.
            let mut agent_files = Vec::new();
            let mut warnings = Vec::new();
            let project = picked_folder(&root).unwrap_or(&root).to_path_buf();
            let agent_setup = (!*write_agent_files).then(|| agent_setup_request(&project, &root));
            if let (true, Some(folder)) = (*write_agent_files, picked_folder(&root)) {
                match add_agent_instructions(folder) {
                    Ok(files) => agent_files = files,
                    Err(e) => warnings.push(Problem::WriteFailed {
                        path: folder.join("AGENTS.md").display().to_string(),
                        detail: e.to_string(),
                    }),
                }
            }
            if !agent_files.is_empty() {
                text.push_str(&format!(
                    "Added Allomos's section for AI agents to {}\n",
                    agent_files.join(" and ")
                ));
            }
            if let Some(request) = &agent_setup {
                text.push_str("\nNext step for the AI agent running this command:\n\n");
                text.push_str(request);
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "agent_files": agent_files, "agent_setup": agent_setup }),
                text,
            );
            done.warnings = warnings;
            Ok(done)
        }
        Command::Confirm {
            title,
            plan,
            timeout,
            evidence,
            task,
        } => confirm(
            cli,
            cwd,
            title,
            plan,
            *timeout,
            evidence.as_deref(),
            task.as_deref(),
        ),
        Command::Progress {
            title,
            step,
            done,
            of,
            finished,
            waiting,
        } => {
            // Passed on to the app as it is (Task-A05-08); alm checks nothing.
            let project = confirm_project(cli, cwd);
            let p = allomos_store::progress::report(
                &project,
                cli.holder.as_deref(),
                &allomos_store::progress::Report {
                    title: title.as_deref(),
                    step: step.as_deref(),
                    done: *done,
                    of: *of,
                    finished: finished.as_deref(),
                    waiting: waiting.as_deref(),
                },
            )?;
            let count = match (p.done, p.of) {
                (Some(d), Some(o)) => format!(" ({d} of {o})"),
                _ => String::new(),
            };
            let text = match (&p.finished, &p.waiting) {
                (Some(f), _) => format!("Shown in the Allomos app: {}: finished: {f}\n", p.title),
                (None, Some(q)) => format!(
                    "Shown in the Allomos app: {}: waiting for the person's reply: {q}\n",
                    p.title
                ),
                (None, None) => format!(
                    "Shown in the Allomos app: {}: {}{count}\n",
                    p.title,
                    p.step.as_deref().unwrap_or("…")
                ),
            };
            Ok(Done::new(
                json!({ "progress": p, "project": project }),
                text,
            ))
        }
        Command::Brief => {
            // With a taskbag, the section names it when it is outside the project folder.
            let text = match locate(cli, cwd) {
                Ok(root) => {
                    let project = load(&root)
                        .taskbag
                        .and_then(|bag| project_folder_of(&root, &bag.project))
                        .unwrap_or_else(|| picked_folder(&root).unwrap_or(&root).to_path_buf());
                    agent_section_for(&project, &root)
                }
                Err(_) => agent_section(),
            };
            Ok(Done::new(json!({ "brief": text }), text))
        }
        Command::Handoff {
            id,
            text,
            from,
            evidence,
        } => {
            let root = locate(cli, cwd)?;
            let note = match from {
                Some(file) => {
                    let file = cwd.join(file);
                    read_text(&file).map_err(|e| Problem::ReadFailed {
                        detail: format!("{}: {e}", file.display()),
                    })?
                }
                None => text.clone().unwrap_or_default(),
            };
            let now = handoff::utc_now();
            let day = now.get(..10).unwrap_or(&now);
            let changed = allomos_store::change_task(&root, Actor::alm(me), lock_wait(), |bag| {
                ops::hand_off(bag, id, me, &note, evidence.as_deref(), day).map(|o| (o, "handoff"))
            })?;
            let notes = handoff::project_notes(&root);
            let text = format!(
                "{}\nHanded off: its handoff note is set{}; `alm show {}` shows it.\n{}",
                task_line(&changed.task),
                if evidence.is_some() {
                    " and the evidence is under its description's Evidence"
                } else {
                    ""
                },
                changed.task.id,
                notes_reminder(&notes)
            );
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&changed.task), "project_notes": notes }),
                text,
            );
            done.warnings = changed.warnings;
            done.issues = changed.issues;
            Ok(done)
        }
        Command::MoveIn {
            language,
            translate,
            name,
        } => {
            let root = locate(cli, cwd)?;
            if let versions::FormatState::Newer { found } = versions::format_state(&root) {
                return Err(Problem::TaskbagNewer {
                    found,
                    supported: FORMAT.to_string(),
                });
            }
            let language = language.clone().unwrap_or_else(|| {
                saved_ui_language().unwrap_or_else(|| {
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    pick_ui_language(&system).to_string()
                })
            });
            // An older taskbag is not read: its project folder comes from its project.json as it is.
            let project = versions::project_folder(&root);
            // The same request as the app's Update the taskbag… (Task-A01-45, A03-27): the move-in
            // request for an older taskbag; otherwise whatever is due (an out-of-date section,
            // card files kept outside, old handoff files), with the person's translate choice
            // (Task-A03-totem655; any language since Task-A05-cape996).
            let code = translate.as_deref().map(|c| match c.trim() {
                "" => language.as_str(),
                code => code,
            });
            if let Some(code) = code.filter(|c| !allomos_core::valid_language(c)) {
                return Err(Problem::InvalidLanguage {
                    value: code.to_string(),
                });
            }
            let request = allomos_store::agent_update_taskbag_request_with(
                &language,
                &project,
                &root,
                code.map(|code| allomos_store::TranslateTarget {
                    code,
                    name: name.as_deref(),
                }),
            );
            let Some(move_in) = versions::move_in(&root) else {
                let cards = allomos_store::cards::outside(&root);
                return Ok(Done::new(
                    json!({
                        "taskbag": root,
                        "move_in": { "needed": false },
                        "cards_outside": cards,
                        "language": language,
                        "request": request,
                    }),
                    request,
                ));
            };
            Ok(Done::new(
                json!({
                    "taskbag": root,
                    "move_in": {
                        "needed": true,
                        "from": move_in.found,
                        "to": FORMAT,
                        "old": move_in.old,
                        "prepared": move_in.prepared,
                    },
                    "language": language,
                    "request": request,
                }),
                request,
            ))
        }
        Command::Translate {
            code,
            name,
            language,
        } => {
            let root = locate(cli, cwd)?;
            let code = code.trim();
            if !allomos_core::valid_language(code) {
                return Err(Problem::InvalidLanguage {
                    value: code.to_string(),
                });
            }
            let language = language.clone().unwrap_or_else(|| {
                saved_ui_language().unwrap_or_else(|| {
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    pick_ui_language(&system).to_string()
                })
            });
            let project = versions::project_folder(&root);
            let request = allomos_store::agent_translate_request_in(
                &language,
                &project,
                &root,
                code,
                name.as_deref(),
            );
            Ok(Done::new(
                json!({
                    "taskbag": root,
                    "code": code,
                    "language": language,
                    "request": request,
                }),
                request,
            ))
        }
        Command::Guide { words, language } => {
            // A topic may be written as separate words (`alm guide github release`,
            // Task-A03-planet530); words after it are the guide's, for the agent to read.
            let (topic, rest) = if words.is_empty() {
                (None, Vec::new())
            } else {
                match guide_topic(words) {
                    Some((topic, rest)) => (Some(topic), rest),
                    None => {
                        return Err(Problem::BadUsage {
                            detail: format!(
                                "There is no guide topic \"{}\". The topics: {}.",
                                words.join(" "),
                                GUIDE_TOPICS.join(", ")
                            ),
                        });
                    }
                }
            };
            let after = if rest.is_empty() {
                String::new()
            } else {
                format!(
                    "Words after the topic, for you to read as this guide says: {}\n\n",
                    rest.join(" ")
                )
            };
            let Some(name) = topic.filter(|t| LOCALIZED_TOPICS.contains(t)) else {
                let text = format!("{after}{}", guide(topic).unwrap_or_default());
                return Ok(Done::new(
                    json!({ "topic": topic, "words": rest, "guide": text }),
                    text,
                ));
            };
            // The person reads it (or reads along): in the task language (Task-A03-21,
            // Task-A03-24). Without a taskbag here, the language the app would use.
            let language = match language {
                Some(code) => code.trim().to_string(),
                None => {
                    let system: Vec<String> = sys_locale::get_locales().collect();
                    match locate(cli, cwd)
                        .ok()
                        .and_then(|root| taskbag_of(&root, load(&root)).ok())
                    {
                        Some((bag, _)) => task_language(&bag.project, &system).language,
                        None => saved_ui_language()
                            .unwrap_or_else(|| pick_ui_language(&system).to_string()),
                    }
                }
            };
            let (text, written) = match localized(name, &language) {
                Some(text) => (text.to_string(), true),
                // The goal prompt and the release procedure are the agent's to follow, not to
                // show (Task-A03-25-01, Task-A03-planet530).
                None if name == "goal" || name == "github-release" => (
                    format!(
                        "This text is not written in {language} yet: follow a faithful translation \
                         of it into {language}, keeping commands as they are{}.\n\n{}",
                        if name == "goal" {
                            format!(", and write the tasks in {language}")
                        } else {
                            String::new()
                        },
                        localized(name, "en").unwrap_or_default()
                    ),
                    false,
                ),
                None => (
                    format!(
                        "This text is not written in {language} yet: show the person a faithful \
                         translation into {language}, keeping commands as they are.\n\n{}",
                        localized(name, "en").unwrap_or_default()
                    ),
                    false,
                ),
            };
            let text = format!("{after}{text}");
            Ok(Done::new(
                json!({ "topic": topic, "words": rest, "language": language, "written_in_language": written, "guide": text }),
                text,
            ))
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}
