//! The monitoring loop (Task-A03-cow112): one process per run, `alm monitor run <id>`, started in
//! the background by `alm monitor start` (or the app's Start button), or in the foreground for a
//! test. It checks at the run's frequency, gives each free agent the next ready task in a new
//! terminal session (each session claims its task first), follows the tasks in the taskbag until
//! they are finished, closes a done task's tab, and tells the person when it is stuck. It writes
//! its run file ([`crate::registry`]) on every change and about every 30 seconds, takes the
//! requests left for it about once a second, and ends when a stop was asked and nothing is worked.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use allomos_core::{Problem, Task, Taskbag};

use crate::flow::{Closing, PauseWhy, Run, Stuck, Working, tab_to_close, waiting_alert};
use crate::quota::{self, UsedUp};
use crate::registry::{self, Held, Record, Request};
use crate::unreadable::{Failures, Step};
use crate::{accounts, notify, request, session_tab, terminal};

/// How often requests are looked for.
const TICK: Duration = Duration::from_secs(1);
/// How often the taskbag is read again to follow the sessions' tasks.
const READ_EVERY: Duration = Duration::from_secs(5);
/// How often the run file is written even when nothing changed ("still alive").
const ALIVE_EVERY: Duration = Duration::from_secs(30);

/// Milliseconds since 1970, now.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

/// What a run is started with.
#[derive(Debug, Clone)]
pub struct Options {
    /// The taskbag's folder (where `project.json` is).
    pub taskbag: PathBuf,
    /// The language requests and notifications are written in (the task language).
    pub language: String,
    /// The run as it starts ([`Run::start`]).
    pub run: Run,
    /// The `alm` the sessions run: its folder goes first in their `PATH`.
    pub alm: Option<PathBuf>,
    /// The version of `alm` that ran this run before, when it is resumed by another one (the app
    /// moved it to its new `alm` after an update, Task-A03-cookie729); `Some("")` for a run whose
    /// record did not say.
    pub updated_from: Option<String>,
}

/// A line in the run's log (its standard error, which `start` points at the log file).
fn log(text: &str) {
    eprintln!("{} {text}", stamp(now_ms()));
}

/// `HH:MM` for a time in ms, in the computer's local time (from the `date` command; UTC when it
/// cannot be asked).
pub fn local_hhmm(ms: i64) -> String {
    #[cfg(unix)]
    {
        let secs = (ms / 1000).to_string();
        let args: [&str; 3] = if cfg!(target_os = "macos") {
            ["-r", &secs, "+%H:%M"]
        } else {
            ["-d", &format!("@{secs}"), "+%H:%M"]
        };
        if let Ok(out) = terminal::command("date").args(args).output() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if out.status.success() && !text.is_empty() {
                return text;
            }
        }
    }
    let m = (ms / 60_000).rem_euclid(24 * 60);
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn stamp(ms: i64) -> String {
    let s = (ms / 1000).rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}Z", s / 3600, s / 60 % 60, s % 60)
}

/// The project folder of the taskbag at `root`.
pub fn project_of(root: &Path) -> PathBuf {
    allomos_store::versions::project_folder(root)
}

/// The taskbag as an agent working in the project folder passes it to `--taskbag`: relative to
/// that folder, written with `/` (absolute when there is no relative way).
pub fn taskbag_arg(project: &Path, taskbag: &Path) -> String {
    let (project, taskbag) = (allomos_store::tidy(project), allomos_store::tidy(taskbag));
    allomos_store::relative_path(&project, &taskbag)
        .map(|p| allomos_store::portable_path(&p))
        .unwrap_or_else(|| taskbag.display().to_string())
}

