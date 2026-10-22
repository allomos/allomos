// Who stands behind a task, as separate items in the task pane (Task-A03-37): the responsible
// person, the agent, the check and the agent's session.
//
// - Responsible: always the project's approver (approver, 2026-10-22); a per-task field waits for
//   cloud projects with roles.
// - Agent: the session holding the task, or, when nobody holds it, the last one that changed it,
//   with the AI agent it ran under when `alm` recorded one (Task-A03-27).
// - Session: Allomos cannot see whether a session is running. A holder that changed anything in
//   the taskbag within the last 30 minutes is "active"; otherwise "last seen <time>" (approver,
//   2026-10-22). A holder copied from an old board has no change at all and says so, instead of
//   reading as an agent working now (owner, 2026-10-10).

import type { Task, TaskHistory } from "./types";

/** Changed the taskbag this recently: "active". */
export const ACTIVE_MINUTES = 30;

export interface PaneAgent {
  /** The session's `--as` name. */
  by: string;
  /** The AI agent `alm` recorded (`claude`, `codex`), if any; the pane names it (quotaFlow's agentName). */
  agent: string | null;
  /** It holds the task now; false: it was the last to change it. */
  holding: boolean;
}

/** The task's agent: its holder, else the newest `alm` change's name; null when neither is known. */
export function paneAgent(task: Task, history: TaskHistory | null): PaneAgent | null {
  const holder = task.holder?.trim();
  const by = holder || history?.events.find((e) => e.via === "alm" && e.by?.trim())?.by?.trim();
  if (!by) return null;
  const seen = history?.seen?.find((s) => s.by === by);
  return { by, agent: seen?.agent || null, holding: !!holder };
}

export type Session =
  | { kind: "active"; at: string }
  | { kind: "last_seen"; at: string }
  /** The holder has no recorded change anywhere in the taskbag. */
  | { kind: "unknown" };

/** The holder's session; null when nobody holds the task. */
export function session(task: Task, history: TaskHistory | null, now: Date): Session | null {
  const holder = task.holder?.trim();
  if (!holder) return null;
  const seen = history?.seen?.find((s) => s.by === holder);
  if (!seen) return { kind: "unknown" };
  const at = Date.parse(seen.at);
  if (Number.isNaN(at)) return { kind: "unknown" };
  return now.getTime() - at <= ACTIVE_MINUTES * 60_000 ? { kind: "active", at: seen.at } : { kind: "last_seen", at: seen.at };
}

/** The human check: a person checked it (done at the top label), it waits for one, or not yet. */
export type Check = "checked" | "to_check" | "not_yet";

export function check(task: Task, labels: string[]): Check {
  // Like stopFlow's toCheck: finished without the human check.
  if (task.state === "done" && !!task.without_human_check) return "to_check";
  const top = labels[labels.length - 1];
  return task.state === "done" && !!top && task.label === top ? "checked" : "not_yet";
}
