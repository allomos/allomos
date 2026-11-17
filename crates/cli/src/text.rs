//! How tasks, iterations, decisions, notes and events are written out, in lines for people and
//! in JSON for agents; shared by the commands.

use allomos_core::decision::Decision;
use allomos_core::ops;
use allomos_core::{Iteration, State, Task, Taskbag};
use allomos_store::handoff;
use allomos_store::plain_path;
use serde_json::{Value, json};

pub(crate) fn task_json(t: &Task) -> Value {
    let mut v = serde_json::to_value(t).expect("tasks serialize");
    // The version an agent passes back with `--expected-version` (Task-A01-57).
    v["version"] = json!(ops::task_version(t));
    // The level it counts as, and whether that is the default 3 (Task-A01-actor813). The file
    // keeps no difficulty when none was given.
    let (level, default) = allomos_core::difficulty::of(t);
    v["difficulty"] = json!(level);
    v["difficulty_default"] = json!(default);
    v
}

/// A task for `--json` reading commands: whether it is blocked (worked out, decision 0026), and
/// each blocker's current state when it has blockers.
pub(crate) fn task_json_in(bag: &Taskbag, t: &Task) -> Value {
    let mut v = task_json(t);
    v["blocked"] = json!(ops::is_blocked(bag, t));
    if !t.blocked_by.is_empty() {
        v["blockers"] = json!(ops::blockers(bag, t));
    }
    v
}

/// "blocked by A01-17 (in progress), A01-20 (backlog)": the blockers that have not finished;
/// none when the task is not blocked (decision 0026).
pub(crate) fn blocked_text(bag: &Taskbag, t: &Task) -> Option<String> {
    let open: Vec<String> = ops::open_blockers(bag, t)
        .iter()
        .map(ops::Blocker::describe)
        .collect();
    (!open.is_empty()).then(|| format!("blocked by {}", open.join(", ")))
}

/// "blockers finished: A01-17 (done)": the task has blockers and none is open any more. None for
/// a finished task.
pub(crate) fn finished_blockers_text(bag: &Taskbag, t: &Task) -> Option<String> {
    if matches!(t.state, State::Done | State::Dropped) || ops::is_blocked(bag, t) {
        return None;
    }
    let done: Vec<String> = ops::finished_blockers(bag, t)
        .iter()
        .map(ops::Blocker::describe)
        .collect();
    (!done.is_empty()).then(|| format!("blockers finished: {}", done.join(", ")))
}

/// What holds a task back, one line each: "waiting for you: approve the card", "waiting for
/// Ann: …", "blocked by A01-17 (in progress)" (a task can be both, decision 0026); or that its
/// blockers have finished.
pub(crate) fn stop_lines(bag: &Taskbag, t: &Task) -> Vec<String> {
    let mut lines = Vec::new();
    if t.state == State::Waiting {
        lines.push(format!(
            "waiting for {}: {}",
            t.who.as_deref().unwrap_or("the approver"),
            t.waiting_on.as_deref().unwrap_or("-")
        ));
    }
    lines.extend(blocked_text(bag, t));
    lines.extend(finished_blockers_text(bag, t));
    lines
}

/// An iteration's heading: ID and name, then its release and dates when it has them.
pub(crate) fn iteration_heading(it: &Iteration) -> String {
    let mut s = format!("{}  {}", it.id, it.name);
    let dates = match (&it.start, &it.end) {
        (Some(a), Some(b)) => Some(format!("{a} to {b}")),
        (Some(a), None) => Some(format!("from {a}")),
        (None, Some(b)) => Some(format!("until {b}")),
        (None, None) => None,
    };
    let extra: Vec<String> = it.release.iter().cloned().chain(dates).collect();
    if !extra.is_empty() {
        s.push_str(&format!("  ({})", extra.join(", ")));
    }
    // Its state, unless planned (Task-A01-root343).
    if !it.state.is_planned() {
        s.push_str(&format!("  [{}]", it.state.as_str()));
    }
    s
}

/// The line `alm handoff` adds: keep the project notes true too (Task-A03-20).
pub(crate) fn notes_reminder(n: &handoff::ProjectNotes) -> String {
    let when = if n.found {
        format!("last changed {}", n.changed.as_deref().unwrap_or("unknown"))
    } else {
        "this taskbag has none yet".to_string()
    };
    format!(
        "If what is running or a pitfall changed, update `{}` too ({when}): {}\n",
        n.file,
        plain_path(&n.path).display()
    )
}

pub(crate) fn rules_line(r: &allomos_core::AppliedRules) -> String {
    let whose = r
        .override_for
        .as_ref()
        .map(|n| format!(" (override for {n})"))
        .unwrap_or_default();
    format!(
        "Handoff rules{whose}, while you hold a task (from `alm claim` until `alm done` or `alm release`): with {}% of the 5-hour quota left, start nothing new, write your handoff into the task (`alm handoff`), release it (`alm release <id> --note \"<what the next one needs, and the working branch to continue on>\"`) and stop; at {}% context used, finish the current step, write your handoff, commit, release the task the same way, and give the person the exact prompt for a new session. A released task is ready again, so the next session or loop run picks it up and continues on that branch (`alm claim <id> --branch <that branch>`). While you hold no task, these gates do not apply: there is nothing to hand off. Also write a handoff {}. Report your quota each time you read it: `alm quota --agent <your agent's id> --left <percent> [--resets <when>]` (the Allomos app shows it); `alm claim`, `alm label`, `alm release`, `alm done` and `alm handoff` ask for it when your last report is missing or {} minutes old or more.\n",
        r.quota_gate,
        r.context_gate,
        r.write_when,
        allomos_store::QUOTA_ASK_AFTER_MINUTES
    )
}

