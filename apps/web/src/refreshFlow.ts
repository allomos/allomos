// The refresh line at the top of the list (Task-A01-47): when the list was last read from the
// taskbag, and how. Opening the taskbag reads it; the once-a-second follow (Task-A01-03) reads it
// again when its files change ("automatically"); **Refresh now** reads it again whenever the
// person asks ("by you") and says what differs from the list before. Nothing here reads files.

import type { Key } from "./i18n";
import type { Loaded, Task } from "./types";

type T = (key: Key, params?: Record<string, string | number>) => string;

/** What the list shows now and how it came: when the taskbag was opened, after the follow found a
 * change, or after the person pressed Refresh now (with what changed then). */
export type Refreshed =
  | { how: "opened"; at: Date }
  | { how: "auto"; at: Date }
  | { how: "manual"; at: Date; changed: Changed };

/** What differs between two reads of a taskbag: the tasks added, removed or changed (by ID), and
 * whether anything else (project, areas, iterations) changed. */
export type Changed = { tasks: string[]; other: boolean };

/** How many task IDs the line names before it says "and N more". */
export const NAMED_MAX = 5;

/** JSON with object keys sorted, so two reads compare equal whatever order the fields came in. */
function stable(value: unknown): string {
  return JSON.stringify(value, (_key, v: unknown) =>
    v && typeof v === "object" && !Array.isArray(v)
      ? Object.fromEntries(Object.entries(v as Record<string, unknown>).sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0)))
      : v,
  );
}

/** The tasks that differ between two reads: changed and added ones in the new list's order, then
 * removed ones in the old list's order. */
export function changedTasks(before: readonly Task[], after: readonly Task[]): string[] {
  const old = new Map(before.map((task) => [task.id, stable(task)]));
  const now = new Set(after.map((task) => task.id));
  const changed = after.filter((task) => old.get(task.id) !== stable(task)).map((task) => task.id);
  const removed = before.filter((task) => !now.has(task.id)).map((task) => task.id);
  return [...changed, ...removed];
}

/** What differs between two reads of the taskbag. */
export function whatChanged(before: Loaded["taskbag"], after: Loaded["taskbag"]): Changed {
  const tasks = changedTasks(before?.tasks ?? [], after?.tasks ?? []);
  const rest = (b: Loaded["taskbag"]) => (b ? stable({ ...b, tasks: [] }) : "");
  return { tasks, other: rest(before) !== rest(after) };
}

/** The time of day, 14:32:05, in the computer's own time zone. */
export function clock(at: Date): string {
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(at.getHours())}:${p(at.getMinutes())}:${p(at.getSeconds())}`;
}

/** The line's words: "Refreshed automatically at 14:32:05 · the taskbag files changed",
 * "Refreshed by you at 14:35:12 · 1 task changed (A01-33)", "Loaded at 14:20:03, when the taskbag
 * was opened". */
export function refreshText(r: Refreshed, t: T): string {
  const time = clock(r.at);
  if (r.how === "opened") return t("refresh.opened", { time });
  if (r.how === "auto") return t("refresh.auto", { time });
  const { tasks, other } = r.changed;
  if (tasks.length === 0) return t(other ? "refresh.manual_settings" : "refresh.manual_none", { time });
  const named = tasks.slice(0, NAMED_MAX).join(", ");
  const ids = tasks.length > NAMED_MAX ? t("refresh.ids_more", { ids: named, more: tasks.length - NAMED_MAX }) : named;
  return t(tasks.length === 1 ? "refresh.manual_tasks_one" : "refresh.manual_tasks_other", { time, count: tasks.length, ids });
}
