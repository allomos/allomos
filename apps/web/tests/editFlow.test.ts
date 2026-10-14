// Tests for changing a task in the task pane (Task-A01-18). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { SETTABLE_STATES, blockerChoices, draftChanged, startDraft, stateChoices, toEdit, toggleBlocker } from "../src/editFlow.ts";
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
  assert.deepEqual(startDraft(task, "state"), { value: "in_progress", waitingOn: "", who: "", blockedBy: [] });
  assert.equal(startDraft(task, "label").value, "");
  assert.equal(startDraft(task, "next_dependency").value, "the owner");
  assert.equal(startDraft(task, "description").value, "Why.\n");
});

const e = { who: "", blockedBy: [] as string[] };

test("Save is offered only for a change", () => {
  assert.equal(draftChanged(task, "title", { value: " Do it ", waitingOn: "", ...e }), false);
  assert.equal(draftChanged(task, "title", { value: "Do it now", waitingOn: "", ...e }), true);
  assert.equal(draftChanged(task, "description", { value: "Why.", waitingOn: "", ...e }), false);
  assert.equal(draftChanged(task, "state", { value: "in_progress", waitingOn: "", ...e }), false);
  assert.equal(draftChanged(task, "state", { value: "waiting", waitingOn: "", ...e }), true);
  const waiting = { ...task, state: "waiting" as const, waiting_on: "x" };
  assert.equal(draftChanged(waiting, "state", { value: "waiting", waitingOn: "x", ...e }), false);
  assert.equal(draftChanged(waiting, "state", { value: "waiting", waitingOn: "y", ...e }), true);
});

test("each field becomes the change Rust reads", () => {
  const d = (value: string, waitingOn = "", who = "", blockedBy: string[] = []) => ({ value, waitingOn, who, blockedBy });
  assert.deepEqual(toEdit("state", d("waiting", " approve the card ")), {
    field: "state",
    state: "waiting",
    waiting_on: "approve the card",
    who: null,
  });
  assert.deepEqual(toEdit("state", d("waiting", "reply", " Ann ")), { field: "state", state: "waiting", waiting_on: "reply", who: "Ann" });
  assert.deepEqual(toEdit("state", d("blocked", "", "", ["A01-17"])), { field: "state", state: "blocked", blocked_by: ["A01-17"] });
  assert.deepEqual(toEdit("blocked_by", d("", "", "", ["A01-17", "A01-20"])), { field: "blocked_by", ids: ["A01-17", "A01-20"] });
  assert.deepEqual(toEdit("state", d("done")), { field: "state", state: "done" });
  assert.deepEqual(toEdit("label", d("implemented")), { field: "label", label: "implemented" });
  assert.deepEqual(toEdit("waiting_on", d(" x ")), { field: "waiting_on", text: "x" });
  assert.deepEqual(toEdit("next_dependency", d(" ")), { field: "next_dependency", text: null });
  assert.deepEqual(toEdit("handoff_note", d("Read it")), { field: "handoff_note", text: "Read it" });
  assert.deepEqual(toEdit("iteration", d("")), { field: "iteration", iteration: null });
  assert.deepEqual(toEdit("title", d(" New ")), { field: "title", title: "New" });
  assert.deepEqual(toEdit("description", d("Text\n")), { field: "description", text: "Text\n" });
});

test("blocked is a state the app sets, with the tasks that must finish first (decision 0016)", () => {
  assert.ok(SETTABLE_STATES.includes("blocked"));
  const blocked: Task = { ...task, state: "blocked", blocked_by: ["A01-17"] };
  assert.deepEqual(startDraft(blocked, "blocked_by").blockedBy, ["A01-17"]);
  const d = startDraft(blocked, "state");
  assert.equal(draftChanged(blocked, "state", d), false);
  assert.equal(draftChanged(blocked, "state", toggleBlocker(d, "A01-20")), true);
  assert.equal(draftChanged(blocked, "blocked_by", { ...d, blockedBy: [] }), false, "an empty list is not saved");
  assert.deepEqual(toggleBlocker(toggleBlocker(d, "A01-02"), "A01-17").blockedBy, ["A01-02"]);
  // Waiting with a new who counts as a change.
  const waiting: Task = { ...task, state: "waiting", waiting_on: "x" };
  assert.equal(draftChanged(waiting, "state", { ...startDraft(waiting, "state"), who: "Ann" }), true);
  // The picker offers every other task, in ID order.
  const others = [{ ...task, id: "A02-01" }, task, { ...task, id: "A01-02" }];
  assert.deepEqual(
    blockerChoices(task, others).map((x) => x.id),
    ["A01-02", "A02-01"],
  );
});
