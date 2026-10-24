// Each iteration's place in the plan, shown as a number (1, 2, 3…) beside its ID (Task-A00-turkey877):
// iteration IDs are words (`I-knit`), so the number is what shows the order. It is counted here
// from the iterations' order, never stored, so it follows `alm move iteration`.

import type { Iteration } from "./types";

/** The iterations in plan order (by `order`, then by ID, as the taskbag sorts them). */
function inPlanOrder(iterations: Iteration[]): Iteration[] {
  return [...iterations].sort((a, b) => a.order - b.order || (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
}

/** Iteration ID → its place, counted from 1. */
export function iterationPlaces(iterations: Iteration[]): Map<string, number> {
  return new Map(inPlanOrder(iterations).map((it, i) => [it.id, i + 1]));
}

/** How the New task form's iteration picker shows one: its place, then its name. */
export function placedName(place: number | undefined, name: string): string {
  return place === undefined ? name : `${place} · ${name}`;
}
