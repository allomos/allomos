import { test } from "node:test";
import assert from "node:assert/strict";
import { followPace, inUse, paceMs } from "../src/paceFlow.ts";

test("a window shown with the focus is in use; hidden or without focus it is not", () => {
  assert.equal(inUse({ hidden: false, hasFocus: () => true }), true);
  assert.equal(inUse({ hidden: false, hasFocus: () => false }), false);
  assert.equal(inUse({ hidden: true, hasFocus: () => true }), false);
});

test("told of changes, the window checks the taskbag itself only once a minute, in use or not", () => {
  const p = followPace(true);
  assert.equal(paceMs(true, p.awakeMs, p.idleMs), 60_000);
  assert.equal(paceMs(false, p.awakeMs, p.idleMs), 60_000);
});

test("not told (a companion page), it checks every second in use and every five seconds otherwise", () => {
  const p = followPace(false);
  assert.equal(paceMs(true, p.awakeMs, p.idleMs), 1000);
  assert.equal(paceMs(false, p.awakeMs, p.idleMs), 5000);
});

test("the background pace is never quicker than the pace in use", () => {
  assert.equal(paceMs(false, 4000, 1000), 4000);
});
