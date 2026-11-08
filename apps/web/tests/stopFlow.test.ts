// Tests for how a stopped task is worded, and the list's Show filter (decision 0016, Task-A01-31,
// Task-A01-lens781; decision 0020, Task-A01-petal376; the grid, Task-A01-rock075).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { FILTER_STATES, HUMAN_COLUMN, allTicked, allBlockersDone, blockers, bothBanner, columnText, emptyBy, filterBanner, filterColumns, filterGroups, filterName, gridTicks, hiddenByFilter, isAll, isTicked, labelText, onTable, rowTicks, searchBanner, sideOf, storeFilter, tableFilter, taskCount, taskListBanners, tickAll, tickCell, tickRow, type StateFilter, type Words, hiddenText, humanCheckNote, keepState, readFilter, stopFact, stopText, toCheck, toReview, withDecisions, PRESETS, SIDES, matchPreset, presetCount, presetFilter, presetName, presetTicks, type Preset } from "../src/stopFlow.ts";
import { AI_DECISIONS, BY_HUMAN, DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN, LOCALLY_BY_AGENT, LOCALLY_BY_HUMAN } from "../src/stopFlow.ts";
import type { Opened, State, Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };

// The English words, with parameters filled in, as the interface shows them.
const t = (key: string, params?: Record<string, string | number>) =>
  ((en as Record<string, string>)[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));

const base: Task = {
  id: "A01-31",
  title: "Blocked state",
  area: "A01",
  state: "ready",
  holder: null,
  waiting_on: null,
  next_dependency: null,
  iteration: null,
  handoff_note: null,
  without_human_check: null,
  human_check_steps: null,
  description: "",
};
const task = (over: Partial<Task>): Task => ({ ...base, ...over });

test("waiting says who must act: you when no one is named", () => {
  const all: Task[] = [];
  assert.equal(stopText(task({ state: "waiting", waiting_on: "approve the card" }), all, t), "Waiting for you: approve the card");
  assert.equal(
    stopText(task({ state: "waiting", waiting_on: "reply on licence", who: "Ann" }), all, t),
    "Waiting for Ann: reply on licence",
  );
  assert.equal(stopText(task({ state: "in_progress" }), all, t), null);
});

test("the list's stop fact: its name, then its text; a named person leads the text (Task-A01-cactus586)", () => {
  assert.deepEqual(stopFact(task({ state: "waiting", waiting_on: "approve the card" }), [], t), { name: "pane.waiting_on", text: "approve the card" });
  assert.deepEqual(stopFact(task({ state: "waiting", waiting_on: "reply on licence", who: "Ann" }), [], t), {
    name: "pane.waiting_on",
    text: "Waiting for Ann: reply on licence",
  });
  const blocker = task({ id: "A01-17", state: "done" });
  const me = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.deepEqual(stopFact(me, [blocker, me], t), { name: "pane.blocked_by", text: "A01-17 (Done) — all done; unblock it?" });
  assert.equal(stopFact(task({ state: "in_progress" }), [], t), null);
});

test("blocked names each blocker with its state now; a dropped one is flagged, never done", () => {
  const blocker = task({ id: "A01-17", state: "in_progress" });
  const me = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.equal(stopText(me, [blocker, me], t), "Blocked by A01-17 (In progress)");
  assert.equal(allBlockersDone(me, [blocker, me]), false);

  const done = { ...blocker, state: "done" as const };
  assert.equal(stopText(me, [done, me], t), "Blocked by A01-17 (Done) — all done; unblock it?");
  assert.equal(allBlockersDone(me, [done, me]), true);
  assert.equal(me.state, "blocked", "the app only tells; it moves nothing");

  const dropped = { ...blocker, state: "dropped" as const };
  assert.equal(stopText(me, [dropped, me], t), "Blocked by A01-17 (dropped — still blocked?)");
  assert.equal(allBlockersDone(me, [dropped, me]), false);
  assert.deepEqual(blockers(task({ state: "blocked", blocked_by: ["A09-09"] }), []), [{ id: "A09-09", state: null }]);
});

// Decision 0020 (Task-A01-petal376): the table's labels, built-in first, then this project's own.
const LABELS = [LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT, LOCALLY_BY_HUMAN, DEPLOYED_BY_HUMAN, AI_DECISIONS, "shipped"];
const all = allTicked(LABELS);
/** Only these states' rows ticked, each in full; every other row unticked. */
const only = (...states: State[]): StateFilter => FILTER_STATES.filter((s) => !states.includes(s)).reduce((f, s) => tickRow(f, s, false), all);
/** Every row unticked, then these cells ticked. */
const just = (...cells: [State, string, "yes" | "no"][]): StateFilter => cells.reduce((f, [s, c, side]) => tickCell(f, s, c, side, true), tickAll(all, false));

