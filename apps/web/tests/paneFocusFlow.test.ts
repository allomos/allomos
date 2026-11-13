// Tests for what the task pane shows first, by the task's state (Task-A01-berry927).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { paneFocus } from "../src/paneFocusFlow.ts";
import type { Task } from "../src/types.ts";

const task = (over: Partial<Task>): Task => ({ id: "A01-x", title: "x", area: "A01", state: "ready", description: "", ...over }) as Task;

test("a done task to check shows its check steps first", () => {
  assert.deepEqual(paneFocus(task({ state: "done" }), true, false), ["check_steps"]);
});

test("a checked done task shows its description first", () => {
  assert.deepEqual(paneFocus(task({ state: "done" }), false, false), ["description"]);
});

test("a waiting task shows the question, then the handoff note", () => {
  assert.deepEqual(paneFocus(task({ state: "waiting", waiting_on: "q", handoff_note: "stopped here" }), false, false), ["question", "handoff"]);
});

test("a blocked task shows its blockers first", () => {
  assert.deepEqual(paneFocus(task({ state: "blocked" }), false, false), ["blockers"]);
});

test("an in-progress task shows its handoff note; a lost one also its description", () => {
  const t = task({ state: "in_progress", holder: "a", handoff_note: "at step 2" });
  assert.deepEqual(paneFocus(t, false, false), ["handoff"]);
  assert.deepEqual(paneFocus(t, false, true), ["handoff", "description"]);
});

test("a ready task shows its description, after a handoff note when it has one", () => {
  assert.deepEqual(paneFocus(task({ state: "ready" }), false, false), ["description"]);
  assert.deepEqual(paneFocus(task({ state: "ready", handoff_note: "released: next is X" }), false, false), ["handoff", "description"]);
});

test("an empty handoff note is not shown first", () => {
  assert.deepEqual(paneFocus(task({ state: "waiting", handoff_note: "  " }), false, false), ["question"]);
});
