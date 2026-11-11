// How often the window's background checks run (Task-A03-reef286): at their usual pace while
// someone is using the window, more slowly while it is in the background, so an idle Allomos
// costs next to no CPU. Coming back to the window checks at once.

/** In use: the window is shown and has the keyboard focus. */
export function inUse(doc: { hidden: boolean; hasFocus(): boolean }): boolean {
  return !doc.hidden && doc.hasFocus();
}

/** The pause before the next check: `awakeMs` while in use, `idleMs` otherwise. */
export function paceMs(using: boolean, awakeMs: number, idleMs: number): number {
  return using ? awakeMs : Math.max(awakeMs, idleMs);
}

/** Following the taskbag's files (changes by `alm` or by hand; Task-A03-spider528): the app hears
 * of a change from the system, so the window checks the taskbag's fingerprint itself only once a
 * minute, in case a notice was missed (a network drive, a cloud-synced folder). */
export const FOLLOW_FALLBACK_MS = 60_000;
/** A companion page (another device, through Devices on Your Network) hears of no change, so it
 * asks the computer: once a second in use, every five seconds otherwise. */
export const COMPANION_FOLLOW_MS = 1000;
export const COMPANION_FOLLOW_IDLE_MS = 5000;

/** How often the window checks the taskbag's fingerprint itself: `pushed` says whether the app
 * tells it of changes. */
export function followPace(pushed: boolean): { awakeMs: number; idleMs: number } {
  return pushed
    ? { awakeMs: FOLLOW_FALLBACK_MS, idleMs: FOLLOW_FALLBACK_MS }
    : { awakeMs: COMPANION_FOLLOW_MS, idleMs: COMPANION_FOLLOW_IDLE_MS };
}

// The monitor runs (this project's status bars, the Monitors bar) are no longer asked for by the
// window: the app's monitor loop tells every window when they change (Task-A03-spider528).
