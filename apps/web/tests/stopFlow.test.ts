// Tests for how a stopped task is worded, and the list's Show filter (decision 0016, Task-A01-31,
// Task-A01-lens781; decision 0020, Task-A01-petal376).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { FILTER_STATES, allTicked, allBlockersDone, blockers, bothBanner, emptyBy, filterBanner, filterGroups, filterName, groupTicks, hiddenByFilter, isAll, labelText, labelsOff, onTable, searchBanner, storeFilter, tableFilter, taskCount, taskListBanners, tickGroup, tickLabel, tickLabeled, tickState, type StateFilter, type Words, hiddenText, humanCheckNote, keepState, readFilter, stopFact, stopText, toCheck, toReview, withDecisions } from "../src/stopFlow.ts";
import { AI_DECISIONS, DEPLOYED_BY_AGENT, DEPLOYED_BY_HUMAN, LOCALLY_BY_AGENT, LOCALLY_BY_HUMAN } from "../src/stopFlow.ts";
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
/** Only these states ticked; every label side ticked. */
const only = (...states: State[]): StateFilter => ({ ...all, states: FILTER_STATES.filter((s) => !states.includes(s)) });

test("the state ticks keep waiting and blocked apart; everything ticked keeps every task", () => {
  const waiting = task({ state: "waiting", waiting_on: "x" });
  const blocked = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.equal(keepState(only("waiting"), waiting), true);
  assert.equal(keepState(only("waiting"), blocked), false);
  assert.equal(keepState(only("blocked"), blocked), true);
  assert.equal(keepState(only("waiting", "blocked"), blocked), true, "several states: either one");
  assert.equal(keepState(all, blocked), true);
  assert.equal(isAll(all), true);
  assert.equal(isAll(only("done")), false);
});

test("a label's Yes keeps the tasks that have it, its No those that do not; no order, no 'or higher' (decision 0020)", () => {
  const local = task({ labels: [LOCALLY_BY_AGENT] });
  const both = task({ labels: [LOCALLY_BY_AGENT, DEPLOYED_BY_AGENT] });
  const deployed = task({ labels: [DEPLOYED_BY_AGENT] });
  // Locally verified by agent, No unticked: only the tasks that carry it, whatever else they carry.
  const hasLocal = tickLabel(all, LOCALLY_BY_AGENT, "no", false);
  assert.equal(keepState(hasLocal, local), true);
  assert.equal(keepState(hasLocal, both), true);
  assert.equal(keepState(hasLocal, deployed), false, "deployed is not 'higher': it does not carry the label");
  assert.equal(keepState(hasLocal, task({ labels: [] })), true, "no label: Labeled's No alone decides (Task-A01-teacup726)");
  // Deployed and verified by agent, Yes unticked: the tasks without it.
  const notDeployed = tickLabel(all, DEPLOYED_BY_AGENT, "yes", false);
  assert.equal(keepState(notDeployed, local), true);
  assert.equal(keepState(notDeployed, both), false);
  // Two labels at once: both conditions hold.
  const localNotDeployed = tickLabel(hasLocal, DEPLOYED_BY_AGENT, "yes", false);
  assert.equal(keepState(localNotDeployed, local), true);
  assert.equal(keepState(localNotDeployed, both), false);
  // A custom label is a label like any other.
  assert.equal(keepState(tickLabel(all, "shipped", "no", false), task({ labels: ["shipped"] })), true);
  assert.equal(keepState(tickLabel(all, "shipped", "no", false), local), false);
  // A label the table does not list (yet) hides nothing by itself.
  assert.equal(keepState(hasLocal, task({ labels: [LOCALLY_BY_AGENT, "not in the table"] })), true);
});

test("to check through the filter: done, with both 'by human' labels at No (decision 0020)", () => {
  const f = { ...tickLabel(tickLabel(all, LOCALLY_BY_HUMAN, "yes", false), DEPLOYED_BY_HUMAN, "yes", false), states: only("done").states };
  assert.equal(keepState(f, task({ state: "done", labels: [LOCALLY_BY_AGENT] })), true);
  assert.equal(keepState(f, task({ state: "done", labels: [DEPLOYED_BY_AGENT, AI_DECISIONS] })), true);
  assert.equal(keepState(f, task({ state: "done", labels: [] })), true, "a done task with no label is to check too");
  assert.equal(keepState(f, task({ state: "done", labels: [LOCALLY_BY_AGENT, LOCALLY_BY_HUMAN] })), false);
  assert.equal(keepState(f, task({ state: "done", labels: [DEPLOYED_BY_HUMAN] })), false);
  assert.equal(keepState(f, task({ state: "ready", labels: [LOCALLY_BY_AGENT] })), false);
  // The same tasks toCheck finds.
  for (const labels of [[LOCALLY_BY_AGENT], [], [LOCALLY_BY_HUMAN], [DEPLOYED_BY_HUMAN, DEPLOYED_BY_AGENT]]) {
    const x = task({ state: "done", labels });
    assert.equal(keepState(f, x), toCheck(x), labels.join(", "));
  }
});

