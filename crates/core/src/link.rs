//! Links to code and releases (Task-A01-mint242, decision 0021 § 1.1 "Links to code and
//! releases", approver 2026-11-08).
//!
//! A task records the commits, branches, merge commits, pull requests, CI runs and releases that
//! went with it, each as `{kind, ref, url}`: its kind, its reference (a commit hash, a branch
//! name, a PR number, a run ID, a tag) and, when there is one, the web address it opens at. An
//! agent records them (`alm link`); `alm` never runs Git to find them (decision 0017).
//!
//! Each is an event change named `link`: `to` is the link (added, or its address changed), a
//! change with only a `from` takes it off. Like the traces, they live in the history alone:
//! `task.json` gains no field and the format stays `allomos/7`. A link is known by its kind and
//! reference; a task's links are what those changes leave, in the order first recorded.
//!
//! `alm` refuses only what it cannot record: an unknown kind, an empty reference, a commit or
//! merge reference that is not a hash, an address that is not a web address, and taking off a
//! link the task does not record. Whether that commit or run exists is the agent's to check
//! ("`alm` tells; the agent checks").

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event::{Change, Event, Kind};
use crate::ops::{check_holder, find_task};
use crate::{Problem, Taskbag};

/// The field that records a link.
pub const FIELD: &str = "link";
/// The command that records one (`alm link`).
pub const COMMAND: &str = "link";
/// The kinds of link, in the order a task shows them.
pub const KINDS: [&str; 6] = ["commit", "branch", "merge", "pr", "ci", "release"];

/// One link as recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// One of [`KINDS`].
    pub kind: String,
    /// What it is called: a commit hash (lower case), a branch name, a PR number, a run ID, a tag.
    #[serde(rename = "ref")]
    pub reference: String,
    /// Where it opens (an `http://` or `https://` address), when one was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(flatten)]
    pub extra: crate::model::Extra,
}

impl Link {
    fn same(&self, other: &Link) -> bool {
        self.kind == other.kind && self.reference == other.reference
    }
}

/// A link with its task and when it was recorded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Recorded {
    #[serde(flatten)]
    pub link: Link,
    /// The task it belongs to.
    pub task: String,
    /// When it was first recorded (UTC).
    pub at: String,
    /// Who first recorded it (`--as`), when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
}

