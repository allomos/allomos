//! The Evidence screen of a task (decision 0029 points 1, 2 and 4; Task-A01-herb903,
//! Task-A01-inlet632): which parts it has and in which order, worked out from the records the task
//! already has. The app draws the parts the task has and leaves the others out; `alm show <id>
//! --evidence` lists every part in the same order and says "none" for one the task lacks. Both
//! take the order from here, so they never differ. Nothing is judged and nothing refused: a
//! missing part is only said to be missing.

use serde::Serialize;

use crate::Task;
use crate::check_result::{self, Recorded as CheckResult};
use crate::decision::{Decision, DecisionKind};
use crate::event::{Event, Kind, REASON};
use crate::labels::AI_DECISIONS;
use crate::link::Recorded;

/// One part of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Part {
    /// What the person is asked to do, and the risk with the reason the agent gave.
    AskRisk,
    /// The human check steps, and why the task was finished without the check.
    CheckSteps,
    /// The handoff note's first sentence and the commits and merges recorded.
    Summary,
    /// The test results the agent recorded with `alm check-result` (decision 0029 point 5);
    /// a task with none points to the description's Evidence section.
    Tests,
    /// The AI decisions not reviewed yet (to review, and details to read).
    Decisions,
    /// The whole handoff note, folded in the app.
    Handoff,
}

/// Every part, top to bottom (decision 0029 § 2).
pub const ORDER: [Part; 6] = [
    Part::AskRisk,
    Part::CheckSteps,
    Part::Summary,
    Part::Tests,
    Part::Decisions,
    Part::Handoff,
];

/// What the screen shows of one task, as the app receives it with the taskbag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Evidence {
    /// The parts the task has, in [`ORDER`].
    pub parts: Vec<Part>,
    /// The handoff note's first sentence ([`summary_sentence`]); empty when it has none.
    pub summary: String,
    /// The reason given with the newest risk recorded ([`risk_reason`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_reason: Option<String>,
    /// The test results recorded (Task-A01-easel128): failed first, then not run, then passed;
    /// empty when the agent recorded none (older tasks).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub check_results: Vec<CheckResult>,
}

/// The screen of `task`: `links` are its recorded links, `decisions` its AI decisions not reviewed
/// yet, `results` its test results ([`check_result::by_task`]), `events` the history (any task's;
/// only this task's classify events are read).
pub fn evidence(
    task: &Task,
    links: &[Recorded],
    decisions: &[Decision],
    results: &[CheckResult],
    events: &[Event],
) -> Evidence {
    let summary = summary_sentence(task.handoff_note.as_deref());
    let parts = ORDER
        .into_iter()
        .filter(|p| has(*p, task, &summary, links, decisions))
        .collect();
    Evidence {
        parts,
        summary,
        risk_reason: risk_reason(events, &task.id),
        check_results: check_result::ordered(results),
    }
}

/// Whether the task has part `p`. What the person is asked and the test results are always there.
fn has(p: Part, task: &Task, summary: &str, links: &[Recorded], decisions: &[Decision]) -> bool {
    let filled = |s: &Option<String>| s.as_deref().is_some_and(|s| !s.trim().is_empty());
    match p {
        Part::AskRisk | Part::Tests => true,
        Part::CheckSteps => filled(&task.human_check_steps) || filled(&task.without_human_check),
        Part::Summary => !summary.is_empty() || !code_links(links).is_empty(),
        Part::Decisions => {
            task.has_label(AI_DECISIONS)
                || decisions
                    .iter()
                    .any(|d| d.kind == Some(DecisionKind::Detail))
        }
        Part::Handoff => filled(&task.handoff_note),
    }
}

/// The commits and merges recorded, in the order recorded.
pub fn code_links(links: &[Recorded]) -> Vec<&Recorded> {
    links
        .iter()
        .filter(|l| l.link.kind == "commit" || l.link.kind == "merge")
        .collect()
}

/// The handoff note's first sentence (decision 0025 asks for one a person can act on): its first
/// paragraph, up to the first full stop (`。！？`, or `.!?` before a space or the end); a heading
/// or list mark at its start is dropped, and its lines are joined.
pub fn summary_sentence(note: Option<&str>) -> String {
    let note = note.unwrap_or("");
    let mut paragraph = Vec::new();
    for line in note.lines() {
        let line = line.trim();
        if line.is_empty() {
            if paragraph.is_empty() {
                continue;
            }
            break;
        }
        paragraph.push(line);
    }
    let line = strip_mark(&paragraph.join(" "));
    let chars: Vec<char> = line.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let ends = match c {
            '。' | '！' | '？' => true,
            '.' | '!' | '?' => chars.get(i + 1).is_none_or(|n| n.is_whitespace()),
            _ => false,
        };
        if ends {
            return chars[..=i].iter().collect();
        }
    }
    line
}

/// `line` without a markdown heading (`# `), bullet (`- `, `* `, `+ `) or number (`1. `) at its
/// start.
fn strip_mark(line: &str) -> String {
    let hashes = line.len() - line.trim_start_matches('#').len();
    if hashes > 0 && line[hashes..].starts_with(char::is_whitespace) {
        return line[hashes..].trim_start().to_string();
    }
    for bullet in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(bullet) {
            return rest.trim_start().to_string();
        }
    }
    let digits = line.len() - line.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    if digits > 0 && line[digits..].starts_with(". ") {
        return line[digits + 2..].trim_start().to_string();
    }
    line.to_string()
}

