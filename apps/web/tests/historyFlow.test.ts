// Tests for the task pane's History (Task-A01-17). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { changeLine, fieldName, valueText } from "../src/historyFlow.ts";
import { localTime } from "../src/timeFlow.ts";

// Stands in for the interface's `t`: the key, with any parameters after it.
const t = (key: string, params?: Record<string, string | number>) =>
  params ? `${key}(${Object.values(params).join(",")})` : key;

test("task fields use the task pane's names; unknown fields stay as stored", () => {
  assert.equal(fieldName("next_dependency", t), "pane.next");
  assert.equal(fieldName("title", t), "pane.title");
  assert.equal(fieldName("colour", t), "colour");
  // Decision 0020: a task's labels, a list since `allomos/7`, under the same name as before.
  assert.equal(fieldName("labels", t), "pane.label");
  assert.equal(fieldName("label", t), "pane.label");
  // Task-A03-18: the human check's note (under either name) and its steps.
  assert.equal(fieldName("without_human_check", t), "pane.without_human_check");
  assert.equal(fieldName("finished_without_owner", t), "pane.without_human_check");
  assert.equal(fieldName("human_check_steps", t), "pane.human_check_steps");
  // Task-A03-19: a decision recorded, then reviewed.
  assert.equal(fieldName("ai_decision", t), "pane.ai_decision");
  assert.equal(changeLine({ field: "ai_decision", to: "Names: alm note" }, t), "pane.ai_decision: — → Names: alm note");
  assert.equal(changeLine({ field: "ai_decision", from: "Names: alm note" }, t), "pane.ai_decision: Names: alm note → —");
});

test("a state is worded, an empty side is a dash, long text is shortened", () => {
  assert.equal(valueText("state", "in_progress", t), "state.in_progress");
  assert.equal(valueText("holder", undefined, t), "—");
  assert.equal(valueText("holder", "", t), "—");
  assert.equal(valueText("title", "x".repeat(70), t), `${"x".repeat(60)}…`);
  assert.equal(valueText("handoff_note", "first line\nsecond", t), "first line…");
  assert.equal(valueText("labels", ["a", "b"], t), "a, b");
  assert.equal(valueText("labels", [], t), "—");
  assert.equal(valueText("colours", ["a", "b"], t), '["a","b"]');
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

test("an approval record shows as its ID; an older approval text as it is (Task-A01-coach782)", () => {
  assert.equal(valueText("approval", { id: "ap-abc123", channel: "chat" }, t), "ap-abc123");
  assert.equal(valueText("approval", "Deploy (plan 0123)", t), "Deploy (plan 0123)");
});

test("a human check shows its result, where, who, the steps and the words (Task-A01-table778)", () => {
  assert.equal(
    valueText("human_check", { checker: "p-abc123", steps_sha256: "0123456789ab", result: "partly", words: "OK", where: "deployed" }, t),
    'pane.human_check_partly (pane.human_check_deployed), pane.human_check_by(p-abc123), pane.human_check_steps_sha(01234567): "OK"',
  );
  assert.equal(fieldName("human_check", t), "pane.human_check");
});

test("a task's traces show as a list (Task-A01-fennel617)", () => {
  assert.equal(valueText("traces", ["R77", "0021"], t), "R77, 0021");
  assert.equal(valueText("traces", [], t), "—");
  assert.equal(fieldName("traces", t), "pane.traces");
});
