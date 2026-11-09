// Tests for a task's difficulty in the list and pane (Task-A01-actor813).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { difficultyOf, difficultyText } from "../src/difficultyFlow.ts";
import { draftChanged, startDraft, toEdit } from "../src/editFlow.ts";
import type { Task } from "../src/types.ts";

const task = (difficulty?: number | null) => ({ id: "A01-01", title: "t", state: "ready", difficulty }) as unknown as Task;
const t = (key: string, params?: Record<string, string | number>) => (params ? `${key}:${JSON.stringify(params)}` : key);

test("a task without a difficulty counts as 3, marked as the default", () => {
  assert.deepEqual(difficultyOf(task()), { level: 3, isDefault: true });
  assert.deepEqual(difficultyOf(task(null)), { level: 3, isDefault: true });
  assert.deepEqual(difficultyOf(task(9)), { level: 3, isDefault: true });
  assert.deepEqual(difficultyOf(task(5)), { level: 5, isDefault: false });
  assert.equal(difficultyText(task(4), t), "difficulty.level.4");
  assert.equal(difficultyText(task(), t), 'difficulty.default:{"level":"difficulty.level.3"}');
});

test("the pane changes the difficulty, or takes it off", () => {
  const d = startDraft(task(2), "difficulty");
  assert.equal(d.value, "2");
  assert.equal(draftChanged(task(2), "difficulty", d), false);
  assert.equal(startDraft(task(), "difficulty").value, "");
  assert.deepEqual(toEdit("difficulty", { ...d, value: "4" }), { field: "difficulty", difficulty: 4 });
  assert.deepEqual(toEdit("difficulty", { ...d, value: "" }), { field: "difficulty", difficulty: null });
  assert.equal(draftChanged(task(2), "difficulty", { ...d, value: "4" }), true);
});
