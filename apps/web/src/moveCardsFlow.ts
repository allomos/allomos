// Card files beside their tasks (Task-A00-30), and an older Allomos's taskbag (Task-A01-45): one
// Update the taskbag… for both. Kept apart from React so it can be tested with Node alone
// (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// Allomos reads each task's `Card:` link (Task-A01-51), and an AI agent reports any other card
// files a project still keeps in its own folder (`alm cards found`). As soon as either names one,
// the window shows "N tasks keep their card files in the project folder" with Update the
// taskbag…, which opens an agent with the move-cards request typed in. File → Update the
// Taskbag… opens the same dialog at any time; the dialog first says what Allomos found
// (approver, 2026-10-17).

import type { CardOutside, Opened, VersionCheck } from "./types";

/** The card files still outside the taskbag (linked or reported), for tasks the taskbag has. */
export function cardsOutside(opened: Opened | null): CardOutside[] {
  if (!opened?.loaded.taskbag) return [];
  const ids = new Set(opened.loaded.taskbag.tasks.map((t) => t.id));
  return (opened.cards_outside ?? []).filter((c) => ids.has(c.task));
}

/** One line of the dialog's list of what Allomos found. */
export interface CardRow {
  task: string;
  title: string;
  /** Where the card file is now, relative to the project folder. */
  path: string;
  /** An agent reported it (`alm cards found`), rather than the task's Card: link naming it. */
  reported: boolean;
}

/** The dialog's list: each task whose card file is still outside the taskbag, with its title. */
export function cardRows(opened: Opened | null): CardRow[] {
  const tasks = opened?.loaded.taskbag?.tasks ?? [];
  const title = new Map(tasks.map((t) => [t.id, t.title]));
  return cardsOutside(opened).map((c) => ({
    task: c.task,
    title: title.get(c.task) ?? "",
    path: c.path,
    reported: c.source === "report",
  }));
}

/** An older taskbag's handoff files in `handoffs/` (Task-A03-polar770): their notice, and part of
 * Update the taskbag…. */
export function oldHandoffs(opened: Opened | null): string[] {
  return opened?.old_handoffs ?? [];
}

/** How many tasks the notice names; 0: no notice. */
export function cardsNotice(opened: Opened | null): number {
  return cardsOutside(opened).length;
}

/** An older Allomos's taskbag, renamed aside to `old`, whose tasks Update the taskbag… asks an
 * agent to move into the new one (Task-A01-45). */
export interface MoveIn {
  found: string;
  old: string;
}

/** What Update the taskbag… does first (approver 2026-10-16: one button for an older taskbag and
 * for card files): `aside` renames an older taskbag aside before the dialog opens; then the dialog
 * asks for the move-in (`moveIn`) or, with none, for the card files. */
export function tasksInStart(check: VersionCheck | null): { aside: boolean; moveIn: MoveIn | null } {
  const m = check?.move_in;
  if (!m) return { aside: false, moveIn: null };
  return { aside: !m.prepared, moveIn: m.prepared ? { found: m.found, old: m.old } : null };
}
