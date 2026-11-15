// Task-A01-piglet562, decision 0025 § 3: a detail is kept with the task but does not wait for
// review; a visible or principle one, or one with no kind, does.
import { test } from "node:test";
import assert from "node:assert/strict";
import { decisionDetails, decisionsToReview } from "../src/stopFlow.ts";
import type { Task } from "../src/types.ts";

test("only details stay out of review", () => {
  const task = {
    id: "A01-01",
    ai_decisions: [
      { at: "2026-11-15T10:00:00Z", text: "wording", kind: "visible" },
      { at: "2026-11-15T10:01:00Z", text: "by working branch", kind: "principle" },
      { at: "2026-11-15T10:02:00Z", text: "160px high", kind: "detail" },
      { at: "2026-11-15T10:03:00Z", text: "older, no kind" },
    ],
  } as unknown as Task;
  assert.deepEqual(decisionsToReview(task).map((d) => d.text), ["wording", "by working branch", "older, no kind"]);
  assert.deepEqual(decisionDetails(task).map((d) => d.text), ["160px high"]);
  assert.deepEqual(decisionsToReview({ id: "x" } as unknown as Task), []);
});
