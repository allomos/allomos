// Reopening a finished task from the task pane (Task-A01-50), now named Send back… (Task-A01-music456,
// decision 0030 § 4): which tasks offer it, and the change it asks for. Rust checks it by the same
// rule as `alm reopen` and records the reason; the next session reads it first (§ 5, 6).

import type { Edit, Task } from "./types";

/** Only a done or dropped task offers Send back…. */
export function canReopen(task: Pick<Task, "state">): boolean {
  return task.state === "done" || task.state === "dropped";
}

/** The line naming the attached screenshot at the end of the reason, as `alm` prints one, so the
 * next session finds the picture in the taskbag (language-neutral: agents read it). */
export function screenshotLine(task: string, file: string): string {
  return `screenshot: tasks/${task}/evidence/${file}`;
}

/** The change Send back asks for; null while no reason is written (the button stays off). With
 * `shot` (a task ID and the attached picture's kept name), the reason ends with its line. */
export function reopenEdit(reason: string, shot?: { task: string; file: string }): Edit | null {
  const note = reason.trim();
  if (!note) return null;
  return { field: "reopen", note: shot ? `${note}\n\n${screenshotLine(shot.task, shot.file)}` : note };
}

/** The reason box filled in for disagreeing with one AI decision: its words quoted, then a line
 * for why, worded by `intro` and `why` (the person's language). */
export function disagreeText(decision: string, intro: string, why: string): string {
  const quoted = decision
    .trim()
    .split("\n")
    .map((l) => `> ${l}`)
    .join("\n");
  return `${intro}\n${quoted}\n\n${why}`;
}

/** Whether the reason still says only what was filled in (nothing of the person's own yet). */
export function onlyFilledIn(reason: string, filled: string | null): boolean {
  return filled !== null && reason.trim() === filled.trim();
}
