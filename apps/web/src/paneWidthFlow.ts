// The task pane's width (Task-A01-56): the person drags the pane's inner edge, and the width is
// kept for next time. Kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`).

/** The narrowest the pane can be, in pixels. */
export const MIN_PANE = 300;
/** The narrowest the task list beside it can be, in pixels, so it is never squeezed away. */
export const MIN_LIST = 320;
/** How far one arrow key press moves the edge, in pixels. */
export const KEY_STEP = 24;
/** Where the width is kept, in the app's own storage. */
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

/** A width read back from storage; anything unreadable (or never set) is null: the default width. */
export function parsePaneWidth(text: string | null): number | null {
  if (!text) return null;
  const n = Number(text);
  return Number.isFinite(n) && n >= MIN_PANE ? Math.round(n) : null;
}
