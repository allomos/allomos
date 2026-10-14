// Tests for how a stopped task is worded, and the state filter (decision 0016, Task-A01-31).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { allBlockersDone, blockers, filterName, hiddenByFilter, hiddenText, humanCheckNote, keepState, readFilter, stopText, toCheck } from "../src/stopFlow.ts";
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
  without_human_check: null,
  human_check_steps: null,
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

test("not human-checked: a done task with the note, under either name (Task-A03-18)", () => {
  const finished = task({ state: "done", without_human_check: "loop run", human_check_steps: "1. Open it." });
  assert.equal(toCheck(finished), true);
  assert.equal(humanCheckNote(finished), "loop run");
  assert.equal(toCheck(task({ state: "done" })), false, "finished the usual way");
  assert.equal(toCheck(task({ state: "in_progress", without_human_check: "x" })), false, "not finished");
  const old = task({ state: "done", finished_without_owner: "card run" });
  assert.equal(toCheck(old), true, "a taskbag being migrated");
  assert.equal(humanCheckNote(old), "card run");
});

test("the Needs a human filter keeps the tasks to check and to answer, not backlog", () => {
  assert.equal(keepState("human", task({ state: "done", without_human_check: "loop run" })), true);
  assert.equal(keepState("human", task({ state: "waiting", waiting_on: "approve" })), true);
  assert.equal(keepState("human", task({ state: "backlog" })), false);
  assert.equal(keepState("human", task({ state: "done" })), false);
  assert.equal(keepState("human", task({ state: "blocked", blocked_by: ["A01-17"] })), false);
  assert.equal(readFilter("human"), "human");
});

// Task-A01-42: what the state filter hides.
test("the filter's hidden tasks are counted by state, in the filter's order", () => {
  const tasks = [
    task({ id: "a", state: "done" }),
    task({ id: "b", state: "backlog" }),
    task({ id: "c", state: "waiting" }),
    task({ id: "d", state: "done" }),
    task({ id: "e", state: "ready" }),
  ];
  const hidden = hiddenByFilter("backlog", tasks);
  assert.deepEqual(hidden, {
    count: 4,
    states: [
      { state: "ready", count: 1 },
      { state: "waiting", count: 1 },
      { state: "done", count: 2 },
    ],
  });
  assert.equal(hiddenText(hidden, t), "4 more hidden by the filter: 1 Ready, 1 Waiting, 2 Done");
  assert.deepEqual(hiddenByFilter("all", tasks), { count: 0, states: [] });
  // "Needs a human" keeps waiting tasks and done ones not yet checked.
  const human = hiddenByFilter("human", [...tasks, task({ id: "f", state: "done", without_human_check: "loop run" })]);
  assert.equal(human.count, 4);
});

test("the filter is named as the Show box words it", () => {
  assert.equal(filterName("backlog", t), "Backlog");
  assert.equal(filterName("human", t), "Needs a human (to check, to answer)");
  assert.equal(filterName("all", t), "All states");
});