// Task-A01-rock075: the grid, one row per state, a Yes and a No for each column.
test("the grid's columns: Checked by a human first, then every other label of the table, built-in first", () => {
  assert.deepEqual(filterColumns(all), [HUMAN_COLUMN, LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT, AI_DECISIONS, "shipped"]);
  assert.deepEqual(filterColumns(all).map((c) => columnText(c, t)), ["Checked by a human", LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT, AI_DECISIONS, "shipped"]);
});

test("everything ticked lists every task; a row with nothing ticked lists none of its state", () => {
  const waiting = task({ state: "waiting", waiting_on: "x" });
  const blocked = task({ state: "blocked", blocked_by: ["A01-17"], labels: [AI_DECISIONS] });
  const kinds = [waiting, blocked, task({ state: "done", labels: [LOCALLY_BY_HUMAN] }), task({ labels: ["shipped", "not in the table"] })];
  for (const x of kinds) assert.equal(keepState(all, x), true, x.state);
  assert.equal(isAll(all), true);
  assert.equal(keepState(only("waiting"), waiting), true);
  assert.equal(keepState(only("waiting"), blocked), false);
  assert.equal(keepState(only("waiting", "blocked"), blocked), true, "several rows: either one");
  const noBlocked = tickRow(all, "blocked", false);
  assert.equal(keepState(noBlocked, blocked), false);
  assert.equal(keepState(noBlocked, task({ state: "blocked" })), false, "every blocked task, whatever its labels");
  assert.equal(keepState(noBlocked, waiting), true, "other rows untouched");
  assert.equal(isAll(only("done")), false);
});

test("a task is listed when it matches any ticked cell of its own state's row, and no other row's", () => {
  const local = task({ labels: [LOCALLY_BY_AGENT] });
  const both = task({ labels: [LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT] });
  const none = task({});
  // Ready: only "locally verified by agent: Yes".
  const f = just(["ready", LOCALLY_BY_AGENT, "yes"]);
  assert.equal(keepState(f, local), true);
  assert.equal(keepState(f, both), true);
  assert.equal(keepState(f, none), false);
  assert.equal(keepState(f, { ...local, state: "backlog" }), false, "the tick is in Ready's row, not Backlog's");
  // Add "deployed and verified by agent: No": any one of the two is enough (or, not and).
  const g = tickCell(f, "ready", DEPLOYED_BY_AGENT, "no", true);
  assert.equal(keepState(g, none), true, "matches the No alone");
  assert.equal(keepState(g, both), true, "matches the Yes alone");
  assert.equal(keepState(g, task({ labels: [DEPLOYED_BY_AGENT] })), false, "matches neither");
  // Unticking one cell of a full row hides only the tasks matching nothing else: with every other
  // column ticked on both sides, every task still matches some tick.
  assert.equal(keepState(tickCell(all, "ready", LOCALLY_BY_AGENT, "yes", false), local), true);
  // A label the table does not list is no column; a custom one is.
  assert.equal(keepState(just(["ready", "shipped", "yes"]), task({ labels: ["shipped"] })), true);
  assert.equal(keepState(just(["ready", "shipped", "yes"]), task({ labels: ["not in the table"] })), false);
});

test("Checked by a human is one column for both 'by human' labels; its No means neither", () => {
  const yes = just(["done", HUMAN_COLUMN, "yes"]);
  const no = just(["done", HUMAN_COLUMN, "no"]);
  const cases: [string[], boolean][] = [
    [[LOCALLY_BY_HUMAN], true],
    [[DEPLOYED_BY_HUMAN], true],
    [[LOCALLY_BY_HUMAN, DEPLOYED_BY_HUMAN], true],
    [[LOCALLY_BY_AGENT], false],
    [[DEPLOYED_BY_AGENT, AI_DECISIONS], false],
    [[], false],
  ];
  for (const [labels, checked] of cases) {
    const x = task({ state: "done", labels });
    assert.equal(sideOf(x, HUMAN_COLUMN), checked ? "yes" : "no", labels.join(", "));
    assert.equal(keepState(yes, x), checked, `Yes: ${labels.join(", ")}`);
    assert.equal(keepState(no, x), !checked, `No: ${labels.join(", ")}`);
    assert.equal(keepState(no, x), toCheck(x), "Done's No is To check");
  }
  assert.equal(filterColumns(all).filter((c) => BY_HUMAN.includes(c)).length, 0, "no column of their own");
});

