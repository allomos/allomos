// Tests for monitoring ready tasks (Task-A03-moth285): the Which tasks field's counts and first
// choice, the last run line, and reading a run from its file (Task-A03-cow112: the loop itself is
// `alm monitor run`, tested in crates/monitor).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  ALL_LEVELS,
  FIRST_FREQUENCY,
  FREQUENCIES,
  STALL_MS,
  firstScope,
  freeAgents,
  frequencyText,
  isRun,
  lastRunCounts,
  levelsOf,
  levelsText,
  nextCheck,
  parseLevels,
  readEntry,
  readyTasks,
  scopeOptions,
  scopeReady,
  sessionsOf,
  setAgents,
  stalled,
  unblockNotice,
} from "../src/monitorFlow.ts";
import { toCheck } from "../src/stopFlow.ts";
import { levelsHold } from "../src/difficultyFlow.ts";
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

test("the last run counts the tasks done and those that need the person now", () => {
  const last = { ended: "2026-11-04T10:00:00Z", scope: { kind: "all" as const }, taken: ["A01-01", "A01-02", "A01-03", "A01-04"] };
  const now = [
    task("A01-01", { state: "done", labels: ["locally verified by agent", "locally verified by human"] }),
    task("A01-02", { state: "done", labels: ["locally verified by agent"], without_human_check: "only a person can try it" }),
    task("A01-03", { state: "waiting", waiting_on: "which colour?" }),
    task("A01-04", { state: "waiting", waiting_on: "a reply", who: "the designer" }),
    task("A01-05", { state: "waiting", waiting_on: "not in this run" }),
  ];
  assert.deepEqual(lastRunCounts(last, now, toCheck), { done: 2, needYou: 2, byAgent: [] });
});

test("the last run counts the done tasks per agent that gave them out", () => {
  const last = {
    ended: "2026-11-09T10:00:00Z",
    scope: { kind: "all" as const },
    taken: ["A01-01", "A01-02", "A01-03"],
    takenBy: { "A01-01": "claude_cli" as const, "A01-02": "codex_cli" as const, "A01-03": "claude_cli" as const },
  };
  const now = [
    task("A01-01", { state: "done", labels: ["locally verified by agent"] }),
    task("A01-02", { state: "done", labels: ["locally verified by agent"] }),
    task("A01-03", { state: "done", labels: ["locally verified by agent"] }),
  ];
  assert.deepEqual(lastRunCounts(last, now, toCheck).byAgent, [
    { agent: "claude_cli", done: 2 },
    { agent: "codex_cli", done: 1 },
  ]);
});

test("a run's sessions per agent: one unless set, and a free slot for each session it may still start", () => {
  const r = readEntry(entry({ agents: ["claude_cli", "codex_cli"], sessions: { claude_cli: 2, codex_cli: 0, gemini: 3 } }), null);
  assert.ok(r && isRun(r));
  assert.deepEqual(r.sessions, { claude_cli: 2 }, "a count below one and an unknown agent are left out");
  assert.equal(sessionsOf(r, "claude_cli"), 2);
  assert.equal(sessionsOf(r, "codex_cli"), 1);
  // claude_cli has one session at work (the entry's): one more slot for it, and one for codex.
  assert.deepEqual(freeAgents(r), ["claude_cli", "codex_cli"]);
});

test("each agent's difficulty range: every level unless set; a range the run file does not read is left out", () => {
  const r = readEntry(
    entry({ agents: ["claude_cli", "codex_cli"], difficulties: { claude_cli: "3-5", codex_cli: "4", antigravity_cli: "1-2", gemini: "1-2" }, no_task: ["claude_cli", "gemini"] }),
    null,
  );
  assert.ok(r && isRun(r));
  assert.deepEqual(r.difficulties, {
    claude_cli: { low: 3, high: 5 },
    codex_cli: { low: 4, high: 4 },
    antigravity_cli: { low: 1, high: 2 },
  }, "an agent that is not a terminal agent is left out");
  assert.deepEqual(levelsOf(r, "claude_cli"), { low: 3, high: 5 });
  assert.deepEqual(levelsOf(r, "gemini" as never), ALL_LEVELS);
  assert.deepEqual(r.noTask, ["claude_cli"], "an agent that is not a terminal agent is left out of the list too");
  assert.equal(levelsText({ low: 4, high: 4 }), "4");
  assert.equal(levelsText({ low: 1, high: 5 }), "1-5");
});

