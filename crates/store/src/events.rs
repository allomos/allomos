//! The taskbag's `events/` folder: one small file per change (R7, Task-A01-17).
//!
//! Each file is named by the time of the change plus a random suffix,
//! `2026-10-13-140205-123-k7f3q2.json`, so two people's changes never get the same name and Git
//! merges them without a conflict. Events are written under the taskbag's lock, together with the
//! change they describe; they are read without a lock, like everything else.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use allomos_core::event::{Change, Event, Kind};
use allomos_core::{Issue, Problem, event_notes, parse_event};
use serde::Serialize;

use crate::{json_text, sorted_entries, write_atomic, write_failed};

pub const EVENTS_DIR: &str = "events";

/// Who makes a change, and through which writer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor<'a> {
    /// The `--as` name, when one was given.
    pub by: Option<&'a str>,
    /// `alm` or `app`.
    pub via: &'static str,
}

impl<'a> Actor<'a> {
    /// The `alm` command, run as `by`.
    pub fn alm(by: Option<&'a str>) -> Self {
        Actor { by, via: "alm" }
    }

    /// The Allomos app. It does not know the person's name yet.
    pub const APP: Actor<'static> = Actor {
        by: None,
        via: "app",
    };

    /// An event made now by this actor.
    pub fn event(
        &self,
        command: &str,
        kind: Kind,
        id: Option<&str>,
        changes: Vec<Change>,
    ) -> Event {
        // The agent `alm` runs under, from its environment (Task-A03-27); the app records none.
        let (agent, tool) = match self.via {
            "alm" => allomos_core::event::agent_from_env(|name| std::env::var(name).ok())
                .map_or((None, None), |(agent, tool)| (Some(agent), tool)),
            _ => (None, None),
        };
        Event {
            extra: Default::default(),
            at: now().0,
            by: self.by.map(String::from),
            via: self.via.to_string(),
            command: command.to_string(),
            kind,
            id: id.map(String::from),
            agent,
            tool,
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes,
        }
    }
}

/// Now, in UTC: the event's `at` (`2026-10-13T14:02:05Z`) and the start of its file name
/// (`2026-10-13-140205-123`, with milliseconds, so files sort in the order they were written).
/// Tests may fix the time with `ALLOMOS_NOW` (`2026-10-13T14:02:05Z` or `2026-10-13T14:02Z`).
/// Now, in UTC (`2026-10-13T14:02:05Z`); `ALLOMOS_NOW` fixes it in tests.
pub fn now_utc() -> String {
    now().0
}

fn now() -> (String, String) {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let millis = since.subsec_millis();
    if let Ok(fixed) = std::env::var("ALLOMOS_NOW") {
        let stem = format!(
            "{}-{millis:03}",
            fixed.replace('T', "-").replace([':', 'Z'], "")
        );
        return (fixed, stem);
    }
    let secs = since.as_secs() as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (y, m, d) = crate::handoff::civil_from_days(days);
    let (hh, mm, ss) = (rest / 3600, rest % 3600 / 60, rest % 60);
    (
        format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z"),
        format!("{y:04}-{m:02}-{d:02}-{hh:02}{mm:02}{ss:02}-{millis:03}"),
    )
}

/// Six random lowercase letters and digits (a new pseudonym's, too).
pub(crate) fn random_suffix() -> String {
    suffix()
}

/// Six random lowercase letters and digits, from the standard library's random hash keys.
fn suffix() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u128(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
    );
    h.write_u32(std::process::id());
    let mut n = h.finish();
    (0..6)
        .map(|_| {
            let c = b"abcdefghijklmnopqrstuvwxyz0123456789"[(n % 36) as usize];
            n /= 36;
            c as char
        })
        .collect()
}

/// A free file name for an event written now, relative to the taskbag: `events/<stem>-<suffix>.json`.
pub(crate) fn new_file(root: &Path) -> String {
    let stem = now().1;
    loop {
        let name = format!("{EVENTS_DIR}/{stem}-{}.json", suffix());
        if !root.join(&name).exists() {
            return name;
        }
    }
}

