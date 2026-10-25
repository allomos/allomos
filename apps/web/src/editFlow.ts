// Changing a task in the task pane (Task-A01-18): which fields can be changed, what each starts
// from, and the change sent to Rust. Rust decides whether a change is allowed, by the same rule
// as `alm`; nothing here refuses one.

import type { Edit, Iteration, NewIteration, State, Task } from "./types";

/** The task pane's changeable fields. */
export type EditField =
  | "state"
  | "label"
  | "waiting_on"
  | "blocked_by"
  | "next_dependency"
  | "handoff_note"
  | "iteration"
  | "title"
  | "description";

/**
 * What the person typed or chose. `waitingOn` and `who` go with the state `waiting`; `blockedBy`
 * (task IDs) with the state `blocked` and the field "Blocked by" (decision 0016).
 */
export type Draft = { value: string; waitingOn: string; who: string; blockedBy: string[] };

/**
 * The states the app can set, each by its `alm` command (backlog, ready, wait, block, done, drop).
 * In progress comes only from an agent claiming the task (owner, 2026-10-14: no claiming or
 * releasing from the app). Back to backlog since Task-A01-lotus143 (approver, 2026-10-25).
 */
export const SETTABLE_STATES: State[] = ["backlog", "ready", "waiting", "blocked", "done", "dropped"];

/** The states offered for `task`: its own first, so the list shows where it is, then the settable ones. */
export function stateChoices(task: Task): State[] {
  return SETTABLE_STATES.includes(task.state) ? SETTABLE_STATES : [task.state, ...SETTABLE_STATES];
}

/** Back to backlog is allowed for a ready, waiting or blocked task nobody holds, as
 * `alm backlog` allows it (Rust decides; this only greys the choice out). */
export function backlogAllowed(task: Task): boolean {
  return !task.holder && (task.state === "ready" || task.state === "waiting" || task.state === "blocked");
}

/** Why backlog is greyed out for `task`, so a greyed-out choice is never unexplained: it is held
 * (by whom), or it is finished (reopen it instead); null when it is offered, or already backlog. */
export function backlogBlocked(task: Task): { why: "held"; holder: string } | { why: "finished" } | null {
  if (task.state === "backlog" || backlogAllowed(task)) return null;
  if (task.state === "done" || task.state === "dropped") return { why: "finished" };
  return { why: "held", holder: task.holder ?? "" };
}

/** Whether a state is greyed out in `task`'s state menu. */
export function stateDisabled(task: Task, s: State): boolean {
  if (s === "in_progress") return true;
  return s === "backlog" && task.state !== "backlog" && !backlogAllowed(task);
}

/** The draft a field's editor starts from: the task's value now. */
export function startDraft(task: Task, field: EditField): Draft {
  const value =
    field === "state"
      ? task.state
      : field === "description"
        ? task.description
        : field === "blocked_by"
          ? ""
          : (task[field] ?? "");
  return { value, waitingOn: task.waiting_on ?? "", who: task.who ?? "", blockedBy: [...(task.blocked_by ?? [])] };
}

const sameIds = (a: string[], b: string[]) => a.length === b.length && a.every((x, i) => x === b[i]);

/** Whether Save would change anything. Waiting with a new "waiting on" or "who", or blocked by other tasks, counts. */
export function draftChanged(task: Task, field: EditField, draft: Draft): boolean {
  const start = startDraft(task, field);
  if (field === "blocked_by") return draft.blockedBy.length > 0 && !sameIds(draft.blockedBy, start.blockedBy);
  if (field === "state" && draft.value === "waiting") {
    return task.state !== "waiting" || draft.waitingOn.trim() !== start.waitingOn.trim() || draft.who.trim() !== start.who.trim();
  }
  if (field === "state" && draft.value === "blocked") {
    return task.state !== "blocked" || !sameIds(draft.blockedBy, start.blockedBy);
  }
  if (field === "description") return draft.value.trimEnd() !== start.value.trimEnd();
  return draft.value.trim() !== start.value.trim();
}

/** The change Rust is asked for. Empty text clears a field that may be empty. A title or
 * description carries the version of the task the person started from (Task-A01-57), so Rust
 * refuses it if the task changed since. */
