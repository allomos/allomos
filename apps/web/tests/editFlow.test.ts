// Tests for changing a task in the task pane (Task-A01-18). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { SETTABLE_STATES, draftChanged, startDraft, stateChoices, toEdit } from "../src/editFlow.ts";
import type { Task } from "../src/types.ts";

const task: Task = {
  id: "A01-01",
  title: "Do it",
  area: "A01",
  state: "in_progress",
  label: null,
  holder: null,
  waiting_on: null,
  next_dependency: "the owner",
  iteration: "I01",
  handoff_note: null,
  finished_without_owner: null,
  description: "Why.\n",
};

test("only the states alm can set are offered, after the task's own", () => {
  assert.deepEqual(stateChoices(task), ["in_progress", ...SETTABLE_STATES]);
  assert.deepEqual(stateChoices({ ...task, state: "ready" }), SETTABLE_STATES);
});

test("an editor starts from the task's value", () => {
  assert.deepEqual(startDraft(task, "state"), { value: "in_progress", waitingOn: "" });
  assert.equal(startDraft(task, "label").value, "");
  assert.equal(startDraft(task, "next_dependency").value, "the owner");
  assert.equal(startDraft(task, "description").value, "Why.\n");
});

test("Save is offered only for a change", () => {
  assert.equal(draftChanged(task, "title", { value: " Do it ", waitingOn: "" }), false);
  assert.equal(draftChanged(task, "title", { value: "Do it now", waitingOn: "" }), true);
  assert.equal(draftChanged(task, "description", { value: "Why.", waitingOn: "" }), false);
  assert.equal(draftChanged(task, "state", { value: "in_progress", waitingOn: "" }), false);
  assert.equal(draftChanged(task, "state", { value: "waiting", waitingOn: "" }), true);
  const waiting = { ...task, state: "waiting" as const, waiting_on: "x" };
  assert.equal(draftChanged(waiting, "state", { value: "waiting", waitingOn: "x" }), false);
  assert.equal(draftChanged(waiting, "state", { value: "waiting", waitingOn: "y" }), true);
});

test("each field becomes the change Rust reads", () => {
  const d = (value: string, waitingOn = "") => ({ value, waitingOn });
  assert.deepEqual(toEdit("state", d("waiting", " the owner ")), { field: "state", state: "waiting", waiting_on: "the owner" });
  assert.deepEqual(toEdit("state", d("done")), { field: "state", state: "done" });
  assert.deepEqual(toEdit("label", d("implemented")), { field: "label", label: "implemented" });
  assert.deepEqual(toEdit("waiting_on", d(" x ")), { field: "waiting_on", text: "x" });
  assert.deepEqual(toEdit("next_dependency", d(" ")), { field: "next_dependency", text: null });
  assert.deepEqual(toEdit("handoff_note", d("Read it")), { field: "handoff_note", text: "Read it" });
  assert.deepEqual(toEdit("iteration", d("")), { field: "iteration", iteration: null });
  assert.deepEqual(toEdit("title", d(" New ")), { field: "title", title: "New" });
  assert.deepEqual(toEdit("description", d("Text\n")), { field: "description", text: "Text\n" });
});
