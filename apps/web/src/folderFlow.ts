// Project folders in the task list and the task pane (Task-A01-sketch327; decision 0019), kept
// apart from React so Node can test it. The list shows folder chips and the Folder filter only
// while two or more folders are bound (approver, 2026-10-25).

import type { Task } from "./types";

/** A bound folder as the app reads it (`allomos_store::folders::FolderInfo`). */
export interface FolderInfo {
  name: string;
  /** As project.json records it, relative to the folder holding `.allomos/`. */
  path: string;
  /** Found on this computer. */
  exists: boolean;
  section: boolean;
  tasks: number;
  open: number;
}

/** The Folder filter: every task, the tasks with no folder, or one folder's tasks (by name). */
export type FolderFilter = { kind: "all" } | { kind: "none" } | { kind: "folder"; name: string };

/** Chips and the filter show only with two or more folders. */
export function showFolders(folders: readonly FolderInfo[]): boolean {
  return folders.length >= 2;
}

/** A stored filter read back: `all`, `none`, or a folder's name still bound; anything else, and
 * any filter while fewer than two folders are bound, is "all" (a chosen filter is cleared). */
export function readFolderFilter(stored: string | null, folders: readonly FolderInfo[]): FolderFilter {
  if (!showFolders(folders) || !stored || stored === "all") return { kind: "all" };
  if (stored === "none") return { kind: "none" };
  return folders.some((f) => f.name === stored) ? { kind: "folder", name: stored } : { kind: "all" };
}

/** The filter as it is stored and as the menu's value. */
export function folderFilterValue(filter: FolderFilter): string {
  return filter.kind === "folder" ? filter.name : filter.kind;
}

export function keepFolder(filter: FolderFilter, task: Task): boolean {
  const folders = task.folders ?? [];
  if (filter.kind === "all") return true;
  if (filter.kind === "none") return folders.length === 0;
  return folders.includes(filter.name);
}

/** The folders a task made in the app gets: the one chosen in the filter, if any. */
export function newTaskFolders(filter: FolderFilter): string[] {
  return filter.kind === "folder" ? [filter.name] : [];
}

/** A folder's checkbox ticked or unticked, keeping the bound folders' order. */
export function toggleFolder(chosen: readonly string[], name: string, folders: readonly FolderInfo[]): string[] {
  const next = chosen.includes(name) ? chosen.filter((n) => n !== name) : [...chosen, name];
  const order = folders.map((f) => f.name);
  return [...next].sort((a, b) => order.indexOf(a) - order.indexOf(b));
}
