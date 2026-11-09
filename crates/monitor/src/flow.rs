//! One monitoring run's state and the rules it follows (Task-A03-cow112, moved from the
//! interface's `monitorFlow.ts`): which ready task each free agent gets, following each session's
//! task in the taskbag until it is finished, and when to tell the person the run is stuck. No
//! input or output here: the loop ([`crate::runner`]) reads the taskbag and starts sessions.

use std::collections::BTreeMap;

use allomos_core::{State, Task, Taskbag, difficulty::Range, heartbeat, ops};
use serde::{Deserialize, Serialize};

use crate::session_tab::SessionTab;

/// How often to check when no task is ready, in minutes: these seven only (approver, 2026-11-04).
pub const FREQUENCIES: [u32; 7] = [1, 5, 10, 30, 60, 90, 120];
/// The frequency a run starts with when none is given.
pub const FIRST_FREQUENCY: u32 = 30;
/// A session with no sign of life this long is shown as stalled; Allomos never ends it.
pub const STALL_MS: i64 = 30 * 60 * 1000;
/// The agents whose sessions a monitor starts itself, in a terminal.
pub const TERMINAL_AGENTS: [&str; 3] = ["claude_cli", "codex_cli", "antigravity_cli"];

/// Which tasks the run takes: every ready one, or one iteration's.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    All,
    Iteration { id: String },
}

/// A session working a task for the run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Working {
    /// The agent whose session it is.
    pub agent: String,
    /// The task.
    pub id: String,
    /// The session ID Allomos gave the agent (Claude Code's `--session-id`); none when the agent
    /// picks its own.
    pub session_id: Option<String>,
    /// The tab it runs in, when Allomos can close it.
    pub tab: Option<SessionTab>,
    /// When it was started (ms since 1970).
    pub at: i64,
    /// Seen held by someone since it was started: it was claimed.
    pub seen_held: bool,
    /// The task as last seen (what changes when the session works it), and since when (ms).
    pub seen: String,
    pub changed: i64,
}

/// One monitoring run, kept on this computer only, never in the taskbag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    pub scope: Scope,
    /// The agents ready tasks are given to.
    pub agents: Vec<String>,
    /// Each agent's difficulty range (agent ID → `1-3`, `4`, as `alm next --difficulty` takes
    /// it). An agent not listed takes every level, 1 to 5 (Task-A03-wave335).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub difficulties: BTreeMap<String, String>,
    /// The agents with a free session slot that no ready task in their range fitted at the last
    /// check: the status bar says why they got no session (Task-A03-wave335).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub no_task: Vec<String>,
    /// How many sessions each agent may have at work at once (agent ID → count, at least 1). An
    /// agent not listed has one (Task-A03-weasel276).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub sessions: BTreeMap<String, u32>,
    /// The agents that could be chosen when the run started (the app offers them again).
    pub offered: Vec<String>,
    pub quiet: bool,
    /// Minutes between checks when no task is ready.
    pub every: u32,
    /// When it started (ms).
    pub started: i64,
    /// The tasks it started a session for, in order.
    pub taken: Vec<String>,
    /// The agent each of those tasks was given to (task ID → agent ID).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub taken_by: BTreeMap<String, String>,
    /// The sessions working a task now.
    pub working: Vec<Working>,
    /// Stop asked while tasks are worked: the run ends when they are all finished.
    pub stopping: bool,
    /// When it last checked for a ready task (ms).
    pub checked: i64,
    /// What the last sweep (at the start and on Check now) let go on because their blockers had
    /// finished, until the next check: the app's line under the status bar.
    #[serde(default)]
    pub unblocked: Vec<Unblocked>,
}

/// A task a sweep let go on, and the state it went back to (`ready`, or `in_progress` when
/// someone holds it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unblocked {
    pub id: String,
    pub state: String,
}

/// What the monitor tells the person when it is stuck: a task waiting on them. (A quiet monitor
/// with no ready task says nothing: its status bar shows it, A03-stork103.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stuck {
    /// A task its session parked waiting on the person.
    Waiting { id: String, on: String },
}

impl Run {
    /// A new run; it checks at once.
    pub fn start(
        scope: Scope,
        agents: Vec<String>,
        offered: Vec<String>,
        quiet: bool,
        every: u32,
        now: i64,
    ) -> Run {
        Run {
            scope,
            agents,
            offered,
            quiet,
            every,
            started: now,
            sessions: BTreeMap::new(),
            difficulties: BTreeMap::new(),
            no_task: Vec::new(),
            taken: Vec::new(),
            taken_by: BTreeMap::new(),
            working: Vec::new(),
            stopping: false,
            checked: now - i64::from(every) * 60_000,
            unblocked: Vec::new(),
        }
    }

