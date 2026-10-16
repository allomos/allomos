// The progress popup (Task-A05-08), kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// While an AI agent works on a job the app gave it, it reports with `alm progress`; the window
// shows the report as a popup over everything, which the person can send to the background
// (a chip that brings it back). Finished, it says what happened; silent for 10 minutes, it says
// so, so the window is never stuck.

import type { Progress } from "./types";

/** An agent that has not reported for this long may have stopped. */
export const QUIET_AFTER_S = 10 * 60;

export type ProgressState = "working" | "finished" | "quiet";

/** Where a report stands at `now` (seconds since 1970). */
export function progressState(p: Progress, now: number): ProgressState {
  if (p.finished !== undefined) return "finished";
  return now - p.updated >= QUIET_AFTER_S ? "quiet" : "working";
}

/** How far along, from 0 to 1, when the agent counts its steps; otherwise null. */
export function fraction(p: Progress): number | null {
  if (p.done === undefined || !p.of) return null;
  return Math.min(1, Math.max(0, p.done / p.of));
}

/** The report the popup shows (the earliest not in the background), and the ones shown as chips. */
export function arrange(reports: Progress[], background: ReadonlySet<string>): { popup: Progress | null; chips: Progress[] } {
  return {
    popup: reports.find((p) => !background.has(p.id)) ?? null,
    chips: reports.filter((p) => background.has(p.id)),
  };
}