// The tick pattern of "Needs you" in the mockup (A01-wigwam308 will make it a preset): Done keeps
// "Checked by a human: No" (to check), Waiting keeps a whole column (every waiting task is to
// answer), and every row keeps "AI decisions to review: Yes" (to review).
test("the Needs you ticks list exactly alm human's three lists: to check, to answer, to review", () => {
  const needsYou = just(
    ["done", HUMAN_COLUMN, "no"],
    ["waiting", HUMAN_COLUMN, "yes"],
    ["waiting", HUMAN_COLUMN, "no"],
    ...FILTER_STATES.map((s): [State, string, "yes"] => [s, AI_DECISIONS, "yes"]),
  );
  const labelSets: string[][] = [[], [LOCALLY_BY_AGENT], [DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN], [LOCALLY_BY_HUMAN], [AI_DECISIONS], [LOCALLY_BY_HUMAN, AI_DECISIONS], ["shipped"], [LOCALLY_BY_AGENT, AI_DECISIONS, "shipped"]];
  const tasks = FILTER_STATES.flatMap((state) => labelSets.map((labels, i) => task({ id: `${state}-${i}`, state, labels })));
  // As crates/cli/src/lib.rs's `human` lists them.
  const human = new Set(tasks.filter((x) => toCheck(x) || x.state === "waiting" || toReview(x)).map((x) => x.id));
  const kept = new Set(tasks.filter((x) => keepState(needsYou, x)).map((x) => x.id));
  assert.deepEqual([...kept].sort(), [...human].sort());
  assert.equal(kept.has("done-5"), true, "checked but with AI decisions to review: shown");
  assert.equal(kept.has("done-3"), false, "checked, nothing to review: not shown");
  assert.deepEqual(rowTicks(needsYou, "done"), "some");
});

test("the tick before a state sums up its row and ticks or unticks the whole row; Select all and Clear all do the grid", () => {
  assert.equal(rowTicks(all, "ready"), "all");
  const some = tickCell(all, "ready", AI_DECISIONS, "no", false);
  assert.equal(rowTicks(some, "ready"), "some");
  assert.equal(rowTicks(some, "done"), "all");
  assert.equal(rowTicks(tickRow(some, "ready", false), "ready"), "none");
  assert.deepEqual(tickRow(some, "ready", true), all);
  assert.deepEqual(gridTicks(all), { all: true, none: false });
  assert.deepEqual(gridTicks(some), { all: false, none: false });
  const cleared = tickAll(some, false);
  assert.deepEqual(gridTicks(cleared), { all: false, none: true });
  assert.equal(cleared.off.length, FILTER_STATES.length * filterColumns(all).length * 2);
  assert.deepEqual(tickAll(cleared, true), all);
  assert.deepEqual(allTicked(LABELS), all);
  // Only an all-unticked grid empties the list whatever the tasks.
  assert.equal(emptyBy(all), false);
  assert.equal(emptyBy(only("done")), false);
  assert.equal(emptyBy(cleared), true);
  assert.equal(emptyBy(tickCell(cleared, "dropped", "shipped", "no", true)), false);
  // Ticking in any order gives the same filter.
  assert.deepEqual(tickCell(tickCell(all, "done", "shipped", "no", false), "ready", HUMAN_COLUMN, "yes", false), tickCell(tickCell(all, "ready", HUMAN_COLUMN, "yes", false), "done", "shipped", "no", false));
});

test("the filter is named as the Show button words it: what it hides", () => {
  assert.equal(filterName(all, t), "All tasks");
  assert.equal(filterName(tickRow(all, "backlog", false), t), "Hide Backlog");
  assert.equal(filterName(tickCell(tickRow(all, "dropped", false), "done", HUMAN_COLUMN, "yes", false), t), "Hide Done · Checked by a human: yes, Dropped", "in the grid's order of rows");
  assert.equal(filterName(tickCell(all, "ready", LOCALLY_BY_AGENT, "no", false), t), "Hide Ready · locally verified by agent: no");
  assert.equal(filterName(only("done"), t), "Filtered (6 hidden)");
  assert.equal(labelText(DEPLOYED_BY_AGENT, t), "deployed and verified by agent");
  assert.equal(labelText("our own label", t), "our own label");
});