/// The one-time number the change being made now runs under (Task-A01-58): set by `alm` for one
/// command (`--request-id`), so every event that command writes carries it.
static REQUEST: std::sync::Mutex<Option<allomos_core::event::Change>> = std::sync::Mutex::new(None);

/// Mark every event written from now on with the one-time number `id` and the input
/// fingerprint `input` ([`allomos_core::request`]); `None` stops marking.
pub fn set_request(request: Option<(&str, &str)>) {
    let change = request.map(|(id, input)| allomos_core::request::change(id, input));
    *REQUEST.lock().unwrap_or_else(|e| e.into_inner()) = change;
}

/// What the writer said about who runs the changes made now (Task-A01-black004): `alm`'s
/// `--model`, `--for` and `--approval` (or `ALLOMOS_MODEL`, `ALLOMOS_FOR`, `ALLOMOS_APPROVAL`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Who {
    /// The exact model ID the agent runs on.
    pub model: Option<String>,
    /// The pseudonym ID of the person the agent works for, when it is not this computer's person.
    pub on_behalf_of: Option<String>,
    /// The approval ID the change relies on.
    pub approval: Option<String>,
    /// Why (Task-A01-air289, decision 0021 § 1.1): `alm --why`, or the command's own why
    /// (`reopen --note`, `recheck --reason`, `wait --on`, `done --without-human-check`). Recorded
    /// as a `reason` change in each task event the command writes.
    pub reason: Option<String>,
}

static WHO: std::sync::Mutex<Option<Who>> = std::sync::Mutex::new(None);

/// What every event written from now on says about who runs it ([`Who`]); `None` stops it.
pub fn set_who(who: Option<Who>) {
    *WHO.lock().unwrap_or_else(|e| e.into_inner()) = who;
}

/// What the events written since the last [`take_gaps`] could not say about who made them
/// (Task-A01-black004): `reason` (Task-A01-air289), `model`, `session` or `for`, each once, in
/// that order.
static GAPS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());

/// The gaps of the events written since the last call ([`GAPS`]), and forget them.
pub fn take_gaps() -> Vec<&'static str> {
    let mut gaps = std::mem::take(&mut *GAPS.lock().unwrap_or_else(|e| e.into_inner()));
    let order = ["reason", "model", "session", "for"];
    gaps.sort_by_key(|g| order.iter().position(|o| o == g));
    gaps.dedup();
    gaps
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// The project an event belongs to, for its pseudonym: the event's own ID for a project event
/// (`init` writes it before `project.json`), else `project.json`'s `id`.
fn project_id(root: &Path, event: &Event) -> Option<String> {
    if event.kind == Kind::Project {
        if let Some(id) = clean(event.id.clone()) {
            return Some(id);
        }
    }
    project_json_id(root)
}

