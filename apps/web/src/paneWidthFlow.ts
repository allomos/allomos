// The task pane's width (Task-A01-56): the person drags the pane's inner edge, and the width is
// kept for next time as a share of the list-and-pane box (Task-A01-61), so the pane keeps its
// proportion when the window is resized or the app opens on another screen. Kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`).

/** The narrowest the pane can be, in pixels. */
export const MIN_PANE = 300;
/** The narrowest the task list beside it can be, in pixels, so it is never squeezed away. */
export const MIN_LIST = 320;
/** How far one arrow key press moves the edge, in pixels. */
export const KEY_STEP = 24;
/** Where the width's share is kept, in the app's own storage. */
export const PANE_WIDTH_KEY = "allomos.paneWidth";

/** The widest the pane can be beside a list `layout` pixels wide, with `gap` between them. */
export function maxPane(layout: number, gap = 0): number {
  return Math.max(MIN_PANE, Math.floor(layout - gap - MIN_LIST));
}

/** `width` kept between the limits for a list `layout` pixels wide. */
export function clampPane(width: number, layout: number, gap = 0): number {
  return Math.min(Math.max(Math.round(width), MIN_PANE), maxPane(layout, gap));
}

/** The pane's width while dragging: from the pointer's x to the layout's right edge. */
export function dragWidth(pointerX: number, layoutRight: number, layout: number, gap = 0): number {
  return clampPane(layoutRight - pointerX, layout, gap);
}

/** The share of a box `layout` pixels wide that a pane `width` pixels wide takes, between 0 and 1. */
export function paneRatio(width: number, layout: number): number {
  if (!(layout > 0)) return 0;
  return Math.min(Math.max(width / layout, 0), 1);
}

/** The pane's width in pixels for a share `ratio` of a box `layout` pixels wide, kept between the limits. */
export function ratioWidth(ratio: number, layout: number, gap = 0): number {
  return clampPane(ratio * layout, layout, gap);
}

/** The text kept in storage for a share: rounded to four places, which is well under a pixel. */
export function formatPaneRatio(ratio: number): string {
  return String(Math.round(ratio * 10000) / 10000);
}

/**
 * A share read back from storage; anything unreadable (or never set) is null: the default width.
 * A pixel width kept by an app before Task-A01-61 (a number of 1 or more) is also null: without the
 * box's width at that time it cannot be turned into a share, so the pane starts at the default.
 */
export function parsePaneRatio(text: string | null): number | null {
  if (!text) return null;
  const n = Number(text);
  return Number.isFinite(n) && n > 0 && n < 1 ? n : null;
}
