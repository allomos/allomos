// The messenger: the only place the interface talks to Allomos's Rust side.
//
// Inside the desktop window it uses Tauri's built-in channel. When phone and browser
// companions arrive (Iteration 9), a second implementation will talk to the desktop's
// local server instead; the rest of the interface does not change.

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-dialog";
import { withDecisions } from "./stopFlow";
import type { Agent, AgentStatus, AlmInstalled, AlmStatus, Area, ConfirmRequest, Edit, FolderInfo, FormatState, HandoffSettings, InstructionFiles, Iteration, IterationChange, LastAgent, NewIteration, NewTask, NextIds, Opened, Problem, Progress, QuotaReport, SetupState, Task, TaskHistory, UpdateStatus, VersionCheck } from "./types";

/** What the About window shows of the app itself (Task-A07-28). */
export interface AboutInfo {
  name: string;
  version: string;
}

export interface Messenger {
  /** A folder given when Allomos was started, if any. Rejects with a Problem when the given path is not a folder. */
  startupFolder(): Promise<string | null>;
  /** The computer's preferred languages, most preferred first. */
  systemLanguages(): Promise<string[]>;
  /** Saves the interface language, so `alm task-language` can tell agents which language to write tasks in. */
  saveUiLanguage(language: string): Promise<void>;
  /** Ask the user to pick a folder. Returns null if they cancel. */
  pickFolder(title: string): Promise<string | null>;
  inspectFolder(path: string): Promise<FolderInfo>;
  openTaskbag(root: string): Promise<Opened>;
  /** A fingerprint of the taskbag's files; it changes whenever a file is written. */
  taskbagStamp(root: string): Promise<string>;
  /**
   * Opens a link from task `task`'s description: a web page, or a text file inside the project
   * (such as the task's card) in the user's editor. Rejects with a Problem for anything else.
   */
  openTaskLink(root: string, task: string, link: string): Promise<void>;
  /** Show the taskbag's folder in Finder or Explorer. Rejects with a Problem on failure. */
  showInFileManager(root: string): Promise<void>;
  /**
   * Rejects with a Problem (see types.ts) when the taskbag cannot be created. In a folder that
   * looks empty it also writes README-allomos.md in `language` (`readme_added` says so).
   */
  createTaskbag(root: string, name: string, language: string): Promise<Opened>;
  /** Opens another window on the start page (Task-A01-27). */
  newWindow(): Promise<void>;
  /** Tells the app which project this window shows (null: none), and names the window after it. */
  setWindowProject(root: string | null, name: string | null): Promise<void>;
  /** When another window already shows `root`, brings it forward and resolves to true. */
  showProjectElsewhere(root: string): Promise<boolean>;
  /** Calls `handler` when this window comes to the front. */
  onWindowFocus(handler: () => void): Promise<() => void>;
  /** Calls `handler` when File → Open Recent → Clear Menu is chosen. */
  onClearRecentMenu(handler: () => void): Promise<() => void>;
  /** Puts the app menu's words in the interface's language. */
  setMenuTexts(texts: MenuTexts): Promise<void>;
  /** Shows these folders, newest first, under File → Open Recent. */
  setRecentFolders(folders: string[]): Promise<void>;
  /** Calls `handler` when File → Connect with the Agents… is chosen. */
  onConnectMenu(handler: () => void): Promise<() => void>;
  /** Calls `handler` when File → Update the Taskbag… is chosen (Task-A00-30, Task-A01-45). */
  onMoveTasksInMenu(handler: () => void): Promise<() => void>;
  /** Calls `handler` when File → Handoff Settings… is chosen (Task-A03-polar770). */
  onHandoffSettingsMenu(handler: () => void): Promise<() => void>;
  /** Calls `handler` when File → Devices on Your Network… is chosen (Task-A04-02-01). */
  onLanMenu(handler: () => void): Promise<() => void>;
  /** "Allow devices on your network to view": whether it is on, and the address while it is. */
  lanStatus(): Promise<LanStatus>;
  /** Turns it on or off, remembered; resolves to the new status (with `error` when it could not start). */
  setLanViewing(on: boolean): Promise<LanStatus>;
  /** Devices asking to pair and the paired devices (Task-A04-02-02). */
  lanPairing(): Promise<PairingView>;
  /** Approve (a code is shown) or refuse a device's request to pair. */
  answerPairing(id: string, approve: boolean): Promise<PairingView>;
  /** Remove a paired device: its access ends at once; the removal is recorded in each open taskbag. */
  removePairedDevice(id: string): Promise<PairingView>;
  /** Calls `handler` when a device asks to pair, or a request changes. */
  onPairingChanged(handler: () => void): Promise<() => void>;
  /** Update the taskbag… (Task-A01-45: one button for an older taskbag and for card files kept
   * outside the taskbag): opens `agent` in the project folder of the taskbag at `taskbag` with the
   * request for whichever applies typed in (`alm move-in` prints it; Task-A03-27: an out-of-date
   * section too). Nothing is sent. Rejects with a Problem. */
  moveTasksIn(agent: string, taskbag: string, language: string, translate: boolean): Promise<void>;
  /** Update the taskbag… with Manual: the same request, to copy. */
  moveTasksInRequest(taskbag: string, language: string, translate: boolean): Promise<string>;
  /** The agent `alm` last ran under for a task in the taskbag at `root`, from its events (Task-A03-27). */
  lastAgent(root: string): Promise<LastAgent | null>;
  /** The AI agents on this computer that can be asked to add Allomos's instructions. */
  findAgents(): Promise<Agent[]>;
  /**
   * Opens each agent in `project` with its request typed in: the first also looks for the
   * project's tasks and offers to move them; the others only add Allomos's instructions.
   * Resolves to the ones that opened. Rejects with a Problem.
   */
  connectAgents(agents: string[], project: string, taskbag: string, language: string): Promise<string[]>;
  /** The Manual choice (Task-A05-06): the request automatic sending would type in, to copy. Rejects with a Problem. */
  manualRequest(project: string, taskbag: string, language: string): Promise<string>;
  /** Puts `text` on the clipboard. Rejects with a Problem. */
  copyText(text: string): Promise<void>;
  /** Build a new app… (Task-A03-25-02): opens `agent` in `project` with one request typed in
   * (connect, then the goal prompt with `description` and the run mode). Nothing is sent. Rejects with a Problem. */
  buildApp(agent: string, project: string, taskbag: string, language: string, description: string, quiet: boolean): Promise<void>;
  /** Build a new app… with Manual: the same request, to copy. Rejects with a Problem. */
  buildAppRequest(project: string, taskbag: string, language: string, description: string, quiet: boolean): Promise<string>;
  /** The explanation of quiet and normal mode (`alm guide run-modes`, the person's part), as Markdown. */
  runModesText(language: string): Promise<string>;
  /** Whether `project` has AGENTS.md and CLAUDE.md. */
  instructionFiles(project: string): Promise<InstructionFiles>;
  /** The entries under `## Agents set up` in the taskbag's project notes (Task-A03-24): which agents
   * are registered, for Update the taskbag…'s choice of agents (Task-A03-27). */
  agentsSetUp(taskbag: string): Promise<string[]>;
  /** Writes Allomos's section into the project's AGENTS.md and CLAUDE.md; resolves to the files changed. */
  writeAgentFiles(project: string): Promise<string[]>;
  /** Whether the project of the taskbag at `root` is up to date with this Allomos; null until an agent is connected. */
  versionCheck(root: string): Promise<VersionCheck | null>;
  /** Update the taskbag… on an older Allomos's taskbag (Task-A01-45): rename it aside and create a new,
   * empty one in its place; the new one, loaded. */
  moveTaskbagAside(root: string): Promise<Opened>;
  /** The taskbag's format compared with this Allomos's, connected or not. */
  taskbagFormat(root: string): Promise<{ format: FormatState; current: string }>;
  /** The project's handoff rules (Task-A03-06). */
  /** Renames the project, as `alm project --name` (Task-A05-chalk937); resolves to the saved name. */
  renameProject(root: string, name: string): Promise<string>;
  handoffSettings(root: string): Promise<HandoffSettings>;
  /** Changes the project's handoff rules, or one agent's override; resolves to the settings as saved. Rejects with a Problem. */
  setHandoffRules(
    root: string,
    change: { quota?: string; context?: string; writeWhen?: string; agent?: string; clear?: boolean },
  ): Promise<HandoffSettings>;
  /** One task's recorded changes, newest first (Task-A01-17). */
  taskHistory(root: string, id: string): Promise<TaskHistory>;
  /** The person's name the app records its task changes under; null until it has been asked (Task-A01-18). */
  ownerName(): Promise<string | null>;
  /** Keeps the person's name in the app's settings. Rejects with a Problem. */
  saveOwnerName(name: string): Promise<void>;
  /**
   * Changes one field of task `id` under the saved name, by the same rule as its `alm` command,
   * under the taskbag's lock, recorded in its history. Resolves to the task as saved; rejects with
   * the same Problem `alm` gives, and then nothing was changed.
   */
  editTask(root: string, id: string, edit: Edit): Promise<Task>;
  /**
   * The person has reviewed task `id`'s AI decisions (Task-A03-19), like `alm reviewed`: one
   * `reviewed` event under the lock. Rejects with a Problem (`nothing_to_review`, `holder_missing`).
   */
  reviewAiDecisions(root: string, id: string): Promise<void>;
  /** The IDs the New forms suggest: the next free area, iteration, and task in each area (Task-A01-19). */
  nextIds(root: string): Promise<NextIds>;
  /**
   * Adds an area, an iteration or a task under the saved name, by the same rule as `alm add`
   * (a task with an iteration is then planned, as `alm plan`), under the taskbag's lock, recorded
   * in its history. Resolves to what was saved; rejects with the same Problem `alm` gives, and then
   * nothing was added.
   */
  addArea(root: string, id: string | null, name: string): Promise<Area>;
  addIteration(root: string, iteration: NewIteration): Promise<Iteration>;
  addTask(root: string, task: NewTask): Promise<Task>;
  /**
   * The task pane's "New…" iteration (Task-A01-39): adds the iteration as `alm add iteration`,
   * then plans task `task` in it as `alm plan`, under one lock (two events). Resolves to the
   * planned task; rejects with the Problem that refused either, and then nothing changed.
   */
  addIterationAndPlan(root: string, task: string, iteration: NewIteration): Promise<Task>;
  /**
   * Renames an area, changes an iteration, or deletes either (Task-A01-29), under the saved name,
   * by the same rule as `alm rename area`, `alm change iteration` and `alm delete`, under the
   * taskbag's lock, recorded in its history. A delete is refused while any task is in it, done and
   * dropped ones too. Rejects with the same Problem `alm` gives, and then nothing changed.
   */
  renameArea(root: string, id: string, name: string): Promise<Area>;
  changeIteration(root: string, id: string, change: IterationChange): Promise<Iteration>;
  deleteArea(root: string, id: string): Promise<void>;
  deleteIteration(root: string, id: string): Promise<void>;
  /** Opens the page where a newer Allomos is downloaded, in the web browser. */
  openDownloadPage(): Promise<void>;
  /** The About window (Task-A07-28): the app's name and version. */
  aboutInfo(): Promise<AboutInfo>;
  /** The About window's link: www.allomos.com in the default browser. */
  openWebsite(): Promise<void>;
  /** Close the calling window (the About window's Escape key). */
  closeWindow(): Promise<void>;
  /** Whether a newer Allomos is on GitHub (Task-A07-16); `asked` from Check for Updates…. */
  updateStatus(asked: boolean): Promise<UpdateStatus>;
  /** "Skip this version" on the update notice. */
  skipUpdate(version: string): Promise<void>;
  /** Calls `handler` when Check for Updates… is chosen. */
  onCheckUpdatesMenu(handler: () => void): Promise<() => void>;
  /** The update menu item's words and what it does when chosen. */
  setUpdateItem(text: string, action: "check" | "download" | "restart"): Promise<void>;
  /** Calls `handler` when a newer version's installer is downloaded and checked (Task-A07-17). */
  onUpdateReady(handler: () => void): Promise<() => void>;
  /** **Restart to Update** on the update notice: the same as the menu item (Task-A07-17). */
  restartToUpdate(): Promise<void>;
  /** Calls `handler` when Restart to Update could not install the new version. */
  onUpdateFailed(handler: (problem: Problem) => void): Promise<() => void>;
  /** Whether to offer **Install the alm command** (Task-A07-12). */
  almCommandStatus(): Promise<AlmStatus>;
  /** Links the app's own `alm` into Terminal's path, asking for the Mac password. Rejects with a Problem. */
  installAlmCommand(): Promise<AlmInstalled>;
  /** "Don't ask again" for **Install the alm command**. */
  /** "Don't ask again", for this mismatch only (`AlmStatus.mismatch`). */
  dismissAlmOffer(mismatch: string): Promise<void>;
  /** Whether the AI agents know about the project of the taskbag at `root`, and their last use of it. */
  agentStatus(root: string): Promise<AgentStatus>;
  /** Each agent account's quota as the agents reported it (`alm quota`), newest first (Task-A03-02). */
  quotaReports(): Promise<QuotaReport[]>;
  /** Questions an agent asks about `project` with `alm confirm`, oldest first. */
  confirmRequests(project: string): Promise<ConfirmRequest[]>;
  /** What agents report about the jobs they do in `project` (`alm progress`, Task-A05-08). */
  progressReports(project: string): Promise<Progress[]>;
  /** The person closed a finished (or silent) report: remove it. */
  dismissProgress(project: string, id: string): Promise<void>;
  /** The person's answer to one of them. Rejects with a Problem when it is no longer waiting. */
  /** A Go ahead names the fingerprint of what the person was shown, so it counts only for that plan (Task-A03-37). */
  answerConfirm(project: string, id: string, yes: boolean, fingerprint?: string): Promise<void>;
  /** The open-folder flow's state for the taskbag at `root` (records the project folder when it is beside `.allomos/`). */
  projectSetup(root: string): Promise<SetupState>;
  /** Records `folder` as the project folder of the taskbag at `root`. Rejects with a Problem. */
  setProjectFolder(root: string, folder: string): Promise<SetupState>;
  /** Enables or disables File → Connect with the Agents…, File → Update the Taskbag… and File → Handoff Settings…. */
  setConnectEnabled(enabled: boolean): Promise<void>;
}

