// The Evidence screen of a task to check (Task-A01-herb903, decision 0029 points 1 and 2). Which
// parts it shows and in which order come from the core with the taskbag (Task-A01-inlet632,
// crates/core/src/evidence.rs), as `alm show --evidence` lists them, so the two never differ. A
// part with nothing in it is left out, never drawn as an empty box. Test results are not recorded
// yet (point 5, a later task): that part says so and leads to the description's Evidence section.

import type { EvidencePart, LinkRecord, Task } from "./types";

export type { EvidencePart };

/** The parts this task has, in order, as the core sent them; before it has (a task just made in
 * the window), only the two every task has. */
export function evidenceParts(task: Pick<Task, "evidence">): EvidencePart[] {
  return task.evidence?.parts ?? ["ask_risk", "tests"];
}

/** The commits and merges the task recorded, in the order recorded. */
export function codeLinks(task: Pick<Task, "links">): LinkRecord[] {
  return (task.links ?? []).filter((l) => l.kind === "commit" || l.kind === "merge");
}

/** Whether a heading's (or a bold lead's) text names the Evidence section, as agents write it:
 * "Evidence", "Evidence.", "Evidence (A01-x)", "证据", "証拠". */
export function isEvidenceHeading(text: string): boolean {
  const s = text.trim().toLowerCase();
  return /^evidence\b/.test(s) || s.startsWith("证据") || s.startsWith("証拠") || s.startsWith("エビデンス");
}
