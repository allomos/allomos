// The "3 of 5 done" count and meter in an area's or iteration's header (Task-A01-timber110).
// Dropped tasks were given up on purpose: they are neither done nor still to do, so they stay out
// of the total; counting them made a finished iteration read "25 of 26 done". Kept apart from
// React so it can be tested with Node alone (`npm --prefix apps/web test`).

/** How many of a group's tasks are done, out of those not dropped, and that share in percent. */
export function groupCount(states: readonly string[]): { done: number; total: number; pct: number } {
  const total = states.filter((s) => s !== "dropped").length;
  const done = states.filter((s) => s === "done").length;
  return { done, total, pct: total ? Math.round((done / total) * 100) : 0 };
}
