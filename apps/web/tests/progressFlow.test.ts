// Tests for the progress popup (Task-A05-08). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { QUIET_AFTER_S, agentTask, arrange, fraction, progressState, stillWaiting } from "../src/progressFlow.ts";
import type { Progress, Task } from "../src/types.ts";

const report = (id: string, more: Partial<Progress> = {}): Progress => ({ id, title: "Update the taskbag", started: 100, updated: 1000, ...more });

test("a report is working, finished, or quiet after 10 minutes without word", () => {
  assert.equal(progressState(report("a"), 1000 + 5), "working");
  assert.equal(progressState(report("a"), 1000 + QUIET_AFTER_S), "quiet");
  assert.equal(progressState(report("a", { finished: "3 cards moved in" }), 1000 + QUIET_AFTER_S * 9), "finished");
});

test("an agent waiting for the person's answer is never quiet; finished still wins (Task-A03-aster771)", () => {
  const asking = report("a", { waiting: "Which data source?", step: "Planning" });
  assert.equal(progressState(asking, 1000 + 5), "waiting");
  assert.equal(progressState(asking, 1000 + QUIET_AFTER_S * 6), "waiting");
  assert.equal(progressState({ ...asking, finished: "Built" }, 1000 + QUIET_AFTER_S), "finished");
});

test("the popup finds the agent's window from the task it holds with a recorded session", () => {
  const task = (id: string, more: Partial<Task>): Task => ({ id, title: id, area: "A01", state: "in_progress", holder: null, waiting_on: null, next_dependency: null, iteration: null, handoff_note: null, without_human_check: null, human_check_steps: null, description: "", ...more }) as Task;
  const tasks = [
    task("A01-1", { holder: "codex/other", session: { agent: "codex", id: "s1" } as Task["session"] }),
    task("A01-2", { holder: "claude/x" }),
    task("A01-3", { holder: "claude/x", session: { agent: "claude-code", id: "s2", kind: "desktop", app_session_id: "local_1" } as Task["session"] }),
  ];
  assert.deepEqual(agentTask(report("a", { from: "claude/x" }), tasks), { id: "A01-3", place: "app" });
  assert.deepEqual(agentTask(report("a", { from: "codex/other" }), tasks), { id: "A01-1", place: "terminal" });
  assert.equal(agentTask(report("a", { from: "nobody" }), tasks), null);
  assert.equal(agentTask(report("a"), tasks), null);
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

test("a quiet report the person closed stays hidden until its agent reports again", () => {
  const quiet = report("a", { updated: 1000 });
  const closed = new Map([["a", 1000]]);
  assert.equal(arrange([quiet, report("b")], new Set(), closed).popup?.id, "b");
  assert.deepEqual(arrange([quiet], new Set(["a"]), closed), { popup: null, chips: [] });
  // The agent reports again (a step, or that it finished): the popup comes back.
  assert.equal(arrange([report("a", { updated: 1300 })], new Set(), closed).popup?.id, "a");
});
