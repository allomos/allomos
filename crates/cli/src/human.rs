//! `alm human` (Task-A03-18; the order: Task-A01-muffin353, decision 0028): what needs a person,
//! one task at a time, in the order to take them. The order is the core's
//! ([`allomos_core::queue`]), the same one the app lists; `alm` only prints it.

use std::path::Path;

use allomos_core::heartbeat::parse_utc;
use allomos_core::queue::{Row, Todo};
use allomos_core::{Problem, Task, Taskbag};
use allomos_store::{pick_ui_language, task_language};
use serde_json::{Value, json};

use crate::{
    Done, classify, decision_lines, human_task_line, indent, stop_lines, task_json_in,
};

/// "check + review 2 AI decisions": what the person has to do about one task, in the task
/// language like the state and the labels of its line.
pub(crate) fn todo_line(row: &Row, language: &str) -> String {
    let n = row.decisions;
    let parts: Vec<String> = row
        .todo
        .iter()
        .map(|d| match (d, language) {
            (Todo::Answer, "zh-CN") => "回答".to_string(),
            (Todo::Answer, "ja") => "回答".to_string(),
            (Todo::Answer, _) => "answer".to_string(),
            (Todo::Approve, "zh-CN") => "批准".to_string(),
            (Todo::Approve, "ja") => "承認".to_string(),
            (Todo::Approve, _) => "approve".to_string(),
            (Todo::ApproveAfterwards, "zh-CN") => "事后批准".to_string(),
            (Todo::ApproveAfterwards, "ja") => "事後承認".to_string(),
            (Todo::ApproveAfterwards, _) => "approve afterwards".to_string(),
            (Todo::Check, "zh-CN") => "检查".to_string(),
            (Todo::Check, "ja") => "確認".to_string(),
            (Todo::Check, _) => "check".to_string(),
            (Todo::Review, "zh-CN") if n == 0 => "审 AI 决定".to_string(),
            (Todo::Review, "zh-CN") => format!("审 {n} 条 AI 决定"),
            (Todo::Review, "ja") if n == 0 => "AI の決定のレビュー".to_string(),
            (Todo::Review, "ja") => format!("AI の決定 {n} 件のレビュー"),
            (Todo::Review, _) if n == 0 => "review AI decisions".to_string(),
            (Todo::Review, _) if n == 1 => "review 1 AI decision".to_string(),
            (Todo::Review, _) => format!("review {n} AI decisions"),
        })
        .collect();
    let head = match language {
        "zh-CN" => "要你做：",
        "ja" => "あなたの作業：",
        _ => "to do: ",
    };
    format!("{head}{}", parts.join(" + "))
}

/// "waiting since 2026-11-15T12:33:57Z (3 d); holds back 2 tasks", or that the history does not
/// say since when.
fn waiting_line(row: &Row, now: Option<i64>) -> String {
    let mut line = match &row.waiting_since {
        Some(at) => match (parse_utc(at), now) {
            (Some(then), Some(now)) => format!("waiting since {at} ({})", how_long(now - then)),
            _ => format!("waiting since {at}"),
        },
        None => "waiting since: not in the history".to_string(),
    };
    match row.blocks {
        0 => {}
        1 => line.push_str("; holds back 1 task"),
        n => line.push_str(&format!("; holds back {n} tasks")),
    }
    line
}

/// "40 min", "5 h", "3 d": how long, rounded down.
fn how_long(secs: i64) -> String {
    let secs = secs.max(0);
    if secs < 3_600 {
        format!("{} min", secs / 60)
    } else if secs < 48 * 3_600 {
        format!("{} h", secs / 3_600)
    } else {
        format!("{} d", secs / 86_400)
    }
}