/// A session ID for Claude Code's `--session-id`: a random version-4 UUID.
pub fn new_session_id() -> String {
    use std::hash::{BuildHasher, Hasher};
    let mut bytes = [0u8; 16];
    for (i, chunk) in bytes.chunks_mut(8).enumerate() {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default(),
        );
        h.write_usize(i);
        h.write_u32(std::process::id());
        chunk.copy_from_slice(&h.finish().to_le_bytes());
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
        s
    });
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// The taskbag as it is now. When it cannot be read for a reason only a person can fix (its
/// format, Task-A03-raven398), that problem itself, not the general "could not be read".
fn read(root: &Path) -> Result<Taskbag, Problem> {
    let loaded = allomos_store::load(root);
    if loaded.taskbag.is_none() {
        let format = loaded
            .issues
            .iter()
            .find(|i| crate::unreadable::needs_person(&i.problem));
        if let Some(issue) = format {
            return Err(issue.problem.clone());
        }
    }
    allomos_store::taskbag_of(root, loaded).map(|(bag, _)| bag)
}

/// Tell the person a task waits on them: a notification the app shows (see [`notify`]).
fn tell(dir: &Path, run_id: &str, stuck: &Stuck, language: &str) {
    let body = match stuck {
        Stuck::Waiting { id, on } => request::notify_waiting(language, id, on),
    };
    log(&format!("told the person: {body}"));
    notify::show(dir, run_id, request::notify_title(language), &body);
}

/// The running loop's state besides the run itself.
struct Loop {
    dir: PathBuf,
    id: String,
    record: Record,
    held: Held,
    /// The agents' commands, found once when the run starts.
    found: Vec<(String, PathBuf)>,
    alm_dir: Option<PathBuf>,
    /// This `alm`, which starts the tab countdowns.
    alm: Option<PathBuf>,
    taskbag_arg: String,
    last_written: Instant,
    /// Process-local: immediate exit never becomes a resumable run setting.
    stop_now: bool,
    /// The closing countdowns of finished tasks' tabs, each with its thread (A03-pocket537).
    countdowns: Vec<(Closing, std::thread::JoinHandle<()>)>,
}

impl Loop {
    fn write(&mut self) {
        self.record.alive = now_ms();
        self.held.keep();
        // The countdowns still running are kept in the run's file: a run that stops before they
        // end starts them again when it resumes.
        self.countdowns.retain(|(_, h)| !h.is_finished());
        self.record.run.closing = self.countdowns.iter().map(|(c, _)| c.clone()).collect();
        let path = registry::record_path(&self.dir, &self.id);
        match serde_json::to_string_pretty(&self.record) {
            Ok(text) => {
                if let Err(e) = registry::write_whole(&path, &text) {
                    log(&format!("could not write {}: {e}", path.display()));
                }
            }
            Err(e) => log(&format!("could not write the run file: {e}")),
        }
        if let Err(e) = registry::write_resume(&self.dir, &self.record) {
            log(&format!("could not write the run's resume file: {e}"));
        }
        self.last_written = Instant::now();
    }

    fn apply(&mut self, req: Request) -> bool {
        let run = &mut self.record.run;
        log(&format!("asked: {req:?}"));
        match req {
            Request::Check => {
                run.checked = 0;
            }
            Request::StopNow => self.stop_now = true,
            Request::Stop => run.stopping = true,
            Request::UndoStop => run.stopping = false,
            Request::Every { minutes } => {
                if crate::flow::FREQUENCIES.contains(&minutes) {
                    run.every = minutes;
                }
            }
            Request::Agents { agents } => {
                let accounts = accounts::load();
                let usable: Vec<String> = agents
                    .into_iter()
                    .filter(|a| {
                        self.found
                            .iter()
                            .any(|(id, _)| id == accounts::base_agent(a))
                            && accounts::usable(a, &accounts)
                    })
                    .collect();
                run.set_agents(&usable);
            }
            Request::Sessions { agent, count } => {
                if crate::flow::TERMINAL_AGENTS.contains(&accounts::base_agent(&agent)) {
                    run.set_sessions(&agent, count);
                }
            }
            Request::Difficulty { agent, range } => {
                if crate::flow::TERMINAL_AGENTS.contains(&accounts::base_agent(&agent)) {
                    run.set_difficulty(&agent, &range);
                }
            }
            Request::Scope { scope } => run.set_scope(scope),
            Request::Mode { quiet } => run.quiet = quiet,
        }
        true
    }

