// What the window says about a newer Allomos (Task-A07-16). Kept apart from React so it can be
// tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { UpdateStatus } from "./types";

/** How often the window asks the app; the app asks GitHub only when its last answer is a day old. */
export const UPDATE_LOOK_MS = 60 * 60 * 1000;

export type UpdateNote =
  /** The notice with Download (or Restart to Update once `ready`, as the menu item has it),
   * Skip this version and ×. */
  | { kind: "offer"; latest: string; current: string; ready: boolean }
  /** Only after Check for Updates…: nothing newer. */
  | { kind: "current"; current: string }
  /** Only after Check for Updates…: GitHub did not answer. */
  | { kind: "failed" };

/** What the update menu item does: Check for Updates…; Download Allomos <version>… (opens the
 * download page, Task-A07-16); Restart to Update to Allomos <version> (Task-A07-17). */
export type UpdateMenuItem =
  | { action: "check" }
  | { action: "download"; latest: string }
  | { action: "restart"; latest: string };

export function updateMenuItem(status: UpdateStatus | null): UpdateMenuItem {
  const available = status?.available ?? null;
  if (!available) return { action: "check" };
  if (status?.ready === available) return { action: "restart", latest: available };
  return { action: "download", latest: available };
}

/**
 * What to show. A check on its own (at start, then once a day) shows only an offer and stays
 * quiet otherwise; Check for Updates… (`asked`) always answers. `hidden` is the version the
 * person closed with × in this window.
 */
export function updateNote(status: UpdateStatus | null, asked: boolean, hidden: string | null): UpdateNote | null {
  if (!status) return null;
  if (status.newer && (asked || status.newer !== hidden)) {
    return { kind: "offer", latest: status.newer, current: status.current, ready: status.ready === status.newer };
  }
  if (!asked) return null;
  if (status.failed) return { kind: "failed" };
  return { kind: "current", current: status.current };
}
