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

use crate::flow::{Run, Stuck, tab_to_close, waiting_alert};
use crate::registry::{self, Held, Record, Request};
use crate::{notify, request, session_tab, terminal};

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
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// Let every blocked task whose blockers have all finished go on (as the app's sweep does, at
/// the start and on Check now), made as the person's saved name. A failure is logged.
fn sweep(root: &Path) {
    let name = allomos_store::saved_owner_name();
    match allomos_store::release_finished(
        root,
        allomos_store::Actor::alm(name.as_deref()),
        Duration::from_secs(10),
    ) {
        Ok(list) if !list.is_empty() => {
            let ids: Vec<&str> = list.iter().map(|r| r.task.id.as_str()).collect();
            log(&format!(
                "let go on (blockers finished): {}",
                ids.join(", ")
            ));
        }
        Ok(_) => {}
        Err(p) => log(&format!(
            "could not let finished blockers' tasks go on: {}",
            p.english()
        )),
    }
}

/// The taskbag as it is now.
fn read(root: &Path) -> Result<Taskbag, Problem> {
    allomos_store::taskbag_of(root, allomos_store::load(root)).map(|(bag, _)| bag)
}

/// Tell the person the run is stuck: a system notification.
fn tell(stuck: &Stuck, run: &Run, language: &str) {
    let body = match stuck {
        Stuck::Waiting { id, on } => request::notify_waiting(language, id, on),
        Stuck::Idle => request::notify_idle(language, &local_hhmm(run.next_check())),
    };
    log(&format!("told the person: {body}"));
    notify::show(request::notify_title(language), &body);
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
    taskbag_arg: String,
    last_written: Instant,
}

impl Loop {
    fn write(&mut self) {
        self.record.alive = now_ms();
        self.held.keep();
        let path = registry::record_path(&self.dir, &self.id);
        match serde_json::to_string_pretty(&self.record) {
            Ok(text) => {
                if let Err(e) = registry::write_whole(&path, &text) {
                    log(&format!("could not write {}: {e}", path.display()));
                }
            }
            Err(e) => log(&format!("could not write the run file: {e}")),
        }
        self.last_written = Instant::now();
    }

    fn apply(&mut self, req: Request) -> bool {
        let run = &mut self.record.run;
        log(&format!("asked: {req:?}"));
        match req {
            Request::Check => {
                sweep(&self.record.taskbag);
                run.checked = 0;
            }
            Request::Stop => run.stopping = true,
            Request::UndoStop => run.stopping = false,
            Request::Every { minutes } => {
                if crate::flow::FREQUENCIES.contains(&minutes) {
                    run.every = minutes;
                }
            }
            Request::Agents { agents } => {
                let usable: Vec<String> = agents
                    .into_iter()
                    .filter(|a| self.found.iter().any(|(id, _)| id == a))
                    .collect();
                run.set_agents(&usable);
            }
        }
        true
    }

    /// Start one session per share; an agent that cannot be started is taken off the run.
    fn start_sessions(&mut self, shares: Vec<(String, Task)>) {
        for (agent, task) in shares {
            let session_id = (agent == "claude_cli").then(new_session_id);
            let text = request::monitor_request(
                &task.id,
                &task.title,
                &self.taskbag_arg,
                &self.record.language,
                &agent,
                session_id.as_deref(),
                self.record.run.quiet,
            );
            let found = self
                .found
                .iter()
                .find(|(id, _)| *id == agent)
                .map(|(_, p)| p.display().to_string());
            let started = match found {
                Some(found) => terminal::start_session(
                    &agent,
                    &found,
                    &self.record.project,
                    &text,
                    session_id.as_deref(),
                    self.alm_dir.as_deref(),
                ),
                None => Err(terminal::ask_failed(&agent, "not found on this computer")),
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
                    notify::show(request::notify_title(&self.record.language), &line);
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
        let finished = self.record.run.follow(&bag.tasks, now);
        for cur in &finished {
            log(&format!("{} finished ({})", cur.id, cur.agent));
            if let Some(tab) = tab_to_close(cur, &bag.tasks) {
                session_tab::close_when_done(
                    cur.agent.clone(),
                    tab.clone(),
                    request::close_texts(&self.record.language),
                );
            }
            if let Some(stuck) = waiting_alert(&cur.id, &bag.tasks) {
                tell(&stuck, &self.record.run, &self.record.language);
            }
        }
        let run = &self.record.run;
        if run.stopping {
            return (*run != before, run.working.is_empty());
        }
        let due = !finished.is_empty() || now >= run.next_check();
        if !run.free_agents().is_empty() && due {
            let shares = run.share(bag);
            if shares.is_empty() {
                let run = &mut self.record.run;
                run.checked = now;
                if let Some(stuck) = run.idle_alert() {
                    tell(&stuck, run, &self.record.language);
                }
            } else {
                self.start_sessions(shares);
            }
        }
        let run = &self.record.run;
        let over = run.stopping && run.working.is_empty();
        (*run != before, over)
    }
}

/// Run `options` as run `id` until it ends; its files live in `dir`. Refused when another process
/// already runs `id`.
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
    let root = options.taskbag.clone();
    let project = project_of(&root);
    let bag = read(&root)?;
    let found = terminal::find_terminal_agents();
    let alm_dir = options
        .alm
        .as_deref()
        .and_then(Path::parent)
        .map(Path::to_path_buf);
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
        },
        held,
        found,
        alm_dir,
        taskbag_arg: taskbag_arg(&project, &root),
        last_written: Instant::now(),
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
    sweep(&root);
    lp.write();
    let mut last_read: Option<Instant> = None;
    loop {
        let mut changed = false;
        let mut read_now = false;
        for req in registry::take_requests(dir, id) {
            changed |= lp.apply(req);
            read_now = true;
        }
        if read_now || last_read.is_none_or(|t| t.elapsed() >= READ_EVERY) {
            last_read = Some(Instant::now());
            match read(&root) {
                Ok(bag) => {
                    let (c, over) = lp.pass(&bag);
                    changed |= c;
                    if over {
                        break;
                    }
                }
                Err(p) => log(&format!("could not read the taskbag: {}", p.english())),
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
    let _ = std::fs::remove_file(registry::record_path(dir, id));
    lp.held.release();
    // Leave the closing countdowns of done tasks' tabs time to finish.
    std::thread::sleep(Duration::from_secs(1));
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