    /// When the next check is due (ms).
    pub fn next_check(&self) -> i64 {
        self.checked + i64::from(self.every) * 60_000
    }

    /// How many sessions `agent` may have at work at once: one unless the person set more.
    pub fn sessions_of(&self, agent: &str) -> u32 {
        self.sessions.get(agent).copied().unwrap_or(1).max(1)
    }

    /// The count for `agent` from the next check on (Task-A03-weasel276): at least one, with no
    /// upper limit. A lower count lets the sessions at work finish; none is interrupted.
    pub fn set_sessions(&mut self, agent: &str, count: u32) {
        if count >= 1 {
            self.sessions.insert(agent.to_string(), count);
        }
    }

    /// The difficulty range `agent` takes tasks from: every level unless the person chose one
    /// (Task-A03-wave335). A stored range that does not read is taken as every level.
    pub fn range_of(&self, agent: &str) -> Range {
        self.difficulties
            .get(agent)
            .and_then(|r| Range::parse(r).ok())
            .unwrap_or_else(Range::all)
    }

    /// The range `agent` takes tasks from, from the next check on (Task-A03-wave335). A range
    /// that does not read is left as it is.
    pub fn set_difficulty(&mut self, agent: &str, range: &str) {
        if let Ok(r) = Range::parse(range) {
            self.difficulties.insert(agent.to_string(), r.text());
        }
    }

    /// The free session slots: each agent once for every session it may still start (its count
    /// less the sessions at work), in the run's order.
    pub fn free_agents(&self) -> Vec<String> {
        let mut free = Vec::new();
        for agent in &self.agents {
            let at_work = self.working.iter().filter(|w| &w.agent == agent).count() as u32;
            for _ in at_work..self.sessions_of(agent) {
                free.push(agent.clone());
            }
        }
        free
    }

    /// These agents from now on: one added gets tasks from the next check; one taken off is
    /// given no more, while its session finishes. An empty list leaves the run as it is.
    pub fn set_agents(&mut self, agents: &[String]) {
        if agents.is_empty() {
            return;
        }
        let mut kept: Vec<String> = self
            .offered
            .iter()
            .filter(|a| agents.contains(a))
            .cloned()
            .collect();
        for a in agents {
            if !kept.contains(a) {
                kept.push(a.clone());
            }
        }
        self.agents = kept;
    }

    /// Follow every session's task in the taskbag as it is now; returns the sessions whose
    /// tasks are finished, in the order they were started.
    pub fn follow(&mut self, tasks: &[Task], now: i64) -> Vec<Working> {
        let mut finished = Vec::new();
        let mut working = Vec::new();
        for cur in self.working.drain(..) {
            match follow_one(&cur, tasks, now) {
                Some(next) => working.push(next),
                None => finished.push(cur),
            }
        }
        self.working = working;
        finished
    }

    /// One check's share-out: each free session slot, in the run's order, gets the first ready
    /// task in the scope this run has not taken yet that is in its agent's difficulty range; a
    /// slot with no such task gets none (see [`Run::unfilled`]). Every session claims its task
    /// first, so two sessions never work the same one.
    pub fn share(&self, bag: &Taskbag, now_secs: i64) -> Vec<(String, Task)> {
        let iteration = match &self.scope {
            Scope::All => None,
            Scope::Iteration { id } => Some(id.as_str()),
        };
        let mut ready: Vec<&Task> = ops::next(bag, iteration)
            .unwrap_or_default()
            .into_iter()
            .filter(|t| !self.taken.contains(&t.id))
            .collect();
        // A task whose session went quiet (its account ran out, or it was switched) is lost
        // (decision 0022): a new session takes it over, after the ready tasks.
        let lost_after = heartbeat::applied(&bag.project).lost_after_secs;
        for t in &bag.tasks {
            if self.taken.contains(&t.id) || iteration.is_some_and(|i| t.iteration.as_deref() != Some(i)) {
                continue;
            }
            if heartbeat::lost(t, now_secs, lost_after, None).is_some() {
                ready.push(t);
            }
        }
        let mut given = vec![false; ready.len()];
        let mut shares = Vec::new();
        for agent in self.free_agents() {
            let range = self.range_of(&agent);
            let found = ready
                .iter()
                .enumerate()
                .position(|(i, t)| !given[i] && range.holds(t));
            if let Some(i) = found {
                given[i] = true;
                shares.push((agent, ready[i].clone()));
            }
        }
        shares
    }