/// `project.json`'s `id`, read on its own (without loading the taskbag).
fn project_json_id(root: &Path) -> Option<String> {
    let text = crate::read_text(&root.join("project.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    clean(v.get("id")?.as_str().map(String::from))
}

/// Fill in who made `event` (Task-A01-black004, decision 0021 § 1.1), unless its writer already
/// did. An agent's change (`agent` set) gets its session ID from the environment, its model from
/// `--model` (or the one its session named before, on this computer) and `for`, this computer's
/// person's pseudonym unless `--for` names another. A change in the app, or with `alm` where no
/// agent was seen, gets the person's pseudonym as `actor.person`. What could not be learned is
/// left out and noted in [`GAPS`].
fn fill_who(root: &Path, event: &mut Event) {
    let env = |name: &str| clean(std::env::var(name).ok());
    let who = WHO
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_default();
    let mut gaps = Vec::new();
    if event.approval.is_none() {
        event.approval = clean(who.approval).or_else(|| env("ALLOMOS_APPROVAL"));
    }
    let given_for = clean(who.on_behalf_of).or_else(|| env("ALLOMOS_FOR"));
    if event.actor.is_none() {
        let mut actor = allomos_core::event::EventActor::default();
        let project = project_id(root, event);
        let pseudonym = || project.as_deref().and_then(crate::identity::pseudonym);
        match event.agent.clone() {
            Some(agent) if event.via == "alm" => {
                actor.session =
                    allomos_core::event::session_from_env(&agent, |n| std::env::var(n).ok());
                let named = clean(who.model).or_else(|| env("ALLOMOS_MODEL"));
                actor.model = match (&named, &actor.session) {
                    (Some(model), Some(session)) => {
                        crate::identity::remember_session_model(session, model);
                        named.clone()
                    }
                    (Some(_), None) => named.clone(),
                    (None, Some(session)) => crate::identity::session_model(session),
                    (None, None) => None,
                };
                if event.on_behalf_of.is_none() {
                    event.on_behalf_of = given_for.clone().or_else(pseudonym);
                }
                if actor.model.is_none() {
                    gaps.push("model");
                }
                if actor.session.is_none() {
                    gaps.push("session");
                }
                if event.on_behalf_of.is_none() {
                    gaps.push("for");
                }
            }
            _ => {
                actor.person = pseudonym();
                // Events only ever hold the pseudonym ID (decision 0021 § 2, Task-A01-frolic587):
                // the name the person gave the app stays on this computer, so the app's change
                // is recorded under the pseudonym, or under nobody when there is none.
                if event.via == "app" {
                    event.by = actor.person.clone();
                }
                if event.on_behalf_of.is_none() {
                    event.on_behalf_of = given_for;
                }
            }
        }
        if !actor.is_empty() {
            event.actor = Some(actor);
        }
    }
    GAPS.lock().unwrap_or_else(|e| e.into_inner()).extend(gaps);
}

/// Name the checker of a `human_check` change that has none (Task-A01-table778, decision 0021
/// § 1.1): the person an agent works for (`for`), else the person who made the change in the app
/// or with `alm` (`actor.person`). Always a pseudonym; left null when neither is known.
fn fill_checker(event: &mut Event) {
    let who = event
        .on_behalf_of
        .clone()
        .or_else(|| event.actor.as_ref().and_then(|a| a.person.clone()));
    let Some(who) = who else { return };
    for c in event
        .changes
        .iter_mut()
        .filter(|c| c.field == allomos_core::labels::HUMAN_CHECK)
    {
        if let Some(to) = c.to.as_mut().and_then(|v| v.as_object_mut()) {
            if to.get("checker").is_none_or(|v| v.is_null()) {
                to.insert("checker".into(), serde_json::Value::String(who.clone()));
            }
        }
    }
}

/// Add why to a task `event` (Task-A01-air289, decision 0021 § 1.1), unless its writer already
/// did: a `reason` change with what [`Who::reason`] says. A state or plan change `alm` writes
/// without one is noted in [`GAPS`] (`reason`), so `alm` asks for it; nothing is refused.
fn fill_reason(event: &mut Event) {
    use allomos_core::event::{needs_reason, reason, reason_change};
    if event.kind != Kind::Task || reason(event).is_some() {
        return;
    }
    let why = WHO
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|w| clean(w.reason.clone()));
    match why {
        Some(why) => event.changes.push(reason_change(&why)),
        None if event.via == "alm" && needs_reason(event) => GAPS
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push("reason"),
        None => {}
    }
}

/// Refuse `event` when the project has separation of duties on and it would make one person or
/// agent both do and check, or do and approve, its task (Task-A01-grotto036; the rule is
/// [`allomos_core::duties::refuse`]). Run once who made it is filled in, so `alm` and the app are
/// held to the same rule. While the switch is off, nothing is read.
pub(crate) fn separate_duties(root: &Path, event: &Event) -> Result<(), Problem> {
    let Some(project) = crate::read_text(&root.join("project.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<allomos_core::Project>(&t).ok())
    else {
        return Ok(());
    };
    if !allomos_core::duties::on(&project) || event.kind != Kind::Task {
        return Ok(());
    }
    allomos_core::duties::refuse(&project, &list(root).0, event)
}

/// The event as [`write`] would record it, who made it filled in, for checking it beforehand.
pub(crate) fn as_written(root: &Path, event: &Event) -> Event {
    let mut event = event.clone();
    fill_who(root, &mut event);
    fill_checker(&mut event);
    event
}

/// Write one event. Returns its file, so a change that then fails can take it back.
/// Callers hold the taskbag's lock.
pub(crate) fn write(root: &Path, event: &Event) -> Result<PathBuf, Problem> {
    let dir = root.join(EVENTS_DIR);
    fs::create_dir_all(&dir).map_err(|e| write_failed(&dir, e))?;
    let path = root.join(new_file(root));
    let mut event = event.clone();
    fill_who(root, &mut event);
    fill_checker(&mut event);
    separate_duties(root, &event)?;
    fill_reason(&mut event);
    if let Some(change) = REQUEST.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        event.changes.push(change.clone());
    }
    write_atomic(&path, &json_text(&event))?;
    Ok(path)
}

/// Every event in the taskbag, oldest first, plus the files that could not be read.
pub fn list(root: &Path) -> (Vec<Event>, Vec<Issue>) {
    let (events, issues) = list_with_files(root);
    (events.into_iter().map(|(_, e)| e).collect(), issues)
}

/// The ending of a month's packed history (Task-A01-59): `events/2026-09.events`. Not `.json`,
/// so an older Allomos, which reads only `.json` files there, skips it without an error.
pub const PACK_EXT: &str = "events";

/// One line of a packed month: an event and the name of the single file it came from.
/// Fields it does not know are kept, as in every taskbag file (Task-A01-piano403).
#[derive(Serialize, serde::Deserialize)]
struct PackLine {
    file: String,
    event: Event,
    #[serde(flatten)]
    extra: allomos_core::Extra,
}

/// The events in a month's packed file, each with its original file (`events/<name>.json`), and
/// the lines that could not be read.
fn read_pack(path: &Path, issues: &mut Vec<Issue>) -> Vec<(String, Event)> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let file = format!("{EVENTS_DIR}/{name}");
    let text = match crate::read_text(path) {
        Ok(t) => t,
        Err(e) => {
            issues.push(Issue::new(
                &file,
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            ));
            return Vec::new();
        }
    };
    let mut out = Vec::new();
    for (n, line) in text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        match serde_json::from_str::<PackLine>(line) {
            Ok(mut p) => {
                let at = format!("{file}:{}", n + 1);
                if p.event.kind.is_unknown() {
                    if let Some(v) = serde_json::from_str::<serde_json::Value>(line)
                        .ok()
                        .and_then(|v| v.get("event")?.get("kind").cloned())
                    {
                        p.event.extra.insert("kind".into(), v);
                    }
                }
                issues.extend(event_notes(&at, &p.event));
                let mut line_notes = Vec::new();
                allomos_core::unknown_notes(&at, "", &p.extra, &[], &mut line_notes);
                issues.extend(line_notes);
                out.push((format!("{EVENTS_DIR}/{}", p.file), p.event))
            }
            Err(e) => issues.push(Issue::new(
                &format!("{file}:{}", n + 1),
                None,
                Problem::NotJson {
                    detail: e.to_string(),
                },
            )),
        }
    }
    out
}

