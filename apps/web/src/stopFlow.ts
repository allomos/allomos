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

/** Finished without the human check and not checked since: "Not human-checked" (Task-A03-18).
 * A task still carrying the note's name from before allomos/4 counts too. */
export function toCheck(task: Task): boolean {
  return task.state === "done" && !!(task.without_human_check || task.finished_without_owner);
}

/** Why it was finished without the human check (the note, under either name). */
export function humanCheckNote(task: Task): string {
  return task.without_human_check ?? task.finished_without_owner ?? "";
}

/** The list's state filter: every state, one, or every task that needs a person: to check or
 * to answer (waiting), as `alm human` lists them (Task-A03-18). */
export type StateFilter = "all" | "human" | State;

export const FILTER_STATES: State[] = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"];

/** The tasks the filter keeps. */
export function keepState(filter: StateFilter, task: Task): boolean {
  if (filter === "human") return toCheck(task) || task.state === "waiting";
  return filter === "all" || task.state === filter;
}

/** A stored filter read back; anything else is "all". */
export function readFilter(value: string | null): StateFilter {
  if (value === "human") return "human";
  return value && (FILTER_STATES as string[]).includes(value) ? (value as State) : "all";
}

/** What the filter leaves out of a list of tasks: how many, and how many in each state, in the
 * filter's order of states (Task-A01-42). Nothing is left out when the filter is "all". */
export type Hidden = { count: number; states: { state: State; count: number }[] };

export function hiddenByFilter(filter: StateFilter, tasks: readonly Task[]): Hidden {
  const hidden = tasks.filter((task) => !keepState(filter, task));
  const states = FILTER_STATES.map((state) => ({ state, count: hidden.filter((x) => x.state === state).length })).filter(
    (s) => s.count > 0,
  );
  return { count: hidden.length, states };
}

/** "38 more hidden by the filter: 1 Waiting, 4 Ready, 33 Done". */
export function hiddenText(hidden: Hidden, t: T): string {
  const states = hidden.states.map((s) => t("filter.state_count", { count: s.count, state: t(`state.${s.state}` as Key) })).join(t("filter.list_sep"));
  return t("filter.hidden", { count: hidden.count, states });
}

/** The filter's name as the Show box words it. */
export function filterName(filter: StateFilter, t: T): string {
  if (filter === "all") return t("filter.all");
  if (filter === "human") return t("filter.human");
  return t(`state.${filter}` as Key);
}
