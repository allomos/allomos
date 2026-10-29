// Taskbags as tabs (Task-A01-marble320, approver 2026-10-29): each window holds tabs like a
// browser's, one taskbag (or the start page) in each. What the tab bar does, without the screen.

/** What Allomos opens when it starts again (the app's Settings… › General). */
export type StartWith = "reopen" | "last" | "blank";

/** How a tab starts. `blank`: the start page. `open`: a folder from File → Open Recent (one that
 * is gone is said so and leaves Open Recent). `restore`: a tab of the last session (one that is
 * gone leaves quietly). `moved`: a tab dragged from another window, already open there.
 * `startup`: the first window at start-up with nothing restored: a folder given on the command
 * line, else the last taskbag (unless Allomos starts on a blank window). */
export type TabStart =
  | { how: "blank" }
  | { how: "open" | "restore" | "moved"; root: string }
  | { how: "startup"; startWith: StartWith };

/** A tab as the app sends it when a window opens (`window_start`). */
export interface StartTab {
  root: string | null;
  how: "open" | "restore" | "moved";
}

export interface WindowStart {
  tabs: StartTab[];
  active: number;
  /** The first window at start-up: what Allomos opens when it starts. */
  startup: StartWith | null;
}

/** One tab of the window: what it started with, the taskbag it shows now (none: the start page),
 * its project's name, and whether it needs the person. */
export interface TabEntry {
  id: number;
  start: TabStart;
  root: string | null;
  name: string | null;
  needs: boolean;
}

/** How each tab of a window that just opened starts. */
export function startTabs(start: WindowStart): TabStart[] {
  const tabs: TabStart[] = start.tabs.map((t) => (t.root ? { how: t.how, root: t.root } : { how: "blank" }));
  if (tabs.length === 0) tabs.push(start.startup ? { how: "startup", startWith: start.startup } : { how: "blank" });
  return tabs;
}

/** The tabs with the one at `from` moved to `to`. */
export function moveTab<T>(list: readonly T[], from: number, to: number): T[] {
  if (from === to || from < 0 || from >= list.length) return [...list];
  const next = [...list];
  const [tab] = next.splice(from, 1);
  next.splice(Math.max(0, Math.min(to, next.length)), 0, tab);
  return next;
}

/** A tab's left and right edges in the bar. */
export interface Edge {
  left: number;
  right: number;
}

/** The other tabs' edges with the dragged tab at `from` taken out of the bar: the tabs after it
 * close up by `space`, its width and the gap between tabs. */
export function othersOf(edges: readonly Edge[], from: number, space: number): Edge[] {
  return edges
    .filter((_, i) => i !== from)
    .map((e, i) => (i < from ? e : { left: e.left - space, right: e.right - space }));
}

/** Where a dragged tab lands among the `others` (their edges with no gap open), drawn from `left`
 * and `width` wide, while it is now to land at `gap` (the tabs from there slid right by `space`). It
 * trades places with a neighbour once it covers more than half of it, or of itself when it is the
 * narrower one (Task-A01-bell953): measured where that neighbour is shown, so it does not flip back. */
export function landAt(others: readonly Edge[], left: number, width: number, space: number, gap: number): number {
  const right = left + width;
  const half = (e: Edge) => Math.min(width, e.right - e.left) / 2;
  let at = Math.max(0, Math.min(gap, others.length));
  for (let step = 0; step <= 2 * others.length; step++) {
    const before = others[at - 1];
    if (before && left < before.right - half(before)) {
      at -= 1;
      continue;
    }
    const after = others[at];
    if (after && right > after.left + space + half(after)) {
      at += 1;
      continue;
    }
    break;
  }
  return at;
}

/** How far tab `i` slides aside while the tab at `from` is dragged to place `gap` (`w`: the dragged
 * tab's width), so the gap opens where it will land (Task-A01-fresco512). `from` -1: a tab from
 * another window, not in the bar yet: every tab from `gap` on slides right. */
export function shiftFor(i: number, from: number, gap: number, w: number): number {
  if (i === from) return 0;
  if (from < 0) return i >= gap ? w : 0;
  if (from < gap && i > from && i <= gap) return -w;
  if (from > gap && i >= gap && i < from) return w;
  return 0;
}

/** The list with `item` put in at place `at`. */
export function insertAt<T>(list: readonly T[], item: T, at: number): T[] {
  const next = [...list];
  next.splice(Math.max(0, Math.min(at, next.length)), 0, item);
  return next;
}

/** How far the pointer goes above or below the tab bar before the tab tears off, in pixels. */
export const TEAR_DISTANCE = 30;

/** Whether a tab held at (`x`, `y`) has left the tab bar (from `top` to `bottom`) far enough to tear
 * off: above or below it by more than TEAR_DISTANCE, or out of the window at the side. */
export function tearsOff(x: number, y: number, top: number, bottom: number, width: number): boolean {
  return y < top - TEAR_DISTANCE || y > bottom + TEAR_DISTANCE || x < 0 || x >= width;
}

/** The tab in front after tab `id` closes: the one after it, else the one before; null when it was
 * the last. Another tab closing leaves the front tab as it was. */
export function frontAfterClose(list: readonly { id: number }[], id: number, front: number | null): number | null {
  if (front !== id) return front;
  const at = list.findIndex((t) => t.id === id);
  const rest = list.filter((t) => t.id !== id);
  if (rest.length === 0) return null;
  return rest[Math.min(at, rest.length - 1)].id;
}

/** Another tab of the window that shows `root`, if any. */
export function tabShowing(list: readonly TabEntry[], root: string, not: number): TabEntry | null {
  const same = (a: string) => a.replace(/[\\/]+$/, "") === root.replace(/[\\/]+$/, "");
  return list.find((t) => t.id !== not && t.root !== null && same(t.root)) ?? null;
}
