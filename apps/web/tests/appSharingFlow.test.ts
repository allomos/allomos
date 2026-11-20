// Tests for the unshared app changes notice grouping (Task-A01-cliff402). Run: npm --prefix apps/web test

import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { groupAppChanges } from "../src/appSharingFlow.ts";
import type { AppChanges, Loaded } from "../src/types.ts";

const taskbag: NonNullable<Loaded["taskbag"]> = {
  project: { format: "allomos/9", id: "allomos", name: "Allomos", labels: [] },
  areas: [{ id: "A01", name: "Desktop", order: 1 }],
  iterations: [{ id: "I-water", name: "Water", order: 1 }],
  tasks: [
    {
      id: "A01-cliff402",
      title: "Group direct app edits",
      area: "A01",
      state: "in_progress",
      holder: null,
      waiting_on: null,
      next_dependency: null,
      iteration: "I-water",
      handoff_note: null,
      without_human_check: null,
      human_check_steps: null,
      description: "",
    },
  ],
};

const changes = (entries: AppChanges["entries"]): AppChanges => ({
  count: entries.length,
  through: entries.at(-1)?.event ?? null,
  files: [],
  by: ["p-user"],
  entries,
});

test("no unshared app changes returns null", () => {
  assert.equal(groupAppChanges(undefined, taskbag), null);
  assert.equal(groupAppChanges(null, taskbag), null);
  assert.equal(groupAppChanges(changes([]), taskbag), null);
});

test("repeated direct edits to the same task are grouped together with deduplicated fields and latest time", () => {
  const grouped = groupAppChanges(
    changes([
      { event: "events/1.json", at: "2026-11-20T01:00:00Z", id: "A01-cliff402", command: "edit", fields: ["title", "state"] },
      { event: "events/2.json", at: "2026-11-20T01:05:00Z", id: "A01-cliff402", command: "edit", fields: ["state", "description"] },
    ]),
    taskbag,
  );
  assert.deepEqual(grouped, {
    kind: "app_direct",
    count: 2,
    items: [
      {
        key: "A01-cliff402",
        id: "A01-cliff402",
        title: "Group direct app edits",
        fields: ["title", "state", "description"],
        count: 2,
        latestAt: "2026-11-20T01:05:00Z",
      },
    ],
  });
});

test("project, area, iteration and task edits are resolved and ordered newest first", () => {
  const grouped = groupAppChanges(
    changes([
      { event: "events/1.json", at: "2026-11-20T01:00:00Z", id: null, command: "settings", fields: ["name"] },
      { event: "events/2.json", at: "2026-11-20T01:02:00Z", id: "A01", command: "edit", fields: ["name"] },
      { event: "events/3.json", at: "2026-11-20T01:03:00Z", id: "allomos", command: "settings", fields: ["task_language"] },
      { event: "events/4.json", at: "2026-11-20T01:04:00Z", id: "I-water", command: "edit", fields: ["state"] },
    ]),
    taskbag,
  );
  assert.ok(grouped);
  assert.equal(grouped.kind, "app_direct");
  assert.equal(grouped.count, 4);
  assert.deepEqual(
    grouped.items.map((i) => ({ key: i.key, id: i.id, title: i.title, fields: i.fields, count: i.count, latestAt: i.latestAt })),
    [
      { key: "I-water", id: "I-water", title: "Water", fields: ["state"], count: 1, latestAt: "2026-11-20T01:04:00Z" },
      { key: "__project__", id: null, title: null, fields: ["name", "task_language"], count: 2, latestAt: "2026-11-20T01:03:00Z" },
      { key: "A01", id: "A01", title: "Desktop", fields: ["name"], count: 1, latestAt: "2026-11-20T01:02:00Z" },
    ],
  );
});

test("all three interface languages explain the notice reason and name the direct-app-edits group", () => {
  for (const lang of ["en", "zh-CN", "ja"]) {
    const words = JSON.parse(readFileSync(new URL(`../src/i18n/${lang}.json`, import.meta.url), "utf8")) as Record<string, string>;
    assert.ok(words["sharing.reason"]?.includes("Git"), `${lang} sharing.reason explains Git`);
    assert.ok(words["sharing.group_app"]?.includes("{count}"), `${lang} sharing.group_app includes {count}`);
    assert.ok(words["sharing.item_count"]?.includes("{count}"), `${lang} sharing.item_count includes {count}`);
  }
});
