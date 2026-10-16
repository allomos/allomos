// Tests for Reopen… in the task pane (Task-A01-50). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canReopen, reopenEdit } from "../src/reopenFlow.ts";
import { changeLine, fieldName } from "../src/historyFlow.ts";

const t = (key: string, params?: Record<string, string | number>) =>
  params ? `${key}(${Object.values(params).join(",")})` : key;

test("only a done or dropped task offers Reopen…", () => {
  assert.equal(canReopen({ state: "done" }), true);
  assert.equal(canReopen({ state: "dropped" }), true);
  for (const state of ["backlog", "ready", "in_progress", "waiting", "blocked"] as const) {
    assert.equal(canReopen({ state }), false, state);
  }
});

test("Reopen needs a reason, sent trimmed", () => {
  assert.equal(reopenEdit(""), null);
  assert.equal(reopenEdit("  \n "), null);
  assert.deepEqual(reopenEdit("  The check failed \n"), { field: "reopen", note: "The check failed" });
});

test("History names the reason", () => {
  assert.equal(fieldName("reopen_note", t), "pane.reopen_note");
  assert.equal(changeLine({ field: "reopen_note", to: "The check failed" }, t), "pane.reopen_note: — → The check failed");
});
