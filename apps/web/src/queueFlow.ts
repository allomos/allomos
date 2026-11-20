// The For You window's list (Task-A01-oak609, decision 0028): what needs the person, one row per
// task, in the order `alm human` lists it. The order itself comes from Rust (allomos_core::queue);
// here are only the words of a row and which rows fold into one: the tasks not classified, last
// in the list, fold into one line that starts folded (decision 0028 § 7).
// Kept apart from React so it can be tested with Node alone (`npm --prefix apps/web test`).

import type { Key } from "./i18n";
import type { ConfirmRequest, Task } from "./types";

type T = (key: Key, params?: Record<string, string | number>) => string;

/** One thing the person has to do about a task (allomos_core::queue::Todo). */
export type Todo = "answer" | "approve" | "approve_afterwards" | "check" | "review";

/** Why a row in For You cannot be ticked for batch approval (decision 0030 § 2). */
export type CannotBatchReason = "agent_waiting" | "high_risk" | "test_failed" | "not_approvable";

/** One row of the queue, as `human_queue` sends it (allomos_core::queue::Row). */
export interface QueueRow {
  task: string;
  todo: Todo[];
  /** How many AI decisions wait for review. */
  decisions: number;
  /** An agent is waiting for one of the row's todos: the first group. */
  agent_waiting: boolean;
  risk: string | null;
  /** Since when it waits for the person (UTC); null when the history does not say. */
  waiting_since: string | null;
  /** How many unfinished tasks it holds back. */
  blocks: number;
}

/** A plan waiting for a Go ahead on a task, as `human_queue` takes it. */
export interface AskedNow {
  task: string;
  at: string;
}

/** The `alm confirm` questions open now that name a task, for the queue's first group. */
export function askedNow(requests: readonly ConfirmRequest[]): AskedNow[] {
  return requests
    .filter((r): r is ConfirmRequest & { task: string } => !!r.task)
    .map((r) => ({ task: r.task, at: new Date(r.asked * 1000).toISOString().replace(/\.\d{3}Z$/, "Z") }));
}

/** The list as it is drawn: the rows shown one by one, then the tasks not classified, folded into
 * one line. An agent waiting always shows its row, classified or not. */
export function splitQueue(rows: readonly QueueRow[]): { shown: QueueRow[]; unclassified: QueueRow[] } {
  const folded = (r: QueueRow) => !r.agent_waiting && !["high", "medium", "low"].includes(r.risk ?? "");
  return { shown: rows.filter((r) => !folded(r)), unclassified: rows.filter(folded) };
}

/** What the row asks of the person, in one line: "Check + review 2 AI decisions". */
export function todoText(row: Pick<QueueRow, "todo" | "decisions">, t: T): string {
  const parts = row.todo.map((d) =>
    d === "review" && row.decisions > 0 ? t("for_you.todo.review_n", { n: row.decisions }) : t(`for_you.todo.${d}` as Key),
  );
  return parts.join(" + ");
}

/** The one sentence at the top of the pane: what the person is asked to do there. */
export function askText(row: Pick<QueueRow, "todo" | "decisions">, t: T): string {
  return t("for_you.ask", { todo: todoText(row, t) });
}

/** The row's risk word: "High risk", or "Not classified". */
export function riskWord(risk: string | null, t: T): string {
  const level = ["high", "medium", "low"].includes(risk ?? "") ? risk! : "none";
  return t(`for_you.risk.${level}` as Key);
}

/** How long it has waited, in whole minutes, hours or days; null when the history does not say. */
export function waitedText(since: string | null, now: Date, t: T): string | null {
  if (!since) return null;
  const ms = Date.parse(since);
  if (Number.isNaN(ms)) return null;
  const minutes = Math.max(0, Math.floor((now.getTime() - ms) / 60000));
  if (minutes < 60) return t("for_you.waited_minutes", { n: minutes });
  if (minutes < 48 * 60) return t("for_you.waited_hours", { n: Math.floor(minutes / 60) });
  return t("for_you.waited_days", { n: Math.floor(minutes / (24 * 60)) });
}

/** The row chosen after the list changed: the same task while it is still listed; otherwise the
 * row now at the place it had (the next one), else the last; none when the list is empty. */
export function nextChoice(before: readonly QueueRow[], after: readonly QueueRow[], chosen: string | null): string | null {
  if (after.length === 0) return null;
  if (chosen && after.some((r) => r.task === chosen)) return chosen;
  const at = chosen ? before.findIndex((r) => r.task === chosen) : -1;
  if (at < 0) return after[0].task;
  // The next row of the old list still listed, so handling one row moves on to the one below it.
  const next = before.slice(at + 1).find((r) => after.some((a) => a.task === r.task));
  return next?.task ?? after[after.length - 1].task;
}

/** The window's title: "For You · <project>". */
export function forYouTitle(project: string | null | undefined, t: T): string {
  return project?.trim() ? t("for_you.window_title", { project: project.trim() }) : t("for_you.name");
}

/** Whether this row can be ticked for batch approval (decision 0030 § 2).
 * Rows with an agent waiting, high risk, or failed/unrun tests have no checkbox.
 * Unclassified rows can be ticked. */
export function cannotBatchApproveReason(row: QueueRow, task?: Task | null): CannotBatchReason | null {
  if (row.agent_waiting) return "agent_waiting";
  if (row.risk === "high") return "high_risk";
  const results = task?.evidence?.check_results ?? [];
  if (results.some((r) => r.result !== "pass")) return "test_failed";
  if (!row.todo.includes("check") && !row.todo.includes("review")) return "not_approvable";
  return null;
}

/** The reason text written on the row when it has no checkbox (decision 0030 § 2). */
export function cannotBatchApproveText(reason: CannotBatchReason, t: T): string {
  return t(`for_you.batch.reason_${reason}` as Key);
}

/** Split selected tasks into IDs for `alm checked` and `alm reviewed`, and count unclassified. */
export function batchApprovePartition(
  selectedIds: readonly string[],
  rows: readonly QueueRow[],
): { checkIds: string[]; reviewIds: string[]; unclassifiedCount: number } {
  const rowMap = new Map(rows.map((r) => [r.task, r]));
  const checkIds: string[] = [];
  const reviewIds: string[] = [];
  let unclassifiedCount = 0;
  for (const id of selectedIds) {
    const r = rowMap.get(id);
    if (!r) continue;
    if (r.todo.includes("check")) checkIds.push(id);
    if (r.todo.includes("review")) reviewIds.push(id);
    if (!["high", "medium", "low"].includes(r.risk ?? "")) unclassifiedCount++;
  }
  return { checkIds, reviewIds, unclassifiedCount };
}
