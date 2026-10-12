// The messenger: the only place the interface talks to Allomos's Rust side.
//
// Inside the desktop window it uses Tauri's built-in channel. When phone and browser
// companions arrive (Iteration 9), a second implementation will talk to the desktop's
// local server instead; the rest of the interface does not change.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Agent, AgentStatus, ConfirmRequest, FolderInfo, GitKept, InstructionFiles, Opened, SetupState, VersionCheck } from "./types";

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
  /** Calls `handler` whenever File → Open Folder… is chosen. Resolves to a function that stops listening. */
  onOpenFolderMenu(handler: () => void): Promise<() => void>;
  /** Calls `handler` with the folder chosen under File → Open Recent. */
  onOpenRecentMenu(handler: (folder: string) => void): Promise<() => void>;
  /** Calls `handler` when File → Open Recent → Clear Menu is chosen. */
  onClearRecentMenu(handler: () => void): Promise<() => void>;
  /** Puts the app menu's words in the interface's language. */
  setMenuTexts(texts: MenuTexts): Promise<void>;
  /** Shows these folders, newest first, under File → Open Recent. */
  setRecentFolders(folders: string[]): Promise<void>;
  /** Calls `handler` when File → Connect with the Agents… is chosen. */
  onConnectMenu(handler: () => void): Promise<() => void>;
  /** The AI agents on this computer that can be asked to add Allomos's instructions. */
  findAgents(): Promise<Agent[]>;
  /**
   * Opens each agent in `project` with its request typed in: the first also looks for the
   * project's tasks and offers to move them; the others only add Allomos's instructions.
   * Resolves to the ones that opened. Rejects with a Problem.
   */
  connectAgents(agents: string[], project: string, taskbag: string, language: string): Promise<string[]>;
  /** Whether `project` has AGENTS.md and CLAUDE.md. */
  instructionFiles(project: string): Promise<InstructionFiles>;
  /**
   * Puts the folder holding the taskbag at `root` under Git (`git init` when it is in no
   * repository) and commits the taskbag there, nothing else; never pushes. Rejects with a Problem.
   */
  keepInGit(root: string): Promise<GitKept>;
  /** Writes Allomos's section into the project's AGENTS.md and CLAUDE.md; resolves to the files changed. */
  writeAgentFiles(project: string): Promise<string[]>;
  /** Whether the project of the taskbag at `root` is up to date with this Allomos; null until an agent is connected. */
  versionCheck(root: string): Promise<VersionCheck | null>;
  /** Whether the AI agents know about the project of the taskbag at `root`, and their last use of it. */
  agentStatus(root: string): Promise<AgentStatus>;
  /** Questions an agent asks about `project` with `alm confirm`, oldest first. */
  confirmRequests(project: string): Promise<ConfirmRequest[]>;
  /** The person's answer to one of them. Rejects with a Problem when it is no longer waiting. */
  answerConfirm(project: string, id: string, yes: boolean): Promise<void>;
  /** The open-folder flow's state for the taskbag at `root` (records the project folder when it is beside `.allomos/`). */
  projectSetup(root: string): Promise<SetupState>;
  /** Records `folder` as the project folder of the taskbag at `root`. Rejects with a Problem. */
  setProjectFolder(root: string, folder: string): Promise<SetupState>;
  /** Enables or disables File → Connect with the Agents…. */
  setConnectEnabled(enabled: boolean): Promise<void>;
}

export interface MenuTexts {
  file: string;
  openFolder: string;
  openRecent: string;
  clearMenu: string;
  noRecent: string;
  connect: string;
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
  openTaskbag: (root) => invoke<Opened>("open_taskbag", { root }),
  taskbagStamp: (root) => invoke<string>("taskbag_stamp", { root }),
  showInFileManager: (root) => invoke<void>("show_in_file_manager", { root }),
  openTaskLink: (root, task, link) => invoke<void>("open_task_link", { root, task, link }),
  createTaskbag: (root, name, language) => invoke<Opened>("create_taskbag", { root, name, language }),
  onOpenFolderMenu: (handler) => listen("menu:open_folder", () => handler()),
  onOpenRecentMenu: (handler) => listen<string>("menu:open_recent", (e) => handler(e.payload)),
  onClearRecentMenu: (handler) => listen("menu:clear_recent", () => handler()),
  setMenuTexts: (words) => invoke<void>("set_menu_texts", { words }),
  setRecentFolders: (folders) => invoke<void>("set_recent_folders", { folders }),
  onConnectMenu: (handler) => listen("menu:connect", () => handler()),
  findAgents: () => invoke<Agent[]>("find_agents"),
  connectAgents: (agents, project, taskbag, language) =>
    invoke<string[]>("connect_agents", { agents, project, taskbag, language }),
  instructionFiles: (project) => invoke<InstructionFiles>("instruction_files", { project }),
  keepInGit: (root) => invoke<GitKept>("keep_in_git", { root }),
  writeAgentFiles: (project) => invoke<string[]>("write_agent_files", { project }),
  agentStatus: (root) => invoke<AgentStatus>("agent_status", { root }),
  versionCheck: (root) => invoke<VersionCheck | null>("version_check", { root }),
  confirmRequests: (project) => invoke<ConfirmRequest[]>("confirm_requests", { project }),
  answerConfirm: (project, id, yes) => invoke<void>("answer_confirm_request", { project, id, yes }),
  projectSetup: (root) => invoke<SetupState>("project_setup", { root }),
  setProjectFolder: (root, folder) => invoke<SetupState>("set_project_folder", { root, folder }),
  setConnectEnabled: (enabled) => invoke<void>("set_connect_enabled", { enabled }),
};
