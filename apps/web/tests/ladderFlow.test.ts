// Tests for editing the progress ladder in the taskbag's settings (Task-A05-yeti951).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { addStep, ladderChanged, ladderProblem, moveStep, removeStep, renameStep, startLadder } from "../src/ladderFlow.ts";

const ladder = ["implemented", "locally verified", "human verified"];

test("renames keep where each step came from", () => {
  const steps = renameStep(startLadder(ladder), 0, "built");
  assert.deepEqual(steps[0], { from: "implemented", to: "built" });
  assert.equal(ladderChanged(ladder, steps), true);
  assert.equal(ladderChanged(ladder, startLadder(ladder)), false);
});

test("add, remove and move", () => {
  let steps = addStep(startLadder(ladder));
  assert.deepEqual(steps[3], { from: null, to: "" });
  assert.equal(ladderProblem(steps), "empty");
  steps = renameStep(steps, 3, "reviewed");
  steps = moveStep(steps, 3, -1);
  assert.deepEqual(
    steps.map((s) => s.to),
    ["implemented", "locally verified", "reviewed", "human verified"],
  );
  assert.equal(moveStep(steps, 0, -1), steps);
  assert.equal(moveStep(steps, 3, 1), steps);
  steps = removeStep(steps, 1);
  assert.deepEqual(
    steps.map((s) => s.from),
    ["implemented", null, "human verified"],
  );
  assert.equal(ladderProblem(steps), null);
});

test("what keeps Save off", () => {
  assert.equal(ladderProblem([]), "none");
  assert.equal(ladderProblem(renameStep(startLadder(ladder), 1, "implemented")), "duplicate");
  assert.equal(ladderProblem(renameStep(startLadder(ladder), 1, "  ")), "empty");
});
