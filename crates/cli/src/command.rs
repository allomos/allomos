use std::path::PathBuf;

use allomos_core::State;
use clap::Subcommand;

use crate::command_args::{
    Add, CardsWhat, ChangeWhat, Delete, FolderWhat, LabelsWhat, MoveWhat, Rename,
};
use crate::{difficulty_level, difficulty_range, monitor};

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Create an empty taskbag in this folder's .allomos/taskbag/ (or where --taskbag says).
    Init {
        /// The project's name. Default: the folder's name.
        #[arg(long)]
        name: Option<String>,
        /// The project's ID. Default: made from the folder's name.
        #[arg(long)]
        id: Option<String>,
        /// The label ladder, lowest step first, separated by commas.
        /// Default: implemented, locally verified, deployed and verified, human verified.
        #[arg(long, value_name = "A,B,…")]
        labels: Option<String>,
        /// Write Allomos's section for AI agents into AGENTS.md and CLAUDE.md yourself,
        /// instead of printing a request for the agent running `alm init` to do it.
        #[arg(long)]
        write_agent_files: bool,
    },
    /// Add an area or a task.
    #[command(subcommand)]
    Add(Add),
    /// Rename an area (`alm rename area <id> "<name>"`; its ID stays), give an iteration a new ID
    /// with every task planned in it (`alm rename iteration <old> <new>`), or rename a label of the
    /// ladder with every task that carries it (`alm rename label "<old>" "<new>"`).
    #[command(subcommand)]
    Rename(Rename),
    /// Change an iteration: `alm change iteration <id> [--name] [--release] [--start] [--end]
    /// [--state planned|active|closed]`.
    #[command(subcommand)]
    Change(ChangeWhat),
    /// Move an iteration to another place in the plan: `alm move iteration <id> --before <id>`
    /// (or `--after <id>`). Only the order changes.
    #[command(subcommand)]
    Move(MoveWhat),
    /// Delete an area or an iteration; refused while any task is in it, done and dropped ones too.
    #[command(subcommand)]
    Delete(Delete),
    /// Print Allomos's section for this project's AGENTS.md: the short brief agents read first.
    Brief,
    /// Confirm app changes were committed, pushed and verified by the agent. Reads no Git.
    Shared {
        /// The last app event from app_changes.through, relative to the taskbag.
        #[arg(long)]
        through: String,
    },
    /// Report how much of your account's quota is left, at the handoff gate's quota check, from
    /// the figure you just read (for example Claude Code's get_usage). The Allomos app shows it
    /// beside the agent. Kept on this computer, not in the taskbag: nothing to save or share.
    /// Without --agent, it lists the reports.
    Quota {
        /// Your agent's id: claude-code, codex, antigravity (Google Antigravity's agy; its
        /// /usage shows the figure), or another agent's (not your session name; a word starting
        /// with claude, codex or antigravity, or a session name claude/…, is read as that agent).
        #[arg(long, requires = "left")]
        agent: Option<String>,
        /// A name for the account, only when you can tell one; never an email address or user
        /// name.
        #[arg(long, requires = "agent")]
        account: Option<String>,
        /// The share left, 0 to 100.
        #[arg(long, value_name = "PERCENT", requires = "agent")]
        left: Option<u8>,
        /// When this window resets, local time (2026-10-20T17:00).
        #[arg(long, value_name = "WHEN", requires = "agent")]
        resets: Option<String>,
        /// Which limit the figure is for: 5h (the handoff gate's) or week.
        #[arg(long, default_value = "5h", requires = "agent")]
        window: String,
    },
    /// Print the details the brief points to: terms, what a project may customize, commands,
    /// how to work the tasks, the run modes to show the person before an automatic or loop run,
    /// the one-time procedure that puts every agent's shared rules in AGENTS.md, and the goal
    /// prompt that builds an app from a description, and how to publish a GitHub release and install
    /// it. All topics (in English) when none is named. A topic may be written as separate words
    /// (`alm guide github release`); words after the topic are the guide's, for the agent to read.
    Guide {
        /// The topic, as one word or several (joined with `-`), then any words the guide defines.
        words: Vec<String>,
        /// For `run-modes`, `agent-files`, `goal` and `github-release`: the language to print it in
        /// (default: the task language).
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
    },
    /// The language to write new and changed tasks in. Without a value: show it (run this before
    /// adding a task or changing its title or description: `alm title`, `alm describe`). With one: set it, to a language code
    /// such as zh-CN, or to ui to follow the Allomos app's language.
    #[command(name = "task-language", alias = "task_language")]
    TaskLanguage {
        #[arg(value_name = "CODE|ui")]
        value: Option<String>,
    },
    /// The project's label table (decision 0020): every label it knows, the built-in ones first,
    /// with how many tasks carry each. `alm labels add` adds one (or sets its meaning);
    /// `alm labels remove` removes one no task carries. Built-in labels cannot be removed.
    Labels {
        #[command(subcommand)]
        what: Option<LabelsWhat>,
    },
    /// Show a plan to the person in the Allomos app and wait for their answer: exit 0 means
    /// go ahead; exit 1 (confirm_declined, confirm_timeout) means change nothing.
    Confirm {
        /// The popup's heading.
        #[arg(long)]
        title: String,
        /// A Markdown file with the plan to show.
        #[arg(long, value_name = "FILE")]
        plan: PathBuf,
        /// How long to wait for the answer, in seconds.
        #[arg(long, default_value_t = 600, value_name = "SECONDS")]
        timeout: u64,
        /// A Markdown file with the evidence, shown under the plan: what was checked and the
        /// results.
        #[arg(long, value_name = "FILE")]
        evidence: Option<PathBuf>,
        /// The task the approval is for: a Go ahead is recorded in its history, for exactly the
        /// plan shown. If the plan changes afterwards, ask again.
        #[arg(long, value_name = "ID")]
        task: Option<String>,
    },
    /// Tell the person in the Allomos app what you are doing on a job it gave you: the app shows
    /// a progress popup. Start with --title and --step, report each step, and end with --finished.
    Progress {
        /// The job (needed to start one; later reports keep it).
        #[arg(long)]
        title: Option<String>,
        /// What you are doing now.
        #[arg(long)]
        step: Option<String>,
        /// Steps done so far, of --of.
        #[arg(long, value_name = "N")]
        done: Option<u32>,
        /// All steps, when you know the count.
        #[arg(long, value_name = "N")]
        of: Option<u32>,
        /// The job is over: what happened.
        #[arg(long, value_name = "TEXT")]
        finished: Option<String>,
        /// You asked the person something and wait for the answer: the app shows the question
        /// and does not say you went quiet. Your next report ends it.
        #[arg(long, value_name = "QUESTION")]
        waiting: Option<String>,
    },
    /// List tasks, grouped by area (or by iteration with --by iteration).
    List {
        #[arg(long)]
        area: Option<String>,
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// Group by area (the default) or by iteration; tasks with no iteration come last, as unplanned.
        #[arg(long, value_parser = ["area", "iteration"], default_value = "area")]
        by: String,
        #[arg(long, value_parser = State::ALL.map(State::as_str))]
        state: Option<String>,
        /// Only tasks that are blocked: one of their blockers has not finished. Goes with
        /// --state (for example --state ready --blocked).
        #[arg(long)]
        blocked: bool,
        /// Only tasks you hold.
        #[arg(long)]
        mine: bool,
        /// Only tasks to check: done, with neither "by human" label (decision 0020).
        /// `--without-owner` is its old name.
        #[arg(long, alias = "without-owner")]
        without_human_check: bool,
        /// Only tasks that belong to this project folder (its short name).
        #[arg(long, value_name = "NAME")]
        folder: Option<String>,
        /// Only tasks of these difficulties: 1-3, or one level such as 4 (no difficulty counts
        /// as 3).
        #[arg(long, value_name = "RANGE", value_parser = difficulty_range)]
        difficulty: Option<allomos_core::difficulty::Range>,
    },
    /// What needs a person, one task at a time, in the order to take them (decision 0028): first
    /// the tasks an agent is waiting on (to answer, to approve afterwards), then the finished ones
    /// (to check, AI decisions to review) by risk and by how long they wait.
    Human {
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// Only tasks in this area.
        #[arg(long)]
        area: Option<String>,
        /// Under each task to check, the steps for checking it.
        #[arg(long)]
        steps: bool,
    },
    /// A person has checked one or more finished tasks: each gets "deployed and verified by human"
    /// when it carries "deployed and verified by agent", else "locally verified by human", and
    /// leaves To check; the note and the steps stay in its history. Every given task must be
    /// eligible or none is recorded. Run it only when the person says so.
    Checked {
        /// One or more task IDs.
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
        /// The person checked it in the deployed app: "deployed and verified by human".
        #[arg(long, conflicts_with = "locally")]
        deployed: bool,
        /// The person checked it on their own computer: "locally verified by human".
        #[arg(long)]
        locally: bool,
        /// What the person saw: `pass`, or `partly` (passed with conditions; say which in
        /// --words; only one task ID at a time). A check that failed is not `alm checked`: run
        /// `alm reopen` instead.
        #[arg(long, value_name = "RESULT", default_value = "pass", value_parser = ["pass", "partly"])]
        result: String,
        /// The person's own words about the check, as they said them.
        #[arg(long, value_name = "TEXT")]
        words: Option<String>,
        /// The checker's pseudonym ID, when it is not the person you work for (this computer's
        /// person, or `--for`).
        #[arg(long, value_name = "PSEUDONYM")]
        checker: Option<String>,
    },
    /// Send a finished task back to To check: its work changed after a person checked it, so
    /// the check no longer covers it. You decide when; say what changed. A task at the top label
    /// goes one step down.
    Recheck {
        id: String,
        /// What changed since the person checked it, in one line. Shown under To check.
        #[arg(long, value_name = "TEXT")]
        reason: String,
        /// A Markdown file with new steps for checking it (otherwise the earlier steps stay).
        #[arg(long, value_name = "FILE")]
        steps: Option<PathBuf>,
    },
    /// Record a decision you made on your own in a task (a routine choice: a name, wording and
    /// look, a file layout, a small detail with an obvious default): what you decided and why, in
    /// one line, and which kind it is (`--kind`, decision 0025 § 3). It is kept in the task's
    /// history; `alm human` lists a `visible` or `principle` one (or one with no kind) under To
    /// review: AI decisions until a person has reviewed it; a `detail` is kept with the task but
    /// not listed there.
    #[command(name = "ai-decided")]
    AiDecided {
        id: String,
        #[arg(value_name = "WHAT AND WHY")]
        text: String,
        /// visible: what a user sees or runs into (wording, look, behaviour); principle: how it
        /// is built and why; detail: sizes, limits, parameters, names in the code. Unsure
        /// between principle and detail: principle.
        #[arg(long, value_parser = ["visible", "principle", "detail"])]
        kind: Option<String>,
    },
    /// A person has reviewed one or more tasks' AI decisions: they leave To review; the history
    /// keeps them. Every given task must have decisions waiting for review or none is recorded.
    /// Run it only when the person says so.
    Reviewed {
        /// One or more task IDs.
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
    },
    /// Record a person's approval of a plan (decision 0021): who approved (their pseudonym ID),
    /// of exactly which plan (the SHA-256 of the tasks' descriptions, or of `--plan`'s file), when,
    /// through which channel, in their own words, and until when. Once the plan changes, it no
    /// longer applies. Run it only when the person approved; `alm show` lists a task's approvals.
    Approve {
        /// The tasks it covers.
        #[arg(required = true, value_name = "ID")]
        ids: Vec<String>,
        /// The person's own words, exactly as they wrote them (needed for `--channel chat`).
        #[arg(long, value_name = "TEXT")]
        words: Option<String>,
        /// Where they approved: chat (in the conversation), app (a button in the Allomos app) or
        /// confirm (Go ahead on `alm confirm`, which records it by itself).
        #[arg(long, default_value = "chat", value_name = "CHANNEL")]
        channel: String,
        /// The file holding the plan shown to the person, when it is not the tasks' descriptions.
        #[arg(long, value_name = "FILE")]
        plan: Option<PathBuf>,
        /// The last day it holds (YYYY-MM-DD, UTC), when the person gave one.
        #[arg(long, value_name = "DATE")]
        expires: Option<String>,
        /// The approver's pseudonym ID, when it is not this computer's person.
        #[arg(long, value_name = "PSEUDONYM")]
        approver: Option<String>,
    },
    /// Withdraw an approval (when the person takes it back): it no longer applies; the history
    /// keeps it. Say why with `--why`.
    Withdraw {
        #[arg(value_name = "APPROVAL ID")]
        id: String,
    },
    /// Record an outside effect of a task (decision 0021): a deployment, a change to a hosted
    /// resource, SQL run on a real target. Before it: where it acts (--target), what it does
    /// (--action), how it is undone (--rollback), and the approval it relies on (the global
    /// --approval <approval ID>, from `alm approve`). After it: `--of <effect ID> --result
    /// "<what happened>"`. Recorded afterwards: everything at once. `alm effects` lists them.
    Effect {
        id: String,
        /// The effect to record the result of (or correct), by its ID (`ef-…`); without it, a new
        /// one.
        #[arg(long, value_name = "EFFECT ID")]
        of: Option<String>,
        /// Where it acts: the environment or resource (for example "production database").
        #[arg(long, value_name = "TEXT")]
        target: Option<String>,
        /// What is done there.
        #[arg(long, value_name = "TEXT")]
        action: Option<String>,
        /// How it is undone (or why it cannot be).
        #[arg(long, value_name = "TEXT")]
        rollback: Option<String>,
        /// What happened.
        #[arg(long, value_name = "TEXT")]
        result: Option<String>,
    },
    /// A task's outside effects; with none given, every one in the project, by task.
    Effects { id: Option<String> },
    /// Record an exception (decision 0021): a control skipped, why, and who accepted the risk.
    /// --kind check_not_run (a check that was not run: name it with --check), without_human_check
    /// or other; --why (why the control was skipped); --accepted-by (a pseudonym ID or a role, never an email address). Complete
    /// one later with `--of <exception ID>`. `alm done --without-human-check` records one by
    /// itself. `alm exceptions` lists them.
    Exception {
        id: String,
        /// The exception to complete or correct, by its ID (`ex-…`); without it, a new one.
        #[arg(long, value_name = "EXCEPTION ID")]
        of: Option<String>,
        /// check_not_run, without_human_check or other (the default for a new one).
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
        /// The check that was not run (for check_not_run), as the card names it.
        #[arg(long, value_name = "TEXT")]
        check: Option<String>,
        /// Who accepted the risk: a pseudonym ID or a role (for example "approver").
        #[arg(long, value_name = "WHO")]
        accepted_by: Option<String>,
    },
    /// A task's exceptions; with none given, every one in the project, by task.
    Exceptions { id: Option<String> },
    /// Remove personal details written into a task by mistake (decision 0021): every past value
    /// of each --field in the task's history that differs from its current value is replaced by
    /// a marker, and one event records which fields and event files, who approved it (--approved-by)
    /// and why (--reason), never the old values. Correct the current value first (alm describe,
    /// alm title, alm note…). Refused while a legal hold is on.
    Redact {
        id: String,
        /// A field of the task's history, as `alm history` names it (for example description or
        /// handoff_note). Repeatable.
        #[arg(long = "field", value_name = "FIELD", required = true)]
        fields: Vec<String>,
        /// Only these events of the task (`events/<name>.json`, or the name). Repeatable.
        #[arg(long = "event", value_name = "FILE")]
        events: Vec<String>,
        /// Why the details are removed (recorded).
        #[arg(long, value_name = "TEXT")]
        reason: Option<String>,
        /// Who approved the removal: a pseudonym ID or a role (for example "approver").
        #[arg(long, value_name = "WHO")]
        approved_by: Option<String>,
    },
    /// Record a task's change type and risk (decision 0021): --type standard (pre-approved
    /// routine), normal (approved before) or emergency (made first, approved afterwards by a
    /// deadline, --due, three days by default); --risk low, medium or high; --touches production,
    /// others-data, or none. Without options, shows what is recorded and the rules to judge the risk by
    /// (decision 0027: H1–H7, M1–M4 and the project's own; `alm done` lists them too). `alm human` lists
    /// emergencies not approved afterwards.
    Classify {
        id: String,
        #[arg(long = "type", value_name = "standard|normal|emergency")]
        change_type: Option<String>,
        #[arg(long, value_name = "low|medium|high")]
        risk: Option<String>,
        /// What it touches: production, others-data (other people's data), or none. Repeatable
        /// or comma-separated; replaces what was recorded.
        #[arg(long, value_name = "production|others-data|none")]
        touches: Vec<String>,
        /// For an emergency: the last day (YYYY-MM-DD) to record its approval.
        #[arg(long, value_name = "YYYY-MM-DD")]
        due: Option<String>,
    },
    /// Show one task in full.
    Show {
        id: String,
        /// Only its Evidence screen, as the app shows a task to check: every part in the app's
        /// order, "none" for a part it lacks.
        #[arg(long)]
        evidence: bool,
    },
    /// Fill in or correct the session a task records, afterwards (Task-A03-ham538): each option
    /// given replaces that part, the others stay. Any state, by the holder or after done.
    Session {
        id: String,
        /// The AI agent the session runs under (for example `claude-code`).
        #[arg(long, value_name = "AGENT")]
        session_agent: Option<String>,
        /// The agent's own session ID (for Claude Code, what `claude --resume` takes).
        #[arg(long, value_name = "ID")]
        session_id: Option<String>,
        /// Where the session runs (for example `desktop` in the Claude desktop app).
        #[arg(long, value_name = "KIND")]
        session_kind: Option<String>,
        /// The desktop app's ID for the session (in the Claude desktop app, its `local_…` ID).
        #[arg(long, value_name = "ID")]
        app_session_id: Option<String>,
    },
    /// Show a task's history: every recorded change, newest first, with who and when.
    History { id: String },
    /// Hold a task and start it. Say which branch you are on (--branch): alm records the task's
    /// working branch and says which base to start from and merge into, with the Git commands as the
    /// example.
    Claim {
        id: String,
        /// Take the task over from its current holder.
        #[arg(long)]
        take: bool,
        /// The branch you are on, as your version control tool shows it (in Git:
        /// `git branch --show-current`; alm reads no version control). A task without a
        /// base gets it.
        #[arg(long, value_name = "NAME")]
        branch: Option<String>,
        /// The branch you will do the work on, when it is not the one you are on (for example a
        /// new branch for its own worktree).
        #[arg(long, value_name = "NAME")]
        working: Option<String>,
        /// What the Allomos app needs to reopen your session: the AI agent it runs under (for
        /// example `claude-code`). alm stores it and checks nothing.
        #[arg(long, value_name = "AGENT")]
        session_agent: Option<String>,
        /// Your agent's own session ID (for Claude Code, what `claude --resume` takes).
        #[arg(long, value_name = "ID")]
        session_id: Option<String>,
        /// Where your session runs, when your agent needs it to reopen it (for example `desktop`
        /// in the Claude desktop app).
        #[arg(long, value_name = "KIND")]
        session_kind: Option<String>,
        /// The desktop app's ID for your session (in the Claude desktop app, its `local_…` ID).
        #[arg(long, value_name = "ID")]
        app_session_id: Option<String>,
        /// Mark this claim as started by Allomos's automatic monitor (also enabled when
        /// `ALLOMOS_MONITORED=1` is set; Task-A01-mural227). Only monitored sessions write
        /// heartbeats and can be marked lost.
        #[arg(long)]
        monitored: bool,
    },
    /// Change the branch a task starts from and merges into (its base), by its real name.
    Base {
        id: String,
        #[arg(value_name = "BRANCH")]
        branch: String,
    },
    /// Record which requirements (their REQUIREMENTS.md IDs, such as R77) and decisions (their
    /// numbers, such as 0021) a task serves (decision 0021). --remove takes them off. `alm
    /// traces <ref>` finds the tasks of a requirement or decision.
    Trace {
        id: String,
        #[arg(value_name = "REF", required = true)]
        refs: Vec<String>,
        /// Take these off the task instead.
        #[arg(long)]
        remove: bool,
    },
    /// The tasks that serve a requirement (R77) or a decision (0021); with none given, every
    /// requirement and decision some task serves, with its tasks.
    Traces {
        #[arg(value_name = "REF")]
        reference: Option<String>,
    },
    /// Record a commit, branch, merge commit, pull request, CI run or release that goes with a
    /// task (decision 0021): its kind (commit, branch, merge, pr, ci, release), its reference (a
    /// hash, a name, a number, a tag) and, with --url, where it opens. alm runs no Git: you give
    /// what your version control tool and CI show. --remove takes it off. `alm links <ref>`
    /// finds the task of a commit.
    Link {
        id: String,
        #[arg(value_name = "KIND")]
        kind: String,
        #[arg(value_name = "REF")]
        reference: String,
        /// Where it opens: its web address (https://…).
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// Take it off the task instead.
        #[arg(long)]
        remove: bool,
    },
    /// Record one check's result on a task (decision 0029 § 5): its name as you call it (for
    /// example "npm test"), pass, fail or not-run, and with --detail one sentence (what ran, why it
    /// failed or did not run). Record each check you ran or did not run; the same name again
    /// replaces its result. alm runs no check: it records what you report.
    CheckResult {
        id: String,
        /// The check, as you call it (for example "npm test" or "click through in the app").
        #[arg(long, value_name = "CHECK")]
        name: String,
        /// pass, fail or not-run.
        #[arg(long, value_name = "RESULT")]
        result: String,
        /// One sentence: what ran, or why it failed or did not run.
        #[arg(long, value_name = "TEXT")]
        detail: Option<String>,
        /// A screenshot of the failure (PNG, JPEG or WebP, at most 2 MB), only with
        /// --result fail; may be given again; a task keeps at most 3.
        #[arg(long, value_name = "FILE")]
        screenshot: Vec<std::path::PathBuf>,
    },
    /// The tasks a commit (by its hash, 7 digits or more), branch, PR, run, tag or web address
    /// goes with; with none given, every task's links.
    Links {
        #[arg(value_name = "REF")]
        reference: Option<String>,
    },
    /// Add a label to a task, keeping its others (decision 0020). A label the label table does
    /// not list yet is added to it, and alm says so. --remove takes it off the task.
    Label {
        id: String,
        label: String,
        /// Remove the label from the task instead.
        #[arg(long)]
        remove: bool,
    },
    /// Park a task as waiting: a person (or an outside event) must act. Say what, and who when it
    /// is not the approver.
    Wait {
        id: String,
        #[arg(long, value_name = "TEXT")]
        on: String,
        /// Who must act, when it is not the approver (for example Ann).
        #[arg(long, value_name = "NAME")]
        who: Option<String>,
    },
    /// Add blockers to a task: other tasks in this taskbag that must finish first. Its state
    /// stays as it is; it is blocked while one of them has not finished (done or dropped), and
    /// `alm next` does not list it until then.
    Block {
        id: String,
        /// The blocking tasks' IDs (several, or separated by commas).
        #[arg(long, value_name = "TASK-ID", num_args = 1.., required = true)]
        by: Vec<String>,
    },
    /// Take blockers off a task: all of them, or only the ones named with --by. Its state stays
    /// as it is. For when it should not wait for them after all; blockers that finish need no
    /// command.
    Unblock {
        id: String,
        /// Only these blockers (several, or separated by commas); without it, all of them.
        #[arg(long, value_name = "TASK-ID", num_args = 1..)]
        by: Vec<String>,
    },
    /// Finish a task. Finish it at the top label (a person has checked it); when only a person's
    /// hands-on check is left, in any run mode, finish it below it with --without-human-check and
    /// --steps. On a task already done, only its human check is recorded again.
    Done {
        id: String,
        /// Why the task is finished without the human check. Recorded in the task's file; `alm human` lists it under To check.
        /// `--without-owner` is its old name.
        #[arg(long, value_name = "TEXT", alias = "without-owner")]
        without_human_check: Option<String>,
        /// A Markdown file with the steps for the person who checks it: the test folder, what to
        /// open, what to click, what they should see.
        #[arg(long, value_name = "FILE", requires = "without_human_check")]
        steps: Option<PathBuf>,
        /// Who accepted the risk of finishing without the human check: a pseudonym ID or a role
        /// (for example "approver"). Recorded as an exception in the task's history.
        #[arg(long, value_name = "WHO", requires = "without_human_check")]
        accepted_by: Option<String>,
    },
    /// Let go of a task, optionally leaving a handoff note for the next one.
    Release {
        id: String,
        #[arg(long, value_name = "TEXT")]
        note: Option<String>,
    },
    /// Set the task's next dependency (the one thing needed for its next label); no text clears it.
    Needs { id: String, text: Option<String> },
    /// Plan a task in an iteration; no iteration takes it out (it becomes unplanned).
    Plan {
        id: String,
        iteration: Option<String>,
    },
    /// Mark an unheld backlog or waiting task ready (for example, the approver approved it). Its
    /// blockers stay.
    Ready { id: String },
    /// Send an unheld ready or waiting task back to backlog, to be thought over again
    /// (write the open questions in its description's Notes).
    Backlog { id: String },
    /// Give a task a new title.
    Title {
        id: String,
        title: String,
        /// The version you read (`alm show`): if the task changed since, nothing is saved.
        #[arg(long, value_name = "VERSION")]
        expected_version: Option<String>,
    },
    /// Give a task a new description (its description.md): `--text "<markdown>"`, or `--from` a
    /// Markdown file. An empty text (`--text ""`) removes it.
    Describe {
        id: String,
        /// The new description, in Markdown.
        #[arg(
            long,
            value_name = "TEXT",
            required_unless_present = "from",
            conflicts_with = "from"
        )]
        text: Option<String>,
        /// A Markdown file with the new description.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// The version you read (`alm show`): if the task changed since, nothing is saved.
        #[arg(long, value_name = "VERSION")]
        expected_version: Option<String>,
    },
    /// Set the task's handoff note (what the next one needs); no text clears it.
    Note { id: String, text: Option<String> },
    /// Write a heartbeat: the session holding the task (--as) is still at work (decision 0022).
    /// Without an ID, for every task that session holds, in one change. Writes no event, and
    /// nothing while the last heartbeat is under half the interval old (5 of 10 minutes by
    /// default; `alm heartbeat-settings`).
    Beat {
        id: Option<String>,
        /// Declare a busy window: how long you will be quiet (45m, 2h, 1h30m; 0m ends it).
        #[arg(long, value_name = "LENGTH")]
        busy: Option<String>,
        /// Say what you are doing now, in a few words, for whoever may take over; later beats
        /// keep it until you say something else ("" takes it off).
        #[arg(long, value_name = "TEXT")]
        doing: Option<String>,
    },
    /// Set the project folders a task belongs to, by their short names (decision 0019); --none:
    /// no particular folder.
    Folders {
        id: String,
        #[arg(required_unless_present = "none")]
        names: Vec<String>,
        #[arg(long, conflicts_with = "names")]
        none: bool,
    },
    /// Set how hard a task is, from 1 (very easy) to 5 (very hard); --none takes it off (it then
    /// counts as 3). `alm guide tasks` says what each level means.
    Difficulty {
        id: String,
        #[arg(required_unless_present = "none", value_parser = difficulty_level)]
        level: Option<u8>,
        #[arg(long, conflicts_with = "level")]
        none: bool,
    },
    /// The taskbag's project folders (decision 0019): list, add, rename, locate, remove.
    Folder {
        #[command(subcommand)]
        what: FolderWhat,
    },
    /// Upgrade this taskbag to the newest format (allomos/9: blocked worked out from blockers; allomos/8: signed changes, keys and checkpoints,
    /// decision 0021; from allomos/5 or allomos/6 also the label table and several labels per task,
    /// decision 0020; from allomos/5 also project folders, decision 0019). Without --confirm
    /// it only says what it will do. Older Allomos versions
    /// refuse an upgraded taskbag: install the newest Allomos first.
    Upgrade {
        #[arg(long)]
        confirm: bool,
    },
    /// Check the history (decision 0021 § 3.4): recompute the taskbag's fingerprint, check every
    /// signature and each computer's chain of changes, match the checkpoints, and list what was
    /// found (unsigned changes, changed ones, gaps, keys not in keys/). Facts only: it never says
    /// the history is valid or not; you check what they mean.
    Audit,
    /// Make a checkpoint now (decision 0021 § 3.2): a signed file in checkpoints/ that fixes a
    /// fingerprint of the whole taskbag, so a later change to anything it covered is found.
    Checkpoint {
        /// The taskbag repository's current commit, when you know it (alm runs no version
        /// control tool; in Git: `git rev-parse HEAD`).
        #[arg(long, value_name = "COMMIT")]
        git_commit: Option<String>,
    },
    /// Card files still outside the taskbag (linked from a task's description, or reported);
    /// `alm cards found <id>=<path>…` reports others you found (see `alm guide move-cards`).
    Cards {
        #[command(subcommand)]
        what: Option<CardsWhat>,
    },
    /// Move a task's card file in beside it, as tasks/<id>/card.md, and point the description's
    /// `Card:` link to it. Without --from: the file reported with `alm cards found`, else the one
    /// the `Card:` link points to.
    Card {
        id: String,
        /// The card's text (for example a copy whose links you fixed). Default: the reported file.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// Replace a different card.md already there (to keep a moved card up to date).
        #[arg(long)]
        replace: bool,
    },
    /// Change the project's name and/or ID; without either, show them.
    Project {
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        id: Option<String>,
    },
    /// Drop a task: it will not be done.
    Drop { id: String },
    /// Reopen a done or dropped task: it becomes ready, held by nobody, with why in its history.
    /// Its label and human-check record are cleared (both stay in the history).
    Reopen {
        id: String,
        /// Why it is reopened: the more work on the same outcome.
        #[arg(long, value_name = "TEXT")]
        note: String,
    },
    /// Monitor ready tasks without the app: start, check now, stop, and see the runs
    /// (Task-A03-cow112).
    Monitor {
        #[command(subcommand)]
        what: monitor::MonitorWhat,
    },
    /// List the ready tasks that no one holds, with the rule for choosing one.
    Next {
        /// Only tasks planned in this iteration.
        #[arg(long)]
        iteration: Option<String>,
        /// List every task, also when run inside one of the taskbag's project folders (there,
        /// only that folder's tasks are listed).
        #[arg(long)]
        all_folders: bool,
        /// Only tasks of these difficulties: 1-3, or one level such as 4 (no difficulty counts
        /// as 3).
        #[arg(long, value_name = "RANGE", value_parser = difficulty_range)]
        difficulty: Option<allomos_core::difficulty::Range>,
    },
    /// Print the request the app's Update the taskbag… gives an AI agent: for a taskbag written by
    /// an older Allomos, moving its tasks into a new taskbag, like another task system; otherwise,
    /// moving card files kept outside the taskbag in beside their tasks. Changes nothing.
    #[command(name = "move-in")]
    MoveIn {
        /// The language of the request (en, zh-CN, ja). Default: the Allomos app's language.
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
        /// Also ask to translate the taskbag's contents (titles, descriptions, handoff notes, card
        /// files, project notes) into the language CODE, any language code (alone: the request's
        /// language), and make it the task language; without it the request says not to.
        #[arg(long, value_name = "CODE", num_args = 0..=1, default_missing_value = "")]
        translate: Option<String>,
        /// The translation language's name, as the request shows it beside the code.
        #[arg(long, value_name = "NAME", requires = "translate")]
        name: Option<String>,
    },
    /// Print the request the app's Translate the taskbag… gives an AI agent (Task-A05-board831):
    /// rewrite every task's title, description and handoff note, the card files and the project
    /// notes in the language CODE (any language code, such as vi or pt-BR), and make it the task
    /// language. Changes nothing.
    Translate {
        /// The language to translate into, by its code.
        #[arg(value_name = "CODE")]
        code: String,
        /// The language's name, as the request shows it beside the code (for example "Tiếng Việt").
        #[arg(long, value_name = "NAME")]
        name: Option<String>,
        /// The language of the request (en, zh-CN, ja). Default: the Allomos app's language.
        #[arg(long, value_name = "CODE")]
        language: Option<String>,
    },
    /// When to write a handoff: the quota and context gates (with any override for you, --as).
    /// With options, change them.
    #[command(name = "handoff-rules")]
    HandoffRules {
        /// Quota gate: with this percentage of the 5-hour quota left, write the handoff and stop.
        #[arg(long, value_name = "PERCENT")]
        quota: Option<String>,
        /// Context gate: at this percentage of context used, write the handoff and hand over.
        #[arg(long, value_name = "PERCENT")]
        context: Option<String>,
        /// When else to write a handoff, in plain words.
        #[arg(long, value_name = "TEXT")]
        write_when: Option<String>,
        /// Change this agent's or account's own gates (for example codex) instead of the project's.
        #[arg(long, value_name = "NAME")]
        agent: Option<String>,
        /// Back to the defaults (with --agent: remove that agent's own gates).
        #[arg(long)]
        clear: bool,
    },
    /// How often holding sessions beat and after how long a held task is lost (decision 0022 § 3):
    /// project settings, 10 and 30 minutes by default. Without options, show them.
    #[command(name = "heartbeat-settings")]
    HeartbeatSettings {
        /// How often a holding session beats: 10m, 15m, 1h or a number of minutes.
        #[arg(long, value_name = "LENGTH")]
        interval: Option<String>,
        /// After how long without a heartbeat (and no busy window) a held task is lost.
        #[arg(long, value_name = "LENGTH")]
        lost_after: Option<String>,
        /// Whether a session pushes its work in progress at each heartbeat, to prepare for a
        /// takeover: `alm beat` reminds it while on (the default).
        #[arg(long = "wip", value_name = "on|off", value_parser = ["on", "off"])]
        push_work: Option<String>,
        /// Back to the defaults (10 and 30 minutes, pushing on); the other options then still
        /// apply.
        #[arg(long)]
        clear: bool,
    },
    /// The project's line limit for source files (Task-A03-dapper008): a file over it may only get
    /// shorter, task by task; the project's own tests check that (`alm guide file-size`). 1000 by
    /// default, tests not counted. alm keeps and shows the number; it measures no file. Without
    /// options, show it.
    #[command(name = "file-size-settings")]
    FileSizeSettings {
        /// The line limit: a whole number of lines, at least 1.
        #[arg(long, value_name = "LINES")]
        limit: Option<String>,
        /// Back to the default (1000 lines); --limit then still applies.
        #[arg(long)]
        clear: bool,
    },
    /// What an agent does with a task it has just added (Task-A00-wheat560): with
    /// --wait-for-approval on, a new task stays in backlog until the approver agrees (except in a
    /// goal run); off (the default), the agent makes it ready itself when nothing in it is the
    /// person's to decide. alm keeps the setting and words its next steps by it. Without options,
    /// show it.
    #[command(name = "new-task-settings")]
    NewTaskSettings {
        /// Whether new tasks stay in backlog until the approver agrees.
        #[arg(long, value_name = "on|off", value_parser = ["on", "off"])]
        wait_for_approval: Option<String>,
    },
    /// The project's audit settings (decision 0021 § 6): the identity level, separation of
    /// duties, retention and the legal hold, and the outside witness. Without options, show them. The history only ever holds a pseudonym ID at every level; who is
    /// behind it is kept on this computer (local project) or in the cloud (cloud project).
    #[command(name = "audit-settings")]
    AuditSettings {
        /// Where identities come from: pseudonym (the default), account (the Allomos account)
        /// or sso (company sign-in). account and sso need a cloud project.
        #[arg(long, value_name = "LEVEL")]
        identity: Option<String>,
        /// Sign in again to approve or check: on or off (only with account or sso).
        #[arg(long, value_name = "on|off", value_parser = ["on", "off"])]
        reauthenticate: Option<String>,
        /// Separation of duties: on, the same person or agent may not both do and check (or do
        /// and approve) a task, and `alm` refuses it; off (the default) changes nothing. A
        /// project with one person cannot finish its checks while it is on.
        #[arg(long, value_name = "on|off", value_parser = ["on", "off"])]
        separation_of_duties: Option<String>,
        /// Keep the history: forever (the default) or a number of years (for example 7). Allomos
        /// removes nothing when records pass it; it says how many are older. A shorter period
        /// needs a reason.
        #[arg(long, value_name = "forever|YEARS")]
        retention: Option<String>,
        /// Legal hold: on, nothing in the history may be removed or redacted. Turning it on or
        /// off needs a reason.
        #[arg(long, value_name = "on|off", value_parser = ["on", "off"])]
        legal_hold: Option<String>,
        /// Outside witness (decision 0021 § 3.3): on, each checkpoint's fingerprint (a hash, never
        /// the content) is sent to a time-stamp service (RFC 3161) whose receipt is kept in the
        /// checkpoint. Turning it on needs the person's yes (--confirm); off needs a reason.
        #[arg(long, value_name = "on|off", value_parser = ["on", "off"])]
        witness: Option<String>,
        /// The time-stamp service: freetsa (the default), digicert, sectigo, globalsign, or your
        /// own service's address (https://…).
        #[arg(long, value_name = "SERVICE")]
        witness_service: Option<String>,
        /// A second time-stamp service, sent each checkpoint too (none: remove it).
        #[arg(long, value_name = "SERVICE")]
        witness_second: Option<String>,
        /// How often checkpoints are made by themselves while the outside witness is on: daily
        /// (the default when turning it on: by the app while open and by any alm command that
        /// changes the taskbag, once a day), approvals (after each approval and human check), or
        /// asked (only `alm checkpoint`, Make a checkpoint now, and upgrading).
        #[arg(long, value_name = "daily|approvals|asked", value_parser = ["daily", "approvals", "asked"])]
        witness_when: Option<String>,
        /// The person said yes to sending fingerprints to the service (needed with --witness on).
        #[arg(long)]
        confirm: bool,
        /// Why, recorded in the history (needed when turning a control off or down).
        #[arg(long, value_name = "TEXT")]
        reason: Option<String>,
    },
    /// Hand a task off (--as your session): its handoff note becomes this text (where it stopped,
    /// the one next step, who it waits on), --evidence is added under its description's Evidence,
    /// and its history records who handed it off. Several tasks: once for each.
    Handoff {
        id: String,
        /// The handoff note's text.
        #[arg(
            long,
            value_name = "TEXT",
            conflicts_with = "from",
            required_unless_present = "from"
        )]
        text: Option<String>,
        /// A Markdown file with the handoff note's text.
        #[arg(long, value_name = "FILE")]
        from: Option<PathBuf>,
        /// What was done that must not run twice (an outside effect), for the description's Evidence.
        #[arg(long, value_name = "TEXT")]
        evidence: Option<String>,
    },
}
