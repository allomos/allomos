// Tests for the start page's Build a new app… (Task-A03-25-02). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { FIRST_MODE, MANUAL, canStart, checkFolder } from "../src/buildAppFlow.ts";
import type { Agent } from "../src/types.ts";

const agent = (id: Agent["id"]): Agent => ({ id, found: id, how: id.endsWith("app") ? "link" : "paste" });

test("quiet mode is ticked first", () => {
  assert.equal(FIRST_MODE, "quiet");
});

test("only an empty folder is used", () => {
  assert.deepEqual(checkFolder({ kind: "empty", folder: "/a", suggested_root: "/a/.allomos/taskbag" }), {
    kind: "ok",
    folder: "/a",
    root: "/a/.allomos/taskbag",
  });
  assert.deepEqual(checkFolder({ kind: "no_taskbag", folder: "/b", suggested_root: "/b/.allomos/taskbag" }), {
    kind: "not_empty",
    folder: "/b",
  });
  assert.deepEqual(checkFolder({ kind: "found", root: "/c/.allomos/taskbag" }), { kind: "not_empty", folder: "/c/.allomos/taskbag" });
  assert.deepEqual(checkFolder({ kind: "not_a_folder", path: "/d" }), { kind: "not_a_folder", path: "/d" });
});

test("Start needs an empty folder, a description and an agent", () => {
  const ok = checkFolder({ kind: "empty", folder: "/a", suggested_root: "/a/.allomos/taskbag" });
  const full = checkFolder({ kind: "no_taskbag", folder: "/b", suggested_root: "/b/.allomos/taskbag" });
  assert.equal(canStart(ok, "A recipe box", "claude_app"), true);
  assert.equal(canStart(ok, "A recipe box", MANUAL), true);
  assert.equal(canStart(ok, "  \n", "claude_app"), false);
  assert.equal(canStart(ok, "A recipe box", null), false);
  assert.equal(canStart(full, "A recipe box", "claude_app"), false);
  assert.equal(canStart(null, "A recipe box", "claude_app"), false);
});
