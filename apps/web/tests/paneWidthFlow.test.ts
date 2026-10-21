// Tests for the task pane's width (Task-A01-56; kept as a share, Task-A01-61). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  MIN_LIST,
  MIN_PANE,
  clampPane,
  dragWidth,
  formatPaneRatio,
  maxPane,
  paneRatio,
  parsePaneRatio,
  ratioWidth,
} from "../src/paneWidthFlow.ts";

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

test("a chosen width is kept as its share of the box", () => {
  assert.equal(paneRatio(540, 1200), 0.45);
  assert.equal(paneRatio(0, 1200), 0);
  assert.equal(paneRatio(1500, 1200), 1);
  assert.equal(paneRatio(500, 0), 0);
  assert.equal(formatPaneRatio(520 / 1200), "0.4333");
});

test("the share keeps its proportion when the box changes size, within the limits", () => {
  assert.equal(ratioWidth(0.45, 1200, 14), 540);
  assert.equal(ratioWidth(0.45, 1600, 14), 720);
  // A small window: the pane keeps its minimum; a wide share still leaves the list its minimum.
  assert.equal(ratioWidth(0.2, 1000, 14), MIN_PANE);
  assert.equal(ratioWidth(0.9, 1200, 14), 1200 - 14 - MIN_LIST);
});

test("a stored share is read back; anything else, including an older pixel width, means the default", () => {
  assert.equal(parsePaneRatio("0.45"), 0.45);
  assert.equal(parsePaneRatio(formatPaneRatio(0.4333333)), 0.4333);
  assert.equal(parsePaneRatio(null), null);
  assert.equal(parsePaneRatio(""), null);
  assert.equal(parsePaneRatio("wide"), null);
  assert.equal(parsePaneRatio("0"), null);
  assert.equal(parsePaneRatio("1"), null);
  assert.equal(parsePaneRatio("480"), null);
  assert.equal(parsePaneRatio("-0.3"), null);
});
