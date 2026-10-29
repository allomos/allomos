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

/** Where the tab at `from`, dragged along the bar with the pointer at `x`, goes: after every other
 * tab (each given by its left and right edge) whose middle the pointer has passed. */
export function placeAt(edges: readonly { left: number; right: number }[], from: number, x: number): number {
  return edges.filter((e, i) => i !== from && x > (e.left + e.right) / 2).length;
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

/** A point outside the window (in the window's own coordinates): a tab let go there leaves it. */
export function outsideWindow(x: number, y: number, width: number, height: number): boolean {
  return x < 0 || y < 0 || x >= width || y >= height;
}

/** Another tab of the window that shows `root`, if any. */
export function tabShowing(list: readonly TabEntry[], root: string, not: number): TabEntry | null {
  const same = (a: string) => a.replace(/[\\/]+$/, "") === root.replace(/[\\/]+$/, "");
  return list.find((t) => t.id !== not && t.root !== null && same(t.root)) ?? null;
}