test("the filter is remembered as its unticked cells; a filter stored before the grid comes back all ticked", () => {
  const f = tickCell(tickRow(all, "ready", false), "done", HUMAN_COLUMN, "yes", false);
  assert.deepEqual(onTable(readFilter(storeFilter(f)), LABELS), f);
  assert.equal(isAll(readFilter("not json")), true);
  assert.deepEqual(readFilter(null), allTicked());
  // Before Task-A01-rock075: states and labels joined by "and".
  assert.equal(isAll(onTable(readFilter(JSON.stringify({ states: ["done"], labeled: ["no"], yes: [], no: ["shipped"] })), LABELS)), true);
  // Nonsense cells are let go; so are cells of a label no longer in the table, and a new label starts ticked.
  const stored = JSON.stringify({ grid: [["done", "gone", "yes"], ["nowhere", "", "yes"], ["done", "", "maybe"], ["done", "shipped", "no"], 7] });
  assert.deepEqual(onTable(readFilter(stored), LABELS).off, [{ state: "done", column: "shipped", side: "no" }]);
  assert.equal(isTicked(onTable(readFilter(stored), [...LABELS, "new one"]), "done", "new one", "no"), true);
  assert.equal(isAll(onTable(readFilter(stored), LABELS.slice(0, 5))), true, "shipped left the table");
});

// Task-A01-key755: in 0.1.41 just opening the Show filter blanked the window. The desktop app sends
// the label table as entries (`allomos/7`); the filter took them as they came, and drawing an entry
// as a row's text threw in React, which left the whole window blank and unresponsive.
test("the Show filter reads an allomos/7 label table as names, so opening it draws text only (Task-A01-key755)", () => {
  // The project as the desktop app sends it for this project's taskbag: entries, built-in first.
  const project = {
    labels: [
      { name: LOCALLY_BY_AGENT, meaning: "The agent built it and its checks passed.", built_in: true },
      { name: DEPLOYED_BY_AGENT, meaning: "The agent deployed it.", built_in: true },
      { name: LOCALLY_BY_HUMAN, meaning: "A person checked it.", built_in: true },
      { name: DEPLOYED_BY_HUMAN, meaning: "A person checked it live.", built_in: true },
      { name: AI_DECISIONS, meaning: "Decisions wait for review.", built_in: true },
      { name: "shipped", built_in: false },
    ],
  };
  const tasks = [task({ id: "A", labels: ["shipped", "found on a task"] }), task({ id: "B", labels: [LOCALLY_BY_AGENT] }), task({ id: "C" })];
  const f = tableFilter(readFilter(storeFilter(tickCell(all, "done", "shipped", "no", false))), project, tasks);
  const names = [...LABELS, "found on a task"];
  assert.deepEqual(f.labels, names);
  assert.deepEqual(f.off, [{ state: "done", column: "shipped", side: "no" }]);
  // Everything the open panel draws, after every kind of tick, is text.
  const drawn = (x: StateFilter) => [filterName(x, t), ...filterColumns(x).map((c) => columnText(c, t)), ...x.off.map((c) => c.column)];
  const after = [f, tickAll(f, false), tickAll(f, true), tickCell(f, "ready", "found on a task", "yes", false), tickRow(f, "waiting", false), allTicked(f.labels)];
  for (const x of after) for (const word of drawn(x)) assert.equal(typeof word, "string", JSON.stringify(word));
  assert.equal(new Set(filterColumns(f)).size, filterColumns(f).length, "each column's key once");
  // An older taskbag that still sends plain names reads the same.
  assert.deepEqual(tableFilter(allTicked(), { labels: names as never }, tasks).labels, names);
  assert.deepEqual(tableFilter(allTicked(), undefined).labels, LABELS.slice(0, 5));
});

// Task-A01-rock075: the list asks the filter about every task each time it draws; on a taskbag as
// big as this project's (323 tasks on 2026-11-08, so ten times that here) opening the panel,
// ticking and closing it must stay quick.
test("the grid stays quick on a big taskbag: every tick and its list in well under a frame each", () => {
  const custom = Array.from({ length: 8 }, (_, i) => `custom ${i}`);
  const labels = [...LABELS, ...custom];
  const pool = [LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT, LOCALLY_BY_HUMAN, DEPLOYED_BY_HUMAN, AI_DECISIONS, ...custom];
  const tasks = Array.from({ length: 3230 }, (_, i) =>
    task({ id: `T-${i}`, state: FILTER_STATES[i % FILTER_STATES.length], labels: pool.filter((_, j) => (i >> j) % 3 === 0) }),
  );
  let f = allTicked(labels);
  const started = performance.now();
  let clicks = 0;
  for (const state of FILTER_STATES)
    for (const column of filterColumns(f)) {
      f = tickCell(f, state, column, "yes", false);
      clicks++;
      // What a click redraws: the panel's ticks and summaries, the button's name, and the list.
      for (const s of FILTER_STATES) rowTicks(f, s);
      filterName(f, t);
      hiddenByFilter(f, tasks);
    }
  const each = (performance.now() - started) / clicks;
  assert.ok(each < 16, `${each.toFixed(2)} ms per click`);
  // Every Yes unticked: only tasks on the No side of some column stay, whatever their state.
  assert.equal(tasks.every((x) => keepState(f, x) === filterColumns(f).some((c) => sideOf(x, c) === "no")), true);
});

