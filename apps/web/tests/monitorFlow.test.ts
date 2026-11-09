// Tests for monitoring ready tasks (Task-A03-moth285): the Which tasks field's counts and first
// choice, the order tasks are taken in, when a task's session is finished, and the last run line.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  FIRST_FREQUENCY,
  FREQUENCIES,
  STALL_MS,
  firstScope,
  follow,
  frequencyText,
  idleAlert,
  lastRunCounts,
  monitorRequest,
  nextCheck,
  noticeAfterCheck,
  pick,
  readyTasks,
  scopeOptions,
  scopeReady,
  stalled,
  startRun,
  tabToClose,
  take,
  unblockNotice,
  waitingAlert,
  withReleased,
} from "../src/monitorFlow.ts";
import { toCheck } from "../src/stopFlow.ts";
import type { Iteration, Task } from "../src/types.ts";

const task = (id: string, over: Partial<Task> = {}): Task =>
  ({
    id,
    title: `Task ${id}`,
    area: id.split("-")[0],
    state: "ready",
    holder: null,
    waiting_on: null,
    next_dependency: null,
    iteration: null,
    handoff_note: null,
    without_human_check: null,
    human_check_steps: null,
    description: "",
    ...over,
  }) as Task;

const areas = [{ id: "A02" }, { id: "A01" }];
const it = (id: string, order: number): Iteration => ({ id, name: id, order });
const iterations = [it("I-knit", 2), it("I-haul", 1), it("I-cove", 3)];

const tasks = [
  task("A01-01", { iteration: "I-knit" }),
  task("A01-02", { iteration: "I-haul" }),
  task("A02-01", { iteration: "I-haul" }),
  task("A01-03", { iteration: "I-haul", holder: "claude/x", state: "in_progress" }),
  task("A01-04", { iteration: "I-haul", state: "backlog" }),
  task("A01-05"),
  task("A02-02", { iteration: "I-cove", state: "done" }),
];

test("ready tasks come in alm next's order: active iteration, plan order, unplanned last; then area, then ID", () => {
  assert.deepEqual(
    readyTasks(tasks, areas, iterations, "I-knit").map((t) => t.id),
    ["A01-01", "A02-01", "A01-02", "A01-05"],
  );
  assert.deepEqual(
    readyTasks(tasks, areas, iterations, null).map((t) => t.id),
    ["A02-01", "A01-02", "A01-01", "A01-05"],
  );
  assert.deepEqual(
    readyTasks(tasks, areas, iterations, "I-knit", "I-haul").map((t) => t.id),
    ["A02-01", "A01-02"],
  );
});

test("the counts are the ready tasks nobody holds, in each choice", () => {
  const o = scopeOptions(tasks, areas, iterations, "I-knit");
  assert.deepEqual(o.current, { id: "I-knit", ready: 1 });
  assert.equal(o.all, 4);
  assert.equal(o.other.name, "other");
  assert.equal(o.other.none, false);
  // In plan order, the active one left out; one with none ready is still listed.
  assert.deepEqual(o.other.items, [
    { place: 1, id: "I-haul", ready: 2 },
    { place: 3, id: "I-cove", ready: 0 },
  ]);
  assert.equal(scopeReady(o, { kind: "all" }), 4);
  assert.equal(scopeReady(o, { kind: "iteration", id: "I-knit", current: true }), 1);
  assert.equal(scopeReady(o, { kind: "iteration", id: "I-haul", current: false }), 2);
});

test("without an active iteration, One iteration… lists them all; without iterations, it has none", () => {
  const o = scopeOptions(tasks, areas, iterations, null);
  assert.equal(o.current, null);
  assert.equal(o.other.name, "one");
  assert.deepEqual(
    o.other.items.map((i) => i.id),
    ["I-haul", "I-knit", "I-cove"],
  );
  const none = scopeOptions([task("A01-01")], areas, [], null);
  assert.deepEqual(none.other, { name: "one", items: [], none: true });
  assert.equal(none.all, 1);
});

test("the first choice: the active iteration from its card; from the top bar only when it has ready tasks", () => {
  const active = scopeOptions(tasks, areas, iterations, "I-knit");
  assert.deepEqual(firstScope(active, "top"), { kind: "iteration", id: "I-knit", current: true });
  assert.deepEqual(firstScope(active, "iteration"), { kind: "iteration", id: "I-knit", current: true });
  const empty = scopeOptions(tasks, areas, iterations, "I-cove");
  assert.equal(empty.current?.ready, 0);
  assert.deepEqual(firstScope(empty, "top"), { kind: "all" });
  assert.deepEqual(firstScope(empty, "iteration"), { kind: "iteration", id: "I-cove", current: true });
  assert.deepEqual(firstScope(scopeOptions(tasks, areas, iterations, null), "top"), { kind: "all" });
});

