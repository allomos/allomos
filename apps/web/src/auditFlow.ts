// Project settings → Audit → Identity (Task-A01-frolic587, decision 0021 § 6): what the section
// decides without the window, so it is tested on its own.

import type { IdentityLevel } from "./types";

/** Every identity level, lowest first, as `alm audit-settings --identity` takes them. */
export const LEVELS: IdentityLevel[] = ["pseudonym", "account", "sso"];

/** Lowering the level, or turning signing in again off, needs a reason (decision 0021 § 6: an
 * auditor's first question is when a control was switched off, and why). */
export function needsReason(now: { level: IdentityLevel; reauthenticate: boolean }, level: IdentityLevel, reauthenticate: boolean): boolean {
  return LEVELS.indexOf(level) < LEVELS.indexOf(now.level) || (now.reauthenticate && !reauthenticate);
}

/** Turning separation of duties off needs a reason (Task-A01-grotto036, decision 0021 § 6);
 * turning it on does not. */
export function separationNeedsReason(now: boolean, wanted: boolean): boolean {
  return now && !wanted;
}