pub(crate) fn human(
    root: &Path,
    bag: &Taskbag,
    iteration: Option<&str>,
    area: Option<&str>,
    steps: bool,
) -> Result<Done, Problem> {
    if let Some(a) = area {
        if !bag.areas.iter().any(|x| x.id == a) {
            return Err(Problem::UnknownArea { area: a.into() });
        }
    }
    if let Some(i) = iteration {
        if !bag.iterations.iter().any(|x| x.id == i) {
            return Err(Problem::UnknownIteration {
                iteration: i.into(),
            });
        }
    }
    let tasks: Vec<&Task> = bag
        .tasks
        .iter()
        .filter(|t| {
            area.is_none_or(|a| t.area == a)
                && iteration.is_none_or(|i| t.iteration.as_deref() == Some(i))
        })
        .collect();
    // The history, read once: risks (Task-A01-garden385, decision 0027), AI decisions to review
    // (Task-A03-19), emergencies not approved afterwards (Task-A01-palm011) and since when each
    // task waits. A plan waiting for a Go ahead (`alm confirm`) is not in the history: the app
    // adds those to the same queue.
    let read = allomos_store::queue::read(root, bag);
    let rows = read.rows(bag, &tasks, &[]);
    let system: Vec<String> = sys_locale::get_locales().collect();
    let language = pick_ui_language(&[task_language(&bag.project, &system).language]);
    let now = parse_utc(&allomos_store::events::now_utc());
    let task_of = |row: &Row| tasks.iter().copied().find(|t| t.id == row.task);
    let none = Vec::new();
    let waiting = rows.iter().filter(|r| r.agent_waiting).count();
    let mut text = String::new();
    let (mut any_check, mut any_review, mut any_emergency) = (false, false, false);
    for (group, count) in [(true, waiting), (false, rows.len() - waiting)] {
        text.push_str(&if group {
            format!("Agents are waiting for you ({count})\n")
        } else {
            format!("\nFinished, for you to look at ({count})\n")
        });
        if count == 0 {
            text.push_str(if group {
                "  No agent is waiting for you.\n"
            } else {
                "  Nothing to check or review.\n"
            });
        }
        for row in rows.iter().filter(|r| r.agent_waiting == group) {
            let Some(t) = task_of(row) else { continue };
            text.push_str(&format!("  {}\n", human_task_line(t, language)));
            text.push_str(&format!("      {}\n", todo_line(row, language)));
            text.push_str(&format!(
                "      {}\n",
                classify::human_risk_line(row.risk.as_deref(), language)
            ));
            text.push_str(&format!("      {}\n", waiting_line(row, now)));
            for todo in &row.todo {
                match todo {
                    Todo::Answer => {
                        for line in stop_lines(bag, t) {
                            text.push_str(&format!("      {line}\n"));
                        }
                    }
                    // Not listed by `alm` (see above); the app shows the plan itself.
                    Todo::Approve => {}
                    Todo::ApproveAfterwards => {
                        any_emergency = true;
                        if let Some(u) = read.unapproved.iter().find(|u| u.task == t.id) {
                            text.push_str(&format!(
                                "      To approve afterwards: emergency since {}; approval due by {}{}\n",
                                u.since,
                                u.due.as_deref().unwrap_or("no deadline recorded"),
                                if u.overdue { " (past due)" } else { "" }
                            ));
                        }
                    }
                    Todo::Check => {
                        any_check = true;
                        let why = t.without_human_check.as_deref().unwrap_or(
                            "no reason recorded (finished before alm done --without-human-check)",
                        );
                        text.push_str(&format!("      not human-checked: {why}\n"));
                        if steps {
                            match &t.human_check_steps {
                                Some(s) => {
                                    text.push_str("      steps:\n");
                                    for line in indent(s).lines() {
                                        text.push_str(&format!("      {line}\n"));
                                    }
                                }
                                None => text.push_str("      steps: none written\n"),
                            }
                        }
                    }
                    Todo::Review => {
                        any_review = true;
                        let d = read.decisions.get(&t.id).unwrap_or(&none);
                        text.push_str(&format!("      To review: AI decisions ({}):\n", d.len()));
                        for line in decision_lines(d).lines() {
                            text.push_str(&format!("      {line}\n"));
                        }
                    }
                }
            }
        }
    }
    text.push('\n');
    if any_emergency {
        text.push_str(
            "Once the approver approves an emergency change: `alm approve <id> --words \"<their words>\"`.\n",
        );
    }
    if any_check {
        text.push_str("Once a person has checked a task: `alm checked <id>`.\n");
    }
    if any_review {
        text.push_str("Once a person has reviewed a task's AI decisions: `alm reviewed <id>`.\n");
    }
    if !rows.is_empty() {
        text.push_str(
            "The order (decision 0028): what an agent waits on first (the task holding back more tasks, then the longer wait); then finished work by the recorded risk (high, medium, low, not classified), the longer wait first; last, the earlier iteration.\n",
        );
    }
    // `--json`: `queue` is the list above, one task each, in order. The lists by kind stay (a
    // task may be in several of them), each in the queue's order too.
    let with_risk = |t: &Task| {
        let mut v = task_json_in(bag, t);
        v["risk"] = json!(read.classes.get(&t.id).and_then(|c| c.risk.as_deref()));
        v
    };
    let of_kind = |kind: Todo| -> Vec<Value> {
        rows.iter()
            .filter(|r| r.todo.contains(&kind))
            .filter_map(|r| task_of(r))
            .map(|t| {
                let mut v = with_risk(t);
                if kind == Todo::Review {
                    v["ai_decisions"] = json!(read.decisions.get(&t.id).unwrap_or(&none));
                }
                v
            })
            .collect()
    };
    let queue: Vec<Value> = rows
        .iter()
        .filter_map(|r| task_of(r).map(|t| (r, t)))
        .map(|(r, t)| {
            let mut v = with_risk(t);
            v["todo"] = json!(r.todo);
            v["agent_waiting"] = json!(r.agent_waiting);
            v["waiting_since"] = json!(r.waiting_since);
            v["blocks"] = json!(r.blocks);
            v["ai_decisions"] = json!(read.decisions.get(&t.id).unwrap_or(&none));
            v
        })
        .collect();
    let emergencies: Vec<&allomos_core::classify::Unapproved> = rows
        .iter()
        .filter_map(|r| read.unapproved.iter().find(|u| u.task == r.task))
        .collect();
    Ok(Done::new(
        json!({
            "taskbag": root,
            "queue": queue,
            "to_check": of_kind(Todo::Check),
            "to_answer": of_kind(Todo::Answer),
            "to_review": of_kind(Todo::Review),
            "to_approve_afterwards": emergencies,
        }),
        text,
    ))
}
