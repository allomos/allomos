// Tests for the Evidence screen of a task to check (Task-A01-herb903, decision 0029).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { checkResults, codeLinks, evidenceParts, isEvidenceHeading, resultKey, riskReason, summarySentence } from "../src/evidenceFlow.ts";
import { changeLine } from "../src/historyFlow.ts";
import type { Task, TaskEvent, TaskHistory } from "../src/types.ts";

const task = (over: Partial<Task>): Task => ({ id: "A01-x", title: "x", area: "A01", state: "done", description: "", ...over }) as Task;
const event = (command: string, changes: TaskEvent["changes"]): TaskEvent => ({ at: "2026-11-20T00:00:00Z", via: "alm", command, kind: "task", changes });
const history = (events: TaskEvent[]): TaskHistory => ({ since: "2026-11-01T00:00:00Z", events });

test("a task with every record shows all six parts, in the decision's order", () => {
  const t = task({
    human_check_steps: "1. Open it.",
    handoff_note: "Done. Nothing to do.",
    links: [{ kind: "commit", ref: "abc1234", task: "A01-x", at: "2026-11-20T00:00:00Z" }],
  });
  assert.deepEqual(evidenceParts(t, true), ["ask_risk", "check_steps", "summary", "tests", "decisions", "handoff"]);
});

test("a part with nothing in it is left out; ask, risk and tests always show", () => {
  assert.deepEqual(evidenceParts(task({}), false), ["ask_risk", "tests"]);
});

test("finished without the human check: its reason shows in the check steps part", () => {
  assert.deepEqual(evidenceParts(task({ without_human_check: "only a person can try it" }), false), ["ask_risk", "check_steps", "tests"]);
});

test("the change summary shows for commits alone, with no handoff note", () => {
  const t = task({ links: [{ kind: "merge", ref: "def5678", task: "A01-x", at: "2026-11-20T00:00:00Z" }] });
  assert.deepEqual(evidenceParts(t, false), ["ask_risk", "summary", "tests"]);
});

test("the risk's reason is the newest one recorded with alm classify (the history lists newest first)", () => {
  const h = history([
    event("classify", [{ field: "risk", to: "medium" }, { field: "reason", to: "M3: words users see" }]),
    event("label", [{ field: "labels", to: ["x"] }]),
    event("classify", [{ field: "risk", to: "low" }, { field: "reason", to: "only tests" }]),
  ]);
  assert.equal(riskReason(h), "M3: words users see");
  assert.equal(riskReason(history([event("classify", [{ field: "risk", to: "low" }])])), null);
  assert.equal(riskReason(null), null);
});

test("the summary is the handoff note's first sentence", () => {
  assert.equal(summarySentence("The screen is built and merged. Nothing to do.\n\nCommit abc."), "The screen is built and merged.");
  assert.equal(summarySentence("「证据」一屏做好了，已合入。不用做什么。"), "「证据」一屏做好了，已合入。");
  assert.equal(summarySentence("\n\n# Done in v0.1.2\nmore"), "Done in v0.1.2 more");
  // A dot inside a version or file name does not end the sentence.
  assert.equal(summarySentence("Updated app.tsx for v0.1.75 today"), "Updated app.tsx for v0.1.75 today");
  assert.equal(summarySentence(null), "");
});

test("only commits and merges count as the change's code", () => {
  const t = task({
    links: [
      { kind: "branch", ref: "claude/x", task: "A01-x", at: "a" },
      { kind: "commit", ref: "abc", task: "A01-x", at: "b" },
      { kind: "release", ref: "v1", task: "A01-x", at: "c" },
      { kind: "merge", ref: "def", task: "A01-x", at: "d" },
    ],
  });
  assert.deepEqual(codeLinks(t).map((l) => l.ref), ["abc", "def"]);
});

test("the Evidence section is found as agents write its heading", () => {
  for (const s of ["Evidence", "Evidence.", "**Evidence**", "evidence (A01-x)", "证据", "証拠"]) assert.ok(isEvidenceHeading(s.replace(/\*/g, "")), s);
  for (const s of ["Evidenced elsewhere", "Outcome", "Notes"]) assert.ok(!isEvidenceHeading(s), s);
});

const result = (to: unknown, from?: unknown) => event("check-result", [{ field: "check_result", from, to }]);

test("test results: the last record of each name, failed and not run first, in the order first recorded", () => {
  // Newest first, as the app reads the history.
  const h = history([
    result({ name: "npm test", result: "pass" }, { name: "npm test", result: "fail", detail: "2 failed" }),
    result({ name: "click through", result: "not-run", detail: "no display" }),
    result({ name: "typecheck", result: "fail", detail: "1 error" }),
    result({ name: "cargo test", result: "pass" }),
    result({ name: "npm test", result: "fail", detail: "2 failed" }),
    result("not a record"),
  ]);
  assert.deepEqual(checkResults(h), [
    { name: "typecheck", result: "fail", detail: "1 error" },
    { name: "click through", result: "not-run", detail: "no display" },
    { name: "npm test", result: "pass" },
    { name: "cargo test", result: "pass" },
  ]);
});

test("a task with no test results (older tasks) gives none, so the screen points to the description", () => {
  assert.deepEqual(checkResults(history([event("label", [{ field: "labels", to: ["x"] }])])), []);
  assert.deepEqual(checkResults(null), []);
});

test("each result has its words; an unknown one has none", () => {
  assert.equal(resultKey("fail"), "evidence.result_fail");
  assert.equal(resultKey("not-run"), "evidence.result_not_run");
  assert.equal(resultKey("pass"), "evidence.result_pass");
  assert.equal(resultKey("skipped"), null);
});

test("the history shows a test result as its check and result in words", () => {
  const t = (key: string) => ({ "pane.check_result": "Test result", "evidence.result_fail": "Failed", "evidence.result_pass": "Passed" })[key] ?? key;
  const line = changeLine({ field: "check_result", from: { name: "npm test", result: "fail" }, to: { name: "npm test", result: "pass" } }, t as never);
  assert.equal(line, "Test result: npm test: Failed → npm test: Passed");
});
