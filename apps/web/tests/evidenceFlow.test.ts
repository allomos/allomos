// Tests for the Evidence screen of a task to check (Task-A01-herb903, decision 0029).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { changeTotals, codeLinks, commitSubject, evidenceParts, isEvidenceHeading } from "../src/evidenceFlow.ts";
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

test("what changed: files and lines add up; binary files count as files only (Task-A01-hyena938)", () => {
  const files = [
    { path: "a.rs", added: 10, removed: 2 },
    { path: "logo.png", added: null, removed: null },
    { path: "b.ts", added: 1, removed: 0 },
  ];
  assert.deepEqual(changeTotals(files), { files: 3, added: 11, removed: 2 });
  assert.deepEqual(changeTotals([]), { files: 0, added: 0, removed: 0 });
});

test("a recorded commit found in git has its first line; one not on this computer is null; unread is undefined", () => {
  const changes = {
    commits: [
      { hash: "989bed1a", subject: "A01-herb903: Evidence screen", merge: false },
      { hash: "0123456789abcdef0123456789abcdef01234567", subject: null, merge: false },
    ],
    files: [],
  };
  assert.equal(commitSubject(changes, "989bed1a"), "A01-herb903: Evidence screen");
  assert.equal(commitSubject(changes, "0123456789abcdef0123456789abcdef01234567"), null);
  assert.equal(commitSubject(changes, "fffffff"), undefined);
  assert.equal(commitSubject(null, "989bed1a"), undefined);
});
