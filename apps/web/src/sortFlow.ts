// Sorting the tasks inside each iteration and area (Task-A01-52, Task-A01-plum588): by ID (the
// taskbag's own order), by name or by last change, ascending or descending. Kept apart from React
// so it can be tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node
// strips them.
import type { Task } from "./types";

export type SortKey = "id" | "name" | "changed";
export type SortDir = "asc" | "desc";
export interface Sort {
  key: SortKey;
  dir: SortDir;
}

export const SORT_KEYS: SortKey[] = ["id", "name", "changed"];
/** The sort when the person has not chosen one: the most recently changed tasks first
 * (Task-A01-city762). Before, it was the taskbag's own order (ID, ascending). */
export const DEFAULT_SORT: Sort = { key: "changed", dir: "desc" };

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

/** An older, numbered ID (`A01-53`, `A01-03-01`): only digits after the area (crates/core/src/ids.rs). */
export function numbered(id: string): boolean {
  return /^[^-]+-[\d-]+$/.test(id);
}

/** `tasks` in `sort`'s order, as a new list. `changed` holds each task's last change (UTC,
 * `2026-10-13T14:02:05Z`, so text order is time order); a task with none counts as oldest. Ties
 * fall back to the ID order, ascending, so the order never jumps between reloads.
 *
 * The ID order is the taskbag's own order, as `tasks` come (Task-A01-54): numbered IDs
 * (`A01-53`) by number, then word IDs (`A01-tiger042`) in the order they were made, which
 * Allomos works out from the history. Sorting word IDs alphabetically would scatter new tasks. */
export function sortTasks(tasks: Task[], sort: Sort, changed: Record<string, string> = {}): Task[] {
  const at = new Map(tasks.map((t, i) => [t.id, i]));
  const byId = (a: Task, b: Task) => {
    const [x, y] = [numbered(a.id), numbered(b.id)];
    if (x && y) return collator.compare(a.id, b.id);
    if (x !== y) return x ? -1 : 1;
    return (at.get(a.id) ?? 0) - (at.get(b.id) ?? 0);
  };
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
