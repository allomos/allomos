// Card files beside their tasks (Task-A00-30), kept apart from React so it can be tested with Node
// alone (`npm --prefix apps/web test`). Only type imports here: Node strips them.
//
// Allomos reads each task's `Card:` link (Task-A01-51), and an AI agent reports any other card
// files a project still keeps in its own folder (`alm cards found`). As soon as either names one,
// the window shows "N tasks keep their card files in the project folder" with Move the cards in…, which opens an agent with the move-cards request typed
// in. File → Move the Cards In… opens the same dialog at any time.

import type { CardOutside, Opened } from "./types";

/** The card files still outside the taskbag (linked or reported), for tasks the taskbag has. */
export function cardsOutside(opened: Opened | null): CardOutside[] {
  if (!opened?.loaded.taskbag) return [];
  const ids = new Set(opened.loaded.taskbag.tasks.map((t) => t.id));
  return (opened.cards_outside ?? []).filter((c) => ids.has(c.task));
}

/** How many tasks the notice names; 0: no notice. */
export function cardsNotice(opened: Opened | null): number {
  return cardsOutside(opened).length;
}