test("the frequency: seven choices, 30 minutes first; hours from 60 on", () => {
  assert.deepEqual([...FREQUENCIES], [1, 5, 10, 30, 60, 90, 120]);
  assert.equal(FIRST_FREQUENCY, 30);
  assert.deepEqual(frequencyText(1), { unit: "minute", n: 1 });
  assert.deepEqual(frequencyText(5), { unit: "minutes", n: 5 });
  assert.deepEqual(frequencyText(60), { unit: "hour", n: 1 });
  assert.deepEqual(frequencyText(90), { unit: "hours", n: 1.5 });
  assert.deepEqual(frequencyText(120), { unit: "hours", n: 2 });
});

test("a run checks at once, takes tasks in order and never takes one twice", () => {
  let run = startRun({ kind: "iteration", id: "I-haul", current: false }, "claude_cli", true, 30, 1000);
  assert.ok(nextCheck(run) <= 1000);
  const first = pick(run, tasks, areas, iterations, "I-knit");
  assert.equal(first?.id, "A02-01");
  run = take(run, first!, "s-1", 2000);
  assert.equal(run.current?.id, "A02-01");
  assert.equal(nextCheck(run), 2000 + 30 * 60_000);
  // Released again by its session: ready and unheld, but not taken again in this run.
  assert.equal(pick(run, tasks, areas, iterations, "I-knit")?.id, "A01-02");
});

test("the current task is finished once claimed and no longer held, or when it leaves ready unheld", () => {
  const start = take(startRun({ kind: "all" }, "claude_cli", true, 30, 0), task("A01-05"), "s-1", 0);
  // Not claimed yet: still starting.
  let r = follow(start, [task("A01-05")], 10);
  assert.equal(r.finished, null);
  // Claimed.
  r = follow(r.run, [task("A01-05", { holder: "moth", state: "in_progress" })], 20);
  assert.equal(r.finished, null);
  assert.equal(r.run.current?.seenHeld, true);
  assert.equal(r.run.current?.changed, 20);
  // Released: ready again, held by no one.
  const released = follow(r.run, [task("A01-05")], 30);
  assert.equal(released.finished, "A01-05");
  assert.equal(released.run.current, null);
  // Waiting without ever being seen held (claimed and parked between two reads).
  assert.equal(follow(start, [task("A01-05", { state: "waiting", waiting_on: "a question" })], 5).finished, "A01-05");
  // Done, or parked, by its session: alm keeps the last holder (trial run, 2026-11-04).
  assert.equal(follow(r.run, [task("A01-05", { holder: "moth", state: "done" })], 40).finished, "A01-05");
  assert.equal(follow(r.run, [task("A01-05", { holder: "moth", state: "waiting", waiting_on: "a question" })], 40).finished, "A01-05");
  // Gone from the taskbag.
  assert.equal(follow(start, [], 5).finished, "A01-05");
});

test("only a done task's tab is closed, and only the tab Allomos opened (Task-A03-sierra014)", () => {
  const tab = { window: 30229, tty: "/dev/ttys032" };
  const run = take(startRun({ kind: "all" }, "claude_cli", true, 30, 0), task("A01-05"), "s-1", 0, tab);
  const held = { holder: "moth" };
  assert.deepEqual(tabToClose(run, [task("A01-05", { ...held, state: "done" })]), tab);
  for (const state of ["waiting", "blocked", "dropped", "ready", "in_progress"] as const) {
    assert.equal(tabToClose(run, [task("A01-05", { ...held, state })]), null, state);
  }
  // Gone from the taskbag.
  assert.equal(tabToClose(run, []), null);
  // No tab known (Windows, for now): nothing to close.
  const noTab = take(startRun({ kind: "all" }, "codex_cli", true, 30, 0), task("A01-05"), null, 0);
  assert.equal(tabToClose(noTab, [task("A01-05", { state: "done" })]), null);
});

test("a session with no sign of life for 30 minutes shows as stalled", () => {
  const run = take(startRun({ kind: "all" }, "codex_cli", false, 30, 0), task("A01-05"), null, 0);
  assert.equal(stalled(run, STALL_MS - 1), false);
  assert.equal(stalled(run, STALL_MS), true);
  const moved = follow(run, [task("A01-05", { holder: "codex/x", state: "in_progress" })], STALL_MS - 1).run;
  assert.equal(stalled(moved, STALL_MS), false);
});

test("the last run counts the tasks done and those that need the person now", () => {
  const last = { ended: "2026-11-04T10:00:00Z", scope: { kind: "all" as const }, taken: ["A01-01", "A01-02", "A01-03", "A01-04"] };
  const now = [
    task("A01-01", { state: "done", labels: ["locally verified by agent", "locally verified by human"] }),
    task("A01-02", { state: "done", labels: ["locally verified by agent"], without_human_check: "only a person can try it" }),
    task("A01-03", { state: "waiting", waiting_on: "which colour?" }),
    task("A01-04", { state: "waiting", waiting_on: "a reply", who: "the designer" }),
    task("A01-05", { state: "waiting", waiting_on: "not in this run" }),
  ];
  assert.deepEqual(lastRunCounts(last, now, toCheck), { done: 2, needYou: 2 });
});

