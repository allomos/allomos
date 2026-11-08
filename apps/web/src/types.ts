// Shapes returned by the desktop app. They mirror the Rust types in crates/core and crates/store.

export type State = "backlog" | "ready" | "in_progress" | "waiting" | "blocked" | "done" | "dropped";

export interface Project {
  format: string;
  id: string;
  name: string;
  /** The label table (`allomos/7`, decision 0020): every label the project knows, the built-in
   * ones first. */
  labels: LabelEntry[];
  /** `ui` (follow the interface), a language code such as `ja`, or missing (follow the interface). */
  task_language?: string;
  /** `allomos/6`: the project folders, the first the main one (decision 0019). */
  project_folders?: { name: string; path: string }[];
}

/** One label of the project's label table (decision 0020): its name, an optional meaning, and
 * whether it is one of Allomos's five built-in labels. Every label is plain: a task has it or not. */
export interface LabelEntry {
  name: string;
  meaning?: string;
  built_in?: boolean;
}

export interface Area {
  id: string;
  name: string;
  order: number;
}

/** When tasks are planned: a time-box that ends in a release (R89). */
/** Where an iteration is (Task-A01-root343): planned (also when missing), active (at most one), closed. */
export type IterationState = "planned" | "active" | "closed";

export interface Iteration {
  id: string;
  name: string;
  order: number;
  start?: string;
  end?: string;
  release?: string;
  state?: IterationState;
}

/** What an agent passed at `alm claim` so the app can reopen its session (Task-A03-hive049). */
export interface AgentSession {
  /** The session's `--as` name. */
  by?: string;
  agent?: string;
  /** The agent's own session ID (`claude --resume` takes it). */
  id?: string;
  /** Where it runs, e.g. `desktop` for the Claude desktop app. */
  kind?: string;
  /** The desktop app's ID for it (Claude's `local_…`). */
  app_session_id?: string;
  /** The folder `alm claim` ran in, relative to the folder holding `.allomos/`. */
  folder?: string;
}

export interface Task {
  id: string;
  title: string;
  area: string;
  /** The project folders it belongs to (`allomos/6`, decision 0019); missing: none in particular. */
  folders?: string[];
  state: State;
  /** The labels the task carries (`allomos/7`, decision 0020), in the order they were added;
   * left out when none. */
  labels?: string[];
  holder: string | null;
  /** For a waiting task: what a person must do. */
  waiting_on: string | null;
  /** For a waiting task: who must act; none means the approver ("Waiting for you"). Decision 0016. */
  who?: string | null;
  /** For a blocked task: the tasks that must finish first (left out when none). Decision 0016. */
  blocked_by?: string[];
  next_dependency: string | null;
  iteration: string | null;
  handoff_note: string | null;
  /** Why the task was finished without the human check (Task-A03-15, A03-18, A03-31): text for
   * the person who checks; since `allomos/7` it decides nothing (To check is worked out from the
   * labels, decision 0020). */
  without_human_check: string | null;
  /** How a person checks it, in Markdown (Task-A03-18). */
  human_check_steps: string | null;
  /** The agent session that holds it or last held it, as passed at `alm claim` (Task-A03-hive049). */
  session?: AgentSession | null;
  description: string;
  /** Decisions an AI agent made here on its own and nobody has reviewed yet (Task-A03-19). Not
   * a field of task.json: worked out from the history and added when the taskbag is opened. */
  ai_decisions?: AiDecision[];
  /** Its approvals, with where each stands now (Task-A01-coach782). Not a field of task.json:
   * worked out from the history and added when the taskbag is opened. */
  approvals?: ApprovalRecord[];
  /** Its outside effects (Task-A01-job650). Not a field of task.json: worked out from the
   * history and added when the taskbag is opened. */
  effects?: EffectRecord[];
  /** Its links to code and releases (Task-A01-mint242). Not a field of task.json: worked out
   * from the history and added when the taskbag is opened. */
  links?: LinkRecord[];
}

/** A link to code or a release as recorded (crates/core/src/link.rs, decision 0021 § 1.1): a
 * commit, branch, merge commit, pull request, CI run or release, and where it opens. */
export interface LinkRecord {
  kind: "commit" | "branch" | "merge" | "pr" | "ci" | "release";
  ref: string;
  /** Missing when none was given: then it cannot be opened. */
  url?: string;
  task: string;
  at: string;
  by?: string;
}

/** An outside effect as recorded (crates/core/src/effect.rs, decision 0021 § 1.1): where it
 * acts, what it does, how it is undone, the approval it relies on and what happened. */
export interface EffectRecord {
  id: string;
  task: string;
  target: string;
  action: string;
  rollback: string;
  approval?: string;
  /** Missing while it is under way. */
  result?: string;
  at: string;
  by?: string;
  result_at?: string;
}