test("to check: a done task with no 'by human' label, whatever its note (decision 0020)", () => {
  const finished = task({ state: "done", labels: [LOCALLY_BY_AGENT], without_human_check: "loop run", human_check_steps: "1. Open it." });
  assert.equal(toCheck(finished), true);
  assert.equal(humanCheckNote(finished), "loop run");
  assert.equal(toCheck(task({ state: "done" })), true, "no label, no note: still to check");
  assert.equal(toCheck(task({ state: "done", labels: [DEPLOYED_BY_AGENT, AI_DECISIONS] })), true, "'by agent' labels are not the human check");
  assert.equal(toCheck(task({ state: "done", labels: [LOCALLY_BY_HUMAN] })), false);
  assert.equal(toCheck(task({ state: "done", labels: [DEPLOYED_BY_HUMAN], without_human_check: "old note" })), false, "the note decides nothing");
  assert.equal(toCheck(task({ state: "in_progress", without_human_check: "x" })), false, "not finished");
});

// Task-A01-42: what the state filter hides.
test("the filter's hidden tasks are counted by state, in the filter's order", () => {
  const tasks = [
    task({ id: "a", state: "done" }),
    task({ id: "b", state: "backlog" }),
    task({ id: "c", state: "waiting" }),
    task({ id: "d", state: "done" }),
    task({ id: "e", state: "ready" }),
  ];
  const hidden = hiddenByFilter(only("backlog"), tasks);
  assert.deepEqual(hidden, {
    count: 4,
    states: [
      { state: "ready", count: 1 },
      { state: "waiting", count: 1 },
      { state: "done", count: 2 },
    ],
  });
  assert.equal(hiddenText(hidden, t), "4 more hidden by the filter: 1 Ready, 1 Waiting, 2 Done");
  assert.deepEqual(hiddenByFilter(all, tasks), { count: 0, states: [] });
});

test("AI decisions to review: the badge reads the label; the decisions from the history go on their tasks (Task-A03-19, decision 0020)", () => {
  const decision = { at: "2026-10-15T10:00:00Z", by: "claude/x", text: "Names: alm describe — match alm title" };
  const opened: Opened = {
    root: "/r",
    readme_added: false,
    loaded: {
      taskbag: {
        project: { format: "allomos/7", id: "P", name: "P", labels: [{ name: AI_DECISIONS, built_in: true }] },
        areas: [],
        iterations: [],
        tasks: [task({ id: "A01-01", state: "backlog", labels: [AI_DECISIONS] }), task({ id: "A01-02" })],
      },
      issues: [],
    },
    ai_decisions: { "A01-01": [decision] },
  } as Opened;
  const tasks = withDecisions(opened).loaded.taskbag!.tasks;
  assert.deepEqual(tasks[0].ai_decisions, [decision]);
  assert.equal(tasks[1].ai_decisions, undefined);
  assert.equal(toReview(tasks[0]), true, "even a backlog task");
  assert.equal(toReview(tasks[1]), false);
  // The label decides, not the history: reviewed (`alm reviewed` removes the label), the badge goes.
  assert.equal(toReview({ ...tasks[0], labels: [] }), false);
  assert.equal(toReview(task({ labels: [AI_DECISIONS] })), true, "the label alone, before the history is read");
  assert.equal(withDecisions({ ...opened, ai_decisions: undefined }).loaded.taskbag!.tasks[0].ai_decisions, undefined);
  assert.equal(t("task.ai_decided"), "AI decisions to review");
  // Task-A01-coach782: a task's approvals come along too.
  const approval = { id: "ap-abc123", scope: { tasks: ["A01-02"], plan_sha256: "x", plan: "description" }, at: "2026-11-08T10:00:00Z", channel: "chat", status: "applies" as const };
  const withApprovals = withDecisions({ ...opened, approvals: { "A01-02": [approval] } }).loaded.taskbag!.tasks;
  assert.deepEqual(withApprovals[1].approvals, [approval]);
  assert.deepEqual(withApprovals[0].ai_decisions, [decision], "decisions stay");
  assert.equal(withApprovals[0].approvals, undefined);
  // Task-A01-job650: and its outside effects.
  const effect = { id: "ef-abc123", task: "A01-02", target: "production", action: "deploy", rollback: "redeploy", at: "2026-11-08T10:00:00Z" };
  const withEffects = withDecisions({ ...opened, effects: { "A01-02": [effect] } }).loaded.taskbag!.tasks;
  assert.deepEqual(withEffects[1].effects, [effect]);
  assert.equal(withEffects[0].effects, undefined);
  // Task-A01-mint242: and its links to code and releases.
  const link = { kind: "commit" as const, ref: "abcdef1", task: "A01-02", at: "2026-11-08T10:00:00Z" };
  const withLinks = withDecisions({ ...opened, links: { "A01-02": [link] } }).loaded.taskbag!.tasks;
  assert.deepEqual(withLinks[1].links, [link]);
  assert.equal(withLinks[0].links, undefined);
});