export interface MenuTexts {
  file: string;
  newWindow: string;
  openRecent: string;
  clearMenu: string;
  noRecent: string;
  connect: string;
  moveTasksIn: string;
  handoffSettings: string;
  lan: string;
}

/** Pairing (apps/desktop/src-tauri/src/pairing.rs, Task-A04-02-02): what the dialog shows. */
export interface PairingView {
  /** Devices waiting for an answer, and approved ones waiting for their code to be typed. */
  asking: { id: string; name: string; code: string | null; expires_in: number | null; tries_left: number | null }[];
  devices: { id: string; name: string; paired_at: string; last_seen: string | null }[];
}

/** "Allow devices on your network to view" (apps/desktop/src-tauri/src/lan.rs, Task-A04-02-01). */
export interface LanStatus {
  on: boolean;
  port: number | null;
  /** `https://<this computer's address>:<port>`; null without a network address. */
  url: string | null;
  qr_svg: string | null;
  fingerprint: string | null;
  default_port: number;
  error: string | null;
}

export const desktopMessenger: Messenger = {
  startupFolder: () => invoke<string | null>("startup_folder"),
  systemLanguages: () => invoke<string[]>("system_languages"),
  saveUiLanguage: (language) => invoke<void>("save_ui_language", { language }),
  async pickFolder(title) {
    const picked = await open({ directory: true, multiple: false, title });
    return typeof picked === "string" ? picked : null;
  },
  inspectFolder: (path) => invoke<FolderInfo>("inspect_folder", { path }),
  openTaskbag: (root) => invoke<Opened>("open_taskbag", { root }).then(withDecisions),
  taskbagStamp: (root) => invoke<string>("taskbag_stamp", { root }),
  showInFileManager: (root) => invoke<void>("show_in_file_manager", { root }),
  openTaskLink: (root, task, link) => invoke<void>("open_task_link", { root, task, link }),
  createTaskbag: (root, name, language) => invoke<Opened>("create_taskbag", { root, name, language }),
  newWindow: () => invoke<void>("new_window"),
  setWindowProject: (root, name) => invoke<void>("set_window_project", { root, name }),
  showProjectElsewhere: (root) => invoke<boolean>("show_project_elsewhere", { root }),
  onWindowFocus: (handler) =>
    getCurrentWebviewWindow().onFocusChanged(({ payload }) => {
      if (payload) handler();
    }),
  // Menu events are sent to the window in front only (Task-A01-27), so each window listens for its own.
  onClearRecentMenu: (handler) => getCurrentWebviewWindow().listen("menu:clear_recent", () => handler()),
  setMenuTexts: (words) => invoke<void>("set_menu_texts", { words }),
  setRecentFolders: (folders) => invoke<void>("set_recent_folders", { folders }),
  onConnectMenu: (handler) => getCurrentWebviewWindow().listen("menu:connect", () => handler()),
  onMoveTasksInMenu: (handler) => getCurrentWebviewWindow().listen("menu:move_tasks_in", () => handler()),
  onHandoffSettingsMenu: (handler) => getCurrentWebviewWindow().listen("menu:handoff_settings", () => handler()),
  onLanMenu: (handler) => getCurrentWebviewWindow().listen("menu:lan_viewing", () => handler()),
  lanStatus: () => invoke<LanStatus>("lan_status"),
  setLanViewing: (on) => invoke<LanStatus>("set_lan_viewing", { on }),
  lanPairing: () => invoke<PairingView>("lan_pairing"),
  answerPairing: (id, approve) => invoke<PairingView>("answer_pairing", { id, approve }),
  removePairedDevice: (id) => invoke<PairingView>("remove_paired_device", { id }),
  onPairingChanged: (handler) => getCurrentWebviewWindow().listen("lan:pairing", () => handler()),
  moveTasksIn: (agent, taskbag, language, translate) => invoke<void>("move_tasks_in", { agent, taskbag, language, translate }),
  moveTasksInRequest: (taskbag, language, translate) => invoke<string>("move_tasks_in_request", { taskbag, language, translate }),
  lastAgent: (root) => invoke<LastAgent | null>("last_agent", { root }),
  findAgents: () => invoke<Agent[]>("find_agents"),
  connectAgents: (agents, project, taskbag, language) =>
    invoke<string[]>("connect_agents", { agents, project, taskbag, language }),
  manualRequest: (project, taskbag, language) => invoke<string>("manual_request", { project, taskbag, language }),
  copyText: (text) => invoke<void>("copy_text", { text }),
  buildApp: (agent, project, taskbag, language, description, quiet) =>
    invoke<void>("build_app", { agent, project, taskbag, language, description, quiet }),
  buildAppRequest: (project, taskbag, language, description, quiet) =>
    invoke<string>("build_app_request", { project, taskbag, language, description, quiet }),
  runModesText: (language) => invoke<string>("run_modes_text", { language }),
  instructionFiles: (project) => invoke<InstructionFiles>("instruction_files", { project }),
  agentsSetUp: (taskbag) => invoke<string[]>("agents_set_up", { taskbag }),
  writeAgentFiles: (project) => invoke<string[]>("write_agent_files", { project }),
  agentStatus: (root) => invoke<AgentStatus>("agent_status", { root }),
  quotaReports: () => invoke<QuotaReport[]>("quota_reports"),
  versionCheck: (root) => invoke<VersionCheck | null>("version_check", { root }),
  moveTaskbagAside: (root) => invoke<Opened>("move_taskbag_aside", { root }),
  openDownloadPage: () => invoke<void>("open_download_page"),
  aboutInfo: () => invoke<AboutInfo>("about_info"),
  openWebsite: () => invoke<void>("open_website"),
  closeWindow: () => invoke<void>("close_window"),
  updateStatus: (asked) => invoke<UpdateStatus>("update_status", { asked }),
  skipUpdate: (version) => invoke<void>("skip_update", { version }),
  setUpdateItem: (text, action) => invoke<void>("set_update_item", { text, action }),
  restartToUpdate: () => invoke<void>("restart_to_update_now"),
  onUpdateReady: (handler) => getCurrentWebviewWindow().listen("update:ready", () => handler()),
  onUpdateFailed: (handler) => getCurrentWebviewWindow().listen<Problem>("update:failed", (e) => handler(e.payload)),
  onCheckUpdatesMenu: (handler) => getCurrentWebviewWindow().listen("menu:check_updates", () => handler()),
  almCommandStatus: () => invoke<AlmStatus>("alm_command_status"),
  installAlmCommand: () => invoke<AlmInstalled>("install_alm_command"),
  dismissAlmOffer: (mismatch) => invoke<void>("dismiss_alm_offer", { mismatch }),
  handoffSettings: (root) => invoke<HandoffSettings>("handoff_settings", { root }),
  setHandoffRules: (root, c) =>
    invoke<HandoffSettings>("set_handoff_rules", {
      root,
      quota: c.quota ?? null,
      context: c.context ?? null,
      writeWhen: c.writeWhen ?? null,
      agent: c.agent ?? null,
      clear: c.clear ?? false,
    }),
  taskHistory: (root, id) => invoke<TaskHistory>("task_history", { root, id }),
  ownerName: () => invoke<string | null>("owner_name"),
  saveOwnerName: (name) => invoke<void>("save_owner_name", { name }),
  editTask: (root, id, edit) => invoke<Task>("edit_task", { root, id, edit }),
  reviewAiDecisions: (root, id) => invoke<void>("review_ai_decisions", { root, id }),
  nextIds: (root) => invoke<NextIds>("next_ids", { root }),
  addArea: (root, id, name) => invoke<Area>("add_area", { root, id, name }),
  addIteration: (root, iteration) => invoke<Iteration>("add_iteration", { root, iteration }),
  addTask: (root, task) => invoke<Task>("add_task", { root, task }),
  addIterationAndPlan: (root, task, iteration) => invoke<Task>("add_iteration_and_plan", { root, task, iteration }),
  renameArea: (root, id, name) => invoke<Area>("rename_area", { root, id, name }),
  changeIteration: (root, id, change) => invoke<Iteration>("change_iteration", { root, id, change }),
  deleteArea: (root, id) => invoke<void>("delete_area", { root, id }),
  deleteIteration: (root, id) => invoke<void>("delete_iteration", { root, id }),
  renameProject: (root, name) => invoke<string>("rename_project", { root, name }),
  taskbagFormat: (root) => invoke<{ format: FormatState; current: string }>("taskbag_format", { root }),
  confirmRequests: (project) => invoke<ConfirmRequest[]>("confirm_requests", { project }),
  progressReports: (project) => invoke<Progress[]>("progress_reports", { project }),
  dismissProgress: (project, id) => invoke<void>("dismiss_progress", { project, id }),
  answerConfirm: (project, id, yes, fingerprint) =>
    invoke<void>("answer_confirm_request", { project, id, yes, fingerprint: fingerprint ?? null }),
  projectSetup: (root) => invoke<SetupState>("project_setup", { root }),
  setProjectFolder: (root, folder) => invoke<SetupState>("set_project_folder", { root, folder }),
  setConnectEnabled: (enabled) => invoke<void>("set_connect_enabled", { enabled }),
};