    /// The agents with a free session slot that `shares` left without a task: no ready task in
    /// their range was left for them (Task-A03-wave335). Each agent once.
    pub fn unfilled(&self, shares: &[(String, Task)]) -> Vec<String> {
        let mut left = self.free_agents();
        for (agent, _) in shares {
            if let Some(i) = left.iter().position(|a| a == agent) {
                left.remove(i);
            }
        }
        let mut out: Vec<String> = Vec::new();
        for agent in left {
            if !out.contains(&agent) {
                out.push(agent);
            }
        }
        out
    }

    /// A session of `agent` started for `task`.
    pub fn take(
        &mut self,
        agent: &str,
        task: &Task,
        session_id: Option<String>,
        tab: Option<SessionTab>,
        now: i64,
    ) {
        self.taken.push(task.id.clone());
        self.taken_by.insert(task.id.clone(), agent.to_string());
        self.working.push(Working {
            agent: agent.to_string(),
            id: task.id.clone(),
            session_id,
            tab,
            at: now,
            seen_held: task.holder.is_some(),
            seen: seen_of(Some(task)),
            changed: now,
        });
        self.checked = now;
    }

    /// The sessions that have shown no sign of life for [`STALL_MS`].
    pub fn stalled(&self, now: i64) -> Vec<&Working> {
        self.working
            .iter()
            .filter(|w| now - w.changed >= STALL_MS)
            .collect()
    }
}

/// What of a task its session changes as it works it.
fn seen_of(task: Option<&Task>) -> String {
    task.map(|t| {
        serde_json::json!([
            t.state.as_str(),
            t.holder,
            t.labels,
            t.handoff_note,
            t.waiting_on
        ])
        .to_string()
    })
    .unwrap_or_default()
}

/// One session's task as it is now: none when it is finished (done, dropped, waiting or
/// blocked; held by no one after being claimed; left `ready` without ever being held; gone).
fn follow_one(cur: &Working, tasks: &[Task], now: i64) -> Option<Working> {
    let task = tasks.iter().find(|t| t.id == cur.id);
    let seen = seen_of(task);
    let seen_held = cur.seen_held || task.is_some_and(|t| t.holder.is_some());
    let finished = match task {
        None => true,
        Some(t) => {
            matches!(
                t.state,
                State::Done | State::Dropped | State::Waiting | State::Blocked
            ) || (t.holder.is_none() && (seen_held || t.state != State::Ready))
        }
    };
    if finished {
        return None;
    }
    let mut next = cur.clone();
    if seen != cur.seen {
        next.changed = now;
    }
    next.seen = seen;
    next.seen_held = seen_held;
    Some(next)
}

/// The tab to close now that a session's task is finished: only when the task is **done**
/// (approver, 2026-11-08); any other ending leaves it open, so the person can see why.
pub fn tab_to_close<'a>(cur: &'a Working, tasks: &[Task]) -> Option<&'a SessionTab> {
    let done = tasks
        .iter()
        .any(|t| t.id == cur.id && t.state == State::Done);
    cur.tab.as_ref().filter(|_| done)
}