// Task-A01-44: groups with nothing left after the filter are not drawn; the banner says what it shows.
const plain = (w: Words) => w.map((x) => (x.strong ? `**${x.text}**` : x.text)).join("");

test("the filter leaves out groups with no match and keeps the whole list for the count", () => {
  const groups = [
    { key: "a", tasks: [task({ id: "A-1", state: "blocked" }), task({ id: "A-2", state: "done" })] },
    { key: "b", tasks: [task({ id: "B-1", state: "done" })] },
    { key: "c", tasks: [] },
  ];
  assert.equal(filterGroups(groups, all), groups);
  const kept = filterGroups(groups, only("blocked"));
  assert.deepEqual(kept.map((g) => g.key), ["a"]);
  assert.deepEqual(kept[0].tasks.map((x) => x.id), ["A-1"]);
  assert.equal(kept[0].counted?.length, 2);
  assert.deepEqual(filterGroups(groups, only("waiting")), []);
});

test("the filter banner counts tasks and the groups left out", () => {
  assert.deepEqual(filterBanner(all, { kind: "area", tasks: 3, groups: 2, total: 7 }, t), []);
  assert.equal(
    plain(filterBanner(only("blocked"), { kind: "area", tasks: 3, groups: 2, total: 7 }, t)),
    "**3 tasks** match the filter, in 2 of 7 areas. The 5 areas with none are left out.",
  );
  assert.equal(
    plain(filterBanner(only("blocked"), { kind: "iteration", tasks: 1, groups: 1, total: 2 }, t)),
    "**1 task** matches the filter, in 1 of 2 iterations. The 1 iteration with none is left out.",
  );
  assert.equal(plain(filterBanner(only("ready"), { kind: "area", tasks: 4, groups: 3, total: 3 }, t)), "**4 tasks** match the filter, in 3 areas.");
});

test("nothing matches: only the banner, saying every group is left out", () => {
  assert.equal(
    plain(filterBanner(only("blocked"), { kind: "iteration", tasks: 0, groups: 0, total: 12 }, t)),
    "No task matches the filter. All 12 iterations are left out.",
  );
  assert.equal(plain(filterBanner(only("blocked"), { kind: "area", tasks: 0, groups: 0, total: 1 }, t)), "No task matches the filter. The only area is left out.");
  assert.equal(plain(filterBanner(only("done"), { kind: "area", tasks: 0, groups: 0, total: 0 }, t)), "No task matches the filter.");
});

test("while searching, the filter banner says the list is the filter and the search together (Task-A01-48)", () => {
  assert.equal(
    plain(bothBanner(only("waiting"), { kind: "area", query: "alm", tasks: 2, groups: 1, matches: 5 }, t)),
    "Filter and search together: only tasks that match the filter and hold “alm” are listed, in 1 area. 3 more matches are hidden by the filter.",
  );
  assert.equal(
    plain(bothBanner(only("done"), { kind: "iteration", query: "x", tasks: 2, groups: 2, matches: 3 }, t)),
    "Filter and search together: only tasks that match the filter and hold “x” are listed, in 2 iterations. 1 more match is hidden by the filter.",
  );
  assert.equal(
    plain(bothBanner(only("blocked"), { kind: "area", query: "alm", tasks: 0, groups: 0, matches: 3 }, t)),
    "None of the tasks found for “alm” matches the filter. All 3 matches are hidden by the filter.",
  );
  assert.equal(plain(bothBanner(only("blocked"), { kind: "area", query: "alm", tasks: 0, groups: 0, matches: 1 }, t)), "None of the tasks found for “alm” matches the filter. The 1 match is hidden by the filter.");
  // Nothing found at all: "Nothing found" says so, not the banner; no filter: no filter banner.
  assert.deepEqual(bothBanner(only("blocked"), { kind: "area", query: "zzz", tasks: 0, groups: 0, matches: 0 }, t), []);
  assert.deepEqual(bothBanner(all, { kind: "area", query: "alm", tasks: 2, groups: 1, matches: 2 }, t), []);
});

