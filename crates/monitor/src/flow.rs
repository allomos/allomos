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
/// A session with no sign of life this long is shown as stalled; Allomos never ends it. A session
/// whose task it holds and beats for is judged by its heartbeat instead (A03-plum984): see
/// [`Working::changed`].
pub const STALL_MS: i64 = 30 * 60 * 1000;
/// A session that has not claimed its task this long after it was started did not start
/// (A03-otter944): a session that works claims its task within a minute or two.
pub const NOT_STARTED_MS: i64 = 10 * 60 * 1000;
/// How often an agent paused because its quota is used up has its quota read again.
pub const QUOTA_RECHECK_MS: i64 = 10 * 60 * 1000;
/// How many sessions that did not start the run keeps, for the panel (the newest).
pub const NOT_STARTED_KEPT: usize = 10;
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
    /// The task as last seen (what changes when the session works it), and since when (ms): the
    /// last sign of life [`Run::stalled`] looks at. While the session holds its task and beats for
    /// it, its heartbeat decides instead, as for `alm next` (A03-plum984, [`heartbeat::lost`]):
    /// not lost, `changed` is the time it was last followed; lost, `changed` is moved back far
    /// enough for the session to show as stalled.
    pub seen: String,
    pub changed: i64,
    /// The holder's name once the task was seen held by this session (A03-cart761): a later
    /// holder of another name is another session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holder: Option<String>,
    /// Who held the task when this session was started for it (a lost task's former holder,
    /// A03-husky861), and that holder's session ID: while they still hold it, the task is not
    /// this session's claim (see [`prior_holds`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prior_session: Option<String>,
    /// Since when (ms) this session shows a confirmation and waits for the person
    /// (Task-A03-candle790, [`crate::confirm`]): it does not take a slot, it is not lost while it
    /// waits (it sends no heartbeat), and its task stays its own, so no other session is given it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub awaiting_approval: Option<i64>,
}

/// A session the run stopped following because its task went lost while it held it
/// (A03-husky861): no heartbeat and no busy window for longer than the project's limit, as for
/// `alm next`. Its slot is free and its task is given out again; Allomos does not end it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LostSession {
    pub agent: String,
    pub id: String,
    /// Its `--as` name.
    pub holder: String,
    /// When it was started, and when it was found lost (ms).
    pub at: i64,
    pub found: i64,
    /// Its tab said its quota is used up.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub quota: bool,
    /// When that quota comes back, as the agent wrote it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets: Option<String>,
}

/// How many lost sessions a run keeps to show: the newest.
pub const LOST_KEPT: usize = 10;
/// How many times one task's sessions may fail in a run (lost, or did not start) before the run
/// gives it up and tells the person (A03-husky861).
pub const MAX_FAILURES: u32 = 3;

/// A task the run stopped following because another session holds it (A03-cart761): its row is
/// gone and its slot is free; the panel says why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LetGo {
    /// The task.
    pub id: String,
    /// The agent whose session the run had started for it.
    pub agent: String,
    /// Who holds it now (the other session's `--as` name).
    pub holder: String,
    /// When the run let it go (ms).
    pub at: i64,
}

/// How many let-go tasks a run keeps to show: the latest ones.
pub const LET_GO_KEPT: usize = 5;

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
    /// The model each agent's sessions start with (agent ID → a value from the online model
    /// list, A03-alto709). An agent not listed starts with its own setting: no model is given.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<String, String>,
    /// The reasoning effort each agent's sessions start with, the same way (A03-alto709).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub efforts: BTreeMap<String, String>,
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
    /// Tabs of finished tasks whose 30-second countdown to close them has not ended yet. Kept in
    /// the run's file so a run that resumes after it was ended early starts the countdown again
    /// (A03-pocket537).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub closing: Vec<Closing>,
    /// The taskbag cannot be read now and the person has been told (Task-A03-raven398): a
    /// [`crate::unreadable::Why`] code. The run goes on and clears it when it reads it again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<String>,
    /// The tasks it let go because another session holds them, the latest [`LET_GO_KEPT`]
    /// (A03-cart761). They stay in `taken`, so the run does not start a session for them again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub let_go: Vec<LetGo>,
    /// The agents given no session for now, and why (agent ID → pause; A03-otter944): their
    /// quota is used up, or their last session did not start.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub paused: BTreeMap<String, Paused>,
    /// The sessions that did not start (they never claimed their task), newest last
    /// (A03-otter944). Their tasks are given out again.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub not_started: Vec<NotStarted>,
    /// The sessions found lost, newest last, the latest [`LOST_KEPT`] (A03-husky861).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lost: Vec<LostSession>,
    /// How many times each task's sessions failed this run (task ID → count).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub failures: BTreeMap<String, u32>,
    /// The tasks given up after [`MAX_FAILURES`]: no more sessions for them this run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub given_up: Vec<String>,
}

/// The model and reasoning effort a session is started with (A03-alto709). None: the agent's own
/// setting, so its command is the same as without a choice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Choice {
    pub model: Option<String>,
    pub effort: Option<String>,
}

/// A model or effort value is one plain word (letters, digits, `.`, `-`, `_`, `:`, `/`, `[`, `]`),
/// at most 80 characters: it goes into the command that starts the agent.
pub fn plain_choice(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-_:/[]".contains(c))
}

/// Why an agent gets no session for now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PauseWhy {
    /// Its quota is used up: read again every [`QUOTA_RECHECK_MS`] until there is some.
    Quota,
    /// Its last session did not claim its task, for no reason the monitor could read: tried again
    /// at the next check.
    NoClaim,
}

/// An agent given no session until `until` (ms).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Paused {
    pub why: PauseWhy,
    /// When its quota comes back, as the agent wrote it, when it said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets: Option<String>,
    /// Since when (ms).
    pub since: i64,
    /// When it is looked at again (ms).
    pub until: i64,
}

/// A session that did not start: it never claimed its task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotStarted {
    pub agent: String,
    pub id: String,
    /// When it was started, and when it was found not to have started (ms).
    pub at: i64,
    pub found: i64,
    /// Why, when it could be read: `quota`; else `no_claim`.
    pub why: PauseWhy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resets: Option<String>,
}

