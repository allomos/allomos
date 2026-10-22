// Shapes returned by the desktop app. They mirror the Rust types in crates/core and crates/store.

export type State = "backlog" | "ready" | "in_progress" | "waiting" | "blocked" | "done" | "dropped";

export interface Project {
  format: string;
  id: string;
  name: string;
  labels: string[];
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

export interface Task {
  id: string;
  title: string;
  area: string;
  state: State;
  label: string | null;
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
  /** Why the task was finished without the human check (Task-A03-15, A03-18, A03-31). */
  without_human_check: string | null;
  /** How a person checks it, in Markdown (Task-A03-18). */
  human_check_steps: string | null;
  description: string;
  /** Decisions an AI agent made here on its own and nobody has reviewed yet (Task-A03-19). Not
   * a field of task.json: worked out from the history and added when the taskbag is opened. */
  ai_decisions?: AiDecision[];
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
  | { field: "waiting_on"; text: string }
  | { field: "blocked_by"; ids: string[] }
  | { field: "next_dependency"; text: string | null }
  | { field: "handoff_note"; text: string | null }
  | { field: "iteration"; iteration: string | null }
  | { field: "title"; title: string; expected_version?: string | null }
  | { field: "description"; text: string; expected_version?: string | null }
  | { field: "checked" }
  | { field: "reopen"; note: string };

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
  /** Each task's base and working branch, by task ID (Task-A03-22). */
  branches?: Record<string, Branches>;
  /** When each task last changed (UTC), by task ID, from the history (Task-A01-52). */
  last_changed?: Record<string, string>;
  /** Card files agents reported as still in the project folder (Task-A00-30). */
  cards_outside?: CardOutside[];
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

/** One handoff file (crates/store/src/handoff.rs, Task-A03-06). Times are `YYYY-MM-DDTHH:MMZ`, UTC. */
export interface Handoff {
  file: string;
  writer: string;
  tasks: string[];
  written: string;
  updated: string;
  summary: string;
  body: string;
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
export interface HandoffSettings {
  rules: HandoffRules;
  /** With the defaults filled in. */
  applied: { quota_gate: number; context_gate: number; write_when: string; override_for: string | null };
  template: string;
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