/// What [`pack`] did for one month.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Packed {
    /// `2026-09`.
    pub month: String,
    /// `events/2026-09.events`, relative to the taskbag.
    pub file: String,
    /// How many changes the month's file now holds.
    pub changes: usize,
    /// How many single files were packed into it (and removed).
    pub removed: usize,
}

/// Pack the history of whole earlier months (Task-A01-59, approver 2026-10-20): from the 28th of
/// a month (UTC), every earlier month whose changes are still single files gets one file,
/// `events/<yyyy-mm>.events`, and those single files are removed. The file lists the month's
/// changes in file-name order, one per line, so the same changes give the same file on every
/// computer. A month already packed takes in any single files that arrived since. A single file
/// that cannot be read stays where it is (and is still reported). Callers hold the lock.
pub(crate) fn pack(root: &Path) -> Result<Vec<Packed>, Problem> {
    let today = now().0;
    let (this_month, day) = (&today[..7], today[8..10].parse::<u32>().unwrap_or(1));
    if day < 28 {
        return Ok(Vec::new());
    }
    let dir = root.join(EVENTS_DIR);
    // Single files of earlier months, by month.
    let mut months: std::collections::BTreeMap<String, Vec<PathBuf>> = Default::default();
    for path in sorted_entries(&dir) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if path.extension().and_then(|e| e.to_str()) != Some("json") || name.len() < 8 {
            continue;
        }
        let month = &name[..7];
        let looks_dated =
            month.as_bytes()[4] == b'-' && month.chars().filter(char::is_ascii_digit).count() == 6;
        if looks_dated && month < this_month {
            months.entry(month.to_string()).or_default().push(path);
        }
    }
    let mut done = Vec::new();
    for (month, singles) in months {
        let pack_path = dir.join(format!("{month}.{PACK_EXT}"));
        let mut ignored = Vec::new();
        let mut events: std::collections::BTreeMap<String, Event> =
            read_pack(&pack_path, &mut ignored)
                .into_iter()
                .map(|(f, e)| {
                    (
                        f.trim_start_matches(&format!("{EVENTS_DIR}/")).to_string(),
                        e,
                    )
                })
                .collect();
        let mut taken = Vec::new();
        for path in singles {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let file = format!("{EVENTS_DIR}/{name}");
            let Ok(Ok(event)) = crate::read_text(&path).map(|t| parse_event(&file, &t)) else {
                continue;
            };
            events.insert(name, event);
            taken.push(path);
        }
        if taken.is_empty() {
            continue;
        }
        let mut text = String::new();
        for (file, event) in &events {
            let line = PackLine {
                file: file.clone(),
                event: event.clone(),
                extra: Default::default(),
            };
            text.push_str(&serde_json::to_string(&line).expect("events serialize"));
            text.push('\n');
        }
        write_atomic(&pack_path, &text)?;
        for path in &taken {
            fs::remove_file(path).map_err(|e| write_failed(path, e))?;
            crate::note_written(path);
        }
        done.push(Packed {
            file: format!("{EVENTS_DIR}/{month}.{PACK_EXT}"),
            month,
            changes: events.len(),
            removed: taken.len(),
        });
    }
    Ok(done)
}

