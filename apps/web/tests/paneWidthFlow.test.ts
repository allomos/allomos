// Tests for the task pane's width (Task-A01-56). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { MIN_LIST, MIN_PANE, clampPane, dragWidth, maxPane, parsePaneWidth } from "../src/paneWidthFlow.ts";

test("the pane stays between its minimum and what leaves the list its minimum", () => {
  assert.equal(clampPane(100, 1200, 14), MIN_PANE);
  assert.equal(clampPane(500, 1200, 14), 500);
  assert.equal(clampPane(1100, 1200, 14), 1200 - 14 - MIN_LIST);
  assert.equal(maxPane(1200, 14), 866);
});

test("in a narrow window the pane keeps its minimum rather than vanishing", () => {
  assert.equal(maxPane(500, 14), MIN_PANE);
  assert.equal(clampPane(450, 500, 14), MIN_PANE);
});

test("dragging measures from the pointer to the layout's right edge", () => {
  // Layout from x=20 to x=1220: pointer at 700 leaves 520 for the pane.
  assert.equal(dragWidth(700, 1220, 1200, 14), 520);
  assert.equal(dragWidth(1210, 1220, 1200, 14), MIN_PANE);
  assert.equal(dragWidth(0, 1220, 1200, 14), 866);
});

test("a stored width is read back; anything else means the default", () => {
  assert.equal(parsePaneWidth("480"), 480);
  assert.equal(parsePaneWidth("480.6"), 481);
  assert.equal(parsePaneWidth(null), null);
  assert.equal(parsePaneWidth(""), null);
  assert.equal(parsePaneWidth("wide"), null);
  assert.equal(parsePaneWidth("120"), null);
});
