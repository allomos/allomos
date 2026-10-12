// The Handoffs tab's choices (Task-A03-06), kept apart from React so they can be tested with Node
// alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.

import type { Handoff } from "./types";

export interface HandoffFilter {
  /** Only handoffs that name this task. */
  task: string | null;
  /** Only handoffs by this writer. */
  writer: string | null;
  /** Only the newest handoff per task (and per writer for those with no task). Default on. */
  latestOnly: boolean;
}

/** The handoffs to show, newest first (the list comes newest first). With `latestOnly`, an older
 * handoff is hidden once every task it names already has a newer one shown. */
export function filterHandoffs(all: Handoff[], f: HandoffFilter): Handoff[] {
  const picked = all.filter((h) => (!f.task || h.tasks.includes(f.task)) && (!f.writer || h.writer === f.writer));
  if (!f.latestOnly) return picked;
  const seen = new Set<string>();
  return picked.filter((h) => {
    const keys = h.tasks.length > 0 ? h.tasks.map((t) => `task:${t}`) : [`writer:${h.writer}`];
    const fresh = keys.some((k) => !seen.has(k));
    keys.forEach((k) => seen.add(k));
    return fresh;
  });
}

/** The newest handoff that names `task`. */
export function latestFor(all: Handoff[], task: string): Handoff | null {
  return all.find((h) => h.tasks.includes(task)) ?? null;
}

/** Everyone who wrote a handoff, in order of their newest one. */
export function writers(all: Handoff[]): string[] {
  return [...new Set(all.map((h) => h.writer).filter((w) => w))];
}

/** `2026-10-12T11:30Z` as the person's local date and time; the text as it is when it is not one. */
export function localTime(stamp: string): string {
  const d = new Date(stamp.replace(/Z$/, ":00Z"));
  if (Number.isNaN(d.getTime())) return stamp;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}
