// Tests for the task pane's History (Task-A01-17). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { changeLine, fieldName, valueText } from "../src/historyFlow.ts";
import { localTime } from "../src/handoffFlow.ts";

// Stands in for the interface's `t`: the key, with any parameters after it.
const t = (key: string, params?: Record<string, string | number>) =>
  params ? `${key}(${Object.values(params).join(",")})` : key;

test("task fields use the task pane's names; unknown fields stay as stored", () => {
  assert.equal(fieldName("next_dependency", t), "pane.next");
  assert.equal(fieldName("title", t), "pane.title");
  assert.equal(fieldName("labels", t), "labels");
  // Task-A03-18: the human check's note (under either name) and its steps.
  assert.equal(fieldName("without_human_check", t), "pane.without_human_check");
  assert.equal(fieldName("finished_without_owner", t), "pane.without_human_check");
  assert.equal(fieldName("human_check_steps", t), "pane.human_check_steps");
});

test("a state is worded, an empty side is a dash, long text is shortened", () => {
  assert.equal(valueText("state", "in_progress", t), "state.in_progress");
  assert.equal(valueText("holder", undefined, t), "—");
  assert.equal(valueText("holder", "", t), "—");
  assert.equal(valueText("title", "x".repeat(70), t), `${"x".repeat(60)}…`);
  assert.equal(valueText("handoff_note", "first line\nsecond", t), "first line…");
  assert.equal(valueText("labels", ["a", "b"], t), '["a","b"]');
});

test("a change reads field: from → to; a description only says it changed", () => {
  assert.equal(
    changeLine({ field: "state", from: "ready", to: "in_progress" }, t),
    "pane.state: state.ready → state.in_progress",
  );
  assert.equal(changeLine({ field: "holder", to: "claude/x" }, t), "pane.holder: — → claude/x");
  assert.equal(changeLine({ field: "description", to: "long text" }, t), "pane.history_description(pane.description)");
});

test("event times with seconds read as local times", () => {
  assert.match(localTime("2026-10-13T14:02:05Z"), /^2026-10-1[34] \d\d:\d\d$/);
  assert.match(localTime("2026-10-12T11:30Z"), /^2026-10-1[23] \d\d:\d\d$/);
  assert.equal(localTime("soon"), "soon");
});
