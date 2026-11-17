// Tests for a change to one task shown in place (Task-A01-parade485): the list after it is the
// list reading the whole taskbag again would give, and when only reading again can tell.
// Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { withTaskChange, type TaskChange } from "../src/taskChangeFlow.ts";
import { decorated, isBlocked, withBlocked, withDecisions } from "../src/stopFlow.ts";
import type { Opened, Task } from "../src/types.ts";

const base: Task = {
  id: "A01-01",
  title: "T",
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

/** What `open_taskbag` answers for these tasks, before `withDecisions`. */
const read = (tasks: Task[], over: Partial<Opened> = {}): Opened => ({
  root: "/p",
  readme_added: false,
  loaded: {
    taskbag: { project: { id: "p", name: "P" } as never, areas: [], iterations: [], tasks },
    issues: [],
    active_iteration: null,
  },
  ai_decisions: { "A01-02": [{ text: "chose a name", by: "claude", at: "2026-11-01T00:00:00Z" } as never] },
  approvals: {},
  classifications: { "A01-02": { risk: "high" } },
  effects: {},
  links: {},
  branches: {},
  last_changed: Object.fromEntries(tasks.map((t) => [t.id, "2026-11-01T00:00:00Z"])),
  cards_outside: [],
  old_handoffs: [],
  versions: Object.fromEntries(tasks.map((t) => [t.id, `v-${t.id}-${t.state}-${t.title}`])),
  ...over,
});

const change = (t: Task, over: Partial<TaskChange> = {}): TaskChange => ({
  task: t,
  version: `v-${t.id}-${t.state}-${t.title}`,
  changed_at: "2026-11-16T05:00:00Z",
  card_outside: null,
  whole: false,
  stamp_before: "1",
  stamp_after: "2",
  ...over,
});

const one = task({ id: "A01-01", state: "in_progress" });
const two = task({ id: "A01-02", blocked_by: ["A01-01"] });
const three = task({ id: "A01-03" });

test("a changed task in place gives the list a whole read would give", () => {
  const shown = withDecisions(read([one, two, three]));
  const done = task({ id: "A01-01", state: "done", labels: ["locally verified by agent"] });
  const inPlace = withTaskChange(shown, change(done), "1", decorated);
  // The whole read after the same change: the same tasks, the changed one's version and time new.
  const again = withDecisions(
    read([done, two, three], {
      last_changed: { "A01-01": "2026-11-16T05:00:00Z", "A01-02": "2026-11-01T00:00:00Z", "A01-03": "2026-11-01T00:00:00Z" },
    }),
  );
  assert.deepEqual(inPlace, again);
  // What follows the task follows by itself: the task it blocked is no longer blocked.
  const blocked = (o: Opened) => isBlocked(withBlocked(o).loaded.taskbag!.tasks.find((t) => t.id === "A01-02")!);
  assert.equal(blocked(shown), true);
  assert.equal(blocked(inPlace!), false);
  // The other tasks are the same objects: nothing else is drawn again.
  assert.equal(inPlace!.loaded.taskbag!.tasks[2], shown.loaded.taskbag!.tasks[2]);
});

test("the changed task keeps what the history holds for it", () => {
  const shown = withDecisions(read([one, two, three]));
  const renamed = task({ ...two, title: "New title" });
  const next = withTaskChange(shown, change(renamed), "1", decorated)!;
  const t = next.loaded.taskbag!.tasks[1];
  assert.equal(t.title, "New title");
  assert.equal(t.ai_decisions?.length, 1);
  assert.equal(t.risk, "high");
  assert.equal(next.versions?.["A01-02"], "v-A01-02-ready-New title");
  assert.deepEqual(next, withDecisions(read([one, renamed, three], { last_changed: { ...shown.last_changed, "A01-02": "2026-11-16T05:00:00Z" } })));
});

test("a change that changed nothing keeps the last changed time", () => {
  const shown = withDecisions(read([one, two]));
  const next = withTaskChange(shown, change(one, { changed_at: null }), "1", decorated)!;
  assert.equal(next.last_changed?.["A01-01"], "2026-11-01T00:00:00Z");
});

test("only a whole read can tell", () => {
  const shown = withDecisions(read([one, two]));
  const done = task({ id: "A01-01", state: "done" });
  // The change reached beyond the task.
  assert.equal(withTaskChange(shown, change(done, { whole: true }), "1", decorated), null);
  // Another session changed the taskbag too, or this window has no fingerprint yet.
  assert.equal(withTaskChange(shown, change(done), "0", decorated), null);
  assert.equal(withTaskChange(shown, change(done), null, decorated), null);
  // The taskbag's files have problems: the change may have mended one.
  const broken = { ...shown, loaded: { ...shown.loaded, issues: [{ file: "tasks/x/task.json" } as never] } };
  assert.equal(withTaskChange(broken, change(done), "1", decorated), null);
  // A task the list does not hold.
  assert.equal(withTaskChange(shown, change(task({ id: "A01-09" })), "1", decorated), null);
  // A new description of a task with approvals: whether they still apply is in the history.
  const approved = withDecisions(read([one, two], { approvals: { "A01-01": [{ status: "applies" } as never] } }));
  assert.equal(withTaskChange(approved, change(task({ ...one, description: "new plan" })), "1", decorated), null);
  // Its other changes are shown in place, the approvals still on it.
  const next = withTaskChange(approved, change(done), "1", decorated)!;
  assert.equal(next.loaded.taskbag!.tasks[0].approvals?.length, 1);
});

test("the card file a description links to outside the taskbag follows the change", () => {
  const link = (id: string, path: string) => ({ task: id, path, source: "link" as const });
  const shown = withDecisions(read([one, two, three], { cards_outside: [link("A01-03", "docs/c.md")] }));
  // A link added: listed in the tasks' order.
  const linked = withTaskChange(shown, change(task({ ...one, description: "Card: docs/a.md" }), { card_outside: link("A01-01", "docs/a.md") }), "1", decorated)!;
  assert.deepEqual(linked.cards_outside, [link("A01-01", "docs/a.md"), link("A01-03", "docs/c.md")]);
  // A link taken away.
  const gone = withTaskChange(shown, change(task({ ...three, description: "" })), "1", decorated)!;
  assert.deepEqual(gone.cards_outside, []);
  // An agent's report wins and stays.
  const report = { task: "A01-01", path: "docs/r.md", source: "report" as const, at: "2026-11-01T00:00:00Z" };
  const reported = withDecisions(read([one, two], { cards_outside: [report] }));
  const kept = withTaskChange(reported, change(task({ ...one, description: "Card: docs/a.md" }), { card_outside: link("A01-01", "docs/a.md") }), "1", decorated)!;
  assert.deepEqual(kept.cards_outside, [report]);
});
