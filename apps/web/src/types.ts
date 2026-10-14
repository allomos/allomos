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
export interface Iteration {
  id: string;
  name: string;
  order: number;
  start?: string;
  end?: string;
  release?: string;
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
  /** For a waiting task: who must act; none means the owner ("Waiting for you"). Decision 0016. */
  who?: string | null;
  /** For a blocked task: the tasks that must finish first (left out when none). Decision 0016. */
  blocked_by?: string[];
  next_dependency: string | null;
  iteration: string | null;
  handoff_note: string | null;
  /** Why the task was finished without the owner's check (Task-A03-15). */
  finished_without_owner: string | null;
  description: string;
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
  | { field: "title"; title: string }
  | { field: "description"; text: string };

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
  changes: FieldChange[];
}

/** A task's history: newest first, and when the taskbag started recording (null: not yet). */
export interface TaskHistory {
  since: string | null;
  events: TaskEvent[];
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
  git_root: string | null;
}

export interface Opened {
  root: string;
  loaded: Loaded;
  /** README-allomos.md was just written next to .allomos/ (only when creating). */
  readme_added: boolean;
}

/** An AI agent found on this computer (see apps/desktop/src-tauri/src/agents.rs). */
export interface Agent {
  id: "claude_cli" | "claude_app" | "codex_cli" | "codex_app";
  /** The command's path, or the app's name. */
  found: string;
  /** "link": opens with the request typed in; "paste": a terminal opens and the request is on the clipboard. */
  how: "link" | "paste";
}

/** Do the AI agents know about this project? (apps/desktop: `agent_status`) */
export interface AgentStatus {
  project: string;
  /** Which agents' instruction files tell them about Allomos. */
  setup: { codex: boolean; claude: boolean };
  /** The newest taskbag change an agent made with `alm --commit` / `--push`. `when` is in seconds. */
  last: { who: string; when: number } | null;
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
  | { state: "older"; found: string }
  | { state: "too_old"; found: string; oldest: string }
  | { state: "newer"; found: string }
  | { state: "unknown" };

/** Is the project up to date with this Allomos? (crates/store/src/versions.rs, Task-A01-25) */
export interface VersionCheck {
  format_current: string;
  section_current: number;
  format: FormatState;
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

/** Where the setup wizard keeps the taskbag in Git (apps/desktop: `keep_in_git`). */
export interface GitKept {
  repo: string;
  /** The folder was not in a repository, so it was made one. */
  initialized: boolean;
  /** A commit was made (false: already committed as it is). */
  committed: boolean;
}

/** Whether the project folder has its agent instruction files (apps/desktop: `instruction_files`). */
export interface InstructionFiles {
  agents_md: boolean;
  claude_md: boolean;
}

/** Whether to offer **Install the alm command** (apps/desktop/src-tauri/src/alm_command.rs, Task-A07-12). */
export interface AlmStatus {
  offer: boolean;
  /** The `alm` Terminal finds now, when it is not the app's own. */
  found: string | null;
  /** A real file that installing would replace. */
  replaces_file: string | null;
  /** An `alm` Terminal finds first, which would still be used after installing. */
  shadowed_by: string | null;
}

export interface AlmInstalled {
  cancelled: boolean;
  status: AlmStatus;
}