/** A person's approval as recorded (crates/core/src/approval.rs, decision 0021 § 1.1). */
export interface ApprovalRecord {
  id: string;
  /** The approver's pseudonym ID. */
  approver?: string;
  scope: { tasks: string[]; plan_sha256: string; plan: string };
  at: string;
  channel: "chat" | "app" | "confirm" | string;
  words?: string;
  expires?: string;
  recorded_by?: string;
  withdrawn_at?: string;
  withdrawn_by?: string;
  status: "applies" | "plan_changed" | "expired" | "withdrawn";
}

/** A task's branches (crates/core/src/branch.rs, Task-A03-22): the base it starts from and
 * merges into, and the branch the work is done on. Not fields of task.json: from the history. */
export interface Branches {
  base?: string;
  working?: string;
}

/** One decision an agent recorded with `alm ai-decided` (crates/core/src/decision.rs). */
export interface AiDecision {
  at: string;
  by?: string;
  text: string;
}

/**
 * One change the task pane asks for (Task-A01-18), as `crates/core/src/ops.rs` `Edit` reads it.
 * Each goes through the same rule as its `alm` command; Rust decides whether it is allowed.
 */
export type Edit =
  | { field: "state"; state: State; waiting_on?: string | null; who?: string | null; blocked_by?: string[] }
  | { field: "label"; label: string }
  | { field: "labels"; labels: string[] }
  | { field: "waiting_on"; text: string }
  | { field: "blocked_by"; ids: string[] }
  | { field: "next_dependency"; text: string | null }
  | { field: "handoff_note"; text: string | null }
  | { field: "iteration"; iteration: string | null }
  | { field: "title"; title: string; expected_version?: string | null }
  | { field: "description"; text: string; expected_version?: string | null }
  | { field: "checked" }
  | { field: "reopen"; note: string }
  | { field: "folders"; folders: string[] };

/**
 * A new task from the New task form (Task-A01-19), as `crates/store` `NewTask` reads it. No ID
 * means the next free one in the area; with an iteration it is also planned there (two events).
 */
export interface NewTask {
  id: string | null;
  area: string;
  title: string;
  description: string | null;
  iteration: string | null;
  /** The base branch the person chose, or that every open task shares (Task-A03-22). */
  base: string | null;
  /** The project folders it belongs to: the Folder filter's, when one is chosen (Task-A01-sketch327). */
  folders?: string[];
}

/** A new iteration from the New iteration form (Task-A01-19). No ID means the next free one. */
export interface NewIteration {
  id: string | null;
  name: string;
  release: string | null;
  start: string | null;
  end: string | null;
}

/** What the Edit form of an iteration sends (Task-A01-29): every field; an empty release or date clears it. */
export interface IterationChange {
  name: string;
  release: string;
  start: string;
  end: string;
  state: IterationState;
}

/** The IDs the New forms suggest, each the next free one, worked out in Rust (Task-A01-19). */
export interface NextIds {
  area: string;
  iteration: string;
  /** For each area, the next free task ID in it. */
  tasks: Record<string, string>;
}

/** One field's change in an event (crates/core/src/event.rs). A missing side was empty. */
export interface FieldChange {
  field: string;
  from?: unknown;
  to?: unknown;
}

/** One recorded change (Task-A01-17). */
export interface TaskEvent {
  at: string;
  by?: string;
  via: string;
  command: string;
  kind: "task" | "area" | "iteration" | "project";
  id?: string;
  /** The AI agent `alm` ran under (`claude`, `codex`), when recorded (Task-A03-27). */
  agent?: string;
  tool?: string;
  changes: FieldChange[];
}

/** When a name last changed anything in the taskbag, and its newest recorded agent (Task-A03-37). */
export interface Seen {
  by: string;
  at: string;
  agent?: string;
  tool?: string;
}

/** A task's history: newest first, and when the taskbag started recording (null: not yet). */
export interface TaskHistory {
  since: string | null;
  events: TaskEvent[];
  /** Everyone named in the task's events, with their newest change anywhere in the taskbag. */
  seen?: Seen[];
  /** This computer's person's pseudonym in the project (Task-A01-frolic587): shown with their own name. */
  you?: string;
}

/** A problem from Rust: a code the interface words in the chosen language (see i18n/codes.json). */
export interface Problem {
  code: string;
  params?: Record<string, string>;
}

export function isProblem(v: unknown): v is Problem {
  return typeof v === "object" && v !== null && typeof (v as Problem).code === "string";
}

export interface Issue {
  file: string;
  field: string | null;
  problem: Problem;
}

export interface Loaded {
  taskbag: { project: Project; areas: Area[]; iterations: Iteration[]; tasks: Task[] } | null;
  issues: Issue[];
  /** The active iteration's ID: the one marked active, or else the first planned one (Task-A01-root343). */
  active_iteration?: string | null;
}

