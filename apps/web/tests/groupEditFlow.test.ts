// Tests for renaming, changing and deleting areas and iterations in the list (Task-A01-29).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canSaveGroup, startGroupEdit, toIterationChange } from "../src/groupEditFlow.ts";
import type { Area, Iteration } from "../src/types.ts";

const area: Area = { id: "A01", name: "Tracking", order: 1 };
const iteration: Iteration = { id: "I02", name: "Two", order: 2, release: "v0.2", start: "2026-10-01", end: "2026-10-10" };

test("the form starts from the group as it is", () => {
  assert.deepEqual(startGroupEdit(area), { name: "Tracking", release: "", start: "", end: "", state: "planned" });
  assert.deepEqual(startGroupEdit(iteration), { name: "Two", release: "v0.2", start: "2026-10-01", end: "2026-10-10", state: "planned" });
  assert.deepEqual(startGroupEdit({ id: "I03", name: "Three", order: 3, state: "active" }), { name: "Three", release: "", start: "", end: "", state: "active" });
});

test("save is offered only with a name and a difference", () => {
  const a = startGroupEdit(area);
  assert.equal(canSaveGroup("area", a, area), false);
  assert.equal(canSaveGroup("area", { ...a, name: " Tracking " }, area), false);
  assert.equal(canSaveGroup("area", { ...a, name: "Tracking work" }, area), true);
  assert.equal(canSaveGroup("area", { ...a, name: "  " }, area), false);
  const i = startGroupEdit(iteration);
  assert.equal(canSaveGroup("iteration", i, iteration), false);
  assert.equal(canSaveGroup("iteration", { ...i, end: "2026-10-20" }, iteration), true);
  assert.equal(canSaveGroup("iteration", { ...i, release: "" }, iteration), true);
  assert.equal(canSaveGroup("iteration", { ...i, name: "", end: "2026-10-20" }, iteration), false);
  // The state alone is a change (Task-A01-root343).
  assert.equal(canSaveGroup("iteration", { ...i, state: "active" }, iteration), true);
});

test("every iteration field is sent; an empty one clears it", () => {
  assert.deepEqual(toIterationChange({ name: " Two ", release: "", start: "2026-10-01", end: " 2026-10-20 ", state: "closed" }), {
    name: "Two",
    release: "",
    start: "2026-10-01",
    end: "2026-10-20",
    state: "closed",
  });
});
