// Tests for the Handoffs tab (Task-A03-06). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { filterHandoffs, latestFor, writers } from "../src/handoffFlow.ts";
import type { Handoff } from "../src/types.ts";

const h = (file: string, writer: string, tasks: string[]): Handoff => ({
  file,
  writer,
  tasks,
  written: "",
  updated: "",
  summary: "",
  body: "",
});
// Newest first, as `list_handoffs` gives them.
const all = [
  h("c.md", "claude/b", ["A1-02"]),
  h("b.md", "codex/a", ["A1-01", "A1-02"]),
  h("a.md", "claude/a", ["A1-01"]),
  h("p.md", "will", []),
];

test("only the latest per task by default; a handoff with no task stands for its writer", () => {
  const shown = filterHandoffs(all, { task: null, writer: null, latestOnly: true }).map((x) => x.file);
  assert.deepEqual(shown, ["c.md", "b.md", "p.md"]);
});

test("everything when latest only is off", () => {
  assert.equal(filterHandoffs(all, { task: null, writer: null, latestOnly: false }).length, 4);
});

test("by task and by writer", () => {
  assert.deepEqual(
    filterHandoffs(all, { task: "A1-01", writer: null, latestOnly: false }).map((x) => x.file),
    ["b.md", "a.md"],
  );
  assert.deepEqual(
    filterHandoffs(all, { task: null, writer: "claude/a", latestOnly: true }).map((x) => x.file),
    ["a.md"],
  );
});

test("a task's latest handoff, and the writers", () => {
  assert.equal(latestFor(all, "A1-01")?.file, "b.md");
  assert.equal(latestFor(all, "A9-99"), null);
  assert.deepEqual(writers(all), ["claude/b", "codex/a", "claude/a", "will"]);
});
