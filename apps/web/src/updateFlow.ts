// What the window says about a newer Allomos (Task-A07-16). Kept apart from React so it can be
// tested with Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { UpdateStatus } from "./types";

/** How often the window asks the app; the app asks GitHub only when its last answer is a day old. */
export const UPDATE_LOOK_MS = 60 * 60 * 1000;

export type UpdateNote =
  /** The installer is downloaded and checked: Restart to Update, Skip this version and ×. The
   * notice never offers a version whose installer is not ready (Task-A07-fact210). */
  | { kind: "offer"; latest: string; current: string }
  /** The installer is downloading: its percentage (`null` while its size is not known), Cancel
   * and ×. Shown for a background download, and for Check for Updates… (A07-fact210). */
  | { kind: "downloading"; latest: string; current: string; percent: number | null }
  /** Only after Check for Updates…, while the installer is not ready and nothing is downloading
   * yet, or once its download has failed (A07-fact210). */
  | { kind: "preparing"; latest: string; current: string }
  /** Only after Check for Updates…: nothing newer. */
  | { kind: "current"; current: string }
  /** Only after Check for Updates…: GitHub did not answer. */
  | { kind: "failed" };

/** What the update menu item does: Check for Updates…; Download Allomos <version>… (greyed out
 * while the installer downloads, with its percentage when known, A07-glove300 and A07-fact210);
 * Restart to Update to Allomos <version> (Task-A07-17). */
export type UpdateMenuItem =
  | { action: "check" }
  | { action: "download"; latest: string; waiting: boolean; percent: number | null }
  | { action: "restart"; latest: string };

/** `percent` is how much of the installer is downloaded now (`update_progress`), or `null`. */
export function updateMenuItem(status: UpdateStatus | null, percent: number | null = null): UpdateMenuItem {
  const available = status?.available ?? null;
  if (!available) return { action: "check" };
  if (status?.ready === available) return { action: "restart", latest: available };
  const waiting = status?.preparing === available;
  return { action: "download", latest: available, waiting, percent: waiting ? percent : null };
}

/**
 * What to show. An offer appears only once the installer is ready. While it downloads, the
 * notice shows the progress (a check on its own shows it too, as the download is going on).
 * A check on its own (at start, then once a day) stays quiet otherwise; Check for Updates…
 * (`asked`) always answers. `hidden` is the version the person closed with × in this window.
 */
export function updateNote(
  status: UpdateStatus | null,
  asked: boolean,
  hidden: string | null,
  percent: number | null = null,
): UpdateNote | null {
  if (!status) return null;
  if (status.newer && (asked || status.newer !== hidden)) {
    const base = { latest: status.newer, current: status.current };
    if (status.ready === status.newer) return { kind: "offer", ...base };
    if (status.preparing === status.newer) return { kind: "downloading", ...base, percent };
  }
  if (status.newer && asked) return { kind: "preparing", latest: status.newer, current: status.current };
  if (!asked) return null;
  if (status.failed) return { kind: "failed" };
  return { kind: "current", current: status.current };
}
