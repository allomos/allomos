// The messenger: the only place the interface talks to FlipHopper's Rust side.
//
// Inside the desktop window it uses Tauri's built-in channel. When phone and browser
// companions arrive (Iteration 9), a second implementation will talk to the desktop's
// local server instead; the rest of the interface does not change.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { Agent, AgentStatus, FolderInfo, Opened, TaskSystem } from "./types";

export interface Messenger {
  /** A folder given when FlipHopper was started, if any. Rejects with a Problem when the given path is not a folder. */
  startupFolder(): Promise<string | null>;
  /** The computer's preferred languages, most preferred first. */
  systemLanguages(): Promise<string[]>;
  /** Ask the user to pick a folder. Returns null if they cancel. */
  pickFolder(title: string): Promise<string | null>;
  inspectFolder(path: string): Promise<FolderInfo>;
  openSeabag(root: string): Promise<Opened>;
  /** A fingerprint of the seabag's files; it changes whenever a file is written. */
  seabagStamp(root: string): Promise<string>;
  /** Show the seabag's folder in Finder or Explorer. Rejects with a Problem on failure. */
  showInFileManager(root: string): Promise<void>;
  /**
   * Rejects with a Problem (see types.ts) when the seabag cannot be created. In a folder that
   * looks empty it also writes README-fliphopper.md in `language` (`readme_added` says so).
   */
  createSeabag(root: string, name: string, language: string): Promise<Opened>;
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
  /** Calls `handler` when File → Tell AI Agents About This Project… is chosen. */
  onAgentsMenu(handler: () => void): Promise<() => void>;
  /** The AI agents on this computer that can be asked to add FlipHopper's instructions. */
  findAgents(): Promise<Agent[]>;
  /** Opens each agent in `project` with the request typed in; resolves to the ones that opened. Rejects with a Problem. */
  askAgents(agents: string[], project: string, seabag: string, language: string): Promise<string[]>;
  /** Writes FlipHopper's section into the project's AGENTS.md and CLAUDE.md; resolves to the files changed. */
  writeAgentFiles(project: string): Promise<string[]>;
  /** Whether the AI agents know about the project of the seabag at `root`, and their last use of it. */
  agentStatus(root: string): Promise<AgentStatus>;
  /** Calls `handler` when File → Move Tasks Into FlipHopper… is chosen. */
  onConvertMenu(handler: () => void): Promise<() => void>;
  /** Signs that `project` already tracks its tasks another way. Changes nothing. */
  taskSystem(project: string): Promise<TaskSystem>;
  /** Opens one agent in `project` with the request to move its tasks into the seabag at `seabag` typed in. Rejects with a Problem. */
  askAgentToConvert(agent: string, project: string, seabag: string, language: string): Promise<void>;
}

export interface MenuTexts {
  file: string;
  openFolder: string;
  openRecent: string;
  clearMenu: string;
  noRecent: string;
  agents: string;
  convert: string;
}

export const desktopMessenger: Messenger = {
  startupFolder: () => invoke<string | null>("startup_folder"),
  systemLanguages: () => invoke<string[]>("system_languages"),
  async pickFolder(title) {
    const picked = await open({ directory: true, multiple: false, title });
    return typeof picked === "string" ? picked : null;
  },
  inspectFolder: (path) => invoke<FolderInfo>("inspect_folder", { path }),
  openSeabag: (root) => invoke<Opened>("open_seabag", { root }),
  seabagStamp: (root) => invoke<string>("seabag_stamp", { root }),
  showInFileManager: (root) => invoke<void>("show_in_file_manager", { root }),
  createSeabag: (root, name, language) => invoke<Opened>("create_seabag", { root, name, language }),
  onOpenFolderMenu: (handler) => listen("menu:open_folder", () => handler()),
  onOpenRecentMenu: (handler) => listen<string>("menu:open_recent", (e) => handler(e.payload)),
  onClearRecentMenu: (handler) => listen("menu:clear_recent", () => handler()),
  setMenuTexts: (words) => invoke<void>("set_menu_texts", { words }),
  setRecentFolders: (folders) => invoke<void>("set_recent_folders", { folders }),
  onAgentsMenu: (handler) => listen("menu:agents", () => handler()),
  findAgents: () => invoke<Agent[]>("find_agents"),
  askAgents: (agents, project, seabag, language) =>
    invoke<string[]>("ask_agents", { agents, project, seabag, language }),
  writeAgentFiles: (project) => invoke<string[]>("write_agent_files", { project }),
  agentStatus: (root) => invoke<AgentStatus>("agent_status", { root }),
  onConvertMenu: (handler) => listen("menu:convert", () => handler()),
  taskSystem: (project) => invoke<TaskSystem>("task_system", { project }),
  askAgentToConvert: (agent, project, seabag, language) =>
    invoke<void>("ask_agent_to_convert", { agent, project, seabag, language }),
};
