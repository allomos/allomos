// The "Open recent" dialog on the start page (Task-A01-43). It lists the same folders as File →
// Open Recent (Task-A01-11), newest first; for each it also shows the project's name and when it
// was last opened, which Allomos notes on this computer each time a taskbag opens.

/** What Allomos noted about one recent folder the last time it opened. */
export interface RecentInfo {
  /** The project's name in its taskbag. */
  name?: string;
  /** When it was last opened, as an ISO time. */
  opened?: string;
}

/** Notes per project folder. Folders opened before Task-A01-43 have none. */
export type RecentInfoMap = Record<string, RecentInfo>;

/** One line of the dialog. */
export interface RecentRow {
  folder: string;
  name: string;
  /** When it was last opened (ISO time), if known. */
  opened: string | null;
  /** The folder no longer exists: shown greyed, it can only be removed (Task-A01-28). */
  missing: boolean;
}

/** Reads the stored notes; anything unreadable counts as none. */
export function parseRecentInfo(raw: string | null): RecentInfoMap {
  let value: unknown;
  try {
    value = JSON.parse(raw ?? "{}");
  } catch {
    return {};
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  const info: RecentInfoMap = {};
  for (const [folder, entry] of Object.entries(value as Record<string, unknown>)) {
    if (!entry || typeof entry !== "object") continue;
    const { name, opened } = entry as Record<string, unknown>;
    info[folder] = {
      ...(typeof name === "string" && name.trim() ? { name } : {}),
      ...(typeof opened === "string" && !Number.isNaN(Date.parse(opened)) ? { opened } : {}),
    };
  }
  return info;
}

/** Notes that `folder` (project `name`, if the taskbag has one) was opened at `now`. */
export function noteOpened(info: RecentInfoMap, folder: string, name: string | null, now: Date): RecentInfoMap {
  const before = info[folder] ?? {};
  return { ...info, [folder]: { ...before, ...(name?.trim() ? { name } : {}), opened: now.toISOString() } };
}

/** Keeps only the notes of folders still in the recent list. */
export function keepListed(info: RecentInfoMap, list: readonly string[]): RecentInfoMap {
  const kept: RecentInfoMap = {};
  for (const folder of list) if (info[folder]) kept[folder] = info[folder];
  return kept;
}

/** The last part of a folder path, with `/` or `\`: the name shown when no project name is known. */
export function folderName(folder: string): string {
  const parts = folder.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? folder;
}

/** The dialog's lines, in the recent list's order (newest first). */
export function recentRows(list: readonly string[], info: RecentInfoMap, missing: ReadonlySet<string>): RecentRow[] {
  return list.map((folder) => ({
    folder,
    name: info[folder]?.name ?? folderName(folder),
    opened: info[folder]?.opened ?? null,
    missing: missing.has(folder),
  }));
}

/** Where the recent folders are kept on this computer (the window's storage), newest first. */
export const RECENT_KEY = "allomos.recentFolders";
export const RECENT_MAX = 10;

/** The recent folders; none when the storage cannot be read. */
export function readRecent(): string[] {
  try {
    const list: unknown = JSON.parse(localStorage.getItem(RECENT_KEY) ?? "[]");
    return Array.isArray(list) ? list.filter((x): x is string => typeof x === "string").slice(0, RECENT_MAX) : [];
  } catch {
    return [];
  }
}

/** Each recent folder's project name and when it was last opened, for the start page's Open recent
 * dialog (Task-A01-43). Kept only for folders still in the list. */
export const RECENT_INFO_KEY = "allomos.recentInfo";

export function readRecentInfo(): RecentInfoMap {
  try {
    return parseRecentInfo(localStorage.getItem(RECENT_INFO_KEY));
  } catch {
    return {};
  }
}
