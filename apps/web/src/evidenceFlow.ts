// The Evidence screen of a task to check (Task-A01-herb903, decision 0029 points 1 and 2): what
// it shows and in which order, worked out from the records the task already has. A part with
// nothing in it is left out, never drawn as an empty box. Test results are not recorded yet
// (point 5, a later task): that part says so and leads to the description's Evidence section.

import type { LinkRecord, Task, TaskHistory } from "./types";

/** The parts, top to bottom (decision 0029 § 2). */
export type EvidencePart = "ask_risk" | "check_steps" | "summary" | "tests" | "decisions" | "handoff";

/** The parts this task has, in order. `decisions`: it has AI decisions to show (to review, or
 * details to count). */
export function evidenceParts(task: Task, decisions: boolean): EvidencePart[] {
  const parts: EvidencePart[] = ["ask_risk"];
  if (task.human_check_steps?.trim() || task.without_human_check?.trim()) parts.push("check_steps");
  if (summarySentence(task.handoff_note) || codeLinks(task).length > 0) parts.push("summary");
  parts.push("tests");
  if (decisions) parts.push("decisions");
  if (task.handoff_note?.trim()) parts.push("handoff");
  return parts;
}

/** The reason the agent gave with the newest risk it recorded (`alm classify --why`, kept in the
 * history as the change of `reason`); null when the history is not read or holds none. */
export function riskReason(history: TaskHistory | null): string | null {
  const events = history?.events ?? [];
  for (let i = events.length - 1; i >= 0; i--) {
    const e = events[i];
    if (e.command !== "classify") continue;
    const reason = e.changes.find((c) => c.field === "reason")?.to;
    if (typeof reason === "string" && reason.trim()) return reason.trim();
  }
  return null;
}

/** The handoff note's first sentence (decision 0025 asks it to be one a person can act on): its
 * first paragraph, up to the first full stop; markdown marks at its start are dropped. */
export function summarySentence(note: string | null | undefined): string {
  const first =
    (note ?? "")
      .split(/\n\s*\n/)
      .map((p) => p.trim())
      .find((p) => p.length > 0) ?? "";
  const line = first.replace(/\s*\n\s*/g, " ").replace(/^(#+\s+|[-*+]\s+|\d+\.\s+)/, "");
  const end = line.search(/[。！？]|[.!?](?=\s|$)/);
  return end < 0 ? line : line.slice(0, end + 1);
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