test("a task is in an agent's range when its level (the default 3 included) is", () => {
  const leveled = (difficulty?: number) => ({ difficulty: difficulty ?? null }) as Pick<Task, "difficulty">;
  assert.equal(levelsHold({ low: 1, high: 2 }, leveled(2)), true);
  assert.equal(levelsHold({ low: 1, high: 2 }, leveled(3)), false);
  assert.equal(levelsHold({ low: 3, high: 5 }, leveled()), true, "no level counts as 3");
  assert.equal(levelsHold({ low: 4, high: 5 }, leveled()), false);
});

test("a difficulty range reads as low-high or one level, both ends from 1 to 5, the low not above the high", () => {
  assert.deepEqual(parseLevels("1-5"), { low: 1, high: 5 });
  assert.deepEqual(parseLevels(" 2 "), { low: 2, high: 2 });
  assert.equal(parseLevels("4-2"), null);
  assert.equal(parseLevels("0-3"), null);
  assert.equal(parseLevels("6"), null);
  assert.equal(parseLevels(3), null);
});

/** A run's entry as `alm monitor status --json` lists it. */
const entry = (over: Record<string, unknown> = {}) => ({
  record: "present",
  id: "m-1",
  pid: 4242,
  next_check: 1_900_000,
  run: {
    scope: { kind: "iteration", id: "I-haul" },
    agents: ["claude_cli", "codex_cli"],
    offered: ["claude_cli", "codex_cli", "antigravity_cli"],
    quiet: true,
    every: 30,
    started: 0,
    taken: ["A01-02"],
    working: [{ agent: "claude_cli", id: "A01-02", session_id: "s-1", tab: { window: 7, tty: "/dev/ttys004" }, at: 10, seen_held: true, seen: "", changed: 20 }],
    stopping: false,
    checked: 100,
    unblocked: [{ id: "A01-03", state: "ready" }],
    ...over,
  },
});

test("a run is read from its file: scope, agents, sessions at work, next check", () => {
  const r = readEntry(entry(), "I-haul");
  assert.ok(r && isRun(r));
  assert.deepEqual(r.scope, { kind: "iteration", id: "I-haul", current: true });
  assert.equal(nextCheck(r), 1_900_000);
  assert.deepEqual(r.working, [{ agent: "claude_cli", id: "A01-02", sessionId: "s-1", tab: { window: 7, tty: "/dev/ttys004" }, at: 10, changed: 20 }]);
  assert.deepEqual(freeAgents(r), ["codex_cli"]);
  assert.deepEqual(readEntry(entry(), "I-knit") && (readEntry(entry(), "I-knit") as { scope: unknown }).scope, { kind: "iteration", id: "I-haul", current: false });
  // An agent alm does not know and a frequency the app does not offer are left out.
  const odd = readEntry(entry({ agents: ["claude_cli", "gemini"], every: 7 }), null);
  assert.ok(odd && isRun(odd));
  assert.deepEqual(odd.agents, ["claude_cli"]);
  assert.equal(odd.every, FIRST_FREQUENCY);
  assert.equal(readEntry({ nonsense: true }, null), null);
});

test("a process with no run file is read as running, record missing", () => {
  const l = readEntry({ record: "missing", pid: 77, id: "m-2", taskbag: null }, null);
  assert.ok(l && !isRun(l));
  assert.deepEqual(l, { pid: 77, id: "m-2" });
});

test("a session with no sign of life for 30 minutes shows as stalled", () => {
  const r = readEntry(entry(), null);
  assert.ok(r && isRun(r));
  assert.deepEqual(stalled(r, 20 + STALL_MS - 1), []);
  assert.deepEqual(stalled(r, 20 + STALL_MS).map((c) => c.id), ["A01-02"]);
});

test("agents added or taken off: in the order offered, and never none", () => {
  const r = readEntry(entry(), null);
  assert.ok(r && isRun(r));
  assert.deepEqual(setAgents(r, ["antigravity_cli", "claude_cli"]), ["claude_cli", "antigravity_cli"]);
  assert.deepEqual(setAgents(r, []), ["claude_cli", "codex_cli"]);
});

test("the sweep's line groups the tasks by the state they went back to", () => {
  assert.equal(unblockNotice([]), null, "nothing let go on: no line");
  assert.deepEqual(
    unblockNotice([
      { id: "A01-02", state: "ready" },
      { id: "A01-03", state: "in_progress" },
      { id: "A01-04", state: "ready" },
    ]),
    {
      groups: [
        { state: "ready", ids: ["A01-02", "A01-04"] },
        { state: "in_progress", ids: ["A01-03"] },
      ],
    },
  );
});
