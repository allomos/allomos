// Changing a task in the task pane (Task-A01-18): which fields can be changed, what each starts
// from, and the change sent to Rust. Rust decides whether a change is allowed, by the same rule
// as `alm`; nothing here refuses one.

import type { Edit, State, Task } from "./types";

/** The task pane's changeable fields. */
export type EditField = "state" | "label" | "waiting_on" | "next_dependency" | "handoff_note" | "iteration" | "title" | "description";

/** What the person typed or chose; `waitingOn` goes with the state `waiting`. */
export type Draft = { value: string; waitingOn: string };

/**
 * The states the app can set, each by its `alm` command (ready, wait, done, drop). In progress
 * comes only from an agent claiming the task, and nothing moves a task back to backlog
 * (owner, 2026-10-14: no claiming or releasing from the app).
 */
export const SETTABLE_STATES: State[] = ["ready", "waiting", "done", "dropped"];

/** The states offered for `task`: its own first, so the list shows where it is, then the settable ones. */
export function stateChoices(task: Task): State[] {
  return SETTABLE_STATES.includes(task.state) ? SETTABLE_STATES : [task.state, ...SETTABLE_STATES];
}

/** The draft a field's editor starts from: the task's value now. */
export function startDraft(task: Task, field: EditField): Draft {
  const value = field === "state" ? task.state : field === "description" ? task.description : (task[field] ?? "");
  return { value, waitingOn: task.waiting_on ?? "" };
}

/** Whether Save would change anything. Waiting with a new "waiting on" counts as a change. */
export function draftChanged(task: Task, field: EditField, draft: Draft): boolean {
  const start = startDraft(task, field);
  if (field === "state" && draft.value === "waiting") {
    return task.state !== "waiting" || draft.waitingOn.trim() !== start.waitingOn.trim();
  }
  if (field === "description") return draft.value.trimEnd() !== start.value.trimEnd();
  return draft.value.trim() !== start.value.trim();
}

/** The change Rust is asked for. Empty text clears a field that may be empty. */
export function toEdit(field: EditField, draft: Draft): Edit {
  const text = draft.value.trim();
  switch (field) {
    case "state":
      return draft.value === "waiting"
        ? { field, state: "waiting", waiting_on: draft.waitingOn.trim() }
        : { field, state: draft.value as State };
    case "label":
      return { field, label: draft.value };
    case "waiting_on":
      return { field, text };
    case "next_dependency":
    case "handoff_note":
      return { field, text: text || null };
    case "iteration":
      return { field, iteration: text || null };
    case "title":
      return { field, title: text };
    case "description":
      return { field, text: draft.value };
  }
}
