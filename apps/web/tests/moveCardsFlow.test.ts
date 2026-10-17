// Tests for the card files notice (Task-A00-30). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { cardRows, cardsNotice, cardsOutside } from "../src/moveCardsFlow.ts";
import { changeLine } from "../src/historyFlow.ts";
import type { Opened, Task } from "../src/types.ts";

const task = (id: string) => ({ id, title: `Title of ${id}` }) as Task;

const opened = (ids: string[], cards: Opened["cards_outside"]): Opened =>
  ({
    root: "/p/.allomos/taskbag",
    readme_added: false,
    loaded: { taskbag: { tasks: ids.map(task) }, issues: [] },
    cards_outside: cards,
  }) as unknown as Opened;

test("the notice counts the card files linked or reported, for tasks the taskbag has", () => {
  const cards: Opened["cards_outside"] = [
    {
      task: "A01-01",
      path: "docs/tasks/A01-01.md",
      source: "report",
      at: "2026-10-16T10:00:00Z",
      by: "claude/x",
    },
    // Linked from the description (Task-A01-51): shown without any report.
    { task: "A01-02", path: "docs/tasks/A01-02.md", source: "link" },
    { task: "A09-09", path: "docs/tasks/A09-09.md", source: "report", at: "2026-10-16T10:00:00Z" },
  ];
  assert.equal(cardsNotice(opened(["A01-01", "A01-02"], cards)), 2);
  assert.equal(cardsNotice(opened(["A01-02"], cards)), 1);
  assert.deepEqual(
    cardsOutside(opened(["A01-01"], cards)).map((c) => c.task),
    ["A01-01"],
  );
  // Nothing reported, an older app's result without the field, or no taskbag: no notice.
  assert.equal(cardsNotice(opened(["A01-01"], [])), 0);
  assert.equal(cardsNotice(opened(["A01-01"], undefined)), 0);
  assert.equal(cardsNotice(null), 0);
});

test("the dialog lists what Allomos found, with each task's title and how it was found", () => {
  const cards: Opened["cards_outside"] = [
    { task: "A01-01", path: "docs/tasks/A01-01.md", source: "link" },
    { task: "A01-02", path: "docs/tasks/A01-02.md", source: "report", at: "2026-10-17T08:00:00Z" },
    { task: "A09-09", path: "docs/tasks/A09-09.md", source: "link" },
  ];
  assert.deepEqual(cardRows(opened(["A01-01", "A01-02"], cards)), [
    { task: "A01-01", title: "Title of A01-01", path: "docs/tasks/A01-01.md", reported: false },
    { task: "A01-02", title: "Title of A01-02", path: "docs/tasks/A01-02.md", reported: true },
  ]);
  // Opened from the menu with nothing found: an empty list (the dialog says so).
  assert.deepEqual(cardRows(opened(["A01-01"], [])), []);
  assert.deepEqual(cardRows(null), []);
});

test("history names the card changes in the interface's words", () => {
  const t = (key: string) => key;
  assert.equal(changeLine({ field: "card", to: "card.md" }, t as never), "pane.card: — → card.md");
  assert.equal(
    changeLine({ field: "card_outside", from: "docs/tasks/A01-01.md" }, t as never),
    "pane.card_outside: docs/tasks/A01-01.md → —",
  );
});