/// [`list`], with each event's file relative to the taskbag (`events/<name>.json`). Packed months
/// (Task-A01-59) are read too, each event under the name of the single file it came from, so
/// every reader sees the same history as before packing.
pub fn list_with_files(root: &Path) -> (Vec<(String, Event)>, Vec<Issue>) {
    let mut events = Vec::new();
    let mut issues = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for path in sorted_entries(&root.join(EVENTS_DIR)) {
        if path.extension().and_then(|e| e.to_str()) == Some(PACK_EXT) {
            for (file, e) in read_pack(&path, &mut issues) {
                if seen.insert(file.clone()) {
                    events.push((file, e));
                }
            }
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        let file = format!("{EVENTS_DIR}/{name}");
        if seen.contains(&file) {
            // Packed already; the single file is a copy (for example, merged in afterwards).
            continue;
        }
        match crate::read_text(&path) {
            Ok(text) => match parse_event(&file, &text) {
                Ok(e) => {
                    issues.extend(event_notes(&file, &e));
                    seen.insert(file.clone());
                    events.push((file, e));
                }
                Err(i) => issues.push(i),
            },
            Err(e) => issues.push(Issue::new(
                &file,
                None,
                Problem::ReadFailed {
                    detail: e.to_string(),
                },
            )),
        }
    }
    // By time, then by file name (which has milliseconds and is unique).
    events.sort_by(|(fa, a), (fb, b)| a.at.cmp(&b.at).then(fa.cmp(fb)));
    (events, issues)
}

/// The newest change an agent made through `alm` under a name (`--as`): who and when. The app's
/// "last used by … at …" line reads it from the change history, not from Git (decision 0017).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastUse {
    pub who: String,
    /// `2026-10-13T14:02:05Z`, UTC.
    pub at: String,
}

