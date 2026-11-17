// Tests for the dependency tree (Task-A01-alpine458): building the trees of one group and keeping
// the sort's order among siblings. Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { depTreeRows, parseDepTree, type TreeRow } from "../src/depTreeFlow.ts";
import { sortTasks } from "../src/sortFlow.ts";
import type { Task } from "../src/types.ts";

const task = (id: string, by: string[] = [], title = id) => ({ id, title, blocked_by: by.length ? by : undefined }) as Task;
/** Each line as `depth:id`, so a tree reads at a glance. */
const shape = (rows: TreeRow[]) => rows.map((r) => `${r.depth}:${r.task.id}`);
const row = (rows: TreeRow[], id: string) => rows.find((r) => r.task.id === id)!;

test("a chain: each task hangs under the one that blocks it", () => {
  const rows = depTreeRows([task("A01-3", ["A01-2"]), task("A01-1"), task("A01-2", ["A01-1"])]);
  assert.deepEqual(shape(rows), ["0:A01-1", "1:A01-2", "2:A01-3"]);
  assert.deepEqual(row(rows, "A01-1").kids, 1);
  assert.deepEqual(row(rows, "A01-3").lines, [false]);
});

test("a fork: the children keep the sort's order, and so do the roots; a task alone is a root among them", () => {
  const tasks = [task("A01-1"), task("A01-2", ["A01-1"]), task("A01-3", ["A01-1"]), task("A01-4"), task("A01-5", ["A01-4"])];
  assert.deepEqual(shape(depTreeRows(sortTasks(tasks, { key: "id", dir: "asc" }))), ["0:A01-1", "1:A01-2", "1:A01-3", "0:A01-4", "1:A01-5"]);
  // The other way round: roots and siblings turn, children stay under their parent.
  const desc = depTreeRows(sortTasks(tasks, { key: "id", dir: "desc" }));
  assert.deepEqual(shape(desc), ["0:A01-4", "1:A01-5", "0:A01-1", "1:A01-3", "1:A01-2"]);
  assert.equal(row(desc, "A01-3").last, false);
  assert.equal(row(desc, "A01-2").last, true);
  // By last change, newest first: the single task A01-6 sorts between the two trees, not after them.
  const changed = { "A01-1": "2026-11-01T00:00:00Z", "A01-4": "2026-11-03T00:00:00Z", "A01-6": "2026-11-02T00:00:00Z", "A01-2": "2026-11-05T00:00:00Z", "A01-3": "2026-11-06T00:00:00Z" };
  const newest = depTreeRows(sortTasks([...tasks, task("A01-6")], { key: "changed", dir: "desc" }, changed));
  assert.deepEqual(shape(newest), ["0:A01-4", "1:A01-5", "0:A01-6", "0:A01-1", "1:A01-3", "1:A01-2"]);
});

test("when sorted by creation time, children under the same parent keep that order", () => {
  const tasks = [task("A01-1"), task("A01-10", ["A01-1"]), task("A01-2", ["A01-1"])];
  const created = {
    "A01-1": "2026-10-01T00:00:00Z",
    "A01-10": "2026-10-03T00:00:00Z",
    "A01-2": "2026-10-02T00:00:00Z",
  };
  const rows = depTreeRows(sortTasks(tasks, { key: "created", dir: "asc" }, {}, created));
  assert.deepEqual(shape(rows), ["0:A01-1", "1:A01-2", "1:A01-10"]);
});

test("guide lines: an ancestor with a later sibling runs a line through the lines under it", () => {
  const rows = depTreeRows([task("A01-1"), task("A01-2", ["A01-1"]), task("A01-3", ["A01-2"]), task("A01-4", ["A01-3"]), task("A01-5", ["A01-1"])]);
  assert.deepEqual(shape(rows), ["0:A01-1", "1:A01-2", "2:A01-3", "3:A01-4", "1:A01-5"]);
  assert.deepEqual(row(rows, "A01-3").lines, [true]);
  assert.deepEqual(row(rows, "A01-4").lines, [true, false]);
  assert.deepEqual(row(rows, "A01-5").lines, []);
});

