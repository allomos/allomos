// Tests for Work on it… (Task-A03-galley051): when a row offers it, and the request Allomos types in.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canProceed, edited, proceedRequest, stopped } from "../src/proceedFlow.ts";
import type { Task } from "../src/types.ts";

const TB = "../allomos-taskbag/.allomos/taskbag";

const task = (over: Partial<Task>): Task => ({
  id: "A01-41",
  title: "Search finds words in the handoff note",
  area: "A01",
  state: "ready",
  label: null,
  holder: null,
  waiting_on: null,
  next_dependency: null,
  iteration: null,
  handoff_note: null,
  without_human_check: null,
  ...over,
} as Task);

test("Work on it… shows for unheld tasks that are not finished", () => {
  for (const state of ["backlog", "ready", "blocked", "waiting"] as const) assert.equal(canProceed(task({ state })), true, state);
  for (const state of ["done", "dropped"] as const) assert.equal(canProceed(task({ state })), false, state);
  assert.equal(canProceed(task({ holder: "claude/x" })), false);
  assert.equal(stopped(task({ state: "blocked" })), true);
  assert.equal(stopped(task({ state: "waiting" })), true);
  assert.equal(stopped(task({ state: "backlog" })), false);
});

test("a ready task's request, word for word", () => {
  assert.equal(
    proceedRequest(task({}), TB, "en"),
    `Work on task A01-41 ("Search finds words in the handoff note") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

1. Read it: alm show A01-41 (its description and latest handoff note).
2. Claim it as AGENTS.md says (alm claim A01-41 --as <your session name> --branch <your branch> ...), and share the claim.
3. Work it in a worktree of its own, cut from the base branch the claim prints.
4. Report progress with alm label; finish with alm done, or stop with alm release --note.`,
  );
});

test("a backlog task's request makes it ready first", () => {
  assert.equal(
    proceedRequest(task({ id: "A01-44", title: "A task row offers to proceed with an AI agent", state: "backlog" }), TB, "en"),
    `Work on task A01-44 ("A task row offers to proceed with an AI agent") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

1. Read it: alm show A01-44 (its description and latest handoff note).
2. It is in backlog. I chose Work on it in Allomos, which approves it: make it ready (alm ready A01-44).
3. Claim it as AGENTS.md says (alm claim A01-44 --as <your session name> --branch <your branch> ...), and share the claim.
4. Work it in a worktree of its own, cut from the base branch the claim prints.
5. Report progress with alm label; finish with alm done, or stop with alm release --note.`,
  );
});

test("a blocked task's request asks the person", () => {
  assert.equal(
    proceedRequest(task({ id: "A01-45", title: "Sort by modified keeps the open task in view", state: "blocked", blocked_by: ["A01-41"] }), TB, "en"),
    `Look at task A01-45 ("Sort by modified keeps the open task in view") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

It cannot start now: it is blocked by A01-41. Do not claim it and do not change it yet.

1. Read it: alm show A01-45 and alm show A01-41.
2. Tell me in plain words why it is stopped and what could be done about it (for example: work A01-41 first, or unblock A01-45 if A01-41 is already done).
3. Ask me what I want you to do with A01-45, and wait for my answer before you change anything.`,
  );
  const two = proceedRequest(task({ id: "A01-45", state: "blocked", blocked_by: ["A01-41", "A01-42"] }), TB, "en");
  assert.match(two, /blocked by A01-41 and A01-42\./);
  assert.match(two, /Read it: alm show A01-45, alm show A01-41 and alm show A01-42\./);
  assert.match(two, /if A01-41 and A01-42 are already done/);
});

test("a waiting task's request asks the person", () => {
  assert.equal(
    proceedRequest(
      task({ id: "A01-46", title: "The burndown shows a forecast finish date", state: "waiting", waiting_on: "The approver: which week the forecast starts from" }),
      TB,
      "en",
    ),
    `Look at task A01-46 ("The burndown shows a forecast finish date") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

It cannot start now: it is waiting on: The approver: which week the forecast starts from. Do not claim it and do not change it yet.

1. Read it: alm show A01-46.
2. Tell me in plain words why it is stopped and what could be done about it (for example: answer the question it waits on, or ask the person it waits on).
3. Ask me what I want you to do with A01-46, and wait for my answer before you change anything.`,
  );
});

test("Chinese and Japanese requests keep the commands, steps and product name", () => {
  for (const language of ["zh-CN", "ja"] as const) {
    const ready = proceedRequest(task({}), TB, language);
    assert.match(ready, /--taskbag "\.\.\/allomos-taskbag\/\.allomos\/taskbag"/);
    assert.match(ready, /alm claim A01-41 --as/);
    assert.match(ready, /\n4\. /);
    const backlog = proceedRequest(task({ state: "backlog" }), TB, language);
    assert.match(backlog, /alm ready A01-41/);
    assert.match(backlog, language === "zh-CN" ? /「换肩」/ : /「肩代」/);
    assert.match(backlog, /\n5\. /);
    assert.match(proceedRequest(task({ state: "blocked", blocked_by: ["A01-40"] }), TB, language), /alm show A01-40/);
    assert.match(proceedRequest(task({ state: "waiting", waiting_on: "Pick a week" }), TB, language), /Pick a week/);
  }
});

test("a request edited back to Allomos's counts as not edited", () => {
  assert.equal(edited("same", "same"), null);
  assert.equal(edited("mine", "same"), "mine");
});