test("ticking and unticking keeps the panel's order; Select all and Clear all per group, and when each is grey", () => {
  let f = tickState(all, "done", false);
  f = tickState(f, "backlog", false);
  assert.deepEqual(f.states, ["backlog", "done"]);
  assert.deepEqual(tickState(f, "done", true).states, ["backlog"]);
  assert.deepEqual(groupTicks(all, "states"), { all: true, none: false });
  assert.deepEqual(groupTicks(f, "states"), { all: false, none: false });
  const noStates = tickGroup(f, "states", false);
  assert.deepEqual(noStates.states, FILTER_STATES);
  assert.deepEqual(groupTicks(noStates, "states"), { all: false, none: true });
  assert.deepEqual(tickGroup(noStates, "states", true).states, []);
  const noLabels = tickGroup(all, "labels", false);
  assert.deepEqual(groupTicks(noLabels, "labels"), { all: false, none: true });
  assert.deepEqual(noLabels.states, [], "clearing labels leaves the states alone");
  assert.deepEqual(groupTicks(tickGroup(noLabels, "labels", true), "labels"), { all: true, none: false });
  let g = tickLabel(all, AI_DECISIONS, "yes", false);
  g = tickLabel(g, LOCALLY_BY_AGENT, "yes", false);
  assert.deepEqual(g.yes, [LOCALLY_BY_AGENT, AI_DECISIONS], "in the table's order");
});

test("the list is empty whatever the tasks: no state ticked, or a label with neither side", () => {
  assert.equal(emptyBy(all), null);
  assert.deepEqual(emptyBy(tickGroup(all, "states", false)), { kind: "states" });
  const neither = tickLabel(tickLabel(all, "shipped", "yes", false), "shipped", "no", false);
  assert.equal(emptyBy(neither), null, "tasks with no label are still listed");
  assert.equal(keepState(neither, task({ labels: ["shipped"] })), false);
  assert.equal(keepState(neither, task({ labels: [LOCALLY_BY_AGENT] })), false);
  assert.equal(keepState(neither, task({})), true);
  const labelledOnly = tickLabeled(neither, "no", false);
  assert.deepEqual(emptyBy(labelledOnly), { kind: "label", label: "shipped" });
  assert.deepEqual(emptyBy(tickLabeled(tickLabeled(all, "yes", false), "no", false)), { kind: "labeled" });
  assert.equal(emptyBy(tickLabeled(all, "yes", false)), null);
});

// Task-A01-teacup726: Labeled (Yes / No) first in the Labels column.
test("Labeled's No hides exactly the tasks with no label; its Yes hides every labelled task", () => {
  const none = task({});
  const local = task({ labels: [LOCALLY_BY_AGENT] });
  const off = task({ labels: ["not in the table"] });
  const noNo = tickLabeled(all, "no", false);
  assert.equal(keepState(noNo, none), false);
  assert.equal(keepState(noNo, local), true);
  assert.equal(keepState(noNo, off), true, "a label the table does not list is still a label");
  const noYes = tickLabeled(all, "yes", false);
  assert.equal(keepState(noYes, none), true);
  assert.equal(keepState(noYes, local), false);
  assert.equal(keepState(noYes, off), false);
  assert.equal(isAll(noYes), false);
  // The labels grey out with their ticks kept; ticking Yes again brings them back.
  const ticks = tickLabel(all, LOCALLY_BY_HUMAN, "yes", false);
  const greyed = tickLabeled(ticks, "yes", false);
  assert.equal(labelsOff(greyed), true);
  assert.deepEqual(greyed.yes, [LOCALLY_BY_HUMAN]);
  const back = tickLabeled(greyed, "yes", true);
  assert.deepEqual(back, ticks);
  assert.equal(labelsOff(back), false);
  assert.equal(keepState(back, task({ labels: [LOCALLY_BY_HUMAN] })), false);
});