    /// Whether `agent`'s quota is used up (A03-otter944): a terminal Claude Code is asked
    /// (`claude -p /usage`, answered without the model); when it cannot be, and for the other
    /// agents, the agents' last `alm quota` reports say.
    fn used_up(&self, agent: &str, accounts: &[accounts::Account]) -> Option<UsedUp> {
        let base = accounts::base_agent(agent);
        if base == "claude_cli" {
            let asked = self
                .found
                .iter()
                .find(|(id, _)| id == base)
                .zip(accounts::config_of(agent, accounts).ok())
                .and_then(|((_, program), config)| {
                    quota::ask_usage(
                        program,
                        config.as_ref().map(|(var, dir)| (*var, dir.as_path())),
                    )
                });
            if let Some(read) = asked.as_deref().and_then(quota::from_usage) {
                return read;
            }
        }
        quota::from_reports(agent, &allomos_store::quota_reports(), &quota::now_local())
    }

    /// `agent` gets no session until its quota comes back; the person is told once.
    fn pause_for_quota(&mut self, agent: &str, task: Option<&str>, used: UsedUp, now: i64) {
        let told = self
            .record
            .run
            .paused
            .get(agent)
            .is_some_and(|p| p.why == PauseWhy::Quota);
        self.record
            .run
            .pause(agent, PauseWhy::Quota, used.resets.clone(), now);
        if !told {
            let line = request::quota_used_up(
                &self.record.language,
                terminal::name(agent),
                task,
                used.resets.as_deref(),
            );
            log(&line);
            notify::show(
                &self.dir,
                &self.id,
                request::notify_title(&self.record.language),
                &line,
            );
        }
    }

    /// The sessions that did not start (A03-otter944): each one's tab says why when it can be
    /// read ("You've hit your 5-hour limit · resets 7:10pm"); its task is given out again.
    fn not_started(&mut self, gone: Vec<Working>, now: i64) {
        for w in gone {
            let used = w
                .tab
                .as_ref()
                .and_then(session_tab::text_of)
                .and_then(|text| quota::from_session_text(&text));
            log(&format!(
                "{} did not start ({}): it never claimed its task{}",
                w.id,
                w.agent,
                if used.is_some() {
                    "; its quota is used up"
                } else {
                    ""
                }
            ));
            match used {
                Some(used) => {
                    self.pause_for_quota(&w.agent, Some(&w.id), used.clone(), now);
                    self.record
                        .run
                        .did_not_start(&w, PauseWhy::Quota, used.resets, now);
                }
                None => {
                    let line = request::not_started(
                        &self.record.language,
                        terminal::name(&w.agent),
                        &w.id,
                    );
                    notify::show(
                        &self.dir,
                        &self.id,
                        request::notify_title(&self.record.language),
                        &line,
                    );
                    self.record
                        .run
                        .did_not_start(&w, PauseWhy::NoClaim, None, now);
                }
            }
        }
    }

    /// The sessions found lost while they held their task (A03-husky861): their slots are free
    /// and their tasks go out again. A tab that says the quota is used up pauses the agent until
    /// it comes back. The sessions themselves are left as they are.
    fn lost(&mut self, gone: Vec<Working>, now: i64) {
        for w in gone {
            let used = w
                .tab
                .as_ref()
                .and_then(session_tab::text_of)
                .and_then(|text| quota::from_session_text(&text));
            log(&format!(
                "{} lost ({}): no heartbeat past the project's limit; its slot is free{}",
                w.id,
                w.agent,
                if used.is_some() {
                    "; its quota is used up"
                } else {
                    ""
                }
            ));
            let resets = used.as_ref().and_then(|u| u.resets.clone());
            if let Some(used) = used {
                self.pause_for_quota(&w.agent, Some(&w.id), used, now);
            }
            let quota = self
                .record
                .run
                .paused
                .get(&w.agent)
                .is_some_and(|p| p.why == PauseWhy::Quota);
            self.record.run.went_lost(&w, quota, resets, now);
        }
    }

