// Approve and Withdraw in the task pane (Task-A01-baker800, decision 0021 § 1.1). Rust records
// them by the same rules as `alm approve` (channel app) and `alm withdraw`; neither changes the
// task's state (approver, 2026-11-08).

import type { ApprovalRecord, Task } from "./types";

/** Only an approval that applies offers Withdraw…. */
export function canWithdraw(a: Pick<ApprovalRecord, "status">): boolean {
  return a.status === "applies";
}

/** Approve and make ready is offered beside Approve only on a backlog task. */
export function canMakeReady(task: Pick<Task, "state">): boolean {
  return task.state === "backlog";
}

/** There is a plan to approve: the description, as it is, is what the person approves. */
export function hasPlan(task: Pick<Task, "description">): boolean {
  return task.description.trim() !== "";
}

/** The expiry sent: a date picker's `YYYY-MM-DD`, or none. */
export function expiresValue(value: string): string | null {
  return /^\d{4}-\d{2}-\d{2}$/.test(value.trim()) ? value.trim() : null;
}

/** The withdrawn approval to point out (the same rule as `alm show` and `alm next`,
 * `allomos_core::approval::withdrawn_notice`): while the task is in progress, its latest approval
 * when that one was withdrawn. It stops nothing; the holder decides. */
export function withdrawnNotice(task: Pick<Task, "state" | "approvals">): ApprovalRecord | null {
  if (task.state !== "in_progress") return null;
  const last = (task.approvals ?? []).at(-1);
  return last?.status === "withdrawn" ? last : null;
}