/// The reason given with the newest risk recorded for task `id` (`alm classify --why`, kept in a
/// classify event as the change of `reason`); none when the history holds none.
pub fn risk_reason(events: &[Event], id: &str) -> Option<String> {
    events
        .iter()
        .filter(|e| e.command == "classify" && e.kind == Kind::Task && e.id.as_deref() == Some(id))
        .filter_map(|e| {
            let reason = e.changes.iter().find(|c| c.field == REASON)?.to.as_ref()?;
            let reason = reason.as_str()?.trim();
            (!reason.is_empty()).then(|| reason.to_string())
        })
        .next_back()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(json: serde_json::Value) -> Task {
        let mut base = serde_json::json!({
            "id": "A01-01", "title": "t", "area": "A01", "state": "done", "labels": []
        });
        base.as_object_mut()
            .unwrap()
            .extend(json.as_object().unwrap().clone());
        serde_json::from_value(base).unwrap()
    }

    fn link(kind: &str, reference: &str) -> Recorded {
        Recorded {
            link: serde_json::from_value(serde_json::json!({"kind": kind, "ref": reference}))
                .unwrap(),
            task: "A01-01".into(),
            at: "2026-11-20T00:00:00Z".into(),
            by: None,
        }
    }

    fn classify(id: &str, reason: Option<&str>) -> Event {
        let mut changes = vec![serde_json::json!({"field": "risk", "to": "medium"})];
        if let Some(r) = reason {
            changes.push(serde_json::json!({"field": "reason", "to": r}));
        }
        serde_json::from_value(serde_json::json!({
            "at": "2026-11-20T00:00:00Z", "via": "alm", "command": "classify", "kind": "task",
            "id": id, "changes": changes
        }))
        .unwrap()
    }

    #[test]
    fn every_part_in_order_when_the_task_has_them() {
        let t = task(serde_json::json!({
            "labels": [AI_DECISIONS], "human_check_steps": "1. Open it.",
            "handoff_note": "Built and merged. Nothing to do."
        }));
        let e = evidence(&t, &[], &[], &[], &[]);
        assert_eq!(e.parts, ORDER.to_vec());
        assert_eq!(e.summary, "Built and merged.");
    }

    #[test]
    fn test_results_come_failed_first_and_are_left_out_when_none() {
        let result = |name: &str, result: &str| {
            serde_json::from_value::<Event>(serde_json::json!({
                "at": "2026-11-20T00:00:00Z", "via": "alm", "command": "check-result",
                "kind": "task", "id": "A01-01",
                "changes": [{"field": "check_result", "to": {"name": name, "result": result}}]
            }))
            .unwrap()
        };
        let events = [result("npm test", "pass"), result("cargo test", "fail")];
        let results = check_result::by_task(&events).remove("A01-01").unwrap();
        let e = evidence(&task(serde_json::json!({})), &[], &[], &results, &events);
        let names: Vec<&str> = e
            .check_results
            .iter()
            .map(|r| r.check.name.as_str())
            .collect();
        assert_eq!(names, vec!["cargo test", "npm test"]);
        let json = serde_json::to_value(&e).unwrap();
        assert_eq!(json["check_results"][0]["result"], "fail");
        let bare = serde_json::to_value(evidence(&task(serde_json::json!({})), &[], &[], &[], &[]));
        assert!(bare.unwrap().get("check_results").is_none());
    }

    #[test]
    fn a_bare_task_has_only_the_ask_and_the_tests() {
        let e = evidence(&task(serde_json::json!({})), &[], &[], &[], &[]);
        assert_eq!(e.parts, vec![Part::AskRisk, Part::Tests]);
        assert_eq!(e.summary, "");
        assert_eq!(e.risk_reason, None);
    }

    #[test]
    fn check_steps_count_the_reason_for_no_check_and_summary_counts_commits() {
        let t = task(serde_json::json!({"without_human_check": "only a person can try it"}));
        let links = [
            link("commit", "abcdef1"),
            link("pr", "https://example.com/1"),
        ];
        let e = evidence(&t, &links, &[], &[], &[]);
        assert_eq!(
            e.parts,
            vec![Part::AskRisk, Part::CheckSteps, Part::Summary, Part::Tests]
        );
        assert_eq!(code_links(&links).len(), 1);
    }

    #[test]
    fn the_newest_reason_of_this_task_is_kept() {
        let events = [
            classify("A01-01", Some("old")),
            classify("A01-02", Some("other task")),
            classify("A01-01", Some(" M3: words users see ")),
            classify("A01-01", None),
        ];
        assert_eq!(
            risk_reason(&events, "A01-01").as_deref(),
            Some("M3: words users see")
        );
        assert_eq!(risk_reason(&events, "A01-03"), None);
    }

    #[test]
    fn the_first_sentence_of_the_first_paragraph() {
        let s = |n: &str| summary_sentence(Some(n));
        assert_eq!(
            s("The screen is built and merged. Nothing to do.\n\nCommit abc."),
            "The screen is built and merged."
        );
        assert_eq!(
            s("「证据」一屏做好了，已合入。不用做什么。"),
            "「证据」一屏做好了，已合入。"
        );
        assert_eq!(s("\n\n# Done in v0.1.2\nmore"), "Done in v0.1.2 more");
        assert_eq!(s("- Done. More"), "Done.");
        assert_eq!(s("2. Merged! Yes"), "Merged!");
        assert_eq!(
            s("Updated app.tsx for v0.1.75 today"),
            "Updated app.tsx for v0.1.75 today"
        );
        assert_eq!(summary_sentence(None), "");
    }
}
