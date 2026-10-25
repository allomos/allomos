// The companion page's choices (Task-A04-02-03), kept apart from React so they can be tested with
// Node alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

/** A taskbag the desktop app has open, as `/api/taskbags` lists it. */
export interface OpenTaskbag {
  root: string;
  name: string;
}

/** Which taskbag the page shows: the one chosen while it is still open; otherwise the only one
 * open; with several and none chosen, none (the page lists them to choose from). */
export function shownRoot(open: readonly OpenTaskbag[], chosen: string | null): string | null {
  if (chosen && open.some((b) => b.root === chosen)) return chosen;
  return open.length === 1 ? open[0].root : null;
}

/** How long the page waits between looks: at the list of open taskbags, and at a closed desktop app. */
export const WATCH_MS = 3000;
export const RETRY_MS = 5000;
