// Tests for editing a running monitor and its panel (A03-yolk481): the requests a saved Edit
// monitor… dialog sends, the panel's state badge and the section's counts, the sessions grouped by
// agent, and the folds this computer remembers.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  STALL_MS,
  editRequests,
  grouped,
  keptFolds,
  readEntry,
  readFolds,
  runState,
  sessionGroups,
  stateCounts,
  workedFor,
} from "../src/monitorFlow.ts";
import type { MonitorSettings, Run } from "../src/monitorFlow.ts";
import type { Task } from "../src/types.ts";

const run = (over: Record<string, unknown> = {}): Run =>
  readEntry(
    {
      record: "present",
      id: "m-1",
      pid: 1,
      next_check: 1_000,
      run: {
        scope: { kind: "iteration", id: "I-sand" },
        agents: ["claude_cli"],
        offered: ["claude_cli", "codex_cli"],
        quiet: true,
        every: 30,
        started: 0,
        taken: ["A01-1"],
        working: [{ agent: "claude_cli", id: "A01-1", at: 0, changed: 0 }],
        stopping: false,
        checked: 0,
        ...over,
      },
    },
    "I-sand",
  ) as Run;

const settings = (r: Run, over: Partial<MonitorSettings> = {}): MonitorSettings => ({
  scope: r.scope,
  agents: r.agents,
  quiet: r.quiet,
  every: r.every,
  sessions: Object.fromEntries(r.agents.map((a) => [a, r.sessions[a] ?? 1])),
  difficulties: {},
  ...over,
});

const task = (id: string, over: Partial<Task> = {}): Task => ({ id, title: id, area: "A01", state: "in_progress", labels: [], ...over }) as Task;

test("saving the dialog unchanged sends nothing", () => {
  const r = run();
  assert.deepEqual(editRequests(r, settings(r)), []);
});

test("each change is one request, in the order the run takes them", () => {
  const r = run();
  const reqs = editRequests(
    r,
    settings(r, {
      scope: { kind: "all" },
      quiet: false,
      every: 5,
      agents: ["claude_cli", "codex_cli"],
      sessions: { claude_cli: 2, codex_cli: 1 },
      difficulties: { codex_cli: { low: 1, high: 2 } },
    }),
  );
  assert.deepEqual(reqs, [
    { action: "scope", scope: { kind: "all" } },
    { action: "mode", quiet: false },
    { action: "every", minutes: 5 },
    { action: "agents", agents: ["claude_cli", "codex_cli"] },
    { action: "sessions", agent: "claude_cli", count: 2 },
    { action: "difficulty", agent: "codex_cli", range: "1-2" },
  ]);
  // Another iteration: its ID, without the "current" flag.
  assert.deepEqual(editRequests(r, settings(r, { scope: { kind: "iteration", id: "I-two", current: false } })), [
    { action: "scope", scope: { kind: "iteration", id: "I-two" } },
  ]);
  // No agent left: the agents stay (Stop ends a run).
  assert.deepEqual(editRequests(r, settings(r, { agents: [] })), []);
});

test("one state per monitor, stalled first, then needs you, stopping, running", () => {
  const now = STALL_MS + 10;
  assert.equal(runState(run({ working: [{ agent: "claude_cli", id: "A01-1", at: 0, changed: now }] }), [], now), "running");
  assert.equal(runState(run(), [], now), "stalled");
  const fresh = { working: [{ agent: "claude_cli", id: "A01-1", at: 0, changed: now }] };
  assert.equal(runState(run({ ...fresh, taken: ["A01-1", "A01-0"] }), [task("A01-0", { state: "waiting" })], now), "need_you");
  // Waiting on someone named is not on you.
  assert.equal(runState(run({ ...fresh, taken: ["A01-0"] }), [task("A01-0", { state: "waiting", who: "the designer" })], now), "running");
  assert.equal(runState(run({ ...fresh, stopping: true }), [], now), "stopping");
  assert.equal(runState(run({ stopping: true }), [], now), "stalled");
  assert.deepEqual(stateCounts(["running", "stalled", "running"]), [
    { state: "stalled", n: 1 },
    { state: "running", n: 2 },
  ]);
  assert.deepEqual(stateCounts(["stalled"]), [{ state: "stalled", n: 1 }]);
});

test("sessions are grouped by agent; an agent taken off finishes its task in a group of its own", () => {
  const one = run();
  assert.equal(grouped(one), false);
  const r = run({
    agents: ["codex_cli"],
    sessions: { codex_cli: 2 },
    no_task: ["codex_cli"],
    difficulties: { codex_cli: "1-3" },
  });
  const groups = sessionGroups(r);
  assert.deepEqual(
    groups.map((g) => [g.agent, g.removed, g.working.map((w) => w.id), g.sessions, g.noTask, g.levels]),
    [
      ["codex_cli", false, [], 2, true, { low: 1, high: 3 }],
      ["claude_cli", true, ["A01-1"], 1, false, { low: 1, high: 5 }],
    ],
  );
  assert.equal(grouped(r), true);
});

test("how long a session has worked", () => {
  assert.deepEqual(workedFor(0, 59_000), { unit: "minutes", n: 0 });
  assert.deepEqual(workedFor(0, 42 * 60_000), { unit: "minutes", n: 42 });
  assert.deepEqual(workedFor(0, 130 * 60_000), { unit: "hours", n: 2 });
});

test("the folds this computer keeps: section open and panels folded unless stored", () => {
  assert.deepEqual(readFolds(null), { sectionFolded: false, open: [] });
  assert.deepEqual(readFolds("nonsense"), { sectionFolded: false, open: [] });
  assert.deepEqual(readFolds('{"sectionFolded":true,"open":["m-1",3]}'), { sectionFolded: true, open: ["m-1"] });
  assert.deepEqual(keptFolds({ sectionFolded: true, open: ["m-1", "m-2"] }, ["m-2"]), { sectionFolded: true, open: ["m-2"] });
});
