// The messenger: the only place the interface talks to FlipHopper's Rust side.
//
// Inside the desktop window it uses Tauri's built-in channel. When phone and browser
// companions arrive (Iteration 9), a second implementation will talk to the desktop's
// local server instead; the rest of the interface does not change.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { FolderInfo, Opened } from "./types";

export interface Messenger {
  /** A folder given when FlipHopper was started, if any. */
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
  /** Rejects with a Problem (see types.ts) when the seabag cannot be created. */
  createSeabag(root: string, name: string): Promise<Opened>;
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
  createSeabag: (root, name) => invoke<Opened>("create_seabag", { root, name }),
};
