// Why a stopped task is stopped, in words (decision 0016, Task-A01-31): `waiting` means a person
// (or an outside event) must act — "Waiting for you: approve the card", "Waiting for Ann: …";
// `blocked` means another task must finish first — "Blocked by A01-17 (in progress)", each
// blocker with its state now. Also the list's state filter. Nothing here moves a task: when every
// blocker is done the app only says so, as `alm` does.

import type { Key } from "./i18n";
import type { State, Task } from "./types";

type T = (key: Key, params?: Record<string, string | number>) => string;

/** One task that blocks another, with its state now (`null`: no such task in the taskbag). */
export type Blocker = { id: string; state: State | null };

/** The tasks blocking `task`, in the order it lists them, each with its current state. */
export function blockers(task: Task, tasks: readonly Task[]): Blocker[] {
  return (task.blocked_by ?? []).map((id) => ({ id, state: tasks.find((x) => x.id === id)?.state ?? null }));
}

/** Whether a blocked task's blockers are all done. A dropped blocker never counts as done. */
export function allBlockersDone(task: Task, tasks: readonly Task[]): boolean {
  const list = blockers(task, tasks);
  return task.state === "blocked" && list.length > 0 && list.every((b) => b.state === "done");
}

/** "A01-17 (in progress)"; a dropped blocker is flagged; an unknown one said so. */
export function blockerText(b: Blocker, t: T): string {
  if (b.state === null) return t("task.blocker_missing", { id: b.id });
  if (b.state === "dropped") return t("task.blocker_dropped", { id: b.id });
  return t("task.blocker", { id: b.id, state: t(`state.${b.state}` as Key) });
}

/** The line a stopped task shows in the list and the task pane; `null` when it is not stopped. */
export function stopText(task: Task, tasks: readonly Task[], t: T): string | null {
  if (task.state === "waiting") {
    const what = task.waiting_on ?? "";
    return task.who ? t("task.waiting_for", { who: task.who, what }) : t("task.waiting_for_you", { what });
  }
  if (task.state === "blocked") {
    const list = blockers(task, tasks)
      .map((b) => blockerText(b, t))
      .join(", ");
    const line = t("task.blocked_by", { list });
    return allBlockersDone(task, tasks) ? `${line} — ${t("task.blockers_done")}` : line;
  }
  return null;
}

/** The list's state filter: every state, or one. */
export type StateFilter = "all" | State;

export const FILTER_STATES: State[] = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"];

/** The tasks the filter keeps. */
export function keepState(filter: StateFilter, task: Task): boolean {
  return filter === "all" || task.state === filter;
}

/** A stored filter read back; anything else is "all". */
export function readFilter(value: string | null): StateFilter {
  return value && (FILTER_STATES as string[]).includes(value) ? (value as State) : "all";
}