fn is_hash(s: &str) -> bool {
    (7..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A link as given, in its one written form; refused when it cannot be one: an unknown kind
/// (`link_kind_unknown`), an empty reference, or a commit or merge reference that is not a hash
/// of 7 to 64 hexadecimal digits (`link_ref_invalid`), an address that does not start with
/// `http://` or `https://` (`link_url_invalid`). A kind is taken in either case; a hash is kept
/// in lower case; a PR given as `#12` is kept as `12`.
pub fn parse(kind: &str, reference: &str, url: Option<&str>) -> Result<Link, Problem> {
    let k = kind.trim().to_ascii_lowercase();
    if !KINDS.contains(&k.as_str()) {
        return Err(Problem::LinkKindUnknown {
            kind: kind.to_string(),
        });
    }
    let mut r = reference.trim().to_string();
    if k == "commit" || k == "merge" {
        r = r.to_ascii_lowercase();
        if !is_hash(&r) {
            return Err(Problem::LinkRefInvalid {
                kind: k,
                reference: reference.to_string(),
            });
        }
    }
    if k == "pr" {
        r = r.trim_start_matches('#').to_string();
    }
    if r.is_empty() {
        return Err(Problem::LinkRefInvalid {
            kind: k,
            reference: reference.to_string(),
        });
    }
    let url = url.map(str::trim).filter(|u| !u.is_empty());
    if let Some(u) = url {
        let web = ["http://", "https://"]
            .iter()
            .any(|p| u.len() > p.len() && u[..p.len()].eq_ignore_ascii_case(p));
        if !web || u.chars().any(char::is_whitespace) {
            return Err(Problem::LinkUrlInvalid { url: u.to_string() });
        }
    }
    Ok(Link {
        kind: k,
        reference: r,
        url: url.map(String::from),
        extra: Default::default(),
    })
}

/// Every link in `events` (oldest first), in the order first recorded; a later change of the same
/// link replaces it, a change with only a `from` takes it off. A change whose value is not a link
/// is left out.
pub fn all(events: &[Event]) -> Vec<Recorded> {
    let mut out: Vec<Recorded> = Vec::new();
    let read = |v: &Option<serde_json::Value>| {
        v.clone()
            .and_then(|v| serde_json::from_value::<Link>(v).ok())
    };
    for e in events {
        let (Kind::Task, Some(task)) = (e.kind, e.id.as_deref()) else {
            continue;
        };
        for c in e.changes.iter().filter(|c| c.field == FIELD) {
            match (read(&c.from), read(&c.to)) {
                (_, Some(link)) => {
                    match out
                        .iter_mut()
                        .find(|r| r.task == task && r.link.same(&link))
                    {
                        Some(r) => r.link = link,
                        None => out.push(Recorded {
                            link,
                            task: task.to_string(),
                            at: e.at.clone(),
                            by: e.by.clone(),
                        }),
                    }
                }
                (Some(gone), None) => out.retain(|r| !(r.task == task && r.link.same(&gone))),
                (None, None) => {}
            }
        }
    }
    out
}

/// [`all`], by task ID, each task's links by kind ([`KINDS`]'s order), then as first recorded.
/// Tasks with none are left out.
pub fn by_task(events: &[Event]) -> BTreeMap<String, Vec<Recorded>> {
    let mut out: BTreeMap<String, Vec<Recorded>> = BTreeMap::new();
    for r in all(events) {
        out.entry(r.task.clone()).or_default().push(r);
    }
    for list in out.values_mut() {
        list.sort_by_key(|r| KINDS.iter().position(|k| *k == r.link.kind));
    }
    out
}

/// Whether `link` is what `given` names (the reverse lookup, from a commit to its task): its
/// reference or its address, exactly; for a commit or merge, also a hash of 7 digits or more that
/// one of the two starts with, in either case.
pub fn matches(link: &Link, given: &str) -> bool {
    let g = given.trim();
    if g.is_empty() {
        return false;
    }
    if link.reference == g || link.url.as_deref() == Some(g) {
        return true;
    }
    if link.kind == "commit" || link.kind == "merge" {
        let g = g.to_ascii_lowercase();
        return is_hash(&g) && (link.reference.starts_with(&g) || g.starts_with(&link.reference));
    }
    if link.kind == "pr" {
        return link.reference == g.trim_start_matches('#');
    }
    false
}

/// The change `alm link` makes on task `id`, and the link after it. Refused: an unknown task, one
/// someone else holds (the holder rule; done and dropped tasks may be linked too, a release often
/// comes after), a link that cannot be one ([`parse`]), and taking off one the task does not
/// record (`link_not_recorded`). `None` as the change: nothing would change (the same link again, or
/// one without an address given again without one).
pub fn record(
    taskbag: &Taskbag,
    recorded: &[Recorded],
    id: &str,
    me: Option<&str>,
    given: &Link,
    remove: bool,
) -> Result<(Option<Change>, Link), Problem> {
    let task = find_task(taskbag, id)?;
    check_holder(&task, me)?;
    let before = recorded
        .iter()
        .find(|r| r.task == task.id && r.link.same(given))
        .map(|r| r.link.clone());
    let value = |l: &Link| serde_json::to_value(l).expect("links serialize");
    if remove {
        let Some(before) = before else {
            return Err(Problem::LinkNotRecorded {
                kind: given.kind.clone(),
                reference: given.reference.clone(),
                task: task.id,
            });
        };
        let change = Change {
            extra: Default::default(),
            field: FIELD.to_string(),
            from: Some(value(&before)),
            to: None,
        };
        return Ok((Some(change), before));
    }
    let mut after = given.clone();
    if let Some(b) = &before {
        // Given again without an address: the one recorded stays.
        if after.url.is_none() {
            after.url = b.url.clone();
        }
        after.extra = b.extra.clone();
    }
    if before.as_ref() == Some(&after) {
        return Ok((None, after));
    }
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
    use crate::{Area, Project, State, Task};

    fn bag(holder: Option<&str>) -> Taskbag {
        let task = Task {
            folders: Default::default(),
            session: None,
            extra: Default::default(),
            id: "A01-1".into(),
            title: "Ship".into(),
            area: "A01".into(),
            state: State::InProgress,
            labels: Vec::new(),
            holder: holder.map(String::from),
            waiting_on: None,
            who: None,
            blocked_by: Vec::new(),
            next_dependency: None,
            iteration: None,
            handoff_note: None,
            without_human_check: None,
            human_check_steps: None,
            description: String::new(),
        };
        Taskbag {
            project: serde_json::from_value::<Project>(serde_json::json!({
                "format": crate::FORMAT, "id": "P", "name": "P", "labels": ["implemented"]
            }))
            .unwrap(),
            areas: vec![Area {
                extra: Default::default(),
                id: "A01".into(),
                name: "A".into(),
                order: 0,
            }],
            iterations: Vec::new(),
            tasks: vec![task],
        }
    }

    fn event(at: &str, change: Change) -> Event {
        Event {
            extra: Default::default(),
            at: at.into(),
            by: Some("me".into()),
            via: "alm".into(),
            command: COMMAND.into(),
            kind: Kind::Task,
            id: Some("A01-1".into()),
            agent: None,
            tool: None,
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes: vec![change],
        }
    }

    #[test]
    fn a_link_has_one_written_form() {
        let l = parse(" Commit ", "ABCDEF1234", None).unwrap();
        assert_eq!(
            (l.kind.as_str(), l.reference.as_str()),
            ("commit", "abcdef1234")
        );
        assert_eq!(parse("pr", "#12", None).unwrap().reference, "12");
        let l = parse(
            "release",
            "v0.1.43",
            Some(" https://example.test/r/v0.1.43 "),
        )
        .unwrap();
        assert_eq!(l.url.as_deref(), Some("https://example.test/r/v0.1.43"));
        assert!(matches!(
            parse("tag", "v1", None),
            Err(Problem::LinkKindUnknown { .. })
        ));
        for (k, r) in [
            ("commit", "main"),
            ("merge", "abc"),
            ("branch", "  "),
            ("pr", "#"),
        ] {
            assert!(
                matches!(parse(k, r, None), Err(Problem::LinkRefInvalid { .. })),
                "{k} {r}"
            );
        }
        for u in ["example.test", "ftp://x", "https://", "https://a b"] {
            assert!(
                matches!(
                    parse("ci", "1", Some(u)),
                    Err(Problem::LinkUrlInvalid { .. })
                ),
                "{u}"
            );
        }
    }

    #[test]
    fn links_are_added_changed_and_taken_off() {
        let b = bag(Some("me"));
        let commit = parse("commit", "abcdef1234567", None).unwrap();
        let (c, _) = record(&b, &[], "A01-1", Some("me"), &commit, false).unwrap();
        let mut events = vec![event("2026-11-08T01:00:00Z", c.unwrap())];
        let pr = parse("pr", "7", Some("https://example.test/pull/7")).unwrap();
        let (c, _) = record(&b, &all(&events), "A01-1", Some("me"), &pr, false).unwrap();
        events.push(event("2026-11-08T01:01:00Z", c.unwrap()));
        // The same commit again, now with its address: one change, the link keeps its first time.
        let with_url = parse(
            "commit",
            "ABCDEF1234567",
            Some("https://example.test/c/abcdef1"),
        )
        .unwrap();
        let (c, after) = record(&b, &all(&events), "A01-1", Some("me"), &with_url, false).unwrap();
        let c = c.unwrap();
        assert!(c.from.is_some());
        assert_eq!(after.url.as_deref(), Some("https://example.test/c/abcdef1"));
        events.push(event("2026-11-08T01:02:00Z", c));
        let rec = all(&events);
        assert_eq!(rec.len(), 2);
        assert_eq!(rec[0].at, "2026-11-08T01:00:00Z");
        assert_eq!(
            rec[0].link.url.as_deref(),
            Some("https://example.test/c/abcdef1")
        );
        // Given again without an address: nothing changes.
        let (c, after) = record(&b, &rec, "A01-1", Some("me"), &commit, false).unwrap();
        assert!(c.is_none());
        assert!(after.url.is_some());
        // Taken off.
        let (c, _) = record(&b, &rec, "A01-1", Some("me"), &pr, true).unwrap();
        let c = c.unwrap();
        assert!(c.to.is_none());
        events.push(event("2026-11-08T01:03:00Z", c));
        let rec = all(&events);
        assert_eq!(rec.len(), 1);
        assert!(matches!(
            record(&b, &rec, "A01-1", Some("me"), &pr, true),
            Err(Problem::LinkNotRecorded { .. })
        ));
        assert!(matches!(
            record(&bag(Some("other")), &rec, "A01-1", Some("me"), &pr, false),
            Err(Problem::TaskHeld { .. })
        ));
    }

    #[test]
    fn the_reverse_lookup_takes_a_short_hash_or_the_address() {
        let c = parse(
            "commit",
            "abcdef1234567890",
            Some("https://example.test/c/1"),
        )
        .unwrap();
        assert!(matches(&c, "abcdef1"));
        assert!(matches(&c, "ABCDEF1234567890ffff"));
        assert!(matches(&c, "https://example.test/c/1"));
        assert!(!matches(&c, "abcdef"), "fewer than 7 digits");
        assert!(!matches(&c, "abcdef2"));
        let b = parse("branch", "claude/mint242", None).unwrap();
        assert!(matches(&b, "claude/mint242"));
        assert!(!matches(&b, "claude"));
        let p = parse("pr", "12", None).unwrap();
        assert!(matches(&p, "#12"));
    }

    #[test]
    fn a_task_shows_its_links_by_kind() {
        let b = bag(None);
        let mut events = Vec::new();
        for (k, r) in [("release", "v1"), ("commit", "abcdef1"), ("branch", "x")] {
            let l = parse(k, r, None).unwrap();
            let (c, _) = record(&b, &all(&events), "A01-1", None, &l, false).unwrap();
            events.push(event("2026-11-08T01:00:00Z", c.unwrap()));
        }
        let kinds: Vec<String> = by_task(&events)["A01-1"]
            .iter()
            .map(|r| r.link.kind.clone())
            .collect();
        assert_eq!(kinds, ["commit", "branch", "release"]);
    }
}