test("several parents: under the one first in the sort, shown once, the others named", () => {
  const tasks = [task("A01-1"), task("A01-2"), task("A01-3", ["A01-2", "A01-1"])];
  const asc = depTreeRows(sortTasks(tasks, { key: "id", dir: "asc" }));
  assert.deepEqual(shape(asc), ["0:A01-1", "1:A01-3", "0:A01-2"]);
  assert.deepEqual(row(asc, "A01-3").also, ["A01-2"]);
  const desc = depTreeRows(sortTasks(tasks, { key: "id", dir: "desc" }));
  assert.deepEqual(shape(desc), ["0:A01-2", "1:A01-3", "0:A01-1"]);
  assert.deepEqual(row(desc, "A01-3").also, ["A01-1"]);
});

test("a parent in another group: the task is a root here, or hangs under its parent in the group", () => {
  const rows = depTreeRows([task("A01-1"), task("A01-2", ["A02-9"]), task("A01-3", ["A02-9", "A01-1"])]);
  assert.deepEqual(shape(rows), ["0:A01-1", "1:A01-3", "0:A01-2"]);
  assert.deepEqual(row(rows, "A01-2").outside, ["A02-9"]);
  assert.deepEqual(row(rows, "A01-3").outside, ["A02-9"]);
  assert.deepEqual(row(rows, "A01-3").also, []);
});

test("a circle is cut by ID and every task still shows once", () => {
  const two = depTreeRows([task("A01-2", ["A01-1"]), task("A01-1", ["A01-2"])]);
  assert.deepEqual(shape(two), ["0:A01-1", "1:A01-2"]);
  assert.deepEqual(row(two, "A01-1").also, ["A01-2"]);
  const three = depTreeRows([task("A01-3", ["A01-2"]), task("A01-2", ["A01-1"]), task("A01-1", ["A01-3"]), task("A01-4", ["A01-4"])]);
  assert.deepEqual(shape(three), ["0:A01-1", "1:A01-2", "2:A01-3", "0:A01-4"]);
  // A task in a circle that has a blocker outside it hangs there instead of becoming a root.
  const out = depTreeRows([task("A01-0"), task("A01-1", ["A01-2", "A01-0"]), task("A01-2", ["A01-1"])]);
  assert.deepEqual(shape(out), ["0:A01-0", "1:A01-1", "2:A01-2"]);
  // Word IDs: the taskbag's ID order decides, when given.
  const words = [task("A01-zebra001", ["A01-apple002"]), task("A01-apple002", ["A01-zebra001"])];
  assert.deepEqual(shape(depTreeRows(words, [], new Map([["A01-zebra001", 0], ["A01-apple002", 1]]))), ["0:A01-zebra001", "1:A01-apple002"]);
});

test("folding a parent hides everything under it and counts it", () => {
  const tasks = [task("A01-1"), task("A01-2", ["A01-1"]), task("A01-3", ["A01-2"]), task("A01-4", ["A01-1"]), task("A01-5")];
  const rows = depTreeRows(tasks, ["A01-1", "A01-5"]);
  assert.deepEqual(shape(rows), ["0:A01-1", "0:A01-5"]);
  assert.deepEqual([row(rows, "A01-1").folded, row(rows, "A01-1").hidden], [true, 3]);
  // A fold remembered for a task with nothing under it means nothing.
  assert.deepEqual([row(rows, "A01-5").folded, row(rows, "A01-5").hidden], [false, 0]);
  const inner = depTreeRows(tasks, ["A01-2"]);
  assert.deepEqual(shape(inner), ["0:A01-1", "1:A01-2", "1:A01-4", "0:A01-5"]);
  assert.equal(row(inner, "A01-2").hidden, 1);
});

test("the remembered choice: on only when storage says so", () => {
  assert.equal(parseDepTree("on"), true);
  assert.equal(parseDepTree("off"), false);
  assert.equal(parseDepTree(null), false);
});
