// Tests for Hand over to another agent… (Task-A01-minnow200): which step the dialog is on, which
// request is in force, and the requests Allomos writes.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { handoverAskRequest, handoverClaimRequest, handoverLate, handoverStep, pendingHandover } from "../src/handoverFlow.ts";
import { quotaOfChoice } from "../src/quotaFlow.ts";
import type { HandoverRequest, QuotaReport, Task } from "../src/types.ts";

const TB = "../allomos-taskbag/.allomos/taskbag";
const REQUEST: HandoverRequest = { holder: "s1", by: "will", at: "2026-11-11T06:00:00Z", to: "codex_cli" };

const task = (over: Partial<Task>): Task =>
  ({
    id: "A01-41",
    title: "Search finds words",
    area: "A01",
    state: "in_progress",
    holder: "s1",
    waiting_on: null,
    next_dependency: null,
    iteration: null,
    handoff_note: null,
    without_human_check: null,
    ...over,
  }) as Task;

test("a request is in force only for the holder it names, on an open task", () => {
  assert.equal(pendingHandover(task({})), null);
  assert.deepEqual(pendingHandover(task({ handover_request: REQUEST })), REQUEST);
  assert.equal(pendingHandover(task({ holder: "s2", handover_request: REQUEST })), null);
  assert.equal(pendingHandover(task({ state: "done", handover_request: REQUEST })), null);
  assert.equal(pendingHandover(task({ state: "dropped", holder: null, handover_request: REQUEST })), null);
});

test("the dialog goes from choosing to waiting to opening", () => {
  assert.equal(handoverStep(task({}), null), "choose");
  assert.equal(handoverStep(task({}), "s1"), "wait");
  assert.equal(handoverStep(task({ state: "ready", holder: null }), "s1"), "open");
});

test("a new holder or a finished task ends the dialog's work", () => {
  assert.equal(handoverStep(task({ holder: "s2" }), "s1"), "gone");
  assert.equal(handoverStep(task({ state: "done" }), "s1"), "gone");
  assert.equal(handoverStep(task({ state: "dropped", holder: null }), "s1"), "gone");
});

test("Take over… is suggested only after a full heartbeat interval", () => {
  assert.equal(handoverLate(REQUEST, 10, new Date("2026-11-11T06:09:59Z")), false);
  assert.equal(handoverLate(REQUEST, 10, new Date("2026-11-11T06:10:01Z")), true);
  assert.equal(handoverLate(null, 10, new Date("2026-11-11T07:00:00Z")), false);
});

test("the quota shown is the agent's newest 5-hour report", () => {
  const r = (agent: string, window: string, left: number, at: string): QuotaReport => ({ agent, window, left, at });
  const reports = [
    r("claude-code", "5h", 80, "2026-11-11T05:00:00Z"),
    r("claude-code", "5h", 60, "2026-11-11T06:00:00Z"),
    r("claude-code", "week", 10, "2026-11-11T07:00:00Z"),
    r("codex", "5h", 5, "2026-11-11T06:00:00Z"),
  ];
  assert.equal(quotaOfChoice(reports, "claude_cli")?.left, 60);
  assert.equal(quotaOfChoice(reports, "claude_app")?.left, 60);
  assert.equal(quotaOfChoice(reports, "codex_app")?.left, 5);
  assert.equal(quotaOfChoice(reports, "antigravity_cli"), null);
});

test("the request to the holder, word for word", () => {
  assert.equal(
    handoverAskRequest(task({}), "en"),
    `Please hand task A01-41 ("Search finds words") over to another agent: I asked for this in Allomos.

1. Finish the small step you are on and start nothing new on it.
2. Write your handoff: alm handoff A01-41 --as <your session name> --text "<where it stopped, the one next step, who it waits on>".
3. Commit and push your work to the working branch.
4. Release the task: alm release A01-41 --as <your session name> --note "<what the next one needs, and the working branch>", and share that change.
5. Then stop work on it: a new session takes it from there.`,
  );
});

test("the new session claims without --take, on the working branch", () => {
  for (const language of ["en", "zh-CN", "ja"] as const) {
    const text = handoverClaimRequest(task({ state: "ready", holder: null }), TB, language, "s1", "claude/x");
    assert.ok(text.includes("alm show A01-41"), language);
    assert.ok(text.includes("--working claude/x"), language);
    assert.ok(text.includes("git worktree add") && text.includes("claude/x"), language);
    assert.ok(text.includes("s1"), language);
    assert.ok(text.includes(TB), language);
    assert.ok(!text.includes("--take"), language);
  }
  const none = handoverClaimRequest(task({ state: "ready", holder: null }), TB, "en", "s1", null);
  assert.ok(!none.includes("--working"));
  assert.ok(none.includes("<working branch>"));
});
