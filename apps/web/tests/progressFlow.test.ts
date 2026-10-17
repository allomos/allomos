// Tests for the progress popup (Task-A05-08). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { QUIET_AFTER_S, arrange, fraction, progressState, stillWaiting } from "../src/progressFlow.ts";
import type { Progress } from "../src/types.ts";

const report = (id: string, more: Partial<Progress> = {}): Progress => ({ id, title: "Update the taskbag", started: 100, updated: 1000, ...more });

test("a report is working, finished, or quiet after 10 minutes without word", () => {
  assert.equal(progressState(report("a"), 1000 + 5), "working");
  assert.equal(progressState(report("a"), 1000 + QUIET_AFTER_S), "quiet");
  assert.equal(progressState(report("a", { finished: "3 cards moved in" }), 1000 + QUIET_AFTER_S * 9), "finished");
});

test("the bar shows only when the agent counts its steps", () => {
  assert.equal(fraction(report("a", { done: 1, of: 4 })), 0.25);
  assert.equal(fraction(report("a", { done: 0, of: 3 })), 0);
  assert.equal(fraction(report("a", { done: 2 })), null);
  assert.equal(fraction(report("a", { done: 1, of: 0 })), null);
  assert.equal(fraction(report("a")), null);
});

test("the popup shows the earliest report not in the background; the others are chips", () => {
  const list = [report("a"), report("b"), report("c")];
  assert.equal(arrange(list, new Set()).popup?.id, "a");
  const { popup, chips } = arrange(list, new Set(["a", "c"]));
  assert.equal(popup?.id, "b");
  assert.deepEqual(chips.map((p) => p.id), ["a", "c"]);
  assert.equal(arrange(list, new Set(["a", "b", "c"])).popup, null);
  assert.deepEqual(arrange([], new Set()), { popup: null, chips: [] });
});

test("the waiting popup shows from the moment the agent is opened until its first report", () => {
  const waiting = { root: "/p/.allomos/taskbag", job: "cards" as const, since: 2000 };
  assert.equal(stillWaiting(waiting, []), true);
  // An old report (an earlier job, not closed yet) does not end the wait.
  assert.equal(stillWaiting(waiting, [report("a", { updated: 1000, finished: "done" })]), true);
  assert.equal(stillWaiting(waiting, [report("a", { updated: 2001 })]), false);
  assert.equal(stillWaiting(null, []), false);
});