test("filter and search narrow the task list together; clearing either falls back to the other (Task-A01-48)", () => {
  const a = task({ id: "A-1", title: "alm search", state: "waiting" });
  const b = task({ id: "A-2", title: "alm list", state: "done" });
  const c = task({ id: "B-1", title: "other", state: "waiting", area: "B" });
  const d = task({ id: "B-2", title: "alm too", state: "done", area: "B" });
  const groups = [
    { key: "a-A", tasks: [a, b] },
    { key: "a-B", tasks: [c, d] },
  ];
  const holds = (x: Task) => x.title.includes("alm");
  const search = <G extends { tasks: Task[] }>(gs: G[]) => gs.map((g) => ({ ...g, tasks: g.tasks.filter(holds) })).filter((g) => g.tasks.length > 0);
  const matches = 3;
  // Both on: only A-1 (waiting and holds "alm"); area B, with none, is left out.
  const filtered = filterGroups(groups, only("waiting"));
  const found = search(filtered);
  assert.deepEqual(found.map((g) => g.tasks.map((x) => x.id)), [["A-1"]]);
  const both = taskListBanners({ kind: "area", filter: only("waiting"), query: " alm ", groups, filtered, found, matches }, t);
  assert.equal(plain(both.search), "**1 task** found for “alm” with the “Filtered (6 hidden)” filter on.");
  assert.equal(
    plain(both.filter),
    "Filter and search together: only tasks that match the filter and hold “alm” are listed, in 1 area. 2 more matches are hidden by the filter.",
  );
  // The search cleared: the filter banner alone.
  const filterOnly = taskListBanners({ kind: "area", filter: only("waiting"), query: "", groups, filtered, found: filtered, matches: 0 }, t);
  assert.deepEqual(filterOnly.search, []);
  assert.equal(plain(filterOnly.filter), "**2 tasks** match the filter, in 2 areas.");
  // The filter off: the search banner alone, counting every match.
  const searchOnly = taskListBanners({ kind: "area", filter: all, query: "alm", groups, filtered: groups, found: search(groups), matches }, t);
  assert.equal(plain(searchOnly.search), "**3 tasks** found for “alm”.");
  assert.deepEqual(searchOnly.filter, []);
  // Every match hidden by the filter: no search banner, the filter banner says why.
  const none = taskListBanners({ kind: "area", filter: only("blocked"), query: "alm", groups, filtered: [], found: [], matches }, t);
  assert.deepEqual(none.search, []);
  assert.equal(plain(none.filter), "None of the tasks found for “alm” matches the filter. All 3 matches are hidden by the filter.");
});

test("the search banner says how much was found", () => {
  assert.equal(plain(searchBanner("task", 2, "alm", t)), "**2 tasks** found for “alm”.");
  assert.equal(plain(searchBanner("task", 2, "alm", t, "In progress")), "**2 tasks** found for “alm” with the “In progress” filter on.");
});

// The Show filter's presets (Task-A01-wigwam308): each only sets the grid's ticks.

/** Every kind of task the grid tells apart: each state, with and without each label of LABELS. */
const everyTask: Task[] = FILTER_STATES.flatMap((state) =>
  [[], [LOCALLY_BY_AGENT], [DEPLOYED_BY_AGENT], [LOCALLY_BY_HUMAN], [DEPLOYED_BY_HUMAN], [AI_DECISIONS], [LOCALLY_BY_HUMAN, AI_DECISIONS], ["shipped"], [DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN, AI_DECISIONS, "shipped"]].flatMap((labels, i) =>
    [null, "Ann"].map((who) => task({ id: `${state}-${i}-${who ?? "me"}`, state, labels, who: who ?? undefined })),
  ),
);
const listed = (f: StateFilter) => everyTask.filter((x) => keepState(f, x)).map((x) => x.id);
const inStates = (...states: State[]) => everyTask.filter((x) => states.includes(x.state)).map((x) => x.id);

