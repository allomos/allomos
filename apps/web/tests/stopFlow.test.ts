// Tests for how a stopped task is worded, and the state filter (decision 0016, Task-A01-31).
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { allBlockersDone, blockers, bothBanner, filterBanner, filterGroups, filterName, hiddenByFilter, searchBanner, taskCount, taskListBanners, type Words, hiddenText, humanCheckNote, keepState, readFilter, stopText, toCheck, toReview, withDecisions } from "../src/stopFlow.ts";
import type { Opened, Task } from "../src/types.ts";
import en from "../src/i18n/en.json" with { type: "json" };

// The English words, with parameters filled in, as the interface shows them.
const t = (key: string, params?: Record<string, string | number>) =>
  ((en as Record<string, string>)[key] ?? key).replace(/\{(\w+)\}/g, (_, p) => String(params?.[p] ?? `{${p}}`));

const base: Task = {
  id: "A01-31",
  title: "Blocked state",
  area: "A01",
  state: "ready",
  label: null,
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

test("the state filter keeps waiting and blocked apart", () => {
  const waiting = task({ state: "waiting", waiting_on: "x" });
  const blocked = task({ state: "blocked", blocked_by: ["A01-17"] });
  assert.equal(keepState("waiting", waiting), true);
  assert.equal(keepState("waiting", blocked), false);
  assert.equal(keepState("blocked", blocked), true);
  assert.equal(keepState("all", blocked), true);
  assert.equal(readFilter("blocked"), "blocked");
  assert.equal(readFilter("nonsense"), "all");
  assert.equal(readFilter(null), "all");
});

test("not human-checked: a done task with the note (Task-A03-18)", () => {
  const finished = task({ state: "done", without_human_check: "loop run", human_check_steps: "1. Open it." });
  assert.equal(toCheck(finished), true);
  assert.equal(humanCheckNote(finished), "loop run");
  assert.equal(toCheck(task({ state: "done" })), false, "finished the usual way");
  assert.equal(toCheck(task({ state: "in_progress", without_human_check: "x" })), false, "not finished");
});

test("the Needs a human filter keeps the tasks to check and to answer, not backlog", () => {
  assert.equal(keepState("human", task({ state: "done", without_human_check: "loop run" })), true);
  assert.equal(keepState("human", task({ state: "waiting", waiting_on: "approve" })), true);
  assert.equal(keepState("human", task({ state: "backlog" })), false);
  assert.equal(keepState("human", task({ state: "done" })), false);
  assert.equal(keepState("human", task({ state: "blocked", blocked_by: ["A01-17"] })), false);
  assert.equal(readFilter("human"), "human");
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
  const hidden = hiddenByFilter("backlog", tasks);
  assert.deepEqual(hidden, {
    count: 4,
    states: [
      { state: "ready", count: 1 },
      { state: "waiting", count: 1 },
      { state: "done", count: 2 },
    ],
  });
  assert.equal(hiddenText(hidden, t), "4 more hidden by the filter: 1 Ready, 1 Waiting, 2 Done");
  assert.deepEqual(hiddenByFilter("all", tasks), { count: 0, states: [] });
  // "Needs a human" keeps waiting tasks and done ones not yet checked.
  const human = hiddenByFilter("human", [...tasks, task({ id: "f", state: "done", without_human_check: "loop run" })]);
  assert.equal(human.count, 4);
});

test("the filter is named as the Show box words it", () => {
  assert.equal(filterName("backlog", t), "Backlog");
  assert.equal(filterName("human", t), "Needs a human (to check, to answer, to review)");
  assert.equal(filterName("all", t), "All states");
});

test("AI decided: the decisions from the history go on their tasks, and the Needs a human filter keeps them (Task-A03-19)", () => {
  const decision = { at: "2026-10-15T10:00:00Z", by: "claude/x", text: "Names: alm describe — match alm title" };
  const opened: Opened = {
    root: "/r",
    readme_added: false,
    loaded: {
      taskbag: {
        project: { format: "allomos/4", id: "P", name: "P", labels: ["human verified"] },
        areas: [],
        iterations: [],
        tasks: [task({ id: "A01-01", state: "backlog" }), task({ id: "A01-02" })],
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
  assert.equal(keepState("human", tasks[0]), true);
  assert.equal(keepState("human", tasks[1]), false);
  // Reviewed: the history no longer lists it, so the badge goes.
  const after = withDecisions({ ...opened, ai_decisions: {} }).loaded.taskbag!.tasks;
  assert.equal(toReview(after[0]), false);
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
  assert.equal(filterGroups(groups, "all"), groups);
  const kept = filterGroups(groups, "blocked");
  assert.deepEqual(kept.map((g) => g.key), ["a"]);
  assert.deepEqual(kept[0].tasks.map((x) => x.id), ["A-1"]);
  assert.equal(kept[0].counted?.length, 2);
  assert.deepEqual(filterGroups(groups, "waiting"), []);
});

test("the filter banner counts tasks and the groups left out", () => {
  assert.deepEqual(filterBanner("all", { kind: "area", tasks: 3, groups: 2, total: 7 }, t), []);
  assert.equal(
    plain(filterBanner("human", { kind: "area", tasks: 3, groups: 2, total: 7 }, t)),
    "**3 tasks** need a human (to check, to answer, to review), in 2 of 7 areas. The 5 areas with none are left out.",
  );
  assert.equal(
    plain(filterBanner("blocked", { kind: "iteration", tasks: 1, groups: 1, total: 2 }, t)),
    "**1 task** is blocked, in 1 of 2 iterations. The 1 iteration with none is left out.",
  );
  assert.equal(plain(filterBanner("ready", { kind: "area", tasks: 4, groups: 3, total: 3 }, t)), "**4 tasks** are ready, in 3 areas.");
});

test("nothing matches: only the banner, saying every group is left out", () => {
  assert.equal(
    plain(filterBanner("blocked", { kind: "iteration", tasks: 0, groups: 0, total: 12 }, t)),
    "No task is blocked. All 12 iterations are left out.",
  );
  assert.equal(plain(filterBanner("blocked", { kind: "area", tasks: 0, groups: 0, total: 1 }, t)), "No task is blocked. The only area is left out.");
  assert.equal(plain(filterBanner("done", { kind: "area", tasks: 0, groups: 0, total: 0 }, t)), "No task is done.");
});

test("while searching, the filter banner says the list is the filter and the search together (Task-A01-48)", () => {
  assert.equal(
    plain(bothBanner("waiting", { kind: "area", query: "alm", tasks: 2, groups: 1, matches: 5 }, t)),
    "Filter and search together: only tasks that are waiting and hold “alm” are listed, in 1 area. 3 more matches are hidden by the filter.",
  );
  assert.equal(
    plain(bothBanner("done", { kind: "iteration", query: "x", tasks: 2, groups: 2, matches: 3 }, t)),
    "Filter and search together: only tasks that are done and hold “x” are listed, in 2 iterations. 1 more match is hidden by the filter.",
  );
  assert.equal(
    plain(bothBanner("ready", { kind: "area", query: "x", tasks: 4, groups: 3, matches: 4 }, t)),
    "Filter and search together: only tasks that are ready and hold “x” are listed, in 3 areas.",
  );
  assert.equal(
    plain(bothBanner("blocked", { kind: "area", query: "alm", tasks: 0, groups: 0, matches: 3 }, t)),
    "None of the tasks found for “alm” is blocked. All 3 matches are hidden by the filter.",
  );
  assert.equal(plain(bothBanner("blocked", { kind: "area", query: "alm", tasks: 0, groups: 0, matches: 1 }, t)), "None of the tasks found for “alm” is blocked. The 1 match is hidden by the filter.");
  // Nothing found at all: "Nothing found" says so, not the banner; no filter: no filter banner.
  assert.deepEqual(bothBanner("blocked", { kind: "area", query: "zzz", tasks: 0, groups: 0, matches: 0 }, t), []);
  assert.deepEqual(bothBanner("all", { kind: "area", query: "alm", tasks: 2, groups: 1, matches: 2 }, t), []);
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
  const filtered = filterGroups(groups, "waiting");
  const found = search(filtered);
  assert.deepEqual(found.map((g) => g.tasks.map((x) => x.id)), [["A-1"]]);
  const both = taskListBanners({ kind: "area", filter: "waiting", query: " alm ", groups, filtered, found, matches }, t);
  assert.equal(plain(both.search), "**1 task** found for “alm” with the “Waiting” filter on.");
  assert.equal(
    plain(both.filter),
    "Filter and search together: only tasks that are waiting and hold “alm” are listed, in 1 area. 2 more matches are hidden by the filter.",
  );
  // The search cleared: the filter banner alone.
  const filterOnly = taskListBanners({ kind: "area", filter: "waiting", query: "", groups, filtered, found: filtered, matches: 0 }, t);
  assert.deepEqual(filterOnly.search, []);
  assert.equal(plain(filterOnly.filter), "**2 tasks** are waiting, in 2 areas.");
  // The filter off: the search banner alone, counting every match.
  const searchOnly = taskListBanners({ kind: "area", filter: "all", query: "alm", groups, filtered: groups, found: search(groups), matches }, t);
  assert.equal(plain(searchOnly.search), "**3 tasks** found for “alm”.");
  assert.deepEqual(searchOnly.filter, []);
  // Every match hidden by the filter: no search banner, the filter banner says why.
  const none = taskListBanners({ kind: "area", filter: "blocked", query: "alm", groups, filtered: [], found: [], matches }, t);
  assert.deepEqual(none.search, []);
  assert.equal(plain(none.filter), "None of the tasks found for “alm” is blocked. All 3 matches are hidden by the filter.");
});

test("the search banner says how much was found", () => {
  assert.equal(plain(searchBanner("task", 2, "alm", t)), "**2 tasks** found for “alm”.");
  assert.equal(plain(searchBanner("task", 2, "alm", t, "In progress")), "**2 tasks** found for “alm” with the “In progress” filter on.");
});

test("Not done keeps every task neither done nor dropped, is remembered, and is worded (A01-party987)", () => {
  const states = ["backlog", "ready", "in_progress", "waiting", "blocked", "done", "dropped"] as const;
  const kept = states.filter((state) => keepState("open", task({ state })));
  assert.deepEqual(kept, ["backlog", "ready", "in_progress", "waiting", "blocked"]);
  assert.equal(readFilter("open"), "open");
  assert.equal(filterName("open", t), "Not done");
  const all = [...states.map((state, i) => task({ id: `A01-${i}`, state })), task({ id: "A01-9", state: "done" })];
  const hidden = hiddenByFilter("open", all);
  assert.equal(hidden.count, 3);
  assert.deepEqual(hidden.states, [
    { state: "done", count: 2 },
    { state: "dropped", count: 1 },
  ]);
  assert.equal(hiddenText(hidden, t), "3 more hidden by the filter: 2 Done, 1 Dropped");
  assert.equal(
    plain(filterBanner("open", { kind: "area", tasks: 12, groups: 3, total: 3 }, t)),
    "**12 tasks** are still open, in 3 areas.",
  );
  assert.equal(plain(filterBanner("open", { kind: "iteration", tasks: 0, groups: 0, total: 2 }, t)), "No task is still open. All 2 iterations are left out.");
  assert.equal(
    plain(bothBanner("open", { kind: "area", query: "alm", tasks: 2, groups: 1, matches: 5 }, t)),
    "Filter and search together: only tasks that are still open and hold “alm” are listed, in 1 area. 3 more matches are hidden by the filter.",
  );
});
