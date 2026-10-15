// Tests for the update notice (Task-A07-16). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { updateNote } from "../src/updateFlow.ts";
import type { UpdateStatus } from "../src/types.ts";

const status = (s: Partial<UpdateStatus>): UpdateStatus => ({ current: "0.1.5", latest: "0.1.5", newer: null, available: null, failed: false, ...s });

test("nothing is shown before the first answer", () => {
  assert.equal(updateNote(null, false, null), null);
});

test("a newer version is offered", () => {
  assert.deepEqual(updateNote(status({ latest: "0.1.6", newer: "0.1.6" }), false, null), {
    kind: "offer",
    latest: "0.1.6",
    current: "0.1.5",
  });
});

test("a check on its own is quiet when up to date or offline", () => {
  assert.equal(updateNote(status({}), false, null), null);
  assert.equal(updateNote(status({ failed: true }), false, null), null);
});

test("closing the notice hides that version, not a newer one", () => {
  assert.equal(updateNote(status({ newer: "0.1.6" }), false, "0.1.6"), null);
  assert.equal(updateNote(status({ newer: "0.1.7" }), false, "0.1.6")?.kind, "offer");
});

test("Check for Updates… always answers", () => {
  assert.deepEqual(updateNote(status({}), true, null), { kind: "current", current: "0.1.5" });
  assert.deepEqual(updateNote(status({ failed: true, latest: null }), true, null), { kind: "failed" });
  assert.equal(updateNote(status({ newer: "0.1.6" }), true, "0.1.6")?.kind, "offer");
});
