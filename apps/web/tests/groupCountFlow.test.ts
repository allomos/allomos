// Tests for the count in an area's or iteration's header (Task-A01-timber110). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { groupCount } from "../src/groupCountFlow.ts";

test("dropped tasks stay out of the total: 25 done and 1 dropped reads 25 of 25", () => {
  const states = [...Array(25).fill("done"), "dropped"];
  assert.deepEqual(groupCount(states), { done: 25, total: 25, pct: 100 });
});

test("tasks still open count toward the total", () => {
  assert.deepEqual(groupCount(["done", "ready", "in_progress", "dropped"]), { done: 1, total: 3, pct: 33 });
});

test("an empty or all-dropped group reads 0 of 0 at 0%", () => {
  assert.deepEqual(groupCount([]), { done: 0, total: 0, pct: 0 });
  assert.deepEqual(groupCount(["dropped"]), { done: 0, total: 0, pct: 0 });
});