test("each task's request names the session, the run mode and only that task", () => {
  const tb = "../allomos-taskbag/.allomos/taskbag";
  const en = monitorRequest(task("A01-05"), tb, "en", "claude_cli", "1234-abcd", true);
  assert.match(en, /--session-agent claude-code --session-id 1234-abcd --session-kind terminal/);
  assert.match(en, /Run mode: quiet/);
  assert.match(en, /alm wait A01-05 --on/);
  assert.match(en, /do not take another one/);
  assert.match(en, /--taskbag "\.\.\/allomos-taskbag\/\.allomos\/taskbag"/);
  const codex = monitorRequest(task("A01-05"), tb, "en", "codex_cli", null, false);
  assert.match(codex, /--session-agent codex --session-id <your session's ID>/);
  // Task-A03-queen969: Antigravity's agy claims under its quota id.
  const agy = monitorRequest(task("A01-05"), tb, "en", "antigravity_cli", null, false);
  assert.match(agy, /--session-agent antigravity --session-id <your session's ID>/);
  assert.match(codex, /Run mode: normal/);
  for (const language of ["zh-CN", "ja"] as const) {
    const text = monitorRequest(task("A01-05"), tb, language, "claude_cli", "1234-abcd", true);
    assert.match(text, /--session-id 1234-abcd/, language);
    assert.match(text, /alm handoff A01-05/, language);
  }
});

// Task-A03-folk351: a terminal Claude Code session has no get_usage, so its request says to read
// the quota with `claude -p /usage`; the other agents' requests do not.
test("a Claude Code session's request says how to read its quota; the others' do not", () => {
  const tb = "../allomos-taskbag/.allomos/taskbag";
  for (const language of ["en", "zh-CN", "ja"] as const) {
    const claude = monitorRequest(task("A01-05"), tb, language, "claude_cli", "1234-abcd", false);
    assert.match(claude, /claude -p \/usage/, language);
    assert.match(claude, /alm quota --agent claude-code --left .* --window 5h/, language);
    assert.match(claude, /--window week/, language);
    for (const agent of ["codex_cli", "antigravity_cli"] as const) {
      const other = monitorRequest(task("A01-05"), tb, language, agent, null, false);
      assert.doesNotMatch(other, /claude -p \/usage|alm quota/, `${language} ${agent}`);
    }
  }
});

// Task-A01-steam936: the sweep at start and on Check now. What it let go on is said in one line per
// state, the check it belongs to keeps the line and the next clears it, and until the taskbag is
// read again the monitor picks from the tasks as the sweep left them.
test("the sweep's line groups the tasks by the state they went back to", () => {
  assert.equal(unblockNotice([]), null, "nothing let go on: no line");
  const n = unblockNotice([
    { task: task("A01-02"), blockers: ["A01-01"] },
    { task: task("A01-03", { state: "in_progress", holder: "b" }), blockers: ["A01-01"] },
    { task: task("A01-04"), blockers: ["A02-01"] },
  ]);
  assert.deepEqual(n, {
    groups: [
      { state: "ready", ids: ["A01-02", "A01-04"] },
      { state: "in_progress", ids: ["A01-03"] },
    ],
    seen: false,
  });
  const seen = noticeAfterCheck(n);
  assert.equal(seen?.seen, true, "the check it belongs to keeps it");
  assert.equal(noticeAfterCheck(seen), null, "the next check clears it");
  assert.equal(noticeAfterCheck(null), null);
});

test("until the taskbag is read again, the monitor picks a task the sweep let go on", () => {
  const stale = [task("A01-01", { state: "done" }), task("A01-02", { state: "blocked" })];
  const released = [task("A01-02")];
  const run = startRun({ kind: "all" }, "claude_cli", true, 30, 0);
  const areas = [{ id: "A01" }];
  assert.equal(pick(run, stale, areas, [], null), null, "as read: still blocked");
  assert.equal(pick(run, withReleased(stale, released), areas, [], null)?.id, "A01-02");
  // Read again (here: moved on since): the taskbag as read wins.
  const read = [task("A01-01", { state: "done" }), task("A01-02", { state: "in_progress", holder: "x" })];
  assert.equal(withReleased(read, released)[1].holder, "x");
  assert.equal(withReleased(stale, []), stale);
});

// Task-A00-eclair542: the monitor tells the person when it is stuck.
test("a task its session parked waiting on the person is told; one waiting on someone named is not", () => {
  const parked = [task("A01-05", { state: "waiting", waiting_on: "Which name?" }), task("A01-06", { state: "waiting", waiting_on: "x", who: "Ann" })];
  assert.deepEqual(waitingAlert("A01-05", parked), { kind: "waiting", id: "A01-05", on: "Which name?" });
  assert.equal(waitingAlert("A01-06", parked), null);
  assert.equal(waitingAlert(null, parked), null);
  assert.equal(waitingAlert("A01-01", tasks), null);
});

test("no task ready is told once per stretch, and again after a task was taken", () => {
  const run = startRun({ kind: "all" }, "claude_cli", true, 30, 0);
  const first = idleAlert(run);
  assert.deepEqual(first.alert, { kind: "idle" });
  assert.equal(idleAlert(first.run).alert, null);
  const taken = take(first.run, task("A01-05"), "s-1", 0);
  assert.deepEqual(idleAlert(taken).alert, { kind: "idle" });
});
