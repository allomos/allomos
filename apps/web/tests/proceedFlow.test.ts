// Tests for Work on it… (Task-A03-galley051): when a row offers it, and the request Allomos types in.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { canProceed, canTakeOver, edited, proceedRequest, stopped, takeOverRequest } from "../src/proceedFlow.ts";
import type { Task } from "../src/types.ts";

const TB = "../allomos-taskbag/.allomos/taskbag";

const task = (over: Partial<Task>): Task => ({
  id: "A01-41",
  title: "Search finds words in the handoff note",
  area: "A01",
  state: "ready",
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

// Take over… (Task-A01-voyage603): the request for a lost task.
const lostTask = () => task({ id: "A01-50", title: "One", state: "in_progress", holder: "claude/s1" });

test("a lost task's take-over request, word for word", () => {
  assert.equal(
    takeOverRequest(lostTask(), TB, "en", { lostAfter: 30, working: "claude/one" }),
    `Take over the lost task A01-50 ("One") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

claude/s1 holds it, but has sent no heartbeat for over 30 minutes and has no busy window running: it is lost.

1. Read it: alm show A01-50 (its description and latest handoff note).
2. Claim it as AGENTS.md says; that is the takeover (alm claim A01-50 --as <your session name> --branch <your branch> ...), and share the claim.
3. Go on on its working branch, claude/one: open a worktree of your own for it (git worktree add <folder> claude/one, without -b).
4. Only what was pushed counts: go on from the commits on the working branch and the handoff note. The former session's worktree (the claim prints where) may hold changes it never committed, but that folder is of use only on its own computer: on the same computer, look at them if you need to, but do not change, commit or delete anything there; on another computer, go by the working branch and the handoff note. If unsure, ask me.
5. Report progress with alm label; finish with alm done, or stop with alm release --note.`,
  );
});

test("the take-over request without a recorded working branch, and in each language", () => {
  const en = takeOverRequest(lostTask(), TB, "en", { lostAfter: 45, working: null });
  assert.match(en, /over 45 minutes/);
  assert.match(en, /its working branch \(the claim prints it\)/);
  assert.match(en, /git worktree add <folder> <working branch>, without -b/);
  for (const language of ["zh-CN", "ja"] as const) {
    const r = takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: "claude/one" });
    assert.match(r, /alm claim A01-50 --as/, language);
    assert.match(r, /git worktree add <[^>]+> claude\/one/, language);
    assert.match(r, /claude\/s1/, language);
    assert.match(r, /--taskbag "\.\.\/allomos-taskbag\/\.allomos\/taskbag"/, language);
    assert.doesNotMatch(r, /-b claude/, language);
  }
});

// Take over… on a holder that is not lost (Task-A01-rocket260): the person asks, so the request uses --take.
test("Take over… can be offered for a held task that is not finished, in any state", () => {
  for (const state of ["in_progress", "waiting", "blocked"] as const) assert.equal(canTakeOver(task({ state, holder: "claude/s1" })), true, state);
  for (const state of ["done", "dropped"] as const) assert.equal(canTakeOver(task({ state, holder: "claude/s1" })), false, state);
  assert.equal(canTakeOver(task({ state: "in_progress", holder: null })), false);
});

test("taking over a holder that is not lost, word for word", () => {
  assert.equal(
    takeOverRequest(lostTask(), TB, "en", { lostAfter: null, working: "claude/one" }),
    `Take over task A01-50 ("One") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

claude/s1 holds it and is not lost: its session may still be at work. I asked for this takeover in Allomos, so you may use --take.

1. Read it: alm show A01-50 (its description and latest handoff note).
2. Claim it as AGENTS.md says, with --take; that is the takeover (alm claim A01-50 --take --as <your session name> --branch <your branch> ...), and share the claim.
3. Go on on its working branch, claude/one: open a worktree of your own for it (git worktree add <folder> claude/one, without -b).
4. Only what was pushed counts: go on from the commits on the working branch and the handoff note. The former session's worktree (the claim prints where) may hold changes it never committed, but that folder is of use only on its own computer: on the same computer, look at them if you need to, but do not change, commit or delete anything there; on another computer, go by the working branch and the handoff note. If unsure, ask me.
5. That session learns of the takeover only the next time it runs alm, so it may still push to the branch until then: get the latest changes before you commit, and if its commits collide with yours, tell me.
6. Report progress with alm label; finish with alm done, or stop with alm release --note.`,
  );
});

test("taking over a holder that is not lost, in Chinese and Japanese, and a lost holder gets no --take", () => {
  for (const language of ["zh-CN", "ja"] as const) {
    const r = takeOverRequest(lostTask(), TB, language, { lostAfter: null, working: "claude/one" });
    assert.match(r, /alm claim A01-50 --take --as/, language);
    assert.match(r, language === "zh-CN" ? /「换肩」/ : /「肩代」/, language);
    assert.match(r, /\n6\. /, language);
    assert.doesNotMatch(r, /null/, language);
  }
  for (const language of ["en", "zh-CN", "ja"] as const) {
    const r = takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: "claude/one" });
    assert.doesNotMatch(r, /--take/, language);
    assert.doesNotMatch(r, /\n6\. /, language);
  }
});

