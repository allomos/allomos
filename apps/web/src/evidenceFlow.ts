// The Evidence screen of a task to check (Task-A01-herb903, decision 0029 points 1 and 2): what
// it shows and in which order, worked out from the records the task already has. A part with
// nothing in it is left out, never drawn as an empty box. Test results are the agent's
// `alm check-result` records (point 5, Task-A01-easel128); a task with none (older tasks) says
// so and leads to the description's Evidence section.

import type { Key } from "./i18n";
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
 * history as the change of `reason`); null when the history is not read or holds none. The
 * history lists its events newest first. */
export function riskReason(history: TaskHistory | null): string | null {
  for (const e of history?.events ?? []) {
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

/** One check's result as the agent recorded it with `alm check-result` (decision 0029 § 5). */
export type CheckResult = { name: string; result: string; detail?: string };

/** The results in the order shown: failed, then not run, then passed (anything else last). */
const RESULT_ORDER = ["fail", "not-run", "pass"];

/** The task's test results from its history (newest first, as read): the last record of each
 * name, failed first, then not run, then passed; within each, in the order first recorded. */
export function checkResults(history: TaskHistory | null): CheckResult[] {
  const byName = new Map<string, CheckResult>();
  const events = history?.events ?? [];
  for (let i = events.length - 1; i >= 0; i--) {
    for (const c of events[i].changes) {
      if (c.field !== "check_result" || !c.to || typeof c.to !== "object") continue;
      const r = c.to as Partial<CheckResult>;
      if (typeof r.name !== "string" || typeof r.result !== "string") continue;
      byName.set(r.name, { name: r.name, result: r.result, ...(typeof r.detail === "string" && r.detail ? { detail: r.detail } : {}) });
    }
  }
  const rank = (r: CheckResult) => {
    const i = RESULT_ORDER.indexOf(r.result);
    return i < 0 ? RESULT_ORDER.length : i;
  };
  return [...byName.values()].map((r, i) => ({ r, i })).sort((a, b) => rank(a.r) - rank(b.r) || a.i - b.i).map(({ r }) => r);
}

/** The words for a result, by its written form; null for one Allomos does not know. */
export function resultKey(result: string): Key | null {
  return result === "fail" ? "evidence.result_fail" : result === "not-run" ? "evidence.result_not_run" : result === "pass" ? "evidence.result_pass" : null;
}