export type Inspection =
  | { kind: "found"; root: string }
  | { kind: "empty"; folder: string; suggested_root: string }
  | { kind: "no_taskbag"; folder: string; suggested_root: string }
  | { kind: "not_a_folder"; path: string };

export interface FolderInfo {
  inspection: Inspection;
}

export interface Opened {
  root: string;
  loaded: Loaded;
  /** README-allomos.md was just written next to .allomos/ (only when creating). */
  readme_added: boolean;
  /** Each task's AI decisions not reviewed yet, by task ID (Task-A03-19). */
  ai_decisions?: Record<string, AiDecision[]>;
  /** Each task's approvals, by task ID (Task-A01-coach782). */
  approvals?: Record<string, ApprovalRecord[]>;
  /** Each task's outside effects, by task ID (Task-A01-job650). */
  effects?: Record<string, EffectRecord[]>;
  /** Each task's links to code and releases, by task ID (Task-A01-mint242). */
  links?: Record<string, LinkRecord[]>;
  /** Each task's base and working branch, by task ID (Task-A03-22). */
  branches?: Record<string, Branches>;
  /** When each task last changed (UTC), by task ID, from the history (Task-A01-52). */
  last_changed?: Record<string, string>;
  /** Card files agents reported as still in the project folder (Task-A00-30). */
  cards_outside?: CardOutside[];
  /** An older taskbag's handoff files in `handoffs/` (Task-A03-polar770): the taskbag needs updating. */
  old_handoffs?: string[];
  /** Each task's version, by task ID (Task-A01-57): sent back when a title or description is saved. */
  versions?: Record<string, string>;
}

/**
 * A card file still outside the taskbag (Task-A00-30): the task's `Card:` link points there
 * (Task-A01-51), or an agent reported it (`alm cards found`). One per task; a report wins.
 */
export interface CardOutside {
  task: string;
  /** Relative to the project folder, with `/`; a linked file outside it is named in full. */
  path: string;
  source: "link" | "report";
  /** When it was reported (reports only). */
  at?: string;
  by?: string;
}

/** An AI agent found on this computer (see apps/desktop/src-tauri/src/agents.rs). */
export interface Agent {
  id: "claude_cli" | "claude_app" | "codex_cli" | "codex_app";
  /** The command's path, or the app's name. */
  found: string;
  /** "link": opens with the request typed in; "paste": a terminal opens and the request is on the clipboard. */
  how: "link" | "paste";
}

/** The agent `alm` last ran under for a task (crates/store `events::last_agent`, Task-A03-27). */
export interface LastAgent {
  /** `claude` or `codex`. */
  agent: string;
  /** `claude_cli` or `claude_app`, when the environment told it. */
  tool: string | null;
  /** UTC. */
  at: string;
}

/** Do the AI agents know about this project? (apps/desktop: `agent_status`) */
export interface AgentStatus {
  project: string;
  /** Which agents' instruction files tell them about Allomos. */
  setup: { codex: boolean; claude: boolean };
  /** The newest change an agent made through `alm` under a name, from the change history. `at` is UTC. */
  last: { who: string; at: string } | null;
  /** Changes made in the app that no agent has been told to share yet (decision 0017). */
  unshared: number;
  /** Who holds tasks in progress now. */
  holders: string[];
}

export interface Gates {
  quota_gate?: number;
  context_gate?: number;
}

/** The project's handoff rules as written in project.json; a missing field takes its default. */
export interface HandoffRules extends Gates {
  write_when?: string;
  overrides?: Record<string, Gates>;
}

/** The handoff settings as the dialog shows them. */
/** Where identities come from (decision 0021 § 2, Task-A01-frolic587). */
export type IdentityLevel = "pseudonym" | "account" | "sso";

/** Project settings → Audit (decision 0021 § 6): the identity level and separation of duties. */
export interface AuditSettings {
  identity: {
    level: IdentityLevel;
    /** As `project.json` writes it, when this Allomos does not know it (read as the pseudonym). */
    written?: string;
    reauthenticate: boolean;
    /** Only a cloud project can choose the sign-in levels; this Allomos has none yet. */
    sign_in_available: boolean;
  };
  /** Separation of duties (Task-A01-grotto036): the same person or agent may not both do and
   * check (or do and approve) a task. Off by default. */
  separation_of_duties: boolean;
  /** Where this computer keeps who is behind its pseudonyms; null without a settings folder. */
  identities_file: string | null;
}

export interface HandoffSettings {
  rules: HandoffRules;
  /** With the defaults filled in. */
  applied: { quota_gate: number; context_gate: number; write_when: string; override_for: string | null };
}

