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
// - Where it runs (Task-A03-hive049): what the agent passed at `alm claim`. A session in the
//   Claude desktop app opens there, one in a terminal resumes in a new terminal. The task keeps
//   it after release and done, so the row then reads "Last session".

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
  | { kind: "unknown" }
  /** Nobody holds it: the session that last held it (Task-A03-hive049); `at`, its last change. */
  | { kind: "last_session"; at: string | null };

/** Where the recorded session runs, so the pane can reopen it. */
export type SessionPlace = "app" | "terminal";

/** Where the task's recorded session runs; null: nothing to reopen (older claims, other agents). */
export function sessionPlace(task: Task): SessionPlace | null {
  const s = task.session;
  if (s?.app_session_id?.trim()) return "app";
  if (s?.id?.trim()) return "terminal";
  return null;
}

/** The row under the title (Task-A03-smile488): the human check, when the task waits for one, and
 * the session's open button at its far right, whenever the task records a session and this app
 * can open it (the desktop app). `waitsForCheck` is stopFlow's toCheck. Null: no row, as before. */
export type CheckRow = { toCheck: boolean; open: SessionPlace | null };

export function checkRow(task: Task, waitsForCheck: boolean, canOpen: boolean): CheckRow | null {
  const open = canOpen ? sessionPlace(task) : null;
  return waitsForCheck || open ? { toCheck: waitsForCheck, open } : null;
}

/** The holder's session; when nobody holds the task, the last one it records; else null. */
export function session(task: Task, history: TaskHistory | null, now: Date): Session | null {
  const holder = task.holder?.trim();
  if (!holder) {
    if (!sessionPlace(task)) return null;
    const by = task.session?.by?.trim();
    return { kind: "last_session", at: (by && history?.seen?.find((s) => s.by === by)?.at) || null };
  }
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
