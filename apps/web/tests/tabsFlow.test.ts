// Tests for taskbags as tabs (Task-A01-marble320): how a window's tabs start, move and close, and
// the dot for a taskbag that needs the person.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { needsPerson } from "../src/stopFlow.ts";
import { frontAfterClose, insertAt, moveTab, placeAt, shiftFor, startTabs, tabShowing, tearsOff, type TabEntry } from "../src/tabsFlow.ts";
import type { Task } from "../src/types.ts";

test("a window starts with the tabs it was given, or one tab", () => {
  // Restored tabs, the start page among them.
  assert.deepEqual(
    startTabs({ tabs: [{ root: "/a/.allomos/taskbag", how: "restore" }, { root: null, how: "restore" }], active: 1, startup: "reopen" }),
    [{ how: "restore", root: "/a/.allomos/taskbag" }, { how: "blank" }],
  );
  // The first window with nothing restored decides at start-up; another window gets the start page.
  assert.deepEqual(startTabs({ tabs: [], active: 0, startup: "last" }), [{ how: "startup", startWith: "last" }]);
  assert.deepEqual(startTabs({ tabs: [], active: 0, startup: null }), [{ how: "blank" }]);
  // A tab dragged in from another window, and Open Recent's folder.
  assert.deepEqual(startTabs({ tabs: [{ root: "/b", how: "moved" }], active: 0, startup: null }), [{ how: "moved", root: "/b" }]);
  assert.deepEqual(startTabs({ tabs: [{ root: "/c", how: "open" }], active: 0, startup: null }), [{ how: "open", root: "/c" }]);
});

test("dragging a tab along the bar changes the tabs' order", () => {
  assert.deepEqual(moveTab(["a", "b", "c"], 0, 2), ["b", "c", "a"]);
  assert.deepEqual(moveTab(["a", "b", "c"], 2, 0), ["c", "a", "b"]);
  assert.deepEqual(moveTab(["a", "b", "c"], 1, 1), ["a", "b", "c"]);
  assert.deepEqual(moveTab(["a", "b", "c"], 1, 9), ["a", "c", "b"]);
  // Three tabs 100 wide; the first one dragged: past the second's middle, then past the third's.
  const edges = [
    { left: 0, right: 100 },
    { left: 100, right: 200 },
    { left: 200, right: 300 },
  ];
  assert.equal(placeAt(edges, 0, 120), 0);
  assert.equal(placeAt(edges, 0, 160), 1);
  assert.equal(placeAt(edges, 0, 290), 2);
  // The last one dragged back to the front.
  assert.equal(placeAt(edges, 2, 10), 0);
  assert.equal(placeAt(edges, 2, 140), 1);
});

test("closing the tab in front brings the next one forward", () => {
  const tabs = [{ id: 1 }, { id: 2 }, { id: 3 }];
  assert.equal(frontAfterClose(tabs, 2, 2), 3);
  assert.equal(frontAfterClose(tabs, 3, 3), 2);
  // Another tab closing leaves the front tab; the last tab closing leaves none (the window closes).
  assert.equal(frontAfterClose(tabs, 1, 3), 3);
  assert.equal(frontAfterClose([{ id: 7 }], 7, 7), null);
});

test("the other tabs slide aside to open the gap where the dragged tab will land", () => {
  // Four tabs; the first dragged to place 2: the second and third slide left by its width.
  assert.deepEqual([0, 1, 2, 3].map((i) => shiftFor(i, 0, 2, 100)), [0, -100, -100, 0]);
  // The last dragged to place 1: the second and third slide right.
  assert.deepEqual([0, 1, 2, 3].map((i) => shiftFor(i, 3, 1, 100)), [0, 100, 100, 0]);
  // Back at its own place: nothing moves.
  assert.deepEqual([0, 1, 2].map((i) => shiftFor(i, 1, 1, 100)), [0, 0, 0]);
  // A tab from another window over the bar at place 1: every tab from there slides right.
  assert.deepEqual([0, 1, 2].map((i) => shiftFor(i, -1, 1, 100)), [0, 100, 100]);
  // And it joins at that place.
  assert.deepEqual(insertAt(["a", "b", "c"], "x", 1), ["a", "x", "b", "c"]);
  assert.deepEqual(insertAt(["a"], "x", 9), ["a", "x"]);
});

test("a tab tears off once it is pulled well out of the bar", () => {
  // The bar runs from 0 to 37 in an 800-wide window.
  assert.equal(tearsOff(100, 50, 0, 37, 800), false);
  assert.equal(tearsOff(100, 68, 0, 37, 800), true);
  assert.equal(tearsOff(-1, 20, 0, 37, 800), true);
  assert.equal(tearsOff(800, 20, 0, 37, 800), true);
});

const task = (over: Partial<Task>): Task => ({
  id: "A01-1",
  title: "x",
  area: "A01",
  state: "ready",
  label: null,
  holder: null,
  waiting_on: null,
  next_dependency: null,
  iteration: null,
  handoff_note: null,
  without_human_check: null,
  human_check_steps: null,
  description: "",
  ...over,
});

test("the dot marks a taskbag with a question to answer or a check", () => {
  assert.equal(needsPerson([task({}), task({ state: "done" })], false), false);
  assert.equal(needsPerson([task({ state: "waiting", waiting_on: "the approver" })], false), true);
  assert.equal(needsPerson([task({ state: "done", without_human_check: "try it by hand" })], false), true);
  // An agent's question (`alm confirm`) waiting in the window.
  assert.equal(needsPerson([], true), true);
});

test("a taskbag already open in another tab is found", () => {
  const tab = (id: number, root: string | null): TabEntry => ({ id, start: { how: "blank" }, root, name: null, needs: false });
  const tabs = [tab(1, "/a/.allomos/taskbag"), tab(2, null)];
  assert.equal(tabShowing(tabs, "/a/.allomos/taskbag/", 2)?.id, 1);
  // A tab does not count itself; a taskbag no tab shows is not found.
  assert.equal(tabShowing(tabs, "/a/.allomos/taskbag", 1), null);
  assert.equal(tabShowing(tabs, "/b/.allomos/taskbag", 2), null);
});