pub fn last_agent_use(root: &Path) -> Option<LastUse> {
    list(root)
        .0
        .into_iter()
        .rev()
        .find(|e| e.via == "alm" && e.by.as_deref().is_some_and(|b| !b.is_empty()))
        .map(|e| LastUse {
            who: e.by.unwrap_or_default(),
            at: e.at,
        })
}

/// The agent `alm` last ran under for a task (Task-A03-27): the newest event that changed a task
/// and records an agent ([`Event::agent`]), with its tool when known. **Update the taskbag…**
/// offers only that agent while it is not registered under `## Agents set up`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LastAgent {
    /// `claude` or `codex`.
    pub agent: String,
    /// `claude_cli` or `claude_app`, when the environment told it.
    pub tool: Option<String>,
    /// `2026-10-13T14:02:05Z`, UTC.
    pub at: String,
}

pub fn last_agent(root: &Path) -> Option<LastAgent> {
    list(root)
        .0
        .into_iter()
        .rev()
        .filter(|e| e.kind == Kind::Task && !e.changes.is_empty())
        .find_map(|e| {
            Some(LastAgent {
                agent: e.agent?,
                tool: e.tool,
                at: e.at,
            })
        })
}

/// Changes made in the app that no agent has been told to share yet (decision 0017, row 9): the
/// app's events after the newest change made through `alm`, and the files they touch that still
/// exist, relative to the taskbag. The next `alm` change lists them with the Git commands to
/// commit them as the person's; after that change they are no longer "after the newest `alm`
/// change". Allomos never reads Git, so a change someone already committed by hand may be listed
/// once more; committing it again changes nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct AppChanges {
    /// How many changes (events) the app made.
    pub count: usize,
    /// Who made them, as the app recorded it (the person's name), without repeats.
    pub by: Vec<String>,
    /// Their files, relative to the taskbag, with `/`.
    pub files: Vec<String>,
}

pub fn unshared_app_changes(root: &Path) -> AppChanges {
    let (all, _) = list_with_files(root);
    let after = all
        .iter()
        .rposition(|(_, e)| e.via == "alm")
        .map_or(0, |i| i + 1);
    let mut out = AppChanges::default();
    let mut files = std::collections::BTreeSet::new();
    for (file, e) in &all[after..] {
        if e.via != "app" {
            continue;
        }
        out.count += 1;
        if let Some(by) = &e.by {
            if !out.by.contains(by) {
                out.by.push(by.clone());
            }
        }
        files.insert(file.clone());
        let id = e.id.as_deref().unwrap_or_default();
        let item: Vec<String> = match e.kind {
            Kind::Task => vec![
                format!("tasks/{id}/task.json"),
                format!("tasks/{id}/description.md"),
            ],
            Kind::Area => vec![format!("areas/{id}.json")],
            Kind::Iteration => vec![format!("iterations/{id}.json")],
            Kind::Project => vec!["project.json".to_string()],
            Kind::Unknown => Vec::new(),
        };
        for f in item {
            // A file the app removed (a deleted area or iteration, an emptied description) is
            // listed too, so the commit records the removal.
            let removed = match e.kind {
                Kind::Area | Kind::Iteration => e.command.starts_with("delete"),
                Kind::Task => {
                    f.ends_with("description.md")
                        && e.changes
                            .iter()
                            .any(|c| c.field == "description" && c.from.is_some() && c.to.is_none())
                }
                Kind::Project | Kind::Unknown => false,
            };
            if removed || root.join(&f).is_file() {
                files.insert(f);
            }
        }
    }
    out.files = files.into_iter().collect();
    out
}