// Saving what the former worktree has not pushed (Task-A01-flag101).
const former = {
  path: "/work/project/.claude/worktrees/one",
  files: [
    { kind: "modified" as const, path: "src/a.ts" },
    { kind: "untracked" as const, path: "notes.md" },
  ],
  commits: [{ hash: "abc1234", subject: "WIP A01-50: half" }],
};

test("a lost task's request with the former worktree's changes saved first, word for word", () => {
  assert.equal(
    takeOverRequest(lostTask(), TB, "en", { lostAfter: 30, working: "claude/one", save: former }),
    `Take over the lost task A01-50 ("One") in this project's taskbag. Run every alm command with --taskbag "../allomos-taskbag/.allomos/taskbag".

claude/s1 holds it, but has sent no heartbeat for over 30 minutes and has no busy window running: it is lost.

1. Read it: alm show A01-50 (its description and latest handoff note).
2. Claim it as AGENTS.md says; that is the takeover (alm claim A01-50 --as <your session name> --branch <your branch> ...), and share the claim.
3. I chose "Commit and push these changes for it first" in Allomos, so this once you may change the former holder's worktree: /work/project/.claude/worktrees/one. There, commit the files it has not committed (src/a.ts, notes.md) as one WIP commit (git -C "/work/project/.claude/worktrees/one" add -A, then git -C "/work/project/.claude/worktrees/one" commit -m "WIP: changes saved for claude/s1 before the takeover (A01-50)", with your attribution line), then push the working branch claude/one with the 1 commit it committed but did not push to origin (git -C "/work/project/.claude/worktrees/one" push origin claude/one), and make sure it is not "ahead" (git -C "/work/project/.claude/worktrees/one" status -sb). Apart from that, change or delete nothing there. If the changes you find differ from what I saw in Allomos, ask me first.
4. Go on on its working branch, claude/one: open a worktree of your own for it (git worktree add <folder> claude/one, without -b).
5. Only what was pushed counts: go on from the commits on the working branch and the handoff note. Step 3 pushed what the former session's worktree had not; apart from step 3, do not change, commit or delete anything there. If unsure, ask me.
6. Report progress with alm label; finish with alm done, or stop with alm release --note.`,
  );
});

test("saving first: only commits to push, a forced takeover, each language, and nothing to save", () => {
  const pushOnly = takeOverRequest(lostTask(), TB, "en", { lostAfter: 30, working: "claude/one", save: { ...former, files: [] } });
  assert.doesNotMatch(pushOnly, /add -A/);
  assert.match(pushOnly, /There, push the working branch claude\/one with the 1 commit/);
  const filesOnly = takeOverRequest(lostTask(), TB, "en", { lostAfter: 30, working: "claude/one", save: { ...former, commits: [] } });
  assert.match(filesOnly, /then push the working branch claude\/one to origin/);
  for (const language of ["en", "zh-CN", "ja"] as const) {
    const forced = takeOverRequest(lostTask(), TB, language, { lostAfter: null, working: "claude/one", save: former });
    assert.match(forced, /alm claim A01-50 --take --as/, language);
    assert.match(forced, /\n3\. [^\n]*\/work\/project\/\.claude\/worktrees\/one[^\n]*push origin claude\/one/, language);
    assert.match(forced, /\n7\. /, language);
    assert.match(forced, /src\/a\.ts/, language);
    // Nothing to save, no working branch, or not chosen: the request is as before.
    const plain = takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: "claude/one" });
    assert.equal(takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: "claude/one", save: { ...former, files: [], commits: [] } }), plain, language);
    assert.equal(takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: "claude/one", save: null }), plain, language);
    assert.doesNotMatch(takeOverRequest(lostTask(), TB, language, { lostAfter: 30, working: null, save: former }), /worktrees\/one/, language);
  }
  const zh = takeOverRequest(lostTask(), TB, "zh-CN", { lostAfter: 30, working: "claude/one", save: former });
  assert.match(zh, /「先替它提交并推送这些改动」/);
  assert.match(zh, /WIP: 接手前替 claude\/s1 保存的改动（A01-50）/);
  const ja = takeOverRequest(lostTask(), TB, "ja", { lostAfter: 30, working: "claude/one", save: former });
  assert.match(ja, /「先にこれらの変更を代わりにコミットしてプッシュする」/);
});

test("a long file list names thirty files and says how many more", () => {
  const many = { ...former, files: Array.from({ length: 33 }, (_, i) => ({ kind: "modified" as const, path: `f${i}.ts` })) };
  const r = takeOverRequest(lostTask(), TB, "en", { lostAfter: 30, working: "claude/one", save: many });
  assert.match(r, /f29\.ts and 3 more\)/);
  assert.doesNotMatch(r, /f30\.ts/);
});
