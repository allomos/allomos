import { test } from "node:test";
import assert from "node:assert/strict";
import { FOLLOW_IDLE_MS, FOLLOW_MS, MONITOR_IDLE_MS, MONITOR_MS, inUse, paceMs } from "../src/paceFlow.ts";

test("a window shown with the focus is in use; hidden or without focus it is not", () => {
  assert.equal(inUse({ hidden: false, hasFocus: () => true }), true);
  assert.equal(inUse({ hidden: false, hasFocus: () => false }), false);
  assert.equal(inUse({ hidden: true, hasFocus: () => true }), false);
});

test("checks run at their usual pace in use and more slowly in the background", () => {
  assert.equal(paceMs(true, FOLLOW_MS, FOLLOW_IDLE_MS), 1000);
  assert.equal(paceMs(false, FOLLOW_MS, FOLLOW_IDLE_MS), 5000);
  assert.equal(paceMs(true, MONITOR_MS, MONITOR_IDLE_MS), 2000);
  assert.equal(paceMs(false, MONITOR_MS, MONITOR_IDLE_MS), 10_000);
});

test("the background pace is never quicker than the pace in use", () => {
  assert.equal(paceMs(false, 4000, 1000), 4000);
});