/// One task's history.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct History {
    /// When this taskbag started recording: its oldest event. `None` when it has none yet.
    pub since: Option<String>,
    /// The task's events, newest first.
    pub events: Vec<Event>,
    /// Event files that could not be read.
    pub issues: Vec<Issue>,
    /// Everyone named in the task's events, with their newest change anywhere in the taskbag
    /// (Task-A03-37): the task pane says whether its holder's session is still active from it.
    pub seen: Vec<Seen>,
    /// This computer's person's pseudonym ID in the project, when one was made
    /// (Task-A01-frolic587): the task pane shows the person's own name beside it, on this
    /// computer only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub you: Option<String>,
}

/// When someone (an `--as` name) last changed anything in the taskbag, and the agent they ran
/// under when known. Allomos cannot see whether a session is running; a recent change is the
/// only sign it has (approver, 2026-10-22).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Seen {
    pub by: String,
    /// `2026-10-13T14:02:05Z`, UTC.
    pub at: String,
    /// The newest agent recorded for this name ([`Event::agent`]): `claude` or `codex`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
}

/// [`Seen`] for each name in `names`, from `all` (oldest first); a name with no event is left out.
fn seen_of(all: &[Event], names: &[String]) -> Vec<Seen> {
    names
        .iter()
        .filter_map(|name| {
            let mine = || all.iter().rev().filter(|e| e.by.as_deref() == Some(name));
            let at = mine().next()?.at.clone();
            let (agent, tool) = mine()
                .find(|e| e.agent.is_some())
                .map_or((None, None), |e| (e.agent.clone(), e.tool.clone()));
            Some(Seen {
                by: name.clone(),
                at,
                agent,
                tool,
            })
        })
        .collect()
}

