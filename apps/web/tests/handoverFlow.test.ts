// Tests for Prepare a handoff… (Task-A01-kind858): whether the holder's session can be opened, and
// the prompt the person pastes into it.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { handoverAskRequest, handoverOpen } from "../src/handoverFlow.ts";
import type { Task } from "../src/types.ts";

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

test("the holder's session opens only from the desktop app, when the claim recorded it", () => {
  assert.deepEqual(handoverOpen(task({ session: { by: "s1", id: "u1" } }), true), { can: true });
  assert.deepEqual(handoverOpen(task({ session: { by: "s1", app_session_id: "local_1" } }), true), { can: true });
  assert.deepEqual(handoverOpen(task({ session: { id: "u1" } }), true), { can: true });
  assert.deepEqual(handoverOpen(task({ session: { by: "s1", id: "u1" } }), false), { can: false, why: "no_app" });
  assert.deepEqual(handoverOpen(task({}), true), { can: false, why: "not_recorded" });
  assert.deepEqual(handoverOpen(task({ session: { by: "s1", id: "  " } }), true), { can: false, why: "not_recorded" });
  assert.deepEqual(handoverOpen(task({ session: { by: "s0", id: "u0" } }), true), { can: false, why: "other_session" });
});

test("the prompt for the holder, word for word", () => {
  assert.equal(
    handoverAskRequest(task({}), "en"),
    `Please get task A01-41 ("Search finds words") ready for a handoff: I asked for this in Allomos.

1. Finish the small step you are on and start nothing new on it.
2. Write your handoff: alm handoff A01-41 --as <your session name> --text "<where it stopped, the one next step, who it waits on>".
3. Commit and push your work to the working branch.
4. Release the task: alm release A01-41 --as <your session name> --note "<what the next one needs, and the working branch>", and share that change.
5. Then stop work on it: I will start whoever goes on with it separately.`,
  );
});

test("the prompt asks for handoff, push and release in every language, and names no next agent", () => {
  for (const language of ["en", "zh-CN", "ja"] as const) {
    const text = handoverAskRequest(task({}), language);
    assert.ok(text.includes("alm handoff A01-41"), language);
    assert.ok(text.includes("alm release A01-41"), language);
    assert.ok(!text.includes("--taskbag"), language);
    assert.ok(!/another agent|另一个智能体|別のエージェント/.test(text), language);
  }
});
