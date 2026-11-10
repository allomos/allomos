// Tests for the update notice (Task-A07-16) and its menu item (Task-A07-17, A07-glove300).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { updateMenuItem, updateNote } from "../src/updateFlow.ts";
import type { UpdateStatus } from "../src/types.ts";

const status = (s: Partial<UpdateStatus>): UpdateStatus => ({ current: "0.1.5", latest: "0.1.5", newer: null, available: null, failed: false, ready: null, preparing: null, ...s });

test("nothing is shown before the first answer", () => {
  assert.equal(updateNote(null, false, null), null);
});

test("a newer version is offered", () => {
  assert.deepEqual(updateNote(status({ latest: "0.1.6", newer: "0.1.6" }), false, null), {
    kind: "offer",
    latest: "0.1.6",
    current: "0.1.5",
    ready: false,
    waiting: false,
  });
});

test("the notice offers Restart to Update once the installer is ready, as the menu does", () => {
  const ready = status({ newer: "0.1.6", available: "0.1.6", ready: "0.1.6" });
  assert.deepEqual(updateNote(ready, false, null), { kind: "offer", latest: "0.1.6", current: "0.1.5", ready: true, waiting: false });
  assert.deepEqual(updateMenuItem(ready), { action: "restart", latest: "0.1.6" });
  // An installer for an older find does not make the notice offer a restart to a newer one.
  const older = status({ newer: "0.1.7", available: "0.1.7", ready: "0.1.6" });
  assert.deepEqual(updateNote(older, false, null), { kind: "offer", latest: "0.1.7", current: "0.1.5", ready: false, waiting: false });
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

test("the menu item checks, then downloads, then restarts to update (Task-A07-17)", () => {
  assert.deepEqual(updateMenuItem(null), { action: "check" });
  assert.deepEqual(updateMenuItem(status({})), { action: "check" });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6" })), { action: "download", latest: "0.1.6", waiting: false });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6", ready: "0.1.6" })), { action: "restart", latest: "0.1.6" });
  // A download of an older find does not offer a restart to a newer one.
  assert.deepEqual(updateMenuItem(status({ available: "0.1.7", ready: "0.1.6" })), { action: "download", latest: "0.1.7", waiting: false });
});

test("Download waits, greyed out, while the installer downloads, then becomes Restart to Update (A07-glove300)", () => {
  const downloading = status({ available: "0.1.6", preparing: "0.1.6" });
  assert.deepEqual(updateMenuItem(downloading), { action: "download", latest: "0.1.6", waiting: true });
  assert.deepEqual(updateNote(status({ newer: "0.1.6", available: "0.1.6", preparing: "0.1.6" }), false, null), {
    kind: "offer",
    latest: "0.1.6",
    current: "0.1.5",
    ready: false,
    waiting: true,
  });
  // Once ready, nothing waits any more.
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6", ready: "0.1.6" })), { action: "restart", latest: "0.1.6" });
});

test("a download of another version does not make Download wait (A07-glove300)", () => {
  assert.deepEqual(updateMenuItem(status({ available: "0.1.7", preparing: "0.1.6" })), { action: "download", latest: "0.1.7", waiting: false });
});