/** A taskbag's format compared with this Allomos's (crates/store/src/versions.rs). */
export type FormatState =
  | { state: "current" }
  /** Written by an older Allomos, in any older format: not read; its tasks are moved in (Task-A01-45). */
  | { state: "older"; found: string }
  | { state: "newer"; found: string }
  | { state: "unknown" };

/** Is the project up to date with this Allomos? (crates/store/src/versions.rs, Task-A01-25) */
export interface VersionCheck {
  format_current: string;
  section_current: number;
  format: FormatState;
  /** An older Allomos's taskbag whose tasks are still to be moved in (Task-A01-45): the taskbag
   * itself (`prepared` false: Update the taskbag… renames it aside), or one already renamed aside
   * to `old` beside a new taskbag that has no tasks yet. */
  move_in: { old: string; found: string; prepared: boolean } | null;
  /** Instruction files holding Allomos's section; `version` null when its heading has none. */
  sections: { file: string; version: number | null }[];
  /** A taskbag this Allomos reads as it is but writes newer (`allomos/5`): its format, so Update
   * the Taskbag… offers the upgrade (Task-A05-fish286); null otherwise. */
  upgrade?: string | null;
}

/** A question an agent asks with `alm confirm` (crates/store/src/confirm.rs). */
export interface ConfirmRequest {
  id: string;
  title: string;
  /** Markdown. */
  plan: string;
  from: string | null;
  /** Seconds since 1970. */
  asked: number;
  until: number;
  /** What shows the plan is sound, in Markdown, shown under it (`--evidence`, Task-A03-37). */
  evidence?: string | null;
  /** The task the approval is for (`--task`). */
  task?: string | null;
  /** A fingerprint of exactly what is shown: a Go ahead counts only for it. */
  fingerprint?: string;
}

/** What an AI agent reports about a job the app gave it (`alm progress`, Task-A05-08). */
export interface Progress {
  /** One report per agent. */
  id: string;
  title: string;
  step?: string;
  done?: number;
  of?: number;
  /** The job is over: what happened. */
  finished?: string;
  /** The agent asked the person something and waits for the answer (Task-A03-aster771). */
  waiting?: string;
  from?: string;
  /** Seconds since 1970. */
  started: number;
  updated: number;
}

/** Where the open taskbag stands in the open-folder flow (crates/store/src/setup.rs). */
export interface SetupState {
  /** The folder holding `.allomos/`. */
  allomos_folder: string;
  /** The recorded project folder; null until known. */
  project_folder: string | null;
  /** Which agents' instruction files in the project folder mention Allomos. */
  agents: { codex: boolean; claude: boolean };
  has_tasks: boolean;
}

/** Whether the project folder has its agent instruction files (apps/desktop: `instruction_files`). */
export interface InstructionFiles {
  agents_md: boolean;
  claude_md: boolean;
}

/** Whether to offer **Install the alm command** (apps/desktop/src-tauri/src/alm_command.rs, Task-A07-12, A07-14, A07-15). */
export interface AlmStatus {
  offer: boolean;
  /** Which `alm` Terminal uses: none, one built from source, or another program. */
  kind?: "missing" | "development" | "other" | null;
  /** The app's own `alm`. */
  own?: string | null;
  /** The `alm` Terminal finds now, when it is not the app's own. */
  found: string | null;
  /** Where `found` leads, when it is a link. */
  leads_to?: string | null;
  /** A real file that installing would replace. */
  replaces_file: string | null;
  /** An `alm` Terminal finds first, which would still be used after installing. */
  shadowed_by: string | null;
  /** What "Don't ask again" remembers: the path Terminal's `alm` leads to, empty when there is none (Task-A07-14). */
  mismatch?: string | null;
}

export interface AlmInstalled {
  cancelled: boolean;
  status: AlmStatus;
}

/** Whether a newer Allomos is on GitHub (Task-A07-16). */
export interface UpdateStatus {
  /** This Allomos's version. */
  current: string;
  /** The newest version known, from this check or the last one that worked. */
  latest: string | null;
  /** The version to offer: newer than this one, and not skipped (unless asked from the menu). */
  newer: string | null;
  /** A newer version than this one, even a skipped one: the menu offers it. */
  available: string | null;
  /** GitHub was asked now and did not answer. */
  failed: boolean;
  /** `available`'s installer is downloaded and its hash matched (Task-A07-17). */
  ready: string | null;
}

/** How much of an agent account's quota is left, as the agent reported it (`alm quota`, Task-A03-02). */
export interface QuotaReport {
  /** `claude-code`, `codex`, or another agent's id. */
  agent: string;
  /** A name for the account, only when the agent could tell one. */
  account?: string | null;
  /** `5h` or `week`. */
  window: string;
  /** Share left, 0–100. */
  left: number;
  /** When the window resets, local time (`2026-10-20T17:00`). */
  resets?: string | null;
  /** When it was reported, UTC. */
  at: string;
}
