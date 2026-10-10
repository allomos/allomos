// Shapes returned by the desktop app. They mirror the Rust types in crates/core and crates/store.

export type State = "backlog" | "ready" | "in_progress" | "waiting" | "done" | "dropped";

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
  waiting_on: string | null;
  next_dependency: string | null;
  iteration: string | null;
  handoff_note: string | null;
  description: string;
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

/** Whether the project folder has its agent instruction files (apps/desktop: `instruction_files`). */
export interface InstructionFiles {
  agents_md: boolean;
  claude_md: boolean;
}
