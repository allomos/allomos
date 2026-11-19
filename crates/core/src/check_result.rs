//! Test results as records (Task-A01-easel128, decision 0029 § 5, approver 2026-11-20).
//!
//! An agent records each check it ran, or did not run, with `alm check-result <id> --name
//! "<check>" --result pass|fail|not-run [--detail "<one sentence>"]`. Like the traces, a result
//! lives only in the task's history: an event change named `check_result` whose `to` is the whole
//! record (`{ "name", "result", "detail"? }`) and whose `from` is the record of the same name
//! before, when there was one. `task.json` gains no field and the format stays `allomos/7`. A
//! task's results are the last record of each name, in the order first recorded; the Evidence
//! screen and `alm show` list failed and not run first ([`ordered`]).
//!
//! A failed check may bring screenshots (`--screenshot <file>`, decision 0029 § 6,
//! Task-A01-fondue470): `alm` copies each into `tasks/<id>/evidence/` and the record names them
//! (`screenshots`), so another computer sees them after it gets the taskbag. A task keeps at most
//! three; only PNG, JPEG and WebP pictures of at most 2 MB are taken (checks the approver
//! approved in that decision).
//!
//! `alm` refuses only what it cannot record: an unknown task, one someone else holds, a check
//! without a name, and a result that is none of the three. Whether a task has enough results, or
//! the right ones, it does not judge ("`alm` tells; the agent checks").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records a test result.
pub const FIELD: &str = "check_result";
/// The command that records one (`alm check-result`).
pub const COMMAND: &str = "check-result";

/// The written results, in the order they are listed: failed, not run, passed.
pub const RESULTS: [&str; 3] = ["fail", "not-run", "pass"];

/// The folder beside a task's `task.json` that keeps its screenshots (decision 0029 § 6,
/// Task-A01-fondue470): `tasks/<id>/evidence/`.
pub const EVIDENCE_DIR: &str = "evidence";
/// The most screenshots one task keeps (approver 2026-11-20: "最多提交3张").
pub const MOST_SCREENSHOTS: usize = 3;
/// The largest screenshot taken, in bytes (2 MB).
pub const MOST_BYTES: u64 = 2 * 1024 * 1024;

/// One check's result as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckResult {
    /// The check, as the agent names it (for example "npm test").
    pub name: String,
    /// `pass`, `fail` or `not-run`.
    pub result: String,
    /// One sentence: what ran, or why it failed or did not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// The screenshots that go with a failed check: file names in the task's
    /// [`EVIDENCE_DIR`] (Task-A01-fondue470). Only a `fail` has any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

/// A result with when and by whom it was last recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub check: CheckResult,
    /// When this result was recorded (UTC).
    pub at: String,
    /// Who recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

/// The one written form of a result: `pass`, `fail` or `not-run` (any case; `not_run`, `notrun`
/// and `not run` are taken as `not-run`). Anything else is refused (`check_result_unknown`).
pub fn result_of(given: &str) -> Result<&'static str, Problem> {
    let s = given.trim().to_ascii_lowercase();
    match s.as_str() {
        "pass" | "passed" => Ok("pass"),
        "fail" | "failed" => Ok("fail"),
        "not-run" | "not_run" | "notrun" | "not run" => Ok("not-run"),
        _ => Err(Problem::CheckResultUnknown {
            result: given.to_string(),
        }),
    }
}

/// The kind of picture `bytes` hold, by their first bytes: `png`, `jpeg` or `webp`; `None` for
/// anything else (decision 0029 § 6: only these three are taken).
pub fn image_kind(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpeg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

/// Why the file `file` (as the agent gave it), `size` bytes long and starting with `head`, cannot
/// be a screenshot: larger than [`MOST_BYTES`] (`screenshot_too_large`, with how to make it
/// smaller) or not a PNG, JPEG or WebP picture (`screenshot_not_image`).
pub fn screenshot_problem(file: &str, size: u64, head: &[u8]) -> Option<Problem> {
    if image_kind(head).is_none() {
        return Some(Problem::ScreenshotNotImage {
            file: file.to_string(),
        });
    }
    (size > MOST_BYTES).then(|| Problem::ScreenshotTooLarge {
        file: file.to_string(),
        size: format!("{:.1} MB", size as f64 / (1024.0 * 1024.0)),
    })
}

/// The name a screenshot is kept under in the task's [`EVIDENCE_DIR`]: the given file's own name,
/// with anything but letters, digits, `.`, `-` and `_` made `-`, its ending set by its `kind`
/// (`.png`, `.jpg`, `.webp`), and `-2`, `-3` … added while `taken` has it.
pub fn screenshot_name(given: &str, kind: &str, taken: &[String]) -> String {
    let base = given.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = match base.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => base,
    };
    let mut stem: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    stem = stem.trim_matches('-').chars().take(60).collect();
    if stem.is_empty() {
        stem = "screenshot".to_string();
    }
    let ending = match kind {
        "jpeg" => "jpg",
        other => other,
    };
    let mut name = format!("{stem}.{ending}");
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&name)) {
        name = format!("{stem}-{n}.{ending}");
        n += 1;
    }
    name
}

