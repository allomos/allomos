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
  for (const state of ["backlog", "ready", "in_progress", "waiting"] as const) {
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

// Send back… (Task-A01-music456, decision 0030 § 4).
import { disagreeText, onlyFilledIn, screenshotLine } from "../src/reopenFlow.ts";

test("An attached screenshot is named at the end of the reason, as alm prints one", () => {
  assert.equal(screenshotLine("A01-01", "send-back.jpg"), "screenshot: tasks/A01-01/evidence/send-back.jpg");
  assert.deepEqual(reopenEdit(" Button does nothing ", { task: "A01-01", file: "send-back.jpg" }), {
    field: "reopen",
    note: "Button does nothing\n\nscreenshot: tasks/A01-01/evidence/send-back.jpg",
  });
  assert.equal(reopenEdit(" ", { task: "A01-01", file: "send-back.jpg" }), null);
});

test("Disagreeing with an AI decision quotes it and leaves a line for why", () => {
  const text = disagreeText("Put it at the bottom.\nSecond line", "I disagree with this AI decision:", "Why: ");
  assert.equal(text, "I disagree with this AI decision:\n> Put it at the bottom.\n> Second line\n\nWhy: ");
  assert.equal(onlyFilledIn(text, text), true);
  assert.equal(onlyFilledIn(`${text}it hides the button`, text), false);
  assert.equal(onlyFilledIn("anything", null), false);
});
