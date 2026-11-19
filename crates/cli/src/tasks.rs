//! The commands that change or read one task: its traces, labels, session, waits and blockers,
//! finishing and checks, AI decisions, approvals, effects, exceptions, links, title, description,
//! cards, heartbeats, folders and state.

use std::path::Path;

use allomos_core::ops;
use allomos_core::{Problem, State};
use allomos_store::{Actor, Lock, load, plain_path, read_text, taskbag_of};
use serde_json::json;

use crate::{
    CardsWhat, Cli, Command, Done, approve, change, check_results, classify, decision_text, effects, exceptions,
    folders, for_codex, heartbeat, human, links, locate, lock_wait, non_blank, quoted, redact,
    save_project, stop_lines, task_json, task_json_in, task_line, traces_text, trail,
};

/// Runs the commands this module holds; `execute` in `lib.rs` sends only those here.
pub(crate) fn execute(cli: &Cli, cwd: &Path) -> Result<Done, Problem> {
    let me = cli.holder.as_deref();
    match &cli.command {
        Command::Trace { id, refs, remove } => {
            let root = locate(cli, cwd)?;
            let (add, take) = if *remove {
                (&[][..], &refs[..])
            } else {
                (&refs[..], &[][..])
            };
            let (task, after, issues) =
                allomos_store::set_traces(&root, Actor::alm(me), lock_wait(), id, add, take)?;
            let mut text = format!("{}\n", task_line(&task));
            text.push_str(&format!("  traces: {}\n", traces_text(&after)));
            text.push_str(
                "Fact: alm checks only how each is written; check that the requirement or decision exists (REQUIREMENTS.md, docs/decisions/).\n",
            );
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&task), "traces": after }),
                text,
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Traces { reference } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            let recorded = allomos_store::traces(&root);
            let line = |id: &str| match bag.tasks.iter().find(|t| t.id == id) {
                Some(t) => task_line(t),
                None => id.to_string(),
            };
            let mut done = match reference {
                Some(given) => {
                    let reference = allomos_core::traces::trace_ref(given)?;
                    let ids = allomos_core::traces::tasks_for(&recorded, &reference);
                    let mut text = if ids.is_empty() {
                        format!("No task records {reference}.\n")
                    } else {
                        format!("Tasks that serve {reference}:\n")
                    };
                    for id in &ids {
                        text.push_str(&format!("  {}\n", line(id)));
                    }
                    Done::new(
                        json!({ "taskbag": root, "reference": reference, "tasks": ids }),
                        text,
                    )
                }
                None => {
                    let index = allomos_core::traces::index(&recorded);
                    let mut text = if index.is_empty() {
                        "No task records a requirement or a decision yet (`alm trace <id> <ref>…`).\n"
                            .to_string()
                    } else {
                        String::new()
                    };
                    for (reference, ids) in &index {
                        text.push_str(&format!("{reference}:\n"));
                        for id in ids {
                            text.push_str(&format!("  {}\n", line(id)));
                        }
                    }
                    let json_index: serde_json::Map<String, serde_json::Value> = index
                        .iter()
                        .map(|(r, ids)| (r.clone(), json!(ids)))
                        .collect();
                    Done::new(json!({ "taskbag": root, "traces": json_index }), text)
                }
            };
            done.issues = issues;
            Ok(done)
        }
        Command::Label { id, label, remove } => change(cli, cwd, |bag| {
            if *remove {
                allomos_core::labels::remove(bag, id, me, label)
            } else {
                ops::set_label(bag, id, me, label)
            }
        }),
        Command::Session {
            id,
            session_agent,
            session_id,
            session_kind,
            app_session_id,
        } => {
            let given = allomos_core::AgentSession {
                agent: non_blank(session_agent),
                id: non_blank(session_id),
                kind: non_blank(session_kind),
                app_session_id: non_blank(app_session_id),
                ..Default::default()
            };
            change(cli, cwd, |bag| ops::set_session(bag, id, me, given.clone()))
        }
        Command::Wait { id, on, who } => {
            change(cli, cwd, |bag| ops::wait(bag, id, me, on, who.as_deref()))
        }
        Command::Block { id, by } => {
            let mut done = change(cli, cwd, |bag| ops::block(bag, id, me, by))?;
            let root = locate(cli, cwd)?;
            if let Some(bag) = load(&root).taskbag {
                if let Some(task) = bag.tasks.iter().find(|t| &t.id == id) {
                    for line in stop_lines(&bag, task) {
                        done.text.push_str(&format!("  {line}\n"));
                    }
                    done.text.push_str("  Its state stays as it is; while it is blocked, `alm next` does not list it.\n");
                    done.fields["task"] = task_json_in(&bag, task);
                }
            }
            Ok(done)
        }
        Command::Unblock { id, by } => {
            let mut done = change(cli, cwd, |bag| ops::unblock(bag, id, me, by))?;
            let root = locate(cli, cwd)?;
            if let Some(bag) = load(&root).taskbag {
                if let Some(task) = bag.tasks.iter().find(|t| &t.id == id) {
                    for line in stop_lines(&bag, task) {
                        done.text.push_str(&format!("  {line}\n"));
                    }
                    if task.blocked_by.is_empty() {
                        done.text
                            .push_str("  It has no blockers now; its state stays as it is.\n");
                    }
                    done.fields["task"] = task_json_in(&bag, task);
                }
            }
            Ok(done)
        }
        Command::Done {
            id,
            without_human_check,
            steps,
            accepted_by,
        } => {
            let steps = match steps {
                Some(file) => {
                    let path = cwd.join(file);
                    Some(
                        std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
                            detail: format!("{}: {e}", path.display()),
                        })?,
                    )
                }
                None => None,
            };
            // Decision 0027 § 5 (Task-A01-stable046): with the project's limit on, a task
            // recorded as high risk is not finished without the human check.
            if without_human_check.is_some() {
                crate::risk_rules::refuse_without_human_check(&locate(cli, cwd)?, id)?;
            }
            let mut recorded_exception = None;
            change(cli, cwd, |bag| {
                // Decision 0020: finishing without --without-human-check needs a "by human" label
                // (checked after the usual rules, so a held task still says who holds it).
                let was_done = ops::find_task(bag, id)?.state == State::Done;
                let out = ops::finish(
                    bag,
                    id,
                    me,
                    without_human_check.as_deref(),
                    steps.as_deref(),
                )?;
                if without_human_check.is_none() && !was_done && !out.task.human_checked() {
                    return Err(Problem::HumanCheckLabelMissing { id: out.task.id });
                }
                let mut out = out;
                // Decision 0021 § 1.1 (Task-A01-seed343): finishing without the human check is an
                // exception, with why and who accepted the risk.
                if let Some(why) = without_human_check.as_deref() {
                    let (change, ex) = allomos_core::exception::new(
                        &allomos_store::exceptions::new_id(),
                        &allomos_core::exception::Given {
                            kind: Some(allomos_core::exception::WITHOUT_HUMAN_CHECK),
                            why: Some(why),
                            accepted_by: accepted_by.as_deref(),
                            ..Default::default()
                        },
                    )?;
                    out.recorded.push(change);
                    recorded_exception = Some(ex);
                }
                Ok(out)
            })
            .map(|mut done| {
                if let Some(ex) = &recorded_exception {
                    done.text
                        .push_str(&format!("Exception {} recorded:\n", ex.id));
                    done.text.push_str(&exceptions::exception_lines(ex, "  "));
                    done.text.push_str(&exceptions::facts(id, ex));
                    done.fields["exception"] = json!(ex);
                }
                // Decision 0027 § 3 (Task-A01-steam844): the rules to judge the risk by, and one
                // sentence with the command when none is recorded. alm tells; it refuses nothing.
                if let Ok(root) = locate(cli, cwd) {
                    let (language, applied) = crate::risk_rules::of_taskbag(&root);
                    let risk = allomos_store::classify::by_task(&root)
                        .remove(id.as_str())
                        .and_then(|c| c.risk);
                    if risk.is_none() {
                        done.after
                            .push_str(&crate::risk_rules::not_classified(&language, id));
                    }
                    done.after
                        .push_str(&crate::risk_rules::text(&language, &applied));
                    done.fields["risk_rules"] = crate::risk_rules::json(&applied);
                    done.fields["risk_recorded"] = json!(risk.is_some());
                }
                done
            })
        }
        Command::Checked {
            id,
            deployed,
            locally,
            result,
            words,
            checker,
        } => {
            let given = allomos_core::labels::CheckGiven {
                checker: checker.as_deref(),
                result: if result == "partly" {
                    allomos_core::labels::CheckResult::Partly
                } else {
                    allomos_core::labels::CheckResult::Pass
                },
                words: words.as_deref(),
            };
            let which = if *deployed {
                Some(allomos_core::labels::DEPLOYED_BY_HUMAN)
            } else if *locally {
                Some(allomos_core::labels::LOCALLY_BY_HUMAN)
            } else {
                None
            };
            change(cli, cwd, |bag| {
                allomos_core::labels::checked(bag, id, which, &given)
            })
        }
        Command::Recheck { id, reason, steps } => {
            let steps = match steps {
                Some(file) => {
                    let path = cwd.join(file);
                    Some(
                        std::fs::read_to_string(&path).map_err(|e| Problem::ReadFailed {
                            detail: format!("{}: {e}", path.display()),
                        })?,
                    )
                }
                None => None,
            };
            change(cli, cwd, |bag| {
                ops::recheck(bag, id, reason, steps.as_deref())
            })
        }
        Command::AiDecided { id, text, kind } => {
            let root = locate(cli, cwd)?;
            let kind = kind
                .as_deref()
                .and_then(allomos_core::decision::DecisionKind::parse);
            let d = allomos_store::record_ai_decision(
                &root,
                Actor::alm(cli.holder.as_deref()),
                lock_wait(),
                id,
                text,
                kind,
            )?;
            let mut out = format!("{}\n", task_line(&d.task));
            for x in &d.decisions {
                out.push_str(&format!("  AI decided: {}\n", decision_text(x)));
            }
            let to_review = allomos_core::decision::to_review(&d.pending);
            out.push_str(&format!(
                "{} AI decision(s) to review; `alm human` lists them for the person.\n",
                to_review.len()
            ));
            match kind {
                Some(allomos_core::decision::DecisionKind::Detail) => out.push_str(
                    "A detail: kept with the task (`alm show`), not listed for review.\n",
                ),
                Some(_) => {}
                None => out.push_str(
                    "Fact: this decision has no kind, so it waits for review. Next time, say which: `--kind visible` (what a user sees or runs into), `--kind principle` (how it is built and why) or `--kind detail` (sizes, limits, parameters, names in the code; not reviewed).\n",
                ),
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&d.task), "decision": d.decisions.first(), "ai_decisions": to_review, "ai_details": d.pending.iter().filter(|x| !x.to_review()).collect::<Vec<_>>() }),
                out,
            );
            done.issues = d.issues;
            Ok(done)
        }
        Command::Approve {
            ids,
            words,
            channel,
            plan,
            expires,
            approver,
        } => approve::approve(
            cli,
            cwd,
            &approve::Args {
                ids,
                words: words.as_deref(),
                channel,
                plan: plan.as_deref(),
                expires: expires.as_deref(),
                approver: approver.as_deref(),
            },
        ),
        Command::Withdraw { id } => approve::withdraw(cli, cwd, id),
        Command::Effect {
            id,
            of,
            target,
            action,
            rollback,
            result,
        } => effects::effect(
            cli,
            cwd,
            &effects::Args {
                id,
                of: of.as_deref(),
                target: target.as_deref(),
                action: action.as_deref(),
                rollback: rollback.as_deref(),
                result: result.as_deref(),
            },
        ),
        Command::Effects { id } => effects::list(cli, cwd, id.as_deref()),
        Command::Exception {
            id,
            of,
            kind,
            check,
            accepted_by,
        } => exceptions::exception(
            cli,
            cwd,
            id,
            &allomos_core::exception::Given {
                of: of.as_deref(),
                kind: kind.as_deref(),
                why: cli.why.as_deref(),
                accepted_by: accepted_by.as_deref(),
                check: check.as_deref(),
            },
        ),
        Command::Exceptions { id } => exceptions::list(cli, cwd, id.as_deref()),
        Command::Redact {
            id,
            fields,
            events,
            reason,
            approved_by,
        } => redact::redact(
            cli,
            cwd,
            id,
            &allomos_store::redact::Request {
                fields,
                events,
                reason: reason.as_deref().or(cli.why.as_deref()),
                approved_by: approved_by.as_deref(),
            },
        ),
        Command::Classify {
            id,
            change_type,
            risk,
            touches,
            due,
        } => classify::classify(
            cli,
            cwd,
            id,
            &allomos_core::classify::Given {
                change_type: change_type.as_deref(),
                risk: risk.as_deref(),
                touches: (!touches.is_empty()).then_some(&touches[..]),
                due: due.as_deref(),
            },
        ),
        Command::Link {
            id,
            kind,
            reference,
            url,
            remove,
        } => links::link(cli, cwd, id, kind, reference, url.as_deref(), *remove),
        Command::CheckResult {
            id,
            name,
            result,
            detail,
        } => check_results::record(cli, cwd, id, name, result, detail.as_deref()),
        Command::Links { reference } => links::list(cli, cwd, reference.as_deref()),
        Command::Reviewed { id } => {
            let root = locate(cli, cwd)?;
            let d = allomos_store::review_ai_decisions(
                &root,
                Actor::alm(cli.holder.as_deref()),
                lock_wait(),
                id,
            )?;
            let mut out = format!("{}\n", task_line(&d.task));
            out.push_str(&format!("Reviewed {} AI decision(s):\n", d.decisions.len()));
            for x in &d.decisions {
                out.push_str(&format!("  {}\n", x.text));
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "task": task_json(&d.task), "reviewed": d.decisions }),
                out,
            );
            done.issues = d.issues;
            Ok(done)
        }
        Command::Human {
            iteration,
            area,
            steps,
        } => {
            let root = locate(cli, cwd)?;
            let (bag, issues) = taskbag_of(&root, load(&root))?;
            human::human(&root, &bag, iteration.as_deref(), area.as_deref(), *steps).map(
                |mut done| {
                    done.issues = issues;
                    done
                },
            )
        }
        Command::Title {
            id,
            title,
            expected_version,
        } => change(cli, cwd, |bag| {
            ops::check_version(bag, id, expected_version.as_deref())?;
            ops::set_title(bag, id, me, title)
        }),
        Command::Describe {
            id,
            text,
            from,
            expected_version,
        } => {
            // `--text` and `--from` exclude each other, and one of them is required.
            let text = match from {
                Some(file) => {
                    let file = cwd.join(file);
                    read_text(&file).map_err(|e| Problem::ReadFailed {
                        detail: format!("{}: {e}", file.display()),
                    })?
                }
                None => text.clone().unwrap_or_default(),
            };
            change(cli, cwd, |bag| {
                ops::check_version(bag, id, expected_version.as_deref())?;
                ops::set_description(bag, id, me, &text)
            })
        }
        Command::Cards { what } => {
            let root = locate(cli, cwd)?;
            match what {
                None => {
                    let (_, issues) = taskbag_of(&root, load(&root))?;
                    let outside = allomos_store::cards::outside(&root);
                    let mut done = Done::new(
                        json!({ "taskbag": root, "cards_outside": outside }),
                        cards_text(&outside),
                    );
                    done.issues = issues;
                    Ok(done)
                }
                Some(CardsWhat::Found { items }) => {
                    let items = items
                        .iter()
                        .map(|item| match item.split_once('=') {
                            Some((id, path)) => Ok((id.trim().to_string(), path.to_string())),
                            None => Err(Problem::BadUsage {
                                detail: format!(
                                    "\"{item}\" is not <id>=<path>, for example A01-01=docs/tasks/A01-01.md."
                                ),
                            }),
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    let r =
                        allomos_store::cards::report(&root, Actor::alm(me), lock_wait(), &items)?;
                    let mut text = String::new();
                    for o in &r.found {
                        text.push_str(&format!("Reported: {}  {}\n", o.task, o.path));
                    }
                    for id in &r.withdrawn {
                        text.push_str(&format!("Withdrawn: {id}\n"));
                    }
                    if r.found.is_empty() && r.withdrawn.is_empty() {
                        text.push_str("Nothing new to report.\n");
                    }
                    text.push_str(&cards_text(&r.outside));
                    let mut done = Done::new(
                        json!({
                            "taskbag": root,
                            "found": r.found,
                            "withdrawn": r.withdrawn,
                            "cards_outside": r.outside,
                        }),
                        text,
                    );
                    done.issues = r.issues;
                    Ok(done)
                }
            }
        }
        Command::Card { id, from, replace } => {
            let root = locate(cli, cwd)?;
            let from = from.as_ref().map(|f| cwd.join(f));
            let m = allomos_store::cards::move_in(
                &root,
                Actor::alm(me),
                lock_wait(),
                id,
                from.as_deref(),
                *replace,
            )?;
            let mut text = format!("{}\n", task_line(&m.task));
            text.push_str(&format!(
                "  card file: {} (from {})\n  The description's Card: link points to it.\n",
                m.card,
                plain_path(&m.source).display()
            ));
            if let Some(path) = &m.reported {
                let folder = quoted(&plain_path(&m.project).display().to_string());
                // For Codex without `-C` (Task-A01-flame635): the folder is named in words.
                let example = if for_codex(cli, cwd) {
                    format!("in {folder}: git rm -- {}", quoted(path))
                } else {
                    format!("git -C {folder} rm -- {}", quoted(path))
                };
                text.push_str(&format!(
                    "alm removes nothing outside the taskbag. When your confirmed plan says so, remove the original from the project folder, for example: {example}\n"
                ));
            } else if let Some(original) = &m.original {
                // Linked from the description, not reported (Task-A01-51).
                text.push_str(&format!(
                    "alm removes nothing outside the taskbag. When your confirmed plan says so, remove the original, for example: git rm -- {}\n",
                    quoted(&plain_path(original).display().to_string())
                ));
            }
            let mut done = Done::new(
                json!({
                    "taskbag": root,
                    "task": task_json(&m.task),
                    "card": m.card,
                    "source": m.source,
                    "reported": m.reported,
                    "original": m.original,
                }),
                text,
            );
            done.issues = m.issues;
            Ok(done)
        }
        Command::Beat { id, busy, doing } => {
            heartbeat::beat(cli, cwd, id.as_deref(), busy.as_deref(), doing.as_deref())
        }
        Command::Note { id, text } => change(cli, cwd, |bag| {
            ops::set_handoff_note(bag, id, me, text.as_deref())
        }),
        Command::Folders { id, names, .. } => change(cli, cwd, |bag| {
            allomos_core::folders::set(bag, id, me, names)
        }),
        Command::Difficulty { id, level, .. } => change(cli, cwd, |bag| {
            allomos_core::difficulty::set(bag, id, me, *level)
        }),
        Command::Upgrade { confirm } => folders::upgrade(&locate(cli, cwd)?, me, *confirm),
        Command::Audit => trail::audit(&locate(cli, cwd)?),
        Command::Checkpoint { git_commit } => {
            trail::checkpoint(&locate(cli, cwd)?, git_commit.as_deref())
        }
        Command::Folder { what } => {
            folders::folder(&locate(cli, cwd)?, me, what, cwd, for_codex(cli, cwd))
        }
        Command::Project { name, id } => {
            let root = locate(cli, cwd)?;
            let lock = Lock::acquire(&root, lock_wait())?;
            let (bag, issues) = taskbag_of(&root, lock.load())?;
            let project = ops::rename_project(&bag, name.as_deref(), id.as_deref())?;
            if name.is_some() || id.is_some() {
                save_project(&lock, me, "project", &bag.project, &project)?;
            }
            let mut done = Done::new(
                json!({ "taskbag": root, "project": { "id": project.id, "name": project.name } }),
                format!("Project: {} ({})\n", project.name, project.id),
            );
            done.issues = issues;
            Ok(done)
        }
        Command::Needs { id, text } => {
            change(cli, cwd, |bag| ops::set_next(bag, id, me, text.as_deref()))
        }
        Command::Plan { id, iteration } => {
            change(cli, cwd, |bag| ops::plan(bag, id, iteration.as_deref()))
        }
        Command::Ready { id } => change(cli, cwd, |bag| ops::make_ready(bag, id, me)),
        Command::Backlog { id } => change(cli, cwd, |bag| ops::back_to_backlog(bag, id, me)),
        Command::Drop { id } => change(cli, cwd, |bag| ops::drop_task(bag, id, me)),
        Command::Reopen { id, note } => change(cli, cwd, |bag| ops::reopen(bag, id, note)),
        Command::Release { id, note } => {
            change(cli, cwd, |bag| ops::release(bag, id, me, note.as_deref()))
        }
        _ => unreachable!("lib.rs sends only this module's commands here"),
    }
}

/// What `alm cards` says: the card files agents reported as still in the project folder, and what
/// to do about them (Task-A00-30).
pub(crate) fn cards_text(outside: &[allomos_core::card::Outside]) -> String {
    if outside.is_empty() {
        return "No task's Card: link points outside the taskbag, and no card files are reported as \
kept in the project folder. If you find others, report them with `alm cards found <id>=<path>` \
(`alm guide move-cards`).\n"
            .to_string();
    }
    let mut text = format!(
        "{} task(s) keep their card files in the project folder:\n",
        outside.len()
    );
    for o in outside {
        let how = match o.source {
            allomos_core::card::Source::Link => "its Card: link".to_string(),
            allomos_core::card::Source::Report => format!(
                "reported by {}, {}",
                o.by.as_deref().unwrap_or("?"),
                o.at.as_deref().unwrap_or("?")
            ),
        };
        text.push_str(&format!("  {}  {}  ({how})\n", o.task, o.path));
    }
    text.push_str(
        "Propose moving them in beside their tasks, and do it only after the person's Go ahead: `alm guide move-cards`.\n",
    );
    text
}
