// Tests for the Monitors list (Task-A03-motor529): rows from `monitor_all`, the project's name and
// taskbag, and the count the button and the Window menu show.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { folderName, listedRuns } from "../src/monitorFlow.ts";

const run = (over: Record<string, unknown> = {}) => ({
  id: "m-6af1a1f8-8782",
  pid: 25001,
  taskbag: "/Users/me/projects/jobs/.allomos/taskbag",
  project: "/Users/me/projects/jobs",
  record: "present",
  next_check: 1_000,
  run: {
    scope: { kind: "all" },
    agents: ["claude_cli"],
    offered: ["claude_cli", "codex_cli"],
    quiet: true,
    every: 30,
    started: 1,
    taken: [],
    working: [],
    stopping: false,
    checked: 0,
    unblocked: [],
  },
  ...over,
});

test("a run's row names its project's folder and taskbag", () => {
  const [row] = listedRuns([run()]);
  assert.equal(row.key, "m-6af1a1f8-8782");
  assert.equal(row.name, "jobs");
  assert.equal(row.root, "/Users/me/projects/jobs/.allomos/taskbag");
  assert.ok(row.run);
  assert.equal(row.lost, null);
});

test("a process with no record is a row that can only be stopped by its process", () => {
  const [row] = listedRuns([{ record: "missing", pid: 25547, id: null, taskbag: null }]);
  assert.equal(row.key, "pid:25547");
  assert.equal(row.run, null);
  assert.equal(row.lost?.pid, 25547);
  assert.equal(row.root, null);
  assert.equal(row.name, null);
});

test("a missing record that names its taskbag still shows the project", () => {
  const [row] = listedRuns([{ record: "missing", pid: 9, id: "m-x", taskbag: "/a/b/.allomos/taskbag" }]);
  assert.equal(row.lost?.id, "m-x");
  assert.equal(row.name, "b");
  assert.equal(row.root, "/a/b/.allomos/taskbag");
});

test("entries that are neither a run nor a process are left out, and the rest keep their order", () => {
  const rows = listedRuns([run({ id: "m-1" }), { nonsense: true }, null, run({ id: "m-2", project: "/x/other" })]);
  assert.deepEqual(rows.map((r) => r.key), ["m-1", "m-2"]);
  assert.equal(rows[1].name, "other");
});

test("every run and every process with a missing record is a row, so the count is the rows", () => {
  assert.equal(listedRuns([]).length, 0);
  assert.equal(listedRuns([run(), { record: "missing", pid: 3, id: null, taskbag: null }]).length, 2);
});

test("folder names split on both kinds of separator and ignore trailing ones", () => {
  assert.equal(folderName("C:\\Users\\me\\jobs\\"), "jobs");
  assert.equal(folderName("/a/b/"), "b");
  assert.equal(folderName("/"), null);
});