/// Text indented by four spaces, ending with a new line.
pub(crate) fn indent(text: &str) -> String {
    text.trim_end()
        .lines()
        .map(|l| {
            if l.is_empty() {
                "\n".to_string()
            } else {
                format!("    {l}\n")
            }
        })
        .collect()
}

/// A task's AI decisions to review, one per line: "  - <text> (<by>, <at>)" (Task-A03-19).
pub(crate) fn decision_lines(decisions: &[Decision]) -> String {
    decisions
        .iter()
        .map(|d| match &d.by {
            Some(by) => format!("  - {} ({by}, {})\n", decision_text(d), d.at),
            None => format!("  - {} ({})\n", decision_text(d), d.at),
        })
        .collect()
}

/// A decision's text, after its kind in brackets when it has one (decision 0025 § 3).
pub(crate) fn decision_text(d: &Decision) -> String {
    match d.kind {
        Some(kind) => format!("[{}] {}", kind.as_str(), d.text),
        None => d.text.clone(),
    }
}

/// A task's traces in one line (Task-A01-fennel617): "R77, 0021", or "none".
pub(crate) fn traces_text(traces: &[String]) -> String {
    if traces.is_empty() {
        "none".to_string()
    } else {
        traces.join(", ")
    }
}

/// `--difficulty` and `alm difficulty`'s level, checked by the core rule (1 to 5).
pub(crate) fn difficulty_level(given: &str) -> Result<u8, String> {
    allomos_core::difficulty::level(given).map_err(|p| p.english())
}

/// `--difficulty <range>` for `alm next` and `alm list`.
pub(crate) fn difficulty_range(given: &str) -> Result<allomos_core::difficulty::Range, String> {
    allomos_core::difficulty::Range::parse(given).map_err(|p| p.english())
}

pub(crate) fn state_text(state: State, language: &str) -> &'static str {
    match (state, language) {
        (State::Backlog, "zh-CN") => "储备",
        (State::Backlog, "ja") => "バックログ",
        (State::Backlog, _) => "Backlog",
        (State::Ready, "zh-CN") => "就绪",
        (State::Ready, "ja") => "準備完了",
        (State::Ready, _) => "Ready",
        (State::InProgress, "zh-CN") => "进行中",
        (State::InProgress, "ja") => "進行中",
        (State::InProgress, _) => "In progress",
        (State::Waiting, "zh-CN") => "等待中",
        (State::Waiting, "ja") => "待機中",
        (State::Waiting, _) => "Waiting",
        (State::Done, "zh-CN") => "已完成",
        (State::Done, "ja") => "完了",
        (State::Done, _) => "Done",
        (State::Dropped, "zh-CN") => "已放弃",
        (State::Dropped, "ja") => "中止",
        (State::Dropped, _) => "Dropped",
        (State::Unknown, _) => state.as_str(),
    }
}

pub(crate) fn label_text<'a>(label: &'a str, language: &str) -> &'a str {
    use allomos_core::labels::{
        AI_DECISIONS, DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN, LOCALLY_BY_AGENT, LOCALLY_BY_HUMAN,
    };
    match (label, language) {
        (LOCALLY_BY_AGENT, "zh-CN") => "智能体已在本地验证",
        (LOCALLY_BY_AGENT, "ja") => "エージェントがローカルで検証済み",
        (DEPLOYED_BY_AGENT, "zh-CN") => "智能体已部署并验证",
        (DEPLOYED_BY_AGENT, "ja") => "エージェントがデプロイして検証済み",
        (LOCALLY_BY_HUMAN, "zh-CN") => "人已在本地验证",
        (LOCALLY_BY_HUMAN, "ja") => "人がローカルで検証済み",
        (DEPLOYED_BY_HUMAN, "zh-CN") => "人已在部署后验证",
        (DEPLOYED_BY_HUMAN, "ja") => "人がデプロイ後に検証済み",
        (AI_DECISIONS, "zh-CN") => "待审阅 AI 决定",
        (AI_DECISIONS, "ja") => "レビュー待ちの AI の決定",
        _ => label,
    }
}

pub(crate) fn human_task_line(t: &Task, language: &str) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        t.id,
        state_text(t.state, language),
        if t.labels.is_empty() {
            "-".to_string()
        } else {
            t.labels
                .iter()
                .map(|l| label_text(l, language))
                .collect::<Vec<_>>()
                .join(", ")
        },
        t.holder.as_deref().unwrap_or("-"),
        t.title
    )
}

pub(crate) fn task_line(t: &Task) -> String {
    format!(
        "{}  {}  {}  {}  {}",
        t.id,
        t.state.as_str(),
        if t.labels.is_empty() {
            "-".to_string()
        } else {
            t.labels.join(", ")
        },
        t.holder.as_deref().unwrap_or("-"),
        t.title
    )
}
