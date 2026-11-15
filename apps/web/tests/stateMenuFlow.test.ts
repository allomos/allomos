// Tests for the task list's state menu (Task-A01-frost005): the same choices and rules as the
// task pane's state menu. Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { stateChoices, stateItems, stateMenuEdit } from "../src/editFlow.ts";
import type { Task } from "../src/types.ts";

const task: Task = {
  id: "A01-01",
  title: "Do it",
  area: "A01",
  state: "ready",
  labels: [],
  holder: null,
  waiting_on: null,
  next_dependency: null,
  iteration: "I01",
  handoff_note: null,
  without_human_check: null,
  human_check_steps: null,
  description: "",
};

test("the list offers the task pane's states, in its order, the task's own ticked", () => {
  const items = stateItems(task);
  assert.deepEqual(items.map((i) => i.state), stateChoices(task));
  assert.deepEqual(items.filter((i) => i.current).map((i) => i.state), ["ready"]);
  assert.ok(items.every((i) => !i.disabled && i.why === null));
});

test("greyed-out choices say why, by the pane's rule (Task-A01-lotus143)", () => {
  const held = stateItems({ ...task, state: "in_progress", holder: "claude/x" });
  assert.equal(held[0].state, "in_progress");
  assert.equal(held[0].current, true);
  assert.equal(held[0].disabled, false);
  const backlog = held.find((i) => i.state === "backlog")!;
  assert.equal(backlog.disabled, true);
  assert.deepEqual(backlog.why, { why: "held", holder: "claude/x" });
  const done = stateItems({ ...task, state: "done" }).find((i) => i.state === "backlog")!;
  assert.deepEqual(done.why, { why: "finished" });
  // In progress is never set from here: a waiting task lists it only as its own state, never.
  assert.equal(stateItems(task).some((i) => i.state === "in_progress"), false);
});

test("choosing a state sends the pane's change; nothing for the own or a greyed-out state", () => {
  assert.deepEqual(stateMenuEdit(task, "done"), { field: "state", state: "done" });
  assert.deepEqual(stateMenuEdit(task, "dropped"), { field: "state", state: "dropped" });
  assert.equal(stateMenuEdit(task, "ready"), null);
  assert.equal(stateMenuEdit({ ...task, holder: "claude/x" }, "backlog"), null);
  assert.equal(stateMenuEdit(task, "in_progress"), null);
});

test("waiting asks what it waits on, as the pane does", () => {
  assert.equal(stateMenuEdit(task, "waiting"), null);
  assert.equal(stateMenuEdit(task, "waiting", "   "), null);
  assert.deepEqual(stateMenuEdit(task, "waiting", " the approver's answer ", ""), {
    field: "state",
    state: "waiting",
    waiting_on: "the approver's answer",
    who: null,
  });
  assert.deepEqual(stateMenuEdit(task, "waiting", "x", " Will "), { field: "state", state: "waiting", waiting_on: "x", who: "Will" });
});