    /// Tell the person once about each task given up since `before` (A03-husky861).
    fn tell_given_up(&self, before: &[String]) {
        for id in self
            .record
            .run
            .given_up
            .iter()
            .filter(|g| !before.contains(g))
        {
            let line = request::given_up(&self.record.language, id, crate::flow::MAX_FAILURES);
            log(&line);
            notify::show(
                &self.dir,
                &self.id,
                request::notify_title(&self.record.language),
                &line,
            );
        }
    }

    /// The paused agents whose time has come: an agent out of quota has it read again (and gets
    /// sessions again when there is some); one paused after a session that did not start, for
    /// no reason read, gets sessions again.
    fn look_at_pauses(&mut self, now: i64) {
        let due = self.record.run.pauses_due(now);
        if due.is_empty() {
            return;
        }
        let accounts = accounts::load();
        for (agent, why) in due {
            match (why, self.used_up(&agent, &accounts)) {
                (PauseWhy::Quota, Some(used)) => {
                    self.record.run.pause(&agent, why, used.resets, now);
                }
                _ => {
                    log(&format!("{agent} gets sessions again"));
                    self.record.run.resume(&agent);
                }
            }
        }
    }

    /// Start one session per share; an agent that cannot be started is taken off the run.
    fn start_sessions(&mut self, shares: Vec<(String, Task)>) {
        // The accounts as they are now: an account added or removed in the app counts from here
        // (Task-A03-oven548).
        let accounts = accounts::load();
        // The working branches, read once and only when a lost task is given out (A03-husky861).
        let mut branches = None;
        for (agent, task) in shares {
            // An agent taken off by an earlier failure gets no more sessions this check, nor one
            // paused at this check.
            if !self.record.run.agents.contains(&agent)
                || self.record.run.paused.contains_key(&agent)
            {
                continue;
            }
            // Its quota used up: no session; the task stays ready (A03-otter944).
            if let Some(used) = self.used_up(&agent, &accounts) {
                self.pause_for_quota(&agent, Some(&task.id), used, now_ms());
                continue;
            }
            // The agent's command and its settings (`claude_cli:<account>` runs `claude`, in the
            // account's folder).
            let base = accounts::base_agent(&agent).to_string();
            let session_id = (base == "claude_cli").then(new_session_id);
            // A lost task is taken over on its working branch (A03-husky861).
            let takeover = task.holder.as_ref().map(|holder| request::Takeover {
                holder: holder.clone(),
                branch: branches
                    .get_or_insert_with(|| allomos_store::branches(&self.record.taskbag))
                    .get(&task.id)
                    .and_then(|b| b.working.clone()),
            });
            let text = request::monitor_request(
                &task.id,
                &task.title,
                &self.taskbag_arg,
                &self.record.language,
                &base,
                session_id.as_deref(),
                self.record.run.quiet,
                takeover.as_ref(),
            );
            let found = self
                .found
                .iter()
                .find(|(id, _)| *id == base)
                .map(|(_, p)| p.display().to_string());
            // Antigravity adds the taskbag and its other project folders to its workspace
            // (Task-A03-latch882); Codex makes them writable in its sandbox (Task-A03-bag625).
            let dirs = if base == "antigravity_cli" || base == "codex_cli" {
                terminal::workspace_dirs(&self.record.taskbag, &self.record.project)
            } else {
                Vec::new()
            };
            let started = match (found, accounts::config_of(&agent, &accounts)) {
                (Some(found), Ok(config)) => terminal::start_session(
                    &base,
                    &found,
                    &self.record.project,
                    &text,
                    session_id.as_deref(),
                    self.alm_dir.as_deref(),
                    config.as_ref().map(|(var, dir)| (*var, dir.as_path())),
                    self.record.run.quiet,
                    &dirs,
                    // The model and effort chosen for this agent, if any (A03-alto709).
                    &self.record.run.choice_of(&agent),
                ),
                (None, _) => Err(terminal::ask_failed(&agent, "not found on this computer")),
                (_, Err(why)) => Err(terminal::ask_failed(&agent, why)),
            };
            match started {
                Ok(tab) => {
                    log(&format!("started {agent} for {}", task.id));
                    self.record
                        .run
                        .take(&agent, &task, session_id, tab, now_ms());
                }
                Err(p) => {
                    let line = request::start_failed(
                        &self.record.language,
                        terminal::name(&agent),
                        &task.id,
                        &p.english(),
                    );
                    log(&line);
                    notify::show(
                        &self.dir,
                        &self.id,
                        request::notify_title(&self.record.language),
                        &line,
                    );
                    let run = &mut self.record.run;
                    let left: Vec<String> = run
                        .agents
                        .iter()
                        .filter(|a| **a != agent)
                        .cloned()
                        .collect();
                    if left.is_empty() {
                        run.stopping = true;
                    } else {
                        run.set_agents(&left);
                    }
                }
            }
        }
    }