/// A finished task waiting on the person (not on someone named): tell them.
pub fn waiting_alert(id: &str, tasks: &[Task]) -> Option<Stuck> {
    let task = tasks.iter().find(|t| t.id == id)?;
    (task.state == State::Waiting && task.who.is_none()).then(|| Stuck::Waiting {
        id: task.id.clone(),
        on: task.waiting_on.clone().unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, state: State, holder: Option<&str>) -> Task {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": id, "title": id, "area": "A01", "state": "ready"
        }))
        .unwrap();
        t.state = state;
        t.holder = holder.map(String::from);
        t
    }

    fn run(agents: &[&str]) -> Run {
        let agents: Vec<String> = agents.iter().map(|a| a.to_string()).collect();
        Run::start(Scope::All, agents.clone(), agents, true, 30, 1_000_000)
    }

    /// Task-A03 (account switch): a task its old session held, last heartbeat at `at` (UTC).
    fn held_since(id: &str, holder: &str, at: &str) -> Task {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": id, "area": "A01", "state": "in_progress",
            "holder": holder, "heartbeat": { "at": at, "by": holder }
        }))
        .unwrap()
    }

    /// A task held by a quiet session (its heartbeat is over the limit) is offered again, so a
    /// new session takes it over; one still beating is not.
    #[test]
    fn a_lost_task_is_offered_to_a_new_session() {
        let r = run(&["claude_cli"]);
        let beat = heartbeat::parse_utc("2026-11-09T10:00:00Z").unwrap();
        let bag = bag_of(vec![held_since("A01-9", "old/session", "2026-11-09T10:00:00Z")]);
        let quiet = r.share(&bag, beat + 2 * 3600);
        assert_eq!(quiet.len(), 1);
        assert_eq!(quiet[0].1.id, "A01-9");
        assert!(r.share(&bag, beat + 5 * 60).is_empty());
    }

    #[test]
    fn a_new_run_checks_at_once() {
        let r = run(&["claude_cli"]);
        assert_eq!(r.next_check(), 1_000_000);
    }

    #[test]
    fn a_session_is_finished_when_its_task_is_let_go() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.take(
            "claude_cli",
            &task("A01-1", State::Ready, None),
            None,
            None,
            10,
        );
        r.take(
            "codex_cli",
            &task("A01-2", State::Ready, None),
            None,
            None,
            10,
        );
        assert!(r.free_agents().is_empty());
        // Claimed: still working.
        let tasks = [
            task("A01-1", State::InProgress, Some("s1")),
            task("A01-2", State::Ready, None),
        ];
        assert!(r.follow(&tasks, 20).is_empty());
        assert!(r.working[0].seen_held);
        assert_eq!(r.working[0].changed, 20);
        // Released after the claim, and done: both finished.
        let tasks = [
            task("A01-1", State::Ready, None),
            task("A01-2", State::Done, Some("s2")),
        ];
        let finished = r.follow(&tasks, 30);
        let ids: Vec<&str> = finished.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["A01-1", "A01-2"]);
        // Never claimed and still ready: still waiting for its session.
        r.take(
            "claude_cli",
            &task("A01-3", State::Ready, None),
            None,
            None,
            40,
        );
        assert!(
            r.follow(&[task("A01-3", State::Ready, None)], 50)
                .is_empty()
        );
        assert!(r.follow(&[], 60).len() == 1);
        assert_eq!(r.free_agents(), ["claude_cli", "codex_cli"]);
    }

    #[test]
    fn an_agent_may_have_several_sessions_at_once() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        assert_eq!(r.sessions_of("claude_cli"), 1);
        r.set_sessions("claude_cli", 2);
        assert_eq!(r.free_agents(), ["claude_cli", "claude_cli", "codex_cli"]);
        r.take(
            "claude_cli",
            &task("A01-1", State::Ready, None),
            None,
            None,
            10,
        );
        assert_eq!(r.free_agents(), ["claude_cli", "codex_cli"]);
        r.take(
            "claude_cli",
            &task("A01-2", State::Ready, None),
            None,
            None,
            10,
        );
        assert_eq!(r.free_agents(), ["codex_cli"]);
        assert_eq!(
            r.taken_by.get("A01-2").map(String::as_str),
            Some("claude_cli")
        );
        // Lowered to one: the two at work are not interrupted, and no new one starts until fewer
        // than one is at work.
        r.set_sessions("claude_cli", 1);
        assert_eq!(r.free_agents(), ["codex_cli"]);
        let tasks = [
            task("A01-1", State::Done, Some("s1")),
            task("A01-2", State::InProgress, Some("s2")),
        ];
        assert_eq!(r.follow(&tasks, 20).len(), 1);
        assert_eq!(r.free_agents(), ["codex_cli"]);
        // A count below one is ignored.
        r.set_sessions("claude_cli", 0);
        assert_eq!(r.sessions_of("claude_cli"), 1);
    }

    #[test]
    fn only_a_done_task_closes_its_tab() {
        let tab = SessionTab {
            window: 1,
            tty: "/dev/ttys001".into(),
        };
        let mut r = run(&["claude_cli"]);
        r.take(
            "claude_cli",
            &task("A01-1", State::Ready, None),
            None,
            Some(tab),
            0,
        );
        let w = r.working[0].clone();
        assert!(tab_to_close(&w, &[task("A01-1", State::Done, Some("s"))]).is_some());
        assert!(tab_to_close(&w, &[task("A01-1", State::Waiting, Some("s"))]).is_none());
        let mut waiting = task("A01-1", State::Waiting, Some("s"));
        waiting.waiting_on = Some("which colour?".into());
        assert_eq!(
            waiting_alert("A01-1", &[waiting.clone()]),
            Some(Stuck::Waiting {
                id: "A01-1".into(),
                on: "which colour?".into()
            })
        );
        waiting.who = Some("the designer".into());
        assert_eq!(waiting_alert("A01-1", &[waiting]), None);
    }

    #[test]
    fn agents_taken_off_keep_their_session() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.take(
            "codex_cli",
            &task("A01-1", State::Ready, None),
            None,
            None,
            0,
        );
        r.set_agents(&["claude_cli".to_string()]);
        assert_eq!(r.agents, ["claude_cli"]);
        assert_eq!(r.working.len(), 1);
        r.set_agents(&[]);
        assert_eq!(r.agents, ["claude_cli"]);
    }

    #[test]
    fn each_agent_has_every_level_unless_it_is_given_a_range() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        assert_eq!(r.range_of("claude_cli"), Range::all());
        r.set_difficulty("claude_cli", "3-5");
        r.set_difficulty("codex_cli", "1-2");
        assert_eq!(r.difficulties["claude_cli"], "3-5");
        assert_eq!(r.range_of("codex_cli"), Range { low: 1, high: 2 });
        // A range that does not read is left as it was.
        r.set_difficulty("codex_cli", "4-2");
        assert_eq!(r.difficulties["codex_cli"], "1-2");
        r.set_difficulty("claude_cli", "4");
        assert_eq!(r.difficulties["claude_cli"], "4");
    }

    /// A ready task of level `level` (as `task.json` writes it).
    fn leveled(id: &str, level: i64) -> Task {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": id, "area": "A01", "state": "ready", "difficulty": level
        }))
        .unwrap()
    }

    /// A taskbag of these tasks, one area and no iterations.
    fn bag_of(tasks: Vec<Task>) -> Taskbag {
        Taskbag {
            project: serde_json::from_value(serde_json::json!({
                "format": "allomos/7", "id": "P", "name": "P"
            }))
            .unwrap(),
            areas: vec![
                serde_json::from_value(serde_json::json!({
                    "id": "A01", "name": "A01", "order": 1
                }))
                .unwrap(),
            ],
            iterations: vec![],
            tasks,
        }
    }

    #[test]
    fn each_slot_gets_the_first_ready_task_in_its_agents_range() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.set_difficulty("claude_cli", "4-5");
        r.set_difficulty("codex_cli", "1-2");
        let bag = bag_of(vec![
            leveled("A01-1", 1),
            leveled("A01-2", 5),
            leveled("A01-3", 2),
        ]);
        let shares: Vec<(String, String)> =
            r.share(&bag, 0).into_iter().map(|(a, t)| (a, t.id)).collect();
        // Claude takes the first task of 4-5 (A01-2); codex the first of 1-2 (A01-1).
        assert_eq!(
            shares,
            [
                ("claude_cli".to_string(), "A01-2".to_string()),
                ("codex_cli".to_string(), "A01-1".to_string())
            ]
        );
        // No task is in claude's 4-5: it gets none and is named; codex takes the first of 1-2.
        let bag = bag_of(vec![leveled("A01-1", 1), leveled("A01-3", 2)]);
        let shares = r.share(&bag, 0);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].1.id, "A01-1");
        assert_eq!(r.unfilled(&shares), ["claude_cli"]);
    }

    #[test]
    fn a_slot_with_no_task_in_its_range_is_named() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.set_difficulty("claude_cli", "5");
        // Both agents have a free slot; no task was given to either: both are named, once each.
        assert_eq!(r.unfilled(&[]), ["claude_cli", "codex_cli"]);
        // Give codex its task: only claude is left without one.
        let shares = vec![("codex_cli".to_string(), task("A01-1", State::Ready, None))];
        assert_eq!(r.unfilled(&shares), ["claude_cli"]);
        // A session at work takes its slot away from the free ones.
        r.take(
            "codex_cli",
            &task("A01-1", State::Ready, None),
            None,
            None,
            1,
        );
        assert_eq!(r.unfilled(&[]), ["claude_cli"]);
    }

    #[test]
    fn a_run_round_trips_through_its_file() {
        let mut r = run(&["claude_cli"]);
        r.scope = Scope::Iteration {
            id: "I-sand".into(),
        };
        let text = serde_json::to_string(&r).unwrap();
        assert!(text.contains(r#""scope":{"kind":"iteration","id":"I-sand"}"#));
        assert_eq!(serde_json::from_str::<Run>(&text).unwrap(), r);
    }
}
