// What the task pane shows first, by the task's state (Task-A01-berry927, decision 0025 point 2).
// The parts named here open at the top of the pane, in this order; everything else (the field
// table, the description, the handoff note and the history, when not named) stays folded.
// The check box and the AI decisions box keep their own places at the top: they show whenever the
// task is to check or to review, so they are not parts here.

import type { Task } from "./types";

export type FocusPart = "check_steps" | "question" | "blockers" | "handoff" | "description";

/** The parts to show first, in order. `toCheck`: a done task waiting for the human check (stopFlow's
 * toCheck); `lost`: the holder of an in-progress task is lost. */
export function paneFocus(task: Task, toCheck: boolean, lost: boolean): FocusPart[] {
  const handoff = task.handoff_note?.trim() ? (["handoff"] as const) : [];
  switch (task.state) {
    case "done":
      // To check: the steps a person follows. Otherwise the outcome, as the description says it.
      return toCheck ? ["check_steps"] : ["description"];
    case "waiting":
      // The question the task waits on, then where the work stopped.
      return ["question", ...handoff];
    case "blocked":
      return ["blockers", ...handoff];
    case "in_progress":
      // Lost or not, the handoff note says where the work is; a lost one has nothing else to go by.
      return lost ? [...handoff, "description"] : [...handoff];
    default:
      // Ready, backlog, dropped: what the task is for; a released task's note says what is left.
      return [...handoff, "description"];
  }
}