    /// One pass: follow the sessions' tasks, then give free agents tasks when a check is due.
    /// Returns whether the run changed, and whether it is over.
    fn pass(&mut self, bag: &Taskbag) -> (bool, bool) {
        let now = now_ms();
        let before = self.record.run.clone();
        // Sessions that never claimed their task did not start (A03-otter944).
        let gone = self.record.run.unclaimed(&bag.tasks, now);
        let gone_any = !gone.is_empty();
        self.not_started(gone, now);
        let lost_after = allomos_core::heartbeat::applied(&bag.project).lost_after_secs;
        // Sessions lost while holding their task free their slots (A03-husky861).
        let lost = self.record.run.lost_ones(&bag.tasks, lost_after, now);
        let gone_any = gone_any || !lost.is_empty();
        self.lost(lost, now);
        self.tell_given_up(&before.given_up);
        let finished = self.record.run.follow(&bag.tasks, lost_after, now);
        for cur in &finished {
            log(&format!("{} finished ({})", cur.id, cur.agent));
            if let Some(tab) = tab_to_close(cur, &bag.tasks) {
                // Its own process unless that cannot start (then a thread, kept for the run to wait for).
                let handle = session_tab::close_when_done(
                    self.alm.as_deref(),
                    &registry::log_path(&self.dir, &self.id),
                    &cur.agent,
                    tab,
                    &self.record.language,
                );
                if let Some(handle) = handle {
                    self.countdowns.push((
                        Closing {
                            agent: cur.agent.clone(),
                            tab: tab.clone(),
                        },
                        handle,
                    ));
                }
            }
            if let Some(stuck) = waiting_alert(&cur.id, &bag.tasks) {
                tell(&self.dir, &self.id, &stuck, &self.record.language);
            }
        }
        let run = &self.record.run;
        if run.stopping {
            return (*run != before, run.working.is_empty());
        }
        // A slot freed by a finished, a let-go (A03-cart761) or a not-started task (A03-otter944)
        // is filled at once.
        let freed = !finished.is_empty() || gone_any || run.let_go != before.let_go;
        let due = freed || now >= run.next_check();
        self.look_at_pauses(now);
        let run = &self.record.run;
        if !run.free_agents().is_empty() && due {
            let run = &self.record.run;
            let shares = run.share(bag, now / 1000);
            // An agent with a free slot and no task in its range gets no session; the status bar
            // says so (Task-A03-wave335).
            self.record.run.no_task = run.unfilled(&shares);
            if shares.is_empty() {
                // Nothing to take: the status bar says so; the person is not told (A03-stork103).
                self.record.run.checked = now;
            } else {
                self.start_sessions(shares);
            }
        } else if self.record.run.free_agents().is_empty() {
            self.record.run.no_task.clear();
        }
        let run = &self.record.run;
        let over = run.stopping && run.working.is_empty();
        (*run != before, over)
    }
}