/// A finished task's tab waiting to be closed (see [`Run::closing`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Closing {
    /// The agent whose session the tab shows.
    pub agent: String,
    pub tab: SessionTab,
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
            models: BTreeMap::new(),
            efforts: BTreeMap::new(),
            no_task: Vec::new(),
            taken: Vec::new(),
            taken_by: BTreeMap::new(),
            working: Vec::new(),
            stopping: false,
            checked: now - i64::from(every) * 60_000,
            closing: Vec::new(),
            unreadable: None,
            let_go: Vec::new(),
            paused: BTreeMap::new(),
            not_started: Vec::new(),
            lost: Vec::new(),
            failures: BTreeMap::new(),
            given_up: Vec::new(),
        }
    }

    /// One more failure of `id`'s sessions; at [`MAX_FAILURES`] the task is given up.
    fn failed(&mut self, id: &str) {
        let n = self.failures.entry(id.to_string()).or_insert(0);
        *n += 1;
        if *n >= MAX_FAILURES && !self.given_up.iter().any(|g| g == id) {
            self.given_up.push(id.to_string());
        }
    }

    /// The sessions whose task went lost while they held it (A03-husky861): the same test as
    /// `alm next` ([`heartbeat::lost`], the project's `lost_after_secs`). They leave the sessions at
    /// work, which frees their slots, and their tasks leave the taken ones, so the next share
    /// offers them first; returned in the order started. Nothing is done to the sessions.
    pub fn lost_ones(&mut self, tasks: &[Task], lost_after_secs: i64, now: i64) -> Vec<Working> {
        let (gone, kept): (Vec<Working>, Vec<Working>) = self.working.drain(..).partition(|w| {
            tasks.iter().find(|t| t.id == w.id).is_some_and(|t| {
                w.seen_held
                    && w.awaiting_approval.is_none()
                    && !prior_holds(w, t)
                    && others(w, t).is_none()
                    && beats_for(t)
                    && heartbeat::lost(t, now.div_euclid(1000), lost_after_secs, None).is_some()
            })
        });
        self.working = kept;
        for w in &gone {
            self.taken.retain(|id| id != &w.id);
            self.taken_by.remove(&w.id);
            self.failed(&w.id);
        }
        gone
    }

    /// `w` went lost (its tab said its quota is used up, when `quota`): kept for the panel. Its
    /// agent is paused by the caller when its quota is used up.
    pub fn went_lost(&mut self, w: &Working, quota: bool, resets: Option<String>, now: i64) {
        self.lost.push(LostSession {
            agent: w.agent.clone(),
            id: w.id.clone(),
            holder: w.holder.clone().unwrap_or_default(),
            at: w.at,
            found: now,
            quota,
            resets,
        });
        let over = self.lost.len().saturating_sub(LOST_KEPT);
        self.lost.drain(..over);
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

    /// The model and the reasoning effort `agent`'s sessions start with (A03-alto709): none for
    /// either is the agent's own setting.
    pub fn choice_of(&self, agent: &str) -> Choice {
        Choice {
            model: self.models.get(agent).cloned(),
            effort: self.efforts.get(agent).cloned(),
        }
    }

    /// Start `agent`'s sessions with this model and effort (A03-alto709); none is its own setting.
    /// A value that is not a plain word ([`plain_choice`]) is left out.
    pub fn set_choice(&mut self, agent: &str, choice: &Choice) {
        for (map, value) in [
            (&mut self.models, &choice.model),
            (&mut self.efforts, &choice.effort),
        ] {
            match value.as_deref().map(str::trim) {
                Some(v) if plain_choice(v) => {
                    map.insert(agent.to_string(), v.to_string());
                }
                _ => {
                    map.remove(agent);
                }
            }
        }
    }

    /// Which tasks the run takes from the next check on (A03-yolk481). The sessions at work
    /// finish their tasks; a task the run already took is not given again. An iteration with no
    /// ID is ignored.
    pub fn set_scope(&mut self, scope: Scope) {
        if matches!(&scope, Scope::Iteration { id } if id.trim().is_empty()) {
            return;
        }
        self.scope = scope;
    }

    /// The free session slots: each agent once for every session it may still start (its count
    /// less the sessions at work), in the run's order.
    pub fn free_agents(&self) -> Vec<String> {
        let mut free = Vec::new();
        for agent in self.agents.iter().filter(|a| !self.paused.contains_key(*a)) {
            // A session waiting at a confirmation leaves its slot free (Task-A03-candle790); once
            // it goes on, the agent may have one session more than its count for a while.
            let at_work = self
                .working
                .iter()
                .filter(|w| &w.agent == agent && w.awaiting_approval.is_none())
                .count() as u32;
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
    /// tasks are finished, in the order they were started. A task another session holds is let
    /// go instead (A03-cart761): it leaves the sessions at work, which frees its slot, and is
    /// noted in [`Run::let_go`]; it is not among the finished ones (its tab stays open).
    /// `lost_after_secs`: the project's limit for a held task's heartbeat
    /// (`heartbeat::applied(..).lost_after_secs`).
    pub fn follow(&mut self, tasks: &[Task], lost_after_secs: i64, now: i64) -> Vec<Working> {
        let mut finished = Vec::new();
        let mut working = Vec::new();
        for cur in self.working.drain(..) {
            match follow_one(&cur, tasks, lost_after_secs, now) {
                Followed::Working(next) => working.push(next),
                Followed::Finished => finished.push(cur),
                Followed::Others(holder) => self.let_go.push(LetGo {
                    id: cur.id,
                    agent: cur.agent,
                    holder,
                    at: now,
                }),
            }
        }
        if self.let_go.len() > LET_GO_KEPT {
            self.let_go.drain(..self.let_go.len() - LET_GO_KEPT);
        }
        self.working = working;
        finished
    }

    /// The sessions that did not start (A03-otter944): started [`NOT_STARTED_MS`] ago or more and
    /// their task never claimed by them: still ready and held by no one, or still held by the
    /// former holder it had when the session was started (A03-husky861). They are taken off the
    /// sessions at work, and their tasks off the taken ones, so they are given out again (until
    /// [`MAX_FAILURES`]); returned in the order started.
    pub fn unclaimed(&mut self, tasks: &[Task], now: i64) -> Vec<Working> {
        let (gone, kept): (Vec<Working>, Vec<Working>) = self.working.drain(..).partition(|w| {
            let unclaimed = tasks.iter().any(|t| {
                t.id == w.id
                    && ((t.state == State::Ready && t.holder.is_none()) || prior_holds(w, t))
            });
            !w.seen_held && unclaimed && now - w.at >= NOT_STARTED_MS
        });
        self.working = kept;
        for w in &gone {
            self.taken.retain(|id| id != &w.id);
            self.taken_by.remove(&w.id);
            self.failed(&w.id);
        }
        gone
    }

    /// `w` did not start, for `why` (with the quota's reset time, when read): it is kept for the
    /// panel, and its agent gets no session until it is looked at again.
    pub fn did_not_start(&mut self, w: &Working, why: PauseWhy, resets: Option<String>, now: i64) {
        self.not_started.push(NotStarted {
            agent: w.agent.clone(),
            id: w.id.clone(),
            at: w.at,
            found: now,
            why,
            resets: resets.clone(),
        });
        let over = self.not_started.len().saturating_sub(NOT_STARTED_KEPT);
        self.not_started.drain(..over);
        self.pause(&w.agent, why, resets, now);
    }

    /// `agent` gets no session for now: until its quota is read again, or (no reason read) until
    /// the next check.
    pub fn pause(&mut self, agent: &str, why: PauseWhy, resets: Option<String>, now: i64) {
        let until = match why {
            PauseWhy::Quota => now + QUOTA_RECHECK_MS,
            PauseWhy::NoClaim => now + i64::from(self.every) * 60_000,
        };
        let since = self.paused.get(agent).map_or(now, |p| p.since);
        self.paused.insert(
            agent.to_string(),
            Paused {
                why,
                resets,
                since,
                until,
            },
        );
    }

    /// The paused agents due to be looked at again (`until` reached), in the run's order.
    pub fn pauses_due(&self, now: i64) -> Vec<(String, PauseWhy)> {
        self.paused
            .iter()
            .filter(|(_, p)| now >= p.until)
            .map(|(a, p)| (a.clone(), p.why))
            .collect()
    }

    /// `agent` gets sessions again.
    pub fn resume(&mut self, agent: &str) {
        self.paused.remove(agent);
    }

    /// One check's share-out: each free session slot, in the run's order, gets the first task in
    /// its agent's difficulty range: first the lost tasks in the scope (A03-husky861: a session
    /// went quiet holding them, decision 0022), then the ready ones this run has not taken yet; a
    /// slot with no such task gets none (see [`Run::unfilled`]). A lost task is offered even when
    /// this run took it before; only one a session of the run works now, or one given up
    /// ([`MAX_FAILURES`]), is not. Every session claims its task first, so two sessions never work
    /// the same one.
    pub fn share(&self, bag: &Taskbag, now_secs: i64) -> Vec<(String, Task)> {
        let iteration = match &self.scope {
            Scope::All => None,
            Scope::Iteration { id } => Some(id.as_str()),
        };
        let left_out =
            |t: &Task| self.given_up.contains(&t.id) || self.working.iter().any(|w| w.id == t.id);
        let lost_after = heartbeat::applied(&bag.project).lost_after_secs;
        let mut ready: Vec<&Task> = bag
            .tasks
            .iter()
            .filter(|t| {
                !left_out(t)
                    && iteration.is_none_or(|i| t.iteration.as_deref() == Some(i))
                    && heartbeat::lost(t, now_secs, lost_after, None).is_some()
            })
            .collect();
        for t in ops::next(bag, iteration).unwrap_or_default() {
            if !self.taken.contains(&t.id) && !left_out(t) && !ready.iter().any(|r| r.id == t.id) {
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
            // A lost task's former holder is not this session's claim (A03-husky861).
            seen_held: false,
            seen: seen_of(Some(task)),
            changed: now,
            holder: None,
            prior: task.holder.clone(),
            prior_session: task
                .holder
                .as_ref()
                .and_then(|_| present_session(task).and_then(|s| s.id.clone())),
            awaiting_approval: None,
        });
        self.checked = now;
    }

    /// The sessions that have shown no sign of life for [`STALL_MS`].
    pub fn stalled(&self, now: i64) -> Vec<&Working> {
        self.working
            .iter()
            .filter(|w| w.awaiting_approval.is_none() && now - w.changed >= STALL_MS)
            .collect()
    }

    /// Record whether `id`'s session shows a confirmation now (`since`, ms) or not (none); true when
    /// it has just begun to (Task-A03-candle790).
    pub fn set_awaiting(&mut self, id: &str, since: Option<i64>) -> bool {
        let Some(w) = self.working.iter_mut().find(|w| w.id == id) else {
            return false;
        };
        let began = w.awaiting_approval.is_none() && since.is_some();
        w.awaiting_approval = since;
        began
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

/// Whether `task` is in progress and its holder has beaten for it: then [`heartbeat::lost`] tells
/// whether its session is still alive. A session that never beat (one not yet claimed, or an
/// older `alm`) is timed by its task's changes instead.
fn beats_for(task: &Task) -> bool {
    task.state == State::InProgress
        && task
            .holder
            .as_ref()
            .is_some_and(|h| task.heartbeat.as_ref().is_some_and(|b| &b.by == h))
}

/// What following one session's task found.
enum Followed {
    /// Still its session's: the session as it is now.
    Working(Working),
    /// Finished: done, dropped or waiting; held by no one after being claimed; left `ready`
    /// without ever being held; gone.
    Finished,
    /// Held by another session, of this name (A03-cart761).
    Others(String),
}

/// The holder's name when the task is held by a session that is not `cur`'s (A03-cart761). Told
/// by what the holder passed at `alm claim`, in this order: its session ID against the one
/// Allomos gave this session (the same ID is this session, whatever else says); its agent
/// against this session's; its name against the holder this session was seen as before. A holder
/// that passed nothing to tell by counts as this session.
fn others(cur: &Working, task: &Task) -> Option<String> {
    let holder = task.holder.as_ref()?;
    let session = present_session(task);
    if let (Some(mine), Some(theirs)) = (&cur.session_id, session.and_then(|s| s.id.as_ref())) {
        return (mine != theirs).then(|| holder.clone());
    }
    let agent = session.and_then(|s| s.agent.as_deref());
    let other = agent.is_some_and(|a| a != crate::request::session_agent(&cur.agent))
        || cur.holder.as_ref().is_some_and(|own| own != holder);
    other.then(|| holder.clone())
}

/// The session recorded on `task` when it is its present holder's (a former holder's may be left
/// there).
fn present_session(task: &Task) -> Option<&allomos_core::AgentSession> {
    let holder = task.holder.as_ref()?;
    task.session
        .as_ref()
        .filter(|s| s.by.as_ref().is_none_or(|by| by == holder))
}

/// Whether `task` is still held by the former holder it had when `cur` was started for it
/// (A03-husky861), so `cur` has not claimed it yet. A claim by `cur` shows as another holder name,
/// another session ID (this session's own, when Allomos gave it one), or, when neither tells, a
/// heartbeat by the holder since `cur` was started.
fn prior_holds(cur: &Working, task: &Task) -> bool {
    let (Some(prior), Some(holder)) = (&cur.prior, &task.holder) else {
        return false;
    };
    if prior != holder || task.state != State::InProgress {
        return false;
    }
    let id = present_session(task).and_then(|s| s.id.as_deref());
    if let (Some(mine), Some(id)) = (cur.session_id.as_deref(), id) {
        if mine == id {
            return false;
        }
    }
    if let (Some(before), Some(id)) = (cur.prior_session.as_deref(), id) {
        return before == id;
    }
    let beat = task
        .heartbeat
        .as_ref()
        .filter(|b| &b.by == holder)
        .and_then(|b| heartbeat::parse_utc(&b.at));
    beat.is_none_or(|at| at < cur.at.div_euclid(1000))
}

/// One session's task as it is now (see [`Followed`]).
fn follow_one(cur: &Working, tasks: &[Task], lost_after_secs: i64, now: i64) -> Followed {
    let task = tasks.iter().find(|t| t.id == cur.id);
    // Still the former holder's (A03-husky861): not claimed yet, so timed from the start, and
    // the former holder is neither this session nor another one that took the task.
    if task.is_some_and(|t| prior_holds(cur, t)) {
        return Followed::Working(cur.clone());
    }
    if let Some(holder) = task.and_then(|t| others(cur, t)) {
        return Followed::Others(holder);
    }
    let seen = seen_of(task);
    let seen_held = cur.seen_held || task.is_some_and(|t| t.holder.is_some());
    let finished = match task {
        None => true,
        Some(t) => {
            matches!(t.state, State::Done | State::Dropped | State::Waiting)
                || (t.holder.is_none() && (seen_held || t.state != State::Ready))
        }
    };
    if finished {
        return Followed::Finished;
    }
    let mut next = cur.clone();
    if seen != cur.seen {
        next.changed = now;
    }
    // A session beating for the task it holds is judged by its heartbeat (A03-plum984), the same
    // way `alm next` tells a lost task.
    if let Some(t) = task.filter(|t| beats_for(t)) {
        if heartbeat::lost(t, now.div_euclid(1000), lost_after_secs, None).is_some() {
            next.changed = next.changed.min(now - STALL_MS);
        } else {
            next.changed = now;
        }
    }
    next.seen = seen;
    next.seen_held = seen_held;
    if let Some(holder) = task.and_then(|t| t.holder.clone()) {
        next.holder = Some(holder);
    }
    Followed::Working(next)
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

    /// The default limit for a held task's heartbeat (30 minutes), in seconds.
    const LOST: i64 = 30 * 60;

    fn task(id: &str, state: State, holder: Option<&str>) -> Task {
        let mut t: Task = serde_json::from_value(serde_json::json!({
            "id": id, "title": id, "area": "A01", "state": "ready"
        }))
        .unwrap();
        t.state = state;
        t.holder = holder.map(String::from);
        t
    }

    #[test]
    fn a_chosen_model_and_effort_are_kept_and_a_default_or_odd_value_is_left_out() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        assert_eq!(r.choice_of("claude_cli"), Choice::default());
        let pick = |m: Option<&str>, e: Option<&str>| Choice {
            model: m.map(str::to_string),
            effort: e.map(str::to_string),
        };
        r.set_choice("claude_cli", &pick(Some("opus"), Some("high")));
        r.set_choice("codex_cli", &pick(None, Some("xhigh")));
        assert_eq!(r.choice_of("claude_cli"), pick(Some("opus"), Some("high")));
        assert_eq!(r.choice_of("codex_cli"), pick(None, Some("xhigh")));
        // Saved with the run, and an older run's file without them reads as the defaults.
        let text = serde_json::to_string(&r).unwrap();
        let back: Run = serde_json::from_str(&text).unwrap();
        assert_eq!(
            back.choice_of("claude_cli"),
            pick(Some("opus"), Some("high"))
        );
        let plain = serde_json::to_string(&run(&["claude_cli"])).unwrap();
        assert!(!plain.contains("models") && !plain.contains("efforts"));
        // Back to the default, and a value that is not one plain word is not kept.
        r.set_choice("claude_cli", &pick(None, Some("high; rm -rf ~")));
        assert_eq!(r.choice_of("claude_cli"), Choice::default());
        assert!(plain_choice("gpt-6.2-astra") && plain_choice("opus[1m]"));
        assert!(!plain_choice("") && !plain_choice("a b") && !plain_choice("$(x)"));
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
        let bag = bag_of(vec![held_since(
            "A01-9",
            "old/session",
            "2026-11-09T10:00:00Z",
        )]);
        let quiet = r.share(&bag, beat + 2 * 3600);
        assert_eq!(quiet.len(), 1);
        assert_eq!(quiet[0].1.id, "A01-9");
        assert!(r.share(&bag, beat + 5 * 60).is_empty());
    }

    /// A task held by `holder`, with what its session passed at `alm claim`.
    fn claimed(id: &str, holder: &str, agent: Option<&str>, session_id: Option<&str>) -> Task {
        let mut t = task(id, State::InProgress, Some(holder));
        t.session = Some(allomos_core::AgentSession {
            by: Some(holder.to_string()),
            agent: agent.map(String::from),
            id: session_id.map(String::from),
            ..Default::default()
        });
        t
    }

    /// A03-cart761 (the approver's screenshot): the Claude Code session never claimed its task;
    /// a Codex session claims it. The run lets it go at the next check: the row is gone, the
    /// slot is free, the reason is noted, and the task is not given to this run again.
    #[test]
    fn a_task_another_session_claims_is_let_go_and_its_slot_freed() {
        let mut r = run(&["claude_cli"]);
        r.set_sessions("claude_cli", 3);
        let sid = Some("mine-1".to_string());
        r.take(
            "claude_cli",
            &task("A01-1", State::Ready, None),
            sid.clone(),
            None,
            10,
        );
        r.take(
            "claude_cli",
            &task("A01-2", State::Ready, None),
            sid,
            None,
            10,
        );
        assert_eq!(r.free_agents().len(), 1);
        // Never claimed, still ready: both stay.
        let ready = [
            task("A01-1", State::Ready, None),
            task("A01-2", State::Ready, None),
        ];
        assert!(r.follow(&ready, LOST, 20).is_empty());
        assert_eq!(r.working.len(), 2);
        // Codex sessions claim both (another agent, another session ID).
        let tasks = [
            claimed("A01-1", "codex-a01-1", Some("codex"), Some("theirs-1")),
            claimed("A01-2", "codex-a01-2", Some("codex"), None),
        ];
        let finished = r.follow(&tasks, LOST, 30);
        assert!(
            finished.is_empty(),
            "let go, not finished: no tab is closed"
        );
        assert!(r.working.is_empty());
        assert_eq!(r.free_agents().len(), 3);
        let noted: Vec<(&str, &str, i64)> = r
            .let_go
            .iter()
            .map(|l| (l.id.as_str(), l.holder.as_str(), l.at))
            .collect();
        assert_eq!(
            noted,
            [("A01-1", "codex-a01-1", 30), ("A01-2", "codex-a01-2", 30)]
        );
        // Still taken: not given to this run again while ready; once lost, a new session takes
        // it over (A03-husky861).
        assert!(r.share(&bag_of(ready.to_vec()), 0).is_empty());
        let lost = bag_of(vec![held_since(
            "A01-1",
            "codex-a01-1",
            "2026-11-09T10:00:00Z",
        )]);
        let later = heartbeat::parse_utc("2026-11-09T10:00:00Z").unwrap() + 2 * 3600;
        let ids: Vec<String> = r
            .share(&lost, later)
            .into_iter()
            .map(|(_, t)| t.id)
            .collect();
        assert_eq!(ids, ["A01-1"]);
    }

    /// The run's own session's task stays and is timed as before: the same session ID is this
    /// session whatever name it claims under; a holder that passed nothing counts as this one.
    #[test]
    fn a_task_its_own_session_claims_is_kept() {
        let mut r = run(&["claude_cli:work", "codex_cli"]);
        let sid = Some("mine-1".to_string());
        r.take(
            "claude_cli:work",
            &task("A01-1", State::Ready, None),
            sid,
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
        r.take(
            "codex_cli",
            &task("A01-3", State::Ready, None),
            None,
            None,
            10,
        );
        let tasks = [
            claimed("A01-1", "claude-a01-1", Some("claude-code"), Some("mine-1")),
            // Codex picks its own session ID: its agent is what tells.
            claimed("A01-2", "codex-a01-2", Some("codex"), Some("its-own")),
            task("A01-3", State::InProgress, Some("someone")),
        ];
        assert!(r.follow(&tasks, LOST, 20).is_empty());
        assert_eq!(r.working.len(), 3);
        assert!(r.let_go.is_empty());
        assert_eq!(r.working[0].changed, 20);
        assert_eq!(r.working[0].holder.as_deref(), Some("claude-a01-1"));
        // Nothing changes: the time since the last sign of life goes on.
        assert!(r.follow(&tasks, LOST, 30).is_empty());
        assert_eq!(r.working[0].changed, 20);
        // Released after the claim: finished as before, not let go.
        let finished = r.follow(&[task("A01-1", State::Ready, None)], LOST, 40);
        assert_eq!(finished.len(), 3);
        assert!(r.let_go.is_empty());
    }

    /// A task taken over from the run's session is let go the same way, by each sign: another
    /// session ID, another agent, or (when the new holder passed nothing) another holder name.
    #[test]
    fn a_task_taken_over_is_let_go() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.set_sessions("codex_cli", 2);
        let sid = Some("mine-1".to_string());
        r.take(
            "claude_cli",
            &task("A01-1", State::Ready, None),
            sid,
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
        r.take(
            "codex_cli",
            &task("A01-3", State::Ready, None),
            None,
            None,
            10,
        );
        let own = [
            claimed("A01-1", "claude-a01-1", Some("claude-code"), Some("mine-1")),
            claimed("A01-2", "codex-a01-2", Some("codex"), Some("c-1")),
            claimed("A01-3", "codex-a01-3", Some("codex"), Some("c-2")),
        ];
        assert!(r.follow(&own, LOST, 20).is_empty());
        assert_eq!(r.working.len(), 3);
        // A01-1: another Claude Code session (another ID). A01-2: taken with nothing passed, so
        // the record still shows the former holder's session. A01-3: the same name, still held.
        let mut bare = own[1].clone();
        bare.holder = Some("a-person".into());
        let taken = [
            claimed("A01-1", "claude-other", Some("claude-code"), Some("theirs")),
            bare,
            own[2].clone(),
        ];
        assert!(r.follow(&taken, LOST, 30).is_empty());
        let ids: Vec<&str> = r.working.iter().map(|w| w.id.as_str()).collect();
        assert_eq!(ids, ["A01-3"]);
        let noted: Vec<(&str, &str)> = r
            .let_go
            .iter()
            .map(|l| (l.id.as_str(), l.holder.as_str()))
            .collect();
        assert_eq!(noted, [("A01-1", "claude-other"), ("A01-2", "a-person")]);
        // Done by the other session: still let go (the tab of this run's session stays open).
        let mut r = run(&["claude_cli"]);
        r.take(
            "claude_cli",
            &task("A01-9", State::Ready, None),
            None,
            None,
            10,
        );
        let mut done = claimed("A01-9", "codex-a01-9", Some("codex"), None);
        done.state = State::Done;
        assert!(r.follow(&[done], LOST, 20).is_empty());
        assert_eq!(r.let_go.len(), 1);
    }

    /// Only the latest let-go tasks are kept to show.
    #[test]
    fn the_run_keeps_the_latest_let_go_tasks() {
        let mut r = run(&["claude_cli"]);
        for n in 0..LET_GO_KEPT + 2 {
            let id = format!("A01-{n}");
            r.take("claude_cli", &task(&id, State::Ready, None), None, None, 10);
            r.follow(&[claimed(&id, "codex-x", Some("codex"), None)], LOST, 20);
        }
        assert_eq!(r.let_go.len(), LET_GO_KEPT);
        assert_eq!(r.let_go[0].id, "A01-2");
        assert_eq!(r.taken.len(), LET_GO_KEPT + 2);
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
        assert!(r.follow(&tasks, LOST, 20).is_empty());
        assert!(r.working[0].seen_held);
        assert_eq!(r.working[0].changed, 20);
        // Released after the claim, and done: both finished.
        let tasks = [
            task("A01-1", State::Ready, None),
            task("A01-2", State::Done, Some("s2")),
        ];
        let finished = r.follow(&tasks, LOST, 30);
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
            r.follow(&[task("A01-3", State::Ready, None)], LOST, 50)
                .is_empty()
        );
        assert!(r.follow(&[], LOST, 60).len() == 1);
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
        assert_eq!(r.follow(&tasks, LOST, 20).len(), 1);
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
        let shares: Vec<(String, String)> = r
            .share(&bag, 0)
            .into_iter()
            .map(|(a, t)| (a, t.id))
            .collect();
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
    fn a_new_scope_counts_from_the_next_share_and_keeps_the_sessions_at_work() {
        let mut r = run(&["claude_cli"]);
        r.set_sessions("claude_cli", 2);
        let mut a: Task = leveled("A01-1", 3);
        a.iteration = Some("I-1".into());
        let mut b: Task = leveled("A01-2", 3);
        b.iteration = Some("I-2".into());
        r.take("claude_cli", &a, None, None, 1);
        r.set_scope(Scope::Iteration { id: "I-2".into() });
        let mut bag = bag_of(vec![a.clone(), b.clone()]);
        for (id, order) in [("I-1", 1), ("I-2", 2)] {
            bag.iterations.push(
                serde_json::from_value(serde_json::json!({ "id": id, "name": id, "order": order }))
                    .unwrap(),
            );
        }
        let shares = r.share(&bag, 0);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].1.id, "A01-2");
        // The session at work is kept.
        assert_eq!(r.working.len(), 1);
        // An iteration with no ID is ignored.
        r.set_scope(Scope::Iteration { id: " ".into() });
        assert_eq!(r.scope, Scope::Iteration { id: "I-2".into() });
        r.set_scope(Scope::All);
        assert_eq!(r.scope, Scope::All);
    }

    /// A03-otter944: a session that never claims its task did not start; its task is given out
    /// again, and its agent waits while its quota is used up.
    #[test]
    fn a_session_that_never_claims_its_task_did_not_start_and_its_task_goes_out_again() {
        let mut r = run(&["claude_cli"]);
        let bag = bag_of(vec![leveled("A01-1", 3)]);
        let shares = r.share(&bag, 0);
        assert_eq!(shares.len(), 1);
        r.take("claude_cli", &shares[0].1, None, None, 0);
        // Not yet 10 minutes: still waiting for its session to claim it.
        assert!(r.unclaimed(&bag.tasks, NOT_STARTED_MS - 1).is_empty());
        assert!(r.share(&bag, 0).is_empty(), "the slot is taken meanwhile");
        // 10 minutes, still ready and never held: it did not start.
        let gone = r.unclaimed(&bag.tasks, NOT_STARTED_MS);
        assert_eq!(gone.len(), 1);
        assert!(r.working.is_empty() && r.taken.is_empty() && r.taken_by.is_empty());
        r.did_not_start(
            &gone[0],
            PauseWhy::Quota,
            Some("7:10pm".into()),
            NOT_STARTED_MS,
        );
        assert_eq!(r.not_started[0].id, "A01-1");
        assert_eq!(r.not_started[0].resets.as_deref(), Some("7:10pm"));
        // Its quota used up: no session for the agent until it is read again.
        assert!(r.free_agents().is_empty());
        assert!(r.share(&bag, 0).is_empty());
        assert!(
            r.pauses_due(NOT_STARTED_MS + QUOTA_RECHECK_MS - 1)
                .is_empty()
        );
        let due = r.pauses_due(NOT_STARTED_MS + QUOTA_RECHECK_MS);
        assert_eq!(due, [("claude_cli".to_string(), PauseWhy::Quota)]);
        // Back: the same task is given out again.
        r.resume("claude_cli");
        let again = r.share(&bag, 0);
        assert_eq!(again.len(), 1);
        assert_eq!(again[0].1.id, "A01-1");
    }

    #[test]
    fn a_session_that_claimed_its_task_is_not_taken_for_one_that_did_not_start() {
        let mut r = run(&["claude_cli"]);
        r.take("claude_cli", &leveled("A01-1", 3), None, None, 0);
        // Claimed, then released after the claim: finished as before, not "did not start".
        let held = [task("A01-1", State::InProgress, Some("s"))];
        assert!(r.unclaimed(&held, 1).is_empty());
        r.follow(&held, LOST, 1);
        assert!(
            r.unclaimed(&[task("A01-1", State::Ready, None)], NOT_STARTED_MS)
                .is_empty()
        );
    }

    #[test]
    fn an_agent_paused_for_no_reason_read_is_tried_again_at_the_next_check() {
        let mut r = run(&["claude_cli", "codex_cli"]);
        r.pause("codex_cli", PauseWhy::NoClaim, None, 0);
        assert_eq!(r.free_agents(), ["claude_cli"]);
        assert_eq!(r.paused["codex_cli"].until, 30 * 60_000);
        // A second pause keeps when it started.
        r.pause("codex_cli", PauseWhy::Quota, None, 5);
        assert_eq!(r.paused["codex_cli"].since, 0);
        let text = serde_json::to_string(&r).unwrap();
        assert!(text.contains(r#""paused":{"codex_cli":{"why":"quota""#));
        assert_eq!(serde_json::from_str::<Run>(&text).unwrap(), r);
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

    /// A03-plum984: `A01-7` held by `me`, its last heartbeat `beat_min` minutes after `T0`, with
    /// a busy window to `busy_min` when given.
    fn beating(beat_min: i64, busy_min: Option<i64>) -> Task {
        let at = |m: i64| heartbeat::format_utc(T0 / 1000 + m * 60);
        let mut beat = serde_json::json!({ "at": at(beat_min), "by": "me" });
        if let Some(b) = busy_min {
            beat["busy_until"] = serde_json::json!(at(b));
        }
        serde_json::from_value(serde_json::json!({
            "id": "A01-7", "title": "A01-7", "area": "A01", "state": "in_progress",
            "holder": "me", "heartbeat": beat
        }))
        .unwrap()
    }

    /// 2026-11-16T10:00:00Z, in ms.
    const T0: i64 = 1_794_823_200_000;
    const MIN: i64 = 60 * 1000;

    /// A run whose one session took `A01-7` at `T0` and claimed it a minute later.
    fn claimed_at_t0() -> Run {
        let mut r = run(&["claude_cli"]);
        r.take(
            "claude_cli",
            &task("A01-7", State::Ready, None),
            None,
            None,
            T0,
        );
        assert!(r.follow(&[beating(1, None)], LOST, T0 + MIN).is_empty());
        r
    }

    /// A session that only beats (nothing else of its task changes) is not stalled after 30
    /// minutes, nor after an hour (A03-plum984: A01-muffin353 beat on and was shown as stalled).
    #[test]
    fn a_session_still_beating_is_not_stalled() {
        let mut r = claimed_at_t0();
        for m in [11, 21, 31, 41, 61, 91] {
            r.follow(&[beating(m - 1, None)], LOST, T0 + m * MIN);
            assert!(r.stalled(T0 + m * MIN).is_empty(), "{m} minutes");
        }
    }

    /// A session whose heartbeat stopped is stalled exactly when `alm next` lists its task as
    /// lost, with the project's limit (here 30 minutes, then 15).
    #[test]
    fn a_session_whose_heartbeat_stopped_is_stalled_when_its_task_is_lost() {
        for limit in [LOST, 15 * 60] {
            let mut r = claimed_at_t0();
            let task = beating(1, None);
            for m in [2, 10, 16, 20, 31, 32, 60] {
                let now = T0 + m * MIN;
                r.follow(std::slice::from_ref(&task), limit, now);
                let lost = heartbeat::lost(&task, now / 1000, limit, None).is_some();
                assert_eq!(
                    !r.stalled(now).is_empty(),
                    lost,
                    "{m} minutes, limit {limit}s"
                );
            }
            assert!(!r.stalled(T0 + 60 * MIN).is_empty());
        }
    }

    /// A session that declared a busy window is not stalled while it runs, and is once the
    /// window is over by more than the limit.
    #[test]
    fn a_busy_session_is_not_stalled_in_its_busy_window() {
        let mut r = claimed_at_t0();
        let task = beating(1, Some(90));
        for m in [31, 60, 89, 119] {
            r.follow(std::slice::from_ref(&task), LOST, T0 + m * MIN);
            assert!(r.stalled(T0 + m * MIN).is_empty(), "{m} minutes");
        }
        r.follow(std::slice::from_ref(&task), LOST, T0 + 121 * MIN);
        assert_eq!(r.stalled(T0 + 121 * MIN).len(), 1);
    }

    /// A session not claimed yet has no heartbeat: it is timed from its start, as before.
    #[test]
    fn a_session_not_claimed_yet_is_timed_from_its_start() {
        let mut r = run(&["claude_cli"]);
        r.take(
            "claude_cli",
            &task("A01-7", State::Ready, None),
            None,
            None,
            T0,
        );
        let ready = [task("A01-7", State::Ready, None)];
        r.follow(&ready, LOST, T0 + 9 * MIN);
        assert!(r.stalled(T0 + 29 * MIN).is_empty());
        assert_eq!(r.stalled(T0 + 30 * MIN).len(), 1);
    }

    /// A task held by `holder` (session `sid` of Claude Code) that last beat `beat_min` minutes
    /// after [`T0`].
    fn held_by(id: &str, holder: &str, sid: &str, beat_min: i64) -> Task {
        let mut t = claimed(id, holder, Some("claude-code"), Some(sid));
        t.heartbeat = Some(
            serde_json::from_value(serde_json::json!({
                "at": heartbeat::format_utc(T0 / 1000 + beat_min * 60), "by": holder
            }))
            .unwrap(),
        );
        t
    }

    /// A run whose Claude Code session `mine` claimed A01-7 at [`T0`] and beat once.
    fn holding_a01_7() -> Run {
        let mut r = run(&["claude_cli"]);
        r.take(
            "claude_cli",
            &task("A01-7", State::Ready, None),
            Some("mine".into()),
            None,
            T0,
        );
        let held = held_by("A01-7", "claude-a", "mine", 1);
        assert!(r.follow(&[held], LOST, T0 + 2 * MIN).is_empty());
        assert!(r.working[0].seen_held);
        r
    }

    /// Task-A03-candle790: a session at a confirmation frees its slot at once, is neither lost nor
    /// stalled while it waits without a heartbeat, and its task is not given out again; once it
    /// goes on it takes its slot again.
    #[test]
    fn a_session_waiting_for_an_approval_frees_its_slot_but_keeps_its_task() {
        let mut r = holding_a01_7();
        let held = held_by("A01-7", "claude-a", "mine", 1);
        assert!(r.free_agents().is_empty());
        assert!(r.set_awaiting("A01-7", Some(T0 + 3 * MIN)));
        assert!(!r.set_awaiting("A01-7", Some(T0 + 3 * MIN)), "told once");
        assert_eq!(r.free_agents(), ["claude_cli"]);
        // Long past the heartbeat limit: still its own, neither lost nor stalled.
        assert!(r.lost_ones(&[held.clone()], LOST, T0 + 90 * MIN).is_empty());
        assert!(r.follow(&[held.clone()], LOST, T0 + 90 * MIN).is_empty());
        assert!(r.stalled(T0 + 90 * MIN).is_empty());
        assert_eq!(r.working.len(), 1);
        assert!(r.taken.contains(&"A01-7".to_string()));
        let bag = bag_of(vec![held.clone()]);
        assert!(r.share(&bag, (T0 + 90 * MIN) / 1000).is_empty());
        // Approved: it takes its slot again.
        assert!(!r.set_awaiting("A01-7", None));
        assert!(r.free_agents().is_empty());
        assert!(!r.set_awaiting("A09-9", Some(T0)), "no such session");
        // The run file keeps it, and an older one reads without it.
        r.set_awaiting("A01-7", Some(T0 + 3 * MIN));
        let saved = serde_json::to_value(&r).unwrap();
        assert_eq!(
            saved["working"][0]["awaiting_approval"],
            serde_json::json!(T0 + 3 * MIN)
        );
        let mut older = saved.clone();
        older["working"][0]
            .as_object_mut()
            .unwrap()
            .remove("awaiting_approval");
        let read: Run = serde_json::from_value(older).unwrap();
        assert_eq!(read.working[0].awaiting_approval, None);
    }

    #[test]
    fn a_session_lost_for_any_reason_frees_its_slot_and_its_task_goes_out_first() {
        let mut r = holding_a01_7();
        let held = held_by("A01-7", "claude-a", "mine", 1);
        // Within the limit: kept.
        assert!(r.lost_ones(&[held.clone()], LOST, T0 + 30 * MIN).is_empty());
        assert!(r.free_agents().is_empty());
        // Past it: let go of, its slot free and the task no longer counted as taken.
        let gone = r.lost_ones(&[held.clone()], LOST, T0 + 32 * MIN);
        assert_eq!(gone.len(), 1);
        r.went_lost(&gone[0], false, None, T0 + 32 * MIN);
        assert!(r.working.is_empty());
        assert_eq!(r.free_agents(), ["claude_cli"]);
        assert!(!r.taken.contains(&"A01-7".to_string()));
        assert_eq!(r.lost[0].holder, "claude-a");
        assert_eq!(r.failures["A01-7"], 1);
        // Offered before a ready task, though the run took it before.
        let bag = bag_of(vec![task("A01-1", State::Ready, None), held]);
        let shares = r.share(&bag, (T0 + 33 * MIN) / 1000);
        assert_eq!(shares.len(), 1);
        assert_eq!(shares[0].1.id, "A01-7");
    }

    #[test]
    fn a_session_lost_when_its_quota_ran_out_is_taken_over_only_once_the_quota_is_back() {
        let mut r = holding_a01_7();
        let held = held_by("A01-7", "claude-a", "mine", 1);
        let now = T0 + 40 * MIN;
        let gone = r.lost_ones(&[held.clone()], LOST, now);
        // The runner reads "You've hit your limit · resets 7:10pm" in its tab and pauses the agent.
        r.pause("claude_cli", PauseWhy::Quota, Some("7:10pm".into()), now);
        r.went_lost(&gone[0], true, Some("7:10pm".into()), now);
        assert!(r.lost[0].quota);
        // While paused: no slot, so no new session for it.
        let bag = bag_of(vec![held.clone()]);
        assert!(r.free_agents().is_empty());
        assert!(r.share(&bag, now / 1000).is_empty());
        assert!(r.pauses_due(now + QUOTA_RECHECK_MS - 1).is_empty());
        // Its quota read again and back: the lost task goes to a new session.
        assert_eq!(r.pauses_due(now + QUOTA_RECHECK_MS).len(), 1);
        r.resume("claude_cli");
        let shares = r.share(&bag, (now + QUOTA_RECHECK_MS) / 1000);
        assert_eq!(shares[0].1.id, "A01-7");
    }

    #[test]
    fn the_former_holder_is_not_taken_for_the_new_sessions_claim() {
        let mut r = run(&["claude_cli"]);
        let old = held_by("A01-7", "claude-old", "old-sid", 0);
        let start = T0 + 60 * MIN;
        r.take("claude_cli", &old, Some("new-sid".into()), None, start);
        let w = &r.working[0];
        assert!(!w.seen_held, "the former holder's record is not a claim");
        assert_eq!(w.prior.as_deref(), Some("claude-old"));
        assert_eq!(w.prior_session.as_deref(), Some("old-sid"));
        // Not let go as another session's, not finished, not judged by the old heartbeat.
        assert!(r.follow(&[old.clone()], LOST, start + MIN).is_empty());
        assert_eq!(r.working.len(), 1);
        assert!(r.let_go.is_empty());
        assert!(!r.working[0].seen_held);
        assert!(r.stalled(start + MIN).is_empty());
        assert!(r.lost_ones(&[old.clone()], LOST, start + MIN).is_empty());
        // The new session claims it (its own session ID, under the same name): now it is its.
        let mut new = held_by("A01-7", "claude-old", "new-sid", 62);
        new.session.as_mut().unwrap().by = Some("claude-old".into());
        assert!(r.follow(&[new], LOST, start + 3 * MIN).is_empty());
        assert!(r.working[0].seen_held);
    }

    #[test]
    fn a_claim_by_the_new_session_shows_as_a_new_name_or_session_or_a_later_heartbeat() {
        let start = T0 + 60 * MIN;
        let started = |sid: Option<&str>, old: &Task| {
            let mut r = run(&["codex_cli"]);
            r.take("codex_cli", old, sid.map(String::from), None, start);
            r.working.remove(0)
        };
        let old = held_by("A01-7", "old", "old-sid", 0);
        let w = started(None, &old);
        assert!(prior_holds(&w, &old));
        assert!(!prior_holds(&w, &held_by("A01-7", "new", "x", 61)));
        assert!(!prior_holds(&w, &held_by("A01-7", "old", "other-sid", 61)));
        // No session IDs to tell by: a heartbeat since the start is the new claim.
        let mut bare = held_since("A01-7", "old", &heartbeat::format_utc(T0 / 1000));
        let w = started(None, &bare);
        assert!(prior_holds(&w, &bare));
        bare.heartbeat.as_mut().unwrap().at = heartbeat::format_utc(start / 1000 + 60);
        assert!(!prior_holds(&w, &bare));
    }

    #[test]
    fn a_new_session_that_never_claims_a_lost_task_is_tried_again_then_given_up() {
        let mut r = run(&["claude_cli"]);
        let old = held_by("A01-7", "claude-old", "old-sid", 0);
        let mut now = T0 + 60 * MIN;
        for tried in 1..=MAX_FAILURES {
            let bag = bag_of(vec![old.clone()]);
            let shares = r.share(&bag, now / 1000);
            assert_eq!(shares.len(), 1, "try {tried}");
            r.take(
                "claude_cli",
                &shares[0].1,
                Some(format!("s{tried}")),
                None,
                now,
            );
            // The old holder still holds it: not claimed yet, the slot is kept for a while.
            assert!(
                r.unclaimed(&[old.clone()], now + NOT_STARTED_MS - 1)
                    .is_empty()
            );
            let gone = r.unclaimed(&[old.clone()], now + NOT_STARTED_MS);
            assert_eq!(gone.len(), 1, "try {tried}");
            r.did_not_start(&gone[0], PauseWhy::NoClaim, None, now + NOT_STARTED_MS);
            r.resume("claude_cli");
            now += NOT_STARTED_MS + MIN;
        }
        // Given up: no more sessions for it, and the run says so.
        assert_eq!(r.given_up, ["A01-7"]);
        assert!(r.share(&bag_of(vec![old]), now / 1000).is_empty());
    }

    #[test]
    fn a_run_file_from_before_lost_sessions_still_reads() {
        let r = holding_a01_7();
        let mut v = serde_json::to_value(&r).unwrap();
        for w in v["working"].as_array_mut().unwrap() {
            let w = w.as_object_mut().unwrap();
            w.remove("prior");
            w.remove("prior_session");
        }
        let back: Run = serde_json::from_value(v).unwrap();
        assert_eq!(back, r);
    }
}
