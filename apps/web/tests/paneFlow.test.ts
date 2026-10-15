// Tests for which task the pane shows after a click (Task-A01-46). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { afterClick } from "../src/paneFlow.ts";

test("clicking the task already shown keeps the pane open on it", () => {
  assert.equal(afterClick("A01-46", "A01-46"), "A01-46");
});

test("clicking another task switches the pane to it", () => {
  assert.equal(afterClick("A01-46", "A01-16"), "A01-16");
});

test("clicking a task with the pane closed opens it", () => {
  assert.equal(afterClick(null, "A01-16"), "A01-16");
});