/// The history of the task `id`. Changes made before the taskbag started recording have no
/// events; none are invented (owner, 2026-10-13).
pub fn history(root: &Path, id: &str) -> History {
    let (all, issues) = list(root);
    let since = all.first().map(|e| e.at.clone());
    let mut events: Vec<Event> = all
        .iter()
        .filter(|e| e.kind == Kind::Task && e.id.as_deref() == Some(id))
        .cloned()
        .map(|mut e| {
            // A one-time number (Task-A01-58) is not a change to the task: not shown.
            e.changes
                .retain(|c| c.field != allomos_core::request::FIELD);
            e
        })
        .collect();
    events.reverse();
    let mut names: Vec<String> = Vec::new();
    for by in events.iter().filter_map(|e| e.by.as_deref()) {
        if !by.is_empty() && !names.iter().any(|n| n == by) {
            names.push(by.to_string());
        }
    }
    History {
        since,
        seen: seen_of(&all, &names),
        events,
        issues,
        you: project_json_id(root)
            .as_deref()
            .and_then(crate::identity::known_pseudonym),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_names_are_unique_and_sort_by_time() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-test-events-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..200 {
            let name = new_file(&dir);
            assert!(name.starts_with("events/20"), "{name}");
            assert!(name.ends_with(".json"));
            assert!(seen.insert(name));
        }
    }

    fn event(
        at: &str,
        kind: Kind,
        agent: Option<&str>,
        tool: Option<&str>,
        changed: bool,
    ) -> Event {
        Event {
            extra: Default::default(),
            at: at.into(),
            by: Some("s".into()),
            via: "alm".into(),
            command: "label".into(),
            kind,
            id: Some("A01-01".into()),
            agent: agent.map(String::from),
            tool: tool.map(String::from),
            actor: None,
            on_behalf_of: None,
            approval: None,
            changes: if changed {
                vec![Change {
                    extra: Default::default(),
                    field: "label".into(),
                    from: None,
                    to: Some("implemented".into()),
                }]
            } else {
                Vec::new()
            },
        }
    }

    #[test]
    fn history_says_when_each_name_on_the_task_last_changed_anything() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-test-seen-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let put = |name: &str, e: &Event| {
            fs::create_dir_all(dir.join(EVENTS_DIR)).unwrap();
            fs::write(dir.join(EVENTS_DIR).join(name), json_text(e)).unwrap();
        };
        let mut on_task = event(
            "2026-10-13T10:00:00Z",
            Kind::Task,
            Some("claude"),
            Some("claude_app"),
            true,
        );
        put("2026-10-13-100000-000-aaaaaa.json", &on_task);
        // A newer change by the same name on another task, recording no agent, counts as seen;
        // the agent is the newest one recorded.
        on_task.at = "2026-10-14T09:00:00Z".into();
        on_task.id = Some("A01-02".into());
        on_task.agent = None;
        on_task.tool = None;
        put("2026-10-14-090000-000-aaaaaa.json", &on_task);
        // Someone who never touched this task is not listed.
        let mut other = event("2026-10-15T09:00:00Z", Kind::Task, None, None, true);
        other.by = Some("codex/x".into());
        other.id = Some("A01-02".into());
        put("2026-10-15-090000-000-aaaaaa.json", &other);
        assert_eq!(
            history(&dir, "A01-01").seen,
            vec![Seen {
                by: "s".into(),
                at: "2026-10-14T09:00:00Z".into(),
                agent: Some("claude".into()),
                tool: Some("claude_app".into()),
            }]
        );
        assert_eq!(history(&dir, "A01-09").seen, Vec::new());
    }

    #[test]
    fn the_last_agent_is_the_newest_task_change_that_records_one() {
        let dir = std::env::temp_dir().join(format!(
            "allomos-test-last-agent-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        assert_eq!(last_agent(&dir), None);
        let put = |name: &str, e: &Event| {
            fs::create_dir_all(dir.join(EVENTS_DIR)).unwrap();
            fs::write(dir.join(EVENTS_DIR).join(name), json_text(e)).unwrap();
        };
        // Older events record no agent: none found.
        put(
            "2026-10-13-100000-000-aaaaaa.json",
            &event("2026-10-13T10:00:00Z", Kind::Task, None, None, true),
        );
        assert_eq!(last_agent(&dir), None);
        put(
            "2026-10-14-100000-000-aaaaaa.json",
            &event(
                "2026-10-14T10:00:00Z",
                Kind::Task,
                Some("claude"),
                Some("claude_cli"),
                true,
            ),
        );
        let claude = Some(LastAgent {
            agent: "claude".into(),
            tool: Some("claude_cli".into()),
            at: "2026-10-14T10:00:00Z".into(),
        });
        assert_eq!(last_agent(&dir), claude);
        // Newer events that changed no task, or record no agent, do not count.
        put(
            "2026-10-15-100000-000-aaaaaa.json",
            &event(
                "2026-10-15T10:00:00Z",
                Kind::Area,
                Some("codex"),
                None,
                true,
            ),
        );
        put(
            "2026-10-15-110000-000-aaaaaa.json",
            &event(
                "2026-10-15T11:00:00Z",
                Kind::Task,
                Some("codex"),
                None,
                false,
            ),
        );
        put(
            "2026-10-15-120000-000-aaaaaa.json",
            &event("2026-10-15T12:00:00Z", Kind::Task, None, None, true),
        );
        assert_eq!(last_agent(&dir), claude);
        put(
            "2026-10-16-100000-000-aaaaaa.json",
            &event(
                "2026-10-16T10:00:00Z",
                Kind::Task,
                Some("codex"),
                None,
                true,
            ),
        );
        assert_eq!(
            last_agent(&dir).map(|a| (a.agent, a.tool)),
            Some(("codex".into(), None))
        );
    }

    #[test]
    fn suffixes_are_six_letters_or_digits() {
        let s = suffix();
        assert_eq!(s.len(), 6);
        assert!(
            s.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        );
    }
}
