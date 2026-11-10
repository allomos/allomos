// Tests for the update notice (Task-A07-16), its menu item (Task-A07-17, A07-glove300) and what it
// shows while the installer downloads (Task-A07-fact210).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { updateMenuItem, updateNote } from "../src/updateFlow.ts";
import type { UpdateStatus } from "../src/types.ts";

const status = (s: Partial<UpdateStatus>): UpdateStatus => ({ current: "0.1.5", latest: "0.1.5", newer: null, available: null, failed: false, ready: null, preparing: null, ...s });

test("nothing is shown before the first answer", () => {
  assert.equal(updateNote(null, false, null), null);
});

test("a newer version whose installer is not ready is not offered (Task-A07-fact210)", () => {
  // Only the source code is released so far: a check on its own shows nothing, and no Download.
  assert.equal(updateNote(status({ newer: "0.1.6", available: "0.1.6" }), false, null), null);
  // Check for Updates… says the installer is being prepared, never offers Download.
  assert.deepEqual(updateNote(status({ newer: "0.1.6", available: "0.1.6" }), true, null), {
    kind: "preparing",
    latest: "0.1.6",
    current: "0.1.5",
  });
});

test("the notice offers Restart to Update once the installer is ready, as the menu does", () => {
  const ready = status({ newer: "0.1.6", available: "0.1.6", ready: "0.1.6" });
  assert.deepEqual(updateNote(ready, false, null), { kind: "offer", latest: "0.1.6", current: "0.1.5" });
  assert.deepEqual(updateMenuItem(ready), { action: "restart", latest: "0.1.6" });
  // An installer for an older find does not make the notice offer a restart to a newer one.
  const older = status({ newer: "0.1.7", available: "0.1.7", ready: "0.1.6" });
  assert.equal(updateNote(older, false, null), null);
});

test("a check on its own is quiet when up to date or offline", () => {
  assert.equal(updateNote(status({}), false, null), null);
  assert.equal(updateNote(status({ failed: true }), false, null), null);
});

test("closing the notice hides that version, not a newer one", () => {
  const ready = (v: string) => status({ newer: v, available: v, ready: v });
  assert.equal(updateNote(ready("0.1.6"), false, "0.1.6"), null);
  assert.equal(updateNote(ready("0.1.7"), false, "0.1.6")?.kind, "offer");
});

test("Check for Updates… always answers", () => {
  assert.deepEqual(updateNote(status({}), true, null), { kind: "current", current: "0.1.5" });
  assert.deepEqual(updateNote(status({ failed: true, latest: null }), true, null), { kind: "failed" });
  const ready = status({ newer: "0.1.6", available: "0.1.6", ready: "0.1.6" });
  assert.equal(updateNote(ready, true, "0.1.6")?.kind, "offer");
});

test("the menu item checks, then downloads, then restarts to update (Task-A07-17)", () => {
  assert.deepEqual(updateMenuItem(null), { action: "check" });
  assert.deepEqual(updateMenuItem(status({})), { action: "check" });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6" })), { action: "download", latest: "0.1.6", waiting: false, percent: null });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6", ready: "0.1.6" })), { action: "restart", latest: "0.1.6" });
  // A download of an older find does not offer a restart to a newer one.
  assert.deepEqual(updateMenuItem(status({ available: "0.1.7", ready: "0.1.6" })), { action: "download", latest: "0.1.7", waiting: false, percent: null });
});

test("while the installer downloads, the notice shows its percentage, and Download waits (A07-fact210, A07-glove300)", () => {
  const downloading = status({ newer: "0.1.6", available: "0.1.6", preparing: "0.1.6" });
  assert.deepEqual(updateNote(downloading, false, null, 42), { kind: "downloading", latest: "0.1.6", current: "0.1.5", percent: 42 });
  assert.deepEqual(updateNote(downloading, true, null, 42), { kind: "downloading", latest: "0.1.6", current: "0.1.5", percent: 42 });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6", preparing: "0.1.6" }), 42), {
    action: "download",
    latest: "0.1.6",
    waiting: true,
    percent: 42,
  });
  // The size is not known yet: the notice says only that it is being prepared.
  assert.deepEqual(updateNote(downloading, false, null, null), { kind: "downloading", latest: "0.1.6", current: "0.1.5", percent: null });
  // Once ready, the notice offers Restart to Update and nothing waits.
  assert.deepEqual(updateNote(status({ newer: "0.1.6", available: "0.1.6", ready: "0.1.6" }), false, null, null), {
    kind: "offer",
    latest: "0.1.6",
    current: "0.1.5",
  });
  assert.deepEqual(updateMenuItem(status({ available: "0.1.6", ready: "0.1.6" }), 100), { action: "restart", latest: "0.1.6" });
});

test("closing the downloading notice hides that version in this window", () => {
  const downloading = status({ newer: "0.1.6", available: "0.1.6", preparing: "0.1.6" });
  assert.equal(updateNote(downloading, false, "0.1.6", 10), null);
  assert.equal(updateNote(downloading, true, "0.1.6", 10)?.kind, "downloading");
});

test("a download of another version does not make Download wait (A07-glove300)", () => {
  assert.deepEqual(updateMenuItem(status({ available: "0.1.7", preparing: "0.1.6" }), 42), {
    action: "download",
    latest: "0.1.7",
    waiting: false,
    percent: null,
  });
});
