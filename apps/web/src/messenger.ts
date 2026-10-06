// The messenger: the only place the interface talks to glidebox's Rust side.
//
// Inside the desktop window it uses Tauri's built-in channel. When phone and browser
// companions arrive (Iteration 9), a second implementation will talk to the desktop's
// local server instead; the rest of the interface does not change.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { FolderInfo, Opened } from "./types";

export interface Messenger {
  /** A folder given when glidebox was started, if any. */
  startupFolder(): Promise<string | null>;
  /** The computer's preferred languages, most preferred first. */
  systemLanguages(): Promise<string[]>;
  /** Ask the user to pick a folder. Returns null if they cancel. */
  pickFolder(title: string): Promise<string | null>;
  inspectFolder(path: string): Promise<FolderInfo>;
  openTaskBox(root: string): Promise<Opened>;
  /** Rejects with a Problem (see types.ts) when the task box cannot be created. */
  createTaskBox(root: string, name: string): Promise<Opened>;
}

export const desktopMessenger: Messenger = {
  startupFolder: () => invoke<string | null>("startup_folder"),
  systemLanguages: () => invoke<string[]>("system_languages"),
  async pickFolder(title) {
    const picked = await open({ directory: true, multiple: false, title });
    return typeof picked === "string" ? picked : null;
  },
  inspectFolder: (path) => invoke<FolderInfo>("inspect_folder", { path }),
  openTaskBox: (root) => invoke<Opened>("open_task_box", { root }),
  createTaskBox: (root, name) => invoke<Opened>("create_task_box", { root, name }),
};
