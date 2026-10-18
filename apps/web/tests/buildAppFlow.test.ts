// Tests for the start page's Build a new app… (Task-A03-25-02). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { FIRST_MODE, MANUAL, canStart, checkFolder, checkTaskbagFolder, taskbagRoot } from "../src/buildAppFlow.ts";
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

test("the taskbag goes in the app folder unless the person chooses another place", () => {
  const ok = checkFolder({ kind: "empty", folder: "/a", suggested_root: "/a/.allomos/taskbag" });
  assert.equal(taskbagRoot(ok, null), "/a/.allomos/taskbag");
  assert.equal(taskbagRoot(ok, "/tb/.allomos/taskbag"), "/tb/.allomos/taskbag");
  assert.equal(taskbagRoot(null, "/tb/.allomos/taskbag"), null);
  assert.equal(taskbagRoot(checkFolder({ kind: "not_a_folder", path: "/d" }), null), null);
});

test("a place that already holds a taskbag is refused", () => {
  assert.deepEqual(checkTaskbagFolder({ kind: "empty", folder: "/t", suggested_root: "/t/.allomos/taskbag" }), {
    kind: "ok",
    root: "/t/.allomos/taskbag",
  });
  assert.deepEqual(checkTaskbagFolder({ kind: "no_taskbag", folder: "/n", suggested_root: "/n/.allomos/taskbag" }), {
    kind: "ok",
    root: "/n/.allomos/taskbag",
  });
  assert.deepEqual(checkTaskbagFolder({ kind: "found", root: "/f/.allomos/taskbag" }), { kind: "has_taskbag", root: "/f/.allomos/taskbag" });
  assert.deepEqual(checkTaskbagFolder({ kind: "not_a_folder", path: "/x" }), { kind: "not_a_folder", path: "/x" });
});