export function toEdit(field: EditField, draft: Draft, version: string | null = null): Edit {
  const text = draft.value.trim();
  switch (field) {
    case "state":
      return draft.value === "waiting"
        ? { field, state: "waiting", waiting_on: draft.waitingOn.trim(), who: draft.who.trim() || null }
        : draft.value === "blocked"
          ? { field, state: "blocked", blocked_by: draft.blockedBy }
          : { field, state: draft.value as State };
    case "label":
      return { field, label: draft.value };
    case "waiting_on":
      return { field, text };
    case "blocked_by":
      return { field, ids: draft.blockedBy };
    case "next_dependency":
    case "handoff_note":
      return { field, text: text || null };
    case "iteration":
      return { field, iteration: text || null };
    case "title":
      return { field, title: text, expected_version: version };
    case "description":
      return { field, text: draft.value, expected_version: version };
  }
}

/** The tasks a task may be blocked by: every other task, in ID order (Rust checks the rest). */
export function blockerChoices(task: Task, tasks: readonly Task[]): Task[] {
  return tasks.filter((x) => x.id !== task.id).sort((a, b) => a.id.localeCompare(b.id));
}

/** The draft's blocking tasks with `id` switched on or off, kept in ID order. */
export function toggleBlocker(draft: Draft, id: string): Draft {
  const on = draft.blockedBy.includes(id);
  const blockedBy = on ? draft.blockedBy.filter((x) => x !== id) : [...draft.blockedBy, id].sort();
  return { ...draft, blockedBy };
}

// The iteration drop-down (Task-A01-39): "New…" first, "No iteration" second (owner,
// 2026-10-14), then every iteration in order, by ID and name.

/** The value of "New…": never an iteration ID, which is only letters, digits, `-` and `_`. */
export const NEW_ITERATION = ":new";

/** One line of the iteration drop-down. `value` is what the draft holds when it is chosen. */
export type IterationChoice =
  | { kind: "new"; value: typeof NEW_ITERATION }
  | { kind: "none"; value: "" }
  | { kind: "iteration"; value: string; id: string; name: string | null; release: string | null };

/**
 * The iteration drop-down for `task`: New…, No iteration, then each iteration. An iteration the
 * task names that the taskbag does not list stays offered by its ID, so the list still shows where
 * the task is.
 */
export function iterationChoices(task: Task, iterations: readonly Iteration[]): IterationChoice[] {
  const known = iterations.map((i): IterationChoice => ({ kind: "iteration", value: i.id, id: i.id, name: i.name, release: i.release ?? null }));
  const lost: IterationChoice[] =
    task.iteration && !iterations.some((i) => i.id === task.iteration)
      ? [{ kind: "iteration", value: task.iteration, id: task.iteration, name: null, release: null }]
      : [];
  return [{ kind: "new", value: NEW_ITERATION }, { kind: "none", value: "" }, ...lost, ...known];
}

/** What "New…" asks for: a name and two optional dates (`YYYY-MM-DD`, from date boxes). */
export type NewIterationDraft = { name: string; start: string; end: string };

export const EMPTY_NEW_ITERATION: NewIterationDraft = { name: "", start: "", end: "" };

/** Whether the iteration editor's Save is offered: "New…" needs a name; otherwise a different choice. */
export function iterationSavable(task: Task, draft: Draft, created: NewIterationDraft): boolean {
  return draft.value === NEW_ITERATION ? created.name.trim() !== "" : draftChanged(task, "iteration", draft);
}

/** The new iteration Rust is asked for: the next free ID (as `alm add iteration <name>`), no release. */
export function toPaneIteration(created: NewIterationDraft): NewIteration {
  const orNull = (text: string) => text.trim() || null;
  return { id: null, name: created.name.trim(), release: null, start: orNull(created.start), end: orNull(created.end) };
}

/** A save refused because the task changed after the person started editing (Task-A01-57):
 * the task's version now, or null for any other refusal. */
export function changedVersion(e: unknown): string | null {
  if (typeof e !== "object" || e === null) return null;
  const p = e as { code?: unknown; params?: Record<string, unknown> };
  return p.code === "task_changed" && typeof p.params?.version === "string" ? p.params.version : null;
}
