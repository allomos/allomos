// Tests for editing the project's own labels in the taskbag's settings (Task-A05-yeti951,
// Task-A01-petal376).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { BUILT_IN_NAMES } from "../src/stopFlow.ts";
import { addStep, ladderChanged, ladderProblem, removeStep, renameStep, startLadder } from "../src/ladderFlow.ts";

const ladder = ["implemented", "shipped", "reviewed by legal"];

test("renames keep where each step came from", () => {
  const steps = renameStep(startLadder(ladder), 0, "built");
  assert.deepEqual(steps[0], { from: "implemented", to: "built" });
  assert.equal(ladderChanged(ladder, steps), true);
  assert.equal(ladderChanged(ladder, startLadder(ladder)), false);
});

test("add and remove", () => {
  let steps = addStep(startLadder(ladder));
  assert.deepEqual(steps[3], { from: null, to: "" });
  assert.equal(ladderProblem(steps), "empty");
  steps = renameStep(steps, 3, "reviewed");
  assert.deepEqual(
    steps.map((s) => s.to),
    ["implemented", "shipped", "reviewed by legal", "reviewed"],
  );
  steps = removeStep(steps, 1);
  assert.deepEqual(
    steps.map((s) => s.from),
    ["implemented", "reviewed by legal", null],
  );
  assert.equal(ladderProblem(steps), null);
});

test("what keeps Save off", () => {
  assert.equal(ladderProblem([]), null, "no label of its own is fine: the built-in labels stay");
  assert.equal(ladderProblem(renameStep(startLadder(ladder), 1, "locally verified by agent"), BUILT_IN_NAMES), "duplicate", "a built-in label's name is taken");
  assert.equal(ladderProblem(renameStep(startLadder(ladder), 1, "implemented")), "duplicate");
  assert.equal(ladderProblem(renameStep(startLadder(ladder), 1, "  ")), "empty");
});
