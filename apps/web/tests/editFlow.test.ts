// Tests for changing a task in the task pane (Task-A01-18). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  EMPTY_NEW_ITERATION,
  NEW_ITERATION,
  SETTABLE_STATES,
  backlogAllowed,
  backlogBlocked,
  stateDisabled,
  blockerChoices,
  draftChanged,
  iterationChoices,
  iterationSavable,
  startDraft,
  stateChoices,
  toEdit,
  changedVersion,
  toPaneIteration,
  toggleBlocker,
} from "../src/editFlow.ts";
import type { Iteration, Task } from "../src/types.ts";

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
  without_human_check: null,
  human_check_steps: null,
  description: "Why.\n",
};

test("only the states alm can set are offered, after the task's own", () => {
  assert.deepEqual(stateChoices(task), ["in_progress", ...SETTABLE_STATES]);
  assert.deepEqual(stateChoices({ ...task, state: "ready" }), SETTABLE_STATES);
});

test("back to backlog: offered for an unheld ready, waiting or blocked task, explained when greyed out (Task-A01-lotus143)", () => {
  const unheld: Task = { ...task, state: "ready", holder: null };
  assert.equal(backlogAllowed(unheld), true);
  assert.equal(stateDisabled(unheld, "backlog"), false);
  assert.equal(backlogBlocked(unheld), null);
  for (const state of ["waiting", "blocked"] as const) assert.equal(backlogAllowed({ ...unheld, state }), true);
  // Held: greyed out, saying who holds it.
  const held: Task = { ...task, state: "waiting", holder: "claude/x" };
  assert.equal(stateDisabled(held, "backlog"), true);
  assert.deepEqual(backlogBlocked(held), { why: "held", holder: "claude/x" });
  assert.deepEqual(backlogBlocked({ ...task, holder: "codex/y" }), { why: "held", holder: "codex/y" });
  // Finished: greyed out, with the reopen hint.
  for (const state of ["done", "dropped"] as const) {
    assert.equal(stateDisabled({ ...unheld, state }, "backlog"), true);
    assert.deepEqual(backlogBlocked({ ...unheld, state }), { why: "finished" });
  }
  // Already backlog: its own choice, nothing to explain; in progress is never chosen here.
  const inBacklog: Task = { ...unheld, state: "backlog" };
  assert.equal(stateDisabled(inBacklog, "backlog"), false);
  assert.equal(backlogBlocked(inBacklog), null);
  assert.equal(stateDisabled(unheld, "in_progress"), true);
  assert.equal(SETTABLE_STATES[0], "backlog");
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
  assert.deepEqual(toEdit("title", d(" New ")), { field: "title", title: "New", expected_version: null });
  assert.deepEqual(toEdit("description", d("Text\n")), { field: "description", text: "Text\n", expected_version: null });
});

test("a title or description carries the version it started from; a refusal names the version now (Task-A01-57)", () => {
  const d = (value: string) => ({ value, waitingOn: "", who: "", blockedBy: [] });
  assert.deepEqual(toEdit("title", d("New"), "0123456789abcdef"), { field: "title", title: "New", expected_version: "0123456789abcdef" });
  assert.deepEqual(toEdit("description", d("Text"), "0123456789abcdef"), { field: "description", text: "Text", expected_version: "0123456789abcdef" });
  // Other fields carry no version.
  assert.deepEqual(toEdit("label", d("implemented"), "0123456789abcdef"), { field: "label", label: "implemented" });
  assert.equal(changedVersion({ code: "task_changed", params: { id: "A01-01", version: "fedcba9876543210" } }), "fedcba9876543210");
  assert.equal(changedVersion({ code: "task_held", params: { id: "A01-01", holder: "x" } }), null);
  assert.equal(changedVersion("boom"), null);
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

// The iteration drop-down (Task-A01-39).
const iterations: Iteration[] = [
  { id: "I01", name: "First", order: 1, release: "v0.1" },
  { id: "I02", name: "Second", order: 2 },
];

test("the iteration list: New… first, No iteration second, then every iteration by ID and name", () => {
  const choices = iterationChoices(task, iterations);
  assert.deepEqual(
    choices.map((c) => c.value),
    [NEW_ITERATION, "", "I01", "I02"],
  );
  assert.deepEqual(choices[2], { kind: "iteration", value: "I01", id: "I01", name: "First", release: "v0.1" });
  // The current one is selected when the list opens: the draft starts from it.
  assert.equal(startDraft(task, "iteration").value, "I01");
  assert.equal(startDraft({ ...task, iteration: null }, "iteration").value, "");
  // An iteration the taskbag does not list stays offered, after No iteration.
  assert.deepEqual(
    iterationChoices({ ...task, iteration: "I09" }, iterations).map((c) => c.value),
    [NEW_ITERATION, "", "I09", "I01", "I02"],
  );
  // "New…" can never be taken for an iteration ID.
  assert.ok(!/^[A-Za-z0-9_-]+$/.test(NEW_ITERATION));
});

test("choosing an iteration plans the task, as alm plan; New… needs a name", () => {
  const d = (value: string) => ({ value, waitingOn: "", who: "", blockedBy: [] as string[] });
  assert.equal(iterationSavable(task, d("I01"), EMPTY_NEW_ITERATION), false);
  assert.equal(iterationSavable(task, d("I02"), EMPTY_NEW_ITERATION), true);
  assert.equal(iterationSavable(task, d(""), EMPTY_NEW_ITERATION), true);
  assert.deepEqual(toEdit("iteration", d("I02")), { field: "iteration", iteration: "I02" });
  assert.deepEqual(toEdit("iteration", d("")), { field: "iteration", iteration: null });
  assert.equal(iterationSavable(task, d(NEW_ITERATION), EMPTY_NEW_ITERATION), false);
  assert.equal(iterationSavable(task, d(NEW_ITERATION), { name: " ", start: "", end: "" }), false);
  assert.equal(iterationSavable(task, d(NEW_ITERATION), { name: "Third", start: "", end: "" }), true);
  assert.deepEqual(toPaneIteration({ name: " Third ", start: "2026-11-01", end: " " }), {
    id: null,
    name: "Third",
    release: null,
    start: "2026-11-01",
    end: null,
  });
});
