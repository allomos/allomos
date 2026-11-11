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

/** Following the taskbag's files (changes by `alm` or by hand): once a second in use. */
export const FOLLOW_MS = 1000;
export const FOLLOW_IDLE_MS = 5000;
/** The monitor runs (this project's and the Monitors bar): every two seconds in use. */
export const MONITOR_MS = 2000;
export const MONITOR_IDLE_MS = 10_000;
