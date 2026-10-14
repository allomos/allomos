// Tests for how a stopped task is worded, and the state filter (decision 0016, Task-A01-31).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { allBlockersDone, blockers, keepState, readFilter, stopText } from "../src/stopFlow.ts";
import type { Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };

// The English words, with parameters filled in, as the interface shows them.
const t = (key: string, params?: Record<string, string | number>) =>
  ((en as Record<string, string>)[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));

const base: Task = {
  id: "A01-31",
  title: "Blocked state",
  area: "A01",
  state: "ready",
  label: null,
  holder: null,
  waiting_on: null,
  next_dependency: null,
  iteration: null,
  handoff_note: null,
  finished_without_owner: null,
  description: "",
};
const task = (over: Partial<Task>): Task => ({ ...base, ...over });

test("waiting says who must act: you when no one is named", () => {
  const all: Task[] = [];
  assert.equal(stopText(task({ state: "waiting", waiting_on: "approve the card" }), all, t), "Waiting for you: approve the card");
  assert.equal(
    stopText(task({ state: "waiting", waiting_on: "reply on licence", who: "Ann" }), all, t),
    "Waiting for Ann: reply on licence",
  );
  assert.equal(stopText(task({ state: "in_progress" }), all, t), null);
});

test("blocked names each blocker with its state now; a dropped one is flagged, never done", () => {
  const blocker = task({ id: "A01-17", state: "in_progress" });
  const me = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.equal(stopText(me, [blocker, me], t), "Blocked by A01-17 (In progress)");
  assert.equal(allBlockersDone(me, [blocker, me]), false);

  const done = { ...blocker, state: "done" as const };
  assert.equal(stopText(me, [done, me], t), "Blocked by A01-17 (Done) — all done; unblock it?");
  assert.equal(allBlockersDone(me, [done, me]), true);
  assert.equal(me.state, "blocked", "the app only tells; it moves nothing");

  const dropped = { ...blocker, state: "dropped" as const };
  assert.equal(stopText(me, [dropped, me], t), "Blocked by A01-17 (dropped — still blocked?)");
  assert.equal(allBlockersDone(me, [dropped, me]), false);
  assert.deepEqual(blockers(task({ state: "blocked", blocked_by: ["A09-09"] }), []), [{ id: "A09-09", state: null }]);
});

test("the state filter keeps waiting and blocked apart", () => {
  const waiting = task({ state: "waiting", waiting_on: "x" });
  const blocked = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.equal(keepState("waiting", waiting), true);
  assert.equal(keepState("waiting", blocked), false);
  assert.equal(keepState("blocked", blocked), true);
  assert.equal(keepState("all", blocked), true);
  assert.equal(readFilter("blocked"), "blocked");
  assert.equal(readFilter("nonsense"), "all");
  assert.equal(readFilter(null), "all");
});