test("Select all, Clear all and the restore include Labeled; the button names it", () => {
  const cleared = tickGroup(all, "labels", false);
  assert.deepEqual(cleared.labeled, ["yes", "no"]);
  assert.deepEqual(groupTicks(cleared, "labels"), { all: false, none: true });
  assert.deepEqual(groupTicks(tickLabeled(all, "no", false), "labels"), { all: false, none: false });
  assert.deepEqual(tickGroup(cleared, "labels", true), all);
  assert.deepEqual(allTicked(LABELS), all);
  assert.equal(filterName(tickLabeled(all, "no", false), t), "Hide Labeled: no");
  assert.equal(filterName(tickLabeled(tickState(all, "done", false), "yes", false), t), "Hide Done, Labeled: yes");
  // While greyed out, the labels' ticks hide nothing, so the button leaves them out.
  const greyed = tickLabeled(tickLabel(tickLabel(all, "shipped", "no", false), AI_DECISIONS, "no", false), "yes", false);
  assert.equal(filterName(greyed, t), "Hide Labeled: yes");
  // Stored and read back; an older stored filter has Labeled all ticked.
  assert.deepEqual(onTable(readFilter(storeFilter(greyed)), LABELS), greyed);
  assert.deepEqual(onTable(readFilter(JSON.stringify({ states: [], yes: [], no: ["shipped"] })), LABELS).labeled, []);
});

test("the filter is remembered as what is unticked; the older single choice comes back as that state", () => {
  const f = tickLabel(only("ready", "done"), LOCALLY_BY_HUMAN, "yes", false);
  const back = onTable(readFilter(storeFilter(f)), LABELS);
  assert.deepEqual(back, f);
  assert.equal(readFilter("not json").states.length, 0);
  assert.deepEqual(readFilter(null), allTicked());
  // Before Task-A01-lens781: a state alone, or everything ticked for the choices that are gone.
  assert.deepEqual(onTable(readFilter(null, "blocked"), LABELS), only("blocked"));
  for (const old of ["all", "open", "human", "nonsense", null]) assert.equal(isAll(readFilter(null, old)), true, String(old));
  // A label no longer in the table is let go, such as an older build's ladder step; a new one starts ticked.
  const old = onTable(readFilter(JSON.stringify({ states: [], yes: ["human verified"], no: ["locally verified"] })), LABELS);
  assert.equal(isAll(old), true);
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
  // A filter 0.1.41 may have stored: its Clear all put the entries themselves in the lists.
  const stored = readFilter(JSON.stringify({ states: ["done"], labeled: [], yes: project.labels, no: [project.labels[0], "shipped"] }));
  const f = tableFilter(stored, project, tasks);
  const names = [...LABELS, "found on a task"];
  assert.deepEqual(f.labels, names);
  assert.deepEqual(f.yes, []);
  assert.deepEqual(f.no, ["shipped"]);
  // Everything the open panel draws, after every kind of tick, is text.
  const drawn = (x: StateFilter) => [filterName(x, t), ...x.labels.map((l) => labelText(l, t)), ...x.yes, ...x.no];
  const after = [f, tickGroup(f, "labels", false), tickGroup(f, "labels", true), tickLabel(f, "found on a task", "yes", false), tickLabeled(f, "no", false), allTicked(f.labels)];
  for (const x of after) for (const word of drawn(x)) assert.equal(typeof word, "string", JSON.stringify(word));
  assert.equal(new Set(f.labels).size, f.labels.length, "each row's key once");
  assert.deepEqual(tickGroup(f, "labels", false).yes, names);
  // An older taskbag that still sends plain names reads the same.
  assert.deepEqual(tableFilter(allTicked(), { labels: names as never }, tasks).labels, names);
  assert.deepEqual(tableFilter(allTicked(), undefined).labels, LABELS.slice(0, 5));
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

test("the filter is named as the Show button words it: what it hides", () => {
  assert.equal(filterName(all, t), "All tasks");
  assert.equal(filterName(tickState(all, "backlog", false), t), "Hide Backlog");
  assert.equal(filterName(tickLabel(tickState(all, "dropped", false), LOCALLY_BY_HUMAN, "yes", false), t), "Hide Dropped, locally verified by human: yes");
  assert.equal(filterName(only("done"), t), "Filtered (6 hidden)");
  assert.equal(labelText(DEPLOYED_BY_AGENT, t), "deployed and verified by agent");
  assert.equal(labelText("our own label", t), "our own label");
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
