// Tests for number labels on elements (Task-A03-12). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { assignLabels, isLabelShortcut, labelName } from "../src/labelFlow.ts";

const keys = { key: "S", code: "KeyS", ctrlKey: true, shiftKey: true, altKey: false, metaKey: false };

test("Ctrl+Shift+S toggles labels, in either case; other combinations don't", () => {
  assert.equal(isLabelShortcut(keys), true);
  assert.equal(isLabelShortcut({ ...keys, key: "s" }), true);
  assert.equal(isLabelShortcut({ ...keys, code: undefined }), true);
  assert.equal(isLabelShortcut({ ...keys, shiftKey: false }), false);
  assert.equal(isLabelShortcut({ ...keys, ctrlKey: false }), false);
  assert.equal(isLabelShortcut({ ...keys, metaKey: true }), false);
  assert.equal(isLabelShortcut({ ...keys, altKey: true }), false);
  assert.equal(isLabelShortcut({ ...keys, key: "D", code: "KeyD" }), false);
});

test("labels are a letter then a digit, and stay unique past 260", () => {
  assert.deepEqual([0, 9, 10, 259].map(labelName), ["A0", "A9", "B0", "Z9"]);
  assert.deepEqual([260, 270].map(labelName), ["AA0", "BB0"]);
  const names = Array.from({ length: 1000 }, (_, i) => labelName(i));
  assert.equal(new Set(names).size, names.length);
});

test("labels follow reading order: top to bottom, then left to right", () => {
  const box = (top: number, left: number) => ({ top, left, width: 20, height: 10 });
  // Given out of order; the second row's items are a few pixels apart vertically.
  assert.deepEqual(assignLabels([box(50, 200), box(0, 100), box(52, 10), box(0, 0)]), ["A3", "A1", "A2", "A0"]);
});

test("an element in exactly the same place as a labelled one gets no label of its own", () => {
  const button = { top: 10, left: 10, width: 80, height: 24 };
  const text = { top: 10.2, left: 9.8, width: 80, height: 24 };
  const other = { top: 10, left: 100, width: 80, height: 24 };
  assert.deepEqual(assignLabels([button, text, other]), ["A0", null, "A1"]);
});