test("each row preset ticks its rows whole, nothing else", () => {
  const rows: Record<string, State[]> = {
    all: FILTER_STATES,
    not_finished: ["backlog", "ready", "in_progress", "waiting", "blocked"],
    ready: ["ready"],
    in_hand: ["in_progress", "waiting", "blocked"],
    finished: ["done", "dropped"],
  };
  for (const [p, states] of Object.entries(rows)) {
    const f = presetFilter(p as Preset, LABELS);
    for (const s of FILTER_STATES) assert.equal(rowTicks(f, s), states.includes(s) ? "all" : "none", `${p}: ${s}`);
    assert.deepEqual(listed(f), inStates(...states), p);
  }
  assert.ok(isAll(presetFilter("all", LABELS)), "All tasks is everything ticked");
});

test("Needs you ticks the grid as the approver set it (2026-11-08)", () => {
  const f = presetFilter("needs_you", LABELS);
  const ticked = (s: State) =>
    filterColumns(f).flatMap((c) => SIDES.filter((side) => isTicked(f, s, c, side)).map((side) => `${c || "human"}:${side}`));
  assert.deepEqual(ticked("done"), ["human:no", `${AI_DECISIONS}:yes`]);
  assert.equal(rowTicks(f, "waiting"), "all");
  for (const s of ["backlog", "ready", "in_progress", "blocked", "dropped"] as State[]) assert.deepEqual(ticked(s), [`${AI_DECISIONS}:yes`], s);
});

test("Needs you lists exactly alm human's three lists: to check, to answer, to review", () => {
  // As `alm human` (crates/cli/src/lib.rs, fn human) works them out.
  const toAnswer = (x: Task) => x.state === "waiting";
  const human = everyTask.filter((x) => toCheck(x) || toAnswer(x) || toReview(x)).map((x) => x.id);
  assert.deepEqual(listed(presetFilter("needs_you", LABELS)), human);
  // A done task a person checked, whose AI decisions wait for review, is among them.
  const checked = task({ id: "D", state: "done", labels: [LOCALLY_BY_HUMAN, AI_DECISIONS] });
  assert.ok(!toCheck(checked) && toReview(checked));
  assert.ok(keepState(presetFilter("needs_you", LABELS), checked));
  // Waiting for someone else is To answer too (no "Waits for me" yet).
  assert.ok(keepState(presetFilter("needs_you", LABELS), task({ state: "waiting", who: "Ann" })));
});

test("the ticks of a preset show it: ✓ and the Show button's name; any change clears it", () => {
  for (const p of PRESETS) {
    const f = presetFilter(p, LABELS);
    assert.equal(matchPreset(f), p);
    assert.equal(filterName(f, t), presetName(p, t));
  }
  assert.equal(filterName(presetFilter("needs_you", LABELS), t), "Needs you");
  assert.equal(filterName(presetFilter("ready", LABELS), t), "Ready to start");
  assert.equal(filterName(all, t), "All tasks");
  const changed = tickCell(presetFilter("needs_you", LABELS), "ready", "shipped", "no", true);
  assert.equal(matchPreset(changed), null);
  assert.match(filterName(changed, t), /^Filtered \(\d+ hidden\)$/);
  assert.equal(matchPreset(tickRow(presetFilter("finished", LABELS), "dropped", false)), null);
  assert.equal(matchPreset(tickAll(all, false)), null, "nothing ticked is no preset");
  // Ticked by hand to the same cells, it is the preset again.
  assert.equal(matchPreset(tickRow(tickRow(tickAll(all, false), "done", true), "dropped", true)), "finished");
  // A new label in the table starts ticked everywhere, so a preset's ticks no longer match it.
  assert.equal(matchPreset(onTable(presetFilter("ready", LABELS), [...LABELS, "new"])), null);
  assert.equal(matchPreset(presetFilter("ready", [...LABELS, "new"])), "ready");
});

test("each preset says what it ticks and how many tasks it lists now", () => {
  assert.equal(presetTicks("not_finished", t), "Backlog, Ready, In progress, Waiting, Blocked");
  assert.equal(presetTicks("in_hand", t), "In progress, Waiting, Blocked");
  assert.equal(presetTicks("finished", t), "Done, Dropped");
  assert.equal(presetTicks("ready", t), "Ready");
  assert.match(presetTicks("needs_you", t), /AI decisions to review/);
  const tasks = [task({ id: "a", state: "ready" }), task({ id: "b", state: "waiting" }), task({ id: "c", state: "done" }), task({ id: "d", state: "done", labels: [LOCALLY_BY_HUMAN] })];
  assert.deepEqual(
    PRESETS.map((p) => presetCount(p, LABELS, tasks)),
    [4, 2, 1, 1, 2, 2],
  );
});
