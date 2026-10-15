// Tests for the refresh line (Task-A01-47): which message shows, and which tasks changed.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { changedTasks, clock, refreshText, whatChanged, type Refreshed } from "../src/refreshFlow.ts";
import type { Loaded, Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };

// The English words, with parameters filled in, as the interface shows them.
const t = (key: string, params?: Record<string, string | number>) =>
  ((en as Record<string, string>)[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));

const base: Task = {
  id: "A01-33",
  title: "Refresh line",
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
};
const task = (over: Partial<Task>): Task => ({ ...base, ...over });
const at = new Date(2026, 9, 15, 14, 35, 12);

test("the time of day is written with seconds, padded", () => {
  assert.equal(clock(new Date(2026, 9, 15, 9, 5, 3)), "09:05:03");
  assert.equal(clock(at), "14:35:12");
});

test("the three messages the approver chose", () => {
  assert.equal(refreshText({ how: "opened", at: new Date(2026, 9, 15, 14, 20, 3) }, t), "Loaded at 14:20:03, when the taskbag was opened");
  assert.equal(refreshText({ how: "auto", at: new Date(2026, 9, 15, 14, 32, 5) }, t), "Refreshed automatically at 14:32:05 · the taskbag files changed");
  assert.equal(refreshText({ how: "manual", at, changed: { tasks: [], other: false } }, t), "Refreshed by you at 14:35:12 · nothing changed");
  assert.equal(refreshText({ how: "manual", at, changed: { tasks: ["A01-33"], other: false } }, t), "Refreshed by you at 14:35:12 · 1 task changed (A01-33)");
});

test("several changed tasks are counted, and a long list is cut after five", () => {
  const two: Refreshed = { how: "manual", at, changed: { tasks: ["A01-33", "A01-34"], other: true } };
  assert.equal(refreshText(two, t), "Refreshed by you at 14:35:12 · 2 tasks changed (A01-33, A01-34)");
  const seven: Refreshed = { how: "manual", at, changed: { tasks: ["T1", "T2", "T3", "T4", "T5", "T6", "T7"], other: false } };
  assert.equal(refreshText(seven, t), "Refreshed by you at 14:35:12 · 7 tasks changed (T1, T2, T3, T4, T5 and 2 more)");
});

test("no task changed but areas, iterations or the project did: not called 'nothing changed'", () => {
  assert.equal(
    refreshText({ how: "manual", at, changed: { tasks: [], other: true } }, t),
    "Refreshed by you at 14:35:12 · no task changed; areas, iterations or the project did",
  );
});

test("changed, added and removed tasks are found; field order does not count as a change", () => {
  const a = task({ id: "A01-01" });
  const b = task({ id: "A01-02" });
  const c = task({ id: "A01-03" });
  assert.deepEqual(changedTasks([a, b, c], [a, b, c]), []);
  // Same fields in another order: the same task.
  const reordered = Object.fromEntries(Object.entries(b).reverse()) as Task;
  assert.deepEqual(changedTasks([a, b], [a, reordered]), []);
  // A01-02 changed its label, A01-04 is new, A01-03 is gone.
  const d = task({ id: "A01-04" });
  assert.deepEqual(changedTasks([a, b, c], [a, { ...b, label: "implemented" }, d]), ["A01-02", "A01-04", "A01-03"]);
  // A new AI decision counts as a change of its task.
  assert.deepEqual(changedTasks([a], [{ ...a, ai_decisions: [{ at: "2026-10-15T14:00Z", text: "x" }] }]), ["A01-01"]);
});

test("whatChanged tells task changes from other changes", () => {
  const box = (over: Partial<NonNullable<Loaded["taskbag"]>>): Loaded["taskbag"] => ({
    project: { id: "P", name: "Trial" } as NonNullable<Loaded["taskbag"]>["project"],
    areas: [],
    iterations: [],
    tasks: [task({})],
    ...over,
  });
  assert.deepEqual(whatChanged(box({}), box({})), { tasks: [], other: false });
  assert.deepEqual(whatChanged(box({}), box({ tasks: [task({ state: "done" })] })), { tasks: ["A01-33"], other: false });
  assert.deepEqual(
    whatChanged(box({}), box({ project: { id: "P", name: "Renamed" } as NonNullable<Loaded["taskbag"]>["project"] })),
    { tasks: [], other: true },
  );
  assert.deepEqual(whatChanged(null, null), { tasks: [], other: false });
});
