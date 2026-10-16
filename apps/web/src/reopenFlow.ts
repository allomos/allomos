// Reopening a finished task from the task pane (Task-A01-50): which tasks offer Reopen…, and the
// change it asks for. Rust checks it by the same rule as `alm reopen` and records the reason.

import type { Edit, Task } from "./types";

/** Only a done or dropped task offers Reopen…. */
export function canReopen(task: Pick<Task, "state">): boolean {
  return task.state === "done" || task.state === "dropped";
}

/** The change Reopen asks for; null while no reason is written (the button stays off). */
export function reopenEdit(reason: string): Edit | null {
  const note = reason.trim();
  return note ? { field: "reopen", note } : null;
}
