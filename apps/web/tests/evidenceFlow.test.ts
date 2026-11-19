// Tests for the Evidence screen of a task to check (Task-A01-herb903, decision 0029).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { codeLinks, evidenceParts, isEvidenceHeading } from "../src/evidenceFlow.ts";
import { decorated } from "../src/stopFlow.ts";
import type { Opened, Task } from "../src/types.ts";

const task = (over: Partial<Task>): Task => ({ id: "A01-x", title: "x", area: "A01", state: "done", description: "", ...over }) as Task;

// Which parts a task has and their order are worked out in the core (crates/core/src/evidence.rs,
// Task-A01-inlet632) and tested there; the app draws what it receives.
test("the parts come from the core, in the order it sent them", () => {
  const parts = ["ask_risk", "check_steps", "summary", "tests", "decisions", "handoff"] as const;
  const opened = { evidence: { "A01-x": { parts: [...parts], summary: "Done.", risk_reason: "M3" } } } as unknown as Opened;
  const t = decorated(task({}), opened);
  assert.deepEqual(evidenceParts(t), parts);
  assert.equal(t.evidence?.summary, "Done.");
  assert.equal(t.evidence?.risk_reason, "M3");
});

test("before the core has sent them, only the ask and the tests show", () => {
  assert.deepEqual(evidenceParts(task({})), ["ask_risk", "tests"]);
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
