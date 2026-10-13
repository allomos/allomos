// Tests for adding tasks, areas and iterations in the list (Task-A01-19). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canAdd, pickArea, refreshItem, refreshTask, startItem, startTask, toNewIteration, toNewTask } from "../src/createFlow.ts";
import type { NextIds } from "../src/types.ts";

const next: NextIds = { area: "A03", iteration: "I16", tasks: { A01: "A01-30", A02: "A02-05" } };

test("a new task in an area group suggests the next free ID in that area", () => {
  const d = startTask({ kind: "task", group: "a-A02", area: "A02", iteration: null }, ["A01", "A02"], next);
  assert.deepEqual(d, { id: "A02-05", suggested: "A02-05", area: "A02", title: "", description: "", iteration: "" });
});

test("in an iteration group the area is picked, and the suggested ID follows it", () => {
  let d = startTask({ kind: "task", group: "i-I15", area: null, iteration: "I15" }, ["A01", "A02"], next);
  assert.equal(d.area, "A01");
  assert.equal(d.id, "A01-30");
  assert.equal(d.iteration, "I15");
  d = pickArea(d, "A02", next);
  assert.equal(d.id, "A02-05");
  // An ID the person typed stays.
  d = pickArea({ ...d, id: "A02-99" }, "A01", next);
  assert.equal(d.id, "A02-99");
  // An emptied ID box takes the suggestion again.
  d = pickArea({ ...d, id: "" }, "A02", next);
  assert.equal(d.id, "A02-05");
});

test("new suggestions after a reload replace only an untyped ID", () => {
  const d = startTask({ kind: "task", group: "a-A01", area: "A01", iteration: null }, ["A01"], next);
  const later: NextIds = { ...next, tasks: { A01: "A01-31" } };
  assert.equal(refreshTask(d, later).id, "A01-31");
  assert.equal(refreshTask({ ...d, id: "A01-77" }, later).id, "A01-77");
});

test("a project with no areas or suggestions starts empty", () => {
  const d = startTask({ kind: "task", group: "unplanned", area: null, iteration: null }, [], null);
  assert.equal(d.area, "");
  assert.equal(d.id, "");
});

test("what is sent: an empty ID means the next free one; empty text is left out", () => {
  const d = startTask({ kind: "task", group: "i-I15", area: null, iteration: "I15" }, ["A01"], next);
  assert.equal(canAdd(d), false);
  assert.deepEqual(toNewTask({ ...d, id: " ", title: " Plan it ", description: "Why.\n" }), {
    id: null,
    area: "A01",
    title: "Plan it",
    description: "Why.\n",
    iteration: "I15",
  });
  assert.equal(toNewTask({ ...d, title: "t", description: "  " }).description, null);
  assert.equal(toNewTask({ ...d, title: "t", iteration: "" }).iteration, null);
});

test("new areas and iterations start from the suggested ID", () => {
  assert.equal(startItem("area", next).id, "A03");
  const it = startItem("iteration", next);
  assert.equal(it.id, "I16");
  assert.equal(canAdd(it), false);
  assert.deepEqual(toNewIteration({ ...it, name: " v0.3 ", release: "v0.3", start: "2026-10-15" }), {
    id: "I16",
    name: "v0.3",
    release: "v0.3",
    start: "2026-10-15",
    end: null,
  });
  assert.equal(startItem("area", null).id, "");
  // Suggestions that arrive later fill an untyped box; a typed ID stays.
  const empty = startItem("area", null);
  assert.equal(refreshItem(empty, "area", next).id, "A03");
  assert.equal(refreshItem({ ...empty, id: "X1" }, "area", next).id, "X1");
});
