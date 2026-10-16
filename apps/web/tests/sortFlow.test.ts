// Tests for sorting the tasks inside each iteration (Task-A01-52). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_SORT, parseSort, sortTasks } from "../src/sortFlow.ts";
import type { Task } from "../src/types.ts";

const task = (id: string, title: string) => ({ id, title }) as Task;
const tasks = [task("A01-10", "beta"), task("A01-2", "Alpha"), task("A02-01", "gamma"), task("A01-03", "alpha 2")];
const ids = (list: Task[]) => list.map((t) => t.id);

test("by ID, ascending, is the taskbag's own order; numbers count as numbers", () => {
  assert.deepEqual(ids(sortTasks(tasks, DEFAULT_SORT)), ["A01-2", "A01-03", "A01-10", "A02-01"]);
  assert.deepEqual(ids(sortTasks(tasks, { key: "id", dir: "desc" })), ["A02-01", "A01-10", "A01-03", "A01-2"]);
});

test("by name ignores case, A to Z and Z to A", () => {
  assert.deepEqual(ids(sortTasks(tasks, { key: "name", dir: "asc" })), ["A01-2", "A01-03", "A01-10", "A02-01"]);
  assert.deepEqual(ids(sortTasks(tasks, { key: "name", dir: "desc" })), ["A02-01", "A01-10", "A01-03", "A01-2"]);
});

test("by last change: oldest or newest first; a task never changed counts as oldest; ties by ID", () => {
  const changed = { "A01-10": "2026-10-16T09:00:00Z", "A02-01": "2026-10-17T08:00:00Z", "A01-03": "2026-10-16T09:00:00Z" };
  assert.deepEqual(ids(sortTasks(tasks, { key: "changed", dir: "asc" }, changed)), ["A01-2", "A01-03", "A01-10", "A02-01"]);
  assert.deepEqual(ids(sortTasks(tasks, { key: "changed", dir: "desc" }, changed)), ["A02-01", "A01-03", "A01-10", "A01-2"]);
});

test("sorting leaves the given list as it was", () => {
  const before = ids(tasks);
  sortTasks(tasks, { key: "name", dir: "desc" });
  assert.deepEqual(ids(tasks), before);
});

test("a remembered sort survives a round trip; anything unreadable is the default", () => {
  assert.deepEqual(parseSort(JSON.stringify({ key: "changed", dir: "desc" })), { key: "changed", dir: "desc" });
  assert.deepEqual(parseSort(null), DEFAULT_SORT);
  assert.deepEqual(parseSort("nope"), DEFAULT_SORT);
  assert.deepEqual(parseSort('{"key":"size","dir":"asc"}'), DEFAULT_SORT);
  assert.deepEqual(parseSort('{"key":"name","dir":"up"}'), DEFAULT_SORT);
});
