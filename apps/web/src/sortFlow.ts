// Sorting the tasks inside each iteration (Task-A01-52): by ID (the taskbag's own order), by name
// or by last change, ascending or descending. Kept apart from React so it can be tested with Node
// alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.
import type { Task } from "./types";

export type SortKey = "id" | "name" | "changed";
export type SortDir = "asc" | "desc";
export interface Sort {
  key: SortKey;
  dir: SortDir;
}

export const SORT_KEYS: SortKey[] = ["id", "name", "changed"];
/** The taskbag's own order: what the list showed before the menu existed. */
export const DEFAULT_SORT: Sort = { key: "id", dir: "asc" };

/** A sort read back from storage; anything unreadable is the default. */
export function parseSort(text: string | null): Sort {
  if (!text) return DEFAULT_SORT;
  try {
    const s: unknown = JSON.parse(text);
    if (s && typeof s === "object") {
      const { key, dir } = s as Record<string, unknown>;
      if (SORT_KEYS.includes(key as SortKey) && (dir === "asc" || dir === "desc")) return { key: key as SortKey, dir };
    }
  } catch {
    /* fall through */
  }
  return DEFAULT_SORT;
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: "base" });

/** `tasks` in `sort`'s order, as a new list. `changed` holds each task's last change (UTC,
 * `2026-10-13T14:02:05Z`, so text order is time order); a task with none counts as oldest. Ties
 * fall back to the ID, ascending, so the order never jumps between reloads. */
export function sortTasks(tasks: Task[], sort: Sort, changed: Record<string, string> = {}): Task[] {
  const byId = (a: Task, b: Task) => collator.compare(a.id, b.id);
  const primary = (a: Task, b: Task): number => {
    if (sort.key === "name") return collator.compare(a.title, b.title);
    if (sort.key === "changed") {
      const x = changed[a.id] ?? "";
      const y = changed[b.id] ?? "";
      return x < y ? -1 : x > y ? 1 : 0;
    }
    return byId(a, b);
  };
  const sign = sort.dir === "asc" ? 1 : -1;
  return [...tasks].sort((a, b) => sign * primary(a, b) || byId(a, b));
}
