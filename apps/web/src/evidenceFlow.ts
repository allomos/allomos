// The Evidence screen of a task to check (Task-A01-herb903, decision 0029 points 1 and 2). Which
// parts it shows and in which order come from the core with the taskbag (Task-A01-inlet632,
// crates/core/src/evidence.rs), as `alm show --evidence` lists them, so the two never differ. A
// part with nothing in it is left out, never drawn as an empty box. Test results are the agent's
// `alm check-result` records (point 5, Task-A01-easel128), ordered in the core; a task with none
// (older tasks) says so and leads to the description's Evidence section.

import type { Key } from "./i18n";
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

/** What changed, read from the project's own git (Task-A01-hyena938, decision 0029 point 3), as
 * the desktop app reads it (`code_changes`): each recorded commit's first line (null: not on this
 * computer), and the files changed with the lines added and removed (null: a binary file). */
export interface CodeChanges {
  commits: { hash: string; subject: string | null; merge: boolean }[];
  files: { path: string; added: number | null; removed: number | null }[];
}

/** How many changed files are listed before "and n more". */
export const MOST_FILES = 30;

/** The changed files together: how many, and the lines added and removed (binary files count as
 * files only). */
export function changeTotals(files: CodeChanges["files"]): { files: number; added: number; removed: number } {
  return files.reduce((s, f) => ({ files: s.files + 1, added: s.added + (f.added ?? 0), removed: s.removed + (f.removed ?? 0) }), { files: 0, added: 0, removed: 0 });
}

/** The first line read for the recorded commit `ref`: a string when found, null when this
 * computer does not have it, undefined while not read (or where git cannot be read). */
export function commitSubject(changes: CodeChanges | null, ref: string): string | null | undefined {
  return changes?.commits.find((c) => c.hash === ref)?.subject;
}

/** The words for a test result, by its written form; null for one Allomos does not know. */
export function resultKey(result: string): Key | null {
  return result === "fail" ? "evidence.result_fail" : result === "not-run" ? "evidence.result_not_run" : result === "pass" ? "evidence.result_pass" : null;
}