/// Every task's results, by task ID, from `events` (oldest first): the last record of each name,
/// in the order the names were first recorded. A change whose value is not a record is left out;
/// tasks with none are left out.
pub fn by_task(events: &[Event]) -> BTreeMap<String, Vec<Recorded>> {
    let mut out: BTreeMap<String, Vec<Recorded>> = BTreeMap::new();
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            let Some(Ok(check)) = c.to.clone().map(serde_json::from_value::<CheckResult>) else {
                continue;
            };
            let rec = Recorded {
                check,
                at: e.at.clone(),
                by: e.by.clone(),
            };
            let list = out.entry(task.to_string()).or_default();
            match list.iter_mut().find(|r| r.check.name == rec.check.name) {
                Some(r) => *r = rec,
                None => list.push(rec),
            }
        }
    }
    out
}

/// `results` in the order they are shown: failed first, then not run, then passed (an unknown
/// result after them); within each, in the order first recorded.
pub fn ordered(results: &[Recorded]) -> Vec<Recorded> {
    let rank = |r: &Recorded| {
        RESULTS
            .iter()
            .position(|w| *w == r.check.result)
            .unwrap_or(RESULTS.len())
    };
    let mut out = results.to_vec();
    out.sort_by_key(rank);
    out
}

/// The change `alm check-result` makes on task `id`, and the record after it. Refused: an unknown
/// task, one someone else holds (the holder rule; done and dropped tasks may be recorded on too, a
/// result is often written down at the end), a check without a name (`check_name_missing`), and a
/// result that is none of the three (`check_result_unknown`). `None` as the change: the same
/// result, detail and screenshots are recorded already.
///
/// `shots` are the new screenshots' names (already checked as pictures, [`screenshot_problem`]),
/// `kept` the files the task's [`EVIDENCE_DIR`] holds now. Refused (decision 0029 § 6): a
/// screenshot with a result other than `fail` (`screenshot_needs_fail`), and more than
/// [`MOST_SCREENSHOTS`] for the task (`screenshots_full`, naming those it has). A failed check
/// recorded again keeps its earlier screenshots and adds the new ones; a pass or not run has none.
#[allow(clippy::too_many_arguments)]
pub fn record(
    taskbag: &Taskbag,
    recorded: &BTreeMap<String, Vec<Recorded>>,
    id: &str,
    me: Option<&str>,
    name: &str,
    result: &str,
    detail: Option<&str>,
    shots: &[String],
    kept: &[String],
) -> Result<(Option<Change>, CheckResult), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if name.is_empty() {
        return Err(Problem::CheckNameMissing);
    }
    let result = result_of(result)?;
    if !shots.is_empty() && result != "fail" {
        return Err(Problem::ScreenshotNeedsFail {
            result: result.to_string(),
        });
    }
    if kept.len() + shots.len() > MOST_SCREENSHOTS {
        return Err(Problem::ScreenshotsFull {
            id: task.id.clone(),
            count: kept.len().to_string(),
            have: if kept.is_empty() {
                "none".to_string()
            } else {
                kept.join(", ")
            },
            room: MOST_SCREENSHOTS.saturating_sub(kept.len()).to_string(),
        });
    }
    let detail = detail
        .map(|d| d.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|d| !d.is_empty());
    let before = recorded
        .get(&task.id)
        .and_then(|list| list.iter().find(|r| r.check.name == name))
        .map(|r| r.check.clone());
    let mut screenshots = Vec::new();
    if result == "fail" {
        if let Some(b) = before.as_ref().filter(|b| b.result == "fail") {
            screenshots.extend(b.screenshots.iter().cloned());
        }
        screenshots.extend(shots.iter().cloned());
    }
    let after = CheckResult {
        name,
        result: result.to_string(),
        detail,
        screenshots,
        extra: Default::default(),
    };
    if before.as_ref().is_some_and(|b| {
        b.result == after.result && b.detail == after.detail && b.screenshots == after.screenshots
    }) {
        return Ok((None, after));
    }
    let value = |c: &CheckResult| serde_json::to_value(c).expect("a check result serializes");
    let change = Change {
        extra: Default::default(),
        field: FIELD.to_string(),
        from: before.as_ref().map(value),
        to: Some(value(&after)),
    };
    Ok((Some(change), after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn event(id: &str, at: &str, to: Value) -> Event {
        serde_json::from_value(json!({
            "at": at,
            "via": "alm",
            "command": COMMAND,
            "kind": "task",
            "id": id,
            "by": "s1",
            "changes": [ { "field": FIELD, "to": to } ],
        }))
        .unwrap()
    }

    #[test]
    fn a_result_has_one_written_form() {
        for (given, want) in [
            ("pass", "pass"),
            ("PASS", "pass"),
            (" Fail ", "fail"),
            ("not-run", "not-run"),
            ("NOT_RUN", "not-run"),
            ("not run", "not-run"),
        ] {
            assert_eq!(result_of(given).unwrap(), want, "{given}");
        }
        for given in ["", "ok", "skipped", "passes"] {
            assert!(
                matches!(result_of(given), Err(Problem::CheckResultUnknown { .. })),
                "{given}"
            );
        }
    }

    #[test]
    fn the_last_record_of_each_name_wins_in_first_order() {
        let events = vec![
            event(
                "A01-1",
                "2026-11-20T00:00:01.000Z",
                json!({ "name": "npm test", "result": "fail", "detail": "2 failed" }),
            ),
            event(
                "A01-1",
                "2026-11-20T00:00:02.000Z",
                json!({ "name": "cargo test", "result": "pass" }),
            ),
            event(
                "A01-2",
                "2026-11-20T00:00:03.000Z",
                json!({ "name": "npm test", "result": "pass" }),
            ),
            event(
                "A01-1",
                "2026-11-20T00:00:04.000Z",
                json!({ "name": "npm test", "result": "pass" }),
            ),
            event("A01-1", "2026-11-20T00:00:05.000Z", json!("not a record")),
        ];
        let rec = by_task(&events);
        let one = &rec["A01-1"];
        assert_eq!(one.len(), 2);
        assert_eq!(one[0].check.name, "npm test");
        assert_eq!(one[0].check.result, "pass");
        assert_eq!(one[0].check.detail, None);
        assert_eq!(one[0].at, "2026-11-20T00:00:04.000Z");
        assert_eq!(one[1].check.name, "cargo test");
        assert_eq!(rec["A01-2"].len(), 1);
    }

    #[test]
    fn screenshots_are_pictures_of_at_most_2_mb() {
        assert_eq!(image_kind(b"\x89PNG\r\n\x1a\n..."), Some("png"));
        assert_eq!(image_kind(&[0xFF, 0xD8, 0xFF, 0xE1]), Some("jpeg"));
        assert_eq!(image_kind(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(image_kind(b"GIF89a"), None);
        assert!(screenshot_problem("a.png", MOST_BYTES, b"\x89PNG\r\n\x1a\n").is_none());
        assert!(matches!(
            screenshot_problem("a.png", MOST_BYTES + 1, b"\x89PNG\r\n\x1a\n"),
            Some(Problem::ScreenshotTooLarge { .. })
        ));
        assert!(matches!(
            screenshot_problem("a.txt", 10, b"hello"),
            Some(Problem::ScreenshotNotImage { .. })
        ));
    }

    #[test]
    fn a_screenshot_is_kept_under_a_plain_free_name() {
        let taken = vec!["shot.png".to_string(), "shot-2.png".to_string()];
        assert_eq!(screenshot_name("/tmp/x/shot.PNG", "png", &taken), "shot-3.png");
        assert_eq!(screenshot_name("Screen Shot 1.jpeg", "jpeg", &[]), "Screen-Shot-1.jpg");
        assert_eq!(screenshot_name("C:\\a\\b.webp", "webp", &[]), "b.webp");
        assert_eq!(screenshot_name("截图.png", "png", &[]), "screenshot.png");
        assert_eq!(screenshot_name(".png", "png", &[]), "png.png");
    }

    #[test]
    fn failed_and_not_run_come_first() {
        let events = vec![
            event("A01-1", "t1", json!({ "name": "a", "result": "pass" })),
            event("A01-1", "t2", json!({ "name": "b", "result": "not-run" })),
            event("A01-1", "t3", json!({ "name": "c", "result": "pass" })),
            event("A01-1", "t4", json!({ "name": "d", "result": "fail" })),
        ];
        let names: Vec<String> = ordered(&by_task(&events)["A01-1"])
            .into_iter()
            .map(|r| r.check.name)
            .collect();
        assert_eq!(names, vec!["d", "b", "a", "c"]);
    }
}