/// Run `options` as run `id` until it ends; its files live in `dir`. Refused when another process
/// already runs `id`. A run resumed from its resume file (A03-pocket537) carries on with the
/// sessions it had at work and starts again the closing countdowns it had not finished.
pub fn run(dir: &Path, id: &str, options: Options) -> Result<(), Problem> {
    let failed = |e: std::io::Error| Problem::WriteFailed {
        path: dir.display().to_string(),
        detail: e.to_string(),
    };
    std::fs::create_dir_all(dir).map_err(failed)?;
    let Some(held) = Held::take(&registry::lock_path(dir, id)).map_err(failed)? else {
        return Err(Problem::BadUsage {
            detail: format!("monitor run {id} is already running"),
        });
    };
    let mut options = options;
    let root = options.taskbag.clone();
    let project = project_of(&root);
    let bag = read(&root)?;
    let found = terminal::find_terminal_agents();
    let alm_dir = options
        .alm
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
    // A resumed run: the sessions at work are timed again from now, and the countdowns come back.
    let now = now_ms();
    for w in &mut options.run.working {
        w.changed = now;
    }
    let countdowns = std::mem::take(&mut options.run.closing)
        .into_iter()
        .filter_map(|c| {
            let handle = session_tab::close_when_done(
                options.alm.as_deref(),
                &registry::log_path(dir, id),
                &c.agent,
                &c.tab,
                &options.language,
            )?;
            Some((c, handle))
        })
        .collect();
    let mut lp = Loop {
        dir: dir.to_path_buf(),
        id: id.to_string(),
        record: Record {
            format: registry::FORMAT,
            id: id.to_string(),
            pid: std::process::id(),
            taskbag: root.clone(),
            project: project.clone(),
            language: options.language.clone(),
            log: registry::log_path(dir, id),
            alive: now_ms(),
            run: options.run,
            program: registry::Program {
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                path: options.alm.clone(),
                updated_from: options.updated_from.clone(),
            },
        },
        held,
        found,
        alm_dir,
        alm: options.alm.clone(),
        taskbag_arg: taskbag_arg(&project, &root),
        last_written: Instant::now(),
        stop_now: false,
        countdowns,
    };
    log(&format!(
        "monitor {id} started: {} ({:?}, agents {}, every {} min, {})",
        root.display(),
        lp.record.run.scope,
        lp.record.run.agents.join(", "),
        lp.record.run.every,
        if lp.record.run.quiet {
            "quiet"
        } else {
            "normal"
        }
    ));
    drop(bag);
    // Moved to this alm after an update (Task-A03-cookie729): the person is told once.
    if let Some(from) = &options.updated_from {
        let body = request::updated(&lp.record.language, env!("CARGO_PKG_VERSION"));
        log(&format!(
            "carries on with alm {} (was {}): {body}",
            env!("CARGO_PKG_VERSION"),
            if from.is_empty() {
                "an older one"
            } else {
                from
            }
        ));
        notify::show(
            &lp.dir,
            &lp.id,
            request::notify_title(&lp.record.language),
            &body,
        );
    }
    lp.write();
    let mut last_read: Option<Instant> = None;
    // Reads that failed in a row (Task-A03-raven398).
    let mut failures = Failures::default();
    loop {
        let mut changed = false;
        let mut read_now = false;
        for req in registry::take_requests(dir, id) {
            changed |= lp.apply(req);
            read_now = true;
        }
        // Stop before any taskbag read, follow, quota probe or dispatch, even if another
        // request in the same batch asked to check or undo the graceful stop.
        if lp.stop_now {
            break;
        }
        if read_now || last_read.is_none_or(|t| t.elapsed() >= READ_EVERY) {
            last_read = Some(Instant::now());
            match read(&root) {
                Ok(bag) => {
                    if let Some(line) = failures.read() {
                        log(&line);
                    }
                    changed |= lp.record.run.unreadable.take().is_some();
                    let (c, over) = lp.pass(&bag);
                    changed |= c;
                    if over && lp.countdowns.iter().all(|(_, h)| h.is_finished()) {
                        break;
                    }
                }
                Err(p) => {
                    match failures.failed(&p, now_ms()) {
                        Step::Log(line) => log(&line),
                        Step::Quiet => {}
                        Step::Tell(why) => {
                            // The run goes on trying (approver, 2026-11-16); the person is told
                            // once and the status bar shows it until the taskbag reads again.
                            let body = request::unreadable(&lp.record.language, why, &p.english());
                            log(&format!("told the person: {body} ({})", p.english()));
                            notify::show(
                                &lp.dir,
                                &lp.id,
                                request::notify_title(&lp.record.language),
                                &body,
                            );
                            lp.record.run.unreadable = Some(why.code().to_string());
                            changed = true;
                        }
                    }
                    // A stop asked while the taskbag cannot be read: the sessions at work cannot be
                    // followed, so the run ends now; they finish their tasks by themselves.
                    if lp.record.run.stopping {
                        break;
                    }
                }
            }
        }
        if changed || lp.last_written.elapsed() >= ALIVE_EVERY {
            lp.write();
        }
        std::thread::sleep(TICK);
    }
    log(&format!(
        "monitor {id} ended; it started sessions for: {}",
        if lp.record.run.taken.is_empty() {
            "none".to_string()
        } else {
            lp.record.run.taken.join(", ")
        }
    ));
    // The last run's line counts it, even when the app is closed (A03-pocket537).
    let last = registry::LastSummary {
        ended: now_ms(),
        scope: lp.record.run.scope.clone(),
        taken: lp.record.run.taken.clone(),
        taken_by: lp.record.run.taken_by.clone(),
    };
    if let Err(e) = registry::write_last(dir, &root, &last) {
        log(&format!("could not write the last run: {e}"));
    }
    // The countdowns of done tasks' tabs finish before the run is gone: it stays listed (stopping)
    // until then, and a stop in the meantime finds it still there.
    if !lp.stop_now {
        for (_, handle) in lp.countdowns.drain(..) {
            let _ = handle.join();
        }
    }
    let _ = std::fs::remove_file(registry::record_path(dir, id));
    // Only what finding and controlling a running run needs stays on this computer (approver,
    // 2026-11-09): the log of a run that ended as asked goes too. A run that could not start, or
    // was ended from outside, leaves its log for the person to read.
    let _ = std::fs::remove_file(registry::log_path(dir, id));
    registry::remove_resume(dir, id);
    lp.held.release();
    Ok(())
}

/// Start `program args…` in the background, apart from the terminal it was started from (its own
/// session on macOS and Linux, detached on Windows), with its output going to `log`. Returns its
/// process number.
pub fn spawn_detached(program: &Path, args: &[String], log: &Path) -> std::io::Result<u32> {
    use std::process::{Command, Stdio};
    let out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)?;
    let err = out.try_clone()?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: `setsid` only makes the child the leader of a new session; it is safe to call
        // between fork and exec.
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW);
    }
    command.spawn().map(|c| c.id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids_are_uuids() {
        let a = new_session_id();
        assert_eq!(a.len(), 36);
        assert_eq!(&a[14..15], "4");
        assert!(terminal::plain_session_id(&a));
        assert_ne!(a, new_session_id());
    }

    #[test]
    fn the_taskbag_is_named_from_the_project() {
        assert_eq!(
            taskbag_arg(Path::new("/w/proj"), Path::new("/w/tb/.allomos/taskbag")),
            "../tb/.allomos/taskbag"
        );
    }
}

#[cfg(test)]
#[path = "runner_stop_tests.rs"]
mod stop_tests;
