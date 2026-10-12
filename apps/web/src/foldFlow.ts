// Collapsing and expanding sections (Task-A01-26): area and iteration groups in the task list, and
// handoffs in the Handoffs tab. Kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). Only type imports here: Node strips them.

/** Which list a set of keys belongs to. Groups start expanded, so their set holds the collapsed
 * ones; handoffs start collapsed (date, writer and first line), so their set holds the open ones. */
export type FoldList = "groups" | "handoffs";

/** The window-storage key for one project's list: remembered per project, per viewer. */
export function foldStorageKey(root: string, list: FoldList): string {
  return `allomos.fold.${list}:${root}`;
}

/** Keys read back from storage; anything unreadable counts as nothing remembered. */
export function parseKeys(text: string | null): string[] {
  if (!text) return [];
  try {
    const list: unknown = JSON.parse(text);
    return Array.isArray(list) ? [...new Set(list.filter((k): k is string => typeof k === "string"))] : [];
  } catch {
    return [];
  }
}

/** `key` added when missing, removed when present. */
export function toggleKey(keys: string[], key: string): string[] {
  return keys.includes(key) ? keys.filter((k) => k !== key) : [...keys, key];
}

/** Every shown key added (`on`) or removed; keys of sections not shown now, such as the areas
 * while the list is grouped by iteration, are kept. */
export function setKeys(keys: string[], shown: string[], on: boolean): string[] {
  if (on) return [...keys, ...shown.filter((k) => !keys.includes(k))];
  return keys.filter((k) => !shown.includes(k));
}

/** `key` added when missing (for example to expand the group of a task opened from elsewhere). */
export function withKey(keys: string[], key: string): string[] {
  return keys.includes(key) ? keys : [...keys, key];
}

/** `key` removed. */
export function withoutKey(keys: string[], key: string): string[] {
  return keys.includes(key) ? keys.filter((k) => k !== key) : keys;
}

/** The tasks the arrow keys move through: those of groups that are not collapsed, in order. */
export function visibleItems<T>(groups: { key: string; tasks: T[] }[], collapsed: string[]): T[] {
  return groups.filter((g) => !collapsed.includes(g.key)).flatMap((g) => g.tasks);
}
